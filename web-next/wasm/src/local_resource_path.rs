use shared::app::file_system::file::LocalResourcePath;

pub trait WebExtLocalResourcePath {
    fn device_file(id: u64) -> Self;
    fn device_file_id(&self) -> Option<u64>;
    fn cache(store: impl Into<String>, key: impl Into<String>) -> Self;
    fn cache_store_key_pair(&self) -> Option<(String, String)>;
    fn thumbnail_resource_id(&self) -> Option<u64>;
    fn resource_id(&self) -> Option<u64>;
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

    fn resource_id(&self) -> Option<u64> {
        let (store, key) = self.cache_store_key_pair()?;
        if store == "resources" {
            return key.trim_start_matches('/').parse::<u64>().ok()
        };

        None
    }

    fn cache(store: impl Into<String>, key: impl Into<String>) -> Self {
        let store_name: String = store.into();
        let key_name: String = key.into();

        Self::PlatformIdentifier(format!(
            "cache://{}/{}",
            store_name, key_name
        ))
    }

    fn cache_store_key_pair(&self) -> Option<(String, String)> {
        match self {
            Self::PlatformIdentifier(path) => {
                if !path.starts_with("cache://") {
                    return None;
                }

                let path = path.trim_start_matches("cache://");
                let parts: Vec<&str> = path.split('/').collect();
                let store_name = parts[0];
                Some((store_name.to_string(), path.trim_start_matches(store_name).to_string()))
            }
            _ => None,
        }
    }

    fn thumbnail_resource_id(&self) -> Option<u64> {
        let Some((store, key)) = self.cache_store_key_pair() else {
            return None;
        };

        if store != "thumbnails" {
            return None;
        }

        key.trim_start_matches('/').parse::<u64>().ok()
    }
}
