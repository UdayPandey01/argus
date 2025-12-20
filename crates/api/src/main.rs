use axum::{
    Json,
    extract::State,
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use serde::Deserialize;
use sqlx::PgPool;

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
    let db = PgPool::connect("postgresql://neondb_owner:npg_OzqwJVEs5YM0@ep-sparkling-waterfall-a4qtgccn-pooler.us-east-1.aws.neon.tech/neondb?sslmode=require&channel_binding=require").await.unwrap();

    let app = Router::new()
        .route("/", get(root))
        .route("/signin", post(sign_in))
        .fallback(default_fallback)
        .with_state(db);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
