use std::str::FromStr;

use anyhow::Result;
use bollard::plugin::RestartPolicyNameEnum;
use elevon_contracts::deploy::{AppPayload, AppRole, AppRuntimeOptions};

#[derive(Debug, toasty::Model)]
pub struct App {
    #[key]
    #[auto]
    pub id: uuid::Uuid,

    #[column(type = varchar(32))]
    #[index]
    pub name: String,

    #[index]
    pub agent: String,

    #[index]
    pub project: String,

    pub domain: Option<String>,

    #[default(5)]
    pub keep_releases: u8,

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

        if let Some(mut app) = app {
            if app.keep_releases != payload.keep_releases {
                app.update()
                    .keep_releases(payload.keep_releases)
                    .exec(db)
                    .await?;
            }

            return Ok(app);
        }

        let domain: Option<String> = payload
            .web_app
            .as_ref()
            .map(|web_app| web_app.domain.clone());

        let app = toasty::create!(App {
            name: payload.name.to_string(),
            project: payload.project.to_string(),
            domain,
            keep_releases: payload.keep_releases
        })
        .exec(db)
        .await?;

        Ok(app)
    }
}

#[derive(Debug, toasty::Embed, PartialEq, Eq)]
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
    pub image_ref: Option<String>,

    #[index]
    pub container_id: Option<String>,

    pub port: u16,

    #[default(DeploymentStatus::Pending)]
    pub status: DeploymentStatus,

    #[belongs_to(key = app_id, references = id)]
    pub app: toasty::Deferred<App>,

    #[has_one]
    pub runtime_options: toasty::Deferred<Option<DeploymentRuntimeOption>>,

    #[auto]
    pub created_at: jiff::Timestamp,

    #[auto]
    pub updated_at: jiff::Timestamp,
}

impl Deployment {
    // It would be better this to be also checking the containers status just to make sure its running or not
    async fn _get_by_app_id(
        db: &mut toasty::Db,
        app_id: &uuid::Uuid,
        status: DeploymentStatus,
    ) -> Result<Option<Self>> {
        let deployment = Deployment::filter(
            Deployment::fields()
                .app_id()
                .eq(app_id)
                .and(Deployment::fields().status().eq(status)),
        )
        .include(Deployment::fields().app())
        .include(Deployment::fields().runtime_options())
        .latest_by(Deployment::fields().updated_at())
        .first()
        .exec(db)
        .await?;

        Ok(deployment)
    }

    pub async fn get_latest_deployment(
        db: &mut toasty::Db,
        app_id: &uuid::Uuid,
    ) -> Result<Option<Deployment>> {
        let deployment = Deployment::_get_by_app_id(db, app_id, DeploymentStatus::Active).await?;
        Ok(deployment)
    }

    pub async fn get_previous_deployment(
        db: &mut toasty::Db,
        app_id: &uuid::Uuid,
    ) -> Result<Option<Deployment>> {
        let deployment = Deployment::_get_by_app_id(db, app_id, DeploymentStatus::Drained).await?;
        Ok(deployment)
    }

    pub async fn list_by_app_id(
        db: &mut toasty::Db,
        app_id: &uuid::Uuid,
        keep: usize,
    ) -> Result<Vec<Self>> {
        Ok(Deployment::filter_by_app_id(app_id)
            .latest_by(Deployment::fields().updated_at())
            .limit(i64::MAX as usize)
            .offset(keep)
            .exec(db)
            .await?)
    }
}

#[derive(Debug, Default, toasty::Embed)]
pub struct DeploymentRuntimeOptions {
    pub role: String,
    pub port: Option<u16>,
    /// Comma separated string
    pub cmd: Option<String>,
    pub restart: Option<String>,
    pub memory_limit: Option<i64>,
    pub cpu_limit: Option<i64>,
    pub network: Option<String>,
}

impl From<&AppRuntimeOptions> for DeploymentRuntimeOptions {
    fn from(value: &AppRuntimeOptions) -> Self {
        Self {
            role: value.role.to_string(),
            cmd: value.cmd.clone().map(|c| c.join(",")),
            restart: value.restart.as_ref().map(ToString::to_string),
            memory_limit: value.memory_limit,
            cpu_limit: value.cpu_limit,
            network: value.network.clone(),
            ..Default::default()
        }
    }
}

impl From<&DeploymentRuntimeOptions> for AppRuntimeOptions {
    fn from(value: &DeploymentRuntimeOptions) -> Self {
        Self {
            role: AppRole::from_str(&value.role).unwrap(),
            cmd: value
                .cmd
                .clone()
                .map(|s| s.split(',').map(|item| item.trim().to_string()).collect()),
            restart: value
                .restart
                .clone()
                .map(|r| RestartPolicyNameEnum::from_str(&r).unwrap()),
            memory_limit: value.memory_limit,
            cpu_limit: value.cpu_limit,
            network: value.network.clone(),
        }
    }
}

#[derive(Debug, toasty::Model)]
pub struct DeploymentRuntimeOption {
    #[key]
    #[auto]
    pub id: uuid::Uuid,

    #[unique]
    pub deployment_id: uuid::Uuid,

    pub options: DeploymentRuntimeOptions,

    #[belongs_to(key = deployment_id, references = id)]
    pub deployment: toasty::Deferred<Deployment>,
}
