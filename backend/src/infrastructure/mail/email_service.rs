use crate::mail::service::{EmailService, EmailServiceErrors};
use crate::user::Token;
use schema::crafter::EmailTemplate;
use schema::devlog::app_gateway::rpc::mail_service_client::MailServiceClient;
use schema::devlog::app_gateway::rpc::SendMailRequest;
use tonic::metadata::MetadataValue;
use tonic::transport::Channel;
use tonic::Request;

pub struct EmailServiceImpl {
    pub mail_service: MailServiceClient<Channel>,
    pub user_token: Option<Token>
}

impl EmailServiceImpl {
    pub fn new(mail_service: MailServiceClient<Channel>, user_token: Option<Token>) -> Self {
        Self { mail_service, user_token }
    }
}

#[async_trait::async_trait]
impl EmailService for EmailServiceImpl {
    async fn send_email(&self, user_email: &str, template: EmailTemplate) -> Result<(), EmailServiceErrors> {
        let request = Request::new(SendMailRequest {
            from: "team@bytover.com".to_owned(),
            to: user_email.to_string(),
            template
        });
        let mut request = request;
        if let Some(token) = &self.user_token {
            request.metadata_mut().insert("authorization", MetadataValue::try_from(token.as_str()).unwrap());
        }

        let mut mail_service = self.mail_service.clone();
        let response = mail_service.send(request).await.map_err(|e| {
            EmailServiceErrors::SmtpErrors(core_services::services::errors::Errors::FailedToSendEmail(format!(
                "Failed to send email via mail service: {e}"
            )))
        })?;

        let send_mail_response = response.into_inner();
        if !send_mail_response.success {
            return Err(EmailServiceErrors::SmtpErrors(
                core_services::services::errors::Errors::FailedToSendEmail(
                    send_mail_response.message.unwrap_or("Unknown error".to_string())
                )
            ));
        }

        Ok(())
    }
}
