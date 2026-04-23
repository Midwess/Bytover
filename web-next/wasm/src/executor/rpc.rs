use shared::protocol::rpc::app_server::AppServer;
use shared::protocol::rpc::cloud_server::CloudServer;
use shared::shell::executor::rpc::NativeRpc;
use tonic_web_wasm_client::Client;

pub struct NativeRpcImpl {
    pub auth_server: AppServer<Client>,
    pub cloud_server: &'static CloudServer<Client>,
}

#[async_trait::async_trait(?Send)]
impl NativeRpc<Client> for NativeRpcImpl {
    fn app_server(&self) -> &AppServer<Client> {
        &self.auth_server
    }

    fn cloud_server(&self) -> &CloudServer<Client> {
        self.cloud_server
    }
}
