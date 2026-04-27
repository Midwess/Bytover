use futures::task::AtomicWaker;
use n0_future::task::{spawn, JoinHandle};
use parking_lot::Mutex;
use std::fmt::Debug;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Weak};

#[derive(Debug, Default)]
struct CancelState {
    cancelled: AtomicBool,
    waker: AtomicWaker,
    children: Mutex<Vec<Weak<CancelState>>>
}

#[derive(Serialize, Deserialize)]
pub struct CancellationToken {
    #[serde(skip)]
    state: Arc<CancelState>,
    #[serde(skip)]
    timeout_handle: Option<JoinHandle<()>>
}

impl Clone for CancellationToken {
    fn clone(&self) -> Self {
        Self {
            state: self.state.clone(),
            timeout_handle: None
        }
    }
}

impl Debug for CancellationToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CancellationToken").field("cancelled", &self.is_cancelled()).finish()
    }
}

impl PartialEq for CancellationToken {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

impl Eq for CancellationToken {}

impl Default for CancellationToken {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for CancellationToken {
    fn drop(&mut self) {
        if let Some(handle) = self.timeout_handle.take() {
            handle.abort()
        }
    }
}

impl CancellationToken {
    pub fn new() -> Self {
        Self {
            state: Arc::new(CancelState {
                cancelled: AtomicBool::new(false),
                waker: AtomicWaker::new(),
                children: Mutex::new(Vec::new())
            }),
            timeout_handle: None
        }
    }

    pub fn timeout(duration: Duration) -> Self {
        let mut token = CancellationToken::new();
        let child = token.clone();
        let handle = spawn(async move {
            Delay::new(duration).await;
            child.cancel();
        });

        token.timeout_handle = Some(handle);
        token
    }

    /// Returns true if this token or its parent was cancelled.
    pub fn is_cancelled(&self) -> bool {
        self.state.cancelled.load(Ordering::Acquire)
    }

    pub fn cancel_after(&self, duration: Duration) {
        let token = self.clone();
        spawn(async move {
            Delay::new(duration).await;
            token.cancel();
        });
    }

    /// Cancel this token and recursively cancel all child tokens.
    pub fn cancel(&self) {
        if !self.state.cancelled.swap(true, Ordering::AcqRel) {
            // Wake up any pending futures
            self.state.waker.wake();

            // Cancel all children
            // Use a block to limit the lock scope (best practice)
            let children = {
                let mut children_lock = self.state.children.lock();
                // Collect valid children and clean up weak references
                let mut valid_children = Vec::new();
                children_lock.retain(|weak_ref| {
                    if let Some(child) = weak_ref.upgrade() {
                        valid_children.push(child);
                        true
                    } else {
                        false
                    }
                });
                valid_children
            };

            // Cancel children without holding the lock
            for child in children {
                child.cancelled.store(true, Ordering::Release);
                child.waker.wake();
            }
        }
    }

    /// Creates a new child token that is linked to this token.
    pub fn child_token(&self) -> Self {
        let child = CancellationToken::new();

        // Add child to parent's list within a limited scope
        {
            let mut children = self.state.children.lock();
            children.push(Arc::downgrade(&child.state));
        }

        // If parent is already cancelled, cancel the child
        if self.is_cancelled() {
            child.cancel();
        }

        child
    }

    /// Returns a future that resolves when this token is cancelled.
    pub fn cancelled(&self) -> Cancelled {
        Cancelled {
            state: Arc::downgrade(&self.state)
        }
    }

    pub fn drop_guard(&self) -> DropGuard {
        DropGuard::new(self.clone())
    }
}

pub struct DropGuard {
    token: Option<CancellationToken>
}

impl DropGuard {
    pub fn new(token: CancellationToken) -> Self {
        Self { token: Some(token) }
    }

    pub fn disarm(&mut self) {
        self.token = None;
    }
}

impl Drop for DropGuard {
    fn drop(&mut self) {
        if let Some(token) = &self.token {
            token.cancel();
        }
    }
}

pub struct Cancelled {
    state: Weak<CancelState>
}

impl Future for Cancelled {
    type Output = ();

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
        if let Some(state) = self.state.upgrade() {
            if state.cancelled.load(Ordering::Acquire) {
                std::task::Poll::Ready(())
            } else {
                state.waker.register(cx.waker());
                if state.cancelled.load(Ordering::Acquire) {
                    std::task::Poll::Ready(())
                } else {
                    std::task::Poll::Pending
                }
            }
        } else {
            // Parent is dropped, consider it cancelled
            std::task::Poll::Ready(())
        }
    }
}

use futures_timer::Delay;
use futures_util::{select, FutureExt};
use n0_future::pin;
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::time::Duration;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum TaskErrors {
    #[error("Task cancelled")]
    Cancelled
}

pub trait FutureExtension: Future + Sized {
    fn with_cancel(self, cancellation: &CancellationToken) -> impl Future<Output = Result<Self::Output, TaskErrors>>;
}

impl<T: Future> FutureExtension for T {
    async fn with_cancel(self, cancellation: &CancellationToken) -> Result<Self::Output, TaskErrors> {
        if cancellation.is_cancelled() {
            return Err(TaskErrors::Cancelled);
        }

        let self_ = self.fuse();
        pin!(self_);
        let mut cancelled = cancellation.cancelled().fuse();
        select! {
            output = self_ => {
                Ok(output)
            },
            _ = cancelled => {
                Err(TaskErrors::Cancelled)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_cancellation_token_basic() {
        let token = CancellationToken::new();
        assert!(!token.is_cancelled());

        token.cancel();
        assert!(token.is_cancelled());
    }

    #[test]
    fn test_child_token() {
        let parent = CancellationToken::new();
        let child = parent.child_token();

        assert!(!parent.is_cancelled());
        assert!(!child.is_cancelled());

        parent.cancel();
        assert!(parent.is_cancelled());
        assert!(child.is_cancelled());
    }
}
