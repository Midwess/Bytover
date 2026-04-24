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
    pub store_url: Option<String>,
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

const UNIVERSAL_ARCH: &str = "universal";

pub fn compute_is_critical(
    row_is_critical: bool,
    row_store_url: Option<&str>,
    client_major: u64,
    latest_major: u64,
    force_update_enabled: bool,
) -> bool {
    let is_store_release = row_store_url.is_some();
    let major_bump = latest_major > client_major;
    row_is_critical || (force_update_enabled && is_store_release && major_bump)
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
        .filter(|r| r.platform == *target && (r.architecture == *arch || r.architecture == UNIVERSAL_ARCH))
        .filter_map(|r| Version::parse(&r.version).ok().map(|v| (r, v)))
        .filter(|(_, v)| *v > current_semver)
        .max_by_key(|(_, v)| v.clone());

    match latest {
        Some((release, _)) => {
            let mut platforms = std::collections::HashMap::new();
            if let Some(url) = release.download_url.clone() {
                platforms.insert(
                    target.clone(),
                    PlatformInfo {
                        signature: release.signature.clone(),
                        url,
                    },
                );
            }

            let is_critical = compute_is_critical(
                release.is_critical,
                release.store_url.as_deref(),
                current_semver.major,
                latest_version.major,
                force_update_enabled,
            );
            let is_store_release = release.store_url.is_some();
            let major_bump = latest_version.major > current_semver.major;

            let manifest = UpdateManifest {
                version: release.version,
                notes: release.release_notes,
                pubdate: release.created_at.format("%Y-%m-%d").to_string(),
                is_critical,
                platforms,
                store_url: release.store_url,
            };

            log::info!(
                "Update available: v{}, is_critical={}, major_bump={}, store_release={}",
                manifest.version,
                manifest.is_critical,
                major_bump,
                is_store_release,
            );
            Ok(HttpResponse::Ok().json(manifest))
        }
        None => {
            log::info!("No update available for {}-{}-{}", target, arch, current_version);
            Ok(HttpResponse::NoContent().finish())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::compute_is_critical;

    #[test]
    fn non_store_row_respects_stored_is_critical_flag() {
        assert!(compute_is_critical(true, None, 1, 1, true));
        assert!(!compute_is_critical(false, None, 1, 2, true));
        assert!(!compute_is_critical(false, None, 1, 2, false));
    }

    #[test]
    fn store_row_major_bump_promoted_when_flag_on() {
        assert!(compute_is_critical(false, Some("https://apps.apple.com/app/bytover/id1"), 1, 2, true));
    }

    #[test]
    fn store_row_major_bump_not_promoted_when_flag_off() {
        assert!(!compute_is_critical(false, Some("https://apps.apple.com/app/bytover/id1"), 1, 2, false));
    }

    #[test]
    fn store_row_minor_bump_never_promoted() {
        assert!(!compute_is_critical(false, Some("https://apps.apple.com/app/bytover/id1"), 1, 1, true));
    }

    #[test]
    fn stored_is_critical_overrides_flag_regardless_of_channel() {
        assert!(compute_is_critical(true, None, 1, 1, false));
        assert!(compute_is_critical(true, Some("https://apps.apple.com/app/bytover/id1"), 1, 1, false));
    }
}

