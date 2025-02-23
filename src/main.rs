mod handlers;

use std::sync::{Arc, Mutex};
use axum::{
    routing::{get, post},
    Router,
};
use tower_http::services::ServeDir;
use handlers::*;

#[tokio::main]
async fn main() {
    let conn = rusqlite::Connection::open("./file_storage.db").unwrap();
    let _ = conn.execute(
            "CREATE TABLE IF NOT EXISTS file_state(file_name VARCHAR PRIMARY KEY, salt VARCHAR);",
            [],
        ).unwrap();
    let application_state = DatabaseConnection {
        ctx: Arc::new(Mutex::new(conn)),
    };

    let listener = tokio::net::TcpListener::bind("127.0.0.1:42069")
        .await
        .unwrap();
    let router = Router::new()
        .route("/", get(home))
        .nest_service("/assets/css/", ServeDir::new("./assets/css/"))
        .nest_service("/assets/icons/", ServeDir::new("./assets/icons/"))
        .route("/upload_file", post(upload_file))
        .route("/download_file", post(download_file))
        .with_state(application_state);
    axum::serve(listener, router).await.unwrap();
}
