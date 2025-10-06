use std::future::Future;

use crux_core::capability::Operation;
use crux_core::Command;

use crate::app::operations::CoreOperationOutput;
use crate::app::NotifiedOperation;

use super::operations::CoreOperation;
use super::{AppCommand, AppCommandContext, AppEvent, AppRequestBuilder};

pub trait CoreCommandUtils {
    fn empty() -> Self;
    fn render() -> Self;
    fn then_render(self) -> Self;
    fn request_from_shell<O>(operation: O) -> AppRequestBuilder<impl Future<Output = CoreOperationOutput>>
    where
        O: Operation + Into<CoreOperation> + 'static;
    fn operate<O>(operation: O) -> AppCommand
    where
        O: Operation + Into<CoreOperation> + 'static;
}

pub trait CoreCommandContextUtils {
    fn notify_event(&self, event: impl Into<AppEvent>);
    fn app(&self) -> crate::app::core::command::AppCommand;
}

impl CoreCommandContextUtils for AppCommandContext {
    fn notify_event(&self, event: impl Into<AppEvent>) {
        self.notify_shell(NotifiedOperation::AppEvent(event.into()))
    }

    fn app(&self) -> crate::app::core::command::AppCommand {
        crate::app::core::command::AppCommand::new(self.clone())
    }
}

impl CoreCommandUtils for AppCommand {
    fn empty() -> Self {
        Command::new(|_| async move {})
    }

    fn then_render(self) -> Self {
        self.then(Command::new(|it| async move {
            it.notify_shell(CoreOperation::Render)
        }))
    }

    fn render() -> Self {
        Command::empty().then_render()
    }

    fn operate<O>(operation: O) -> AppCommand
    where
        O: Operation + Into<CoreOperation> + 'static,
    {
        AppCommand::new(move |it| async move {
            it.notify_shell(operation.into())
        })
    }

    fn request_from_shell<O>(
        operation: O,
    ) -> AppRequestBuilder<impl Future<Output = CoreOperationOutput>>
    where
        O: Operation + Into<CoreOperation> + 'static,
    {
        Command::request_from_shell(operation.into())
    }
}
