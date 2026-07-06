use aileron_config::Config;

use crate::db::{get_db, models::user::User};
use crate::services::auth::hash_password;

pub async fn create_user(config: Config, email: String, password: String) -> anyhow::Result<()> {
    let mut db = get_db(&config.database).await?;

    let password_hash = hash_password(&password)?;
    toasty::create!(User {
        email,
        password: password_hash
    })
    .exec(&mut db)
    .await?;

    Ok(())
}
