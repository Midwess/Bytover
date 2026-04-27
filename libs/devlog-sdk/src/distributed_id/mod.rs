use chrono::{DateTime, Utc};
use generator::DistributedIdGenerator;
use n0_future::future::block_on;
use std::sync::Arc;
use tokio::sync::{Mutex, OnceCell};

#[cfg(feature = "distributed_id")]
pub mod etcd;
pub mod generator;

#[cfg(feature = "distributed_id")]
pub use etcd::EtcdWorkerOptions;

pub use generator::EPOCH_SINCE;

pub static DISTRIBUTED_ID_GENERATOR: OnceCell<Arc<Mutex<DistributedIdGenerator>>> = OnceCell::const_new();
#[cfg(feature = "distributed_id")]
static WORKER_LEASE_GUARD: OnceCell<etcd::WorkerLeaseGuard> = OnceCell::const_new();

pub async fn gen_id() -> u64 {
    let generator = DISTRIBUTED_ID_GENERATOR.get().unwrap();
    generator.lock().await.next_id().await.unwrap()
}

pub fn gen_id_sync() -> u64 {
    block_on(async { gen_id().await })
}

pub fn id_to_unix_timestamp(id: u64) -> u64 {
    let timestamp = id >> 23;
    let epoch_since_ms = EPOCH_SINCE;
    epoch_since_ms + timestamp
}

#[cfg(any(feature = "distributed_id"))]
pub async fn init_id_generator(
    app_name: String,
    options: EtcdWorkerOptions
) -> Result<(), generator::DistributedIdError> {
    if DISTRIBUTED_ID_GENERATOR.get().is_some() {
        return Ok(());
    }

    let (worker, guard) = etcd::register_worker(&app_name, options).await?;

    let generator = DistributedIdGenerator::from_worker(worker);
    let generator_arc = Arc::new(Mutex::new(generator));

    DISTRIBUTED_ID_GENERATOR.set(generator_arc.clone()).map_err(|_| {
        generator::DistributedIdError::Coordinator("distributed id generator already initialized".into())
    })?;

    // Spawn a task to sync worker_id from guard to generator
    let guard_clone = guard.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(5));
        loop {
            interval.tick().await;
            let current_worker_id = guard_clone.worker_id().await;
            let mut gen = generator_arc.lock().await;
            if gen.worker.distributed_id != current_worker_id as u64 {
                log::info!(
                    target: "distributed-id",
                    "updating generator worker_id from {} to {}",
                    gen.worker.distributed_id, current_worker_id
                );
                gen.worker.distributed_id = current_worker_id as u64;
            }
        }
    });

    WORKER_LEASE_GUARD
        .set(guard)
        .map_err(|_| generator::DistributedIdError::Coordinator("worker lease already registered".into()))?;

    Ok(())
}

pub fn init_scoped_id_generator(app_name: String) {
    let distributed_id_generator = DistributedIdGenerator::init_scoped(app_name.clone());
    let _ = DISTRIBUTED_ID_GENERATOR.set(Arc::new(Mutex::new(distributed_id_generator)));
}

pub fn init_scoped_id_generator_with_id(id: u32, name: String) {
    let distributed_id_generator = DistributedIdGenerator::init_scoped_with_id(id, name);
    let _ = DISTRIBUTED_ID_GENERATOR.set(Arc::new(Mutex::new(distributed_id_generator)));
}

pub fn id_to_datetime(id: u64) -> DateTime<Utc> {
    let timestamp = id >> 23;
    let epoch_since_ms = EPOCH_SINCE;
    DateTime::from_timestamp_millis(epoch_since_ms as i64 + timestamp as i64).unwrap()
}
