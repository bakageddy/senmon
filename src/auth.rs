use axum::http::StatusCode;
use axum::Form;
use axum::{body::Body, http::Response};
use serde::{Deserialize, Serialize};
use std::ops::Deref;

use crate::db::{self, *};

#[derive(Serialize, Deserialize)]
pub struct AuthRequest {
    username: String,
    password: String,
}

#[axum::debug_handler]
pub async fn auth(
    axum::extract::State(state): axum::extract::State<DatabaseConnection>,
    Form(req): Form<AuthRequest>,
) -> Response<Body> {
    if is_present(&state, &req.username).await {
        return Response::builder()
            .status(StatusCode::CONFLICT)
            .header("HX-Location", "/")
            .body(Body::empty())
            .unwrap();
    }
    let result = db::add_user(&state, &req.username, &req.password).await;
    return match result {
        Some(_) => Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .header("HX-Location", "/")
            .body(Body::empty())
            .unwrap(),
        None => Response::builder()
            .status(StatusCode::ACCEPTED)
            .header("HX-Location", "/assets/html/land.html")
            .body(Body::empty())
            .unwrap(),
    };
}

#[axum::debug_handler]
pub async fn login(
    axum::extract::State(state): axum::extract::State<DatabaseConnection>,
    Form(req): Form<AuthRequest>,
) -> Response<Body> {
    if !is_present(&state, &req.username).await {
        return Response::builder()
            .header("HX-Location", "/")
            .body(Body::empty())
            .unwrap();
    }
    if !validate_user(&state, &req.username, &req.password).await {
        return Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .header("HX-Location", "/")
            .body(Body::empty())
            .unwrap();
    }

    let result = get_user_id(&state, &req.username).await;
    if let Ok(id) = result {
        return Response::builder()
            .status(StatusCode::ACCEPTED)
            .header("HX-Location", "/assets/html/land.html")
            .header("Set-Cookie", format!("session={id}"))
            .body(Body::empty())
            .unwrap();
    } else {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .header("HX-Location", "/")
            .body(Body::empty())
            .unwrap();
    }
}
