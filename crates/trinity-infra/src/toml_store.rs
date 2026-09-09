use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use trinity_app::error::AppError;
use trinity_app::ipc::{ButtonMappingDto, ProfileDto};
use trinity_app::ports::{CalibrationRepository, ProfileRepository};
use trinity_core::button::Button;
use trinity_core::calibration::{Calibration, PhysicalCode};
use trinity_core::combo::KeyCombination;
use trinity_core::key::KeyCode;
use trinity_core::modifier::Modifier;
use trinity_core::node::NodeId;
use trinity_core::profile::Profile;

fn port_err(context: &str, err: impl std::fmt::Display) -> AppError {
    AppError::Port(format!("{context} : {err}"))
}

/// Default profile directory (`~/.config/razer-trinity-mapper/profiles`).
pub fn default_profiles_dir() -> PathBuf {
    config_root().join("profiles")
}

/// Default calibration path.
pub fn default_calibration_path() -> PathBuf {
    config_root().join("calibration.toml")
}

fn config_root() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("razer-trinity-mapper")
}

fn key_name(key: KeyCode) -> Result<&'static str, AppError> {
    key.name()
        .ok_or_else(|| AppError::Port(format!("key without a known name: {}", key.label())))
}

fn parse_key(name: &str) -> Result<KeyCode, AppError> {
    KeyCode::from_name(name).map_err(AppError::from)
}

fn parse_modifier(name: &str) -> Result<Modifier, AppError> {
    let key = parse_key(name)?;
    Modifier::from_key_code(key)
        .ok_or_else(|| AppError::Port(format!("'{name}' is not a modifier")))
}

/// Canonical name of the target key of a combination.
pub fn combination_key_name(key: KeyCode) -> Result<&'static str, AppError> {
    key_name(key)
}

/// Canonical names of the modifiers of a combination.
pub fn combination_modifier_names(
    combination: &KeyCombination,
) -> Result<Vec<&'static str>, AppError> {
    combination
        .modifiers()
        .iter()
        .map(|modifier| key_name(modifier.key_code()))
        .collect()
}

/// Domain to IPC DTO conversion.
pub fn profile_to_dto(profile: &Profile) -> Result<ProfileDto, AppError> {
    let mut buttons = Vec::new();
    for (button, combination) in profile.mappings() {
        buttons.push(ButtonMappingDto {
            button: button.number(),
            key: combination_key_name(combination.key())?.to_owned(),
            modifiers: combination_modifier_names(combination)?
                .into_iter()
                .map(str::to_owned)
                .collect(),
        });
    }
    Ok(ProfileDto {
        name: profile.name().to_owned(),
        buttons,
    })
}

/// IPC DTO to domain conversion (validates names and modifiers).
pub fn profile_from_dto(dto: &ProfileDto) -> Result<Profile, AppError> {
    let mut profile = Profile::new(&dto.name)?;
    for mapping in &dto.buttons {
        let mut modifiers = Vec::with_capacity(mapping.modifiers.len());
        for name in &mapping.modifiers {
            modifiers.push(parse_modifier(name)?);
        }
        let combination = KeyCombination::new(modifiers, parse_key(&mapping.key)?)?;
        profile.set_mapping(Button::new(mapping.button)?, combination);
    }
    Ok(profile)
}

fn validate_profile_name(name: &str) -> Result<(), AppError> {
    if name.is_empty()
        || name.contains('/')
        || name.contains('\\')
        || name.contains('\0')
        || name.trim() != name
        || name == "."
        || name == ".."
    {
        return Err(AppError::Port(format!("invalid profile name: '{name}'")));
    }
    Ok(())
}

#[derive(serde::Deserialize, serde::Serialize)]
struct ProfileFile {
    name: String,
    #[serde(default)]
    buttons: BTreeMap<String, ButtonEntry>,
}

#[derive(serde::Deserialize, serde::Serialize)]
struct ButtonEntry {
    key: String,
    #[serde(default)]
    modifiers: Vec<String>,
}

/// Profile repository: one TOML file per profile in a directory.
pub struct TomlProfileRepository {
    dir: PathBuf,
}

impl TomlProfileRepository {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    pub fn default_repository() -> Self {
        Self::new(default_profiles_dir())
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    fn path_for(&self, name: &str) -> Result<PathBuf, AppError> {
        validate_profile_name(name)?;
        Ok(self.dir.join(format!("{name}.toml")))
    }
}

impl ProfileRepository for TomlProfileRepository {
    fn list(&self) -> Result<Vec<String>, AppError> {
        let entries =
            fs::read_dir(&self.dir).map_err(|err| port_err("reading profile directory", err))?;
        let mut names = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|err| port_err("reading profile directory", err))?;
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "toml") {
                if let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) {
                    names.push(stem.to_owned());
                }
            }
        }
        names.sort_unstable();
        Ok(names)
    }

    fn load(&self, name: &str) -> Result<Profile, AppError> {
        let path = self.path_for(name)?;
        let content = fs::read_to_string(&path)
            .map_err(|_| AppError::NotFound(format!("profile '{name}'")))?;
        let file: ProfileFile =
            toml::from_str(&content).map_err(|err| port_err("invalid profile file", err))?;
        let mut profile = Profile::new(&file.name)?;
        for (button, entry) in &file.buttons {
            let number: u8 = button
                .parse()
                .map_err(|_| AppError::Port(format!("invalid button number: '{button}'")))?;
            let mut modifiers = Vec::with_capacity(entry.modifiers.len());
            for modifier in &entry.modifiers {
                modifiers.push(parse_modifier(modifier)?);
            }
            let combination = KeyCombination::new(modifiers, parse_key(&entry.key)?)?;
            profile.set_mapping(Button::new(number)?, combination);
        }
        Ok(profile)
    }

    fn save(&mut self, profile: &Profile) -> Result<(), AppError> {
        let path = self.path_for(profile.name())?;
        let mut buttons = BTreeMap::new();
        for (button, combination) in profile.mappings() {
            buttons.insert(
                button.number().to_string(),
                ButtonEntry {
                    key: key_name(combination.key())?.to_owned(),
                    modifiers: combination
                        .modifiers()
                        .iter()
                        .map(|modifier| key_name(modifier.key_code()).map(str::to_owned))
                        .collect::<Result<Vec<_>, _>>()?,
                },
            );
        }
        let file = ProfileFile {
            name: profile.name().to_owned(),
            buttons,
        };
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|err| port_err("creating profile directory", err))?;
        }
        let content =
            toml::to_string_pretty(&file).map_err(|err| port_err("profile serialization", err))?;
        fs::write(&path, content).map_err(|err| port_err("writing profile", err))?;
        Ok(())
    }

    fn delete(&mut self, name: &str) -> Result<(), AppError> {
        let path = self.path_for(name)?;
        fs::remove_file(&path).map_err(|_| AppError::NotFound(format!("profile '{name}'")))
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
struct CalibrationFile {
    buttons: BTreeMap<String, CalibrationEntry>,
}

#[derive(serde::Deserialize, serde::Serialize)]
struct CalibrationEntry {
    node: String,
    code: u16,
}

/// Calibration repository: a single TOML file.
pub struct TomlCalibrationRepository {
    path: PathBuf,
}

impl TomlCalibrationRepository {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn default_repository() -> Self {
        Self::new(default_calibration_path())
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl CalibrationRepository for TomlCalibrationRepository {
    fn load(&self) -> Result<Option<Calibration>, AppError> {
        let Ok(content) = fs::read_to_string(&self.path) else {
            return Ok(None);
        };
        let file: CalibrationFile =
            toml::from_str(&content).map_err(|err| port_err("invalid calibration file", err))?;
        let mut calibration = Calibration::new();
        for (button, entry) in &file.buttons {
            let number: u8 = button
                .parse()
                .map_err(|_| AppError::Port(format!("invalid button number: '{button}'")))?;
            calibration.record(
                Button::new(number)?,
                PhysicalCode::new(NodeId::new(&entry.node)?, entry.code),
            )?;
        }
        Ok(Some(calibration))
    }

    fn save(&mut self, calibration: &Calibration) -> Result<(), AppError> {
        let mut buttons = BTreeMap::new();
        for (button, code) in calibration.buttons() {
            buttons.insert(
                button.number().to_string(),
                CalibrationEntry {
                    node: code.node().as_str().to_owned(),
                    code: code.code(),
                },
            );
        }
        let file = CalibrationFile { buttons };
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|err| port_err("creating config directory", err))?;
        }
        let content = toml::to_string_pretty(&file)
            .map_err(|err| port_err("calibration serialization", err))?;
        fs::write(&self.path, content).map_err(|err| port_err("writing calibration", err))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use trinity_core::modifier::Modifier;

    struct TempDir(PathBuf);

    impl TempDir {
        fn new(tag: &str) -> Self {
            let dir = std::env::temp_dir().join(format!(
                "trinity-infra-{tag}-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn sample_profile() -> Profile {
        let mut profile = Profile::new("mmo").unwrap();
        profile.set_mapping(
            Button::new(1).unwrap(),
            KeyCombination::plain(KeyCode::KEY_F),
        );
        profile.set_mapping(
            Button::new(3).unwrap(),
            KeyCombination::new(vec![Modifier::LeftCtrl], KeyCode::KEY_1).unwrap(),
        );
        profile
    }

    #[test]
    fn profile_roundtrip() {
        let temp = TempDir::new("profile");
        let mut repo = TomlProfileRepository::new(&temp.0);
        let profile = sample_profile();
        repo.save(&profile).unwrap();

        assert_eq!(repo.list().unwrap(), vec!["mmo".to_owned()]);
        assert_eq!(repo.load("mmo").unwrap(), profile);
        let raw = fs::read_to_string(temp.0.join("mmo.toml")).unwrap();
        assert!(raw.contains("key = \"KEY_1\""));
        assert!(raw.contains("\"KEY_LEFTCTRL\""));

        repo.delete("mmo").unwrap();
        assert!(repo.list().unwrap().is_empty());
        assert!(matches!(
            repo.load("mmo").unwrap_err(),
            AppError::NotFound(_)
        ));
    }

    #[test]
    fn invalid_profile_names_are_rejected() {
        let temp = TempDir::new("names");
        let mut repo = TomlProfileRepository::new(&temp.0);
        assert!(matches!(
            repo.load("../escape").unwrap_err(),
            AppError::Port(_)
        ));
        assert!(repo.save(&Profile::new("a/b").unwrap()).is_err());
    }

    #[test]
    fn unknown_key_name_is_reported() {
        let temp = TempDir::new("badkey");
        let repo = TomlProfileRepository::new(&temp.0);
        fs::write(
            temp.0.join("bad.toml"),
            "name = \"bad\"\n[buttons.\"1\"]\nkey = \"KEY_NOPE\"\n",
        )
        .unwrap();
        assert!(matches!(repo.load("bad").unwrap_err(), AppError::Domain(_)));
    }

    #[test]
    fn calibration_roundtrip() {
        let temp = TempDir::new("calibration");
        let path = temp.0.join("calibration.toml");
        let mut repo = TomlCalibrationRepository::new(&path);

        assert_eq!(repo.load().unwrap(), None);

        let node = NodeId::new("usb-Razer_Razer_Naga_Trinity_00000000001A-if01-event-kbd").unwrap();
        let mut calibration = Calibration::new();
        for button in trinity_core::Button::ALL {
            calibration
                .record(
                    button,
                    PhysicalCode::new(node.clone(), 40 + u16::from(button.number())),
                )
                .unwrap();
        }
        repo.save(&calibration).unwrap();
        assert_eq!(repo.load().unwrap(), Some(calibration));
    }

    #[test]
    fn calibration_duplicate_code_is_rejected_on_load() {
        let temp = TempDir::new("dupcal");
        let path = temp.0.join("calibration.toml");
        fs::write(
            &path,
            "[buttons.\"1\"]\nnode = \"n1\"\ncode = 42\n[buttons.\"2\"]\nnode = \"n1\"\ncode = 42\n",
        )
        .unwrap();
        let repo = TomlCalibrationRepository::new(&path);
        assert!(matches!(repo.load().unwrap_err(), AppError::Domain(_)));
    }
}
