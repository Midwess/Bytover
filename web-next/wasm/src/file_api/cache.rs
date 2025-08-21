use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use bytes::Bytes;
use futures::lock::Mutex;
use idb::{Database, KeyRange, Query, TransactionMode};
use js_sys::{Array, Uint8Array};
use serde::{Deserialize, Serialize};
use bincode;
use core_services::utils::never_send::NeverSend;
use thiserror::Error;
use wasm_bindgen::{JsCast, JsValue};
use core_services::utils::pool::request::PoolRequest;
use shared::core_api::{IOReader, IOWriter};
use anyhow::{anyhow, Result};
use crate::file_api::file_extension::VecExtension;

#[derive(Debug, Error)]
pub enum BrowserCacheErrors {
    #[error("Cache data is incomplete - missing end marker")]
    IncompleteData,
    #[error("Failed to put: {0}")]
    FailedToPut(String),
    #[error("Failed to get: {0}")]
    FailedToGet(String),
    #[error("IndexDb storage error: {0}")]
    IndexDbStorageError(String)
}

impl From<idb::Error> for BrowserCacheErrors {
    fn from(e: idb::Error) -> Self {
        Self::IndexDbStorageError(e.to_string())
    }
}

#[derive(Clone, Default)]
pub struct MemBuffer {
    // The current buffer
    pub(crate) buffer: Vec<u8>,
    // The chunk index of entire file
    pub(crate) chunk_index: usize,
    pub(crate) max_chunk_size: usize,
}

impl MemBuffer {
    pub fn new(chunk_index: usize) -> Self {
        Self {
            buffer: Vec::with_capacity(BrowserCache::MAX_CHUNK_SIZE),
            chunk_index,
            max_chunk_size: BrowserCache::MAX_CHUNK_SIZE,
        }
    }

    pub fn extend(&mut self, bytes: &Vec<u8>) -> Option<Vec<u8>> {
        self.buffer.extend_from_slice(bytes);
        if self.buffer.len() >= self.max_chunk_size {
            let chunk = self.buffer.drain(..self.max_chunk_size).collect();
            self.chunk_index += 1;
            Some(chunk)
        }
        else {
            None
        }
    }
}

#[derive(Clone)]
pub struct BrowserCache {
    db: PoolRequest<NeverSend<Database>>,
    pub(crate) mem_buffer: Arc<Mutex<MemBuffer>>,
    pub resource: CacheResource,
    pub current_size: Arc<AtomicUsize>
}

impl PartialEq for BrowserCache {
    fn eq(&self, other: &Self) -> bool {
        self.resource.id == other.resource.id
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct CacheResource {
    pub table: String,
    pub id: u64,
    pub total_size: usize,
}

impl CacheResource {
    pub fn thumbnail(resource_id: u64, size: usize) -> Self {
        Self {
            table: "thumbnails".to_string(),
            id: resource_id,
            total_size: size,
        }
    }

    pub fn resource(resource_id: u64) -> Self {
        Self {
            table: "resources".to_string(),
            id: resource_id,
            total_size: 0
        }
    }
}

impl BrowserCache {
    const MAX_CHUNK_SIZE: usize = 1024 * 1024 * 8;
    const END_MARKER_CHUNK: usize = usize::MAX - 1;

    pub async fn open(db: PoolRequest<NeverSend<Database>>, table: &str, id: u64) -> Result<Self, BrowserCacheErrors> {
        let db_handle = db.retrieve().await.unwrap();
        let trans = db_handle.transaction(&[&table], TransactionMode::ReadOnly)?;
        let store = trans.object_store(table)?;

        let end_marker_key = Self::create_chunk_id(id, Self::END_MARKER_CHUNK);
        let end_marker_query = Query::KeyRange(KeyRange::only(&end_marker_key).unwrap());
        
        let end_marker_result = store.get(end_marker_query)?.await?;

        let resource = match end_marker_result {
            Some(end_marker_data) => {
                CacheResource::try_from(end_marker_data)?
            },
            None => {
                return Err(BrowserCacheErrors::IncompleteData);
            }
        };
        
        Ok(Self {
            current_size: Arc::new(AtomicUsize::new(resource.total_size)),
            db,
            mem_buffer: Arc::new(Mutex::new(MemBuffer::new(0))),
            resource,
        })
    }

    pub async fn create(db: PoolRequest<NeverSend<Database>>, resource: CacheResource) -> Result<Self, BrowserCacheErrors> {
        let db_handle = db.retrieve().await.unwrap();
        let trans = db_handle.transaction(&[&resource.table], TransactionMode::ReadWrite)?;
        let store = trans.object_store(&resource.table)?;

        let all_query = Query::KeyRange(Self::create_all_range_query(resource.id));
        let existing_keys = store.get_all_keys(Some(all_query), None)?
            .await?;

        for key in existing_keys {
            store.delete(Query::Key(key))?
                .await?;
        }
        
        trans.commit()?.await?;

        Ok(Self {
            db,
            mem_buffer: Arc::new(Mutex::new(MemBuffer::new(0))),
            resource,
            current_size: Arc::new(AtomicUsize::new(0)),
        })
    }

    pub fn get_reader(&self) -> impl IOReader {
        IOReaderBrowserCacheImpl::new(self.clone())
    }

    pub async fn read_chunk(&self, chunk_index: usize, offset: usize) -> Result<Option<Vec<u8>>, BrowserCacheErrors> {
        let db = self.db.retrieve().await.unwrap();
        let trans = db.transaction(&[&self.resource.table], TransactionMode::ReadOnly)?;
        let store = trans.object_store(&self.resource.table)?;
        let query = Query::KeyRange(self.chunk_index_query(chunk_index));
        let result = match store.get(query)?.await? {
            Some(val) => {
                Self::merge_bytes(vec![val])
            },
            None => {
                let mem_buffer = self.mem_buffer.lock().await;
                if mem_buffer.chunk_index == chunk_index {
                    if mem_buffer.buffer.len() >= offset {
                        mem_buffer.buffer[offset..].to_vec()
                    }
                    else {
                        return Ok(None)
                    }
                }
                else {
                    return Ok(None);
                }
            }
        };

        if offset >= result.len() {
            return Ok(None);
        }

        Ok(Some(result[offset..].to_vec()))
    }

    async fn write_chunk(&self, chunk_index: usize, bytes: &Vec<u8>) -> Result<(), BrowserCacheErrors> {
        let len = bytes.len();
        if len > Self::MAX_CHUNK_SIZE {
            // return Err(BrowserCacheErrors::FailedToPut(format!("Chunk size exceeded: {} > {}", len, Self::MAX_CHUNK_SIZE)));
        }

        let current_size = self.current_size.load(Ordering::SeqCst);
        if current_size + len > self.resource.total_size {
            // return Err(BrowserCacheErrors::FailedToPut(format!("Cache size exceeded: {} > {}", current_size + len, self.resource.total_size)));
        }

        let db = self.db.retrieve().await.unwrap();
        let trans = db.transaction(&[&self.resource.table], TransactionMode::ReadWrite)
            .map_err(|it| BrowserCacheErrors::FailedToPut(format!("Failed to write chunk: {it:?}")))?;
        let store = trans.object_store(&self.resource.table)
            .map_err(|it| BrowserCacheErrors::FailedToPut(format!("Failed to write chunk: {it:?}")))?;
        let key: JsValue = self.chunk_id(chunk_index);
        let js_value: JsValue = bytes.into_js_value();

        store.put(&js_value, Some(&key))?.await?;
        trans.commit()?.await?;

        self.current_size.fetch_add(len, Ordering::SeqCst);

        Ok(())
    }
    
    pub async fn get_resource_info(&self) -> CacheResource {
        self.resource.clone()
    }

    fn chunk_id(&self, chunk_index: usize) -> JsValue {
        Self::create_chunk_id(self.resource.id, chunk_index)
    }
    
    fn create_chunk_id(resource_id: u64, chunk_index: usize) -> JsValue {
        let arr = Array::new();
        arr.push(&JsValue::from(resource_id.to_string()));
        arr.push(&JsValue::from(chunk_index));
        arr.into()
    }

    fn chunk_index_query(&self, chunk_index: usize) -> KeyRange {
        KeyRange::only(&self.chunk_id(chunk_index)).unwrap()
    }

    fn create_all_range_query(resource_id: u64) -> KeyRange {
        let arr = Array::new();
        arr.push(&JsValue::from(resource_id.to_string()));
        arr.push(&JsValue::from(0));
        let lower_bound: JsValue = arr.into();

        let arr = Array::new();
        arr.push(&JsValue::from(resource_id.to_string()));
        arr.push(&JsValue::from(Self::END_MARKER_CHUNK));
        let upper_bound: JsValue = arr.into();

        KeyRange::bound(&lower_bound, &upper_bound, Some(true), Some(true)).unwrap()
    }
    
    fn create_end_marker_data(&self) -> Result<Vec<u8>, BrowserCacheErrors> {
        bincode::serialize(&self.resource)
            .map_err(|e| BrowserCacheErrors::FailedToPut(format!("Failed to serialize ResourceInfo: {}", e)))
    }
    
    fn merge_bytes(values: Vec<JsValue>) -> Vec<u8> {
        let mut merged: Vec<u8> = Vec::new();

        for val in values {
            let u8arr = if val.is_instance_of::<Uint8Array>() {
                val.unchecked_into::<Uint8Array>()
            } else {
                Uint8Array::new(&val)
            };

            let mut buf = vec![0; u8arr.length() as usize];
            u8arr.copy_to(&mut buf[..]);
            merged.extend_from_slice(&buf);
        }

        merged
    }
}

pub struct IOReaderBrowserCacheImpl {
    cache: BrowserCache,
    current_chunk_index: usize,
    current_offset: usize,
}

impl Clone for IOReaderBrowserCacheImpl {
    fn clone(&self) -> Self {
        Self {
            cache: self.cache.clone(),
            current_chunk_index: 0,
            current_offset: 0,
        }
    }
}

impl IOReaderBrowserCacheImpl {
    pub fn new(cache: BrowserCache) -> Self {
        Self {
            cache,
            current_chunk_index: 0,
            current_offset: 0,
        }
    }
}

#[async_trait::async_trait(?Send)]
impl IOReader for IOReaderBrowserCacheImpl {
    async fn next(&mut self) -> Result<Option<Bytes>> {
        let Some(bytes) = self.cache.read_chunk(self.current_chunk_index, self.current_offset).await
            .map_err(|e| anyhow::anyhow!("Cache error: {:?}", e))? else {
            return Ok(None);
        };

        let max_chunk_size = BrowserCache::MAX_CHUNK_SIZE;
        if self.current_offset + bytes.len() >= max_chunk_size {
            self.current_chunk_index += 1;
            self.current_offset = 0;
        } else {
            self.current_offset += bytes.len();
        }

        Ok(Some(Bytes::from(bytes)))
    }

    async fn total_size(&self) -> Result<u64> {
        Ok(self.cache.resource.total_size as u64)
    }
}

#[async_trait::async_trait(?Send)]
impl IOWriter for BrowserCache {
    async fn write(&mut self, data: Bytes) -> anyhow::Result<()> {
        let mut mem_buffer = self.mem_buffer.lock().await;
        if let Some(flushed_bytes) = mem_buffer.extend(&data.to_vec()) {
            let chunk_index = mem_buffer.chunk_index;
            drop(mem_buffer); // Release lock before async operation
            self.write_chunk(chunk_index, &flushed_bytes).await
                .map_err(|e| anyhow::anyhow!("Failed to write chunk: {:?}", e))?;
        }

        Ok(())
    }

    async fn flush(&mut self) -> anyhow::Result<()> {
        let mem_buffer = self.mem_buffer.lock().await;
        if mem_buffer.buffer.len() > 0 {
            let chunk_index = mem_buffer.chunk_index;
            let buffer_copy = mem_buffer.buffer.clone();
            drop(mem_buffer);
            
            let max_chunk_size = BrowserCache::MAX_CHUNK_SIZE;
            let mut current_chunk_index = chunk_index;
            
            for chunk_data in buffer_copy.chunks(max_chunk_size) {
                self.write_chunk(current_chunk_index, &chunk_data.to_vec()).await
                    .map_err(|e| anyhow::anyhow!("Failed to write chunk: {:?}", e))?;
                current_chunk_index += 1;
            }
            
            let mut mem_buffer = self.mem_buffer.lock().await;
            mem_buffer.chunk_index = current_chunk_index;
            mem_buffer.buffer.clear();
        }
        
        Ok(())
    }

    async fn end(&mut self) -> Result<()> {
        self.flush().await?;
        let db = self.db.retrieve().await.unwrap();
        let trans = db.transaction(&[&self.resource.table], TransactionMode::ReadWrite)
            .map_err(|it| anyhow!("Failed to get transaction {it:?}"))?;
        let store = trans.object_store(&self.resource.table)
            .map_err(|it| anyhow!("Failed to get store {it:?}"))?;
        let key: JsValue = self.chunk_id(Self::END_MARKER_CHUNK);
        let current_size = self.current_size.load(Ordering::SeqCst);
        if current_size != self.resource.total_size {
            // return Err(anyhow!("Cache size mismatch: {} != {}", current_size, self.resource.total_size));
        }

        let end_marker_data = self.create_end_marker_data()?;
        let js_value = (&end_marker_data).into_js_value();
        store.put(&js_value, Some(&key)).map_err(|it| anyhow!("error while ending write {it:?}"))?.await
            .map_err(|it| anyhow!("Failed to put while ending write {it:?}"))?;
        let _ = trans.commit().map_err(|it| anyhow!("Failed to commit while ending write {it:?}"))?.await
            .map_err(|it| anyhow!("Failed to commit while ending write {it:?}"))?;

        Ok(())
    }
}

impl TryFrom<JsValue> for CacheResource {
    type Error = BrowserCacheErrors;

    fn try_from(js_value: JsValue) -> Result<Self, Self::Error> {
        let u8arr = if js_value.is_instance_of::<Uint8Array>() {
            js_value.unchecked_into::<Uint8Array>()
        } else {
            Uint8Array::new(&js_value)
        };
        
        let mut bytes = vec![0u8; u8arr.length() as usize];
        u8arr.copy_to(&mut bytes);
        
        bincode::deserialize(&bytes)
            .map_err(|e| BrowserCacheErrors::FailedToGet(format!("Failed to deserialize CacheResource: {}", e)))
    }
}