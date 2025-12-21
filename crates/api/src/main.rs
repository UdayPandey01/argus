use argon2::{Argon2, PasswordHasher, PasswordVerifier};
use axum::{
    Json, Router,
    extract::State,
    response::IntoResponse,
    routing::{get, post},
};
use dotenvy::dotenv;
use rand_core::OsRng;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use sqlx::Row; 
use jsonwebtoken::{encode, Header, EncodingKey};
use chrono::{Utc, Duration};
use std::env;

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
    sub : String,
    exp : usize
}

async fn sign_in(State(db): State<PgPool>, Json(payload): Json<SignInUser>) -> impl IntoResponse {
    let SignInUser { email, password } = payload;

    let row = sqlx::query("SELECT id, email, password FROM users WHERE email = $1")
        .bind(&email)
        .persistent(false)
        .fetch_optional(&db)
        .await
        .unwrap();

    if row.is_none() {
        return Json(serde_json::json!({"error" : "User not found"}));
    }

    let user = row.unwrap();
    let hashed : String = user.get("password");

    let password_hash = argon2::password_hash::PasswordHash::new(&hashed).unwrap();

    let argon2 = Argon2::default();

    if argon2
        .verify_password(password.as_bytes(), &password_hash)
        .is_err()
    {
        Json(serde_json::json!({
            "error": "Invalid credentials"
        }));
    }

    let expiration = Utc::now() + Duration::hours(24);

    let claims = Claims {
        sub : email.clone(),
        exp : expiration.timestamp() as usize,
    };

    let secret = "Uday62832983";

    let token = encode(&Header::default(), &claims, &EncodingKey::from_secret(secret.as_ref())).unwrap();

    Json(serde_json::json!({
        "status": "logged_in",
        "token" : token
    }))
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
