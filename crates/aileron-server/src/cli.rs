use aileron_config::Config;
use argon2::{
    Argon2,
    password_hash::{PasswordHasher, SaltString, rand_core::OsRng},
};

use crate::db::{get_db, models::user::User};

pub async fn create_user(config: Config, email: String, password: String) -> anyhow::Result<()> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();

    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|err| anyhow::anyhow!("Failed to hash password: {err}"))?
        .to_string();

    let mut db = get_db(&config.database).await?;

    toasty::create!(User {
        email,
        password: password_hash
    })
    .exec(&mut db)
    .await?;

    Ok(())
}
