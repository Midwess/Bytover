use std::collections::HashMap;

use core_services::services::base::Resolve;
use core_services::services::errors::Errors;
use reqwest::StatusCode;
use serde_json::json;

use crate::api_gateway::client::ApiGatewayClient;
use crate::api_gateway::service::{GatewayRoute, GatewayService};

pub struct KongGatewayAdminClient {
    url: String
}

impl KongGatewayAdminClient {
    pub fn new(url: String) -> Self {
        Self { url }
    }

    pub fn url(&self) -> &str {
        &self.url
    }

    async fn handle_cors_plugin(&self, service_name: String, enable_cors: bool) -> Resolve<()> {
        let kong_url = self.url.clone();
        let client = reqwest::Client::new();

        log::info!(target: "kong-admin", "Enabling CORS plugin for service {:?}", service_name);
        if enable_cors {
            let cors_config = json!({
                "name": "cors",
                "config": {
                    "origins": ["*"], // Allow all origins
                    "methods": [
                        "GET", "POST", "PUT", "PATCH", "DELETE", "OPTIONS", "HEAD"
                    ],
                    "headers": [
                        "Accept",
                        "Accept-Encoding",
                        "Authorization",
                        "Content-Type",
                        "X-Grpc-Web",
                        "Grpc-Timeout",
                        "Grpc-Encoding",
                        "Grpc-Accept-Encoding",
                        "User-Agent",
                        "Referer",
                        "Origin",
                        "x-user-agent"
                    ],
                    "exposed_headers": [
                        "*"
                    ],
                    "max_age": 86400 // 24 hours - maximum reasonable cache time for preflight requests
                }
            });

            // Add the CORS plugin to the service
            let resp = client
                .post(format!("{kong_url}/services/{}/plugins", service_name))
                .json(&cors_config)
                .send()
                .await
                .map_err(|it| Errors::UnnableToRegisterApiGateway(format!("Unable to enable CORS plugin: {it}")))?;

            let resp = match resp.status() {
                StatusCode::CONFLICT => {
                    // Get the existing CORS plugin ID
                    let plugins_resp =
                        client.get(format!("{kong_url}/services/{}/plugins", service_name)).send().await.map_err(
                            |it| Errors::UnnableToRegisterApiGateway(format!("Unable to get plugins: {it}"))
                        )?;

                    let plugins: serde_json::Value = plugins_resp.json().await.map_err(|it| {
                        Errors::UnnableToRegisterApiGateway(format!("Unable to parse plugins response: {it}"))
                    })?;

                    let plugin_id = plugins
                        .get("data")
                        .and_then(|d| d.as_array())
                        .and_then(|arr| {
                            arr.iter()
                                .find(|p| p.get("name").and_then(|n| n.as_str()) == Some("cors"))
                                .and_then(|p| p.get("id").and_then(|id| id.as_str()))
                        })
                        .ok_or_else(|| {
                            Errors::UnnableToRegisterApiGateway("CORS plugin exists but ID not found".to_owned())
                        })?;

                    // Update the existing plugin
                    client.patch(format!("{kong_url}/plugins/{}", plugin_id)).json(&cors_config).send().await.map_err(
                        |it| Errors::UnnableToRegisterApiGateway(format!("Unable to update CORS plugin: {it}"))
                    )?
                }
                _ => resp
            };

            if !resp.status().is_success() {
                return Err(Errors::UnnableToRegisterApiGateway(format!(
                    "Failed to enable CORS plugin: {} {:?}",
                    resp.status(),
                    resp.text().await.unwrap()
                )));
            }
        } else {
            // Get the plugin ID first, then delete it
            let plugins_resp = client
                .get(format!("{kong_url}/services/{}/plugins", service_name))
                .send()
                .await
                .map_err(|it| Errors::UnnableToRegisterApiGateway(format!("Unable to get plugins: {it}")))?;

            if plugins_resp.status().is_success() {
                let plugins: serde_json::Value = plugins_resp.json().await.map_err(|it| {
                    Errors::UnnableToRegisterApiGateway(format!("Unable to parse plugins response: {it}"))
                })?;

                if let Some(plugins_array) = plugins.get("data").and_then(|d| d.as_array()) {
                    for plugin in plugins_array {
                        if let Some(name) = plugin.get("name").and_then(|n| n.as_str()) {
                            if name == "cors" {
                                if let Some(plugin_id) = plugin.get("id").and_then(|id| id.as_str()) {
                                    let _ = client
                                        .delete(format!("{kong_url}/plugins/{}", plugin_id))
                                        .send()
                                        .await
                                        .map_err(|it| {
                                            Errors::UnnableToRegisterApiGateway(format!(
                                                "Failed to delete CORS plugin {}: {it}",
                                                plugin_id
                                            ))
                                        })?;
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    async fn handle_options_route(&self, service_name: String, route: &GatewayRoute, enable_web: bool) -> Resolve<()> {
        if enable_web {
            let mut options_route = route.clone();
            options_route.name = format!("{}_-_option", route.name);
            options_route.methods = Some(vec!["OPTIONS".to_owned()]);
            options_route.request_buffering = true;
            options_route.response_buffering = true;
            options_route.protocols = Some(vec![
                "http".to_owned(),
                "https".to_owned(),
            ]);
            options_route.headers = Default::default();
            options_route.priority += 1;

            // Create route for OPTIONS requests
            self.create_or_update_route(service_name, &options_route).await?;

            // Add Access-Control-Allow-Private-Network header to OPTIONS responses
            let kong_url = self.url.clone();
            let client = reqwest::Client::new();
            let resp = client
                .post(format!("{kong_url}/routes/{}/plugins", options_route.name))
                .json(&json!({
                    "name": "response-transformer",
                    "config": {
                        "add": {
                            "headers": ["Access-Control-Allow-Private-Network:true"]
                        }
                    }
                }))
                .send()
                .await
                .map_err(|it| {
                    Errors::UnnableToRegisterApiGateway(format!("Unable to enable response-transformer plugin: {it}"))
                })?;

            if !resp.status().is_success() && StatusCode::CONFLICT != resp.status() {
                return Err(Errors::UnnableToRegisterApiGateway(format!(
                    "Failed to enable response-transformer plugin: {} {:?}",
                    resp.status(),
                    resp.text().await.unwrap()
                )));
            }
        } else {
            let kong_url = self.url.clone();
            let client = reqwest::Client::new();

            // Delete the OPTIONS route
            let options_route_name = format!("{}_-_option", route.name);
            let _ = client
                .delete(format!("{kong_url}/services/{service_name}/routes/{}", options_route_name))
                .send()
                .await
                .map_err(|it| {
                    Errors::UnnableToRegisterApiGateway(format!(
                        "Failed to delete OPTIONS route {}: {it}",
                        options_route_name
                    ))
                })?;
        }

        Ok(())
    }

    async fn handle_grpc_web_plugin(
        &self,
        service_name: String,
        route: &GatewayRoute,
        enable_web: bool
    ) -> Resolve<()> {
        let kong_url = self.url.clone();
        let client = reqwest::Client::new();

        let mut web_route = route.clone();
        web_route.name = format!("{}_-_web", web_route.name);
        web_route.priority += 1;
        web_route.protocols = Some(vec![
            "http".to_owned(),
            "https".to_owned(),
        ]);

        web_route.headers.remove("content-type");
        web_route.headers.insert("content-type".to_owned(), vec!["application/grpc-web+proto".to_owned()]);

        if enable_web {
            // Create route for web
            self.create_or_update_route(service_name.clone(), &web_route).await?;
            // Add the grpc-web plugin to the route
            let resp = client
                .post(format!("{kong_url}/routes/{}/plugins", web_route.name))
                .json(&json!({
                    "name": "grpc-web"
                }))
                .send()
                .await
                .map_err(|it| Errors::UnnableToRegisterApiGateway(format!("Unable to enable rpc-web plugin: {it}")))?;

            if !resp.status().is_success() && StatusCode::CONFLICT != resp.status() {
                return Err(Errors::UnnableToRegisterApiGateway(format!(
                    "Failed to enable grpc-web plugin: {} {:?}",
                    resp.status(),
                    resp.text().await.unwrap()
                )));
            }
        } else {
            // Delete the web route
            let _ = client
                .delete(format!("{kong_url}/services/{service_name}/routes/{}", web_route.name))
                .send()
                .await
                .map_err(|it| {
                    Errors::UnnableToRegisterApiGateway(format!(
                        "Failed to delete web grpc route {}: {it}",
                        web_route.name
                    ))
                })?;
        }

        self.handle_options_route(service_name, &web_route, enable_web).await?;

        Ok(())
    }

    async fn handle_request_transformer_plugin(
        &self,
        service_name: String,
        request_headers: &HashMap<String, String>
    ) -> Resolve<()> {
        let kong_url = self.url.clone();
        let client = reqwest::Client::new();

        if !request_headers.is_empty() {
            let headers_list: Vec<String> = request_headers.iter().map(|(k, v)| format!("{k}:{v}")).collect();

            let plugin_config = json!({
                "name": "request-transformer",
                "config": {
                    "add": {
                        "headers": headers_list
                    }
                }
            });

            let resp = client
                .post(format!("{kong_url}/services/{}/plugins", service_name))
                .json(&plugin_config)
                .send()
                .await
                .map_err(|it| {
                    Errors::UnnableToRegisterApiGateway(format!("Unable to enable request-transformer plugin: {it}"))
                })?;

            let resp = match resp.status() {
                StatusCode::CONFLICT => {
                    let plugins_resp =
                        client.get(format!("{kong_url}/services/{}/plugins", service_name)).send().await.map_err(
                            |it| Errors::UnnableToRegisterApiGateway(format!("Unable to get plugins: {it}"))
                        )?;

                    let plugins: serde_json::Value = plugins_resp.json().await.map_err(|it| {
                        Errors::UnnableToRegisterApiGateway(format!("Unable to parse plugins response: {it}"))
                    })?;

                    let plugin_id = plugins
                        .get("data")
                        .and_then(|d| d.as_array())
                        .and_then(|arr| {
                            arr.iter()
                                .find(|p| p.get("name").and_then(|n| n.as_str()) == Some("request-transformer"))
                                .and_then(|p| p.get("id").and_then(|id| id.as_str()))
                        })
                        .ok_or_else(|| {
                            Errors::UnnableToRegisterApiGateway(
                                "request-transformer plugin exists but ID not found".to_owned()
                            )
                        })?;

                    client
                        .patch(format!("{kong_url}/plugins/{}", plugin_id))
                        .json(&plugin_config)
                        .send()
                        .await
                        .map_err(|it| {
                            Errors::UnnableToRegisterApiGateway(format!(
                                "Unable to update request-transformer plugin: {it}"
                            ))
                        })?
                }
                _ => resp
            };

            if !resp.status().is_success() {
                return Err(Errors::UnnableToRegisterApiGateway(format!(
                    "Failed to enable request-transformer plugin: {} {:?}",
                    resp.status(),
                    resp.text().await.unwrap()
                )));
            }
        } else {
            let plugins_resp = client
                .get(format!("{kong_url}/services/{}/plugins", service_name))
                .send()
                .await
                .map_err(|it| Errors::UnnableToRegisterApiGateway(format!("Unable to get plugins: {it}")))?;

            if plugins_resp.status().is_success() {
                let plugins: serde_json::Value = plugins_resp.json().await.map_err(|it| {
                    Errors::UnnableToRegisterApiGateway(format!("Unable to parse plugins response: {it}"))
                })?;

                if let Some(plugins_array) = plugins.get("data").and_then(|d| d.as_array()) {
                    for plugin in plugins_array {
                        if plugin.get("name").and_then(|n| n.as_str()) == Some("request-transformer") {
                            if let Some(plugin_id) = plugin.get("id").and_then(|id| id.as_str()) {
                                let _ =
                                    client.delete(format!("{kong_url}/plugins/{}", plugin_id)).send().await.map_err(
                                        |it| {
                                            Errors::UnnableToRegisterApiGateway(format!(
                                                "Failed to delete request-transformer plugin {}: {it}",
                                                plugin_id
                                            ))
                                        }
                                    )?;
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    async fn create_or_update_route(&self, service_name: String, route: &GatewayRoute) -> Resolve<()> {
        log::info!(target: "kong-admin", "Creating {}", route.log_summary());
        let kong_url = self.url.clone();
        let client = reqwest::Client::new();

        let resp = client
            .post(format!("{kong_url}/services/{service_name}/routes"))
            .json(&route.as_request_body())
            .send()
            .await
            .map_err(|it| Errors::UnnableToRegisterApiGateway(format!("Unable to register route {it}")))?;

        let resp = match resp.status() {
            StatusCode::CONFLICT => client
                .patch(format!("{kong_url}/services/{service_name}/routes/{}", &route.name))
                .json(&route.as_request_body())
                .send()
                .await
                .map_err(|it| Errors::UnnableToRegisterApiGateway(format!("Unable to register route {it}")))?,
            _ => resp
        };

        if !resp.status().is_success() {
            return Err(Errors::UnnableToRegisterApiGateway(format!(
                "{} {:?}",
                resp.status(),
                resp.text().await.unwrap()
            )));
        }

        Ok(())
    }
}

#[async_trait::async_trait]
impl ApiGatewayClient for KongGatewayAdminClient {
    async fn route_to(&self, service: GatewayService) -> Resolve<()> {
        self.handle_cors_plugin(service.name.clone(), service.enable_cors).await?;
        self.handle_request_transformer_plugin(service.name.clone(), &service.request_headers).await?;
        for route in service.routes {
            self.create_or_update_route(service.name.clone(), &route).await?;

            self.handle_grpc_web_plugin(service.name.clone(), &route, route.enable_grpc_web).await?;
        }

        Ok(())
    }

    async fn delete_service(&self, service_name: &str) -> Resolve<()> {
        let kong_url = self.url.as_str();
        let client = reqwest::Client::new();

        let routes_resp = client.get(format!("{kong_url}/services/{service_name}/routes")).send().await;

        if let Ok(routes_resp) = routes_resp {
            if routes_resp.status().is_success() {
                if let Ok(routes_data) = routes_resp.json::<serde_json::Value>().await {
                    if let Some(routes) = routes_data.get("data").and_then(|v| v.as_array()) {
                        for route in routes {
                            if let Some(route_name) = route.get("name").and_then(|v| v.as_str()) {
                                let _ = client
                                    .delete(format!("{kong_url}/services/{service_name}/routes/{route_name}"))
                                    .send()
                                    .await;
                            }
                        }
                    }
                }
            }
        }

        let resp = client.delete(format!("{kong_url}/services/{service_name}")).send().await.map_err(|it| {
            Errors::UnnableToRegisterApiGateway(format!("Unable to delete service {}: {it}", service_name))
        })?;

        if resp.status() == StatusCode::NOT_FOUND {
            log::info!(target: "kong-admin", "Service {} not found, skipping deletion", service_name);
            return Ok(());
        }

        if !resp.status().is_success() {
            return Err(Errors::UnnableToRegisterApiGateway(format!(
                "Failed to delete service {}: {} {:?}",
                service_name,
                resp.status(),
                resp.text().await.unwrap_or_default()
            )));
        }

        Ok(())
    }

    async fn register(&self, service: GatewayService) -> Resolve<()> {
        let kong_url = self.url.as_str();
        let client = reqwest::Client::new();

        log::info!(target: "kong-admin", "Creating {}", service.log_summary());
        let resp = client.post(format!("{kong_url}/services")).json(&service).send().await.map_err(|it| {
            Errors::UnnableToRegisterApiGateway(format!("Unable to register service, failed to create {it}"))
        })?;

        let resp = match resp.status() {
            StatusCode::CONFLICT => client
                .patch(format!("{kong_url}/services/{}", &service.name))
                .json(&service)
                .send()
                .await
                .map_err(|it| {
                    Errors::UnnableToRegisterApiGateway(format!("Unable to register service, failed to update {it}"))
                })?,
            _ => resp
        };

        if !resp.status().is_success() {
            return Err(Errors::UnnableToRegisterApiGateway(format!("{:?}", resp.text().await)));
        } else if !resp.status().is_success() {
            return Err(Errors::UnnableToRegisterApiGateway(format!(
                "Failed to update service {} {:?}",
                resp.status(),
                resp.text().await
            )));
        }

        self.route_to(service).await?;
        Ok(())
    }

    async fn get_service(&self, service_name: &str) -> Resolve<Option<GatewayService>> {
        let kong_url = self.url.as_str();
        let resp = reqwest::get(format!("{kong_url}/services/{service_name}"))
            .await
            .map_err(|_it| Errors::GatewayOperationFailed("Failed to retrieving service info".to_owned()))?;

        if resp.status() == StatusCode::NOT_FOUND {
            return Ok(None);
        }

        let service: GatewayService = resp.json().await.map_err(|it| Errors::GatewayOperationFailed(it.to_string()))?;

        Ok(Some(service))
    }
}
