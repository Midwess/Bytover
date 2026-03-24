# Refactor
- The current signaling logic was too complicated
# What to remove:
- The rooms, scopes, broadcast scopes, PeerRequestRelayOnly need to be removed
- Also review messages in @../libs/schema/proto/devlog/rpc-signaling to remove unused event
- from_id, and to_id should be replaced by a single request_id
# Ideas for new flow:
- The server-client will call websocket endpoint at /server/<key>
- Then we create Client instance like current,
- We create client instance with a key: String
- Every client need wrapped in Arc<Client> like current
- Create a ClientManager.rs with a hashmap inside which hold Weak ref to client
- Then we accept another http server with endpoint /offer/<key> (within the same port)
- Then other client may send an http request with an offer to endpoint /offer/<key> with the body is a message in protobuf
- Then we will get correct client-server instance inside ClientManager.rs
- And forward the message to the client-server using client.request() -> Result<Response>
- The request will have a request_id with random string in message, so that when client-server send back a message with correct string we will treated it as a response
- All logic will handle inside request function the http server just see if let Some(client) = manager.get_client() {
  - let response = client.send(request).await;
- }
- The http request will wait for the response from client-server
# What unchanges:
- Don't changes the turn logic (turn_manager.rs, turn_server.registry.rs)