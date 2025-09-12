use gloo_worker::Codec;
use wasm_bindgen::JsValue;

pub struct WorkerMessageCodec;

impl Codec for WorkerMessageCodec {
    fn encode<I>(input: I) -> JsValue
    where
        I: serde::Serialize
    {
        serde_wasm_bindgen::to_value(&input).expect("failed to encode")
    }

    fn decode<O>(input: JsValue) -> O
    where
        O: for<'de> serde::Deserialize<'de>
    {
        serde_wasm_bindgen::from_value(input).expect("failed to decode")
    }
}
