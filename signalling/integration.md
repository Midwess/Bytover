# WebRTC Signaling Server Integration

This document provides developer documentation for integrating with the WebRTC signaling server.

The signaling server handles peer coordination for WebRTC connections, including:
- WebSocket-based session establishment
- SDP offer/answer exchange via HTTP
- TURN/STUN relay configuration provisioning

## Architecture Overview

```
┌─────────────┐         ┌─────────────┐         ┌─────────────┐
│  WebRTC     │         │  Signaling  │         │   TURN      │
│  Client A   │────────▶│   Server    │────────▶│   Servers   │
└─────────────┘         └─────────────┘         └─────────────┘
       │                       │                       │
       │   WebSocket           │   HTTP POST /offer    │ DNS
       │   /server/{key}       │   /relay/{key}       │ Discovery
       │                       │                       │
       ▼                       ▼                       ▼
┌─────────────┐         ┌─────────────┐         ┌─────────────┐
│  WebRTC     │◀────────│   Client    │◀────────│  Cloudflare │
│  Client B   │         │  Registry   │         │     DNS     │
└─────────────┘         └─────────────┘         └─────────────┘
```

## Endpoints

### 1. WebSocket: `/server/{key}`

Establishes a bidirectional signaling channel. Connect using a unique key identifier.

**Connection Flow:**
1. Client opens WebSocket to `ws://host:3003/server/{your_unique_key}`
2. Server detects client continent via GeoIP (from `CF-Connecting-IP`, `X-Forwarded-For`, or peer address)
3. Server assigns TURN/STUN relay configuration based on nearest continent
4. Client can send and receive Protobuf `Message` frames

**Example (tokio-tungstenite):**
```rust
use tokio_tungstenite::{connect_async, tungstenite::Message};

let url = "ws://localhost:3003/server/my-client-id";
let (ws_stream, _) = connect_async(url).await?;
let (mut write, mut read) = ws_stream.split();

// Receive messages
while let Some(msg) = read.next().await {
    if let Ok(Message::Binary(data)) = msg {
        let incoming = Message::decode(&data)?;
        // Handle incoming message
    }
}
```

### 2. HTTP POST: `/offer/{key}`

Sends an SDP offer to a connected WebSocket client and waits for their answer.

**Request:**
- Method: `POST`
- Path: `/offer/{target_client_key}`
- Body: Binary-encoded `Message` containing an `offer` field

**Response:**
- Success: Binary-encoded `Message` containing an `answer` field
- Timeout: 30 seconds
- Errors: `400` (decode error), `503` (client not connected)

**Example:**
```rust
use reqwest::Client;
use schema::devlog::rpc_signalling::server::{Message, OfferMessage};

let client = Client::new();
let offer_msg = Message {
    offer: Some(OfferMessage { sdp: my_sdp_offer }),
    ..Default::default()
};

let mut buf = Vec::new();
offer_msg.encode(&mut buf)?;

let response = client
    .post("http://localhost:3003/offer/remote-client-id")
    .body(buf)
    .send()
    .await?;

let answer_bytes = response.bytes().await?;
let answer_msg = Message::decode(&*answer_bytes)?;
let answer_sdp = answer_msg.answer.unwrap().sdp;
```

### 3. HTTP GET: `/relay/{key}`

Retrieves the pre-assigned ICE (STUN/TURN) configuration for a connected client.

**Request:**
- Method: `GET`
- Path: `/relay/{client_key}`

**Response:**
- Success: Binary-encoded `IceConfig`
- Errors: `503` (client not connected)

**Example:**
```rust
use schema::devlog::rpc_signalling::server::IceConfig;

let response = client
    .get("http://localhost:3003/relay/my-client-id")
    .send()
    .await?;

let config_bytes = response.bytes().await?;
let ice_config = IceConfig::decode(&*config_bytes)?;

// Use with your WebRTC engine
println!("STUN/TURN URLs: {:?}", ice_config.urls);
println!("TURN username: {}", ice_config.username.as_ref().unwrap());
println!("TURN credential: {}", ice_config.credential.as_ref().unwrap());
```

## ICE Server Configuration

The `IceConfig` returned by `/relay/{key}` contains everything needed to configure
a WebRTC engine for NAT traversal:

```rust
pub struct IceConfig {
    /// STUN/TURN server URLs
    /// Example: ["stun:turn-fra.example.com", "turn:turn-fra.example.com:3478?transport=udp", "turn:turn-fra.example.com:3478?transport=tcp"]
    pub urls: Vec<String>,

    /// TURN REST API username (format: "{expiry_timestamp}:{user_identifier}")
    pub username: Option<String>,

    /// HMAC-SHA1 password for TURN authentication
    pub credential: Option<String>,
}
```

### Using IceConfig with str0m

```rust
use str0m::{Rtc, RtcConfig};
use str0m::net::Protocol;

let mut rtc = RtcConfig::new()
    .set_ice_lite(true)
    .build(Instant::now());

// Add ICE servers from config
for url in &ice_config.urls {
    if url.starts_with("stun:") {
        rtc.add_stun_server(url);
    } else if url.starts_with("turn:") {
        rtc.add_turn_server(url, username.as_deref(), credential.as_deref());
    }
}
```

## TURN Credential Generation

The server generates TURN credentials using the TURN REST API protocol,
compatible with Coturn's `static-auth-secret` mechanism.

### How It Works

1. **Username**: `{unix_timestamp}:{user_identifier}`
   - Timestamp is current time + 24 hours (credential expiry)
   - User identifier is a short hash of the peer pair (~72 bits entropy)

2. **Password**: `base64(HMAC-SHA1(secret, username))`
   - Uses the `BYTOVER_TURN_SECRET` environment variable as the HMAC key
   - HMAC provides security without storing per-user secrets on the TURN server

### Credential Format

```
username = "1743000000:A3f2kLmNOPq"
password = base64(HMAC-SHA1("my-secret", "1743000000:A3f2kLmNOPq"))
```

### Why This Matters

The TURN REST API credential scheme allows:
- **Short-lived credentials**: Clients get 24-hour credentials, reducing credential theft risk
- **No per-user secrets on TURN server**: The TURN server only needs the shared secret
- **Scalable authentication**: No database lookup needed; credentials are self-validating

## Geo-Priority TURN Selection

The server uses geo-aware routing to select the nearest TURN server for each client.

### Continent Priority Matrix

| Client Region | Primary TURN | Fallback Order |
|---------------|--------------|----------------|
| Asia (AS)     | Asia         | HKG → Singapore → Tokyo → NorthAS |
| Tokyo         | Tokyo        | NorthAS → Asia → HKG → Singapore |
| Europe (EU)   | Europe       | NorthAS → NA → Asia |
| North America | NA           | SJC → SA → Tokyo |
| US-West (SJC) | SJC          | NA → Tokyo → SA |
| Unknown       | Asia         | HKG → Singapore → Tokyo |

### Load Balancing

Within each continent pool, the server uses round-robin selection based on
`AtomicUsize` connection counters. The server with the lowest counter receives
the next assignment.

```rust
// Simplified selection logic
for &target_continent in priority_order {
    let candidates: Vec<&TurnServer> = servers.iter()
        .filter(|s| s.continent == target_continent)
        .collect();

    if !candidates.is_empty() {
        return candidates.iter()
            .min_by_key(|s| s.counter.load(Ordering::Relaxed))
            .unwrap()
            .clone();
    }
}
```

## Environment Variables

### Required for Full Functionality

| Variable | Purpose |
|----------|---------|
| `BYTOVER_TURN_SECRET` | HMAC secret for TURN credential generation |
| `CLOUD_FLARE_API_TOKEN` | Cloudflare API token for TURN server DNS discovery |
| `CLOUD_FLARE_ZONE_ID` | Cloudflare zone ID for DNS record queries |

### Optional

| Variable | Default | Purpose |
|----------|---------|---------|
| `PORT` | `3003` | Server port |

### DNS Discovery Convention

TURN servers are discovered by querying Cloudflare DNS for A records matching the pattern `turn.*`.
For example, if your zone is `example.com`, create:
- `turn-fra.example.com` → TURN server in Frankfurt
- `turn-sjc.example.com` → TURN server in US-West
- `turn-hkg.example.com` → TURN server in Hong Kong

The continent for each server is detected via MaxMind GeoLite2-City database.

## Complete Integration Example

Here's a full example showing how a WebRTC client integrates with the signaling server:

```rust
use schema::devlog::rpc_signalling::server::{IceConfig, Message, OfferMessage};
use tokio_tungstenite::{connect_async, tungstenite::Message as WsMessage};
use futures_util::{SinkExt, StreamExt};

// 1. Connect to signaling WebSocket
let signaling_url = "ws://localhost:3003/server/my-client-id";
let (ws, _) = connect_async(signaling_url).await?;
let (mut ws_write, mut ws_read) = ws.split();

// 2. Fetch relay configuration
let relay_url = "http://localhost:3003/relay/my-client-id";
let ice_config: IceConfig = fetch_protobuf(relay_url).await?;

// 3. Configure your WebRTC engine
let rtc = configure_rtc(&ice_config)?;

// 4. Create SDP offer via your WebRTC engine
let offer = rtc.create_offer();

// 5. Send offer to remote peer via HTTP
let offer_msg = Message {
    offer: Some(OfferMessage { sdp: offer }),
    ..Default::default()
};

let response = post_protobuf(
    "http://localhost:3003/offer/remote-client-id",
    &offer_msg
).await?;

// 6. Handle answer from remote peer
let answer_sdp = response.answer.unwrap().sdp;
rtc.set_remote_answer(answer_sdp);

// 7. Exchange ICE candidates via WebSocket
// (Your WebRTC engine handles this internally)
while let Some(msg) = ws_read.next().await {
    if let Ok(WsMessage::Binary(data)) = msg {
        let signaling_msg = Message::decode(&data)?;
        if let Some(candidate) = signaling_msg.ice_candidate {
            rtc.add_remote_candidate(candidate);
        }
    }
}
```

## Protobuf Schema

The signaling protocol uses Protocol Buffers (protobuf) for all messages.

### Message Envelope

```protobuf
message Message {
    optional string request_id = 1;  // For correlating requests with responses
    optional OfferMessage offer = 2;  // SDP offer from caller
    optional AnswerMessage answer = 3; // SDP answer from callee
    optional string error = 4;         // Error description
}

message OfferMessage {
    required string sdp = 1;  // SDP offer string
}

message AnswerMessage {
    required string sdp = 1;  // SDP answer string
}

message IceConfig {
    repeated string urls = 1;         // STUN/TURN server URLs
    optional string username = 2;       // TURN REST API username
    optional string credential = 3;     // HMAC-SHA1 password
}
```

## Scope/Channel Model

**Note**: The proto schema and integration tests reference scope/channel concepts
(`JoinMessage`, `ScopeState`) but the current server implementation does not
support them. The current model is direct peer-to-peer via key lookup.

Future versions may implement:
- Scope-based signaling channels (groups of peers)
- `JoinMessage` for joining a scope
- `ScopeState` for tracking peers in a scope

## Error Handling

| Error | Cause | Resolution |
|-------|-------|------------|
| `503 client not connected` | Client key not registered | Ensure client connected to `/server/{key}` first |
| `504 request timed out` | No answer within 30s | Remote peer may be disconnected |
| `400 failed to decode message` | Invalid protobuf | Check message encoding |

## Logging

The server logs at `info!` level for:
- Client connections/disconnections
- TURN assignment success/failure
- Gateway registration

At `warn!` level for:
- Missing environment variables (TURN_SECRET, Cloudflare credentials)
- GeoIP database not found

At `error!` level for:
- WebSocket errors
- Message encode/decode failures

## Security Considerations

1. **TURN credentials expire after 24 hours** — clients should fetch fresh credentials
2. **HMAC secret must be kept secure** — anyone with `BYTOVER_TURN_SECRET` can generate valid credentials
3. **No authentication on signaling endpoints** — rely on your application's auth layer
4. **IP extraction trusts proxy headers** — ensure `CF-Connecting-IP` / `X-Forwarded-For` come from trusted sources

## Dependencies

This documentation references these external crates:

- [`tokio-tungstenite`](https://docs.rs/tokio-tungstenite) — WebSocket client
- [`prost`](https://docs.rs/prost) — Protocol Buffers encode/decode
- [`str0m`](https://docs.rs/str0m) — WebRTC engine (used by native clients)
- [`reqwest`](https://docs.rs/reqwest) — HTTP client
