use crate::app::operations::internet::InternetOperation;
use crate::app::operations::transfer::TransferOperation;
use crate::app::AppCommandContext;

use super::finding_scope::FindingScope;

pub struct NearbyService {}

impl NearbyService {
    pub async fn init(&self, ctx: AppCommandContext) {
        match InternetOperation::get_current_ip_address().into_future(ctx.clone()).await {
            Ok(public_ip) => {
                // We use the public ip address to create a local finding scope
                // Every device that has the same public ip address will be able to find each other
                let finding_scope = FindingScope::Local(public_ip);
                TransferOperation::start_nearby_server(vec![finding_scope]).into_future(ctx).await;
            }
            Err(e) => {
                log::error!(target: "nearby", "Failed to get current ip address: {:?}", e);
            }
        }
    }
}
