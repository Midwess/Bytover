use crate::app::authentication::module::AuthenticationEvent;
use crate::app::core::command::AppCommand;
use crate::app::core::extensions::CoreCommandContextUtils;
use crate::app::operations::device::DeviceOperation;
use crate::app::operations::dialog::DialogOperation;
use crate::app::operations::persistent::{
    DeviceAliasPersistentOperation,
    SessionPersistentOperation,
    ShelfPersistentOperation,
    TransferSessionPersistentOperation
};
use crate::app::operations::rpc::RpcOperation;
use crate::app::operations::webview::WebViewOperation;
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
        self.run(SessionPersistentOperation::remove_session()).await?;
        self.run(TransferSessionPersistentOperation::clear_all()).await?;
        self.run(ShelfPersistentOperation::clear_all()).await?;
        self.run(DeviceAliasPersistentOperation::clear_all()).await?;
        self.notify_event(TransferEvent::Clear);
        self.notify_event(ShelfEvent::Launch);
        self.re_authorize().await?;
        Ok(())
    }

    pub async fn re_authorize(&self) -> Result<(), CoreError> {
        let Ok(user) = RpcOperation::get_me().into_future(self.ctx()).await else {
            self.notify_event(AuthenticationEvent::UnAuthorized);
            return Ok(())
        };

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
            value: token
        };

        if token.value.is_empty() {
            log::error!(target: "auth", "Failed to get access token from auth response {url}");
            return Ok(());
        }

        SessionPersistentOperation::save_token(token).into_future(self.ctx()).await?;

        // Clear all data on fresh sign in (user signs in after signing out)
        let session = SessionPersistentOperation::get_session().into_future(self.ctx()).await?;
        if session.is_none() {
            self.run(TransferSessionPersistentOperation::clear_all()).await?;
            self.run(ShelfPersistentOperation::clear_all()).await?;
            self.run(DeviceAliasPersistentOperation::clear_all()).await?;
            self.notify_event(TransferEvent::Clear);
            self.notify_event(ShelfEvent::Launch);
        }

        let user = RpcOperation::get_me().into_future(self.ctx()).await?;
        self.notify_event(AppEvent::Authentication(AuthenticationEvent::Authorized { user }));
        self.notify_shell(CoreOperation::LaunchNearbyServer);

        self.fetch_and_assign_aliases().await;

        Ok(())
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
                    Err(e) => log::error!(target: "auth", "Failed to update shelf alias: {e:?}")
                }
            }
        }
    }
}
