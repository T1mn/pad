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
