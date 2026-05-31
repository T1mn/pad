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
        None => return Err("usage: pad __internal socket-api status|inbox|request <json>".into()),
    };
    let response = send_request(&request)?;
    println!("{}", serde_json::to_string_pretty(&response)?);
    Ok(())
}
