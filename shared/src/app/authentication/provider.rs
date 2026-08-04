use schema::value::auth_method::AuthMethod;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum LoginProvider {
    Google,
    Apple,
}

impl LoginProvider {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Google => "Google",
            Self::Apple => "Apple",
        }
    }
}

impl From<LoginProvider> for AuthMethod {
    fn from(provider: LoginProvider) -> Self {
        match provider {
            LoginProvider::Google => Self::Google,
            LoginProvider::Apple => Self::Apple,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn providers_map_to_stable_schema_values() {
        assert_eq!(AuthMethod::from(LoginProvider::Google), AuthMethod::Google);
        assert_eq!(AuthMethod::from(LoginProvider::Apple), AuthMethod::Apple);
        assert_eq!(AuthMethod::Apple as i32, 3);
    }
}
