use std::future::Future;

use serde::{Deserialize, Serialize};

use crate::app::core::command::AppCommand;
use crate::app::AppRequestBuilder;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum WebViewOperation {
    OpenUrl(String)
}

impl WebViewOperation {
    pub fn open_url(url: String) -> AppRequestBuilder<impl Future<Output = ()>> {
        AppCommand::request_from_shell(WebViewOperation::OpenUrl(url)).map(|it| it.empty())
    }
}
