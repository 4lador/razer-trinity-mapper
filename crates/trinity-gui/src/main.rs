mod icons;
mod ipc;
mod keys;
mod layout;
mod settings;
mod theme;
mod view;

rust_i18n::i18n!("locales", fallback = "en");

use std::time::Duration;

use iced::keyboard::{Key, Modifiers};
use iced::{Element, Subscription, Task};
use trinity_app::ipc::{ButtonMappingDto, ProfileDto, Request, Response, Status};
use trinity_core::combo::KeyCombination;
use trinity_core::key::KeyCode;
use trinity_core::modifier::Modifier;

use crate::ipc::{DAEMON_UNREACHABLE, DaemonClient};
use crate::layout::KeyboardLayout;

const POLL_INTERVAL: Duration = Duration::from_millis(700);
const TOAST_LIFETIME: Duration = Duration::from_secs(4);

/// Client-side decorations under Wayland only: on X11 without a
/// compositor, corner transparency renders black and edge resizing
/// disappears — keep system decorations there.
pub fn wayland_session() -> bool {
    std::env::var("XDG_SESSION_TYPE").is_ok_and(|session| session.eq_ignore_ascii_case("wayland"))
        || std::env::var_os("WAYLAND_DISPLAY").is_some()
}

pub const DEFAULT_WINDOW_WIDTH: f32 = 880.0;
pub const DEFAULT_WINDOW_HEIGHT: f32 = 620.0;
pub const MIN_WINDOW_WIDTH: f32 = 700.0;
pub const MIN_WINDOW_HEIGHT: f32 = 540.0;
pub const NARROW_BREAKPOINT: f32 = 860.0;

/// Layout chosen from the window width.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowLayout {
    /// Grid and panel side by side.
    Wide,
    /// Panel stacked under the grid.
    Narrow,
}

pub fn layout_for(width: f32) -> WindowLayout {
    if width >= NARROW_BREAKPOINT {
        WindowLayout::Wide
    } else {
        WindowLayout::Narrow
    }
}

#[derive(Debug, Clone)]
pub struct Notice {
    pub text: String,
    pub since: std::time::Instant,
}

#[derive(Debug, Clone)]
enum Message {
    Tick,
    WindowResized(f32),
    MaximizedChanged(bool),
    MainWindowReady(Option<iced::window::Id>),
    DragWindow,
    ToggleMaximize,
    MinimizeWindow,
    CloseWindow,
    IpcResult(Box<Response>),
    ToggleEnabled(bool),
    SelectProfile(String),
    EditButton(u8),
    ClearButton(u8),
    KeyPressed(Key, Modifiers),
    StartCalibration,
    CancelCalibration,
    NewProfileName(String),
    CreateProfile,
    Refresh,
    ToggleSettings,
    SelectLocale(String),
    CopyProjectUrl,
    CopyShortcut(String),
    RequestDeleteProfile(String),
    ProfileHover(Option<String>),
    ConfirmDeleteProfile,
    CancelDeleteProfile,
}

/// Page displayed in the main scene.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Route {
    #[default]
    Main,
    Settings,
}

struct App {
    socket: std::path::PathBuf,
    connected: Option<bool>,
    status: Option<Status>,
    profiles: Vec<String>,
    loaded_profile: Option<ProfileDto>,
    editing: Option<u8>,
    new_profile: String,
    notice: Option<Notice>,
    finishing_calibration: bool,
    layout: KeyboardLayout,
    pulse: bool,
    window_width: f32,
    main_window: Option<iced::window::Id>,
    csd: bool,
    maximized: bool,
    route: Route,
    settings: settings::GuiSettings,
    confirm_delete: Option<String>,
    hovered_profile: Option<String>,
}

impl App {
    fn new() -> Self {
        let client = DaemonClient::default_socket();
        let app = Self {
            socket: client.socket().to_path_buf(),
            connected: None,
            status: None,
            profiles: Vec::new(),
            loaded_profile: None,
            editing: None,
            new_profile: String::new(),
            notice: None,
            finishing_calibration: false,
            layout: KeyboardLayout::load().unwrap_or_default(),
            pulse: false,
            window_width: DEFAULT_WINDOW_WIDTH,
            main_window: None,
            csd: wayland_session(),
            maximized: false,
            route: Route::Main,
            settings: settings::GuiSettings::load(),
            confirm_delete: None,
            hovered_profile: None,
        };
        rust_i18n::set_locale(app.settings.effective_locale().as_str());
        app
    }

    fn toast_error(&mut self, text: impl Into<String>) {
        self.notice = Some(Notice {
            text: text.into(),
            since: std::time::Instant::now(),
        });
    }

    fn window_task(&self, action: impl FnOnce(iced::window::Id) -> Task<Message>) -> Task<Message> {
        self.main_window.map(action).unwrap_or_else(Task::none)
    }

    fn send(&self, request: Request) -> Task<Message> {
        Task::perform(
            ipc::request_async(self.socket.clone(), request),
            |response| Message::IpcResult(Box::new(response)),
        )
    }

    fn refresh_all(&self) -> Task<Message> {
        Task::batch([
            self.send(Request::GetStatus),
            self.send(Request::ListProfiles),
        ])
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::WindowResized(width) => {
                self.window_width = width;
                // Re-sync maximized state (KWin snap, Alt+F11, etc.)
                self.window_task(|id| iced::window::is_maximized(id).map(Message::MaximizedChanged))
            }
            Message::MaximizedChanged(maximized) => {
                self.maximized = maximized;
                Task::none()
            }
            Message::MainWindowReady(id) => {
                self.main_window = id;
                Task::none()
            }
            Message::DragWindow => self.window_task(iced::window::drag),
            Message::ToggleMaximize => self.window_task(|id| {
                let toggle: Task<Message> = iced::window::toggle_maximize(id);
                toggle.then(move |_| iced::window::is_maximized(id).map(Message::MaximizedChanged))
            }),
            Message::MinimizeWindow => self.window_task(|id| iced::window::minimize(id, true)),
            Message::CloseWindow => iced::exit(),
            Message::Tick => {
                self.pulse = !self.pulse;
                if self
                    .notice
                    .as_ref()
                    .is_some_and(|notice| notice.since.elapsed() > TOAST_LIFETIME)
                {
                    self.notice = None;
                }
                if self.editing.is_none() {
                    self.send(Request::GetStatus)
                } else {
                    Task::none()
                }
            }
            Message::IpcResult(response) => self.handle_response(*response),
            Message::ToggleEnabled(enabled) => self.send(Request::SetEnabled { enabled }),
            Message::SelectProfile(name) => Task::batch([
                self.send(Request::SetProfile { name }),
                self.send(Request::GetStatus),
            ]),
            Message::EditButton(button) => {
                if self.loaded_profile.is_some() {
                    self.editing = Some(button);
                    self.notice = None;
                }
                Task::none()
            }
            Message::ClearButton(button) => {
                let Some(profile) = self.loaded_profile.as_mut() else {
                    return Task::none();
                };
                let before = profile.buttons.len();
                profile.buttons.retain(|mapping| mapping.button != button);
                if profile.buttons.len() == before {
                    return Task::none();
                }
                let request = Request::SaveProfile {
                    profile: profile.clone(),
                };
                self.send(request)
            }
            Message::KeyPressed(key, modifiers) => {
                if matches!(key, Key::Named(iced::keyboard::key::Named::Escape)) {
                    if self.confirm_delete.is_some() {
                        self.confirm_delete = None;
                        return Task::none();
                    }
                    if self.editing.is_some() {
                        self.editing = None;
                        return Task::none();
                    }
                    if self
                        .status
                        .as_ref()
                        .is_some_and(|status| status.calibrating)
                    {
                        return self.update(Message::CancelCalibration);
                    }
                }
                self.capture_key(key, modifiers)
            }
            Message::StartCalibration => {
                self.notice = None;
                self.send(Request::BeginCalibration)
            }
            Message::CancelCalibration => {
                self.finishing_calibration = false;
                self.send(Request::CancelCalibration)
            }
            Message::NewProfileName(name) => {
                self.new_profile = name;
                Task::none()
            }
            Message::CreateProfile => {
                let name = self.new_profile.trim().to_owned();
                if name.is_empty() {
                    return Task::none();
                }
                self.new_profile.clear();
                let profile = ProfileDto {
                    name,
                    buttons: Vec::new(),
                };
                Task::batch([
                    self.send(Request::SaveProfile { profile }),
                    self.send(Request::GetStatus),
                    self.send(Request::ListProfiles),
                ])
            }
            Message::Refresh => self.refresh_all(),
            Message::ToggleSettings => {
                self.route = match self.route {
                    Route::Main => Route::Settings,
                    Route::Settings => Route::Main,
                };
                Task::none()
            }
            Message::CopyProjectUrl => {
                iced::clipboard::write(crate::settings::PROJECT_URL.to_owned())
            }
            Message::CopyShortcut(command) => iced::clipboard::write(command),
            Message::ProfileHover(name) => {
                self.hovered_profile = name;
                Task::none()
            }
            Message::RequestDeleteProfile(name) => {
                self.confirm_delete = Some(name);
                Task::none()
            }
            Message::ConfirmDeleteProfile => {
                let Some(name) = self.confirm_delete.take() else {
                    return Task::none();
                };
                Task::batch([
                    self.send(Request::DeleteProfile { name }),
                    self.send(Request::ListProfiles),
                    self.send(Request::GetStatus),
                ])
            }
            Message::CancelDeleteProfile => {
                self.confirm_delete = None;
                Task::none()
            }
            Message::SelectLocale(locale) => {
                if settings::LOCALES.contains(&locale.as_str()) {
                    rust_i18n::set_locale(locale.as_str());
                    self.settings.locale = Some(locale);
                    self.settings.save();
                }
                Task::none()
            }
        }
    }

    fn handle_response(&mut self, response: Response) -> Task<Message> {
        match response {
            Response::Status(status) => {
                self.connected = Some(true);
                let previous_profile = self.status.as_ref().and_then(|s| s.profile.clone());
                self.status = Some(status.clone());
                if !status.calibrating {
                    self.finishing_calibration = false;
                }
                let mut tasks = Vec::new();
                if status.calibrating
                    && status.captured_count == status.total_buttons
                    && !self.finishing_calibration
                {
                    self.finishing_calibration = true;
                    tasks.push(self.send(Request::FinishCalibration));
                }
                if previous_profile != status.profile {
                    tasks.push(self.refresh_all());
                }
                Task::batch(tasks)
            }
            Response::Profiles { names } => {
                self.profiles = names;
                if let Some(active) = self.status.as_ref().and_then(|s| s.profile.clone()) {
                    if self.loaded_profile.as_ref().map(|p| p.name.as_str())
                        != Some(active.as_str())
                    {
                        return self.send(Request::GetProfile { name: active });
                    }
                }
                Task::none()
            }
            Response::Profile { profile } => {
                self.profiles.retain(|name| *name != profile.name || true);
                if !self.profiles.contains(&profile.name) {
                    self.profiles.push(profile.name.clone());
                    self.profiles.sort_unstable();
                }
                self.loaded_profile = Some(profile);
                Task::none()
            }
            Response::Ok => self.refresh_all(),
            Response::Error { message } => {
                if message.starts_with(DAEMON_UNREACHABLE) {
                    self.connected = Some(false);
                } else {
                    self.connected = Some(true);
                    self.toast_error(message);
                }
                Task::none()
            }
        }
    }

    fn capture_key(&mut self, key: Key, modifiers: Modifiers) -> Task<Message> {
        let Some(button) = self.editing else {
            return Task::none();
        };
        if matches!(key, Key::Named(iced::keyboard::key::Named::Escape)) {
            self.editing = None;
            return Task::none();
        }
        let Some(keycode) = self.resolve_keycode(&key) else {
            return Task::none();
        };
        let mut mods = Vec::new();
        if modifiers.control() {
            mods.push(Modifier::LeftCtrl);
        }
        if modifiers.shift() {
            mods.push(Modifier::LeftShift);
        }
        if modifiers.alt() {
            mods.push(Modifier::LeftAlt);
        }
        if modifiers.logo() {
            mods.push(Modifier::LeftMeta);
        }
        let Ok(combination) = KeyCombination::new(mods, keycode) else {
            return Task::none();
        };
        self.editing = None;

        let Some(profile) = self.loaded_profile.as_mut() else {
            return Task::none();
        };
        let dto = ButtonMappingDto {
            button,
            key: key_name(combination.key()).to_owned(),
            modifiers: combination
                .modifiers()
                .iter()
                .map(|modifier| key_name(modifier.key_code()).to_owned())
                .collect(),
        };
        profile.buttons.retain(|mapping| mapping.button != button);
        profile.buttons.push(dto);
        let request = Request::SaveProfile {
            profile: profile.clone(),
        };
        self.send(request)
    }

    /// Resolves an Iced key to a physical code using the active
    /// layout, falling back to the QWERTY table.
    fn resolve_keycode(&self, key: &Key) -> Option<KeyCode> {
        if let Key::Character(characters) = key {
            let mut iterator = characters.chars();
            if let (Some(single), None) = (iterator.next(), iterator.next()) {
                if let Some(code) = self.layout.code_for_char(single) {
                    return Some(code);
                }
                if let Some(code) = self.layout.code_for_char(single.to_ascii_lowercase()) {
                    return Some(code);
                }
            }
        }
        keys::iced_key_to_keycode(key)
    }

    /// Display label for a code: layout character when available,
    /// compact key name otherwise.
    fn key_label(&self, key: KeyCode) -> String {
        match self.layout.label_for(key) {
            Some(character) => character.to_string(),
            None => keys::keycode_label(key),
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        let tick = iced::time::every(POLL_INTERVAL).map(|_| Message::Tick);
        let keyboard = iced::keyboard::listen().filter_map(|event| match event {
            iced::keyboard::Event::KeyPressed { key, modifiers, .. } => {
                Some(Message::KeyPressed(key, modifiers))
            }
            _ => None,
        });
        let resizes =
            iced::window::resize_events().map(|(_window, size)| Message::WindowResized(size.width));
        Subscription::batch([tick, keyboard, resizes])
    }

    fn boot() -> (Self, Task<Message>) {
        let app = App::new();
        let task = Task::batch([
            iced::window::oldest().map(Message::MainWindowReady),
            app.refresh_all(),
        ]);
        (app, task)
    }

    fn view(&self) -> Element<'_, Message> {
        view::render(self)
    }
}

fn key_name(key: KeyCode) -> &'static str {
    key.name().unwrap_or("KEY_UNKNOWN")
}

fn main() -> iced::Result {
    iced::application(App::boot, App::update, App::view)
        .title("Trinity Mapper")
        .subscription(App::subscription)
        .style(|_state: &App, _theme: &iced::Theme| iced::theme::Style {
            background_color: iced::Color::TRANSPARENT,
            text_color: theme::TEXT,
        })
        .font(include_bytes!("../assets/fonts/Inter.ttf"))
        .default_font(iced::Font {
            family: iced::font::Family::Name(theme::FONT_NAME),
            ..iced::Font::DEFAULT
        })
        .window(iced::window::Settings {
            size: iced::Size::new(DEFAULT_WINDOW_WIDTH, DEFAULT_WINDOW_HEIGHT),
            min_size: Some(iced::Size::new(MIN_WINDOW_WIDTH, MIN_WINDOW_HEIGHT)),
            decorations: !wayland_session(),
            transparent: wayland_session(),
            ..Default::default()
        })
        .theme(|_state: &App| iced::Theme::Dark)
        .run()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_switches_at_breakpoint() {
        assert_eq!(layout_for(NARROW_BREAKPOINT), WindowLayout::Wide);
        assert_eq!(layout_for(1080.0), WindowLayout::Wide);
        assert_eq!(layout_for(NARROW_BREAKPOINT - 0.1), WindowLayout::Narrow);
        assert_eq!(layout_for(MIN_WINDOW_WIDTH), WindowLayout::Narrow);
    }
}
