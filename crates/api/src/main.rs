use anyhow::{self};
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use chrono::{Duration, Utc};
use dotenvy::dotenv;
use jsonwebtoken::{EncodingKey, Header, encode};
use rand_core::OsRng;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use sqlx::Row;
use std::env;

struct AppError(anyhow::Error);

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error" : "Something went wrong"
            })),
        )
            .into_response()
    }
}

impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}

#[derive(Deserialize, serde::Serialize)]
struct SignInUser {
    email: String,
    password: String,
}

#[derive(serde::Deserialize, serde::Serialize)]
struct SignUpUser {
    username: String,
    email: String,
    password: String,
}

#[derive(serde::Serialize)]
struct SignInResponse {
    email: String,
    password: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,
    exp: usize,
}

async fn sign_in(
    State(db): State<PgPool>,
    Json(payload): Json<SignInUser>,
) -> Result<impl IntoResponse, AppError> {
    let SignInUser { email, password } = payload;

    let user = sqlx::query("SELECT id, email, password FROM users WHERE email = $1")
        .bind(&email)
        .persistent(false)
        .fetch_optional(&db)
        .await?;

    if let Some(user) = user {
        let hashed: String = user.get("password");

        let parsed_hash = PasswordHash::new(&hashed).map_err(|e| anyhow::anyhow!(e))?;

        let is_valid = Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok();

        if !is_valid {
            return Ok((
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({
                    "error" : "Invalid credentials"
                })),
            ));
        }

        let expiration = Utc::now() + Duration::hours(24);

        let claims = Claims {
            sub: email,
            exp: expiration.timestamp() as usize,
        };

        let secret = env::var("JWT_SECRET").expect("JWT_SECRET must be set");

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(secret.as_bytes()),
        )
        .map_err(|e| anyhow::anyhow!(e))?;

        return Ok((
            StatusCode::OK,
            Json(serde_json::json!({
                "status": "logged_in",
                "token": token
            })),
        ));
    }

    Ok((
        StatusCode::UNAUTHORIZED,
        Json(serde_json::json!({
            "error" : "Invalid credentials"
        })),
    ))
}

async fn sign_up(State(db): State<PgPool>, Json(payload): Json<SignUpUser>) -> impl IntoResponse {
    let SignUpUser {
        username,
        email,
        password,
    } = payload;

    let salt = argon2::password_hash::SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();

    let hashed_password = argon2
        .hash_password(password.as_bytes(), &salt)
        .unwrap()
        .to_string();

    sqlx::query("INSERT INTO users (username, email, password) VALUES ($1, $2, $3)")
        .bind(&username)
        .bind(&email)
        .bind(&hashed_password)
        .execute(&db)
        .await
        .unwrap();

    Json(serde_json::json!({
        "status": "user_created"
    }))
}

async fn root() -> impl IntoResponse {
    "signed in"
}

async fn default_fallback() -> impl IntoResponse {
    "Default fallback\n"
}

#[tokio::main]
async fn main() {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let db = PgPool::connect(&database_url).await.unwrap();

    let app = Router::new()
        .route("/", get(root))
        .route("/signin", post(sign_in))
        .route("/signup", post(sign_up))
        .fallback(default_fallback)
        .with_state(db);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}
