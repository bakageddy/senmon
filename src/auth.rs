use std::ops::Deref;

use axum::http::StatusCode;
use axum::Form;
use serde::{Deserialize, Serialize};

use crate::DatabaseConnection;

#[derive(Serialize, Deserialize)]
pub struct AuthRequest {
    username: String,
    password: String,
}

#[axum::debug_handler]
pub async fn auth(
    axum::extract::State(state): axum::extract::State<DatabaseConnection>,
    Form(req): Form<AuthRequest>,
) -> axum::http::StatusCode {
    let cnx = state.ctx.deref().lock().unwrap();
    if let Err(e) = cnx.execute(
        "INSERT INTO user_reg(username, password) VALUES (?1, ?2);",
        [req.username, req.password],
    ) {
        eprintln!("{e:#?}");
        return StatusCode::INTERNAL_SERVER_ERROR;
    }
    StatusCode::OK
}

#[axum::debug_handler]
pub async fn login(
    axum::extract::State(state): axum::extract::State<DatabaseConnection>,
    Form(req): Form<AuthRequest>,
) -> axum::http::StatusCode {
    let cnx = state.ctx.deref().lock().unwrap();
    let result: Result<String, _> = cnx.query_row(
        "SELECT * FROM user_reg WHERE username=?1 AND password=?2;",
        [&req.username, &req.password],
        |r| r.get(0),
    );
    if let Err(e) = result {
        eprintln!("LOGIN: {e:#?}");
        return StatusCode::NOT_FOUND;
    }
    StatusCode::OK
}
