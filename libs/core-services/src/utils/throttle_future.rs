use futures_util::lock::Mutex;
use n0_future::task::{spawn, JoinHandle};
use n0_future::time;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

pub type ThrottleFn<T> = Box<dyn (FnMut(T) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>>) + Send + Sync>;

pub struct ThrottleFuture<T>
where
    T: Send + Sync + 'static
{
    last_value: Arc<Mutex<Option<T>>>,
    join_handle: Option<JoinHandle<()>>
}

impl<T> ThrottleFuture<T>
where
    T: Send + Sync + 'static
{
    pub fn new(mut future_fn: ThrottleFn<T>, delay: Duration) -> Self {
        let last_value = Arc::new(Mutex::new(None::<T>));

        let value_clone = last_value.clone();

        let join_handle = spawn(async move {
            let mut interval = time::interval(delay);
            interval.tick().await;

            loop {
                interval.tick().await;

                let value = {
                    let mut value_guard = value_clone.lock().await;
                    value_guard.take()
                };

                if let Some(value) = value {
                    future_fn(value).await;
                } else {
                    break;
                }
            }
        });

        Self {
            last_value,
            join_handle: Some(join_handle)
        }
    }

    pub async fn call(&self, value: T) {
        if self.join_handle.is_none() {
            return;
        }

        let mut value_guard = self.last_value.lock().await;
        *value_guard = Some(value);
    }

    pub async fn stop(mut self) {
        let mut value_guard = self.last_value.lock().await;
        *value_guard = None;
        if let Some(join_handle) = self.join_handle.take() {
            let _ = join_handle.await;
        }
    }
}

impl<T> Drop for ThrottleFuture<T>
where
    T: Send + Sync + 'static
{
    fn drop(&mut self) {
        if let Some(join_handle) = self.join_handle.take() {
            join_handle.abort();
        }
    }
}
