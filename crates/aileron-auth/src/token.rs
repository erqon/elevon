use base64::{Engine, prelude::BASE64_URL_SAFE_NO_PAD};
use rand::Rng;
use sha2::{Digest, Sha256};

pub fn opaque() -> String {
    let mut bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    BASE64_URL_SAFE_NO_PAD.encode(bytes)
}

pub fn hash(raw_token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(raw_token.as_bytes());
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>()
}

pub struct IssuedToken {
    pub raw: String,
    pub hash: String,
}

pub fn issue() -> IssuedToken {
    let raw = opaque();
    let hash = hash(&raw);

    IssuedToken { raw, hash }
}

pub fn access_key() -> String {
    let opaque_key = opaque();
    format!("aileron_ak_{}", opaque_key)
}

pub fn agent_key() -> String {
    let opaque_key = opaque();
    format!("aileron_ag_{}", opaque_key)
}
