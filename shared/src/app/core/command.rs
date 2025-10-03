use crate::app::AppCommandContext;
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
}
