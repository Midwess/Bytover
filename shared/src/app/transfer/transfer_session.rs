pub struct TransferSession {
    pub id: u64,
    pub peer: Peer,
    pub resource_id: u64,
    pub status: TransferStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, SurrealDerive, Record)]
pub enum TransferStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
}

// After user select the resources, and they press on the send button:
// + Create a new transfer session with all selected resources
// + Send to the native processor 
// + The native processor will transfer the session to the peer

// After the peer accept the transfer, they need:
// + Transfer to us again the session that they accepted
// + We will used that session to display the progress of the transfer
// + Open a data channel to us to get the resource, every resource need a separate data channel
// + The data channel name is the resource id and the offset in bytes that they want to get
// + The native process need to update the transfer progress of the session 
// + We only ping update one seconds a time, and update all active sessions
