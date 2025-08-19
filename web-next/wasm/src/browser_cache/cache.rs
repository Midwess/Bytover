use core_services::utils::never_send::NeverSend;
use js_sys::Uint8Array;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Cache, Request, Response, ResponseInit};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum BrowserCacheErrors {
    #[error("Failed to open cache: {0}")]
    FailedToOpenCache(String),
    #[error("Failed to put: {0}")]
    FailedToPut(String),
    #[error("Failed to get: {0}")]
    FailedToGet(String),
    #[error("Failed to delete: {0}")]
    FailedToDelete(String),
    #[error("Failed to clear: {0}")]
    FailedToClear(String),
    #[error("Failed to close: {0}")]
    FailedToClose(String),
    #[error("Failed to get all: {0}")]
    FailedToGetAll(String),
}

#[derive(Clone)]
pub struct BrowserCache {
    cache: NeverSend<Cache>,
    pub(crate) name: String
}

impl BrowserCache {
    pub async fn open(name: &str) -> Self {
        let caches = web_sys::window()
            .unwrap()
            .caches()
            .expect("Failed to get caches");

        let cache_value = JsFuture::from(caches.open(name)).await.expect("Failed to open cache");

        let cache = Cache::from(cache_value);
        Self {
            cache: NeverSend(cache),
            name: name.to_string()
        }
    }

    pub async fn put(&self, key: &str, mut value: Vec<u8>) -> Result<(), BrowserCacheErrors> {
        let request = Request::new_with_str(key).map_err(|_| BrowserCacheErrors::FailedToOpenCache(key.to_string()))?;
    
        let mut init = ResponseInit::new();
        init.status(200);
    
        let response = Response::new_with_opt_u8_array_and_init(Some(&mut value[..]), &init).map_err(|_| BrowserCacheErrors::FailedToPut(key.to_string()))?;
    
        JsFuture::from(self.cache.put_with_request(&request, &response)).await.map_err(|_| BrowserCacheErrors::FailedToPut(key.to_string()))?;
    
        Ok(())
    }

    pub async fn get(&self, key: &str, take: bool) -> Result<Option<Uint8Array>, BrowserCacheErrors> {
        let request = Request::new_with_str(key).map_err(|_| BrowserCacheErrors::FailedToGet(key.to_string()))?;
        
        let response_value = JsFuture::from(self.cache.match_with_request(&request))
            .await
            .map_err(|_| BrowserCacheErrors::FailedToGet(key.to_string()))?;
        
        if response_value.is_undefined() {
            return Ok(None);
        }
        
        let response = Response::from(response_value);
        
        let array_buffer = JsFuture::from(response.array_buffer().map_err(|_| BrowserCacheErrors::FailedToGet(key.to_string()))?)
            .await
            .map_err(|_| BrowserCacheErrors::FailedToGet(key.to_string()))?;
        
        let uint8_array = Uint8Array::new(&array_buffer);

        if take {
            JsFuture::from(self.cache.delete_with_request(&request))
                .await
                .map_err(|_| BrowserCacheErrors::FailedToDelete(key.to_string()))?;
        }
        
        Ok(Some(uint8_array))
    }
}
