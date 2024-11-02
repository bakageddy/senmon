use axum::{http::StatusCode, response::Html, Form};
use ring::rand::SecureRandom;
use std::io::Read;
use std::ops::Deref;
use std::sync::{Arc, Mutex};

use serde::Deserialize;

#[derive(Clone)]
pub struct DatabaseConnection {
    pub ctx: Arc<Mutex<rusqlite::Connection>>,
}

#[derive(Deserialize)]
pub struct DownloadReq {
    file_name: String,
}

pub struct UploadFile {
    file_name: String,
    file_contents: String,
    password: String,
    salt: String,
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

pub fn generate_salt() -> String {
    let rng = ring::rand::SystemRandom::new();
    let mut salt: [u8; 32] = [0;32];
    rng.fill(&mut salt).unwrap();
    unsafe {std::str::from_utf8_unchecked(&salt).to_string()}
}

pub async fn upload_file(
    axum::extract::State(db): axum::extract::State<DatabaseConnection>,
    form_input: axum::extract::Multipart,
) -> StatusCode {
    let req = match parse_multipart(form_input).await {
        Ok(r) => r,
        Err(x) => return x,
    };
    let ctx = db.ctx.deref().lock().unwrap();
    let _ = ctx.execute(
        "INSERT INTO file_state(file_name, salt) VALUES(?1, ?2)",
        [req.file_name, req.salt],
    ).unwrap();
    StatusCode::OK
}

pub async fn parse_multipart(
    mut form_response: axum::extract::Multipart,
) -> Result<UploadFile, StatusCode> {
    let mut file_name: String = String::new();
    let mut file_contents: String = String::new();
    let mut password: String = String::new();
    while let Ok(Some(field)) = form_response.next_field().await {
        let field_name = field.name();
        match field_name {
            Some("file") => {
                file_name = field.file_name().unwrap_or("default_file_name").to_string();
                file_contents = field.text().await.unwrap_or("".to_string());
            }
            Some("pwd") => {
                password = field.text().await.unwrap_or("default".to_string());
            }
            Some(_) => {
                return Err(StatusCode::BAD_REQUEST);
            }
            None => {
                return Err(StatusCode::BAD_REQUEST);
            }
        }
    }
    Ok(UploadFile {
        file_name,
        file_contents,
        password,
        salt: generate_salt(),
    })
}
