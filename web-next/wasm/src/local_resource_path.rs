use shared::app::file_system::file::LocalResourcePath;

pub trait WebExtLocalResourcePath {
    fn cache(store: impl Into<String>, key: impl Into<String>) -> Self;
    fn cache_store_key_pair(&self) -> Option<(String, String)>;
}

impl WebExtLocalResourcePath for LocalResourcePath {
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
}
