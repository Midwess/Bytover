// pub async fn spawn_blocking_compat<F, R>(f: F) -> Result<R, tokio::task::JoinError>
// where
//     F: FnOnce() -> R + Send + 'static,
//     R: Send + 'static,
// {
//     #[cfg(target_arch = "wasm32")]
//     {
//         Ok(f())
//     }
//
//     #[cfg(not(target_arch = "wasm32"))]
//     {
//         tokio::task::spawn_blocking(f).await
//     }
// }
