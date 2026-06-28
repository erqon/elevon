use axum::Json;
use serde::Deserialize;

#[derive(Deserialize)]
struct Login {
    email: String,
    password: String,
}

pub async fn login(Json(payload): Json<Login>) {}
