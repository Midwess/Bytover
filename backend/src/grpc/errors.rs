use tonic::Status;

use crate::di_container::DiContainerError;
use crate::transfer::transfer_service::TransferErrors;

impl From<DiContainerError> for Status {
    fn from(value: DiContainerError) -> Self {
        match value {
            DiContainerError::GrpcGatewayChannelError(error) => Status::internal(error.to_string())
        }
    }
}

impl From<TransferErrors> for Status {
    fn from(value: TransferErrors) -> Self {
        let value_msg = format!("{value}");
        match value {
            TransferErrors::SessionNotFound => Status::not_found(value_msg),
            TransferErrors::ResourceNotFoundOrAlreadyCompleted => Status::internal(value_msg),
            TransferErrors::EmptyResources => Status::invalid_argument(value_msg),

            _ => Status::internal(value_msg)
        }
    }
}
