use base64::Engine;

fn encode_basic_auth(username: &str, password: &str) -> String {
    let credentials = format!("{}:{}", username, password);
    base64::engine::general_purpose::STANDARD.encode(credentials)
}

#[test]
fn test_basic_auth_encoding() {
    let encoded = encode_basic_auth("user", "secret123");
    assert_eq!(encoded, "dXNlcjpzZWNyZXQxMjM=");

    let decoded = base64::engine::general_purpose::STANDARD.decode(&encoded).unwrap();
    assert_eq!(String::from_utf8(decoded).unwrap(), "user:secret123");
}

#[test]
fn test_basic_auth_decoding() {
    let encoded = "dXNlcjpzZWNyZXQxMjM=";
    let decoded = base64::engine::general_purpose::STANDARD.decode(encoded).unwrap();
    let credentials = String::from_utf8(decoded).unwrap();
    assert_eq!(credentials, "user:secret123");

    let parts: Vec<&str> = credentials.splitn(2, ':').collect();
    assert_eq!(parts.len(), 2);
    assert_eq!(parts[0], "user");
    assert_eq!(parts[1], "secret123");
}
