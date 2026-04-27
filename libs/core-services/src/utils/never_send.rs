use serde::{Deserialize, Serialize};
use std::ops::{Deref, DerefMut};

/// Used this in case your data <T>
/// Is not implement Send, and you actually don't want to Send it
/// For example you are working on wasm, most of the type is not Send
/// Wrap them inside NeverSend will help you deal with Rust issue.
pub struct NeverSend<T>(pub T);

// Serialize impl (only if T: Serialize)
impl<T> Serialize for NeverSend<T>
where
    T: Serialize
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer
    {
        self.0.serialize(serializer)
    }
}

// Deserialize impl (only if T: Deserialize<'de>)
impl<'de, T> Deserialize<'de> for NeverSend<T>
where
    T: Deserialize<'de>
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>
    {
        T::deserialize(deserializer).map(NeverSend)
    }
}

impl<T> NeverSend<T> {
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> Clone for NeverSend<T>
where
    T: Clone
{
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

unsafe impl<T> Send for NeverSend<T> {}

unsafe impl<T> Sync for NeverSend<T> {}

impl<T> Deref for NeverSend<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for NeverSend<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
