use core_services::db::repository::abstraction::local_repository::LocalSurrealDbRepository;

use crate::app::operations::database::{
    DatabaseOperation,
    DatabaseOperationOutput,
    SessionOperation,
    SessionOperationOutput
};
use crate::entities::session::{Session, SessionType};
use crate::persistence::session::{SessionId, SessionRepository};

pub struct NativeDatabase {
    pub session_repository: SessionRepository
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
                log::info!(target: "db-debug", "Deleted session");
                self.session_repository
                    .create(Session {
                        r#type: SessionType::Access,
                        token,
                        user: None
                    })
                    .await
                    .unwrap();
                log::info!(target: "db-debug", "Created session");
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

            _ => panic!("Native database doesn't support this effect {:?}", effect)
        }
    }
}
