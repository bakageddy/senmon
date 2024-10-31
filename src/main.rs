mod handlers;

use axum::{
    routing::{get, post},
    Router,
};
use handlers::*;

#[tokio::main]
async fn main() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:42069")
        .await
        .unwrap();
    let router: Router = Router::new()
        .route("/", get(home))
        .route("/upload_file", post(upload_file))
        .route("/download_file", post(download_file));

    axum::serve(listener, router).await.unwrap();
}
