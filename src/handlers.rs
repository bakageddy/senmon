use crate::db::{self, get_user_from_session_id, get_user_id, is_present_session};
use aes_gcm::aead::Aead;
use aes_gcm::{AeadCore, KeyInit};
use axum::body::Body;
use axum::http::header;
use axum::{http::StatusCode, response::Html, Form};
use axum_extra::extract::CookieJar;
use rand::Rng;
use std::io::Read;
use std::num::NonZeroU32;
use std::ops::Deref;
use std::u64;

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
    jar: CookieJar,
    Form(download_request): Form<DownloadReq>,
) -> axum::response::Response<Body> {

    let session_id: u64;
    if let Some(cookie) = jar.get("session") {
        session_id = cookie.value().parse().unwrap();
        if !is_present_session(&state, session_id).await {
            return axum::response::Response::builder()
                .status(StatusCode::UNAUTHORIZED)
                .header("HX-Redirect", "/assets/html/home.html")
                .body(Body::empty())
                .unwrap();
        }
    } else {
        return axum::response::Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .header("HX-Redirect", "/assets/html/home.html")
            .body(Body::empty())
            .unwrap();
    }

    let user_name: String;
    if let Some(s) = db::get_user_from_session_id(&state, session_id).await {
        user_name = s;
    } else {
        return axum::response::Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .header("HX-Redirect", "/assets/html/home.html")
            .body(Body::empty())
            .unwrap();
    }

    let cnx = state.ctx.lock().unwrap();
    let db_row = cnx.query_row(
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
                .header("HX-Redirect", "/assets/html/home.html")
                .body(Body::empty())
                .unwrap();
        }
    };

    let mut root = std::path::PathBuf::from("./stash");

    let path = std::path::PathBuf::from(&db_row.file_name);
    let count = path.components().count();
    if count > 1 {
        return axum::response::Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .header("HX-Redirect", "/assets/html/home.html")
            .body(Body::empty())
            .unwrap();
    }

    root = root.join(user_name).join(&path);
    if !root.exists() {
        return axum::response::Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .header("HX-Redirect", "/assets/html/home.html")
            .body(Body::empty())
            .unwrap();
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
                .header(
                    header::CONTENT_DISPOSITION,
                    format!("attachment; filename={}", &db_row.file_name),
                )
                .body(Body::new(x.to_owned()))
                .unwrap();
        }
        Err(_) => {
            return axum::response::Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .header("HX-Redirect", "/assets/html/home.html")
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
    jar: CookieJar,
    form_input: axum::extract::Multipart,
) -> axum::response::Response {

    let user_name: String;
    let ssn_id: u64;
    if let Some(cookie) = jar.get("session") {
        let session_id = cookie.value();
        ssn_id = session_id.parse().unwrap();
        if !is_present_session(&db, ssn_id).await {
            return axum::response::Response::builder()
                .status(StatusCode::UNAUTHORIZED)
                .header("HX-Redirect", "/assets/html/home.html")
                .body(Body::empty())
                .unwrap();
        } else {
            if let Some(x) = get_user_from_session_id(&db, ssn_id).await {
                user_name = x;
            } else {
                return axum::response::Response::builder()
                    .status(StatusCode::UNAUTHORIZED)
                    .header("HX-Redirect", "/assets/html/home.html")
                    .body(Body::empty())
                    .unwrap();
            }
        }
    } else {
        return axum::response::Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .header("HX-Redirect", "/assets/html/home.html")
            .body(Body::empty())
            .unwrap();
    }

    let req = match parse_multipart(form_input).await {
        Ok(r) => r,
        Err(_) => {
            return axum::response::Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .header("HX-Redirect", "/assets/html/home.html")
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

    let path = std::path::PathBuf::from(&res.file_name);

    let count = path.components().count();
    if count > 1 {
        return axum::response::Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .header("HX-Redirect", "/assets/html/home.html")
            .body(Body::empty())
            .unwrap();
    }

    let mut root = std::path::PathBuf::from("./stash/");
    root = root.join(&user_name).join(&path);

    if let Err(_) = std::fs::write(root, &res.file_contents) {
        return axum::response::Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .header("HX-Redirect", "/assets/html/home.html")
            .body(Body::empty())
            .unwrap();
    }

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
