mod database;
mod mailer;
mod scraper;
mod zalo;

use database::{CrmLead, CrmStats, Database, DbLead, DbStats};
use scraper::{Lead, ScrapeConfig};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, State};
use zalo::{OutreachConfig, ZaloStatus};

// ============================================================
// App State
// ============================================================

pub struct AppState {
    pub db_path: Mutex<PathBuf>,
    pub session_dir: Mutex<PathBuf>,
}

fn open_db(state: &State<AppState>) -> Result<Database, String> {
    let path = state.db_path.lock().map_err(|e| e.to_string())?;
    Database::open(&path).map_err(|e| e.to_string())
}

// ============================================================
// Scraping Commands
// ============================================================

#[tauri::command]
async fn start_scraping(
    app: AppHandle,
    state: State<'_, AppState>,
    config: ScrapeConfig,
) -> Result<Vec<Lead>, String> {
    let db_path = state.db_path.lock().map_err(|e| e.to_string())?.clone();
    scraper::start_scraping(app, config, db_path).await
}

#[tauri::command]
async fn export_excel(leads: Vec<Lead>, file_path: String) -> Result<String, String> {
    scraper::export_excel(leads, file_path).await
}

// ============================================================
// Database / History Commands
// ============================================================

#[tauri::command]
fn get_history(state: State<AppState>) -> Result<Vec<DbLead>, String> {
    open_db(&state)?.get_all_leads().map_err(|e| e.to_string())
}

#[tauri::command]
fn get_history_by_region(state: State<AppState>, region: String) -> Result<Vec<DbLead>, String> {
    open_db(&state)?.get_leads_by_region(&region).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_db_stats(state: State<AppState>) -> Result<DbStats, String> {
    open_db(&state)?.get_stats().map_err(|e| e.to_string())
}

#[tauri::command]
async fn export_all_to_excel(state: State<'_, AppState>, file_path: String) -> Result<String, String> {
    let db = open_db(&state)?;
    let db_leads = db.get_all_leads().map_err(|e| e.to_string())?;
    let leads: Vec<Lead> = db_leads.into_iter().enumerate().map(|(i, l)| Lead {
        stt: i + 1, name: l.name, phone: l.phone, address: l.address, region: l.region, website: l.website, email: l.email,
    }).collect();
    scraper::export_excel(leads, file_path).await
}

#[tauri::command]
fn clear_all_leads(state: State<AppState>) -> Result<String, String> {
    let deleted = open_db(&state)?.clear_all().map_err(|e| e.to_string())?;
    Ok(format!("Đã xóa {} bản ghi.", deleted))
}

#[tauri::command]
fn clear_leads_by_region(state: State<AppState>, region: String) -> Result<String, String> {
    let deleted = open_db(&state)?.clear_by_region(&region).map_err(|e| e.to_string())?;
    Ok(format!("Đã xóa {} bản ghi khu vực \"{}\".", deleted, region))
}

// ============================================================
// CRM Commands
// ============================================================

#[tauri::command]
fn get_crm_leads(state: State<AppState>, status_filter: Option<String>) -> Result<Vec<CrmLead>, String> {
    let db = open_db(&state)?;
    db.get_crm_leads(status_filter.as_deref()).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_crm_stats(state: State<AppState>) -> Result<CrmStats, String> {
    open_db(&state)?.get_crm_stats().map_err(|e| e.to_string())
}

#[tauri::command]
fn update_lead_status(state: State<AppState>, id: i64, status: String) -> Result<(), String> {
    open_db(&state)?.update_lead_status(id, &status).map_err(|e| e.to_string())
}

#[tauri::command]
fn update_lead_note(state: State<AppState>, id: i64, note: String) -> Result<(), String> {
    open_db(&state)?.update_lead_note(id, &note).map_err(|e| e.to_string())
}

// ============================================================
// Settings Commands
// ============================================================

#[tauri::command]
fn get_settings(state: State<AppState>) -> Result<HashMap<String, String>, String> {
    Ok(open_db(&state)?.get_all_settings())
}

#[tauri::command]
fn save_settings(state: State<AppState>, settings: HashMap<String, String>) -> Result<(), String> {
    let db = open_db(&state)?;
    for (key, value) in &settings {
        db.set_setting(key, value).map_err(|e| e.to_string())?;
    }
    Ok(())
}

// ============================================================
// Zalo Commands (using zca-js sidecar)
// ============================================================

/// Login with cookie + imei + userAgent
#[tauri::command]
fn zalo_login(
    state: State<AppState>,
    cookie: String,
    imei: String,
    user_agent: String,
) -> Result<ZaloStatus, String> {
    let session_dir = state.session_dir.lock().map_err(|e| e.to_string())?.clone();
    zalo::zalo_login(&session_dir, &cookie, &imei, &user_agent)
}

/// Try to login with previously saved credentials
#[tauri::command]
fn zalo_check_session(state: State<AppState>) -> Result<ZaloStatus, String> {
    let session_dir = state.session_dir.lock().map_err(|e| e.to_string())?.clone();
    zalo::zalo_login_saved(&session_dir)
}

/// Login with QR Code — fire-and-forget, emits 'zalo_login_result' event when done
#[tauri::command]
fn zalo_start_qr_login(
    app: AppHandle,
    state: State<AppState>,
) -> Result<String, String> {
    let session_dir = state.session_dir.lock().map_err(|e| e.to_string())?.clone();

    // Spawn a background OS thread so the UI stays fully responsive.
    // zalo_login_qr() holds the SIDECAR mutex while waiting for QR scan (up to 60s).
    // Running it in the main command thread would block ALL other sidecar operations.
    let app_clone = app.clone();
    std::thread::spawn(move || {
        match zalo::zalo_login_qr(&app_clone, &session_dir) {
            Ok(status)  => { let _ = app_clone.emit("zalo_login_result", status); }
            Err(e)      => { let _ = app_clone.emit("zalo_login_error", e); }
        }
    });

    Ok("QR login started — waiting for QR scan...".to_string())
}

/// Start outreach to selected leads
#[tauri::command]
async fn zalo_start_outreach(
    app: AppHandle,
    state: State<'_, AppState>,
    lead_ids: Vec<i64>,
    action: String,
) -> Result<(), String> {
    let db_path = state.db_path.lock().map_err(|e| e.to_string())?.clone();
    let session_dir = state.session_dir.lock().map_err(|e| e.to_string())?.clone();
    
    let db = Database::open(&db_path).map_err(|e| e.to_string())?;
    let settings = db.get_all_settings();

    let all_leads = db.get_crm_leads(None).map_err(|e| e.to_string())?;
    let leads: Vec<(i64, String, String)> = all_leads
        .into_iter()
        .filter(|l| lead_ids.contains(&l.id))
        .map(|l| (l.id, l.name, l.phone))
        .collect();

    let config = OutreachConfig {
        db_path,
        session_dir,
        message_template: settings.get("zalo_msg_template").cloned()
            .unwrap_or("Xin chào! Tôi muốn giới thiệu sản phẩm đến bạn.".to_string()),
        delay_min_secs: settings.get("zalo_delay_min").and_then(|v| v.parse().ok()).unwrap_or(60),
        delay_max_secs: settings.get("zalo_delay_max").and_then(|v| v.parse().ok()).unwrap_or(120),
        daily_limit: settings.get("zalo_daily_limit").and_then(|v| v.parse().ok()).unwrap_or(20),
        action,
    };

    tokio::spawn(async move {
        let _ = zalo::run_outreach(app, leads, config).await;
    });

    Ok(())
}

// ============================================================
// App Entry Point
// ============================================================

// ============================================================
// Email Commands
// ============================================================

#[tauri::command]
async fn test_email(
    config: mailer::SmtpConfig,
    to_email: String,
    subject: String,
    body: String,
) -> Result<String, String> {
    // Run blocking task in async context
    tokio::task::spawn_blocking(move || {
        mailer::send_email_sync(&config, &to_email, &subject, &body)?;
        Ok(format!("Gửi thử email tới {} thành công!", to_email))
    })
    .await
    .map_err(|e| format!("Task failed: {}", e))?
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let data_dir = match app.path().app_local_data_dir() {
                Ok(dir) => { std::fs::create_dir_all(&dir).ok(); dir }
                Err(_) => PathBuf::from("data"),
            };

            let db_path = data_dir.join("leads.db");
            let session_dir = data_dir.clone();

            if let Err(e) = Database::open(&db_path) {
                eprintln!("⚠ Database init error: {}", e);
            }

            app.manage(AppState {
                db_path: Mutex::new(db_path),
                session_dir: Mutex::new(session_dir),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Scraping
            start_scraping,
            export_excel,
            // Database
            get_history,
            get_history_by_region,
            get_db_stats,
            export_all_to_excel,
            clear_all_leads,
            clear_leads_by_region,
            // CRM
            get_crm_leads,
            get_crm_stats,
            update_lead_status,
            update_lead_note,
            // Settings
            get_settings,
            save_settings,
            // Zalo (zca-js sidecar)
            zalo_login,
            zalo_check_session,
            zalo_start_qr_login,
            zalo_start_outreach,
            // Email
            test_email,
            mailer::email_start_outreach,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
