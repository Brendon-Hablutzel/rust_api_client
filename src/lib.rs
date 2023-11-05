use cursive::{
    self,
    theme::{BaseColor, Color, PaletteColor, PaletteStyle, Theme},
    traits::*,
    views::{
        Button, Dialog, EditView, LinearLayout, Panel, ResizedView, SelectView, TextArea, TextView,
        ThemedView,
    },
    Cursive,
};
use dotenv::dotenv;
use reqwest::{self, Method};
use serde::Deserialize;
use serde_json;
use std::{env, fs, io::Write, panic};

type RequestError = String;

type FallibleResponse = Result<reqwest::blocking::Response, String>;

#[derive(Deserialize, Debug)]
struct RequestFile {
    requests: Vec<DeserializedRequestEntry>,
}

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
        let method = value.method.as_str();
        let method = match method {
            "GET" => Method::GET,
            "POST" => Method::POST,
            "DELETE" => Method::DELETE,
            "PATCH" => Method::PATCH,
            _ => return Err(format!("Invalid method: {} to {}", method, value.url)),
        };

        Ok(Self {
            url: value.url,
            method,
            body: value.body,
        })
    }
}

#[macro_export]
macro_rules! err_to_string {
    ($fallible:expr) => {
        $fallible.map_err(|err| err.to_string())
    };
}

// Helper function for appending a string to a file
pub fn append_to_file(file_path: &str, data: &str) -> Result<(), String> {
    let mut file = err_to_string!(fs::OpenOptions::new().append(true).open(file_path))?;
    err_to_string!(file.write_all(data.as_bytes()))?;
    Ok(())
}

/// Launch the interactive API client
pub fn interactive(log_file: &Option<String>) {
    dotenv().ok();

    let panic_log_file = env::var("PANIC_LOG").expect("PANIC_LOG env var should be set");

    panic::set_hook(Box::new(move |panic_info| {
        let file = std::fs::File::create(&panic_log_file);
        if let Ok(mut file) = file {
            // nothing we can do if this errors
            let _ = file.write_all(panic_info.to_string().as_bytes());
        }
    }));

    let log_file = log_file.to_owned();

    let client = reqwest::blocking::Client::new();

    let mut siv = cursive::default();

    let select_method = Panel::new(
        SelectView::<Method>::new()
            .item("GET", Method::GET)
            .item("POST", Method::POST)
            .item("DELETE", Method::DELETE)
            .item("PATCH", Method::PATCH)
            .on_submit(move |s, method| on_request_submit(s, method, &client, &log_file))
            .h_align(cursive::align::HAlign::Center)
            .fixed_width(10)
            .with_name("method"),
    );

    let url_label = TextView::new("URL:").h_align(cursive::align::HAlign::Center);

    let url_field = EditView::new().min_width(5).with_name("url");

    let url = Panel::new(
        LinearLayout::vertical()
            .child(url_label)
            .child(url_field)
            .full_width(),
    );

    let top = LinearLayout::horizontal().child(url).child(select_method);

    let mut custom_theme = Theme::default();

    custom_theme.palette[PaletteStyle::EditableTextCursor] = Color::Light(BaseColor::Black).into();
    custom_theme.palette[PaletteStyle::EditableText] = Color::Dark(BaseColor::Blue).into();
    custom_theme.palette[PaletteColor::Secondary] = Color::Dark(BaseColor::White);

    let request_label = TextView::new("Request body:");
    let request_body = ThemedView::new(custom_theme, TextArea::new().with_name("request"));

    let response_label = TextView::new("Response:").with_name("response_label");
    let response_body = TextView::new("").with_name("response");

    let request = Panel::new(
        LinearLayout::vertical()
            .child(request_label)
            .child(request_body)
            .min_size((5, 1))
            .full_screen(),
    );

    let response = Panel::new(
        LinearLayout::vertical()
            .child(response_label)
            .child(response_body)
            .min_size((5, 1))
            .full_screen(),
    );

    let bottom = LinearLayout::horizontal().child(request).child(response);

    let quit_button = Button::new("Quit", Cursive::quit);

    siv.add_fullscreen_layer(
        LinearLayout::vertical()
            .child(top)
            .child(bottom)
            .child(quit_button),
    );

    siv.run();
}

fn on_request_submit(
    s: &mut Cursive,
    method: &Method,
    client: &reqwest::blocking::Client,
    log_file: &Option<String>,
) {
    let url = s
        .find_name::<ResizedView<EditView>>("url")
        .expect("url field should exist")
        .get_inner()
        .get_content();
    let url = url.as_str();

    let request_body = s
        .find_name::<TextArea>("request")
        .expect("request field should exist");
    let request_body = request_body.get_content().to_owned();

    let fallible_response = make_request(client, method, url, request_body);

    // if let Some(log_file) = log_file {
    //     output_to_file(s, fallible_response, method, url, log_file);
    // }

    // output_to_screen(s, fallible_response, method);

    let datetime = chrono::Utc::now().to_rfc3339();

    let output = match fallible_response {
        Ok(success) => {
            let status = success.status();
            format!(
                "{} - {} {} for {} to {}\n{}\n",
                datetime,
                status.as_str(),
                status.canonical_reason().unwrap_or(""),
                method.to_string(),
                url,
                success.text().unwrap_or("".to_string())
            )
        }
        Err(err) => {
            format!("{} - ERROR: {}\n", datetime, err)
        }
    };

    s.call_on_name("response", |view: &mut TextView| {
        view.set_content(&output);
    });

    if let Some(log_file) = log_file {
        match append_to_file(log_file, &output) {
            Err(err) => {
                let dialog = Dialog::around(TextView::new(format!(
                    "Error logging to file {}: {}",
                    log_file, err
                )))
                .button("Cancel", |s| {
                    s.pop_layer();
                });
                s.add_layer(dialog);
            }
            _ => (),
        }
    }
}

fn make_request(
    client: &reqwest::blocking::Client,
    method: &Method,
    url: &str,
    body: String,
) -> FallibleResponse {
    let builder = match *method {
        Method::GET => Ok(client.get(url)),
        Method::POST => Ok(client.post(url).body(body)),
        Method::DELETE => Ok(client.delete(url)),
        Method::PATCH => Ok(client.patch(url)),
        _ => Err("Invalid method".to_string()),
    }?;

    err_to_string!(builder.send())
}

/// Make requests based on the given JSON file
pub fn from_file(
    file_path: &str,
    stop_early_on_fail: bool,
    log_file: &Option<String>,
) -> Result<(), String> {
    let client = reqwest::blocking::Client::new();

    let content = err_to_string!(fs::read_to_string(file_path))?;
    let content = err_to_string!(serde_json::from_str::<RequestFile>(&content))?;

    let request_results = content.requests.into_iter().map(|request| {
        Request::try_from(request).and_then(|request| process_request(&client, request))
    });

    for result in request_results {
        let output = match result {
            Ok(success) => format!("{success}\n"),
            Err(err) => {
                let err_string = format!("ERROR: {err}\n");
                if stop_early_on_fail {
                    return Err(err_string);
                } else {
                    format!("{}", err_string)
                }
            }
        };

        if let Some(log_file) = log_file {
            append_to_file(log_file, &format!("{}\n", &output))?
        }

        println!("{output}");
    }

    Ok(())
}

// an error here represents an error while sending the request,
// not an unsuccessful response code
fn process_request(
    client: &reqwest::blocking::Client,
    request: Request,
) -> Result<String, RequestError> {
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

    let datetime = chrono::Utc::now().to_rfc3339();

    let response = err_to_string!(builder.send())?;

    let status = response.status();

    let response_body = response.text().unwrap_or(String::from("<no body>"));

    Ok(format!(
        "{} - {} {} for {} to {}\n{}",
        datetime,
        status.as_str(),
        status.canonical_reason().unwrap_or(""),
        method.to_string(),
        url,
        response_body
    ))
}
