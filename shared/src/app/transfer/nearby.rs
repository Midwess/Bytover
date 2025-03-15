use crate::app::operations::transfer::TransferOperation;
use crate::app::AppCommandContext;

pub struct NearbyService {}

impl NearbyService {
    pub async fn init(&self, ctx: AppCommandContext) {
        TransferOperation::start_nearby_server().into_future(ctx).await;
    }
}
