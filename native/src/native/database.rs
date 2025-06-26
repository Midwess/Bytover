use devlog_sdk::local_id_generator::gen_id;
use shared::app::operations::database::{
    DatabaseOperation,
    DatabaseOperationOutput,
    LocalResourceDatabaseOperation,
    LocalResourceDatabaseOperationOutput,
    SessionOperation,
    SessionOperationOutput,
    TransferSessionOperation,
    TransferSessionOperationOutput
};
use shared::app::repository::auth_session::{AuthSessionId, AuthSessionRepository};
use shared::app::repository::local_resource::{LocalResourceId, LocalResourceRepository};
use shared::app::repository::transfer_session::{TransferSessionId, TransferSessionRepository};
use shared::app::transfer::session::TransferType;
use shared::entities::session::{Session, SessionType};

pub struct NativeDatabase {
    pub auth_session_repository: Box<dyn AuthSessionRepository>,
    pub local_resource_repository: Box<dyn LocalResourceRepository>,
    pub transfer_session_repository: Box<dyn TransferSessionRepository>
}

impl NativeDatabase {
    pub async fn handle(&self, effect: DatabaseOperation) -> DatabaseOperationOutput {
        match effect {
            DatabaseOperation::Session(SessionOperation::WriteToken(token)) => {
                let _ = self
                    .auth_session_repository
                    .delete_one(&AuthSessionId {
                        r#type: SessionType::Access
                    })
                    .await;
                self.auth_session_repository
                    .create(Session {
                        r#type: SessionType::Access,
                        token,
                        user: None
                    })
                    .await
                    .unwrap();
                DatabaseOperationOutput::Session(SessionOperationOutput::WriteToken())
            }
            DatabaseOperation::Session(SessionOperation::Get()) => {
                match self
                    .auth_session_repository
                    .find_one(&AuthSessionId {
                        r#type: SessionType::Access
                    })
                    .await
                {
                    Ok(session) => DatabaseOperationOutput::Session(SessionOperationOutput::Get(session)),
                    Err(_error) => DatabaseOperationOutput::Session(SessionOperationOutput::Get(None))
                }
            }
            DatabaseOperation::Session(SessionOperation::WriteUser(user)) => {
                let session = self
                    .auth_session_repository
                    .find_one(&AuthSessionId {
                        r#type: SessionType::Access
                    })
                    .await;
                if let Ok(Some(mut session)) = session {
                    session.user = Some(user);
                    let _ = self.auth_session_repository.update_one(session).await;
                }

                DatabaseOperationOutput::Session(SessionOperationOutput::WriteUser())
            }
            DatabaseOperation::LocalResource(LocalResourceDatabaseOperation::Add(resources)) => {
                let mut created_resources = vec![];
                for resource in resources {
                    if let Ok(resource) = self.local_resource_repository.create(resource).await {
                        created_resources.push(resource);
                    }
                }

                DatabaseOperationOutput::LocalResource(LocalResourceDatabaseOperationOutput::Add(created_resources))
            }
            DatabaseOperation::LocalResource(LocalResourceDatabaseOperation::Remove(id)) => {
                if let Ok(resource) = self
                    .local_resource_repository
                    .delete_one(&LocalResourceId {
                        r#type: None,
                        order_id: Some(id),
                        ..Default::default()
                    })
                    .await
                {
                    DatabaseOperationOutput::LocalResource(LocalResourceDatabaseOperationOutput::Remove(Some(resource)))
                } else {
                    DatabaseOperationOutput::LocalResource(LocalResourceDatabaseOperationOutput::Remove(None))
                }
            }
            DatabaseOperation::LocalResource(LocalResourceDatabaseOperation::FindAll) => {
                let resources = self.local_resource_repository.find_all(None, None, None).await.unwrap();
                DatabaseOperationOutput::LocalResource(LocalResourceDatabaseOperationOutput::FindAll(resources))
            }
            DatabaseOperation::LocalResource(LocalResourceDatabaseOperation::Find(path)) => {
                let id = LocalResourceId {
                    path: Some(path),
                    ..Default::default()
                };

                let result = self.local_resource_repository.find_one(&id).await;
                match result {
                    Ok(resource) => DatabaseOperationOutput::LocalResource(LocalResourceDatabaseOperationOutput::Find(resource)),
                    Err(err) => {
                        log::error!("Failed to find local resource: {err:?}");
                        DatabaseOperationOutput::LocalResource(LocalResourceDatabaseOperationOutput::Find(None))
                    }
                }
            }
            DatabaseOperation::GenId() => DatabaseOperationOutput::GenId(gen_id().await),
            DatabaseOperation::TransferSession(TransferSessionOperation::Save(session)) => {
                let result = self.transfer_session_repository.create(session).await;
                match result {
                    Ok(session) => DatabaseOperationOutput::TransferSession(TransferSessionOperationOutput::Save(Some(session))),
                    Err(err) => {
                        log::error!("Failed to save transfer session: {err:?}");
                        DatabaseOperationOutput::TransferSession(TransferSessionOperationOutput::Save(None))
                    }
                }
            }
            DatabaseOperation::TransferSession(TransferSessionOperation::GetAllReceivedSessions()) => {
                let id = TransferSessionId {
                    r#type: Some(TransferType::Receive),
                    ..Default::default()
                };

                let result = self.transfer_session_repository.find_all(Some(&id), None, None).await;
                match result {
                    Ok(sessions) => DatabaseOperationOutput::TransferSession(TransferSessionOperationOutput::GetAll(sessions)),
                    Err(err) => {
                        log::error!("Failed to get all transfer sessions: {err:?}");
                        DatabaseOperationOutput::TransferSession(TransferSessionOperationOutput::GetAll(vec![]))
                    }
                }
            }
            DatabaseOperation::TransferSession(TransferSessionOperation::UpdateProgresses(order_id, progresses)) => {
                let result = self.transfer_session_repository.update_progresses(order_id, progresses).await;
                match result {
                    Ok(session) => DatabaseOperationOutput::TransferSession(TransferSessionOperationOutput::UpdateProgresses(session)),
                    Err(err) => {
                        log::error!("Failed to update transfer session: {err:?}");
                        DatabaseOperationOutput::TransferSession(TransferSessionOperationOutput::UpdateProgresses(None))
                    }
                }
            }
            DatabaseOperation::TransferSession(TransferSessionOperation::Remove(order_id)) => {
                let result = self
                    .transfer_session_repository
                    .delete_one(&TransferSessionId {
                        order_id: Some(order_id),
                        ..Default::default()
                    })
                    .await;
                match result {
                    Ok(session) => DatabaseOperationOutput::TransferSession(TransferSessionOperationOutput::Remove(Some(session))),
                    Err(err) => {
                        log::error!("Failed to remove transfer session: {err:?}");
                        DatabaseOperationOutput::TransferSession(TransferSessionOperationOutput::Remove(None))
                    }
                }
            }
            DatabaseOperation::TransferSession(TransferSessionOperation::UpdateResource { session_id, resource }) => {
                let id = TransferSessionId {
                    order_id: Some(session_id),
                    ..Default::default()
                };

                let result = self.transfer_session_repository.update_resource(id, resource).await;

                match result {
                    Ok(session) => DatabaseOperationOutput::TransferSession(TransferSessionOperationOutput::UpdateResource(session)),
                    Err(err) => {
                        log::error!("Failed to update transfer session: {err:?}");
                        DatabaseOperationOutput::TransferSession(TransferSessionOperationOutput::UpdateResource(None))
                    }
                }
            }
            _ => panic!("Native database doesn't support this effect {effect:?}")
        }
    }
}
