use axum::http::{HeaderMap, HeaderValue, header::AUTHORIZATION};

use crate::error::AppError;

pub fn get_auth_token(headers: &HeaderMap<HeaderValue>) -> Result<String, AppError> {
    let token = extract_token_from_headers(headers).ok_or(AppError::Unauthorized)?;
    Ok(token)
}

fn extract_token_from_headers(headers: &HeaderMap) -> Option<String> {
    if let Some(value) = headers.get(AUTHORIZATION) {
        let value = value.to_str().ok()?;
        if let Some(token) = value.strip_prefix("Bearer ") {
            return Some(token.to_string());
        }
    }

    None
}
