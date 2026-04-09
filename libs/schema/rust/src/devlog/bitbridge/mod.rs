use peer_message_body::{Request, Response};
use tonic::include_proto;

include_proto!("devlog.bitbridge");

impl TryFrom<Response> for IntroduceResponseMessage {
    type Error = String;

    fn try_from(value: Response) -> Result<Self, Self::Error> {
        if let Response::IntroduceResponse(response) = value {
            return Ok(response);
        }

        Err(format!("Not a response IntroduceResponse got {value:?}"))
    }
}

impl std::error::Error for PeerErrorsMessage {}

impl std::fmt::Display for PeerErrorsMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PeerErrorsMessage::InvalidRequest => write!(f, "Invalid request"),
            PeerErrorsMessage::NoResponse => write!(f, "No response"),
            PeerErrorsMessage::InvalidPassword => write!(f, "Invalid password"),
            PeerErrorsMessage::SessionNotFound => write!(f, "Session not found"),
            PeerErrorsMessage::ResourceNotFound => write!(f, "Resource not found")
        }
    }
}

impl TryFrom<Response> for VoidResponseMessage {
    type Error = String;

    fn try_from(value: Response) -> Result<Self, Self::Error> {
        if let Response::VoidResponse(response) = value {
            return Ok(response);
        }

        Err(format!("Not a response VoidResponse got {value:?}"))
    }
}

impl PeerMessageBody {
    pub fn response(request_id: String, response: Response) -> Self {
        Self {
            request_id,
            response: Some(response),
            ..Default::default()
        }
    }

    pub fn request(request_id: String, request: Request) -> Self {
        Self {
            request_id,
            request: Some(request),
            ..Default::default()
        }
    }
}
