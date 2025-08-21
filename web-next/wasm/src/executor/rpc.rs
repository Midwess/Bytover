use tonic_web_wasm_client::Client;
use shared::rpc::auth_server::AuthServer;
use shared::executor::rpc::NativeRpc;

pub struct NativeRpcImpl {
    pub auth_server: AuthServer<Client>
}

#[async_trait::async_trait(?Send)]
impl NativeRpc<Client> for NativeRpcImpl {
    fn auth_server(&self) -> &AuthServer<Client> {
        &self.auth_server
    }
}
