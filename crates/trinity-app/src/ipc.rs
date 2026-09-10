//! IPC protocol: JSON lines over a Unix socket, shared GUI/daemon contract.

use serde::{Deserialize, Serialize};

/// Request sent to the daemon.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Request {
    GetStatus,
    SetEnabled { enabled: bool },
    ListProfiles,
    GetProfile { name: String },
    SaveProfile { profile: ProfileDto },
    SetProfile { name: String },
    DeleteProfile { name: String },
    RenameProfile { from: String, to: String },
    BeginCalibration,
    CancelCalibration,
    FinishCalibration,
    Shutdown,
}

/// Response from the daemon.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Response {
    Ok,
    Error { message: String },
    Status(Status),
    Profiles { names: Vec<String> },
    Profile { profile: ProfileDto },
}

/// Snapshot of the daemon state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Status {
    pub device_present: bool,
    pub enabled: bool,
    pub calibrated: bool,
    pub profile: Option<String>,
    pub calibrating: bool,
    pub current_button: Option<u8>,
    pub captured_count: usize,
    pub total_buttons: usize,
    pub error: Option<String>,
}

/// IPC-serializable profile (domain types do not depend on serde).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ProfileDto {
    pub name: String,
    pub buttons: Vec<ButtonMappingDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ButtonMappingDto {
    pub button: u8,
    pub key: String,
    pub modifiers: Vec<String>,
}

pub fn decode_request(line: &str) -> Result<Request, String> {
    serde_json::from_str(line).map_err(|err| err.to_string())
}

pub fn encode_request(request: &Request) -> String {
    serde_json::to_string(request).unwrap_or_else(|_| "{}".to_owned())
}

pub fn decode_response(line: &str) -> Result<Response, String> {
    serde_json::from_str(line).map_err(|err| err.to_string())
}

pub fn encode_response(response: &Response) -> String {
    serde_json::to_string(response).unwrap_or_else(|_| {
        "{\"type\":\"error\",\"message\":\"unable to encode response\"}".to_owned()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requests_roundtrip() {
        let cases: Vec<Request> = vec![
            Request::GetStatus,
            Request::SetEnabled { enabled: false },
            Request::SetProfile { name: "mmo".into() },
            Request::ListProfiles,
            Request::GetProfile { name: "mmo".into() },
            Request::SaveProfile {
                profile: ProfileDto {
                    name: "mmo".into(),
                    buttons: vec![ButtonMappingDto {
                        button: 3,
                        key: "KEY_1".into(),
                        modifiers: vec!["KEY_LEFTCTRL".into()],
                    }],
                },
            },
            Request::RenameProfile {
                from: "mmo".into(),
                to: "mmo2".into(),
            },
            Request::BeginCalibration,
            Request::CancelCalibration,
            Request::FinishCalibration,
            Request::Shutdown,
        ];
        for request in cases {
            let json = encode_request(&request);
            assert_eq!(decode_request(&json).unwrap(), request);
        }
    }

    #[test]
    fn requests_use_snake_case_tags() {
        assert_eq!(
            decode_request("{\"type\":\"set_profile\",\"name\":\"mmo\"}").unwrap(),
            Request::SetProfile { name: "mmo".into() }
        );
        assert_eq!(
            decode_request("{\"type\":\"set_enabled\",\"enabled\":true}").unwrap(),
            Request::SetEnabled { enabled: true }
        );
    }

    #[test]
    fn responses_roundtrip() {
        let status = Status {
            device_present: true,
            enabled: true,
            calibrated: false,
            profile: Some("default".into()),
            calibrating: true,
            current_button: Some(3),
            captured_count: 2,
            total_buttons: 12,
            error: None,
        };
        let response = Response::Status(status);
        let json = encode_response(&response);
        assert!(json.contains("\"captured_count\":2"));
        let decoded: Response = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, response);
    }

    #[test]
    fn profile_dto_uses_snake_case() {
        let json = serde_json::to_string(&ProfileDto {
            name: "mmo".into(),
            buttons: vec![ButtonMappingDto {
                button: 1,
                key: "KEY_F".into(),
                modifiers: vec![],
            }],
        })
        .unwrap();
        assert!(json.contains("\"button\":1"));
        assert!(json.contains("\"modifiers\":[]"));
    }

    #[test]
    fn invalid_request_yields_error_message() {
        assert!(decode_request("this is not json").is_err());
        assert!(decode_request("{\"type\":\"inconnu\"}").is_err());
    }
}
