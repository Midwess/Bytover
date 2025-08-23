use chrono::Local;
use futures_util::StreamExt;
use h3o::{LatLng, Resolution};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use crate::app::operations::device::GeoLocation;
use crate::app::operations::internet::InternetOperation;
use crate::app::AppCommandContext;
use crate::errors::NetworkError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FindingScope {
    Global(String),
    Local(String),
    Location(u64)
}

impl FindingScope {
    pub async fn local_network(ctx: AppCommandContext) -> Result<Self, NetworkError> {
        let local_ip = InternetOperation::get_current_ip_address().into_future(ctx.clone()).await?;

        Ok(Self::Local(local_ip))
    }

    pub fn nearby_location(geo_location: GeoLocation) -> Vec<Self> {
        let lat = geo_location.latitude;
        let lng = geo_location.longitude;

        let latlng = LatLng::new(lat, lng).unwrap();
        let center = latlng.to_cell(Resolution::Twelve);

        let mut cells = Vec::with_capacity(5);
        let mut cell_set = HashSet::new();

        // Add the center cell
        let center_str = center.to_string();
        cells.push(center_str.clone());
        cell_set.insert(center_str);

        center
            .grid_disk::<Vec<_>>(1)
            .into_iter()
            .filter(|it| (*it).eq(&center))
            .map(|it| Self::Location(it.into()))
            .collect::<Vec<Self>>()
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
        } else if parts[0] == "location" {
            let Ok(id) = scope_key.parse::<u64>() else {
                return None;
            };

            return Some(FindingScope::Location(id));
        }

        None
    }

    pub fn as_string(&self) -> String {
        let gmt_offset = Self::get_gmt_offset();
        match self {
            FindingScope::Global(content) => format!("public:{content}"),
            FindingScope::Local(content) => format!("local:{content}-gmt{gmt_offset}"),
            FindingScope::Location(content) => format!("location:{content}-gmt{gmt_offset}")
        }
    }

    fn get_gmt_offset() -> i32 {
        let local_time = Local::now();
        let offset_seconds = local_time.offset().local_minus_utc();

        offset_seconds / 3600
    }

    pub fn is_local_network_only(&self) -> bool {
        matches!(self, FindingScope::Local(_) | FindingScope::Location(_))
    }

    pub fn is_local(&self) -> bool {
        matches!(self, FindingScope::Local(_))
    }

    pub fn is_location(&self) -> bool {
        matches!(self, FindingScope::Location(_))
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
