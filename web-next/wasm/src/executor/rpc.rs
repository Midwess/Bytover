use shared::protocol::rpc::app_server::AppServer;
use shared::shell::executor::rpc::NativeRpc;
use tonic_web_wasm_client::Client;

pub struct NativeRpcImpl {
    pub auth_server: AppServer<Client>,
}

#[async_trait::async_trait(?Send)]
impl NativeRpc<Client> for NativeRpcImpl {
    fn app_server(&self) -> &AppServer<Client> {
        &self.auth_server
    }
}
