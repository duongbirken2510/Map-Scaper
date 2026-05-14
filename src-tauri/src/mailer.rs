use lettre::message::{header, MultiPart, SinglePart};
use lettre::{Message, SmtpTransport, Transport};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct SmtpConfig {
    pub email: String,
    pub password: String,
}

pub fn send_email_sync(
    config: &SmtpConfig,
    to_email: &str,
    subject: &str,
    html_body: &str,
) -> Result<(), String> {
    if config.email.is_empty() || config.password.is_empty() {
        return Err("Vui lòng nhập Email và Mật khẩu ứng dụng (App Password).".to_string());
    }

    let email = Message::builder()
        .from(config.email.parse().map_err(|_| "Email gửi không hợp lệ")?)
        .to(to_email.parse().map_err(|_| "Email nhận không hợp lệ")?)
        .subject(subject)
        .multipart(
            MultiPart::alternative()
                .singlepart(
                    SinglePart::builder()
                        .header(header::ContentType::TEXT_HTML)
                        .body(html_body.to_string()),
                ),
        )
        .map_err(|e| format!("Lỗi tạo thư: {}", e))?;

    let creds = lettre::transport::smtp::authentication::Credentials::new(
        config.email.clone(),
        config.password.clone(),
    );

    let mailer = SmtpTransport::relay("smtp.gmail.com")
        .map_err(|e| format!("Lỗi kết nối SMTP: {}", e))?
        .credentials(creds)
        .build();

    mailer.send(&email).map_err(|e| format!("Lỗi gửi email: {}", e))?;

    Ok(())
}
