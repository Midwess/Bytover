# Transfer Cancellation Implementation

## 1. Backend: Transfer Context (webrtc/transfer.rs:181-245)

### Add Resource-Level Cancellation
- Add `HashMap<u64, CancellationToken>` to `SessionContext` for resource tokens
- Resource tokens are children of session token (cancel session → cancels all resources)
- Add methods:
  - `add_resource_token(session_id, resource_id, token)`
  - `cancel_resource(session_id, resource_id)`
  - `get_resource_token(session_id, resource_id)`

## 2. Backend: Peer Download Logic (webrtc/peer.rs:618-708)

### Integrate Cancellation in `request_resource_download`
- Create child token from session token at start (line ~627)
- Store in `TransfersContext` with resource_id
- Wrap async operations with `.with_cancel(&token)`:
  - `rx.next().with_cancel(&token).await` in download loop (674-702)
  - Handle `Err(_)` → cancelled, break loop
- Clean up token on completion/cancellation (line ~704)

### Integrate Cancellation in `stream_resource` (711-774)
- Get token from `TransfersContext`
- Wrap async operations with `.with_cancel(&token)`:
  - `cursor.c_next(None).with_cancel(&token).await` (line 743)
  - `outbound_packet_sender.send().with_cancel(&token).await` (752, 768)
- Stop sending packets when cancelled

## 3. Backend: Cancel Message Handler (webrtc/peer.rs:194-272)

### Extend `process_message_packet`
- Add new message type: `CancelResourceRequest { session_id, resource_id }`
- Handler calls `transfers_context.cancel_resource(session_id, resource_id)`

## 4. Backend: Transfer Command Validation (transfer/command or module)

### Add P2P Check for Cancellation
- When receiving cancel resource operation, check if session is P2P
- Only allow cancellation if `session.transfer_type == TransferType::P2P`
- Return error for cloud transfers (not cancellable)
- Forward to peer.rs only if P2P

## 5. Core Operation Flow
```
User clicks progress → CancelResourceTransfer operation
→ transfer/module checks if session is P2P
→ If P2P: sends CancelResourceRequest to peer
→ peer.rs cancels token via TransfersContext
→ download/upload loop exits on cancellation
→ If Cloud: ignore or return error
```