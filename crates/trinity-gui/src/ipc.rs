//! Blocking IPC client to the daemon, with an async wrapper for Iced.

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::time::Duration;

use trinity_app::ipc::{Request, Response, decode_response, encode_request};

pub const DAEMON_UNREACHABLE: &str = "daemon unreachable";

/// JSON lines client: one request per connection.
#[derive(Debug, Clone)]
pub struct DaemonClient {
    socket: PathBuf,
}

impl DaemonClient {
    pub fn new(socket: PathBuf) -> Self {
        Self { socket }
    }

    /// Default socket (`$XDG_RUNTIME_DIR/trinity-mapper.sock`).
    pub fn default_socket() -> Self {
        let runtime = std::env::var_os("XDG_RUNTIME_DIR")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("TMPDIR").map(PathBuf::from))
            .unwrap_or_else(|| PathBuf::from("/tmp"));
        Self::new(runtime.join("trinity-mapper.sock"))
    }

    pub fn socket(&self) -> &std::path::Path {
        &self.socket
    }

    pub fn request(&self, request: &Request) -> Response {
        let result = self.request_inner(request);
        match result {
            Ok(response) => response,
            Err(err) => Response::Error {
                message: format!("{DAEMON_UNREACHABLE} : {err}"),
            },
        }
    }

    fn request_inner(&self, request: &Request) -> Result<Response, String> {
        let stream = UnixStream::connect(&self.socket).map_err(|err| err.to_string())?;
        stream
            .set_read_timeout(Some(Duration::from_secs(3)))
            .map_err(|err| err.to_string())?;
        stream
            .set_write_timeout(Some(Duration::from_secs(3)))
            .map_err(|err| err.to_string())?;

        let mut writer = stream.try_clone().map_err(|err| err.to_string())?;
        writeln!(writer, "{}", encode_request(request)).map_err(|err| err.to_string())?;
        writer.flush().map_err(|err| err.to_string())?;

        let mut reader = BufReader::new(stream);
        let mut line = String::new();
        reader.read_line(&mut line).map_err(|err| err.to_string())?;
        if line.trim().is_empty() {
            return Err("empty daemon response".to_string());
        }
        decode_response(line.trim_end()).map_err(|err| err.to_string())
    }
}

/// Async wrapper: runs the request on a dedicated thread.
pub fn request_async(socket: PathBuf, request: Request) -> impl Future<Output = Response> {
    let (sender, receiver) = iced::futures::channel::oneshot::channel();
    std::thread::spawn(move || {
        let client = DaemonClient::new(socket);
        let _ = sender.send(client.request(&request));
    });
    async move {
        match receiver.await {
            Ok(response) => response,
            Err(_) => Response::Error {
                message: "{DAEMON_UNREACHABLE}: worker stopped".to_owned(),
            },
        }
    }
}
