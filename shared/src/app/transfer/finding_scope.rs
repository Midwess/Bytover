use chrono::Local;
use serde::{Deserialize, Serialize};
use uniffi::Enum;
use h3ron::{H3Cell, HasH3Resolution, Index, ToCoordinate};
use geo_types::Coord;
use std::collections::HashSet;

use crate::{app::{operations::{device::DeviceOperation, internet::InternetOperation}, AppCommandContext}, errors::NetworkError};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Enum)]
pub enum FindingScope {
    Global(String),
    Local(String),
}

impl FindingScope {
    pub async fn local_network(ctx: AppCommandContext) -> Result<Self, NetworkError> {
        let local_ip = InternetOperation::get_current_ip_address().into_future(ctx).await?;
        Ok(Self::Local(local_ip))
    }

    pub async fn nearby_location(ctx: AppCommandContext) -> Vec<Self> {
        let Some(geo_location) = DeviceOperation::get_geo_location().into_future(ctx).await else {
            return vec![];
        };
        
        let lat = geo_location.latitude;
        let lng = geo_location.longitude;
        
        // Use resolution 12 for cells, approximately 11m edge length
        let resolution = 12;
        
        // Get the center cell
        let Ok(center) = H3Cell::from_coordinate(
            Coord { x: lng, y: lat }, 
            resolution
        ) else {
            return vec![];
        };
        
        let mut cells = Vec::with_capacity(5);
        let mut cell_set = HashSet::new();
        
        // Add the center cell
        let center_str = center.to_string();
        cells.push(center_str.clone());
        cell_set.insert(center_str);
        
        // Get all neighbors arround the center cell
        let Ok(k_ring) = center.grid_ring_unsafe(1) else {
            // If we can't get neighbors, just return the center cell
            return vec![Self::Local(center.to_string())];
        };
        
        let neighbors: Vec<H3Cell> = k_ring.into_iter()
            .filter(|cell| *cell != center)
            .collect();
        
        let Ok(center_coord) = center.to_coordinate() else {
            return vec![Self::Local(center.to_string())];
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
            (-std::f64::consts::FRAC_PI_2, "bottom"),       // South (-90°)
        ];
        
        for (target_angle, _position) in positions {
            // Find the neighbor closest to this angle
            if let Some((neighbor, _)) = neighbors_with_angles.iter()
                .min_by(|(_, angle1), (_, angle2)| {
                    let diff1 = (angle1 - target_angle).abs();
                    let diff2 = (angle2 - target_angle).abs();
                    diff1.partial_cmp(&diff2).unwrap_or(std::cmp::Ordering::Equal)
                }) 
            {
                let neighbor_str = neighbor.to_string();
                if !cell_set.contains(&neighbor_str) {
                    cells.push(neighbor_str.clone());
                    cell_set.insert(neighbor_str);
                }
            }
        }
        
        // Convert H3 cell IDs to FindingScope instances
        cells.into_iter()
            .map(|cell_id| Self::Local(cell_id))
            .collect()
    }

    pub fn from_string(s: String) -> Option<Self> {
        let parts = s.split(':').collect::<Vec<&str>>();
        if parts.len() < 2 {
            return None;
        }

        let Some(scope_key) = parts[1].split('-').next() else {
            return None;
        };

        if parts[0] == "public" {
            return Some(FindingScope::Global(scope_key.to_string()));
        } else if parts[0] == "local" {
            return Some(FindingScope::Local(scope_key.to_string()));
        }

        None
    }

    pub fn as_string(&self) -> String {
        let gmt_offset = Self::get_gmt_offset();
        match self {
            FindingScope::Global(content) => format!("public:{}", content),
            FindingScope::Local(content) => format!("local:{}-gmt{}", content, gmt_offset),
        }
    }

    fn get_gmt_offset() -> i32 {
        let local_time = Local::now();
        let offset_seconds = local_time.offset().local_minus_utc();
        let offset_hours = offset_seconds / 3600;

        offset_hours
    }

    pub fn is_local(&self) -> bool {
        matches!(self, FindingScope::Local(_))
    }

    pub fn is_global(&self) -> bool {
        matches!(self, FindingScope::Global(_))
    }
}

impl From<String> for FindingScope {
    fn from(s: String) -> Self {
        let parts = s.split(':').collect::<Vec<&str>>();
        if parts[0] == "public" {
            FindingScope::Global(parts[1].to_string())
        } else {
            FindingScope::Local(parts[1].to_string())
        }
    }
}
