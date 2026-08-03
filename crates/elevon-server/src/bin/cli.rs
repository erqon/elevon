use elevon_config::ElevonConfig;
use elevon_server::Config;
use elevon_server::db::model::User;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let app_config = Config::from_file("elevon.yml")?;
    let mut db = elevon_server::db::get_db(&app_config.database).await?;

    let user = toasty::create!(User {
        email: "test@test.com",
    })
    .exec(&mut db)
    .await?;

    let (key, _) = elevon_server::service::auth::create_access_key(db, user.id, "Test").await?;

    println!("Key: {}", key);

    Ok(())
}
