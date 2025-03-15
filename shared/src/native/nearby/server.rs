use tiny_http::{Request, Response, Server, StatusCode};
use tokio::{spawn, task::{spawn_blocking, JoinHandle}};
use std::path::Path;
use std::fs::File;
use std::collections::HashMap;
use std::sync::Arc;
use url::Url;

use crate::{entities::user::User, native::message_to_shell::MessageToShell, serialize, ShellRuntime};

pub struct HttpServer {
    port: u16,
    current_active_workdir: String,
    server_handle: Option<JoinHandle<()>>,
    shell_runtime: Arc<dyn ShellRuntime>
}

impl HttpServer {
    pub fn new(port: u16, current_active_workdir: String, shell_runtime: Arc<dyn ShellRuntime>) -> Self {
        Self { port, current_active_workdir, server_handle: None, shell_runtime }
    }

    pub async fn start(&mut self) {
        let ns = "http-server";
        let server = match {
            let port = self.port;
            spawn_blocking(move || Server::http(format!("0.0.0.0:{}", port)))
        }.await.unwrap() {
            Ok(server) => server,
            Err(e) => {
                log::error!(target: ns, "Failed to start server: {:?}", e);
                return;
            }
        };

        log::info!(target: ns, "Server started on port {}", self.port);

        let shell_runtime = self.shell_runtime.clone();
        let server_handle = spawn_blocking(move || {
            for request in server.incoming_requests() {
                let url = request.url().to_string();
                let parsed_url = match Url::parse(&url) {
                    Ok(url) => url,
                    Err(e) => {
                        log::error!(target: ns, "Failed to parse URL: {:?}", e);
                        let response = Response::from_string("Invalid URL")
                            .with_status_code(400);
                        let _ = request.respond(response);
                        continue;
                    }
                };

                let path = parsed_url.path().to_string();
                let query_params: HashMap<String, String> = parsed_url.query_pairs()
                    .map(|(k, v)| (k.to_string(), v.to_string()))
                    .collect();

                match path.as_str() {
                    str if str.ends_with("say-hello") => {
                        let shell_runtime = shell_runtime.clone();
                        spawn(async move {
                            say_hello(request, path, query_params, shell_runtime).await;
                        });
                    }
                    str if str.ends_with("thumbnail") => {

                    }
                    str if str.ends_with("session-invitation") => {
                    }
                    str if str.ends_with("file_download") => {

                    }
                    _ => {
                        let response = Response::new_empty(StatusCode(404));
                        let _ = request.respond(response);
                    }
                }
            }
        });
        
        self.server_handle = Some(server_handle);
    }

    pub fn stop(&mut self) {
        if let Some(server_handle) = self.server_handle.take() {
            log::info!(target: "http-server", "Stopping server");
            server_handle.abort();
        }

        self.server_handle = None;
    }
}

impl Drop for HttpServer {
    fn drop(&mut self) {
        self.stop();
    }
}

async fn say_hello(request: Request, path: String, query_params: HashMap<String, String>, shell_runtime: Arc<dyn ShellRuntime>) {
    let response = Response::new_empty(StatusCode(200));
    shell_runtime.msg_from_native(serialize(&MessageToShell::NewNearby {
        address: request.remote_addr().unwrap().to_string(),
        user: User {
            name: "John Doe".to_string(),
            email: "john.doe@example.com".to_string(),
            avatar: "https://example.com/avatar.png".to_string()
        }
    }));

    let _ = spawn_blocking(move || request.respond(response));
}