use axum::{
    Json,
    extract::State,
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use serde::Deserialize;
use sqlx::PgPool;
use dotenvy::dotenv;
use std::env;

#[derive(Deserialize, serde::Serialize)]
struct SignInUser {
    email : String,
    password : String,
}

#[derive(serde::Serialize)]
struct SignInResponse {
    email: String,
    password: String,
}

async fn sign_in(
    State(db):State<PgPool>,
    Json(payload) : Json<SignInUser>
) -> impl IntoResponse {
    let SignInUser {email, password} = payload;

    sqlx::query!(
        "INSERT INTO users (email, password) VALUES ($1, $2)",
        email,
        password
    ).execute(&db).await.unwrap();

    Json(SignInResponse{
        email, password
    })
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

    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");

    let db = PgPool::connect(&database_url).await.unwrap();

    let app = Router::new()
        .route("/", get(root))
        .route("/signin", post(sign_in))
        .fallback(default_fallback)
        .with_state(db);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
