use std::path::PathBuf;

use trinity_app::ipc::{Request, Response, decode_request, encode_response};
use trinity_daemon::server::{Server, ServerConfig};

fn main() {
    let mut config = ServerConfig::default();
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--socket" => {
                if let Some(value) = args.next() {
                    config = config.with_socket(value);
                }
            }
            "--config-dir" => {
                if let Some(value) = args.next() {
                    let root = PathBuf::from(value);
                    config = config
                        .with_profiles_dir(root.join("profiles"))
                        .with_calibration_path(root.join("calibration.toml"));
                }
            }
            "--profile" => {
                if let Some(value) = args.next() {
                    config.default_profile = value;
                }
            }
            "--device-prefix" => {
                if let Some(value) = args.next() {
                    config.device_prefix = value;
                }
            }
            other => {
                eprintln!("trinity-daemon: unknown option '{other}'");
                eprintln!(
                    "usage: trinity-daemon [--socket PATH] [--config-dir DIR] [--profile NAME] [--device-prefix PREFIX]"
                );
                std::process::exit(2);
            }
        }
    }

    let server = Server::new(config);
    eprintln!(
        "trinity-daemon: starting, socket {}",
        server.socket_path().display()
    );
    if let Err(err) = server.run() {
        eprintln!("trinity-daemon: exiting on error: {err}");
        std::process::exit(1);
    }
    eprintln!("trinity-daemon: clean shutdown");
    let _ = (
        decode_request(""),
        encode_response(&Response::Ok),
        Request::GetStatus,
    );
}
