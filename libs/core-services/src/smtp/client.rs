use crate::services::base::Resolve;
use crate::services::errors::Errors;
use crate::utils::cancellation::{CancellationToken, FutureExtension};
use lettre::message::header::ContentType;
use lettre::message::{MultiPart, SinglePart};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Tokio1Executor};
use prost::Message;
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use reqwest::StatusCode;
use schema::crafter::email_template::Template;
use schema::crafter::EmailTemplate;
use std::env;
use std::sync::Arc;
use std::time::Duration;

#[derive(Clone)]
pub struct SmtpClient {
    pub transport: Arc<AsyncSmtpTransport<Tokio1Executor>>
}

impl SmtpClient {
    pub async fn new(user_name: &str, password: &str, host: &str, port: u16) -> Resolve<Self> {
        let creds = Credentials::new(user_name.to_owned(), password.to_owned());
        log::info!("Using {host}:{port} as SMTP server");
        let transport =
            Arc::new(AsyncSmtpTransport::<Tokio1Executor>::relay(host)?.port(port).credentials(creds).build());
        Ok(SmtpClient { transport })
    }

    pub async fn send(&self, template: EmailTemplate, from: String, receiver: String) -> Resolve<()> {
        let subject = match &template.template {
            Some(Template::Otp(_)) => "OTP verification".to_owned(),
            Some(Template::Welcome(_)) => "Welcome to Midwess".to_owned(),
            Some(Template::SendFile(_)) => "Files received".to_owned(),
            Some(Template::Feedback(_)) => "Feedback received".to_owned(),
            None => "Midwess".to_owned()
        };

        let html = self.get_html_template(&template).await?;
        let message = lettre::Message::builder()
            .from(from.parse().unwrap())
            .to(receiver.parse().unwrap())
            .subject(subject)
            .multipart(
                MultiPart::alternative().singlepart(SinglePart::builder().header(ContentType::TEXT_HTML).body(html))
            )?;
        self.transport
            .send(message)
            .with_cancel(&CancellationToken::timeout(Duration::from_secs(20)))
            .await
            .map_err(|e| Errors::FailedToSendEmail(format!("Failed to send email, error={e:?}")))??;

        Ok(())
    }

    async fn get_html_template(&self, template: &EmailTemplate) -> Resolve<String> {
        let client = reqwest::Client::new();
        let mut buffer = Vec::new();
        template.encode(&mut buffer).expect("Not able to serialize crafter request to a buffer");

        let mut headers = HeaderMap::new();
        let crafter_url = format!(
            "http://{}:{}/crafter",
            env::var("KONG_GATEWAY_HOST").unwrap_or_default(),
            env::var("KONG_GATEWAY_PORT").unwrap_or("80".to_owned())
        );
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/octet-stream"));
        let res =
            client
                .post(format!("{crafter_url}/email"))
                .body(buffer)
                .headers(headers)
                .send()
                .await
                .map_err(|e| {
                    Errors::FailedToSendEmail(format!("Failed to generate email template from crafter service {e:?}"))
                })?;

        if res.status() != StatusCode::OK {
            return Err(Errors::FailedToSendEmail(format!(
                "The crafter service response with error {:?}",
                res.error_for_status()
            )));
        }

        let bytes = res.bytes().await.expect("The response must be in binary format");
        let crafter_response = String::from_utf8(bytes.to_vec())
            .map_err(|e| Errors::FailedToSendEmail(format!("Failed to parse crafter response as UTF-8: {e}")))?;

        Ok(crafter_response)
    }
}
