use crate::app::operations::device::DeviceOperation;
use crate::app::operations::rpc::RpcOperation;
use crate::app::operations::webview::WebViewOperation;
use crate::app::{AppCommandContext, AppEvent};
use crate::app::ports::authentication_service::AuthenticationServer;

pub struct AuthenticationService {
    pub auth_server: &'static Box<dyn AuthenticationServer>
}

impl AuthenticationService {
    pub async fn signin(&self, ctx: AppCommandContext) {
        let device_info = DeviceOperation::get_device_info().into_future(ctx.clone()).await;
        let url = RpcOperation::get_sign_in_url(device_info).into_future(ctx.clone()).await;
        WebViewOperation::open_url(url).into_future(ctx).await;
    }

    pub async fn handle_auth_response(&self, redirect_url: String) {
        // Handle auth response
    }
}
