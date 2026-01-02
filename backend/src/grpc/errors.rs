use crate::app_gateway::app_info::AppInfoErrors;
use crate::cloud_storage::storage::CloudStorageErrors;
use crate::di_container::DiContainerError;
use crate::transfer::p2p_transfer_service::P2PTransferErrors;
use crate::transfer::transfer_service::TransferErrors;
use tonic::Status;

impl From<DiContainerError> for Status {
    fn from(value: DiContainerError) -> Self {
        match value {
            DiContainerError::GrpcGatewayChannelError(error) => Status::internal(error.to_string()),
            DiContainerError::CronError(error) => Status::internal(error)
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

impl From<CloudStorageErrors> for Status {
    fn from(value: CloudStorageErrors) -> Self {
        let value_msg = value.to_string();
        Status::internal(value_msg)
    }
}

impl From<AppInfoErrors> for Status {
    fn from(value: AppInfoErrors) -> Self {
        let value_msg = value.to_string();
        Status::internal(value_msg)
    }
}

impl From<P2PTransferErrors> for Status {
    fn from(value: P2PTransferErrors) -> Self {
        let value_msg = value.to_string();
        Status::internal(value_msg)
    }
}
