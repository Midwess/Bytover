use crate::distributed_id::generator::{DistributedIdError, Worker, MAX_WORKER_ID};
use chrono::Utc;
use etcd_client::{Client, Compare, CompareOp, PutOptions, Txn, TxnOp};
use serde_json::json;
use std::fmt;
use std::sync::atomic::{AtomicI64, AtomicU16, Ordering};
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tokio::sync::Mutex as TokioMutex;
use tokio::task::JoinHandle;

/// Configuration for reserving Snowflake worker IDs via etcd.
///
/// Environment variables used by [`EtcdWorkerOptions::from_env`]:
/// - `SNOWFLAKE_ETCD_ENDPOINTS`: comma-separated list of etcd endpoints
/// - `SNOWFLAKE_ETCD_NAMESPACE`: optional namespace prefix (default: `snowflake`)
/// - `SNOWFLAKE_ETCD_LEASE_TTL_SECS`: optional lease TTL override in seconds (default: 30)
#[derive(Clone, Debug)]
pub struct EtcdWorkerOptions {
    pub endpoints: Vec<String>,
    pub namespace: String,
    pub lease_ttl: Duration
}

impl Default for EtcdWorkerOptions {
    fn default() -> Self {
        Self {
            endpoints: vec!["http://localhost:2379".to_string()],
            namespace: "snowflake".to_string(),
            lease_ttl: Duration::from_secs(30)
        }
    }
}

impl EtcdWorkerOptions {
    pub fn new(endpoints: Vec<String>) -> Self {
        let mut options = Self {
            endpoints,
            ..Default::default()
        };
        options.normalize();
        options
    }

    pub fn namespace(mut self, namespace: impl Into<String>) -> Self {
        self.namespace = namespace.into();
        self.normalize();
        self
    }

    pub fn lease_ttl(mut self, ttl: Duration) -> Self {
        self.lease_ttl = ttl;
        self
    }

    pub fn from_env() -> Result<Self, DistributedIdError> {
        let endpoints = std::env::var("SNOWFLAKE_ETCD_ENDPOINTS")
            .map_err(|_| DistributedIdError::InvalidConfig("SNOWFLAKE_ETCD_ENDPOINTS must be set".into()))?;
        let endpoints = endpoints
            .split(',')
            .map(|endpoint| endpoint.trim())
            .filter(|endpoint| !endpoint.is_empty())
            .map(|endpoint| endpoint.to_string())
            .collect::<Vec<_>>();

        if endpoints.is_empty() {
            return Err(DistributedIdError::InvalidConfig(
                "SNOWFLAKE_ETCD_ENDPOINTS must include at least one http(s) endpoint".into()
            ));
        }

        let mut options = Self::new(endpoints);

        if let Ok(namespace) = std::env::var("SNOWFLAKE_ETCD_NAMESPACE") {
            if !namespace.trim().is_empty() {
                options.namespace = namespace;
            }
        }

        if let Ok(ttl) = std::env::var("SNOWFLAKE_ETCD_LEASE_TTL_SECS") {
            let ttl_secs = ttl.parse::<u64>().map_err(|_| {
                DistributedIdError::InvalidConfig("SNOWFLAKE_ETCD_LEASE_TTL_SECS must be a positive integer".into())
            })?;
            options.lease_ttl = Duration::from_secs(ttl_secs.max(1));
        }

        options.normalize();
        options.validate()?;

        Ok(options)
    }

    pub fn keep_alive_interval(&self) -> Duration {
        let secs = (self.lease_ttl.as_secs() / 3).max(1);
        Duration::from_secs(secs)
    }

    pub fn validate(&self) -> Result<(), DistributedIdError> {
        if self.endpoints.is_empty() {
            return Err(DistributedIdError::InvalidConfig("no etcd endpoints configured".into()));
        }

        if self.lease_ttl.is_zero() {
            return Err(DistributedIdError::InvalidConfig("lease TTL must be greater than zero".into()));
        }

        // Validate TTL fits in i64 for etcd
        if self.lease_ttl.as_secs() > i64::MAX as u64 {
            return Err(DistributedIdError::InvalidConfig(format!(
                "lease TTL too large, must be <= {} seconds",
                i64::MAX
            )));
        }

        Ok(())
    }

    fn normalize(&mut self) {
        let endpoints = self
            .endpoints
            .iter()
            .map(|endpoint| endpoint.trim().to_string())
            .filter(|endpoint| !endpoint.is_empty())
            .collect::<Vec<_>>();
        self.endpoints = endpoints;

        let namespace = self.namespace.trim_matches('/').to_string();
        self.namespace = if namespace.is_empty() {
            "snowflake".to_string()
        } else {
            namespace
        };
    }

    fn namespace_prefix(&self) -> String {
        self.namespace.clone()
    }

    fn namespace_with_app(&self, app_name: &str) -> String {
        format!("{}/{}", self.namespace_prefix(), Self::app_segment(app_name))
    }

    fn workers_prefix(&self, app_name: &str) -> String {
        format!("{}/workers", self.namespace_with_app(app_name))
    }

    fn app_segment(app_name: &str) -> String {
        app_name.trim_matches('/').replace('/', "_")
    }
}

#[derive(Clone)]
pub struct WorkerLeaseGuard {
    client: Client,
    lease_id: Arc<AtomicI64>,
    key: Arc<RwLock<String>>,
    worker_id: Arc<AtomicU16>,
    keep_alive_task: Arc<TokioMutex<Option<JoinHandle<()>>>>,
    app_name: String
}

const MAX_RECONNECT_DELAY: Duration = Duration::from_secs(60);
const INITIAL_RECONNECT_DELAY: Duration = Duration::from_secs(1);

impl WorkerLeaseGuard {
    pub async fn new(
        client: Client,
        lease_id: i64,
        key: String,
        worker_id: u16,
        keep_alive_interval: Duration,
        endpoints: Vec<String>,
        app_name: String,
        options: EtcdWorkerOptions
    ) -> Result<Self, DistributedIdError> {
        let mut keep_alive_client = client.clone();
        // Keep the keeper so we can call its `keep_alive` method periodically.
        let (mut keeper, mut stream) = keep_alive_client
            .lease_keep_alive(lease_id)
            .await
            .map_err(|err| DistributedIdError::Coordinator(format!("failed to start lease keep-alive: {err}")))?;

        let interval_duration = if keep_alive_interval.is_zero() {
            Duration::from_secs(1)
        } else {
            keep_alive_interval
        };

        let endpoints_clone = endpoints.clone();
        let app_name_clone = app_name.clone();
        let options_clone = options.clone();
        let lease_id_shared = Arc::new(AtomicI64::new(lease_id));
        let key_shared = Arc::new(RwLock::new(key.clone()));
        let worker_id_shared = Arc::new(AtomicU16::new(worker_id));
        let lease_id_task = Arc::clone(&lease_id_shared);
        let key_task = Arc::clone(&key_shared);
        let worker_id_task = Arc::clone(&worker_id_shared);

        let keep_alive_task = tokio::spawn(async move {
            use n0_future::StreamExt;

            let mut ticker = tokio::time::interval(interval_duration);
            let mut reconnect_delay = INITIAL_RECONNECT_DELAY;
            let mut consecutive_failures = 0u32;
            let mut current_lease_id = lease_id;

            loop {
                ticker.tick().await;

                // Actively send a keep-alive request using the keeper. This is
                // required by the etcd client so the lease is actually refreshed.
                if let Err(err) = keeper.keep_alive().await {
                    log::warn!(
                        target: "distributed-id",
                        "failed to send etcd lease keep-alive (attempt {}): {err:?}",
                        consecutive_failures + 1
                    );
                    consecutive_failures += 1;

                    // Try to reconnect with exponential backoff
                    match try_reconnect(
                        &mut keep_alive_client,
                        &mut keeper,
                        &mut stream,
                        &endpoints_clone,
                        &mut current_lease_id,
                        &mut reconnect_delay,
                        &mut consecutive_failures,
                        &app_name_clone,
                        &options_clone,
                        &key_task,
                        &worker_id_task
                    )
                    .await
                    {
                        ReconnectResult::Success => {
                            // Update shared lease_id
                            lease_id_task.store(current_lease_id, Ordering::SeqCst);
                            continue;
                        }
                        ReconnectResult::Failed => {
                            log::warn!(target: "distributed-id",
                                "failed to reconnect to etcd after multiple attempts, continuing with worker_id {}",
                                worker_id_task.load(Ordering::SeqCst)
                            );
                            break;
                        }
                    }
                }

                // Observe the resulting keep-alive response on the stream. If the
                // stream yields an error or ends, stop the keep-alive loop.
                match stream.next().await {
                    Some(Ok(_)) => {
                        // Keep-alive successful - reset reconnect delay
                        if consecutive_failures > 0 {
                            log::info!(
                                target: "distributed-id",
                                "etcd lease keep-alive recovered after {} failures",
                                consecutive_failures
                            );
                            consecutive_failures = 0;
                            reconnect_delay = INITIAL_RECONNECT_DELAY;
                        }
                    }
                    Some(Err(err)) => {
                        log::warn!(
                            target: "distributed-id",
                            "failed to keep etcd lease alive (attempt {}): {err:?}",
                            consecutive_failures + 1
                        );
                        consecutive_failures += 1;

                        // Try to reconnect with exponential backoff
                        match try_reconnect(
                            &mut keep_alive_client,
                            &mut keeper,
                            &mut stream,
                            &endpoints_clone,
                            &mut current_lease_id,
                            &mut reconnect_delay,
                            &mut consecutive_failures,
                            &app_name_clone,
                            &options_clone,
                            &key_task,
                            &worker_id_task
                        )
                        .await
                        {
                            ReconnectResult::Success => {
                                // Update shared lease_id
                                lease_id_task.store(current_lease_id, Ordering::SeqCst);
                                continue;
                            }
                            ReconnectResult::Failed => {
                                log::warn!(target: "distributed-id",
                                    "failed to reconnect to etcd after multiple attempts, continuing with worker_id {}",
                                    worker_id_task.load(Ordering::SeqCst)
                                );
                                break;
                            }
                        }
                    }
                    None => {
                        log::warn!(
                            target: "distributed-id",
                            "etcd lease keep-alive stream ended unexpectedly (attempt {})",
                            consecutive_failures + 1
                        );
                        consecutive_failures += 1;

                        // Try to reconnect with exponential backoff
                        match try_reconnect(
                            &mut keep_alive_client,
                            &mut keeper,
                            &mut stream,
                            &endpoints_clone,
                            &mut current_lease_id,
                            &mut reconnect_delay,
                            &mut consecutive_failures,
                            &app_name_clone,
                            &options_clone,
                            &key_task,
                            &worker_id_task
                        )
                        .await
                        {
                            ReconnectResult::Success => {
                                // Update shared lease_id
                                lease_id_task.store(current_lease_id, Ordering::SeqCst);
                                continue;
                            }
                            ReconnectResult::Failed => {
                                log::warn!(target: "distributed-id",
                                    "failed to reconnect to etcd after multiple attempts, continuing with worker_id {}",
                                    worker_id_task.load(Ordering::SeqCst)
                                );
                                break;
                            }
                        }
                    }
                }
            }
        });

        Ok(Self {
            client,
            lease_id: lease_id_shared,
            key: key_shared,
            worker_id: worker_id_shared,
            keep_alive_task: Arc::new(TokioMutex::new(Some(keep_alive_task))),
            app_name
        })
    }

    pub async fn worker_id(&self) -> u16 {
        self.worker_id.load(Ordering::SeqCst)
    }
}

enum ReconnectResult {
    Success,
    Failed
}

async fn try_reconnect(
    client: &mut Client,
    keeper: &mut etcd_client::LeaseKeeper,
    stream: &mut etcd_client::LeaseKeepAliveStream,
    endpoints: &[String],
    lease_id: &mut i64,
    reconnect_delay: &mut Duration,
    consecutive_failures: &mut u32,
    app_name: &str,
    options: &EtcdWorkerOptions,
    key_shared: &Arc<RwLock<String>>,
    worker_id_shared: &Arc<AtomicU16>
) -> ReconnectResult {
    const MAX_RECONNECT_ATTEMPTS: u32 = 10;

    if *consecutive_failures >= MAX_RECONNECT_ATTEMPTS {
        log::error!(target: "distributed-id",
            "exceeded maximum reconnection attempts ({}), will continue using worker_id {}",
            MAX_RECONNECT_ATTEMPTS,
            worker_id_shared.load(Ordering::SeqCst)
        );
        return ReconnectResult::Failed;
    }

    log::info!(target: "distributed-id", "attempting to reconnect to etcd (delay: {:?}, attempt: {})", reconnect_delay, consecutive_failures);
    tokio::time::sleep(*reconnect_delay).await;

    // Try to reconnect
    match Client::connect(endpoints.to_vec(), None).await {
        Ok(new_client) => {
            *client = new_client;
            log::info!(target: "distributed-id", "successfully reconnected to etcd");
            log::info!(target: "distributed-id", "re-registering worker and acquiring new lease");

            let ttl_secs = match i64::try_from(options.lease_ttl.as_secs().max(1)) {
                Ok(secs) => secs,
                Err(_) => {
                    log::error!(target: "distributed-id", "lease TTL too large");
                    *reconnect_delay = (*reconnect_delay * 2).min(MAX_RECONNECT_DELAY);
                    return ReconnectResult::Success;
                }
            };

            match client.lease_grant(ttl_secs, None).await {
                Ok(lease) => {
                    let new_lease_id = lease.id();
                    log::info!(target: "distributed-id", "acquired new lease: {}", new_lease_id);

                    // Try to allocate the same worker ID or a new one
                    match allocate_worker_id(client, app_name, options, new_lease_id).await {
                        Ok((worker_id, new_key)) => {
                            log::info!(target: "distributed-id", "re-registered worker with id: {}", worker_id);

                            // Update the shared key and worker_id
                            if let Ok(mut key_guard) = key_shared.write() {
                                *key_guard = new_key;
                            }
                            worker_id_shared.store(worker_id, Ordering::SeqCst);

                            // Establish keep-alive for the new lease
                            match client.lease_keep_alive(new_lease_id).await {
                                Ok((new_keeper, new_stream)) => {
                                    *keeper = new_keeper;
                                    *stream = new_stream;
                                    *lease_id = new_lease_id;

                                    log::info!(target: "distributed-id", "worker re-registration complete, new lease: {}", new_lease_id);
                                    *reconnect_delay = INITIAL_RECONNECT_DELAY;
                                    *consecutive_failures = 0;
                                    ReconnectResult::Success
                                }
                                Err(err) => {
                                    log::warn!(target: "distributed-id", "failed to establish keep-alive for new lease: {err:?}");
                                    *reconnect_delay = (*reconnect_delay * 2).min(MAX_RECONNECT_DELAY);
                                    ReconnectResult::Success
                                }
                            }
                        }
                        Err(err) => {
                            log::warn!(target: "distributed-id", "failed to re-allocate worker id: {err:?}");
                            *reconnect_delay = (*reconnect_delay * 2).min(MAX_RECONNECT_DELAY);
                            ReconnectResult::Success
                        }
                    }
                }
                Err(err) => {
                    log::warn!(target: "distributed-id", "failed to acquire new lease: {err:?}");
                    *reconnect_delay = (*reconnect_delay * 2).min(MAX_RECONNECT_DELAY);
                    ReconnectResult::Success
                }
            }
        }
        Err(err) => {
            log::warn!(target: "distributed-id", "failed to reconnect to etcd: {err:?}");
            *reconnect_delay = (*reconnect_delay * 2).min(MAX_RECONNECT_DELAY);
            ReconnectResult::Success // Continue trying
        }
    }
}

impl fmt::Debug for WorkerLeaseGuard {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WorkerLeaseGuard").field("app_name", &self.app_name).finish()
    }
}

impl Drop for WorkerLeaseGuard {
    fn drop(&mut self) {
        // Force acquire lock to ensure task is properly aborted
        let handle = Arc::clone(&self.keep_alive_task);
        tokio::spawn(async move {
            if let Some(task) = handle.lock().await.take() {
                task.abort();
            }
        });

        let mut client = self.client.clone();
        let key = Arc::clone(&self.key);
        let lease_id = Arc::clone(&self.lease_id);

        tokio::spawn(async move {
            let key_value = key.read().map(|k| k.clone()).unwrap_or_default();
            let lease_id_value = lease_id.load(Ordering::SeqCst);

            if let Err(err) = client.delete(key_value.clone(), None).await {
                log::debug!(target: "distributed-id", "failed to delete etcd worker key {key_value}: {err:?}");
            }

            if let Err(err) = client.lease_revoke(lease_id_value).await {
                log::debug!(target: "distributed-id", "failed to revoke etcd lease {lease_id_value}: {err:?}");
            }
        });
    }
}

pub async fn register_worker(
    app_name: &str,
    mut options: EtcdWorkerOptions
) -> Result<(Worker, WorkerLeaseGuard), DistributedIdError> {
    options.normalize();
    options.validate()?;

    let mut client = Client::connect(options.endpoints.clone(), None)
        .await
        .map_err(|err| DistributedIdError::Coordinator(format!("failed to connect to etcd: {err}",)))?;

    let ttl_secs = i64::try_from(options.lease_ttl.as_secs().max(1))
        .map_err(|_| DistributedIdError::InvalidConfig("lease TTL is too large to fit into i64".into()))?;

    let lease = client
        .lease_grant(ttl_secs, None)
        .await
        .map_err(|err| DistributedIdError::Coordinator(format!("failed to acquire etcd lease: {err}",)))?;
    let lease_id = lease.id();

    let (worker_id, key) = allocate_worker_id(&mut client, app_name, &options, lease_id).await?;

    let guard = WorkerLeaseGuard::new(
        client.clone(),
        lease_id,
        key.clone(),
        worker_id,
        options.keep_alive_interval(),
        options.endpoints.clone(),
        app_name.to_string(),
        options.clone()
    )
    .await?;

    let worker = Worker {
        distributed_id: worker_id as u64,
        name: app_name.to_owned(),
        created_at: Utc::now()
    };

    Ok((worker, guard))
}

async fn allocate_worker_id(
    client: &mut Client,
    app_name: &str,
    options: &EtcdWorkerOptions,
    lease_id: i64
) -> Result<(u16, String), DistributedIdError> {
    let prefix = options.workers_prefix(app_name);

    for worker_id in 0..=MAX_WORKER_ID {
        let key = format!("{prefix}/{worker_id}");
        let value = json!({
            "app": app_name,
            "worker_id": worker_id,
            "allocated_at": Utc::now().to_rfc3339()
        })
        .to_string();

        let compare = Compare::create_revision(key.clone(), CompareOp::Equal, 0);
        let put = TxnOp::put(key.clone(), value, Some(PutOptions::new().with_lease(lease_id)));
        let txn = Txn::new().when(vec![compare]).and_then(vec![put]);

        let response = client.txn(txn).await.map_err(|err| {
            DistributedIdError::Coordinator(format!("failed while reserving worker id {worker_id}: {err}",))
        })?;

        if response.succeeded() {
            return Ok((worker_id, key));
        }
    }

    Err(DistributedIdError::WorkerIdsExhausted(options.namespace_with_app(app_name)))
}
