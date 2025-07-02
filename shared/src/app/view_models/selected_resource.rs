use serde::{Deserialize, Serialize};
use uniffi::Record;

use crate::app::file_system::file::{LocalResource, LocalResourcePath, ResourceType};

#[derive(Debug, PartialEq, Record, Serialize, Deserialize, Clone)]
pub struct SelectedResourceViewModel {
    pub order_id: u64,
    pub name: String,
    pub size_gb: f64,
    pub size_mb: f64,
    pub display_path: String,
    pub path: LocalResourcePath,
    pub thumbnail_path: Option<LocalResourcePath>,
    pub r#type: ResourceType
}

impl From<&LocalResource> for SelectedResourceViewModel {
    fn from(resource: &LocalResource) -> Self {
        let mut view_model = SelectedResourceViewModel {
            order_id: resource.order_id,
            name: resource.name.clone(),
            size_gb: (format!("{:.2}", resource.size as f64 / 1024.0 / 1024.0 / 1024.0)).parse::<f64>().unwrap_or(0.0),
            size_mb: 0.0,
            display_path: {
                let path_string = resource.path.as_string();
                if path_string.contains("://") {
                    path_string.split("://").last().unwrap_or(&path_string).to_string()
                } else {
                    path_string
                }
            },
            path: resource.path.clone(),
            thumbnail_path: resource.thumbnail_path.clone(),
            r#type: resource.r#type.clone()
        };

        if view_model.size_gb < 0.1 {
            view_model.size_gb = 0.0;
            view_model.size_mb = (format!("{:.2}", resource.size as f64 / 1024.0 / 1024.0)).parse::<f64>().unwrap_or(0.0);
        }

        view_model
    }
}
