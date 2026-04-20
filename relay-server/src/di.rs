use tokio::sync::OnceCell;

use crate::grpc_middleware::auth::RelayAuthInterceptor;

static DI_CONTAINER: OnceCell<DiContainer> = OnceCell::const_new();

pub struct DiContainer;

impl DiContainer {
    pub async fn init() -> &'static Self {
        DI_CONTAINER.get_or_init(|| async { Self }).await
    }

    pub fn get_auth_middleware(&self) -> RelayAuthInterceptor {
        RelayAuthInterceptor::new()
    }
}
