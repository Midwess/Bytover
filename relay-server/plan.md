# Dual-Connection WebRTC Relay Architecture

## Overview
Instead of a TURN server or a transparent SDP hack, each client will explicitly establish **two** concurrent WebRTC connection attempts:
1. **Direct P2P Connection**: The standard peer-to-peer connection flow via signaling.
2. **Relay Connection**: An explicitly negotiated client-server WebRTC connection to a Relay Server.

The Relay Server acts as a stateful message proxy (B2BUA) that stitches two client-server WebRTC connections together. If the direct P2P connection succeeds, clients can terminate the Relay connection. If P2P fails (e.g., due to symmetric NAT), traffic falls back to the Relay connection without interruption.

## Architecture & Flow

### 1. Client 1 Initiates Relay
- Client 1 wants to connect to Client 2. 
- Client 1 creates a WebRTC Offer (`Offer_R1`) intended specifically for the Relay Server.
- Client 1 calls the Signalling server endpoint `/relay` **along with its `signalling_key`**. The Signalling Server proxies this to the Relay Server via the gRPC `connect` endpoint.
- The payload sent to the Relay Server is a `ConnectRequest` protobuf message. It must include the SDP offer, a generated `session_id` (acting as the proxy ID), and the `repeated DataChannel channels` configuration so the Relay Server can provision the exact data channels needed.
- Relay server creates a **Proxy Instance** keyed by the `session_id`, reads the `channels` config to setup the `Rtc` instance, accepts `Offer_R1`, and generates `Answer_R1`.
- Relay server returns a `ConnectResponse` containing `Answer_R1`. Signalling returns this back to Client 1.
- **Result**: Client 1 <-> Relay Server WebRTC connection is forming.

### 2. Client 1 Initiates P2P
- Client 1 generates its normal P2P WebRTC Offer (`Offer_P2P`).
- Client 1 sends `Offer_P2P` to Client 2 via standard Signalling, but **includes the `session_id`** in the message payload.

### 3. Client 2 Receives Request
- Client 2 receives the P2P connection request containing `Offer_P2P` and the `session_id`.
- Client 2 processes `Offer_P2P`, generates `Answer_P2P`, and sends it back to Client 1 via Signalling. 
- **Result**: Client 1 <-> Client 2 Direct P2P WebRTC connection is forming.

### 4. Client 2 Joins Relay
- Concurrently, Client 2 initializes a WebRTC connection for the relay fallback.
- Client 2 creates a WebRTC Offer (`Offer_R2`) intended for the Relay Server.
- Client 2 calls the Signalling server endpoint `/relay` **and its `signalling_key`**.
- Signalling forwards this to the Relay server as a `ConnectRequest`. This request includes `Offer_R2` and specifies the same `session_id` and matching `DataChannel channels` config.
- Relay server looks up the active Proxy Instance by `session_id`, associates the second WebRTC connection leg, sets up the data channels, accepts `Offer_R2`, and generates `Answer_R2`.
- Relay server returns a `ConnectResponse` containing `Answer_R2` via Signalling back to Client 2.
- **Result**: Relay Server <-> Client 2 WebRTC connection is forming.

### 5. Connection Race & Fallback
- Both clients now attempt to establish their connections over Direct P2P and the Relay.
- If the **Direct P2P connection is successfully established**, the clients gracefully close their WebRTC instances connected to the Relay server to save server bandwidth.
- If the **Direct P2P connection fails** (e.g. ICE timeout), they rely on the already-negotiated WebRTC Relay connection for data transfer. 
- The Relay Server automatically forwards data channel messages back and forth between Leg 1 and Leg 2.

## Sequence Diagram

```mermaid
sequenceDiagram
    participant C1 as Client 1
    participant Sig as Signalling
    participant Rel as Relay Server
    participant C2 as Client 2

    Note over C1, Rel: Phase 1: Client 1 establishes Relay Leg
    C1->>Sig: POST /relay (Offer_R1, signalling_key)
    Sig->>Rel: gRPC ConnectRequest(Offer_R1, session_id, channels)
    Rel-->>Sig: ConnectResponse(Answer_R1)
    Sig-->>C1: Answer_R1 + session_id
    Note over C1, Rel: WebRTC Connection (C1 <-> Relay) initializing

    Note over C1, C2: Phase 2: Standard P2P Signaling
    C1->>Sig: Signal P2P Offer (Offer_P2P, session_id)
    Sig->>C2: Deliver P2P (Offer_P2P, session_id)
    
    Note over C2, C1: Phase 3a: Client 2 Responds to P2P
    C2->>Sig: Signal P2P Answer (Answer_P2P)
    Sig->>C1: Deliver P2P Answer
    Note over C1, C2: WebRTC Connection (C1 <-> C2) P2P initializing

    Note over C2, Rel: Phase 3b: Client 2 establishes Relay Leg
    C2->>Sig: POST /relay (Offer_R2, session_id, signalling_key)
    Sig->>Rel: gRPC ConnectRequest(Offer_R2, session_id, channels)
    Rel-->>Sig: ConnectResponse(Answer_R2)
    Sig-->>C2: Answer_R2
    Note over C2, Rel: WebRTC Connection (C2 <-> Relay) initializing
    
    Note over C1, C2: Phase 4: Resolution
    alt P2P Succeeds
        C1->>Rel: Teardown Relay Connection
        C2->>Rel: Teardown Relay Connection
        C1<-->>C2: Data flows via Direct P2P
    else P2P Fails
        C1<-->>Rel: Data (Leg 1)
        Rel<-->>C2: Data (Leg 2)
        Note over Rel: Relay forwards messages transparently
    end
```

## Relay Server Internal Architecture

The `relay-server` internal architecture mirrors the robust async loop design found in `native/src/webrtc/server.rs`, but with modifications to handle dual-leg proxying rather than terminating into the local application core.

### 1. State Management & Dependency Injection
- The Relay Server structure must be completely thread-safe and modeled as a **Singleton via `DIContainer`**.
- All server APIs must only require an immutable `&self` reference. This interior mutability allows concurrent gRPC request handlers to safely instruct the server to construct or modify proxy instances without blocking.
- The server maintains a thread-safe registry of active proxy sessions: `Mutex<HashMap<String, Arc<ProxyInstance>>>`.
- The key is the `session_id` defined in the `ConnectRequest` protobuf message.

### 2. ProxyInstance
A `ProxyInstance` represents a single isolated relay session holding two `RelayRtcClient` connections:
- **Leg 1**: The connection to Client 1 (the initiator).
- **Leg 2**: The connection to Client 2 (the responder, added dynamically when they call `/relay` with the same `session_id`).

### 3. Asynchronous Multiplexing
Similar to `WebRtcServer::start` in the native client, the Relay Server will have a main event loop utilizing `FuturesUnordered` to multiplex concurrent state progressions without blocking:
- **`connect_futs`**: Futures handling the `RelayRtcClient::connect` negotiation (allocating sockets, accepting offers). Once a leg is established, it attaches to the corresponding `ProxyInstance`.
- **`run_handles`**: Futures executing the active WebRTC `poll_event` message loops for each connected leg.

### 4. Transparent Data Forwarding
When both legs of a `ProxyInstance` are actively connected, the sequence behaves as a bridge:
- When a `ChannelData` event is received on **Leg 1**, the `ProxyInstance` immediately routes that data payload to the equivalent `ChannelId` on **Leg 2** via `RelayRtcClient::send()`.
- Likewise, `ChannelData` from **Leg 2** is routed directly to **Leg 1**.
- Because the relay server uses `str0m`, it handles all the heavy lifting of DTLS termination and SCTP packet buffering. The application logic simply shuffles the raw bytes between the two connected `str0m::Rtc` instances without needing to decode the Protobuf bodies.

### 5. Cleanup & Lifecycle
- If a client determines the Direct P2P connection was successful, it will gracefully close its WebRTC connection to the Relay Server.
- **Strict Termination Rule**: Once a `ProxyInstance` has been successfully connected, if *either* Leg 1 or Leg 2 terminates/disconnects for any reason, the `ProxyInstance` will immediately shut down the surviving leg and terminate itself.
- The corresponding `ProxyInstance` cleans up any active Tokio tasks and removes itself from the global `HashMap` to free up server resources immediately.

## Region Detection

The relay-server self-identifies its bytover region (`asia` / `us` / `eu`) for signalling registration via a strict precedence chain:

1. **`BYTOVER_REGION_CODE`** env (operator override). Top priority. Logged as `source=env`.
2. **GeoIP lookup** on the STUN-discovered public IPv4 against a bundled MaxMind GeoLite2-Country DB (`assets/GeoLite2-Country.mmdb`). Logged as `source=geoip ipv4=…`.
3. **gRPC `GetRegion`** call to the backend (legacy fallback). Logged as `source=grpc`.

Country → region mapping lives in `src/geoip.rs::country_to_region`. Unmapped countries (e.g. BR, ZA, RU as of this writing) fall through to the gRPC path; if you operate relays in those markets, set `BYTOVER_REGION_CODE` explicitly.

The GeoIP database is fetched at image-build time using the `MAXMIND_LICENSE_KEY` build arg; see `assets/README.md`. Distributing GeoLite2 data requires the upstream `LICENSE.txt` to ship alongside it (CC BY-SA 4.0):

> This product includes GeoLite2 data created by MaxMind, available from <https://www.maxmind.com>.

If the DB is missing at startup the relay logs a `WARN` and continues with the env-or-gRPC paths only — startup never fails on missing GeoIP data.
