use crate::db::idb::id::IdbId;
use crate::db::repository::abstraction::errors::RepositoryError;
use crate::db::repository::abstraction::table::Table;
use serde_json::Value;
use wasm_bindgen::JsValue;

const MAX_SAFE_JS_INT: u64 = 9_007_199_254_740_991;

pub trait IdbTable<T>: Table<T> + serde::Serialize + for<'de> serde::Deserialize<'de>
where
    T: IdbId
{
    fn id(&self) -> T {
        Table::id(self)
    }

    fn serialize(&self) -> Result<JsValue, RepositoryError> {
        let json_value = serde_json::to_value(self)?;
        let safe_json = convert_large_numbers_to_strings(json_value);
        let js_value = serde_wasm_bindgen::to_value(&safe_json)?;
        Ok(js_value)
    }

    fn deserialize(value: JsValue) -> Result<Self, RepositoryError> {
        let json_value: Value = serde_wasm_bindgen::from_value(value)?;
        let normalized_json = convert_strings_to_numbers(json_value)?;
        Ok(serde_json::from_value(normalized_json)?)
    }
}

pub fn convert_large_numbers_to_strings(value: Value) -> Value {
    match value {
        Value::Number(n) => {
            if let Some(u) = n.as_i64() {
                if u > MAX_SAFE_JS_INT as i64 || u < -(MAX_SAFE_JS_INT as i64) {
                    return Value::String(u.to_string());
                }
            }

            if let Some(u) = n.as_u64() {
                if u > MAX_SAFE_JS_INT {
                    return Value::String(u.to_string());
                }
            }

            Value::Number(n)
        }
        Value::Array(arr) => Value::Array(arr.into_iter().map(convert_large_numbers_to_strings).collect()),
        Value::Object(map) => {
            Value::Object(map.into_iter().map(|(k, v)| (k, convert_large_numbers_to_strings(v))).collect())
        }
        other => other
    }
}

pub fn convert_strings_to_numbers(value: Value) -> Result<Value, RepositoryError> {
    match value {
        Value::String(s) => {
            if let Ok(u) = s.parse::<u64>() {
                if u > MAX_SAFE_JS_INT {
                    return Ok(Value::Number(serde_json::Number::from(u)));
                }
            }

            if let Ok(u) = s.parse::<i64>() {
                if u > MAX_SAFE_JS_INT as i64 || u < -(MAX_SAFE_JS_INT as i64) {
                    return Ok(Value::Number(serde_json::Number::from(u)));
                }
            }

            Ok(Value::String(s))
        }
        Value::Array(arr) => Ok(Value::Array(
            arr.into_iter().map(convert_strings_to_numbers).collect::<Result<_, _>>()?
        )),
        Value::Object(map) => Ok(Value::Object(
            map.into_iter()
                .map(|(k, v)| Ok::<(std::string::String, Value), RepositoryError>((k, convert_strings_to_numbers(v)?)))
                .collect::<Result<_, _>>()?
        )),
        other => Ok(other)
    }
}
