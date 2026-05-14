use lettre::message::{header, MultiPart, SinglePart};
use lettre::{Message, SmtpTransport, Transport};
use regex::Regex;
use serde::{Deserialize, Serialize};
use rand::prelude::*;

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

#[derive(Clone, Serialize)]
struct EmailOutreachProgress {
    lead_id: i64,
    lead_name: String,
    email: String,
    result: String,
    done: usize,
    total: usize,
    is_finished: bool,
}

#[tauri::command]
pub async fn email_start_outreach(
    app: tauri::AppHandle,
    state: tauri::State<'_, crate::AppState>,
    lead_ids: Vec<i64>,
) -> Result<(), String> {
    use tauri::Emitter;

    let db = crate::open_db(&state)?;
    let config = SmtpConfig {
        email: db.get_setting("smtp_email"),
        password: db.get_setting("smtp_password"),
    };
    let subject_tpl = db.get_setting("email_subject");
    let body_tpl = db.get_setting("email_body");

    if config.email.is_empty() || config.password.is_empty() {
        return Err("Vui lòng cấu hình SMTP trước khi gửi.".to_string());
    }

    let delay_min: u64 = db.get_setting("zalo_delay_min").parse().unwrap_or(30);
    let delay_max: u64 = db.get_setting("zalo_delay_max").parse().unwrap_or(60);

    tokio::spawn(async move {
        let total = lead_ids.len();
        for (i, id) in lead_ids.iter().enumerate() {
            let done = i + 1;
            let is_finished = done == total;

            if let Ok(Some(lead)) = db.get_lead_by_id(*id) {
                if lead.email.is_empty() {
                    let _ = app.emit("email_outreach_progress", EmailOutreachProgress {
                        lead_id: *id,
                        lead_name: lead.name.clone(),
                        email: "".to_string(),
                        result: "Không có Email".to_string(),
                        done, total, is_finished,
                    });
                    continue;
                }

                let mut subject = subject_tpl.replace("[Tên]", &lead.name);
                subject = subject.replace("[Website]", &lead.website);
                
                let mut body = body_tpl.replace("[Tên]", &lead.name);
                body = body.replace("[Website]", &lead.website);

                let send_res = tokio::task::block_in_place(|| {
                    send_email_sync(&config, &lead.email, &subject, &body)
                });

                let result = match send_res {
                    Ok(_) => "Thành công".to_string(),
                    Err(e) => format!("Thất bại: {}", e),
                };

                let _ = app.emit("email_outreach_progress", EmailOutreachProgress {
                    lead_id: *id,
                    lead_name: lead.name.clone(),
                    email: lead.email.clone(),
                    result,
                    done, total, is_finished,
                });

                if !is_finished {
                    let delay = {
                        let mut rng = rand::rng();
                        rng.random_range(delay_min..=delay_max)
                    };
                    tokio::time::sleep(std::time::Duration::from_secs(delay)).await;
                }
            }
        }
    });

    Ok(())
}
