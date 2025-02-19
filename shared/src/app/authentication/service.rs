use devlog_sdk::distributed_id::gen_id;

use crate::app::modules::authentication::AuthenticationEvent;
use crate::app::operations::database::SessionOperation;
use crate::app::operations::device::DeviceOperation;
use crate::app::operations::rpc::RpcOperation;
use crate::app::operations::webview::WebViewOperation;
use crate::app::operations::CoreOperation;
use crate::app::{AppCommandContext, AppEvent};
use crate::entities::token::Token;
use std::collections::HashMap;

pub struct AuthenticationService {}

impl AuthenticationService {
    pub async fn update_signin_session(&self, ctx: AppCommandContext) {
        // Call API to update the user info
        let mut user = RpcOperation::get_me().into_future(ctx.clone()).await.ok();
        if user.is_none() {
            let session = SessionOperation::get_session().into_future(ctx.clone()).await;
            if let Some(Some(user_info)) = session.map(|it| it.user) {
                user.replace(user_info);
            }

            // User not signined in
            return;
        } else {
            SessionOperation::save_user(user.clone().unwrap()).into_future(ctx.clone()).await;
        }

        let user = user.unwrap();
        ctx.send_event(AppEvent::Authentication(AuthenticationEvent::OnSignInSuccess { user }));
        ctx.request_from_shell(CoreOperation::Render).await;
    }

    pub async fn signin(&self, ctx: AppCommandContext) {
        let device_info = DeviceOperation::get_device_info().into_future(ctx.clone()).await;
        let url = match RpcOperation::get_sign_in_url(device_info).into_future(ctx.clone()).await {
            Ok(url) => url,
            Err(e) => {
                log::error!(target: "auth", "Failed to get sign in url: {}", e);
                return;
            }
        };

        WebViewOperation::open_url(url).into_future(ctx).await;
    }

    pub async fn handle_auth_response(&self, redirect_url: String, ctx: AppCommandContext) {
        let query_string = redirect_url.split('?').nth(1).unwrap();

        let params: HashMap<String, String> = query_string
            .split('&')
            .filter_map(|pair| {
                let mut parts = pair.split('=');
                match (parts.next(), parts.next()) {
                    (Some(key), Some(value)) => Some((key.to_string(), value.to_string())),
                    _ => None
                }
            })
            .collect();

        let token = Token {
            order_id: gen_id().await,
            value: params.get("access_token").unwrap().to_string()
        };

        if token.value.is_empty() {
            log::error!(target: "auth", "Failed to get access token from auth response {}", redirect_url);
            return;
        }

        SessionOperation::save_token(token).into_future(ctx.clone()).await;
        self.update_signin_session(ctx).await;
    }
}
