// my_number_as_string.rs

/// Serialize any i64 or u64 as a string.
pub mod i64_as_string {
    use serde::de::Visitor;
    use serde::{Deserializer, Serializer};
    use std::fmt;

    pub fn serialize<S>(x: &i64, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer
    {
        s.serialize_str(&x.to_string())
    }

    pub fn deserialize<'de, D>(d: D) -> Result<i64, D::Error>
    where
        D: Deserializer<'de>
    {
        struct StringOrNumber;
        impl<'de> Visitor<'de> for StringOrNumber {
            type Value = i64;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a string or number representing an i64")
            }

            fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E> {
                Ok(v)
            }

            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
            where
                E: serde::de::Error
            {
                i64::try_from(v).map_err(|_| E::custom("u64 too large for i64"))
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error
            {
                v.parse::<i64>().map_err(E::custom)
            }
        }
        d.deserialize_any(StringOrNumber)
    }
}

pub mod u64_as_string {
    use serde::de::Visitor;
    use serde::{Deserializer, Serializer};
    use std::fmt;

    pub fn serialize<S>(x: &u64, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer
    {
        s.serialize_str(&x.to_string())
    }

    pub fn deserialize<'de, D>(d: D) -> Result<u64, D::Error>
    where
        D: Deserializer<'de>
    {
        struct StringOrNumber;
        impl<'de> Visitor<'de> for StringOrNumber {
            type Value = u64;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a string or number representing a u64")
            }

            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E> {
                Ok(v)
            }

            fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
            where
                E: serde::de::Error
            {
                if v < 0 {
                    Err(E::custom("negative value not allowed for u64"))
                } else {
                    Ok(v as u64)
                }
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error
            {
                v.parse::<u64>().map_err(E::custom)
            }
        }
        d.deserialize_any(StringOrNumber)
    }
}
