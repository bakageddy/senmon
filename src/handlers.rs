use crate::db;
use aes_gcm::aead::Aead;
use aes_gcm::{AeadCore, KeyInit};
use axum::body::Body;
use axum::http::header;
use axum::{http::StatusCode, response::Html, Form};
use rand::Rng;
use std::io::Read;
use std::num::NonZeroU32;
use std::ops::Deref;

use serde::Deserialize;

#[derive(Deserialize)]
pub struct DownloadReq {
    file_name: String,
    password: String,
}

pub struct UploadFile {
    pub file_name: String,
    pub file_contents: String,
    pub password: String,
    pub salt: String,
}

pub struct DatabaseRow {
    pub file_name: String,
    pub salt: String,
}

pub async fn home() -> Html<String> {
    let mut handle = std::fs::File::open("./assets/html/home.html").unwrap();
    let mut buffer = String::new();
    let _ = handle.read_to_string(&mut buffer).unwrap();
    axum::response::Html(buffer)
}

pub async fn download_file(
    axum::extract::State(state): axum::extract::State<db::DatabaseConnection>,
    Form(download_request): Form<DownloadReq>,
) -> axum::response::Response<Body> {
    let conn = state.ctx.deref().lock().unwrap();
    let db_row = conn.query_row(
        r#"SELECT file_name, salt FROM file_state WHERE file_name=(?1);"#,
        [&download_request.file_name],
        |row| {
            Ok(DatabaseRow {
                file_name: row.get(0).unwrap(),
                salt: row.get(1).unwrap(),
            })
        },
    );
    let db_row = match db_row {
        Ok(x) => x,
        Err(_) => {
            return axum::response::Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Body::empty())
                .unwrap();
        }
    };

    let path = std::path::PathBuf::from(&db_row.file_name);
    if !path.exists() {
        return axum::response::Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::empty())
            .unwrap();
    }
    let mut count = 0;
    for _ in path.components() {
        if count > 1 {
            return axum::response::Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Body::empty())
                .unwrap();
        }
        count += 1;
    }

    let encrypted_string = std::fs::read_to_string(path).unwrap();
    let encrypted_bytes_with_nonce = hex::decode(encrypted_string).unwrap();
    let (nonce, encrypted_bytes) = encrypted_bytes_with_nonce.split_at(12);
    let nonce = aes_gcm::Nonce::from_slice(nonce);

    let mut password_hash: [u8; 32] = [0; 32];
    ring::pbkdf2::derive(
        ring::pbkdf2::PBKDF2_HMAC_SHA512,
        NonZeroU32::new(600_000).unwrap(),
        db_row.salt.as_bytes(),
        download_request.password.as_bytes(),
        &mut password_hash,
    );
    let key = aes_gcm::Key::<aes_gcm::Aes256Gcm>::from_slice(&password_hash);
    let cipher = aes_gcm::Aes256Gcm::new(key);
    let text = cipher.decrypt(nonce, encrypted_bytes).unwrap();
    match std::str::from_utf8(&text) {
        Ok(x) => {
            return axum::response::Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "text/plain")
                .header(header::CONTENT_DISPOSITION, format!("attachment; filename={}", &db_row.file_name))
                .body(Body::new(x.to_owned()))
                .unwrap();
        },
        Err(_) => {
            return axum::response::Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::empty())
                .unwrap();
        }
    }
}

pub fn generate_salt() -> String {
    let salt: String = rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(32)
        .map(char::from)
        .collect();
    salt
}

pub async fn upload_file(
    axum::extract::State(db): axum::extract::State<db::DatabaseConnection>,
    form_input: axum::extract::Multipart,
// ) -> axum::response::Result<String, StatusCode> {
) -> axum::response::Response {
    let req = match parse_multipart(form_input).await {
        Ok(r) => r,
        Err(_) => {
            return axum::response::Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .header("HX-Redirect", "/assets/html/land.html")
                .body(Body::empty())
                .unwrap();
        }
    };
    let ctx = db.ctx.deref().lock().unwrap();
    let res = encrypt_contents(req);
    let _ = ctx
        .execute(
            "INSERT INTO file_state(file_name, salt) VALUES(?1, ?2)",
            [&res.file_name, &res.salt],
        )
        .unwrap();

    let _ = std::fs::write(&res.file_name, &res.file_contents).unwrap();
    return axum::response::Response::builder()
        .status(StatusCode::OK)
        .header("HX-Redirect", "/assets/html/land.html")
        .body(Body::empty())
        .unwrap();
}

pub fn encrypt_contents(mut request: UploadFile) -> UploadFile {
    let mut password_hash: [u8; 32] = [0; 32];
    ring::pbkdf2::derive(
        ring::pbkdf2::PBKDF2_HMAC_SHA512,
        NonZeroU32::new(600_000).unwrap(),
        request.salt.as_bytes(),
        request.password.as_bytes(),
        &mut password_hash,
    );
    let key = aes_gcm::Key::<aes_gcm::Aes256Gcm>::from_slice(&password_hash);
    let cipher = aes_gcm::Aes256Gcm::new(key);
    let nonce = aes_gcm::Aes256Gcm::generate_nonce(aes_gcm::aead::OsRng);
    let encrypted_contents = cipher
        .encrypt(&nonce, request.file_contents.as_bytes())
        .unwrap();
    let mut encrypted_contents_with_nonce = nonce.to_vec();
    encrypted_contents_with_nonce.extend(encrypted_contents);
    request.file_contents = hex::encode(encrypted_contents_with_nonce);
    request
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
