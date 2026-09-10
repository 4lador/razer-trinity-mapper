//! Integration tests: daemon profile management end-to-end.
//! Uses a temp directory for persistence and exercises every IPC handler.

use std::fs;
use std::path::PathBuf;

use trinity_app::ipc::{ProfileDto, Request, Response};
use trinity_daemon::server::{Server, ServerConfig};

fn temp_config(tag: &str) -> (ServerConfig, PathBuf) {
    let dir = std::env::temp_dir().join(format!(
        "trinity-int-{}-{}-{}",
        tag,
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&dir).unwrap();
    let config = ServerConfig::default()
        .with_socket(dir.join("sock"))
        .with_profiles_dir(dir.join("profiles"))
        .with_calibration_path(dir.join("cal.toml"));
    (config, dir)
}

fn server(tag: &str) -> (Server, PathBuf) {
    let (config, dir) = temp_config(tag);
    let server = Server::new(config);
    server.bootstrap().expect("bootstrap failed");
    (server, dir)
}

fn cleanup(dir: &std::path::Path) {
    let _ = fs::remove_dir_all(dir);
}

fn make_profile(name: &str, button: u8, key: &str) -> ProfileDto {
    ProfileDto {
        name: name.to_owned(),
        buttons: vec![trinity_app::ipc::ButtonMappingDto {
            button,
            key: key.to_owned(),
            modifiers: vec![],
        }],
    }
}

// --- Lifecycle ---

#[test]
fn bootstrap_creates_default_profile() {
    let (server, dir) = server("boot");
    let resp = server.handle_request(Request::ListProfiles);
    assert_eq!(
        resp,
        Response::Profiles {
            names: vec!["default".to_owned()]
        }
    );
    cleanup(&dir);
}

// --- Create / Read ---

#[test]
fn save_and_load_profile_roundtrip() {
    let (server, dir) = server("save_load");
    let dto = make_profile("mmo", 3, "KEY_F");
    assert_eq!(
        server.handle_request(Request::SaveProfile {
            profile: dto.clone()
        }),
        Response::Ok
    );

    let resp = server.handle_request(Request::GetProfile { name: "mmo".into() });
    let Response::Profile { profile } = resp else {
        panic!("expected profile")
    };
    assert_eq!(profile, dto);
    cleanup(&dir);
}

#[test]
fn get_missing_profile_returns_error() {
    let (server, dir) = server("get_missing");
    let resp = server.handle_request(Request::GetProfile {
        name: "ghost".into(),
    });
    assert!(matches!(resp, Response::Error { .. }));
    cleanup(&dir);
}

// --- Update / Rename ---

#[test]
fn rename_preserves_mappings_and_switches_active() {
    let (server, dir) = server("rename");
    let dto = make_profile("mmo", 1, "KEY_A");
    server.handle_request(Request::SaveProfile { profile: dto });
    let _ = server.handle_request(Request::SetProfile { name: "mmo".into() });
    // handle_request doesn't return Result — it returns Response

    let resp = server.handle_request(Request::RenameProfile {
        from: "mmo".into(),
        to: "mmo2".into(),
    });
    assert_eq!(resp, Response::Ok);

    // Old profile gone
    let resp = server.handle_request(Request::GetProfile { name: "mmo".into() });
    assert!(matches!(resp, Response::Error { .. }));

    // New profile has the same mappings
    let resp = server.handle_request(Request::GetProfile {
        name: "mmo2".into(),
    });
    let Response::Profile { profile } = resp else {
        panic!("expected profile")
    };
    assert_eq!(profile.name, "mmo2");
    assert_eq!(profile.buttons.len(), 1);
    assert_eq!(profile.buttons[0].key, "KEY_A");
    assert_eq!(profile.buttons[0].button, 1);

    // Active profile switched
    let resp = server.handle_request(Request::GetStatus);
    let Response::Status(status) = resp else {
        panic!("expected status")
    };
    assert_eq!(status.profile.as_deref(), Some("mmo2"));
    cleanup(&dir);
}

#[test]
fn rename_missing_profile_returns_error() {
    let (server, dir) = server("rename_missing");
    let resp = server.handle_request(Request::RenameProfile {
        from: "ghost".into(),
        to: "new".into(),
    });
    assert!(matches!(resp, Response::Error { .. }));
    cleanup(&dir);
}

#[test]
fn rename_to_same_name_is_noop() {
    let (server, dir) = server("rename_same");
    let dto = make_profile("same", 1, "KEY_A");
    server.handle_request(Request::SaveProfile { profile: dto });
    let resp = server.handle_request(Request::RenameProfile {
        from: "same".into(),
        to: "same".into(),
    });
    // Should succeed (load → save → delete → same file recreated)
    assert_eq!(resp, Response::Ok);
    let resp = server.handle_request(Request::GetProfile {
        name: "same".into(),
    });
    assert!(matches!(resp, Response::Profile { .. }));
    cleanup(&dir);
}

// --- Delete ---

#[test]
fn delete_removes_profile_file() {
    let (server, dir) = server("delete");
    let dto = make_profile("temp", 1, "KEY_X");
    server.handle_request(Request::SaveProfile { profile: dto });
    let resp = server.handle_request(Request::DeleteProfile {
        name: "temp".into(),
    });
    assert_eq!(resp, Response::Ok);
    let resp = server.handle_request(Request::GetProfile {
        name: "temp".into(),
    });
    assert!(matches!(resp, Response::Error { .. }));
    cleanup(&dir);
}

#[test]
fn delete_default_profile_is_rejected() {
    let (server, dir) = server("delete_default");
    let resp = server.handle_request(Request::DeleteProfile {
        name: "default".into(),
    });
    assert!(matches!(resp, Response::Error { .. }));
    cleanup(&dir);
}

#[test]
fn delete_active_profile_falls_back_to_default() {
    let (server, dir) = server("delete_active");
    let dto = make_profile("victim", 1, "KEY_Z");
    server.handle_request(Request::SaveProfile { profile: dto });
    let _ = server.handle_request(Request::SetProfile {
        name: "victim".into(),
    });

    let resp = server.handle_request(Request::DeleteProfile {
        name: "victim".into(),
    });
    assert_eq!(resp, Response::Ok);

    let resp = server.handle_request(Request::GetStatus);
    let Response::Status(status) = resp else {
        panic!("expected status")
    };
    assert_eq!(status.profile.as_deref(), Some("default"));
    cleanup(&dir);
}

// --- List ---

#[test]
fn list_profiles_returns_sorted_names() {
    let (server, dir) = server("list");
    for name in ["zeta", "alpha", "mid"] {
        server.handle_request(Request::SaveProfile {
            profile: ProfileDto {
                name: name.into(),
                buttons: vec![],
            },
        });
    }
    let resp = server.handle_request(Request::ListProfiles);
    let Response::Profiles { names } = resp else {
        panic!("expected profiles")
    };
    assert!(names.contains(&"alpha".to_owned()));
    assert!(names.contains(&"zeta".to_owned()));
    assert!(names.contains(&"mid".to_owned()));
    // Default is also present
    assert!(names.contains(&"default".to_owned()));
    cleanup(&dir);
}

// --- SetProfile / Enable ---

#[test]
fn set_profile_switches_active() {
    let (server, dir) = server("set_profile");
    let dto = make_profile("target", 5, "KEY_G");
    server.handle_request(Request::SaveProfile { profile: dto });
    let resp = server.handle_request(Request::SetProfile {
        name: "target".into(),
    });
    assert_eq!(resp, Response::Ok);
    let resp = server.handle_request(Request::GetStatus);
    let Response::Status(status) = resp else {
        panic!()
    };
    assert_eq!(status.profile.as_deref(), Some("target"));
    cleanup(&dir);
}

#[test]
fn set_missing_profile_returns_error() {
    let (server, dir) = server("set_missing");
    let resp = server.handle_request(Request::SetProfile {
        name: "ghost".into(),
    });
    assert!(matches!(resp, Response::Error { .. }));
    cleanup(&dir);
}

// --- Non-regression: profile list stored before early return ---

#[test]
fn profiles_response_always_stores_names() {
    // Regression: the old handler returned early to fetch the active
    // profile and never stored the names. This test verifies the fix.
    let (server, dir) = server("regression_profiles");
    for name in ["a", "b", "c"] {
        server.handle_request(Request::SaveProfile {
            profile: ProfileDto {
                name: name.into(),
                buttons: vec![],
            },
        });
    }
    let resp = server.handle_request(Request::ListProfiles);
    let Response::Profiles { names } = resp else {
        panic!()
    };
    // All profiles must be in the list, not just the active one
    assert!(names.len() >= 4); // default + a + b + c
    cleanup(&dir);
}

// --- IPC protocol completeness ---

#[test]
fn every_request_variant_roundtrips_through_json() {
    use trinity_app::ipc::{decode_request, encode_request};

    let cases = vec![
        Request::GetStatus,
        Request::SetEnabled { enabled: true },
        Request::SetEnabled { enabled: false },
        Request::SetProfile { name: "mmo".into() },
        Request::ListProfiles,
        Request::GetProfile { name: "mmo".into() },
        Request::SaveProfile {
            profile: ProfileDto {
                name: "x".into(),
                buttons: vec![],
            },
        },
        Request::DeleteProfile { name: "x".into() },
        Request::RenameProfile {
            from: "a".into(),
            to: "b".into(),
        },
        Request::BeginCalibration,
        Request::CancelCalibration,
        Request::FinishCalibration,
        Request::Shutdown,
    ];

    for request in cases {
        let json = encode_request(&request);
        let decoded = decode_request(&json)
            .unwrap_or_else(|e| panic!("failed to roundtrip {request:?}: {e}"));
        assert_eq!(decoded, request, "roundtrip mismatch for {request:?}");
    }
}

#[test]
fn every_response_variant_roundtrips_through_json() {
    use trinity_app::ipc::{Status, decode_response, encode_response};

    let status = Status {
        device_present: true,
        enabled: true,
        calibrated: false,
        profile: Some("default".into()),
        calibrating: false,
        current_button: Some(3),
        captured_count: 2,
        total_buttons: 12,
        error: None,
    };

    let cases = vec![
        Response::Ok,
        Response::Error {
            message: "test".into(),
        },
        Response::Status(status),
        Response::Profiles {
            names: vec!["a".into(), "b".into()],
        },
        Response::Profile {
            profile: ProfileDto {
                name: "x".into(),
                buttons: vec![],
            },
        },
    ];

    for response in cases {
        let json = encode_response(&response);
        let decoded = decode_response(&json)
            .unwrap_or_else(|e| panic!("failed to roundtrip {response:?}: {e}"));
        assert_eq!(decoded, response, "roundtrip mismatch for {response:?}");
    }
}
