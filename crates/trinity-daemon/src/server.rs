use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use trinity_app::engine::Engine;
use trinity_app::error::AppError;
use trinity_app::event::{BTN_EXTRA, BTN_LEFT, BTN_MIDDLE, BTN_RIGHT, BTN_SIDE};
use trinity_app::ports::{CalibrationRepository, Caps, DeviceLocator, ProfileRepository};
use trinity_core::calibration::Calibration;
use trinity_core::error::DomainError;
use trinity_core::key::KeyCode;
use trinity_core::node::NodeId;
use trinity_core::profile::Profile;
use trinity_infra::{
    ByIdLocator, EvdevSource, TomlCalibrationRepository, TomlProfileRepository, UinputSink,
    node_caps,
};

use trinity_app::ipc::{ProfileDto, Request, Response, Status, decode_request, encode_response};

const IDLE_SLEEP: Duration = Duration::from_millis(50);
const ERROR_SLEEP: Duration = Duration::from_millis(500);
/// Off-lock pause between iterations: lets the IPC thread acquire the
/// lock (otherwise the engine loop would hold it continuously and starve
/// IPC responses).
const YIELD_SLEEP: Duration = Duration::from_millis(1);

/// Server configuration (overridable paths, mostly for tests).
pub struct ServerConfig {
    pub socket_path: PathBuf,
    pub profiles_dir: PathBuf,
    pub calibration_path: PathBuf,
    pub default_profile: String,
    pub device_prefix: String,
}

impl ServerConfig {
    pub fn with_socket(mut self, path: impl Into<PathBuf>) -> Self {
        self.socket_path = path.into();
        self
    }

    pub fn with_profiles_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.profiles_dir = dir.into();
        self
    }

    pub fn with_calibration_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.calibration_path = path.into();
        self
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        let runtime = std::env::var_os("XDG_RUNTIME_DIR")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("TMPDIR").map(PathBuf::from))
            .unwrap_or_else(|| PathBuf::from("/tmp"));
        let root = trinity_infra::default_profiles_dir()
            .parent()
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."));
        Self {
            socket_path: runtime.join("trinity-mapper.sock"),
            calibration_path: root.join("calibration.toml"),
            profiles_dir: root.join("profiles"),
            default_profile: "default".to_owned(),
            device_prefix: ByIdLocator::trinity().prefix().to_owned(),
        }
    }
}

struct EngineState {
    engine: Engine<EvdevSource, UinputSink>,
    suspended: bool,
    profile_name: Option<String>,
    calibration: Option<Calibration>,
    error: Option<String>,
    calibration_log: usize,
}

/// Daemon: remapping engine + JSON-lines IPC server.
pub struct Server {
    config: ServerConfig,
    state: Arc<Mutex<EngineState>>,
    shutdown: Arc<AtomicBool>,
}

impl Server {
    pub fn new(config: ServerConfig) -> Self {
        Self {
            config,
            state: Arc::new(Mutex::new(EngineState {
                engine: Engine::new(EvdevSource::new(), UinputSink::new()),
                suspended: false,
                profile_name: None,
                calibration: None,
                error: None,
                calibration_log: 0,
            })),
            shutdown: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn socket_path(&self) -> &std::path::Path {
        &self.config.socket_path
    }

    /// Main loop: loads state, starts the engine, serves IPC.
    pub fn run(&self) -> Result<(), AppError> {
        self.bootstrap()?;

        let listener = self.bind_socket()?;

        let engine_loop = {
            let state = Arc::clone(&self.state);
            let shutdown = Arc::clone(&self.shutdown);
            std::thread::Builder::new()
                .name("engine-loop".into())
                .spawn(move || engine_loop(state, shutdown))
                .map_err(|err| AppError::Port(format!("starting engine loop: {err}")))?
        };

        for stream in listener.incoming() {
            if self.shutdown.load(Ordering::Relaxed) {
                break;
            }
            match stream {
                Ok(stream) => {
                    if self.handle_connection(stream) {
                        break;
                    }
                }
                Err(err) => {
                    eprintln!("trinity-daemon: IPC connection refused: {err}");
                }
            }
        }

        self.shutdown.store(true, Ordering::Relaxed);
        let _ = engine_loop.join();
        if let Ok(mut state) = self.state.lock() {
            if state.engine.is_active() {
                let _ = state.engine.stop();
            }
        }
        let _ = std::fs::remove_file(&self.config.socket_path);
        Ok(())
    }

    /// Initializes state (default profile, calibration) — public for tests.
    pub fn bootstrap(&self) -> Result<(), AppError> {
        let mut profiles = TomlProfileRepository::new(&self.config.profiles_dir);
        if profiles.list().unwrap_or_default().is_empty() {
            let default = Profile::new(&self.config.default_profile)?;
            profiles.save(&default)?;
        }
        let calibration = TomlCalibrationRepository::new(&self.config.calibration_path).load()?;
        let profile = profiles
            .load(&self.config.default_profile)
            .ok()
            .or_else(|| Profile::new(&self.config.default_profile).ok());

        let mut state = self.lock_state();
        state.calibration = calibration;
        state.profile_name = Some(self.config.default_profile.clone());
        if let Some(profile) = profile {
            let _ = state.engine.set_profile(profile);
        }
        if state.calibration.is_some() {
            if let Some(calibration) = state.calibration.clone() {
                let _ = state.engine.set_calibration(calibration);
            }
        }
        if let Err(err) = self.try_enable(&mut state) {
            eprintln!("trinity-daemon: engine not started: {err}");
            state.error = Some(err.to_string());
        }
        Ok(())
    }

    fn bind_socket(&self) -> Result<UnixListener, AppError> {
        if let Some(parent) = self.config.socket_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|err| AppError::Port(format!("socket directory: {err}")))?;
        }
        let _ = std::fs::remove_file(&self.config.socket_path);
        UnixListener::bind(&self.config.socket_path)
            .map_err(|err| AppError::Port(format!("socket bind: {err}")))
    }

    /// Handles one connection; returns `true` if shutdown was requested.
    fn handle_connection(&self, stream: UnixStream) -> bool {
        let mut writer = match stream.try_clone() {
            Ok(writer) => writer,
            Err(err) => {
                eprintln!("trinity-daemon: IPC stream clone failed: {err}");
                return false;
            }
        };
        let reader = BufReader::new(stream);
        for line in reader.lines() {
            let Ok(line) = line else { break };
            if line.trim().is_empty() {
                continue;
            }
            let response = match decode_request(&line) {
                Ok(request) => self.handle_request(request),
                Err(message) => Response::Error { message },
            };
            let encoded = encode_response(&response);
            if writeln!(writer, "{encoded}")
                .and_then(|_| writer.flush())
                .is_err()
            {
                break;
            }
            if matches!(response, Response::Ok) && self.shutdown.load(Ordering::Relaxed) {
                return true;
            }
        }
        false
    }

    /// Handles a single IPC request — public for integration tests.
    pub fn handle_request(&self, request: Request) -> Response {
        let result = match request {
            Request::GetStatus => return self.build_status(),
            Request::SetEnabled { enabled } => self.set_enabled(enabled),
            Request::SetProfile { name } => self.set_profile(&name),
            Request::DeleteProfile { name } => self.delete_profile(&name),
            Request::RenameProfile { from, to } => self.rename_profile(&from, &to),
            Request::ListProfiles => {
                let profiles = TomlProfileRepository::new(&self.config.profiles_dir);
                return match profiles.list() {
                    Ok(names) => Response::Profiles { names },
                    Err(err) => Response::Error {
                        message: err.to_string(),
                    },
                };
            }
            Request::GetProfile { name } => {
                return match self.load_profile_dto(&name) {
                    Ok(profile) => Response::Profile { profile },
                    Err(err) => Response::Error {
                        message: err.to_string(),
                    },
                };
            }
            Request::SaveProfile { profile } => self.save_profile_dto(profile),
            Request::BeginCalibration => self.lock_state().engine.begin_calibration(),
            Request::CancelCalibration => {
                self.lock_state().engine.cancel_calibration();
                Ok(())
            }
            Request::FinishCalibration => self.finish_calibration(),
            Request::Shutdown => {
                self.shutdown.store(true, Ordering::Relaxed);
                Ok(())
            }
        };
        match result {
            Ok(()) => Response::Ok,
            Err(err) => {
                let mut state = self.lock_state();
                state.error = Some(err.to_string());
                Response::Error {
                    message: err.to_string(),
                }
            }
        }
    }

    fn set_enabled(&self, enabled: bool) -> Result<(), AppError> {
        let mut state = self.lock_state();
        if !state.engine.is_active() {
            // Engine never started: only `true` attempts the grab;
            // otherwise just remember the suspended state.
            if enabled {
                self.try_enable(&mut state)?;
            }
            state.suspended = !enabled;
            return Ok(());
        }
        state.suspended = !enabled;
        if enabled {
            state.engine.resume_translation()
        } else {
            // Devices are never torn down: full passthrough instead
            // (recreating mirror + grab on toggle races the Wayland
            // compositor).
            state.engine.suspend_translation()
        }
    }

    fn set_profile(&self, name: &str) -> Result<(), AppError> {
        let profile = TomlProfileRepository::new(&self.config.profiles_dir).load(name)?;
        let mut state = self.lock_state();
        state.engine.set_profile(profile)?;
        state.profile_name = Some(name.to_owned());
        Ok(())
    }

    fn rename_profile(&self, from: &str, to: &str) -> Result<(), AppError> {
        if from == to {
            return Ok(());
        }
        let mut repository = TomlProfileRepository::new(&self.config.profiles_dir);
        let profile = repository.load(from)?;
        let new_profile = trinity_core::Profile::new(to)?;
        let mut renamed = new_profile;
        for (button, combination) in profile.mappings() {
            renamed.set_mapping(button, combination.clone());
        }
        repository.save(&renamed)?;
        repository.delete(from)?;
        let mut state = self.lock_state();
        if state.profile_name.as_deref() == Some(from) {
            state.engine.set_profile(renamed)?;
            state.profile_name = Some(to.to_owned());
        }
        Ok(())
    }

    fn delete_profile(&self, name: &str) -> Result<(), AppError> {
        if name == self.config.default_profile {
            return Err(AppError::Port("cannot delete the default profile".into()));
        }
        TomlProfileRepository::new(&self.config.profiles_dir).delete(name)?;
        let mut state = self.lock_state();
        if state.profile_name.as_deref() == Some(name) {
            if let Ok(default) = TomlProfileRepository::new(&self.config.profiles_dir)
                .load(&self.config.default_profile)
            {
                state.engine.set_profile(default)?;
                state.profile_name = Some(self.config.default_profile.clone());
            }
        }
        Ok(())
    }

    fn load_profile_dto(&self, name: &str) -> Result<ProfileDto, AppError> {
        let profile = TomlProfileRepository::new(&self.config.profiles_dir).load(name)?;
        trinity_infra::profile_to_dto(&profile)
    }

    fn save_profile_dto(&self, dto: ProfileDto) -> Result<(), AppError> {
        let profile = trinity_infra::profile_from_dto(&dto)?;
        TomlProfileRepository::new(&self.config.profiles_dir).save(&profile)?;
        let mut state = self.lock_state();
        if state.profile_name.as_deref() == Some(profile.name()) {
            state.engine.set_profile(profile)?;
        }
        Ok(())
    }

    fn finish_calibration(&self) -> Result<(), AppError> {
        let mut state = self.lock_state();
        let calibration = state.engine.finish_calibration()?;
        TomlCalibrationRepository::new(&self.config.calibration_path).save(&calibration)?;
        state.calibration = Some(calibration);
        Ok(())
    }

    fn try_enable(&self, state: &mut EngineState) -> Result<(), AppError> {
        if state.engine.is_active() {
            state.suspended = false;
            state.engine.resume_translation()?;
            return Ok(());
        }
        let nodes = ByIdLocator::new(&self.config.device_prefix).locate()?;
        if nodes.is_empty() {
            return Err(AppError::Port("no Trinity node found".into()));
        }
        let caps = full_caps(&nodes)?;
        state.engine.start(&nodes, &caps)?;
        // Restore remembered state: without this, an engine restart
        // would run in passthrough with no remapping.
        if let Some(calibration) = state.calibration.clone() {
            state.engine.set_calibration(calibration)?;
        }
        state.engine.resume_translation()?;
        state.suspended = false;
        state.error = None;
        Ok(())
    }

    fn build_status(&self) -> Response {
        let device_present = ByIdLocator::new(&self.config.device_prefix)
            .locate()
            .map(|nodes| !nodes.is_empty())
            .unwrap_or(false);
        let calibrated = TomlCalibrationRepository::new(&self.config.calibration_path)
            .load()
            .map(|calibration| calibration.is_some_and(|c| c.is_complete()))
            .unwrap_or(false);
        let state = self.lock_state();
        let (current_button, captured_count) =
            state.engine.calibration_progress().unwrap_or((None, 0));
        Response::Status(Status {
            device_present,
            enabled: state.engine.is_active() && !state.suspended,
            calibrated: calibrated || state.calibration.is_some(),
            profile: state.profile_name.clone(),
            calibrating: state.engine.is_calibrating(),
            current_button,
            captured_count,
            total_buttons: 12,
            error: state.error.clone(),
        })
    }

    fn lock_state(&self) -> std::sync::MutexGuard<'_, EngineState> {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

fn engine_loop(state: Arc<Mutex<EngineState>>, shutdown: Arc<AtomicBool>) {
    while !shutdown.load(Ordering::Relaxed) {
        let mut guard = state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if !guard.engine.is_active() {
            drop(guard);
            std::thread::sleep(IDLE_SLEEP);
            continue;
        }
        match guard.engine.run_once() {
            Ok(_) => log_new_captures(&mut guard),
            Err(AppError::Domain(DomainError::PhysicalCodeConflict {
                node,
                code,
                first,
                second,
            })) => {
                // Log captures before cancelling: the faulty batch may
                // contain an otherwise invisible successful capture.
                log_new_captures(&mut guard);
                guard.engine.cancel_calibration();
                guard.error = Some(format!(
                    "calibration conflict: code {code} on {node} is emitted by both button {first} and button {second}"
                ));
            }
            Err(err) => {
                guard.error = Some(err.to_string());
                drop(guard);
                std::thread::sleep(ERROR_SLEEP);
                continue;
            }
        }
        drop(guard);
        std::thread::sleep(YIELD_SLEEP);
    }
}

/// Prints calibration captures not logged yet.
fn log_new_captures(guard: &mut EngineState) {
    if !guard.engine.is_calibrating() {
        guard.calibration_log = 0;
        return;
    }
    let fresh: Vec<(u8, String, u16)> = guard
        .engine
        .calibration_snapshot()
        .iter()
        .skip(guard.calibration_log)
        .map(|(button, node, code)| (*button, node.to_string(), *code))
        .collect();
    for (button, node, code) in fresh {
        eprintln!("trinity-daemon: calibration button {button} <- code {code} on {node}");
        guard.calibration_log += 1;
    }
}

/// Mirror caps: union of node caps plus every known key.
fn full_caps(nodes: &[NodeId]) -> Result<Caps, AppError> {
    let mut caps = Caps::new();
    for node in nodes {
        caps.merge(&node_caps(node)?);
    }
    for &(_, code) in KeyCode::known() {
        caps.add_key_code(code);
    }
    for code in [BTN_LEFT, BTN_RIGHT, BTN_MIDDLE, BTN_SIDE, BTN_EXTRA] {
        caps.add_key_code(code);
    }
    Ok(caps)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TempDir(PathBuf);

    impl TempDir {
        fn new(tag: &str) -> Self {
            let dir = std::env::temp_dir().join(format!(
                "trinity-daemon-{tag}-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            std::fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn server(tag: &str) -> (Server, TempDir) {
        let temp = TempDir::new(tag);
        let mut config = ServerConfig::default()
            .with_socket(temp.0.join("sock"))
            .with_profiles_dir(temp.0.join("profiles"))
            .with_calibration_path(temp.0.join("calibration.toml"));
        config.device_prefix = "usb-Trinity-Mapper-Test-Inexistant".to_owned();
        (Server::new(config), temp)
    }

    #[test]
    fn bootstrap_creates_default_profile() {
        let (server, _temp) = server("bootstrap");
        server.bootstrap().unwrap();
        let profiles = TomlProfileRepository::new(&server.config.profiles_dir);
        assert_eq!(profiles.list().unwrap(), vec!["default".to_owned()]);
    }

    #[test]
    fn status_reports_uncalibrated_device() {
        let (server, _temp) = server("status");
        server.bootstrap().unwrap();
        let Response::Status(status) = server.build_status() else {
            panic!("expected status");
        };
        assert_eq!(status.profile.as_deref(), Some("default"));
        assert!(!status.calibrated);
        assert!(!status.calibrating);
    }

    #[test]
    fn unknown_profile_yields_not_found() {
        let (server, _temp) = server("profile");
        server.bootstrap().unwrap();
        let response = server.handle_request(Request::SetProfile {
            name: "inexistant".into(),
        });
        let Response::Error { message } = response else {
            panic!("expected error");
        };
        assert!(message.contains("inexistant"));
    }

    #[test]
    fn shutdown_request_stops_loop() {
        let (server, _temp) = server("shutdown");
        assert!(!server.shutdown.load(Ordering::Relaxed));
        let response = server.handle_request(Request::Shutdown);
        assert_eq!(response, Response::Ok);
        assert!(server.shutdown.load(Ordering::Relaxed));
    }

    #[test]
    fn enable_without_device_reports_error() {
        let (server, _temp) = server("enable");
        server.bootstrap().unwrap();
        let response = server.handle_request(Request::SetEnabled { enabled: true });
        let Response::Error { message } = response else {
            panic!("expected error");
        };
        assert!(message.contains("no Trinity node"), "message: {message}");
    }

    #[test]
    fn socket_binds_and_cleans_up() {
        let (server, _temp) = server("socket");
        let listener = server.bind_socket().unwrap();
        assert!(server.socket_path().exists());
        drop(listener);
        let _ = std::fs::remove_file(server.socket_path());
    }
}
