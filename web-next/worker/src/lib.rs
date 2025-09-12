use gloo_worker::Registrable;
use wasm_bindgen::prelude::*;
use wasm::web_worker::codec::WorkerMessageCodec;
use wasm::web_worker::core::CoreWorker;

#[wasm_bindgen(start)]
pub async fn start_worker() {
    CoreWorker::registrar().encoding::<WorkerMessageCodec>().register();
}
