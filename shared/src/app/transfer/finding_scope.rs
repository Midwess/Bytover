use chrono::Local;
use serde::{Deserialize, Serialize};
use uniffi::Enum;
use h3ron::{H3Cell, HasH3Resolution, Index};
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
        
        // Extract latitude and longitude from geo_location
        let lat = geo_location.latitude;
        let lng = geo_location.longitude;
        
        // Use resolution 15 for approximately 10m² cells
        let resolution = 12;
        
        // Get the center cell
        let center = H3Cell::from_coordinate(
            Coord { x: lng, y: lat }, 
            resolution
        ).unwrap();
        
        let mut cells = Vec::with_capacity(5);
        let mut cell_set = HashSet::new();
        
        // Add the center cell
        let center_str = center.to_string();
        cells.push(center_str.clone());
        cell_set.insert(center_str);
        
        // Define small offsets in degrees for the 4 directions
        // This is approximately 10m at the equator
        let step = 0.0001;
        
        // Top cell
        let top_point = Coord { x: lng, y: lat + step };
        let Ok(top_cell) = H3Cell::from_coordinate(top_point, resolution) else {
            return vec![];
        };

        let top_str = top_cell.to_string();
        if !cell_set.contains(&top_str) {
            cells.push(top_str.clone());
            cell_set.insert(top_str);
        }
        
        // Top-right cell
        let top_right_point = Coord { x: lng + step, y: lat + step };
        let Ok(top_right_cell) = H3Cell::from_coordinate(top_right_point, resolution) else {
            return vec![];
        };

        let top_right_str = top_right_cell.to_string();
        if !cell_set.contains(&top_right_str) {
            cells.push(top_right_str.clone());
            cell_set.insert(top_right_str);
        }
        
        // Bottom-right cell
        let bottom_right_point = Coord { x: lng + step, y: lat - step };
        let Ok(bottom_right_cell) = H3Cell::from_coordinate(bottom_right_point, resolution) else {
            return vec![];
        };

        let bottom_right_str = bottom_right_cell.to_string();
        if !cell_set.contains(&bottom_right_str) {
            cells.push(bottom_right_str.clone());
            cell_set.insert(bottom_right_str);
        }
        
        // Bottom cell
        let bottom_point = Coord { x: lng, y: lat - step };
        let Ok(bottom_cell) = H3Cell::from_coordinate(bottom_point, resolution) else {
            return vec![];
        };

        let bottom_str = bottom_cell.to_string();
        if !cell_set.contains(&bottom_str) {
            cells.push(bottom_str);
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
