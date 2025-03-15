pub const GATEWAY_HOST: &str = env!("DEVLOG_PUBLIC_GATEWAY_HOST");
pub const GATEWAY_PORT: &str = env!("DEVLOG_PUBLIC_GATEWAY_PORT");

pub fn get_gateway_grpc_url() -> String {
    format!("grpc://{}:{}", GATEWAY_HOST, GATEWAY_PORT)
}

pub fn get_signalling_server_ws_url() -> String {
    format!("ws://{}:{}/rpc-signalling", GATEWAY_HOST, GATEWAY_PORT)
}
