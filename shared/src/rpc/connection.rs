use crate::rpc::errors::RpcErrors;
use tonic::client::GrpcService;
use core_services::utils::maybe::{MaybeSend, MaybeSendSync};

#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
pub trait RpcNetworkModule<T>: Send + Sync
where
    T: Clone,
    T: MaybeSend + Sync,
    T: GrpcService<tonic::body::Body>,
    T::Future: MaybeSend,
    T::Error: Into<tonic::codegen::StdError>,
    T::ResponseBody: http_body::Body<Data = bytes::Bytes> + 'static + MaybeSend,
    <T::ResponseBody as http_body::Body>::Error: Into<tonic::codegen::StdError> + MaybeSend
{
    async fn connect(&self) -> Result<T, RpcErrors>;
}
