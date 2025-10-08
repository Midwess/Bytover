use std::future::Future;

use serde::{Deserialize, Serialize};

use super::CoreOperationOutput;
use crate::app::core::command::AppCommand;
use crate::app::AppRequestBuilder;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AlertDialog {
    pub message: String,
    pub affirmative: String,
    pub negative: Option<String>
}

impl AlertDialog {
    pub fn confirmation(message: String, affirmative: String, negative: Option<String>) -> Self {
        Self {
            message,
            affirmative,
            negative
        }
    }

    pub fn alert(message: String) -> Self {
        Self {
            message,
            affirmative: "OK".to_string(),
            negative: None
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MessageReason {
    FailedToFindPublicSession,
    PublicSessionUnauthenticated
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DialogOperation {
    Toast(String),
    Alert(AlertDialog),
    Message(String, MessageReason)
}

impl DialogOperation {
    pub fn toast(message: impl Into<String>) -> AppRequestBuilder<impl Future<Output = ()>> {
        AppCommand::request_from_shell(DialogOperation::Toast(message.into())).map(|_it| ())
    }

    pub fn alert(dialog: AlertDialog) -> AppRequestBuilder<impl Future<Output = bool>> {
        AppCommand::request_from_shell(DialogOperation::Alert(dialog)).map(|it| match it {
            CoreOperationOutput::Bool(is_confirmed) => is_confirmed,
            _ => false
        })
    }

    pub fn message(message: String, reason: MessageReason) -> AppRequestBuilder<impl Future<Output = ()>> {
        AppCommand::request_from_shell(DialogOperation::Message(message, reason)).map(|_it| ())
    }
}
