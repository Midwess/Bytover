use actix_web::{get, web, HttpResponse, Result};
use sea_orm::EntityTrait;
use semver::Version;
use serde::Deserialize;

#[derive(serde::Serialize)]
pub struct UpdateManifest {
    pub version: String,
    pub notes: Option<String>,
    pub pubdate: String,
    pub is_critical: bool,
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
pub async fn get_update_manifest(path: web::Path<UpdatePath>) -> Result<HttpResponse> {
    let target = &path.target;
    let arch = &path.arch;
    let current_version = &path.current_version;

    log::info!(
        "Checking for updates: target={}, arch={}, current_version={}",
        target,
        arch,
        current_version
    );

    let current_semver = match Version::parse(current_version) {
        Ok(v) => v,
        Err(e) => {
            log::warn!("Invalid current version format '{}': {}", current_version, e);
            return Ok(HttpResponse::BadRequest().finish());
        }
    };

    let db = crate::di_container::DiContainer::instance().await.get_db_connection();

    use crate::entities::app_release::Entity as AppReleaseEntity;

    let releases = AppReleaseEntity::find().all(&db).await.map_err(|e| {
        log::error!("Database error: {}", e);
        actix_web::error::ErrorInternalServerError("Database error")
    })?;

    let latest = releases
        .into_iter()
        .filter(|r| r.platform == *target && r.architecture == *arch)
        .filter_map(|r| Version::parse(&r.version).ok().map(|v| (r, v)))
        .filter(|(_, v)| *v > current_semver)
        .max_by_key(|(_, v)| v.clone());

    match latest {
        Some((release, _)) => {
            let mut platforms = std::collections::HashMap::new();
            platforms.insert(
                target.clone(),
                PlatformInfo {
                    signature: release.signature,
                    url: release.download_url,
                },
            );

            let manifest = UpdateManifest {
                version: release.version,
                notes: release.release_notes,
                pubdate: release.created_at.format("%Y-%m-%d").to_string(),
                is_critical: release.is_critical,
                platforms,
            };

            log::info!("Update available: v{}", manifest.version);
            Ok(HttpResponse::Ok().json(manifest))
        }
        None => {
            log::info!("No update available for {}-{}-{}", target, arch, current_version);
            Ok(HttpResponse::NoContent().finish())
        }
    }
}
