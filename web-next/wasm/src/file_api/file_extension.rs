use bytes::Bytes;
use js_sys::Uint8Array;
use wasm_bindgen::JsValue;

pub trait VecExtension {
    fn into_uint_array(&self) -> Uint8Array;

    fn into_js_value(&self) -> JsValue {
        self.into_uint_array().into()
    }
    
    fn into_uint_array_leak(&self) -> Uint8Array;
}

impl VecExtension for Vec<u8> {
    fn into_uint_array(&self) -> Uint8Array {
        Uint8Array::from(self.as_slice())
    }

    fn into_uint_array_leak(&self) -> Uint8Array {
        unsafe { Uint8Array::view(self) }
    }
}

impl VecExtension for Bytes {
    fn into_uint_array(&self) -> Uint8Array {
        Uint8Array::from(self.as_ref())
    }

    fn into_uint_array_leak(&self) -> Uint8Array {
        unsafe { Uint8Array::view(self) }
    }
}
