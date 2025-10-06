use crate::CoreOperation;
use crate::app::{AppCommand, AppCommandContext, AppEvent, AppRequestBuilder};
use crate::app::operations::CoreOperationOutput;
use crate::CoreOperation::Notified;
use crux_core::capability::Operation;
use crux_core::Command;
use std::future::Future;

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
    /// This one will used to update result of command
    /// back to the core, without leaving the core.
    /// so that it will run super fast because it skip the serialize + deserialize,
    /// but it also means it cannot return any effects.
    fn update_model(&self, result: impl Into<AppEvent>);
    fn update_model_series(&self, results: Vec<impl Into<AppEvent>>);
    fn notify_event(&self, event: impl Into<AppEvent>);
    fn app(self) -> crate::app::core::command::AppCommand;
}

impl CoreCommandContextUtils for AppCommandContext {
    fn update_model(&self, result: impl Into<AppEvent>) {
        self.send_event(result.into());
        self.notify_shell(CoreOperation::Render);
    }

    fn update_model_series(&self, results: Vec<impl Into<AppEvent>>) {
        for result in results {
            self.send_event(result.into());
        }

        self.notify_shell(CoreOperation::Render);
    }

    fn notify_event(&self, event: impl Into<AppEvent>) {
        let event: AppEvent = event.into();
        self.notify_shell(Notified(event))
    }

    fn app(self) -> crate::app::core::command::AppCommand {
        crate::app::core::command::AppCommand::new(self)
    }
}

impl CoreCommandUtils for AppCommand {
    fn empty() -> Self {
        Command::new(|_| async move {})
    }

    fn render() -> Self {
        Command::empty().then_render()
    }

    fn then_render(self) -> Self {
        self.then(Command::new(|it| async move { it.notify_shell(CoreOperation::Render) }))
    }

    fn request_from_shell<O>(operation: O) -> AppRequestBuilder<impl Future<Output = CoreOperationOutput>>
    where
        O: Operation + Into<CoreOperation> + 'static
    {
        Command::request_from_shell(operation.into())
    }

    fn operate<O>(operation: O) -> AppCommand
    where
        O: Operation + Into<CoreOperation> + 'static
    {
        AppCommand::new(move |it| async move { it.notify_shell(operation.into()) })
    }
}
