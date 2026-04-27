#[derive(Debug, thiserror::Error)]
pub enum Errors {
    #[error("Invalid s3 path format {}", .0)]
    InvalidS3Path(String),
}
