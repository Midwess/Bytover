tonic::include_proto!("value.datetime");

impl Datetime {
    pub fn now() -> Self {
        Self {
            utc_timestamp: chrono::Utc::now().timestamp_millis(),
            since: 0,
        }
    }
}
