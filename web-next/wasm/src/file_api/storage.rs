use std::sync::Arc;
use futures::lock::Mutex;
use wasm_bindgen::prelude::*;
use web_sys::File;
use web_sys::js_sys::Array;
use core_services::utils::never_send::NeverSend;

#[wasm_bindgen]
#[derive(Debug, Clone)]
pub struct FileStorage {
    selected_files: Arc<Mutex<Vec<NeverSend<File>>>>,
}

#[wasm_bindgen]
impl FileStorage {
    #[wasm_bindgen(constructor)]
    pub fn new() -> FileStorage {
        FileStorage {
            selected_files: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub async fn add_selected_files(&mut self, files: &Array) {
        let mut current_files = self.selected_files.lock().await;
        for file in files.iter() {
            let f = File::from(file);
            current_files.push(NeverSend(f));
        }
    }

    pub async fn get_all_selected_files(&self) -> Array {
        let arr = Array::new();
        for f in self.selected_files.lock().await.iter() {
            arr.push(f);
        }

        arr
    }
}
