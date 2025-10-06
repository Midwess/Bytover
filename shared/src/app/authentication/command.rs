use crate::app::authentication::module::AuthenticationEvent;
use crate::app::operations::device::DeviceOperation;
use crate::app::operations::persistent::SessionPersistentOperation;
use crate::app::operations::rpc::RpcOperation;
use crate::app::operations::webview::WebViewOperation;
use crate::app::operations::CoreOperation;
use crate::app::AppEvent;
use crate::entities::token::Token;
use url::Url;

use crate::app::core::command::AppCommand;
use crate::app::core::extensions::CoreCommandContextUtils;
use devlog_sdk::distributed_id::gen_id;

impl AppCommand {
    pub async fn sign_in(&self) {
        let device_info = DeviceOperation::get_device_info().into_future(self.ctx()).await;
        let url = match RpcOperation::get_sign_in_url(device_info).into_future(self.ctx()).await {
            Ok(url) => url,
            Err(e) => {
                log::error!(target: "auth", "Failed to get sign in url: {e:?}");
                return;
            }
        };

        WebViewOperation::open_url(url).into_future(self.ctx()).await;
    }

    pub async fn re_authorize(&self) {
        let mut user = match RpcOperation::get_me().into_future(self.ctx()).await {
            Ok(user) => Some(user),
            Err(e) => {
                log::info!(target: "auth", "Failed to get user info: {e:?}");
                None
            }
        };

        if user.is_none() {
            let session = SessionPersistentOperation::get_session().into_future(self.ctx()).await;
            if let Some(Some(user_info)) = session.map(|it| it.user) {
                user.replace(user_info);
            }

            return;
        } else {
            SessionPersistentOperation::save_user(user.clone().unwrap()).into_future(self.ctx()).await;
        }

        let user = user.unwrap();
        self.notify_event(AppEvent::Authentication(AuthenticationEvent::UpdateUser { user }));
        self.notify_shell(CoreOperation::Render);
    }

    pub async fn authorize(&self, url: String) {
        let Ok(url) = Url::parse(url.as_str()) else {
            log::warn!("The redirect url is invalid: {url}");
            return;
        };

        let Some(token) = url.query_pairs().find(|it| it.0 == "access_token").map(|it| it.1.to_string()) else {
            log::info!("The redirect url does not contain access token");
            return;
        };

        let token = Token {
            order_id: gen_id().await,
            value: token
        };

        if token.value.is_empty() {
            log::error!(target: "auth", "Failed to get access token from auth response {url}");
            return;
        }

        SessionPersistentOperation::save_token(token).into_future(self.ctx()).await;
        self.re_authorize().await;
    }
}
