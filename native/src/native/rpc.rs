use shared::protocol::rpc::app_server::AppServer;
use shared::protocol::rpc::cloud_server::CloudServer;
use shared::shell::executor::rpc::NativeRpc;
use tonic::transport::Channel;

pub struct NativeRpcImpl {
    pub auth_server: AppServer<Channel>,
    pub cloud_server: &'static CloudServer<Channel>,
}

#[async_trait::async_trait]
impl NativeRpc<Channel> for NativeRpcImpl {
    fn app_server(&self) -> &AppServer<Channel> {
        &self.auth_server
    }

    fn cloud_server(&self) -> &CloudServer<Channel> {
        self.cloud_server
    }
}
