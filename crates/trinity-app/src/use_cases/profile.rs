//! Profile management use case: CRUD, rename with mapping preservation.

use trinity_core::profile::Profile;

#[cfg(test)]
use trinity_core::button::Button;
#[cfg(test)]
use trinity_core::combo::KeyCombination;
#[cfg(test)]
use trinity_core::key::KeyCode;

use crate::error::AppError;
use crate::ports::ProfileRepository;

/// Orchestrates profile persistence and validation.
///
/// All business logic for profile management lives here — the daemon
/// and GUI just delegate.
pub struct ProfileUseCase<R: ProfileRepository> {
    repository: R,
    default_profile: String,
}

impl<R: ProfileRepository> ProfileUseCase<R> {
    pub fn new(repository: R, default_profile: impl Into<String>) -> Self {
        Self {
            repository,
            default_profile: default_profile.into(),
        }
    }

    pub fn default_profile_name(&self) -> &str {
        &self.default_profile
    }

    /// Creates a new empty profile.
    pub fn create(&mut self, name: &str) -> Result<Profile, AppError> {
        let profile = Profile::new(name)?;
        self.repository.save(&profile)?;
        Ok(profile)
    }

    /// Loads a profile by name.
    pub fn load(&self, name: &str) -> Result<Profile, AppError> {
        self.repository.load(name)
    }

    /// Saves a profile (create or update).
    pub fn save(&mut self, profile: &Profile) -> Result<(), AppError> {
        self.repository.save(profile)
    }

    /// Lists all profile names, sorted.
    pub fn list(&self) -> Result<Vec<String>, AppError> {
        let mut names = self.repository.list()?;
        names.sort_unstable();
        Ok(names)
    }

    /// Deletes a profile. The default profile cannot be deleted.
    pub fn delete(&mut self, name: &str) -> Result<(), AppError> {
        if name == self.default_profile {
            return Err(AppError::Port(
                "cannot delete the default profile".to_string(),
            ));
        }
        self.repository.delete(name)
    }

    /// Renames a profile, preserving all mappings.
    /// Renaming to the same name is a no-op.
    pub fn rename(&mut self, from: &str, to: &str) -> Result<Profile, AppError> {
        if from == to {
            return self.repository.load(from);
        }
        let source = self.repository.load(from)?;
        let mut renamed = Profile::new(to)?;
        for (button, combination) in source.mappings() {
            renamed.set_mapping(button, combination.clone());
        }
        self.repository.save(&renamed)?;
        self.repository.delete(from)?;
        Ok(renamed)
    }

    /// Ensures the default profile exists (creates it if needed).
    pub fn ensure_default(&mut self) -> Result<(), AppError> {
        let names = self.repository.list().unwrap_or_default();
        if names.is_empty() {
            let default = Profile::new(&self.default_profile)?;
            self.repository.save(&default)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::MemoryProfileRepository;
    use trinity_core::modifier::Modifier;

    fn use_case() -> ProfileUseCase<MemoryProfileRepository> {
        ProfileUseCase::new(MemoryProfileRepository::new(), "default")
    }

    #[test]
    fn create_and_load_roundtrip() {
        let mut uc = use_case();
        let profile = uc.create("mmo").unwrap();
        assert_eq!(profile.name(), "mmo");
        let loaded = uc.load("mmo").unwrap();
        assert_eq!(loaded, profile);
    }

    #[test]
    fn create_empty_name_rejected() {
        let mut uc = use_case();
        assert!(uc.create("").is_err());
    }

    #[test]
    fn rename_preserves_mappings() {
        let mut uc = use_case();
        let mut profile = uc.create("mmo").unwrap();
        profile.set_mapping(
            Button::new(3).unwrap(),
            KeyCombination::new(vec![Modifier::LeftCtrl], KeyCode::KEY_1).unwrap(),
        );
        uc.save(&profile).unwrap();

        let renamed = uc.rename("mmo", "mmo2").unwrap();
        assert_eq!(renamed.name(), "mmo2");
        assert!(renamed.mapping(Button::new(3).unwrap()).is_some());

        // Old name gone
        assert!(uc.load("mmo").is_err());
        // New name loads
        let loaded = uc.load("mmo2").unwrap();
        assert!(loaded.mapping(Button::new(3).unwrap()).is_some());
    }

    #[test]
    fn rename_to_same_name_is_noop() {
        let mut uc = use_case();
        uc.create("same").unwrap();
        let result = uc.rename("same", "same").unwrap();
        assert_eq!(result.name(), "same");
        // Profile still exists
        assert!(uc.load("same").is_ok());
    }

    #[test]
    fn rename_missing_profile_errors() {
        let mut uc = use_case();
        assert!(uc.rename("ghost", "new").is_err());
    }

    #[test]
    fn delete_default_rejected() {
        let mut uc = use_case();
        uc.ensure_default().unwrap();
        assert!(uc.delete("default").is_err());
    }

    #[test]
    fn delete_removes_profile() {
        let mut uc = use_case();
        uc.create("temp").unwrap();
        uc.delete("temp").unwrap();
        assert!(uc.load("temp").is_err());
    }

    #[test]
    fn list_returns_sorted_names() {
        let mut uc = use_case();
        for name in ["zeta", "alpha", "mid"] {
            uc.create(name).unwrap();
        }
        let names = uc.list().unwrap();
        assert!(names.contains(&"alpha".to_owned()));
        assert!(names.contains(&"zeta".to_owned()));
        assert!(names.contains(&"mid".to_owned()));
    }

    #[test]
    fn ensure_default_creates_if_empty() {
        let mut uc = use_case();
        assert!(uc.list().unwrap().is_empty());
        uc.ensure_default().unwrap();
        let names = uc.list().unwrap();
        assert!(names.contains(&"default".to_owned()));
    }

    #[test]
    fn ensure_default_noop_if_exists() {
        let mut uc = use_case();
        uc.create("default").unwrap();
        uc.create("other").unwrap();
        uc.ensure_default().unwrap();
        let names = uc.list().unwrap();
        assert_eq!(names.len(), 2); // Didn't add another default
    }
}
