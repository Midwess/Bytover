use std::future::Future;

use crux_core::capability::Operation;
use crux_core::Command;
use serde::{Deserialize, Serialize};

use crate::app::AppRequestBuilder;

use super::{CoreOperation, CoreOperationOutput};

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DialogOperationOutput {
    Toast,
    Alert { is_confirmed: bool },
    Message
}

impl Operation for DialogOperation {
    type Output = DialogOperationOutput;
}

impl DialogOperation {
    pub fn toast(message: impl Into<String>) -> AppRequestBuilder<impl Future<Output = ()>> {
        Command::request_from_shell(CoreOperation::Dialog(DialogOperation::Toast(message.into()))).map(|_it| {})
    }

    pub fn alert(dialog: AlertDialog) -> AppRequestBuilder<impl Future<Output = bool>> {
        Command::request_from_shell(CoreOperation::Dialog(DialogOperation::Alert(dialog))).map(|it| match it {
            CoreOperationOutput::Dialog(DialogOperationOutput::Alert { is_confirmed }) => is_confirmed,
            _ => panic!("Invalid output for DialogOperation::Alert")
        })
    }

    pub fn message(message: String, reason: MessageReason) -> AppRequestBuilder<impl Future<Output = ()>> {
        Command::request_from_shell(CoreOperation::Dialog(DialogOperation::Message(message, reason))).map(|_it| {})
    }
}
