use anyhow::Result;
use elevon_contracts::deploy::AppPayload;

#[derive(Debug, toasty::Model)]
pub struct App {
    #[key]
    #[auto]
    pub id: uuid::Uuid,

    #[column(type = varchar(32))]
    #[index]
    pub name: String,

    #[index]
    pub project: String,

    pub domain: Option<String>,

    #[has_many]
    pub deployments: toasty::Deferred<Vec<Deployment>>,

    #[auto]
    pub created_at: jiff::Timestamp,

    #[auto]
    pub updated_at: jiff::Timestamp,
}

impl App {
    pub async fn get_by_project_and_name(
        db: &mut toasty::Db,
        project: &str,
        name: &str,
    ) -> Result<Option<Self>> {
        Ok(Self::filter_by_project(project)
            .filter_by_name(name)
            .first()
            .exec(db)
            .await?)
    }

    pub async fn get_or_create(db: &mut toasty::Db, payload: &AppPayload) -> Result<Self> {
        let app = Self::filter(Self::fields().name().eq(&payload.name))
            .order_by(Self::fields().updated_at().asc())
            .first()
            .exec(db)
            .await?;

        if let Some(app) = app {
            return Ok(app);
        }

        let domain: Option<String> = payload
            .web_app
            .as_ref()
            .map(|web_app| web_app.domain.clone());

        let app = toasty::create!(App {
            name: payload.name.to_string(),
            project: payload.project.to_string(),
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
    Drained,
    Failed,
}

#[derive(Debug, toasty::Model)]
pub struct Deployment {
    #[key]
    #[auto]
    pub id: uuid::Uuid,

    #[index]
    pub app_id: uuid::Uuid,

    #[index]
    pub prev_deployment_id: Option<uuid::Uuid>,

    pub container_id: Option<String>,

    pub port: u16,

    pub status: DeploymentStatus,

    #[belongs_to(key = app_id, references = id)]
    pub app: toasty::Deferred<App>,

    #[belongs_to(key = prev_deployment_id, references = id)]
    pub prev_deployment: toasty::Deferred<Option<Deployment>>,

    #[auto]
    pub created_at: jiff::Timestamp,

    #[auto]
    pub updated_at: jiff::Timestamp,
}

impl Deployment {
    // It would be better this to be also checking the containers status just to make sure its running or not
    pub async fn get_current_deployment(
        db: &mut toasty::Db,
        app_id: &uuid::Uuid,
    ) -> anyhow::Result<Option<Deployment>> {
        let deployment = Deployment::filter(
            Deployment::fields()
                .app_id()
                .eq(app_id)
                .and(Deployment::fields().status().eq(DeploymentStatus::Active)),
        )
        .latest_by(Deployment::fields().created_at())
        .first()
        .exec(db)
        .await?;

        Ok(deployment)
    }

    pub async fn get_previous_deployment(
        db: &mut toasty::Db,
        app_id: &uuid::Uuid,
    ) -> anyhow::Result<Option<Deployment>> {
        let deployment = Deployment::filter(
            Deployment::fields()
                .app_id()
                .eq(app_id)
                .and(Deployment::fields().status().eq(DeploymentStatus::Active)),
        )
        .latest_by(Deployment::fields().created_at())
        .offset(1)
        .limit(1)
        .first()
        .exec(db)
        .await?;

        Ok(deployment)
    }
}
