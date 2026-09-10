//! Dependency injection container: wires infrastructure to use cases.
//!
//! The daemon's composition root creates this once and passes it to the
//! Server — no service locator, no magic, just explicit wiring.

use trinity_app::use_cases::profile::ProfileUseCase;
use trinity_infra::TomlProfileRepository;

use crate::server::ServerConfig;

/// Provides fully-wired use cases with their infrastructure dependencies.
pub struct Container {
    profiles_dir: std::path::PathBuf,
    default_profile: String,
}

impl Container {
    pub fn new(config: &ServerConfig) -> Self {
        Self {
            profiles_dir: config.profiles_dir.clone(),
            default_profile: config.default_profile.clone(),
        }
    }

    /// Profile management use case (repository-backed).
    pub fn profile_use_case(&self) -> ProfileUseCase<TomlProfileRepository> {
        ProfileUseCase::new(
            TomlProfileRepository::new(&self.profiles_dir),
            &self.default_profile,
        )
    }
}
