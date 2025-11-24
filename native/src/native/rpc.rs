use shared::protocol::rpc::auth_server::AppServer;
use shared::shell::executor::rpc::NativeRpc;
use tonic::transport::Channel;

pub struct NativeRpcImpl {
    pub auth_server: AppServer<Channel>
}

#[async_trait::async_trait]
impl NativeRpc<Channel> for NativeRpcImpl {
    fn app_server(&self) -> &AppServer<Channel> {
        &self.auth_server
    }
}
