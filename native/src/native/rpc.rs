use shared::protocol::rpc::auth_server::AuthServer;
use shared::shell::executor::rpc::NativeRpc;
use tonic::transport::Channel;

pub struct NativeRpcImpl {
    pub auth_server: AuthServer<Channel>
}

#[async_trait::async_trait]
impl NativeRpc<Channel> for NativeRpcImpl {
    fn auth_server(&self) -> &AuthServer<Channel> {
        &self.auth_server
    }
}
