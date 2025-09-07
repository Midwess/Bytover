use gloo_worker::Registrable;
use core_services::logger::setup;
use wasm::web_worker::{WriterWebWorker};

fn main() {
    setup();
    println!("Worker started");
    WriterWebWorker::registrar().register();
    println!("Worker started");
}
