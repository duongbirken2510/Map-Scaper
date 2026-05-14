use lettre::message::{header, MultiPart, SinglePart};
use lettre::{Message, SmtpTransport, Transport};
use regex::Regex;
use serde::{Deserialize, Serialize};
use rand::seq::SliceRandom;

#[derive(Deserialize, Serialize)]
pub struct SmtpConfig {
    pub email: String,
    pub password: String,
}

pub fn spin_text(input: &str) -> String {
    let re = Regex::new(r"\{([^{}]*)\}").unwrap();
    let mut rng = rand::rng();
    
    let result = re.replace_all(input, |caps: &regex::Captures| {
        let options: Vec<&str> = caps[1].split('|').collect();
        options.choose(&mut rng).unwrap_or(&"").to_string()
    });
    
    result.into_owned()
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
        .subject(spin_text(subject))
        .multipart(
            MultiPart::alternative()
                .singlepart(
                    SinglePart::builder()
                        .header(header::ContentType::TEXT_HTML)
                        .body(spin_text(html_body)),
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
