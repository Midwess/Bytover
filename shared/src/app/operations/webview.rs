use std::future::Future;

use crux_core::capability::Operation;
use crux_core::Command;
use serde::{Deserialize, Serialize};

use crate::app::AppRequestBuilder;

use super::{CoreOperation, CoreOperationOutput};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum WebViewOperation {
    OpenUrl(String)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum WebViewOperationOutput {
    OpenUrl
}

impl Operation for WebViewOperationOutput {
    type Output = WebViewOperationOutput;
}

impl WebViewOperation {
    pub fn open_url(url: String) -> AppRequestBuilder<impl Future<Output = ()>> {
        Command::request_from_shell(CoreOperation::WebView(WebViewOperation::OpenUrl(url))).map(|res| match res {
            CoreOperationOutput::WebView(WebViewOperationOutput::OpenUrl) => {}
            _ => panic!("Invalid output for WebViewOperation::OpenUrl")
        })
    }
}
