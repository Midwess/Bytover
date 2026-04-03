use log::{info, error};
use stun::message::{Message, BINDING_REQUEST, BINDING_SUCCESS};
use stun::agent::TransactionId;
use stun::xoraddr::XorMappedAddress;
use devlog_sdk::tcp::listener::UdpConnection;

pub async fn run_stun_server(conn: UdpConnection) -> anyhow::Result<()> {
    let socket = conn.socket;
    let addr = socket.local_addr()?;
    info!("STUN server listening on {}", addr);

    let mut buf = [0u8; 1500];
    loop {
        let (len, src_addr) = match socket.recv_from(&mut buf).await {
            Ok(res) => res,
            Err(e) => {
                error!("STUN server recv_from error: {}", e);
                continue;
            }
        };

        let mut message = Message::new();
        if let Err(_e) = message.unmarshal_binary(&buf[..len]) {
            // error!("STUN server unmarshal_binary error: {}", e);
            continue;
        }

        if message.typ == BINDING_REQUEST {
            let mut response = Message::new();
            if let Err(e) = response.build(&[
                Box::new(message.transaction_id),
                Box::new(BINDING_SUCCESS),
                Box::new(XorMappedAddress {
                    ip: src_addr.ip(),
                    port: src_addr.port(),
                }),
            ]) {
                error!("STUN server build response error: {}", e);
                continue;
            }

            let encoded = match response.marshal_binary() {
                Ok(bytes) => bytes,
                Err(e) => {
                    error!("STUN server marshal_binary error: {}", e);
                    continue;
                }
            };

            if let Err(e) = socket.send_to(&encoded, src_addr).await {
                error!("STUN server send_to error: {}", e);
            }
        }
    }
}
