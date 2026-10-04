use anyhow::{Context, Result};
use elevon_contracts::deploy::AppPayload;
use serde::{Deserialize, Serialize};
use tabled::Tabled;

use crate::api::db::models::{TabledView, deployment::Deployment};

#[derive(Debug, toasty::Model)]
#[unique(name, project)]
pub struct App {
    #[key]
    #[auto]
    pub id: uuid::Uuid,

    #[column(type = varchar(32))]
    #[index]
    pub name: String,

    #[index]
    pub project: String,

    #[default(5)]
    pub keep_releases: u8,

    #[default(AppRoleDb::Web)]
    pub role: AppRoleDb,

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

        let role = if payload.web_app.is_some() {
            AppRoleDb::Web
        } else {
            AppRoleDb::Worker
        };

        let app = toasty::create!(App {
            name: payload.name.to_string(),
            project: payload.project.to_string(),
            role,
            keep_releases: payload.keep_releases
        })
        .exec(db)
        .await?;

        Ok(app)
    }

    pub async fn get_by_name_or_id(db: &mut toasty::Db, key: &str) -> Result<Self> {
        match uuid::Uuid::parse_str(key) {
            Ok(id) => match Self::get_by_id(db, id).await {
                Ok(k) => Ok(k),
                Err(id_err) => {
                    anyhow::bail!("app '{key}' not found by ID ({id_err:#})")
                }
            },
            Err(_) => {
                let (project, app_name) = key.split_once(':').unwrap_or((key, ""));
                match Self::get_by_name_and_project(db, app_name, project).await {
                    Ok(k) => Ok(k),
                    Err(err) => Err(err).with_context(|| {
                        format!("no app found with name '{key}' (and it is not a valid UUID)")
                    }),
                }
            }
        }
    }
}

#[derive(Debug, toasty::Embed)]
pub enum AppRoleDb {
    Web,
    Worker,
}

#[derive(Clone, Tabled, Serialize, Deserialize)]
pub struct AppTabled {
    pub id: String,
    pub name: String,
    pub project: String,
    pub keep_releases: String,
    pub updated_at: String,
    pub created_at: String,
}

impl TabledView for App {
    type TabledType = AppTabled;

    fn to_tabled(&self) -> Self::TabledType {
        Self::TabledType {
            id: self.id.to_string(),
            name: self.name.clone(),
            project: self.project.clone(),
            keep_releases: self.keep_releases.to_string(),
            updated_at: self.updated_at.to_string(),
            created_at: self.created_at.to_string(),
        }
    }
}
