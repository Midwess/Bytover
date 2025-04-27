use std::future::Future;

use crux_core::capability::Operation;
use crux_core::Command;
use serde::{Deserialize, Serialize};
use uniffi::{Enum, Record};

use crate::app::AppRequestBuilder;

use super::{CoreOperation, CoreOperationOutput};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Record)]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Enum)]
pub enum DialogOperation {
    Toast(String),
    Alert(AlertDialog)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Enum)]
pub enum DialogOperationOutput {
    Toast,
    Alert { is_confirmed: bool }
}

impl Operation for DialogOperation {
    type Output = DialogOperationOutput;
}

impl DialogOperation {
    pub fn toast(message: String) -> AppRequestBuilder<impl Future<Output = ()>> {
        Command::request_from_shell(CoreOperation::Dialog(DialogOperation::Toast(message))).map(|it| match it {
            CoreOperationOutput::Dialog(DialogOperationOutput::Toast) => {}
            _ => panic!("Invalid output for DialogOperation::Toast")
        })
    }

    pub fn alert(dialog: AlertDialog) -> AppRequestBuilder<impl Future<Output = bool>> {
        Command::request_from_shell(CoreOperation::Dialog(DialogOperation::Alert(dialog))).map(|it| match it {
            CoreOperationOutput::Dialog(DialogOperationOutput::Alert { is_confirmed }) => is_confirmed,
            _ => panic!("Invalid output for DialogOperation::Alert")
        })
    }
}
