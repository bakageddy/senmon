use axum::http::StatusCode;
use axum::{body::Body, http::Response, Form};
use axum_extra::extract::cookie::Cookie;
use serde::{Deserialize, Serialize};

use crate::db::{self, *};
use crate::session::*;

#[derive(Serialize, Deserialize)]
pub struct AuthRequest {
    username: String,
    password: String,
}

// #[axum::debug_handler]
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
    match result {
        Some(_) => {
            return Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .header("HX-Location", "/assets/html/home.html")
                .body(Body::empty())
                .unwrap();
        }
        None => {}
    }
    let result = db::get_user_id(&state, &req.username).await;
    return match result {
        Ok(id) => {
            let session = Session::new(id);
            session_serialize(&state, &session).await.unwrap();
            Response::builder()
                .status(StatusCode::ACCEPTED)
                .header("HX-Location", "/assets/html/land.html")
                .header("Set-Cookie", format!("session={}", session.session_id))
                .body(Body::empty())
                .unwrap()
        }
        Err(_) => Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .header("HX-Location", "/")
            .body(Body::empty())
            .unwrap(),
    };
}

// #[axum::debug_handler]
pub async fn login(
    axum::extract::State(state): axum::extract::State<DatabaseConnection>,
    Form(req): Form<AuthRequest>,
) -> Response<Body> {
    if !is_present(&state, &req.username).await {
        return Response::builder()
            .header("HX-Redirect", "/assets/html/home.html")
            .body(Body::empty())
            .unwrap();
    }

    if !validate_user(&state, &req.username, &req.password).await {
        return Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .header("HX-Redirect", "/assets/html/home.html")
            .body(Body::empty())
            .unwrap();
    }

    let result = get_user_id(&state, &req.username).await;
    if let Ok(id) = result {
        let session = Session::new(id);
        if let Some(err) = session_serialize(&state, &session).await {
            eprintln!("{err}");
            return Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .header("HX-Redirect", "/assets/html/home.html")
                .body(Body::empty())
                .unwrap();
        }
        
        let cookie = Cookie::build(("session", session.session_id.to_string()))
            .path("/")
            .build();
        return Response::builder()
            .status(StatusCode::ACCEPTED)
            .header("HX-Redirect", "/assets/html/land.html")
            .header("Set-Cookie", cookie.to_string())
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
