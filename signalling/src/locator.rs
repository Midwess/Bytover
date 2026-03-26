use actix_web::{App, HttpRequest, HttpServer, Responder, Result as ActixResult, web};
use core_services::services::errors::Errors;
use devlog_sdk::api_gateway::client::ApiGatewayClient;
use devlog_sdk::api_gateway::kong::client::KongGatewayAdminClient;
use devlog_sdk::api_gateway::service::{GatewayRouteBuilder, GatewayRouteExpression, GatewayServiceBuilder};
use devlog_sdk::tcp::listener::{TcpConnection, find_tcp_listener};
use geo_types::Coord;
use h3ron::{H3Cell, ToCoordinate};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum LocatorErrors {
    #[error("Core errors {0:?}")]
    CoreErrors(#[from] Errors),
    #[error("Server error {0:?}")]
    ServerError(#[from] std::io::Error)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LocateResponse {
    pub ip_address: String,
    pub location_codes: Vec<String>
}

/// Generate location codes based on H3 cells using the provided coordinates
pub fn generate_location_codes(latitude: f64, longitude: f64) -> Vec<String> {
    // Use resolution 12 for cells, approximately 11m edge length
    let resolution = 11;

    // Get the center cell
    let Ok(center) = H3Cell::from_coordinate(Coord { x: longitude, y: latitude }, resolution) else {
        return vec![];
    };

    let mut cells = Vec::with_capacity(6);
    let mut cell_set = HashSet::new();

    // Add the center cell
    let center_str = center.to_string();
    cells.push(center_str.clone());
    cell_set.insert(center_str);

    // Get all neighbors around the center cell
    let Ok(k_ring) = center.grid_ring_unsafe(1) else {
        // If we can't get neighbors, just return the center cell
        return cells;
    };

    let neighbors: Vec<H3Cell> = k_ring.into_iter().filter(|cell| *cell != center).collect();

    let Ok(center_coord) = center.to_coordinate() else {
        return cells;
    };

    let mut neighbors_with_angles = Vec::new();
    for neighbor in neighbors {
        if let Ok(neighbor_coord) = neighbor.to_coordinate() {
            // Calculate angle from center to neighbor (in radians)
            let dx = neighbor_coord.x - center_coord.x;
            let dy = neighbor_coord.y - center_coord.y;
            let angle = dy.atan2(dx);

            neighbors_with_angles.push((neighbor, angle));
        }
    }

    neighbors_with_angles.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

    // Select neighbors at specific positions:
    // - North (top): closest to π/2 (90°)
    // - Northeast (top-right): closest to π/4 (45°)
    // - East (right): closest to 0°
    // - Southeast (bottom-right): closest to -π/4 (-45°)
    // - South (bottom): closest to -π/2 (-90°)
    let positions = [
        (std::f64::consts::FRAC_PI_2, "top"),           // North (90°)
        (std::f64::consts::FRAC_PI_4, "top-right"),     // Northeast (45°)
        (0.0, "right"),                                 // East (0°)
        (-std::f64::consts::FRAC_PI_4, "bottom-right"), // Southeast (-45°)
        (-std::f64::consts::FRAC_PI_2, "bottom")        // South (-90°)
    ];

    for (target_angle, _position) in positions {
        // Find the neighbor closest to this angle
        if let Some((neighbor, _)) = neighbors_with_angles.iter().min_by(|(_, angle1), (_, angle2)| {
            let diff1 = (angle1 - target_angle).abs();
            let diff2 = (angle2 - target_angle).abs();
            diff1.partial_cmp(&diff2).unwrap_or(std::cmp::Ordering::Equal)
        }) {
            let neighbor_str = neighbor.to_string();
            if !cell_set.contains(&neighbor_str) {
                cells.push(neighbor_str.clone());
                cell_set.insert(neighbor_str);
            }
        }
    }

    cells
}

/// Locating the peer information
/// - Location
/// - Nearby location id
/// - Current public ip address
pub struct LocatorServer {}

impl LocatorServer {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn run(&self) -> Result<(), LocatorErrors> {
        let address = find_tcp_listener(Some(4000)).await?;
        self.setup_gateway(&address).await?;

        // Bind a std TcpListener for actix-web (actix requires std::net::TcpListener)
        let std_listener = address.listener.into_std()?;
        std_listener.set_nonblocking(true)?;

        HttpServer::new(|| App::new().service(locate))
            .workers(4)
            .listen(std_listener)?.run().await?;

        Ok(())
    }

    pub async fn setup_gateway(&self, connection: &TcpConnection) -> Result<(), LocatorErrors> {
        let api_gateway = KongGatewayAdminClient::new(
            devlog_sdk::config::CONFIGS.kong.admin_url.clone()
        );

        let service = GatewayServiceBuilder::new()
            .http(connection.public_host.clone(), connection.port)
            .name("devlog-locator-server")
            .enable_cors(true)
            .routes(vec![
                GatewayRouteBuilder::new()
                    .path(GatewayRouteExpression::start_with("/locator"))
                    .http()
                    .strip_path(true)
                    .public(true)
                    .preserve_host(false)
                    .priority(10)
                    .name("devlog-locator-server-path")
                    .build(),
            ])
            .build();

        api_gateway.register(service).await?;
        log::info!("Registered http service to gateway");

        Ok(())
    }
}

#[derive(Deserialize)]
pub struct LocateQuery {
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
}

#[actix_web::get("/")]
async fn locate(req: HttpRequest, query: web::Query<LocateQuery>) -> ActixResult<impl Responder> {
    let ip_address = real_ip(&req);

    let ip_address = ip_address.split(',').next().unwrap().to_string();

    let location_codes = if let (Some(latitude), Some(longitude)) =
        (query.latitude, query.longitude)
    {
        generate_location_codes(latitude, longitude)
    } else {
        vec![]
    };

    let response = LocateResponse {
        ip_address,
        location_codes,
    };

    Ok(web::Json(response))
}

fn real_ip(req: &HttpRequest) -> String {
    // 1. Cloudflare header
    if let Some(cf_ip) = req.headers().get("CF-Connecting-IP") {
        if let Ok(ip) = cf_ip.to_str() {
            return ip.split(',').next().unwrap().trim().to_string();
        }
    }

    // 2. Standard X-Forwarded-For
    if let Some(xff) = req.headers().get("X-Forwarded-For") {
        if let Ok(list) = xff.to_str() {
            return list.split(',').next().unwrap().trim().to_string();
        }
    }

    // 3. X-Real-IP
    if let Some(xri) = req.headers().get("X-Real-IP") {
        if let Ok(ip) = xri.to_str() {
            return ip.to_string();
        }
    }

    // 4. Fallback (may be Cloudflare IP)
    if let Some(addr) = req.peer_addr() {
        return addr.ip().to_string();
    }

    // Always return something
    uuid::Uuid::new_v4().to_string()
}
