use crux_core::Command;
use futures::Stream;

use crate::app::operations::CoreOperationOutput;
use crate::app::{AppCommandContext, AppRequestBuilder};
use crate::CoreOperation;
use std::future::Future;
use std::ops::{Deref, DerefMut};

pub struct AppCommand {
    ctx: AppCommandContext
}

impl AppCommand {
    pub fn new(ctx: AppCommandContext) -> Self {
        Self { ctx }
    }
}

impl Deref for AppCommand {
    type Target = AppCommandContext;

    fn deref(&self) -> &AppCommandContext {
        &self.ctx
    }
}

impl DerefMut for AppCommand {
    fn deref_mut(&mut self) -> &mut AppCommandContext {
        &mut self.ctx
    }
}

impl AppCommand {
    pub fn ctx(&self) -> AppCommandContext {
        self.ctx.clone()
    }

    pub async fn run<T, F>(&self, request: AppRequestBuilder<F>) -> T
    where
        F: Future<Output = T> + Send + 'static,
        T: Send + 'static
    {
        request.into_future(self.ctx()).await
    }

    pub async fn request<O: Into<CoreOperation>>(&self, operation: O) -> CoreOperationOutput {
        Command::request_from_shell(operation.into()).into_future(self.ctx()).await
    }

    pub fn request_from_shell<O>(&self, operation: O) -> AppRequestBuilder<impl Future<Output = CoreOperationOutput>>
    where
        O: Into<CoreOperation>
    {
        Command::request_from_shell(operation.into())
    }

    pub fn stream_from_shell<O>(&self, operation: O) -> impl Stream<Item = CoreOperationOutput>
    where
        O: Into<CoreOperation>
    {
        self.ctx.stream_from_shell(operation.into())
    }
}
