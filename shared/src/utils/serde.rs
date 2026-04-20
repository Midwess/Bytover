pub mod as_string {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S, T>(x: &T, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        T: ToString,
    {
        s.serialize_str(&x.to_string())
    }

    pub fn deserialize<'de, D, T>(d: D) -> Result<T, D::Error>
    where
        D: Deserializer<'de>,
        T: std::str::FromStr + Default,
        <T as std::str::FromStr>::Err: std::fmt::Display,
    {
        let s = String::deserialize(d)?;
        match s.parse::<T>() {
            Ok(val) => Ok(val),
            Err(_) => {
                // Fallback to default if parsing fails
                Ok(T::default())
            }
        }
    }
}
