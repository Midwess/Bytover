use std::sync::Arc;

use core_services::local_storage::file_system::File;

use crate::app::file_system::file::LocalResourcePath;
use crate::app::transfer::session::TransferSession;
use crate::ShellRuntime;

use super::transfer::UpStream;

pub static BUFFER_SIZE: usize = 1024 * 1024;

pub struct OutBoundTransferProcessor {
    shell_runtime: Arc<dyn ShellRuntime>,
    session: TransferSession,
    up_stream: Arc<dyn UpStream>
}

impl OutBoundTransferProcessor {
    pub fn new(session: TransferSession, shell_runtime: Arc<dyn ShellRuntime>, up_stream: Arc<dyn UpStream>) -> Self {
        Self {
            shell_runtime,
            session,
            up_stream
        }
    }

    pub async fn start(&self) {
        let streams = self.up_stream.prepare(&self.session).await.unwrap();
        let ns = "transfer-processor".to_string();
        tokio_scoped::scope(|scope| {
            for outbound in streams.iter() {
                scope.spawn(async {
                    let file_path = match &outbound.key {
                        LocalResourcePath::LocalPath(path) => path,
                        LocalResourcePath::PlatformIdentifier(identifier) => {
                            // TODO: Ask the shell_runtime to get the correct source path
                            return;
                        }
                    };

                    let file = match File::existing(file_path).await {
                        Ok(file) => file,
                        Err(e) => {
                            log::error!(target: ns.as_str(), "Failed to get file: {}", e);
                            return;
                        }
                    };

                    let mut cursor = match file.cursor(outbound.start_position, BUFFER_SIZE).await {
                        Ok(cursor) => cursor,
                        Err(e) => {
                            log::error!(target: ns.as_str(), "Failed to get cursor: {}", e);
                            return;
                        }
                    };

                    while let Some(buffer) = cursor.next().await.unwrap() {
                        outbound.stream.send(buffer).await.unwrap();
                    }
                });
            }
        });
    }
}
