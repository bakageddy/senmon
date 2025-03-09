mod auth;
mod db;
mod handlers;
mod session;

use axum::{
    routing::{get, post},
    Router,
};
use handlers::*;
use tower_http::services::ServeDir;

#[tokio::main]
async fn main() {
    let conn = rusqlite::Connection::open("./file_storage.db").unwrap();
    let _ = conn
        .execute(
            "CREATE TABLE IF NOT EXISTS file_state(file_name VARCHAR PRIMARY KEY, salt VARCHAR);",
            [],
        )
        .unwrap();
    let _ = conn
        .execute(
            "CREATE TABLE IF NOT EXISTS user_reg(user_id INTEGER PRIMARY KEY AUTOINCREMENT, username VARCHAR UNIQUE, password VARCHAR);",
            []
        )
        .unwrap();
    let _ = conn
        .execute(
            "CREATE TABLE IF NOT EXISTS sessions(session_id INTEGER PRIMARY KEY, user_id INTEGER REFERENCES user_reg(user_id), expires TEXT);",
            []
        ).unwrap();
    let application_state = db::DatabaseConnection::new(conn);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:42069")
        .await
        .unwrap();
    let router = Router::new()
        .route("/", get(home))
        .nest_service("/assets/css/", ServeDir::new("./assets/css/"))
        .nest_service("/assets/icons/", ServeDir::new("./assets/icons/"))
        .nest_service("/assets/templates/", ServeDir::new("./assets/templates/"))
        .nest_service("/assets/js/", ServeDir::new("./assets/js/"))
        .nest_service("/assets/html/", ServeDir::new("./assets/html/"))
        .route("/api/auth", post(auth::auth))
        .route("/api/login", post(auth::login))
        .route("/api/upload_file", post(upload_file))
        .route("/api/download_file", post(download_file))
        .with_state(application_state);
    axum::serve(listener, router).await.unwrap();
}
