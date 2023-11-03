use api_client::err_to_string;
use reqwest::Method;
use serde::Deserialize;
use serde_json;
use std::{env, fs};

#[derive(Deserialize, Debug)]
struct DeserializedRequestEntry {
    url: String,
    method: String,
    body: Option<String>,
}

struct Request {
    url: String,
    method: Method,
    body: Option<String>,
}

impl TryFrom<DeserializedRequestEntry> for Request {
    type Error = String;

    fn try_from(value: DeserializedRequestEntry) -> Result<Self, Self::Error> {
        let method = match value.method.as_str() {
            "GET" => Method::GET,
            "POST" => Method::POST,
            "DELETE" => Method::DELETE,
            "PATCH" => Method::PATCH,
            _ => return Err("Invalid method".to_string()),
        };

        Ok(Self {
            url: value.url,
            method,
            body: value.body,
        })
    }
}

#[derive(Deserialize, Debug)]
struct RequestFile {
    requests: Vec<DeserializedRequestEntry>,
}

fn main() -> Result<(), String> {
    let mut args = env::args();
    args.next(); // skip first argument

    let file_path = args.next().ok_or("No file path specified".to_string())?;

    let content = err_to_string!(fs::read_to_string(file_path))?;
    let content = err_to_string!(serde_json::from_str::<RequestFile>(&content))?;

    let client = reqwest::blocking::Client::new();

    content
        .requests
        .into_iter()
        .map(|request| {
            Request::try_from(request).and_then(|request| process_request(&client, request))
        })
        .for_each(|result| match result {
            Ok(success) => println!("{}", success),
            Err(err) => println!("ERROR: {err}"),
        });

    Ok(())
}

fn process_request(client: &reqwest::blocking::Client, request: Request) -> Result<String, String> {
    let url = request.url;
    let body = request.body.unwrap_or(String::new());
    let method = request.method;

    let builder = match method {
        Method::GET => client.get(&url),
        Method::POST => client.post(&url).body(body),
        Method::DELETE => client.delete(&url),
        Method::PATCH => client.patch(&url),
        _ => return Err("Invalid method".to_string()),
    };

    let response = err_to_string!(builder.send())?;

    let status = response.status();

    let response_body = response.text().unwrap_or(String::from("<no body>"));

    Ok(format!(
        "{} {} for {} to {}\n{}\n",
        status.as_str(),
        status.canonical_reason().unwrap_or(""),
        method.to_string(),
        url,
        response_body
    ))
}
