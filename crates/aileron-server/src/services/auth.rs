use std::convert::TryFrom;
use std::time::Duration;

use anyhow::Result;
use axum::{
    extract::{FromRequestParts, Request, State},
    http::{HeaderMap, header},
    middleware::Next,
    response::Response,
};
use base64::{Engine, prelude::BASE64_URL_SAFE_NO_PAD};
use pasetors::claims::{Claims, ClaimsValidationRules};
use pasetors::keys::SymmetricKey;
use pasetors::token::UntrustedToken;
use pasetors::{Local, local, version4::V4};
use rand::Rng;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::app::{error::AppError, state::AppState};

const ACCESS_TOKEN_EXPIRES_IN: Duration = Duration::from_secs(60 * 60);

pub struct TokenService {
    key: SymmetricKey<V4>,
}

impl TokenService {
    pub fn new() -> Result<Self> {
        let b64 = std::env::var("AILERON_TOKEN_KEY")?;
        let bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, b64)?;
        let key = SymmetricKey::<V4>::from(&bytes)?;
        Ok(Self { key })
    }

    pub fn create_access_token(&self, user_id: Uuid) -> anyhow::Result<String> {
        let mut claims = Claims::new()?;

        claims.subject(&user_id.to_string())?;
        claims.set_expires_in(&ACCESS_TOKEN_EXPIRES_IN)?;

        let token = local::encrypt(&self.key, &claims, None, None)?;
        Ok(token)
    }

    pub fn validate_access_token(&self, token: &str) -> anyhow::Result<Uuid> {
        let untrusted = UntrustedToken::<Local, V4>::try_from(token)?;
        let rules = ClaimsValidationRules::new();

        let trusted = local::decrypt(&self.key, &untrusted, &rules, None, None)?;
        let sub = trusted
            .payload_claims()
            .ok_or_else(|| anyhow::anyhow!("missing claims"))?
            .get_claim("sub")
            .ok_or_else(|| anyhow::anyhow!("missing sub claim"))?;

        Uuid::parse_str(&sub.to_string()).map_err(Into::into)
    }

    pub fn create_refresh_token(&self) -> (String, String) {
        let mut bytes = [0u8; 32];
        rand::rng().fill_bytes(&mut bytes);

        let refresh_token = BASE64_URL_SAFE_NO_PAD.encode(bytes);

        let mut hasher = Sha256::new();
        hasher.update(refresh_token.as_bytes());
        let hashed_token = hasher
            .finalize()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();

        (refresh_token, hashed_token)
    }
}

#[derive(Clone, Debug)]
pub struct AuthUser {
    pub user_id: Uuid,
}

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &AppState,
    ) -> std::prelude::v1::Result<Self, Self::Rejection> {
        let token = extract_token_from_headers(&parts.headers).ok_or(AppError::Unauthorized)?;

        let user_id = state
            .tokens
            .validate_access_token(&token)
            .map_err(|_| AppError::Unauthorized)?;

        Ok(AuthUser { user_id })
    }
}

fn extract_token_from_headers(headers: &HeaderMap) -> Option<String> {
    if let Some(value) = headers.get(header::AUTHORIZATION) {
        let value = value.to_str().ok()?;
        if let Some(token) = value.strip_prefix("Bearer ") {
            return Some(token.to_string());
        }
    }

    let cookie_header = headers.get(header::COOKIE)?.to_str().ok()?;
    cookie_header.split(';').map(str::trim).find_map(|pair| {
        let (name, value) = pair.split_once('=')?;
        (name == "access_token").then(|| value.to_string())
    })
}

pub async fn require_auth(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let token = extract_token_from_headers(req.headers()).ok_or(AppError::Unauthorized)?;

    let user_id = state
        .tokens
        .validate_access_token(&token)
        .map_err(|_| AppError::Unauthorized)?;

    req.extensions_mut().insert(AuthUser { user_id });
    Ok(next.run(req).await)
}
