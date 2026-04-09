#[cfg(target_family = "wasm")]
pub trait MaybeSendSync: Sync {}
#[cfg(not(target_family = "wasm"))]
pub trait MaybeSendSync: Sync + Send {}

#[cfg(target_family = "wasm")]
impl<T> MaybeSendSync for T where T: Sync {}

#[cfg(not(target_family = "wasm"))]
impl<T> MaybeSendSync for T where T: Sync + Send {}

#[cfg(not(target_family = "wasm"))]
pub trait MaybeSend: Send {}

#[cfg(not(target_family = "wasm"))]
impl<T> MaybeSend for T where T: Send {}

#[cfg(target_family = "wasm")]
pub trait MaybeSend {}

#[cfg(target_family = "wasm")]
impl<T> MaybeSend for T {}
