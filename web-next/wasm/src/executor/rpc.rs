use shared::executor::rpc::NativeRpc;
use shared::rpc::auth_server::AuthServer;
use tonic_web_wasm_client::Client;

pub struct NativeRpcImpl {
    pub auth_server: AuthServer<Client>
}

#[async_trait::async_trait(?Send)]
impl NativeRpc<Client> for NativeRpcImpl {
    fn auth_server(&self) -> &AuthServer<Client> {
        &self.auth_server
    }
}
