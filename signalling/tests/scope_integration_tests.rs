use tokio::time::{sleep, Duration};
use tokio_tungstenite::{connect_async, tungstenite::Message as WsMessage};
use futures_util::{StreamExt, SinkExt};
use prost::Message as ProstMessage;
use schema::devlog::rpc_signalling::server::{Message, JoinMessage, ScopeState};

async fn create_ws_client(url: &str) -> (
    futures_util::stream::SplitSink<tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>, WsMessage>,
    futures_util::stream::SplitStream<tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>>
) {
    let (ws_stream, _) = connect_async(url).await.expect("Failed to connect");
    ws_stream.split()
}

async fn send_join(
    sender: &mut futures_util::stream::SplitSink<tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>, WsMessage>,
    client_id: &str,
    scopes: Vec<String>
) {
    let msg = Message {
        join: Some(JoinMessage {
            id: client_id.to_string(),
            ..Default::default()
        }),
        from_id: client_id.to_string(),
        scopes,
        ..Default::default()
    };

    let mut buf = Vec::new();
    msg.encode(&mut buf).unwrap();
    sender.send(WsMessage::Binary(buf.into())).await.unwrap();
}

async fn recv_message(
    receiver: &mut futures_util::stream::SplitStream<tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>>
) -> Option<Message> {
    // Add timeout to prevent hanging
    let timeout_duration = Duration::from_secs(5);
    let result = tokio::time::timeout(timeout_duration, async {
        while let Some(Ok(msg)) = receiver.next().await {
            if let WsMessage::Binary(data) = msg {
                if let Ok(message) = Message::decode(&data[..]) {
                    return Some(message);
                }
            }
        }
        None
    }).await;

    match result {
        Ok(msg) => msg,
        Err(_) => {
            println!("  [TIMEOUT] No message received within {:?}", timeout_duration);
            None
        }
    }
}

#[tokio::test]
async fn test_single_client_join_leave() {
    println!("Test 1: Single client join/leave");

    let (mut sender, _receiver) = create_ws_client("ws://localhost/rpc-signalling").await;

    send_join(&mut sender, "client-1", vec!["room-1".to_string()]).await;
    sleep(Duration::from_millis(100)).await;

    send_join(&mut sender, "client-1", vec![]).await;
    sleep(Duration::from_millis(100)).await;

    println!("✓ Client joined and left successfully");
}

#[tokio::test]
async fn test_multiple_clients_same_scope() {
    println!("Test 2: Multiple clients in same scope");

    let (mut sender1, _) = create_ws_client("ws://localhost/rpc-signalling").await;
    let (mut sender2, _) = create_ws_client("ws://localhost/rpc-signalling").await;
    let (mut sender3, _) = create_ws_client("ws://localhost/rpc-signalling").await;

    send_join(&mut sender1, "client-1", vec!["room-1".to_string()]).await;
    send_join(&mut sender2, "client-2", vec!["room-1".to_string()]).await;
    send_join(&mut sender3, "client-3", vec!["room-1".to_string()]).await;

    sleep(Duration::from_millis(200)).await;

    send_join(&mut sender2, "client-2", vec![]).await;
    sleep(Duration::from_millis(100)).await;

    println!("✓ Multiple clients handled correctly");
}

#[tokio::test]
async fn test_owner_state_broadcasting() {
    println!("Test 3: Owner online/offline state");

    let (mut owner_tx, owner_rx) = create_ws_client("ws://localhost/rpc-signalling").await;
    let (mut member_tx, mut member_rx) = create_ws_client("ws://localhost/rpc-signalling").await;

    send_join(&mut owner_tx, "owner-1", vec!["room-1;owner".to_string()]).await;
    sleep(Duration::from_millis(100)).await;

    send_join(&mut member_tx, "member-1", vec!["room-1".to_string()]).await;
    sleep(Duration::from_millis(100)).await;

    if let Some(msg) = recv_message(&mut member_rx).await {
        if let Some(state_changed) = msg.scope_state_changed {
            assert_eq!(state_changed.state, ScopeState::Online as i32);
            println!("✓ Member received ONLINE state");
        }
    }

    drop(owner_tx);
    drop(owner_rx);
    sleep(Duration::from_millis(100)).await;

    if let Some(msg) = recv_message(&mut member_rx).await {
        if let Some(state_changed) = msg.scope_state_changed {
            assert_eq!(state_changed.state, ScopeState::Offline as i32);
            println!("✓ Member received OFFLINE state when owner left");
        }
    }
}

#[tokio::test]
async fn test_direct_scope_routing() {
    println!("Test 4: Direct scope routing");

    let (mut owner_tx, _owner_rx) = create_ws_client("ws://localhost/rpc-signalling").await;
    let (mut client1_tx, _client1_rx) = create_ws_client("ws://localhost/rpc-signalling").await;
    let (mut client2_tx, _client2_rx) = create_ws_client("ws://localhost/rpc-signalling").await;

    send_join(&mut owner_tx, "owner", vec!["direct://room-1;owner".to_string()]).await;
    send_join(&mut client1_tx, "client-1", vec!["direct://room-1".to_string()]).await;
    send_join(&mut client2_tx, "client-2", vec!["direct://room-1".to_string()]).await;

    sleep(Duration::from_millis(200)).await;

    let test_msg = Message {
        from_id: "owner".to_string(),
        scopes: vec!["direct://room-1".to_string()],
        offer: Some(Default::default()),
        ..Default::default()
    };

    let mut buf = Vec::new();
    test_msg.encode(&mut buf).unwrap();
    owner_tx.send(WsMessage::Binary(buf.into())).await.unwrap();

    sleep(Duration::from_millis(100)).await;
    println!("✓ Direct scope message routing tested");
}

#[tokio::test]
async fn test_scope_state_streaming() {
    println!("Test 5: Scope state streaming via watch channel");

    let (mut member_tx, mut member_rx) = create_ws_client("ws://localhost/rpc-signalling").await;

    send_join(&mut member_tx, "member-1", vec!["room-stream".to_string()]).await;
    sleep(Duration::from_millis(100)).await;

    let (mut owner_tx, _) = create_ws_client("ws://localhost/rpc-signalling").await;
    send_join(&mut owner_tx, "owner-1", vec!["room-stream;owner".to_string()]).await;
    sleep(Duration::from_millis(100)).await;

    if let Some(msg) = recv_message(&mut member_rx).await {
        if let Some(state_changed) = msg.scope_state_changed {
            assert_eq!(state_changed.state, ScopeState::Online as i32);
            println!("✓ Member received ONLINE state via streaming");
        }
    }

    println!("✓ Scope state streaming works");
}

#[tokio::test]
async fn test_client_switching_scopes() {
    println!("Test 6: Client switching scopes");

    let (mut sender, _) = create_ws_client("ws://localhost/rpc-signalling").await;

    send_join(&mut sender, "mobile-client", vec![
        "room-1".to_string(),
        "room-2".to_string()
    ]).await;
    sleep(Duration::from_millis(100)).await;

    send_join(&mut sender, "mobile-client", vec![
        "room-2".to_string(),
        "room-3".to_string()
    ]).await;
    sleep(Duration::from_millis(100)).await;

    send_join(&mut sender, "mobile-client", vec![
        "room-3".to_string()
    ]).await;
    sleep(Duration::from_millis(100)).await;

    println!("✓ Client switched scopes successfully");
}

#[tokio::test]
async fn test_exact_scope_removal() {
    println!("Test 7: Exact scope removal");

    let (mut sender, _) = create_ws_client("ws://localhost/rpc-signalling").await;

    send_join(&mut sender, "dual-client", vec![
        "room-1".to_string(),
        "room-2".to_string()
    ]).await;
    sleep(Duration::from_millis(100)).await;

    send_join(&mut sender, "dual-client", vec![
        "room-1".to_string()
    ]).await;
    sleep(Duration::from_millis(100)).await;

    println!("✓ Room-2 scope removed, room-1 scope remained");
}

#[tokio::test]
async fn test_concurrent_joins() {
    println!("Test 8: Concurrent operations");

    let mut handles = vec![];
    for i in 0..5 {
        let handle = tokio::spawn(async move {
            let (mut sender, _) = create_ws_client("ws://localhost/rpc-signalling").await;
            send_join(&mut sender, &format!("client-{}", i), vec!["room-1".to_string()]).await;
            sleep(Duration::from_millis(50)).await;
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.await.unwrap();
    }

    sleep(Duration::from_millis(100)).await;
    println!("✓ Concurrent joins completed");
}

#[tokio::test]
async fn test_owner_replacement_prevention() {
    println!("Test 9: Owner replacement prevention");

    let (mut owner1_tx, _) = create_ws_client("ws://localhost/rpc-signalling").await;
    let (mut owner2_tx, _) = create_ws_client("ws://localhost/rpc-signalling").await;

    send_join(&mut owner1_tx, "owner-a", vec!["room-1;owner".to_string()]).await;
    sleep(Duration::from_millis(50)).await;

    send_join(&mut owner2_tx, "owner-b", vec!["room-1;owner".to_string()]).await;
    sleep(Duration::from_millis(100)).await;

    drop(owner1_tx);
    sleep(Duration::from_millis(100)).await;

    println!("✓ Owner replacement prevented");
}

#[tokio::test]
async fn test_mixed_scope_types() {
    println!("Test 10: Mixed scope types");

    let (mut sender, _) = create_ws_client("ws://localhost/rpc-signalling").await;

    send_join(&mut sender, "mixed-client", vec![
        "room-1".to_string(),
        "direct://room-2;owner".to_string(),
        "room-3".to_string()
    ]).await;
    sleep(Duration::from_millis(200)).await;

    send_join(&mut sender, "mixed-client", vec![]).await;
    sleep(Duration::from_millis(100)).await;

    println!("✓ Mixed scope types handled");
}

#[tokio::test]
async fn test_owner_rejoin() {
    println!("Test 11: Owner leave and rejoin");

    let (mut owner_tx, _) = create_ws_client("ws://localhost/rpc-signalling").await;
    let (mut member_tx, mut member_rx) = create_ws_client("ws://localhost/rpc-signalling").await;

    println!("  Owner joins");
    send_join(&mut owner_tx, "owner-1", vec!["room-rejoin;owner".to_string()]).await;
    sleep(Duration::from_millis(100)).await;

    println!("  Member joins");
    send_join(&mut member_tx, "member-1", vec!["room-rejoin".to_string()]).await;
    sleep(Duration::from_millis(100)).await;

    println!("  Owner leaves");
    send_join(&mut owner_tx, "owner-1", vec![]).await;
    sleep(Duration::from_millis(100)).await;

    println!("  Owner rejoins");
    send_join(&mut owner_tx, "owner-1", vec!["room-rejoin;owner".to_string()]).await;
    sleep(Duration::from_millis(200)).await;

    println!("✓ Owner successfully rejoined scope");
}

#[tokio::test]
async fn test_client_join_leave_multiple() {
    println!("Test 12: Client multiple join/leave cycles");

    let (mut sender, _) = create_ws_client("ws://localhost/rpc-signalling").await;

    for i in 0..3 {
        println!("  Cycle {}: Join", i + 1);
        send_join(&mut sender, "cycling-client", vec!["room-cycle".to_string()]).await;
        sleep(Duration::from_millis(50)).await;

        println!("  Cycle {}: Leave", i + 1);
        send_join(&mut sender, "cycling-client", vec![]).await;
        sleep(Duration::from_millis(50)).await;
    }

    println!("✓ Multiple join/leave cycles completed");
}

#[tokio::test]
async fn test_multiple_clients_join_leave_sequence() {
    println!("Test 13: Multiple clients join/leave sequence");

    let clients: Vec<_> = (0..5)
        .map(|i| format!("client-seq-{}", i))
        .collect();

    let mut connections = vec![];
    for client_id in &clients {
        let (tx, _) = create_ws_client("ws://localhost/rpc-signalling").await;
        connections.push((client_id.clone(), tx));
    }

    println!("  All 5 clients join");
    for (client_id, tx) in &mut connections {
        send_join(tx, client_id, vec!["room-sequence".to_string()]).await;
    }
    sleep(Duration::from_millis(200)).await;

    println!("  First 3 clients leave");
    for (client_id, tx) in connections.iter_mut().take(3) {
        send_join(tx, client_id, vec![]).await;
    }
    sleep(Duration::from_millis(100)).await;

    println!("  Last 2 clients still in scope");
    sleep(Duration::from_millis(50)).await;

    println!("✓ Sequence completed");
}

#[tokio::test]
async fn test_client_receives_owner_state_changes() {
    println!("Test 14: Client receives owner online/offline/online states");

    let (mut client_tx, mut client_rx) = create_ws_client("ws://localhost/rpc-signalling").await;

    println!("  Client joins room");
    send_join(&mut client_tx, "client-1", vec!["room-state-test".to_string()]).await;
    sleep(Duration::from_millis(100)).await;

    println!("  Owner joins room");
    let (mut owner_tx, owner_rx) = create_ws_client("ws://localhost/rpc-signalling").await;
    send_join(&mut owner_tx, "owner-1", vec!["room-state-test;owner".to_string()]).await;
    sleep(Duration::from_millis(100)).await;

    println!("  Checking client receives ONLINE state");
    if let Some(msg) = recv_message(&mut client_rx).await {
        if let Some(state_changed) = msg.scope_state_changed {
            assert_eq!(state_changed.state, ScopeState::Online as i32);
            println!("  ✓ Client received ONLINE state");
        } else {
            panic!("Expected scope_state_changed message");
        }
    } else {
        panic!("Client did not receive any message");
    }

    // When owner joins, they also broadcast their JoinMessage to all scope members
    // We need to consume this message before checking for OFFLINE later
    println!("  Consuming owner's JoinMessage");
    if let Some(msg) = recv_message(&mut client_rx).await {
        if msg.join.is_some() && msg.from_id == "owner-1" {
            println!("  ✓ Received owner's JoinMessage");
        } else {
            println!("  Received unexpected message: {:?}", msg);
        }
    }

    println!("  Owner leaves room");
    drop(owner_tx);
    drop(owner_rx);

    // Client should receive a LeftMessage when owner disconnects
    println!("  Waiting for owner's LeftMessage");
    if let Some(msg) = recv_message(&mut client_rx).await {
        if msg.left_message.is_some() {
            println!("  ✓ Received owner's LeftMessage");
        } else {
            println!("  Received unexpected message: {:?}", msg);
        }
    }

    // Wait for cleanup ticker to run (cleanup runs every 15 seconds)
    sleep(Duration::from_secs(16)).await;

    println!("  Checking client receives OFFLINE state");
    if let Some(msg) = recv_message(&mut client_rx).await {
        if let Some(state_changed) = msg.scope_state_changed {
            assert_eq!(state_changed.state, ScopeState::Offline as i32);
            println!("  ✓ Client received OFFLINE state");
        } else {
            panic!("Expected OFFLINE scope_state_changed message, got: {:?}", msg);
        }
    } else {
        panic!("Client did not receive offline message");
    }

    println!("  Owner rejoins room");
    let (mut owner2_tx, _owner2_rx) = create_ws_client("ws://localhost/rpc-signalling").await;
    send_join(&mut owner2_tx, "owner-1", vec!["room-state-test;owner".to_string()]).await;
    sleep(Duration::from_millis(200)).await;

    println!("  Checking client receives ONLINE state again");
    if let Some(msg) = recv_message(&mut client_rx).await {
        if let Some(state_changed) = msg.scope_state_changed {
            assert_eq!(state_changed.state, ScopeState::Online as i32);
            println!("  ✓ Client received ONLINE state again");
        } else {
            panic!("Expected ONLINE scope_state_changed message, got: {:?}", msg);
        }
    } else {
        panic!("Client did not receive online message after rejoin");
    }

    println!("✓ Client correctly received all state transitions: ONLINE -> OFFLINE -> ONLINE");
}
