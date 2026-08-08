mod cli {
    use super::client::send_request;
    use super::model::ApiRequest;
    use std::error::Error;

    pub fn run_args(args: impl IntoIterator<Item = String>) -> Result<(), Box<dyn Error>> {
        let args: Vec<String> = args.into_iter().collect();
        let request = match args.first().map(String::as_str) {
            Some("request") => {
                let raw = args.get(1).ok_or("missing request json")?;
                serde_json::from_str::<ApiRequest>(raw)?
            }
            Some("status") => ApiRequest {
                action: "status".into(),
                ..ApiRequest::default()
            },
            Some("inbox") => ApiRequest {
                action: "inbox".into(),
                ..ApiRequest::default()
            },
            Some(other) => return Err(format!("unknown socket-api command: {other}").into()),
            None => {
                return Err("usage: pad __internal socket-api status|inbox|request <json>".into())
            }
        };
        let response = send_request(&request)?;
        println!("{}", serde_json::to_string_pretty(&response)?);
        Ok(())
    }
}
mod client {
    use super::model::{ApiRequest, ApiResponse};
    use std::io::{self, BufRead, BufReader, Write};
    use std::os::unix::net::UnixStream;

    pub fn send_request(request: &ApiRequest) -> io::Result<ApiResponse> {
        let mut stream = UnixStream::connect(crate::paths::api_socket_path())?;
        let encoded = serde_json::to_string(request)?;
        stream.write_all(encoded.as_bytes())?;
        stream.write_all(b"\n")?;
        let mut line = String::new();
        BufReader::new(stream).read_line(&mut line)?;
        serde_json::from_str::<ApiResponse>(&line).map_err(|err| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("invalid API response: {err}"),
            )
        })
    }
}
mod handler;
mod model;
pub(crate) mod peer;
mod server;
pub(crate) mod socket_file;

pub use cli::run_args;
pub use server::start_api_listener;
