use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::process::ExitCode;

use trinity_app::ipc::{Request, Response, decode_response, encode_request};

fn socket_path() -> PathBuf {
    std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("TMPDIR").map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("trinity-mapper.sock")
}

fn send(request: Request) -> Result<Response, String> {
    let stream = UnixStream::connect(socket_path()).map_err(|e| e.to_string())?;
    let mut writer = stream.try_clone().map_err(|e| e.to_string())?;
    writeln!(writer, "{}", encode_request(&request)).map_err(|e| e.to_string())?;
    let mut line = String::new();
    BufReader::new(stream)
        .read_line(&mut line)
        .map_err(|e| e.to_string())?;
    decode_response(line.trim_end())
}

fn print_response(response: Response) {
    match response {
        Response::Ok => println!("ok"),
        Response::Status(status) => {
            println!(
                "device:  {}",
                if status.device_present {
                    "present"
                } else {
                    "absent"
                }
            );
            println!("active:  {}", if status.enabled { "yes" } else { "no" });
            println!("profile: {}", status.profile.as_deref().unwrap_or("none"));
            println!(
                "calibrated: {}",
                if status.calibrated { "yes" } else { "no" }
            );
            if let Some(error) = status.error {
                println!("error:   {error}");
            }
        }
        Response::Profiles { names } => {
            for name in names {
                println!("{name}");
            }
        }
        Response::Error { message } => {
            eprintln!("error: {message}");
        }
        _ => println!("ok"),
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let request = match args.as_slice() {
        [cmd, name] if cmd == "profile" => Request::SetProfile { name: name.clone() },
        [cmd] if cmd == "status" => Request::GetStatus,
        [cmd] if cmd == "profiles" => Request::ListProfiles,
        [cmd] if cmd == "toggle" => {
            println!(
                "tip: toggle requires knowing the current state; use `status` then `profile <name>`"
            );
            return ExitCode::FAILURE;
        }
        _ => {
            eprintln!("usage: trinity-ctl <command> [args]");
            eprintln!();
            eprintln!("Commands:");
            eprintln!("  status             Show daemon status");
            eprintln!("  profiles           List available profiles");
            eprintln!("  profile <name>     Switch to a profile");
            return ExitCode::FAILURE;
        }
    };

    match send(request) {
        Ok(response) => {
            print_response(response);
            ExitCode::SUCCESS
        }
        Err(message) => {
            eprintln!("error: {message}");
            eprintln!("hint: is trinity-daemon running?");
            ExitCode::FAILURE
        }
    }
}
