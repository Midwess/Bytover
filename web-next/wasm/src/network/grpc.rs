use tonic_web_wasm_client::Client;
use shared::rpc::connection::RpcNetworkModule;
use shared::rpc::errors::RpcErrors;

#[derive(Clone)]
pub struct RpcNetworkModuleImpl {
    pub endpoint: String
}

impl RpcNetworkModuleImpl {
    pub fn new(endpoint: String) -> Self {
        Self {
            endpoint
        }
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
