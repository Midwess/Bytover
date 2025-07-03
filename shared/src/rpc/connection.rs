use async_trait::async_trait;
use tonic::client::GrpcService;
use crate::rpc::errors::RpcErrors;

#[async_trait]
pub trait RpcNetworkModule<T>: Send + Sync
where
    T: Clone,
    T: Send,
    T: Sync,
    T: GrpcService<tonic::body::Body>,
    T::Error: Into<tonic::codegen::StdError>,
    T::ResponseBody: http_body::Body<Data = bytes::Bytes> + Send + 'static,
    <T::ResponseBody as http_body::Body>::Error: Into<tonic::codegen::StdError> + Send
{
    async fn connect(&self) -> Result<T, RpcErrors>;
}
