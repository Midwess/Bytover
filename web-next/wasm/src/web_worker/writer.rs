use gloo::worker::{HandlerId, Worker, WorkerScope};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

// Define the message types for communication
#[derive(Serialize, Deserialize, Debug)]
pub enum WorkerMessage {
    WriteData { data: String },
    ProcessFile { filename: String, content: Vec<u8> },
    CalculateHash { data: Vec<u8> },
}

#[derive(Serialize, Deserialize, Debug)]
pub enum WorkerResponse {
    WriteComplete { bytes_written: usize },
    FileProcessed { result: String },
    HashCalculated { hash: String },
    Error { message: String },
}

pub struct WriterWebWorker {
    // Add any state you need to maintain in the worker
    processed_count: usize,
}

impl Worker for WriterWebWorker {
    type Message = WorkerMessage;
    type Input = WorkerMessage;
    type Output = WorkerResponse;

    fn create(scope: &WorkerScope<Self>) -> Self {
        // Initialize the worker
        web_sys::console::log_1(&"WriterWebWorker created".into());
        
        Self {
            processed_count: 0,
        }
    }

    fn update(&mut self, scope: &WorkerScope<Self>, msg: Self::Message) {
        // Handle internal messages (if any)
        match msg {
            WorkerMessage::WriteData { data } => {
                self.processed_count += 1;
                let response = WorkerResponse::WriteComplete { 
                    bytes_written: data.len() 
                };
            }
            WorkerMessage::ProcessFile { filename, content } => {
                let result = self.process_file(&filename, &content);
                let response = WorkerResponse::FileProcessed { result };
            }
            WorkerMessage::CalculateHash { data } => {
                let hash = self.calculate_hash(&data);
                let response = WorkerResponse::HashCalculated { hash };
            }
        }
    }

    fn received(&mut self, scope: &WorkerScope<Self>, msg: Self::Input, id: HandlerId) {
        // Handle messages from the main thread
        self.update(scope, msg);
    }
}

impl WriterWebWorker {
    fn process_file(&self, filename: &str, content: &[u8]) -> String {
        // Simulate file processing
        format!("Processed {} bytes from {}", content.len(), filename)
    }
    
    fn calculate_hash(&self, data: &[u8]) -> String {
        // Simple hash calculation (you might want to use a proper hashing library)
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        data.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }
}

