//! Technical adapters: evdev (reading), uinput (injection), TOML
//! (persistence), by-id (discovery).

pub mod evdev_source;
pub mod locator;
pub mod toml_store;
pub mod uinput_sink;

pub use evdev_source::{BY_ID_DIR, EvdevSource, node_caps};
pub use locator::ByIdLocator;
pub use toml_store::{
    TomlCalibrationRepository, TomlProfileRepository, combination_key_name,
    combination_modifier_names, default_calibration_path, default_profiles_dir, profile_from_dto,
    profile_to_dto,
};
pub use uinput_sink::UinputSink;
