use elevon_contracts::deploy::AppPayload;

#[derive(Debug, toasty::Model)]
pub struct App {
    #[key]
    #[auto]
    pub id: uuid::Uuid,

    #[column(type = varchar(32))]
    pub name: String,

    pub domain: Option<String>,

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
    pub async fn get_or_create(db: &mut toasty::Db, payload: &AppPayload) -> anyhow::Result<Self> {
        let app = Self::filter(Self::fields().name().eq(&payload.name))
            .order_by(Self::fields().updated_at().asc())
            .first()
            .exec(db)
            .await?;

        if let Some(app) = app {
            return Ok(app);
        }

        let domain: Option<String> = match &payload.web_app {
            Some(web_app) => Some(web_app.domain.clone()),
            None => None,
        };

        let app = toasty::create!(App {
            name: payload.name.to_string(),
            domain
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
