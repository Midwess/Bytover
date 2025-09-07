use gloo_worker::Registrable;
use wasm_bindgen::prelude::*;
use wasm::web_worker::WriterWebWorker;

#[wasm_bindgen(start)]
pub async fn start_worker() {
    WriterWebWorker::registrar().register();
}
