# Support feature download all resources for P2P Session
1. We want to allow receivers to have a button to download all resources into a zip file
2. When user press on download all buttons, we will forward event DownloadAll to the transfer module
3. Then the transfer module will create a command to handle, it will create a new progress and session local resource for download all to keep track of progress
4. The command will start a stream to receive series of events about the progress update of each resource, it will not update progress to the entities but it will progress to update to progress instance of session local resource only.
4. We will need to modify the entities/transfer_session.rs to have session_resource: Option<LocalResource> it will be a resource for download all
4. Send operation with list of resources to the webrtc/peer to download all resources
5. The peer will change the saved path of each local resource from opfs://<path> to opfs://zip_entry://<session-order-id>.zip/<path>; then request resource one by one, 
6. in worker/opfs.rs, we will handle the write operation to detect path and create zip entry accordingly
7. Save the zip entry to a HashMap<String, ZipEntry> to keep track of all zip entries