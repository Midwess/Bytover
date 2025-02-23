use crate::app::modules::transfer::TransferEvent;
use crate::app::operations::database::TransferSessionDatabaseOperation;
use crate::app::{AppCommandContext, AppEvent};
use crate::entities::transfer::{TransferSession, TransferSessionStatus};

pub struct TransferService {}

impl TransferService {
    // If already exist, return
    // If there is no session or all session are closed, create a new one
    pub async fn update_current_transfer_session(&self, ctx: AppCommandContext) {
        if let Some(existing_session) =
            TransferSessionDatabaseOperation::get_last_session().into_future(ctx.clone()).await
        {
            log::info!(target: "tiendang-debug", "Found existing session: {:?}", existing_session);
            if matches!(existing_session.transfer_status(), TransferSessionStatus::New) {
                log::info!(target: "tiendang-debug", "Session is new, update it");
                ctx.send_event(AppEvent::Transfer(TransferEvent::UpdateSession(existing_session)));
                return;
            }
        }

        let session = TransferSession::new().await;
        ctx.send_event(AppEvent::Transfer(TransferEvent::UpdateSession(session)));
    }
}
