use schema::crafter::EmailTemplate;

#[derive(Debug, thiserror::Error)]
pub enum EmailServiceErrors {
    #[error("Email service error {0}")]
    SmtpErrors(#[from] core_services::services::errors::Errors),
    #[error("Template rendering error: {0}")]
    TemplateError(String),
    #[error("Invalid user email: {0}")]
    InvalidUserEmail(String),
}

#[async_trait::async_trait]
pub trait EmailService: Send + Sync {
    async fn send_email(
        &self,
        user_email: &str,
        template: EmailTemplate,
    ) -> Result<(), EmailServiceErrors>;
}
