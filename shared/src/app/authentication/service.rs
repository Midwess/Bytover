use crate::app::modules::authentication::AuthenticationEvent;
use crate::app::operations::device::DeviceOperation;
use crate::app::operations::persistent::SessionPersistentOperation;
use crate::app::operations::rpc::RpcOperation;
use crate::app::operations::webview::WebViewOperation;
use crate::app::operations::CoreOperation;
use crate::app::{AppCommandContext, AppEvent};
use crate::entities::token::Token;
use url::Url;

use devlog_sdk::distributed_id::gen_id;
use std::sync::OnceLock;

pub struct AuthenticationService {}

impl AuthenticationService {
    pub fn instance() -> &'static Self {
        static INSTANCE: OnceLock<AuthenticationService> = OnceLock::new();
        INSTANCE.get_or_init(|| AuthenticationService {})
    }

    pub async fn update_signin_session(&self, ctx: AppCommandContext) {
        // Call API to update the user info
        log::info!(target: "auth", "Updating sign in session");
        let mut user = match RpcOperation::get_me().into_future(ctx.clone()).await {
            Ok(user) => Some(user),
            Err(e) => {
                log::error!(target: "auth", "Failed to get user info: {e:?}");
                None
            }
        };

        if user.is_none() {
            let session = SessionPersistentOperation::get_session().into_future(ctx.clone()).await;
            if let Some(Some(user_info)) = session.map(|it| it.user) {
                user.replace(user_info);
            }

            // User not signined in
            return;
        } else {
            SessionPersistentOperation::save_user(user.clone().unwrap()).into_future(ctx.clone()).await;
        }

        let user = user.unwrap();
        ctx.send_event(AppEvent::Authentication(AuthenticationEvent::OnSignInSuccess { user }));
        ctx.notify_shell(CoreOperation::Render);
    }

    pub async fn signin(&self, ctx: AppCommandContext) {
        let device_info = DeviceOperation::get_device_info().into_future(ctx.clone()).await;
        let url = match RpcOperation::get_sign_in_url(device_info).into_future(ctx.clone()).await {
            Ok(url) => url,
            Err(e) => {
                log::error!(target: "auth", "Failed to get sign in url: {e:?}");
                return;
            }
        };

        WebViewOperation::open_url(url).into_future(ctx).await;
    }

    pub async fn handle_auth_response(&self, redirect_url: String, ctx: AppCommandContext) {
        let Ok(url) = Url::parse(redirect_url.as_str()) else {
            log::warn!("The redirect url is invalid: {redirect_url}");
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
            log::error!(target: "auth", "Failed to get access token from auth response {redirect_url}");
            return;
        }

        log::info!("Saving token");
        SessionPersistentOperation::save_token(token).into_future(ctx.clone()).await;
        log::info!("Updating user");
        self.update_signin_session(ctx).await;
    }
}
