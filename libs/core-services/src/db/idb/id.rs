use super::table::convert_large_numbers_to_strings;
use crate::db::repository::abstraction::errors::RepositoryError;
use crate::db::repository::abstraction::id::DbId;
use crate::db::repository::abstraction::repository::SendSync;
use js_sys::Array;
use std::fmt::Debug;
use wasm_bindgen::{JsCast, JsValue};

pub trait IdbId: DbId + SendSync + Debug + serde::Serialize + for<'de> serde::Deserialize<'de> {
    // The default implementation will serialize your struct
    // into an array in the same order of you defining them.
    fn serialize(&self) -> Result<JsValue, RepositoryError> {
        let json_value = serde_json::to_value(self)?;
        let array_json_value = Self::normalize_to_array(json_value);
        let js_value = serde_wasm_bindgen::to_value(&array_json_value)?;

        Ok(js_value)
    }

    fn into_query_value(&self) -> Result<JsValue, RepositoryError> {
        let js_value = IdbId::serialize(self)?;
        let array: Array = js_value.unchecked_into();
        for i in 0..array.length() {
            let element = array.get(i);
            if element.is_undefined() {
                array.set(i, JsValue::from_str(""));
            }
        }

        Ok(array.into())
    }

    fn equals(&self, other: &Self) -> Result<bool, RepositoryError> {
        let self_json: Array = IdbId::serialize(self)?.unchecked_into();
        let other_json: Array = IdbId::serialize(other)?.unchecked_into();

        if self_json.length() != other_json.length() {
            return Ok(false);
        }

        for i in 0..self_json.length() {
            let self_element = self_json.get(i);
            let other_element = other_json.get(i);
            if self_element.is_undefined() || self_element.is_null() {
                continue;
            }

            if self_element.ne(&other_element) {
                return Ok(false);
            }
        }

        Ok(true)
    }

    fn deserialize(value: JsValue) -> Result<Self, RepositoryError>;

    fn normalize_to_array(value: serde_json::Value) -> serde_json::Value {
        let value = convert_large_numbers_to_strings(value);
        match value {
            serde_json::Value::Array(arr) => serde_json::Value::Array(arr),
            serde_json::Value::Object(obj) => {
                let entries: Vec<_> = obj.into_iter().collect();
                let values = entries.into_iter().map(|(_, v)| v).collect();
                serde_json::Value::Array(values)
            }
            other => serde_json::Value::Array(vec![other])
        }
    }
}
