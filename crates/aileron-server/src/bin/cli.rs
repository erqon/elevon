use aileron_config::AileronConfig;
use aileron_server::Config;
use aileron_server::db::model::User;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let app_config = Config::from_file("aileron.yml")?;
    let mut db = aileron_server::db::get_db(&app_config.database).await?;

    let user = toasty::create!(User {
        email: "test@test.com",
    })
    .exec(&mut db)
    .await?;

    let (key, _) = aileron_server::service::auth::create_access_key(db, user.id, "Test").await?;

    println!("Key: {}", key);

    Ok(())
}
