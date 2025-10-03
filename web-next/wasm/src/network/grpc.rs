use shared::protocol::rpc::connection::RpcNetworkModule;
use shared::protocol::rpc::errors::RpcErrors;
use tonic_web_wasm_client::Client;

#[derive(Clone)]
pub struct RpcNetworkModuleImpl {
    pub endpoint: String
}

impl RpcNetworkModuleImpl {
    pub fn new(endpoint: String) -> Self {
        Self { endpoint }
    }

    pub fn connect(&self) -> Client {
        Client::new(self.endpoint.clone())
    }
}

#[async_trait::async_trait(?Send)]
impl RpcNetworkModule<Client> for RpcNetworkModuleImpl {
    async fn connect(&self) -> Result<Client, RpcErrors> {
        Ok(self.connect())
    }
}
