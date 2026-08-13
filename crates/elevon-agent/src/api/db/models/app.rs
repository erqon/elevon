use crate::api::dto::AppDeployData;

#[derive(Debug, toasty::Model)]
pub struct App {
    #[key]
    #[auto]
    pub id: uuid::Uuid,

    #[column(type = varchar(32))]
    pub name: String,

    pub domain: String,

    #[unique]
    pub current_port: Option<u16>,

    #[has_many]
    pub deployments: toasty::Deferred<Vec<Deployment>>,

    #[auto]
    pub created_at: jiff::Timestamp,

    #[auto]
    pub updated_at: jiff::Timestamp,
}

impl App {
    pub async fn get_or_create(
        db: &mut toasty::Db,
        config: &AppDeployData,
    ) -> anyhow::Result<Self> {
        let app = Self::filter(Self::fields().name().eq(&config.name))
            .order_by(Self::fields().updated_at().asc())
            .first()
            .exec(db)
            .await?;

        if let Some(app) = app {
            return Ok(app);
        }

        let app = toasty::create!(App {
            name: config.name.to_string(),
            domain: config.domain.to_string(),
        })
        .exec(db)
        .await?;

        Ok(app)
    }
}

#[derive(Debug, toasty::Embed)]
pub enum DeploymentStatus {
    Pending,
    Active,
    Failed,
}

#[derive(Debug, toasty::Model)]
pub struct Deployment {
    #[key]
    #[auto]
    pub id: uuid::Uuid,

    #[index]
    pub app_id: uuid::Uuid,

    pub port: u16,

    pub status: DeploymentStatus,

    #[belongs_to(key = app_id, references = id)]
    pub app: toasty::Deferred<App>,

    #[auto]
    pub created_at: jiff::Timestamp,

    #[auto]
    pub updated_at: jiff::Timestamp,
}
