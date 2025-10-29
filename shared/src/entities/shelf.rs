use crate::entities::local_resource::{LocalResource, LocalResourcePath};
use crate::repository::local_resource::LocalResourceId;
use core_services::db::repository::abstraction::id::DbId;
use devlog_sdk::distributed_id::gen_id_sync;

#[derive(Clone, Debug, Default)]
pub struct Shelf {
    pub id: u64,
    pub resources: Vec<LocalResource>,
    pub name: String
}

impl Shelf {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: gen_id_sync(),
            resources: Vec::new(),
            name: name.into()
        }
    }

    pub fn is_exists(&self, path: &LocalResourcePath) -> bool {
        self.resources.iter().any(|resource| resource.path.eq(path))
    }

    pub fn get_resources(&mut self, resources: Vec<LocalResource>) {
        for resource in resources {
            self.resources.push(resource);
        }
    }

    pub fn add_resource(&mut self, resource: LocalResource) {
        if !self.is_exists(&resource.path) {
            self.resources.push(resource);
        }
    }

    pub fn remove_resource(&mut self, remove: &LocalResourceId) {
        self.resources.retain(|it| !remove.is_represent(it));
    }

    pub fn get(&self, id: &LocalResourceId) -> Option<&LocalResource> {
        self.resources.iter().find(|it| id.is_represent(it))
    }
}
