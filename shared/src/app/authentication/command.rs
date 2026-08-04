use crate::app::authentication::module::AuthenticationEvent;
use crate::app::authentication::provider::LoginProvider;
use crate::app::core::command::AppCommand;
use crate::app::core::extensions::CoreCommandContextUtils;
use crate::app::operations::device::DeviceOperation;
use crate::app::operations::dialog::DialogOperation;
use crate::app::operations::p2p::P2POperation;
use crate::app::operations::persistent::{
    DeviceAliasPersistentOperation, SessionPersistentOperation, ShelfPersistentOperation, TransferSessionPersistentOperation,
};
use crate::app::operations::rpc::RpcOperation;
use crate::app::operations::webview::{AuthenticationSessionOutcome, WebViewOperation};
use crate::app::payment::module::PaymentEvent;
use crate::app::shelf::module::ShelfEvent;
use crate::app::transfer::module::TransferEvent;
use crate::app::AppEvent;
use crate::entities::token::Token;
use crate::errors::CoreError;
use crate::CoreOperation;
use devlog_sdk::distributed_id::gen_id;
use url::Url;

const AUTH_CALLBACK_SCHEME: &str = "bytover";
const AUTH_CALLBACK_HOST: &str = "app";
const AUTH_CALLBACK_PATH: &str = "/oauth/callback";

#[derive(Debug, PartialEq, Eq)]
enum AuthCallback {
    Success { access_token: String },
    Cancelled,
    Failed { code: String, message: String },
}

impl AppCommand {
    pub async fn authenticate(&self, provider: LoginProvider) {
        let Some(device_info) = self.run(DeviceOperation::get_device_info()).await else {
            self.run(DialogOperation::toast("Device not found".to_string())).await;
            return;
        };

        let start = match RpcOperation::get_authenticate_url(device_info, provider).into_future(self.ctx()).await {
            Ok(start) => start,
            Err(_) => {
                log::warn!(target: "auth", "provider={} outcome=start_request_failed", provider.label());
                self.update_model(AuthenticationEvent::AuthenticationFailed {
                    code: "start_request_failed".to_string(),
                    message: "Sign in could not start. Please try again.".to_string(),
                });
                return;
            }
        };

        let crate::protocol::rpc::app_server::AuthenticationStart::OpenUrl(url) = start else {
            let crate::protocol::rpc::app_server::AuthenticationStart::ProviderUnavailable {
                code,
                display_message: _,
            } = start
            else {
                return;
            };
            self.update_model(AuthenticationEvent::AuthenticationFailed {
                code,
                message: format!("Sign in with {} is temporarily unavailable.", provider.label()),
            });
            return;
        };

        match WebViewOperation::authenticate(url, AUTH_CALLBACK_SCHEME.to_string()).into_future(self.ctx()).await {
            AuthenticationSessionOutcome::Completed { callback_url } => {
                if self.authorize(callback_url).await.is_err() {
                    log::warn!(target: "auth", "provider={} outcome=callback_failed", provider.label());
                    self.update_model(AuthenticationEvent::AuthenticationFailed {
                        code: "authentication_failed".to_string(),
                        message: "Sign in could not be completed. Please try again.".to_string(),
                    });
                }
            }
            AuthenticationSessionOutcome::Cancelled => self.update_model(AuthenticationEvent::AuthenticationCancelled),
            AuthenticationSessionOutcome::Failed { code } => {
                self.update_model(AuthenticationEvent::AuthenticationFailed {
                    code,
                    message: "The secure sign-in window could not start. Please try again.".to_string(),
                });
            }
            AuthenticationSessionOutcome::ExternalBrowserStarted => {}
        }
    }

    pub async fn sign_out(&self) -> Result<(), CoreError> {
        let _ = self.run(P2POperation::stop()).await;
        self.run(SessionPersistentOperation::remove_session()).await?;
        self.run(TransferSessionPersistentOperation::clear_all()).await?;
        self.run(ShelfPersistentOperation::clear_all()).await?;
        self.run(DeviceAliasPersistentOperation::clear_all()).await?;
        self.notify_event(TransferEvent::Clear);
        self.notify_event(ShelfEvent::Cleared);
        self.notify_event(AppEvent::Payment(PaymentEvent::ClearCapabilities));
        self.re_authorize().await?;
        Ok(())
    }

    pub async fn re_authorize(&self) -> Result<(), CoreError> {
        let Ok((user, device_unique_key)) = RpcOperation::get_me().into_future(self.ctx()).await else {
            self.notify_event(AuthenticationEvent::UnAuthorized);
            return Ok(());
        };

        if !self.ensure_token_matches_local_device(&device_unique_key).await {
            self.run(SessionPersistentOperation::remove_session()).await?;
            self.notify_event(AuthenticationEvent::UnAuthorized);
            return Ok(());
        }

        SessionPersistentOperation::save_user(user.clone()).into_future(self.ctx()).await?;
        self.notify_event(AppEvent::Authentication(AuthenticationEvent::Authorized { user }));
        self.notify_shell(CoreOperation::Render);
        self.notify_shell(CoreOperation::LaunchNearbyServer);

        self.fetch_and_assign_aliases().await;

        Ok(())
    }

    pub async fn authorize(&self, url: String) -> Result<(), CoreError> {
        let callback = match parse_auth_callback(&url) {
            Ok(callback) => callback,
            Err(error) => {
                self.update_model(AuthenticationEvent::AuthenticationFailed {
                    code: "invalid_callback".to_string(),
                    message: "The sign-in callback was invalid. Please try again.".to_string(),
                });
                return Err(error);
            }
        };
        let token = match callback {
            AuthCallback::Success { access_token } => access_token,
            AuthCallback::Cancelled => {
                self.update_model(AuthenticationEvent::AuthenticationCancelled);
                return Ok(());
            }
            AuthCallback::Failed { code, message } => {
                self.update_model(AuthenticationEvent::AuthenticationFailed { code, message });
                return Ok(());
            }
        };

        let token = Token {
            order_id: gen_id().await,
            value: token,
        };

        if token.value.is_empty() {
            log::warn!(target: "auth", "outcome=empty_access_token");
            return Ok(());
        }

        let prior_session = SessionPersistentOperation::get_session().into_future(self.ctx()).await?;
        let prior_user_id = prior_session.as_ref().and_then(|s| s.user.as_ref()).map(|u| u.id);

        SessionPersistentOperation::save_token(token).into_future(self.ctx()).await?;

        let (user, device_unique_key) = RpcOperation::get_me().into_future(self.ctx()).await?;

        if !self.ensure_token_matches_local_device(&device_unique_key).await {
            self.run(SessionPersistentOperation::remove_session()).await?;
            self.run(DialogOperation::toast(
                "Unauthorized: this token belongs to a different device".to_string(),
            ))
            .await;
            self.notify_event(AuthenticationEvent::UnAuthorized);
            return Ok(());
        }

        if prior_user_id.map_or(true, |id| id != user.id) {
            let _ = self.run(P2POperation::stop()).await;
            self.run(TransferSessionPersistentOperation::clear_all()).await?;
            self.run(ShelfPersistentOperation::clear_all()).await?;
            self.run(DeviceAliasPersistentOperation::clear_all()).await?;
            self.notify_event(TransferEvent::Clear);
            self.notify_event(ShelfEvent::Launch);
            self.notify_event(AppEvent::Payment(PaymentEvent::ClearCapabilities));
        }

        self.notify_event(AppEvent::Authentication(AuthenticationEvent::Authorized { user }));
        self.notify_shell(CoreOperation::LaunchNearbyServer);

        self.fetch_and_assign_aliases().await;

        Ok(())
    }

    async fn ensure_token_matches_local_device(&self, server_device_key: &str) -> bool {
        let Some(local) = self.run(DeviceOperation::get_device_info()).await else {
            log::warn!(target: "auth", "Cannot verify token device: local DeviceInfo unavailable");
            return false;
        };
        if server_device_key.is_empty() {
            log::warn!(target: "auth", "Server returned empty device key; skipping device match check");
        }
        let matches = server_token_matches_device(server_device_key, &local.unique_id);
        if !matches {
            log::error!(
                target: "auth",
                "Token device mismatch: local={} server={}",
                local.unique_id,
                server_device_key
            );
        }
        matches
    }

    async fn fetch_and_assign_aliases(&self) {
        let aliases = match RpcOperation::get_device_aliases().into_future(self.ctx()).await {
            Ok(aliases) => aliases,
            Err(e) => {
                log::error!(target: "auth", "Failed to fetch device aliases: {e:?}");
                return;
            }
        };

        if let Err(e) = DeviceAliasPersistentOperation::save_all(aliases.clone()).into_future(self.ctx()).await {
            log::error!(target: "auth", "Failed to save device aliases: {e:?}");
        }

        let mut shelves = match ShelfPersistentOperation::find_all(None).into_future(self.ctx()).await {
            Ok(shelves) => shelves,
            Err(e) => {
                log::error!(target: "auth", "Failed to load shelves: {e:?}");
                return;
            }
        };

        let mut alias_iter = aliases.iter();

        for shelf in shelves.iter_mut() {
            if let Some(alias) = alias_iter.next() {
                shelf.update_name(alias);
                log::info!("Updated shelf {} with alias {}", shelf.id, shelf.name);
                match ShelfPersistentOperation::update(shelf.clone()).into_future(self.ctx()).await {
                    Ok(_) => self.update_model(ShelfEvent::ShelfUpdated(shelf.clone())),
                    Err(e) => log::error!(target: "auth", "Failed to update shelf alias: {e:?}"),
                }
            }
        }
    }
}

fn parse_auth_callback(raw_url: &str) -> Result<AuthCallback, CoreError> {
    let url = Url::parse(raw_url).map_err(|_| CoreError::BadRequest("The sign-in callback was invalid.".to_string()))?;
    if url.scheme() != AUTH_CALLBACK_SCHEME || url.host_str() != Some(AUTH_CALLBACK_HOST) || url.path() != AUTH_CALLBACK_PATH {
        return Err(CoreError::BadRequest("The sign-in callback did not match Bytover.".to_string()));
    }

    let outcome = url.query_pairs().find(|(key, _)| key == "outcome").map(|(_, value)| value.into_owned());
    let access_token = url.query_pairs().find(|(key, _)| key == "access_token").map(|(_, value)| value.into_owned());
    if url.query_pairs().any(|(key, _)| key == "message") {
        return Ok(AuthCallback::Failed {
            code: "provider_error".to_string(),
            message: "Sign in could not be completed. Please try again.".to_string(),
        });
    }

    match outcome.as_deref() {
        Some("success") => access_token
            .filter(|value| !value.is_empty())
            .map(|access_token| AuthCallback::Success { access_token })
            .ok_or_else(|| CoreError::BadRequest("The sign-in response was incomplete.".to_string())),
        Some("authorization_denied") => Ok(AuthCallback::Cancelled),
        Some("profile_unavailable") => Ok(AuthCallback::Failed {
            code: "profile_unavailable".to_string(),
            message: "Apple did not provide the account details needed for first sign-in. Please allow email sharing and try again."
                .to_string(),
        }),
        Some(code) => Ok(AuthCallback::Failed {
            code: code.to_string(),
            message: "Sign in could not be completed. Please try again.".to_string(),
        }),
        None => Err(CoreError::BadRequest("The sign-in response was incomplete.".to_string())),
    }
}

fn server_token_matches_device(server_device_key: &str, local_unique_id: &str) -> bool {
    server_device_key.is_empty() || local_unique_id == server_device_key
}

#[cfg(test)]
mod tests {
    use super::{parse_auth_callback, server_token_matches_device, AuthCallback};

    #[test]
    fn empty_server_device_key_accepts_any_local_device() {
        assert!(server_token_matches_device("", "any-local-id"));
        assert!(server_token_matches_device("", ""));
    }

    #[test]
    fn matching_device_keys_accept() {
        assert!(server_token_matches_device("abc-123", "abc-123"));
    }

    #[test]
    fn mismatched_non_empty_device_keys_reject() {
        assert!(!server_token_matches_device("abc-123", "xyz-789"));
        assert!(!server_token_matches_device("abc-123", ""));
    }

    #[test]
    fn callback_accepts_only_exact_success_uri() {
        assert_eq!(
            parse_auth_callback("bytover://app/oauth/callback?outcome=success&access_token=secret").unwrap(),
            AuthCallback::Success {
                access_token: "secret".to_string()
            }
        );

        assert!(parse_auth_callback("bytover://attacker/oauth/callback?outcome=success&access_token=secret").is_err());
        assert!(parse_auth_callback("bytover://app/oauth/callback/extra?outcome=success&access_token=secret").is_err());
    }

    #[test]
    fn callback_maps_denial_and_missing_profile_without_tokens() {
        assert_eq!(
            parse_auth_callback("bytover://app/oauth/callback?outcome=authorization_denied").unwrap(),
            AuthCallback::Cancelled
        );
        assert!(matches!(
            parse_auth_callback("bytover://app/oauth/callback?outcome=profile_unavailable").unwrap(),
            AuthCallback::Failed { code, .. } if code == "profile_unavailable"
        ));
    }
}
