use futures_timer::Delay;
use futures_util::lock::Mutex;
use n0_future::future::block_on;
use n0_future::task::spawn;
use n0_future::time::Instant;
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, Weak};
use std::time::Duration;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum YieldError {
    #[error("The container is already empty")]
    AlreadyYielded,
    #[error("The container is empty")]
    NotYielded
}

#[derive(Clone)]
pub struct YieldContainer<T: Send + 'static> {
    pub value_container: Arc<Mutex<Option<Yieldable<T>>>>
}

impl<T: Send + 'static> YieldContainer<T> {
    pub fn new(value: T) -> Self {
        let container = Arc::new(Mutex::new(None));
        let yieldable = Yieldable {
            value: Some(value),
            container: Arc::downgrade(&container)
        };

        {
            let mut lock = block_on(container.lock());
            *lock = Some(yieldable);
        }

        Self {
            value_container: container
        }
    }

    pub fn empty() -> Self {
        Self {
            value_container: Arc::new(Mutex::new(None))
        }
    }

    pub async fn deposit(&self, value: T) -> Result<(), YieldError> {
        let mut lock = self.value_container.lock().await;
        if lock.is_some() {
            return Err(YieldError::AlreadyYielded);
        }
        let container = Arc::downgrade(&self.value_container);
        *lock = Some(Yieldable {
            value: Some(value),
            container
        });
        Ok(())
    }

    /// Retrieve ownership of Yieldable, removing it from container.
    pub async fn retrieve(&self) -> Result<Yieldable<T>, YieldError> {
        let mut lock = self.value_container.lock().await;
        match lock.take() {
            Some(yieldable) => Ok(yieldable),
            None => Err(YieldError::NotYielded)
        }
    }

    pub async fn retrieve_timed(&self, timeout: Duration) -> Result<Yieldable<T>, YieldError> {
        let start = Instant::now();
        loop {
            {
                let mut lock = self.value_container.lock().await;
                if let Some(yieldable) = lock.take() {
                    return Ok(yieldable);
                }
            }

            if start.elapsed() > timeout {
                return Err(YieldError::NotYielded);
            }

            Delay::new(Duration::from_millis(10)).await;
        }
    }
}

pub struct Yieldable<T: Send + 'static> {
    pub value: Option<T>,
    container: Weak<Mutex<Option<Yieldable<T>>>>
}

impl<T: Send + 'static> Deref for Yieldable<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.value.as_ref().expect("Already yield back")
    }
}

impl<T: Send + 'static> DerefMut for Yieldable<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.value.as_mut().expect("Already yield back")
    }
}

impl<T: Send + 'static> Drop for Yieldable<T> {
    fn drop(&mut self) {
        if let (Some(value), Some(container)) = (self.value.take(), self.container.upgrade()) {
            spawn(async move {
                let mut lock = container.lock().await;
                *lock = Some(Yieldable {
                    value: Some(value),
                    container: Arc::downgrade(&container)
                });
            });
        }
    }
}
