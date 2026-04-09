#[cfg(feature = "s3")]
use aws_sdk_s3::error::SdkError;
#[cfg(feature = "s3")]
use aws_sdk_s3::presigning::PresigningConfigError;
use thiserror::Error;
#[cfg(feature = "grpc-server")]
use tonic::Status;

#[derive(Error, Debug)]
pub enum Errors {
    #[error("Operation on db name {} got error {}", .db_name, .message)]
    DatabaseError { message: String, db_name: String },
    #[error("{0}")]
    BadRequest(String),
    #[error("User {} already exist", .0)]
    UserAlreadyExist(String),
    #[error("Already exist {}", .0)]
    AlreadyExist(String),
    #[cfg(feature = "grpc-server")]
    #[error("Failed to generate auth token {:?}", .0)]
    GenerateTokenFailed(#[from] jsonwebtoken::errors::Error),
    #[error("UnAuthorize {}", .0)]
    UnAuthorized(String),
    #[error("Resource got error {:?}", .0)]
    AwsS3Error(String),
    #[error("Unexpected data format {}", .0)]
    UnexpectedDataFormat(String),
    #[error("Resource not found {}", .0)]
    ResourceNotFound(String),
    #[error("S3 Resource not found {}", .0)]
    S3NotFound(String),
    #[error("Failed to send email {}", .0)]
    FailedToSendEmail(String),
    #[error("IO Error, {}", .0)]
    IoError(#[from] std::io::Error),
    #[error("All resource is busy")]
    ResourceBusy(),
    #[error("Unsupported resource")]
    UnSupportedResource(),
    #[error("Failed to register service to core_api gateway {}", .0)]
    UnnableToRegisterApiGateway(String),
    #[error("Gateway operation failed {}", .0)]
    GatewayOperationFailed(String),
    #[error("Tcp failure {}", .0)]
    TcpFailure(String),
    #[error("Failed to download file {}", .0)]
    FailedToDownloadFile(String),
    #[error("Invalid s3 path format {}", .0)]
    InvalidS3Path(String),
    #[error("Json error {}", .0)]
    JsonError(#[from] serde_json::Error),
    #[cfg(feature = "smtp")]
    #[error("Failed to send email {}", .0)]
    SmtpError(#[from] lettre::transport::smtp::Error),
    #[cfg(feature = "smtp")]
    #[error("Smtp syntax error {}", .0)]
    SmtpSyntax(#[from] lettre::error::Error)
}

#[cfg(feature = "s3")]
impl<E, R> From<SdkError<E, R>> for Errors
where
    E: std::fmt::Debug,
    R: std::fmt::Debug
{
    fn from(err: SdkError<E, R>) -> Self {
        let msg = match &err {
            SdkError::ConstructionFailure(e) => format!("S3 ConstructionFailure: {:?}", e),
            SdkError::DispatchFailure(e) => format!("S3 DispatchFailure: {:?}", e),
            SdkError::ResponseError(e) => format!("S3 ResponseError: raw={:?}, service={:?}", e.raw(), e),
            SdkError::ServiceError(e) => format!("S3 ServiceError: {:?}", e.err()),
            SdkError::TimeoutError(e) => format!("S3 TimeoutError: {:?}", e),
            other => format!("S3 Unknown error: {:?}", other)
        };

        Self::AwsS3Error(msg)
    }
}

#[cfg(feature = "s3")]
impl From<PresigningConfigError> for Errors {
    fn from(value: PresigningConfigError) -> Self {
        Self::AwsS3Error(format!("{:?}", value))
    }
}

#[cfg(feature = "grpc-server")]
impl From<Errors> for Status {
    fn from(value: Errors) -> Self {
        match value {
            Errors::DatabaseError { message, db_name: _ } => Self::aborted(message),
            Errors::UserAlreadyExist(_) => Self::already_exists("User already exist".to_owned()),
            #[cfg(feature = "grpc-server")]
            Errors::GenerateTokenFailed(_) => Self::unauthenticated("Invalid token"),
            Errors::UnAuthorized(_) => Self::unauthenticated("You are not allow to access this operation"),
            Errors::AwsS3Error(message) => Self::aborted(format!("Resource busy, please contact support {message}")),
            Errors::BadRequest(message) => Self::invalid_argument(message),
            Errors::UnexpectedDataFormat(_) => Self::aborted("Internal error, contact support"),
            Errors::AlreadyExist(message) => Self::already_exists(format!("Already exist {message}")),
            Errors::ResourceNotFound(message) => Self::not_found(message.to_owned()),
            Errors::S3NotFound(message) => Self::not_found(message.to_owned()),
            Errors::FailedToSendEmail(_) => Self::internal("Failed to send email"),
            Errors::IoError(_) => Self::internal("Internal error"),
            Errors::ResourceBusy() => Self::internal("Not resource left, please try again later"),
            Errors::UnSupportedResource() => Self::internal("Unsupported resource"),
            Errors::UnnableToRegisterApiGateway(_) => Self::internal("Service failed to start"),
            Errors::GatewayOperationFailed(_) => Self::internal("Service failed to start"),
            Errors::TcpFailure(_) => Self::internal("Service failed to start"),
            Errors::FailedToDownloadFile(_) => Self::internal("Failed to download file"),
            Errors::InvalidS3Path(_) => Self::invalid_argument("Invalid s3 path"),
            Errors::JsonError(_) => Self::internal("Internal error, contact support"),
            #[cfg(feature = "smtp")]
            Errors::SmtpError(_) => Self::internal("Failed to send email"),
            #[cfg(feature = "smtp")]
            Errors::SmtpSyntax(_) => Self::internal("Failed to send email")
        }
    }
}
