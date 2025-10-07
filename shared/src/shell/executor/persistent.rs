use crate::app::operations::persistent::{
    LocalResourcePersistentOperation,
    LocalResourcePersistentOperationOutput,
    PersistentOperation,
    PersistentOperationOutput,
    SessionPersistentOperation,
    SessionPersistentOperationOutput,
    TransferSessionOperationOutput,
    TransferSessionPersistentOperation
};
use crate::entities::session::{Session, SessionType};
use crate::entities::transfer_session::TransferType;
use crate::repository::auth_session::{AuthSessionId, AuthSessionRepository};
use crate::repository::local_resource::{LocalResourceId, LocalResourceRepository};
use crate::repository::transfer_session::{TransferSessionId, TransferSessionRepository};

#[cfg_attr(not(target_family = "wasm"), async_trait::async_trait)]
#[cfg_attr(target_family = "wasm", async_trait::async_trait(?Send))]
pub trait NativePersistent: Send + Sync {
    fn auth_session_repository(&self) -> &Box<dyn AuthSessionRepository>;
    fn local_resource_repository(&self) -> &dyn LocalResourceRepository;
    fn transfer_session_repository(&self) -> &dyn TransferSessionRepository;

    async fn handle(&self, effect: PersistentOperation) -> PersistentOperationOutput {
        Self::default_handle(self, effect).await
    }

    async fn default_handle(&self, effect: PersistentOperation) -> PersistentOperationOutput {
        match effect {
            PersistentOperation::Session(SessionPersistentOperation::WriteToken(token)) => {
                if let Err(err) = self
                    .auth_session_repository()
                    .delete_one(&AuthSessionId {
                        r#type: SessionType::Access
                    })
                    .await
                {
                    log::error!("Failed to delete token from database: {err:?}");
                }

                if let Err(err) = self
                    .auth_session_repository()
                    .create(Session {
                        r#type: SessionType::Access,
                        token,
                        user: None
                    })
                    .await
                {
                    log::error!("Failed to write token to database: {err:?}");
                    return PersistentOperationOutput::Session(SessionPersistentOperationOutput::WriteToken());
                }

                PersistentOperationOutput::Session(SessionPersistentOperationOutput::WriteToken())
            }
            PersistentOperation::Session(SessionPersistentOperation::Get()) => {
                match self
                    .auth_session_repository()
                    .find_one(&AuthSessionId {
                        r#type: SessionType::Access
                    })
                    .await
                {
                    Ok(session) => PersistentOperationOutput::Session(SessionPersistentOperationOutput::Get(session)),
                    Err(_error) => PersistentOperationOutput::Session(SessionPersistentOperationOutput::Get(None))
                }
            }
            PersistentOperation::Session(SessionPersistentOperation::WriteUser(user)) => {
                let session = self
                    .auth_session_repository()
                    .find_one(&AuthSessionId {
                        r#type: SessionType::Access
                    })
                    .await;
                if let Ok(Some(mut session)) = session {
                    session.user = Some(user);
                    let _ = self.auth_session_repository().update_one(session).await;
                }

                PersistentOperationOutput::Session(SessionPersistentOperationOutput::WriteUser())
            }
            PersistentOperation::LocalResource(LocalResourcePersistentOperation::Add(resources)) => {
                let mut created_resources = vec![];
                for resource in resources {
                    if let Ok(resource) = self.local_resource_repository().create(resource).await {
                        created_resources.push(resource);
                    }
                }

                PersistentOperationOutput::LocalResource(LocalResourcePersistentOperationOutput::Add(created_resources))
            }
            PersistentOperation::LocalResource(LocalResourcePersistentOperation::Remove(id)) => {
                if self
                    .local_resource_repository()
                    .delete_one(&LocalResourceId {
                        order_id: Some(id),
                        ..Default::default()
                    })
                    .await
                    .is_ok()
                {
                    PersistentOperationOutput::LocalResource(LocalResourcePersistentOperationOutput::Removed)
                } else {
                    PersistentOperationOutput::Error("Failed to remove local resource".to_string())
                }
            }
            PersistentOperation::LocalResource(LocalResourcePersistentOperation::FindAll) => {
                let resources = self.local_resource_repository().find_all(None, None, None).await.unwrap();
                PersistentOperationOutput::LocalResource(LocalResourcePersistentOperationOutput::FindAll(resources))
            }
            PersistentOperation::LocalResource(LocalResourcePersistentOperation::Find(path)) => {
                let id = LocalResourceId {
                    path: Some(path.as_string()),
                    ..Default::default()
                };

                let result = self.local_resource_repository().find_one(&id).await;
                match result {
                    Ok(resource) => PersistentOperationOutput::LocalResource(LocalResourcePersistentOperationOutput::Find(resource)),
                    Err(err) => {
                        log::error!("Failed to find local resource: {err:?}");
                        PersistentOperationOutput::LocalResource(LocalResourcePersistentOperationOutput::Find(None))
                    }
                }
            }
            PersistentOperation::LocalResource(LocalResourcePersistentOperation::LoadOnDisk(path)) => {
                match self.local_resource_repository().load(path).await {
                    Ok(result) => PersistentOperationOutput::LocalResource(LocalResourcePersistentOperationOutput::LoadOnDisk(result)),
                    Err(e) => {
                        log::error!("Failed to load local resource: {e:?}");
                        PersistentOperationOutput::Error(e.to_string())
                    }
                }
            }
            PersistentOperation::LocalResource(LocalResourcePersistentOperation::AddThumbnail { png_bytes, resource_id }) => {
                match self.local_resource_repository().save_thumbnail(png_bytes, resource_id).await {
                    Ok(result) => {
                        PersistentOperationOutput::LocalResource(LocalResourcePersistentOperationOutput::AddThumbnail(result))
                    }
                    Err(e) => {
                        log::error!("Failed to save thumbnail: {e:?}");
                        PersistentOperationOutput::Error(e.to_string())
                    }
                }
            }
            PersistentOperation::LocalResource(LocalResourcePersistentOperation::GetResourceType { path }) => {
                match self.local_resource_repository().get_resource_type(path).await {
                    Ok(result) => {
                        PersistentOperationOutput::LocalResource(LocalResourcePersistentOperationOutput::GetResourceType(result))
                    }
                    Err(e) => {
                        log::error!("Failed to get resource type: {e:?}");
                        PersistentOperationOutput::Error(e.to_string())
                    }
                }
            }
            PersistentOperation::TransferSession(TransferSessionPersistentOperation::Save(session)) => {
                let result = self.transfer_session_repository().create(session).await;
                match result {
                    Ok(session) => PersistentOperationOutput::TransferSession(TransferSessionOperationOutput::Save(Some(session))),
                    Err(err) => {
                        log::error!("Failed to save transfer session: {err:?}");
                        PersistentOperationOutput::TransferSession(TransferSessionOperationOutput::Save(None))
                    }
                }
            }
            PersistentOperation::TransferSession(TransferSessionPersistentOperation::GetAllReceivedSessions()) => {
                let id = TransferSessionId {
                    r#type: Some(TransferType::Receive),
                    ..Default::default()
                };

                let result = self.transfer_session_repository().find_all(Some(&id), None, None).await;
                match result {
                    Ok(sessions) => PersistentOperationOutput::TransferSession(TransferSessionOperationOutput::GetAll(sessions)),
                    Err(err) => {
                        log::error!("Failed to get all transfer sessions: {err:?}");
                        PersistentOperationOutput::TransferSession(TransferSessionOperationOutput::GetAll(vec![]))
                    }
                }
            }
            PersistentOperation::TransferSession(TransferSessionPersistentOperation::UpdateProgresses(order_id, progresses)) => {
                let result = self.transfer_session_repository().update_progresses(order_id, progresses).await;
                match result {
                    Ok(session) => {
                        PersistentOperationOutput::TransferSession(TransferSessionOperationOutput::UpdateProgresses(session))
                    }
                    Err(err) => {
                        log::error!("Failed to update transfer session: {err:?}");
                        PersistentOperationOutput::TransferSession(TransferSessionOperationOutput::UpdateProgresses(None))
                    }
                }
            }
            PersistentOperation::TransferSession(TransferSessionPersistentOperation::Remove(id)) => {
                if let Err(_) = self.transfer_session_repository().delete_session(id).await {
                    return PersistentOperationOutput::TransferSession(TransferSessionOperationOutput::Removed(false));
                }

                PersistentOperationOutput::TransferSession(TransferSessionOperationOutput::Removed(true))
            }
            PersistentOperation::TransferSession(TransferSessionPersistentOperation::UpdateResource { session_id, resource }) => {
                let result = self.transfer_session_repository().update_resource(session_id, resource).await;

                match result {
                    Ok(session) => PersistentOperationOutput::TransferSession(TransferSessionOperationOutput::UpdateResource(session)),
                    Err(err) => {
                        log::error!("Failed to update transfer session: {err:?}");
                        PersistentOperationOutput::TransferSession(TransferSessionOperationOutput::UpdateResource(None))
                    }
                }
            }
            PersistentOperation::TransferSession(TransferSessionPersistentOperation::GenerateResourcePath {
                session_id,
                resource_names
            }) => match self.transfer_session_repository().generate_resource_paths(session_id, resource_names).await {
                Ok(result) => PersistentOperationOutput::TransferSession(TransferSessionOperationOutput::GenerateResourcePath(result)),
                Err(e) => {
                    log::error!("Failed to generate resources path: {e:?}");
                    PersistentOperationOutput::Error(e.to_string())
                }
            },
            PersistentOperation::TransferSession(TransferSessionPersistentOperation::GenerateThumbnailPath {
                session_id,
                resource_ids
            }) => match self.local_resource_repository().generate_thumbnail_paths(session_id, resource_ids).await {
                Ok(result) => {
                    PersistentOperationOutput::TransferSession(TransferSessionOperationOutput::GenerateThumbnailPath(result))
                }
                Err(e) => {
                    log::error!("Failed to generate resources path: {e:?}");
                    PersistentOperationOutput::Error(e.to_string())
                }
            },
            PersistentOperation::User(_) => {
                panic!("Unimplemented");
            }
        }
    }
}
