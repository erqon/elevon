use serde::{Deserialize, Serialize};

use crate::db::model::User;

#[derive(Deserialize)]
pub struct LoginPayload {
    pub key: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginResponse {
    pub session_token: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthMe {
    pub id: uuid::Uuid,
    pub email: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub created_at: jiff::Timestamp,
    pub session_id: uuid::Uuid,
}

impl AuthMe {
    pub fn from_user(user: User, session_id: uuid::Uuid) -> Self {
        Self {
            id: user.id,
            email: user.email,
            first_name: user.first_name,
            last_name: user.last_name,
            created_at: user.created_at,
            session_id,
        }
    }
}
