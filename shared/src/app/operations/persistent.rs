use std::collections::HashMap;
use std::future::Future;

use crux_core::capability::Operation;
use serde::{Deserialize, Serialize};

use crate::app::core::command::AppCommand;
use crate::app::AppRequestBuilder;
use crate::entities::local_resource::{LocalResource, LocalResourcePath, ResourceType};
use crate::entities::session::Session;
use crate::entities::shelf::Shelf;
use crate::entities::token::Token;
use crate::entities::user::User;
use crate::errors::CoreError;
use crate::repository::transfer_session::ZipDownloadPaths;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PersistentOperation {
    Session(SessionPersistentOperation),
    User(UserPersistentOperation),
    LocalResource(LocalResourcePersistentOperation),
    TransferSession(TransferSessionPersistentOperation),
    Shelf(ShelfPersistentOperation),
    DeviceAlias(DeviceAliasPersistentOperation)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LocalResourcePersistentOperation {
    Add(Vec<LocalResource>),
    Update(LocalResource),
    Remove { path: LocalResourcePath, shelf_id: u64 },
    FindAll,
    LoadOnDisk(LocalResourcePath),
    GetResourceType { path: LocalResourcePath },
    AddThumbnail { png_bytes: Vec<u8>, resource_id: u64 }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum UserPersistentOperation {
    Save(User)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SessionPersistentOperation {
    WriteToken(Token),
    WriteUser(User),
    Remove,
    Get()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TransferSessionPersistentOperation {
    Clear,
    GenerateResourcePath {
        session_id: u64,
        resource_names: HashMap<u64, (String, ResourceType)>
    },
    GenerateThumbnailPath {
        session_id: Option<u64>,
        resource_ids: Vec<u64>
    },
    GenerateZipDownloadPaths {
        session_order_id: u64,
        resource_names: HashMap<u64, String>
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ShelfPersistentOperation {
    Add(Shelf),
    Update(Shelf),
    Remove(u64),
    FindAll { limit: Option<usize> },
    ClearAll
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DeviceAliasPersistentOperation {
    SaveAll(Vec<String>),
    GetAll,
    ClearAll
}

impl Operation for PersistentOperation {
    type Output = ();
}

impl SessionPersistentOperation {
    pub fn save_token(token: Token) -> AppRequestBuilder<impl Future<Output = Result<(), CoreError>>> {
        AppCommand::request_from_shell(PersistentOperation::Session(SessionPersistentOperation::WriteToken(token)))
            .map(|it| it.result())
    }

    pub fn save_user(user: User) -> AppRequestBuilder<impl Future<Output = Result<(), CoreError>>> {
        AppCommand::request_from_shell(PersistentOperation::Session(SessionPersistentOperation::WriteUser(user))).map(|it| it.result())
    }

    pub fn get_session() -> AppRequestBuilder<impl Future<Output = Result<Option<Session>, CoreError>>> {
        AppCommand::request_from_shell(PersistentOperation::Session(SessionPersistentOperation::Get())).map(|it| it.result_option())
    }

    pub fn remove_session() -> AppRequestBuilder<impl Future<Output = Result<(), CoreError>>> {
        AppCommand::request_from_shell(PersistentOperation::Session(SessionPersistentOperation::Remove)).map(|it| it.result())
    }
}

impl LocalResourcePersistentOperation {
    pub fn add(resources: Vec<LocalResource>) -> AppRequestBuilder<impl Future<Output = Result<Vec<LocalResource>, CoreError>>> {
        AppCommand::request_from_shell(PersistentOperation::LocalResource(LocalResourcePersistentOperation::Add(
            resources
        )))
        .map(|it| it.result())
    }

    pub fn update(resource: LocalResource) -> AppRequestBuilder<impl Future<Output = Result<LocalResource, CoreError>>> {
        AppCommand::request_from_shell(PersistentOperation::LocalResource(LocalResourcePersistentOperation::Update(resource)))
            .map(|it| it.result())
    }

    pub fn remove(path: LocalResourcePath, shelf_id: u64) -> AppRequestBuilder<impl Future<Output = Result<bool, CoreError>>> {
        AppCommand::request_from_shell(PersistentOperation::LocalResource(LocalResourcePersistentOperation::Remove {
            path,
            shelf_id
        }))
        .map(|it| it.result())
    }

    pub fn find_all() -> AppRequestBuilder<impl Future<Output = Result<Vec<LocalResource>, CoreError>>> {
        AppCommand::request_from_shell(PersistentOperation::LocalResource(LocalResourcePersistentOperation::FindAll))
            .map(|it| it.result())
    }

    pub fn add_thumbnail(
        png_bytes: Vec<u8>,
        resource_id: u64
    ) -> AppRequestBuilder<impl Future<Output = Result<LocalResourcePath, CoreError>>> {
        AppCommand::request_from_shell(PersistentOperation::LocalResource(
            LocalResourcePersistentOperation::AddThumbnail { png_bytes, resource_id }
        ))
        .map(|it| it.result())
    }

    pub fn load_from_disk(
        path: LocalResourcePath
    ) -> AppRequestBuilder<impl Future<Output = Result<Option<LocalResource>, CoreError>>> {
        AppCommand::request_from_shell(PersistentOperation::LocalResource(
            LocalResourcePersistentOperation::LoadOnDisk(path)
        ))
        .map(|it| it.result_option())
    }

    pub fn get_resource_type(path: LocalResourcePath) -> AppRequestBuilder<impl Future<Output = Result<ResourceType, CoreError>>> {
        AppCommand::request_from_shell(PersistentOperation::LocalResource(
            LocalResourcePersistentOperation::GetResourceType { path }
        ))
        .map(|it| it.result())
    }
}

impl TransferSessionPersistentOperation {
    pub fn clear_all() -> AppRequestBuilder<impl Future<Output = Result<bool, CoreError>>> {
        AppCommand::request_from_shell(PersistentOperation::TransferSession(TransferSessionPersistentOperation::Clear))
            .map(|it| it.result())
    }

    pub fn generate_thumbnail_paths(
        session_id: Option<u64>,
        resource_ids: Vec<u64>
    ) -> AppRequestBuilder<impl Future<Output = Result<HashMap<u64, LocalResourcePath>, CoreError>>> {
        AppCommand::request_from_shell(PersistentOperation::TransferSession(
            TransferSessionPersistentOperation::GenerateThumbnailPath { session_id, resource_ids }
        ))
        .map(|it| it.result())
    }

    pub fn generate_resource_paths(
        id: u64,
        resource_names: HashMap<u64, (String, ResourceType)>
    ) -> AppRequestBuilder<impl Future<Output = Result<HashMap<u64, LocalResourcePath>, CoreError>>> {
        AppCommand::request_from_shell(PersistentOperation::TransferSession(
            TransferSessionPersistentOperation::GenerateResourcePath {
                session_id: id,
                resource_names
            }
        ))
        .map(|it| it.result())
    }

    pub fn generate_zip_download_paths(
        session_order_id: u64,
        resource_names: HashMap<u64, String>
    ) -> AppRequestBuilder<impl Future<Output = Result<ZipDownloadPaths, CoreError>>> {
        AppCommand::request_from_shell(PersistentOperation::TransferSession(
            TransferSessionPersistentOperation::GenerateZipDownloadPaths {
                session_order_id,
                resource_names
            }
        ))
        .map(|it| it.result())
    }
}

impl ShelfPersistentOperation {
    pub fn add(shelf: Shelf) -> AppRequestBuilder<impl Future<Output = Result<Shelf, CoreError>>> {
        AppCommand::request_from_shell(PersistentOperation::Shelf(ShelfPersistentOperation::Add(shelf))).map(|it| it.result())
    }

    pub fn update(shelf: Shelf) -> AppRequestBuilder<impl Future<Output = Result<Shelf, CoreError>>> {
        AppCommand::request_from_shell(PersistentOperation::Shelf(ShelfPersistentOperation::Update(shelf))).map(|it| it.result())
    }

    pub fn remove(id: u64) -> AppRequestBuilder<impl Future<Output = Result<bool, CoreError>>> {
        AppCommand::request_from_shell(PersistentOperation::Shelf(ShelfPersistentOperation::Remove(id))).map(|it| it.result())
    }

    pub fn find_all(limit: Option<usize>) -> AppRequestBuilder<impl Future<Output = Result<Vec<Shelf>, CoreError>>> {
        AppCommand::request_from_shell(PersistentOperation::Shelf(ShelfPersistentOperation::FindAll { limit })).map(|it| it.result())
    }

    pub fn clear_all() -> AppRequestBuilder<impl Future<Output = Result<(), CoreError>>> {
        AppCommand::request_from_shell(PersistentOperation::Shelf(ShelfPersistentOperation::ClearAll)).map(|it| it.result())
    }
}

impl DeviceAliasPersistentOperation {
    pub fn save_all(aliases: Vec<String>) -> AppRequestBuilder<impl Future<Output = Result<(), CoreError>>> {
        AppCommand::request_from_shell(PersistentOperation::DeviceAlias(DeviceAliasPersistentOperation::SaveAll(
            aliases
        )))
        .map(|it| it.result())
    }

    pub fn get_all() -> AppRequestBuilder<impl Future<Output = Result<Vec<String>, CoreError>>> {
        AppCommand::request_from_shell(PersistentOperation::DeviceAlias(DeviceAliasPersistentOperation::GetAll)).map(|it| it.result())
    }

    pub fn clear_all() -> AppRequestBuilder<impl Future<Output = Result<(), CoreError>>> {
        AppCommand::request_from_shell(PersistentOperation::DeviceAlias(DeviceAliasPersistentOperation::ClearAll))
            .map(|it| it.result())
    }
}
