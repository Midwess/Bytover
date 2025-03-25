use crate::app::modules::transfer::TransferEvent;
use crate::app::operations::internet::InternetOperation;
use crate::app::operations::transfer::TransferOperation;
use crate::app::{AppCommandContext, AppEvent};

use super::finding_scope::FindingScope;

pub struct NearbyService {}

impl NearbyService {
    pub async fn init(&self, ctx: AppCommandContext) {
        TransferOperation::start_nearby_server(vec![]).into_future(ctx.clone()).await;

        if let Ok(local_ip) = InternetOperation::get_current_ip_address().into_future(ctx.clone()).await {
            ctx.send_event(AppEvent::Transfer(TransferEvent::OnIpAddressUpdated(local_ip)));
        }
    }
}
