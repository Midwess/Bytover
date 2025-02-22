use crate::{app::{modules::transfer::TransferEvent, operations::{database::TransferSessionDatabaseOperation, CoreOperation}, AppCommandContext, AppEvent}, entities::transfer::{TransferSession, TransferSessionStatus}};

pub struct TransferService {}

impl TransferService {
    // If already exist, return
    // If there is no session or all session are closed, create a new one
    pub async fn update_current_transfer_session(
        &self, 
        ctx: AppCommandContext
    ) {
        if let Some(existing_session) = TransferSessionDatabaseOperation::get_last_session().into_future(ctx.clone()).await {
            if existing_session.transfer_status() == TransferSessionStatus::New {
                ctx.send_event(AppEvent::Transfer(TransferEvent::UpdateSession(existing_session)));
                return;
            }
        }

        let session = TransferSession::new().await;
        ctx.send_event(AppEvent::Transfer(TransferEvent::UpdateSession(session)));
    }
}
