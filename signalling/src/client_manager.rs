use std::collections::HashMap;
use std::sync::{Arc, Weak};
use tokio::sync::Mutex;

use crate::client::Client;

pub struct ClientManager {
    clients: Mutex<HashMap<String, Weak<Client>>>
}

impl ClientManager {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            clients: Mutex::new(HashMap::new())
        })
    }

    pub async fn register(&self, key: String, client: &Arc<Client>) {
        let mut clients = self.clients.lock().await;
        clients.insert(key, Arc::downgrade(client));
    }

    pub async fn unregister(&self, key: &str) {
        let mut clients = self.clients.lock().await;
        clients.remove(key);
    }

    pub async fn get(&self, key: &str) -> Option<Arc<Client>> {
        let clients = self.clients.lock().await;
        log::info!("Client list {:?}", clients.keys());
        clients.get(key).and_then(|w| w.upgrade())
    }
}
