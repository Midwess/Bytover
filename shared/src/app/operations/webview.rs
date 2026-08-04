use std::future::Future;

use serde::{Deserialize, Serialize};

use crate::app::core::command::AppCommand;
use crate::app::operations::CoreOperationOutput;
use crate::app::AppRequestBuilder;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AuthenticationSessionOutcome {
    Completed { callback_url: String },
    Cancelled,
    Failed { code: String },
    ExternalBrowserStarted,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum WebViewOperation {
    Authenticate { url: String, callback_scheme: String },
}

impl WebViewOperation {
    pub fn authenticate(
        url: String,
        callback_scheme: String,
    ) -> AppRequestBuilder<impl Future<Output = AuthenticationSessionOutcome>> {
        AppCommand::request_from_shell(WebViewOperation::Authenticate { url, callback_scheme }).map(|response| match response {
            CoreOperationOutput::AuthenticationSession(outcome) => outcome,
            CoreOperationOutput::Error(_) => AuthenticationSessionOutcome::Failed {
                code: "session_failed".to_string(),
            },
            _ => AuthenticationSessionOutcome::Failed {
                code: "invalid_session_response".to_string(),
            },
        })
    }
}
