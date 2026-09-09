pub mod engine;
pub mod error;
pub mod event;
pub mod ipc;
pub mod ports;
pub mod session;
pub mod testing;
pub mod translator;

#[cfg(test)]
mod testkit;

pub use engine::{Engine, MIRROR_DEVICE_NAME};
pub use error::AppError;
pub use event::{Action, InputEvent};
pub use ipc::{
    ButtonMappingDto, ProfileDto, Request, Response, Status, decode_request, decode_response,
    encode_request, encode_response,
};
pub use ports::{
    CalibrationRepository, Caps, DeviceLocator, InputEventSource, OutputEventSink,
    ProfileRepository,
};
pub use session::{CalibrationService, CalibrationSession, ProfileService};
pub use translator::Translator;
