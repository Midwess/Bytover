use tonic::metadata::{Ascii, MetadataValue};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token(pub String);

impl Token {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn as_metadata_value(&self) -> MetadataValue<Ascii> {
        MetadataValue::try_from(self.0.as_str()).unwrap()
    }
}

impl From<Token> for MetadataValue<Ascii> {
    fn from(token: Token) -> Self {
        MetadataValue::try_from(token.0.as_str()).unwrap()
    }
}
