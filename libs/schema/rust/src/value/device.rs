use crate::devlog::app_gateway::models::Device;
use crate::value::platform::Platform as SchemaPlatform;
use tonic::include_proto;

include_proto!("value.device");

impl Device {
    pub fn get_platform(&self) -> SchemaPlatform {
        SchemaPlatform::try_from(self.platform).unwrap_or(SchemaPlatform::Web)
    }
}
