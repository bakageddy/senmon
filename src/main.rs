mod auth;
mod db;
mod handlers;
mod session;
mod types;

use std::ops::Deref;

use axum::{
    routing::{get, post},
    Router,
};
use handlers::*;
use tower_http::services::ServeDir;

#[tokio::main]
async fn main() {
    let conn = rusqlite::Connection::open("./file_storage.db").unwrap();
    let application_state = db::DatabaseConnection::new(conn);

    if !init_db(&application_state) {
        eprintln!("FAILED TO INITIALIZE DATABASE");
        return;
    }

    let listener = tokio::net::TcpListener::bind("0.0.0.0:42069")
        .await
        .unwrap();
    let router = Router::new()
        .route("/", get(home))
        .route("/favicon.ico", get(icon))
        .nest_service("/assets", ServeDir::new("./assets"))
        .route("/api/auth", post(auth::auth))
        .route("/api/login", post(auth::login))
        .route("/api/upload_file", post(upload_file))
        .route("/api/download_file", post(download_file))
        .with_state(application_state);

    axum::serve(listener, router).await.unwrap();
}

pub fn init_db(db: &db::DatabaseConnection) -> bool {
    let cnx = db.ctx.deref().lock().unwrap();

    let result = cnx.execute_batch(
        "BEGIN;
        CREATE TABLE IF NOT EXISTS file_state(file_owner INTEGER REFERENCES user_reg(user_id), file_name VARCHAR, salt VARCHAR, PRIMARY KEY (file_owner, file_name));
        CREATE INDEX IF NOT EXISTS file_state_file_owner_file_name ON file_state(file_owner, file_name);

        CREATE TABLE IF NOT EXISTS user_reg(user_id INTEGER PRIMARY KEY AUTOINCREMENT, username VARCHAR UNIQUE, password VARCHAR);
        CREATE INDEX IF NOT EXISTS user_reg_user_id_username ON user_reg(user_id, username);

        CREATE TABLE IF NOT EXISTS sessions(session_id UNSIGNED BIG INT PRIMARY KEY, user_id INTEGER REFERENCES user_reg(user_id), expires TEXT);
        CREATE INDEX IF NOT EXISTS sessions_session_id_user_id ON sessions(session_id, user_id);
        COMMIT;"
    );

    if let Err(why) = result {
        eprintln!("{:?}", why);
        return false;
    }
    return true;
}
