use crux_core::Command;

use super::operations::CoreOperation;
use super::{AppCommand, AppCommandContext, AppEvent};

pub trait CoreCommandUtils {
    fn empty() -> Self;
    fn render() -> Self;
    fn then_render(self) -> Self;
}

pub trait CoreCommandContextUtils {
    fn notify_event(&self, event: AppEvent);
}

impl CoreCommandContextUtils for AppCommandContext {
    fn notify_event(&self, event: AppEvent) {
        AppCommandContext::notify_shell(self, CoreOperation::Notified(event));
    }
}

impl CoreCommandUtils for AppCommand {
    fn empty() -> Self {
        Command::new(|_| async move {})
    }

    fn then_render(self) -> Self {
        self.then(Command::new(|it| async move {
            it.notify_shell(CoreOperation::Render);
        }))
    }

    fn render() -> Self {
        Command::empty().then_render()
    }
}
