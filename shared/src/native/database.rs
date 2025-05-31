use core_services::db::repository::abstraction::local_repository::LocalSurrealDbRepository;
use devlog_sdk::distributed_id::gen_id;

use crate::app::operations::database::{
    DatabaseOperation,
    DatabaseOperationOutput,
    LocalResourceDatabaseOperation,
    LocalResourceDatabaseOperationOutput,
    SessionOperation,
    SessionOperationOutput,
    TransferSessionOperation,
    TransferSessionOperationOutput
};
use crate::entities::session::{Session, SessionType};
use crate::persistence::local_resource::{LocalResourceId, LocalResourceRepository};
use crate::persistence::session::{SessionId, SessionRepository};
use crate::persistence::transfer_session::{TransferSessionId, TransferSessionRepository};

pub struct NativeDatabase {
    pub session_repository: SessionRepository,
    pub local_resource_repository: LocalResourceRepository,
    pub transfer_session_repository: TransferSessionRepository
}

impl NativeDatabase {
    pub async fn handle(&self, effect: DatabaseOperation) -> DatabaseOperationOutput {
        match effect {
            DatabaseOperation::Session(SessionOperation::WriteToken(token)) => {
                let _ = self
                    .session_repository
                    .delete_one(&SessionId {
                        r#type: SessionType::Access
                    })
                    .await;
                self.session_repository
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
                    .session_repository
                    .find_one(&SessionId {
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
                    .session_repository
                    .find_one(&SessionId {
                        r#type: SessionType::Access
                    })
                    .await;
                if let Ok(Some(mut session)) = session {
                    session.user = Some(user);
                    let _ = self.session_repository.update_one(session).await;
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
                        order_id: Some(id)
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
                let resource = self.local_resource_repository.find_by_path(&path).await;
                DatabaseOperationOutput::LocalResource(LocalResourceDatabaseOperationOutput::Find(resource))
            }
            DatabaseOperation::GenId() => DatabaseOperationOutput::GenId(gen_id().await),
            DatabaseOperation::TransferSession(TransferSessionOperation::Save(session)) => {
                let result = self.transfer_session_repository.save_session(session).await;
                match result {
                    Ok(session) => DatabaseOperationOutput::TransferSession(TransferSessionOperationOutput::Save(Some(session))),
                    Err(err) => {
                        log::error!("Failed to save transfer session: {:?}", err);
                        DatabaseOperationOutput::TransferSession(TransferSessionOperationOutput::Save(None))
                    }
                }
            }
            DatabaseOperation::TransferSession(TransferSessionOperation::GetAll(id)) => {
                let result = self.transfer_session_repository.find_all(Some(&id), None, None).await;
                match result {
                    Ok(sessions) => DatabaseOperationOutput::TransferSession(TransferSessionOperationOutput::GetAll(sessions)),
                    Err(err) => {
                        log::error!("Failed to get all transfer sessions: {:?}", err);
                        DatabaseOperationOutput::TransferSession(TransferSessionOperationOutput::GetAll(vec![]))
                    }
                }
            }
            DatabaseOperation::TransferSession(TransferSessionOperation::UpdateProgresses(order_id, progresses)) => {
                let result = self.transfer_session_repository.update_progresses(order_id, progresses).await;
                match result {
                    Ok(session) => DatabaseOperationOutput::TransferSession(TransferSessionOperationOutput::UpdateProgresses(session)),
                    Err(err) => {
                        log::error!("Failed to update transfer session: {:?}", err);
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
                        log::error!("Failed to remove transfer session: {:?}", err);
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
                        log::error!("Failed to update transfer session: {:?}", err);
                        DatabaseOperationOutput::TransferSession(TransferSessionOperationOutput::UpdateResource(None))
                    }
                }
            }
            _ => panic!("Native database doesn't support this effect {effect:?}")
        }
    }
}
