use crate::app::operations::persistent::{
    LocalResourcePersistentOperation,
    PersistentOperation,
    SessionPersistentOperation,
    ShelfPersistentOperation,
    TransferSessionPersistentOperation
};
use crate::app::operations::CoreOperationOutput;
use crate::entities::session::{Session, SessionType};
use crate::errors::CoreError;
use crate::repository::auth_session::{AuthSessionId, AuthSessionRepository};
use crate::repository::local_resource::LocalResourceRepository;
use crate::repository::shelf::ShelfRepository;
use crate::repository::transfer_session::TransferSessionRepository;
use core_services::db::repository::abstraction::table::Table;

#[cfg_attr(not(target_family = "wasm"), async_trait::async_trait)]
#[cfg_attr(target_family = "wasm", async_trait::async_trait(?Send))]
pub trait NativePersistent: Send + Sync {
    fn auth_session_repository(&self) -> &Box<dyn AuthSessionRepository>;
    fn local_resource_repository(&self) -> &dyn LocalResourceRepository;
    fn transfer_session_repository(&self) -> &dyn TransferSessionRepository;
    fn shelf_repository(&self) -> &dyn ShelfRepository;

    async fn handle(&self, effect: PersistentOperation) -> Result<CoreOperationOutput, CoreError> {
        Self::default_handle(self, effect).await
    }

    async fn default_handle(&self, effect: PersistentOperation) -> Result<CoreOperationOutput, CoreError> {
        match effect {
            PersistentOperation::Session(SessionPersistentOperation::WriteToken(token)) => {
                let _ = self
                    .auth_session_repository()
                    .delete_one(&AuthSessionId {
                        r#type: SessionType::Access
                    })
                    .await;

                self.auth_session_repository()
                    .create(Session {
                        r#type: SessionType::Access,
                        token,
                        user: None
                    })
                    .await?;

                Ok(CoreOperationOutput::None)
            }
            PersistentOperation::Session(SessionPersistentOperation::Remove) => {
                self.auth_session_repository()
                    .delete_one(&AuthSessionId {
                        r#type: SessionType::Access
                    })
                    .await?;
                Ok(CoreOperationOutput::None)
            }
            PersistentOperation::Session(SessionPersistentOperation::Get()) => {
                let session = self
                    .auth_session_repository()
                    .find_one(&AuthSessionId {
                        r#type: SessionType::Access
                    })
                    .await
                    .unwrap_or(None);
                Ok(match session {
                    Some(session) => CoreOperationOutput::AuthSession(session),
                    None => CoreOperationOutput::None
                })
            }
            PersistentOperation::Session(SessionPersistentOperation::WriteUser(user)) => {
                let session = self
                    .auth_session_repository()
                    .find_one(&AuthSessionId {
                        r#type: SessionType::Access
                    })
                    .await?;

                if let Some(mut session) = session {
                    session.user = Some(user);
                    self.auth_session_repository().update_one(session).await?;
                }

                Ok(CoreOperationOutput::None)
            }
            PersistentOperation::LocalResource(LocalResourcePersistentOperation::Add(resources)) => {
                let mut created_resources = vec![];
                for resource in resources {
                    created_resources.push(self.local_resource_repository().create(resource).await?);
                }

                Ok(CoreOperationOutput::LocalResources(created_resources))
            }
            PersistentOperation::LocalResource(LocalResourcePersistentOperation::Remove { path, shelf_id }) => {
                self.local_resource_repository().remove(path, shelf_id).await?;

                Ok(CoreOperationOutput::Bool(true))
            }
            PersistentOperation::LocalResource(LocalResourcePersistentOperation::FindAll) => {
                let resources = self.local_resource_repository().find_all(None, None, None).await?;
                Ok(CoreOperationOutput::LocalResources(resources))
            }
            PersistentOperation::LocalResource(LocalResourcePersistentOperation::LoadOnDisk(path)) => {
                let result = self.local_resource_repository().load(path).await?;
                Ok(match result {
                    Some(resource) => CoreOperationOutput::LocalResource(resource),
                    None => CoreOperationOutput::None
                })
            }
            PersistentOperation::LocalResource(LocalResourcePersistentOperation::AddThumbnail { png_bytes, resource_id }) => {
                let result = self.local_resource_repository().save_thumbnail(png_bytes, resource_id).await?;
                Ok(CoreOperationOutput::LocalResourcePath(result))
            }
            PersistentOperation::LocalResource(LocalResourcePersistentOperation::GetResourceType { path }) => {
                let result = self.local_resource_repository().get_resource_type(path).await?;
                Ok(CoreOperationOutput::ResourceType(result))
            }
            PersistentOperation::TransferSession(TransferSessionPersistentOperation::Save(mut session)) => {
                // Reset back to disconnected state
                session.owner_disconnected();
                let session = self.transfer_session_repository().create(session).await?;
                Ok(session.into())
            }
            PersistentOperation::TransferSession(TransferSessionPersistentOperation::GetAllReceivedSessions()) => {
                let sessions = self.transfer_session_repository().find_all(None, None, None).await?;
                log::info!("Found sessions: {:?}", sessions.len());
                Ok(CoreOperationOutput::TransferSessions(sessions))
            }
            PersistentOperation::TransferSession(TransferSessionPersistentOperation::UpdateProgresses(order_id, progresses)) => {
                let session = self.transfer_session_repository().update_progresses(order_id, progresses).await?;
                Ok(match session {
                    Some(session) => CoreOperationOutput::TransferSession(session),
                    None => CoreOperationOutput::None
                })
            }
            PersistentOperation::TransferSession(TransferSessionPersistentOperation::Remove(id)) => {
                self.transfer_session_repository().delete_session(id).await?;
                Ok(CoreOperationOutput::Bool(true))
            }
            PersistentOperation::TransferSession(TransferSessionPersistentOperation::UpdateResource { session_id, resource }) => {
                let session = self.transfer_session_repository().update_resource(session_id, resource).await?;
                Ok(match session {
                    Some(session) => CoreOperationOutput::TransferSession(session),
                    None => CoreOperationOutput::None
                })
            }
            PersistentOperation::TransferSession(TransferSessionPersistentOperation::GenerateResourcePath {
                session_id,
                resource_names
            }) => {
                let result = self.transfer_session_repository().generate_resource_saved_paths(session_id, resource_names).await?;
                Ok(CoreOperationOutput::ResourcePathMap(result))
            }
            PersistentOperation::TransferSession(TransferSessionPersistentOperation::GenerateThumbnailPath {
                session_id,
                resource_ids
            }) => {
                let result = self.local_resource_repository().generate_thumbnail_paths(session_id, resource_ids).await?;
                Ok(CoreOperationOutput::ResourcePathMap(result))
            }
            PersistentOperation::TransferSession(TransferSessionPersistentOperation::GenerateZipDownloadPaths {
                session_order_id,
                resource_names
            }) => {
                let result = self.transfer_session_repository().generate_zip_download_paths(session_order_id, resource_names).await?;
                Ok(CoreOperationOutput::ZipDownloadPaths(result))
            }
            PersistentOperation::TransferSession(TransferSessionPersistentOperation::Clear) => {
                let sessions = self.transfer_session_repository().find_all(None, None, None).await?;
                for session in sessions {
                    let result = self.transfer_session_repository().delete_session(session.id()).await;
                    log::info!("Deleted session: {:?}", result);
                }

                Ok(CoreOperationOutput::Bool(true))
            }
            PersistentOperation::User(_) => Err(CoreError::NotImplemented("User operations not implemented yet".to_string())),
            PersistentOperation::Shelf(ShelfPersistentOperation::Add(shelf)) => {
                let shelf = self.shelf_repository().add(shelf).await?;
                Ok(CoreOperationOutput::Shelf(shelf))
            }
            PersistentOperation::Shelf(ShelfPersistentOperation::Remove(id)) => {
                let removed = self.shelf_repository().remove(id).await?;
                Ok(CoreOperationOutput::Bool(removed))
            }
            PersistentOperation::Shelf(ShelfPersistentOperation::FindAll { limit }) => {
                let shelves = self.shelf_repository().load_all(limit).await?;
                Ok(CoreOperationOutput::Shelves(shelves))
            }
        }
    }
}
