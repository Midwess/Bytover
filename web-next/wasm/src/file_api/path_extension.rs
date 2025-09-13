use shared::entities::file_system::file::LocalResourcePath;

pub trait WebExtLocalResourcePath {
    fn device_file(id: u64) -> Self;
    fn device_file_id(&self) -> Option<u64>;
    fn session_resource(session_id: u64, resource_id: u64, extension: String) -> Self;
    fn resource_thumbnail(session_id: Option<u64>, resource_id: u64) -> Self;
    fn opfs_path(&self) -> Option<String>;
}

impl WebExtLocalResourcePath for LocalResourcePath {
    fn device_file(id: u64) -> Self {
        Self::PlatformIdentifier(format!("device://{}", id))
    }

    fn device_file_id(&self) -> Option<u64> {
        match self {
            Self::PlatformIdentifier(path) => path.split_once("device://")?.1.to_string().parse::<u64>().ok(),
            _ => None
        }
    }

    fn resource_thumbnail(session_id: Option<u64>, resource_id: u64) -> Self {
        match session_id {
            None => Self::PlatformIdentifier(format!("opfs://thumbnails/{}.png", resource_id)),
            Some(session_id) => Self::PlatformIdentifier(format!("opfs://sessions-{session_id}/thumbnails/{}.png", resource_id))
        }
    }

    fn session_resource(session_id: u64, resource_id: u64, extension: String) -> Self {
        Self::PlatformIdentifier(format!("opfs://sessions-{session_id}/{resource_id}.{extension}"))
    }

    fn opfs_path(&self) -> Option<String> {
        match self {
            Self::PlatformIdentifier(path) => {
                if !path.starts_with("opfs://") {
                    return None;
                }

                Some(path.trim_start_matches("opfs://").to_string())
            }
            _ => None
        }
    }
}
