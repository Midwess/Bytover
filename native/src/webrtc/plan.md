# Migration client-server plan
- In the native/webrtc it is the client-server, which it is data source for file transfer
- We want to re-do the low-performance shared/src/protocol/webrtc/webrtc.rs with new version
- Migration all functions of client-server from shared/src/protocol/webrtc/webrtc.rs to native/src/webrtc/server.rs
- We don't migration the start and stop server
- All the function that need @native/src/webrtc/client.rs could be mock with todo! we will do it later