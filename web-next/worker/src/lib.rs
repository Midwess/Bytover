use gloo_worker::Registrable;
use core_services::logger;
use wasm_bindgen::prelude::*;
use wasm::web_worker::codec::WorkerMessageCodec;

#[wasm_bindgen(start)]
pub async fn start_worker() {
    logger::setup();

    #[cfg(feature = "core")]
    {
        use wasm::web_worker::core::CoreWorker;
        CoreWorker::registrar().encoding::<WorkerMessageCodec>().register();
    }

    #[cfg(feature = "native-executor")]
    {
        use wasm::web_worker::executor::ExecutingWorker;
        ExecutingWorker::registrar().encoding::<WorkerMessageCodec>().register();
    }
}
