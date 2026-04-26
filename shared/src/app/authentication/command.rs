use crate::app::authentication::module::AuthenticationEvent;
use crate::app::core::command::AppCommand;
use crate::app::core::extensions::CoreCommandContextUtils;
use crate::app::operations::device::DeviceOperation;
use crate::app::operations::dialog::DialogOperation;
use crate::app::operations::p2p::P2POperation;
use crate::app::operations::persistent::{
    DeviceAliasPersistentOperation, SessionPersistentOperation, ShelfPersistentOperation, TransferSessionPersistentOperation,
};
use crate::app::operations::rpc::RpcOperation;
use crate::app::operations::webview::WebViewOperation;
use crate::app::payment::module::PaymentEvent;
use crate::app::shelf::module::ShelfEvent;
use crate::app::transfer::module::TransferEvent;
use crate::app::AppEvent;
use crate::entities::token::Token;
use crate::errors::CoreError;
use crate::CoreOperation;
use devlog_sdk::distributed_id::gen_id;
use url::Url;

impl AppCommand {
    pub async fn authenticate(&self) {
        let Some(device_info) = self.run(DeviceOperation::get_device_info()).await else {
            self.run(DialogOperation::toast("Device not found".to_string())).await;
            return;
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
            value: token,
        };

        if token.value.is_empty() {
            log::error!(target: "auth", "Failed to get access token from auth response {url}");
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

fn server_token_matches_device(server_device_key: &str, local_unique_id: &str) -> bool {
    server_device_key.is_empty() || local_unique_id == server_device_key
}

#[cfg(test)]
mod tests {
    use super::server_token_matches_device;

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
}
