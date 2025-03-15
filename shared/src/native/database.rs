use core_services::db::repository::abstraction::local_repository::LocalSurrealDbRepository;

use crate::app::operations::database::{
    DatabaseOperation, DatabaseOperationOutput, LocalResourceDatabaseOperation, LocalResourceDatabaseOperationOutput, SessionOperation, SessionOperationOutput
};
use crate::entities::session::{Session, SessionType};
use crate::persistence::session::{SessionId, SessionRepository};
use crate::persistence::local_resource::{LocalResourceId, LocalResourceRepository};

pub struct NativeDatabase {
    pub session_repository: SessionRepository,
    pub local_resource_repository: LocalResourceRepository
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

                return DatabaseOperationOutput::LocalResource(LocalResourceDatabaseOperationOutput::Add(created_resources))
            }
            DatabaseOperation::LocalResource(LocalResourceDatabaseOperation::Remove(id)) => {
                if let Ok(resource) = self.local_resource_repository.delete_one(&LocalResourceId {
                    r#type: None,
                    order_id: Some(id)
                }).await {
                    return DatabaseOperationOutput::LocalResource(LocalResourceDatabaseOperationOutput::Remove(Some(resource)))
                } else {
                    return DatabaseOperationOutput::LocalResource(LocalResourceDatabaseOperationOutput::Remove(None))
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
            _ => panic!("Native database doesn't support this effect {:?}", effect)
        }
    }
}
