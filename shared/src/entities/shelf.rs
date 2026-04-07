use crate::entities::local_resource::{LocalResource, LocalResourcePath};
use crate::repository::local_resource::LocalResourceId;
use chrono::{DateTime, Utc};
use core_services::db::repository::abstraction::id::DbId;
use devlog_sdk::distributed_id::{gen_id_sync, id_to_datetime};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Shelf {
    pub id: u64,
    pub name: String,
    #[serde(skip)]
    pub resources: Vec<LocalResource>
}

impl Shelf {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: gen_id_sync(),
            resources: Vec::new(),
            name: name.into()
        }
    }

    pub fn with_id(id: u64, name: impl Into<String>) -> Self {
        Self {
            id,
            resources: Vec::new(),
            name: name.into()
        }
    }

    pub fn update_name(&mut self, name: &str) {
        self.name = name.to_string();
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        id_to_datetime(self.id)
    }

    pub fn is_exists(&self, path: &LocalResourcePath) -> bool {
        self.resources.iter().any(|resource| resource.path.eq(path))
    }

    pub fn add_resources(&mut self, resources: Vec<LocalResource>) {
        for resource in resources {
            self.resources.push(resource);
        }
    }

    pub fn add_resource(&mut self, resource: LocalResource) {
        if !self.is_exists(&resource.path) {
            self.resources.push(resource);
        }
    }

    pub fn update_resource(&mut self, update: &LocalResourceId, resource: LocalResource) {
        if let Some(existing) = self.resources.iter_mut().find(|it| update.is_represent(it)) {
            *existing = resource;
        }
    }

    pub fn remove_resource(&mut self, remove: &LocalResourceId) {
        self.resources.retain(|it| !remove.is_represent(it));
    }

    pub fn get(&self, id: &LocalResourceId) -> Option<&LocalResource> {
        self.resources.iter().find(|it| id.is_represent(it))
    }
}
