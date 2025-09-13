use gloo_worker::Registrable;
use wasm_bindgen::prelude::*;
use wasm::web_worker::codec::WorkerMessageCodec;

#[cfg(feature = "core")]
use wasm::web_worker::core::CoreWorker;

#[cfg(feature = "opfs")]
use wasm::web_worker::opfs::OpfsWorker;

#[wasm_bindgen(start)]
pub async fn start_worker() {
    #[cfg(feature = "core")]
    CoreWorker::registrar().encoding::<WorkerMessageCodec>().register();
    
    #[cfg(feature = "opfs")]
    OpfsWorker::registrar().encoding::<WorkerMessageCodec>().register();
}
