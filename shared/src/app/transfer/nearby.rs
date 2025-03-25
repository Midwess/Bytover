use crate::app::operations::transfer::TransferOperation;
use crate::app::AppCommandContext;

use super::finding_scope::FindingScope;

pub struct NearbyService {}

impl NearbyService {
    pub async fn init(&self, ctx: AppCommandContext) {
        let mut finding_scopes = vec![];
        // The network scope is required
        let Ok(local_network_scope) = FindingScope::local_network(ctx.clone()).await else {
            log::error!(target: "nearby", "Failed to get local network scope");
            return;
        };

        finding_scopes.push(local_network_scope);

        log::info!(target: "nearby", "Starting nearby server with scopes: {:?}", finding_scopes);
        TransferOperation::start_nearby_server(finding_scopes).into_future(ctx).await;
    }
}
