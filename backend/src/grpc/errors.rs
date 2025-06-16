use tonic::Status;

use crate::di_container::DiContainerError;

impl From<DiContainerError> for Status {
    fn from(value: DiContainerError) -> Self {
        match value {
            DiContainerError::GrpcGatewayChannelError(error) => Status::internal(error.to_string())
        }
    }
}
