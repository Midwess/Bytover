use core_services::db::repository::abstraction::local_repository::LocalSurrealDbRepository;

use crate::app::operations::database::{
    DatabaseOperation,
    DatabaseOperationOutput,
    SessionOperation,
    SessionOperationOutput, TransferSessionDatabaseOperation, TransferSessionDatabaseOperationOutput
};
use crate::entities::session::{Session, SessionType};
use crate::persistence::session::{SessionId, SessionRepository};
use crate::persistence::transfer_session::TransferSessionRepository;

pub struct NativeDatabase {
    pub session_repository: SessionRepository,
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
            DatabaseOperation::TransferSession(TransferSessionDatabaseOperation::GetLastSession()) => {
                let session = self.transfer_session_repository.get_last_session().await.unwrap();
                DatabaseOperationOutput::TransferSession(TransferSessionDatabaseOperationOutput::GetLastSession(session))
            }
            DatabaseOperation::TransferSession(TransferSessionDatabaseOperation::Save(session)) => {
                let _ = self.transfer_session_repository.update_or_create(session).await;
                DatabaseOperationOutput::TransferSession(TransferSessionDatabaseOperationOutput::Save())
            }
            _ => panic!("Native database doesn't support this effect {:?}", effect)
        }
    }
}
