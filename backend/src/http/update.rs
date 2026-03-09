use actix_web::{get, web, HttpResponse, Result};
use sea_orm::{EntityTrait, ColumnTrait, QueryFilter};
use serde::Deserialize;

#[derive(serde::Serialize)]
pub struct UpdateManifest {
    pub version: String,
    pub notes: Option<String>,
    pub pubdate: String,
    pub platforms: std::collections::HashMap<String, PlatformInfo>,
}

#[derive(serde::Serialize)]
pub struct PlatformInfo {
    pub signature: String,
    pub url: String,
}

#[derive(Deserialize)]
pub struct UpdatePath {
    target: String,
    arch: String,
    current_version: String,
}

#[get("/update/{target}/{arch}/{current_version}")]
pub async fn get_update_manifest(
    path: web::Path<UpdatePath>,
) -> Result<HttpResponse> {
    let target = &path.target;
    let arch = &path.arch;
    let current_version = &path.current_version;

    log::info!(
        "Checking for updates: target={}, arch={}, current_version={}",
        target,
        arch,
        current_version
    );

    let db = crate::di_container::DiContainer::instance()
        .await
        .get_db_connection();

    use crate::entities::app_release::{Entity as AppReleaseEntity, Column as AppReleaseColumn};

    let releases = AppReleaseEntity::find()
        .filter(
            AppReleaseColumn::Platform.eq(target)
                .and(AppReleaseColumn::Architecture.eq(arch))
                .and(AppReleaseColumn::Version.gt(current_version))
        )
        .all(&db)
        .await
        .map_err(|e| {
            log::error!("Database error: {}", e);
            actix_web::error::ErrorInternalServerError("Database error")
        })?;

    let latest = releases.into_iter().max_by_key(|r| r.version.clone());

    match latest {
        Some(release) => {
            let mut platforms = std::collections::HashMap::new();
            let platform_key = format!("{}-{}", release.platform, release.architecture);
            platforms.insert(
                platform_key,
                PlatformInfo {
                    signature: release.signature,
                    url: release.download_url,
                },
            );

            let manifest = UpdateManifest {
                version: release.version,
                notes: release.release_notes,
                pubdate: release.created_at.format("%Y-%m-%d").to_string(),
                platforms,
            };

            Ok(HttpResponse::Ok().json(manifest))
        }
        None => {
            log::info!("No update available for {}-{}-{}", target, arch, current_version);
            Ok(HttpResponse::NotFound().finish())
        }
    }
}
