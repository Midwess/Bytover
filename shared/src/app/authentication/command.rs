use crate::app::authentication::module::AuthenticationEvent;
use crate::app::operations::device::DeviceOperation;
use crate::app::operations::persistent::SessionPersistentOperation;
use crate::app::operations::rpc::RpcOperation;
use crate::app::operations::webview::WebViewOperation;
use crate::app::AppEvent;
use crate::entities::token::Token;
use crate::errors::CoreError;
use url::Url;

use crate::app::core::command::AppCommand;
use crate::app::core::extensions::CoreCommandContextUtils;
use crate::app::operations::dialog::DialogOperation;
use devlog_sdk::distributed_id::gen_id;
use crate::CoreOperation;

impl AppCommand {
    pub async fn authenticate(&self) {
        let Some(device_info) = self.run(DeviceOperation::get_device_info()).await else {
            self.run(DialogOperation::toast("Device not found".to_string())).await;
            return
        };

        let url = match RpcOperation::get_authenticate_url(device_info).into_future(self.ctx()).await {
            Ok(url) => url,
            Err(e) => {
                log::error!(target: "auth", "Failed to get sign in url: {e:?}");
                return;
            }
        };

        WebViewOperation::open_url(url).into_future(self.ctx()).await;
    }

    pub async fn sign_out(&self) -> Result<(), CoreError> {
        let _ = self.run(SessionPersistentOperation::remove_session()).await;
        self.re_authorize().await?;
        Ok(())
    }

    pub async fn re_authorize(&self) -> Result<(), CoreError> {
        let Ok(user) = RpcOperation::get_me().into_future(self.ctx()).await else {
            // User is not logged in is fine, some flow not require user to be logged in
            self.notify_event(AppEvent::Authentication(AuthenticationEvent::UnAuthorized));
            return Ok(())
        };

        SessionPersistentOperation::save_user(user.clone()).into_future(self.ctx()).await?;
        self.notify_event(AppEvent::Authentication(AuthenticationEvent::Authorized { user }));
        self.notify_shell(CoreOperation::Render);
        Ok(())
    }

    pub async fn authorize(&self, url: String) -> Result<(), CoreError> {
        let Ok(url) = Url::parse(url.as_str()) else {
            log::warn!("The redirect url is invalid: {url}");
            return Ok(());
        };

        if let Some(error_msg) = url.query_pairs().find(|it| it.0 == "message") {
            return Err(CoreError::BadRequest(error_msg.1.to_string()));
        }

        let Some(token) = url.query_pairs().find(|it| it.0 == "access_token").map(|it| it.1.to_string()) else {
            log::info!("The redirect url does not contain access token");
            return Ok(());
        };

        let token = Token {
            order_id: gen_id().await,
            value: token
        };

        if token.value.is_empty() {
            log::error!(target: "auth", "Failed to get access token from auth response {url}");
            return Ok(());
        }

        SessionPersistentOperation::save_token(token).into_future(self.ctx()).await?;
        let user = RpcOperation::get_me().into_future(self.ctx()).await?;
        self.notify_event(AppEvent::Authentication(AuthenticationEvent::Authorized { user }));

        Ok(())
    }
}
