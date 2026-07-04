use anyhow::Result;
use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};

pub fn hash_password(password: String) -> Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|err| anyhow::anyhow!("Failed to hash password: {err}"))?
        .to_string();

    Ok(password_hash)
}

pub fn verify_password(password: String, password_hash: String) -> Result<bool> {
    let argon2 = Argon2::default();
    let parsed_hash = PasswordHash::new(&password_hash)
        .map_err(|err| anyhow::anyhow!("Failed to verify password: {err}"))?;

    Ok(argon2
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}

