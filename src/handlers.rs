use std::io::Read;
use axum::{http::StatusCode, response::Html, Form};

use serde::Deserialize;

#[derive(Deserialize)]
pub struct DownloadReq {
    file_name: String,
}

pub struct UploadFileReq<'a> {
    file_name: &'a str,
    file_contents: &'a str,
    password: &'a str,
    salt: &'a str,
}

impl<'a> UploadFileReq<'a> {
    pub fn new() -> Self {
        Self {
            file_name: "",
            file_contents: "",
            password: "",
            salt: "",
        }
    }
}

pub async fn home() -> Html<String> {
    let mut handle = std::fs::File::open("./templates/index.html").unwrap();
    let mut buffer = String::new();
    let _ = handle.read_to_string(&mut buffer).unwrap();
    axum::response::Html(buffer)
}

pub async fn download_file(
    Form(download_request): Form<DownloadReq>,
) -> axum::response::Result<String, StatusCode> {
    let file_path = std::path::PathBuf::from(download_request.file_name);
    if !file_path.exists() {
        return Err(StatusCode::BAD_REQUEST);
    }
    let mut count = 0;
    for _ in file_path.components() {
        count += 1;
        if count > 1 {
            return Err(StatusCode::BAD_REQUEST);
        }
    }
    let data = std::fs::read_to_string(file_path).unwrap();
    Ok(data)
}

pub async fn upload_file(mut form_input: axum::extract::Multipart) -> StatusCode {
    let mut file_name: String;
    let mut file_contents: String;
    let mut password: String;
    while let Ok(Some(field)) = form_input.next_field().await {
        let field_name = field.name();
        if field_name.is_none() {
            return StatusCode::BAD_REQUEST;
        }
        if field_name.unwrap() == "file" {
            file_name = field.file_name().unwrap_or("default_file_name").to_string();
            file_contents = field.text().await.unwrap();
        } else if field_name.unwrap() == "pwd" {
            password = field.text().await.expect("Required: password").to_string();
        } else {
            return StatusCode::BAD_REQUEST;
        }
    }
    StatusCode::OK
}
