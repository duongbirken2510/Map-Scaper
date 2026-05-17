/// zalo.rs — Zalo CRM engine using zca-js Node.js sidecar
///
/// Architecture:
///   Tauri (Rust) ←→ Node.js child process (zalo-sidecar/index.mjs)
///   Communication: JSON lines over stdin/stdout
///
/// This replaces browser automation with direct Zalo API calls via zca-js

use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter};
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

// ============================================================
// Types
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZaloUser {
    pub name: String,
    pub avatar: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZaloStatus {
    pub logged_in: bool,
    pub user: Option<ZaloUser>,
    pub message: String,
    pub session_file: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutreachProgress {
    pub lead_id: i64,
    pub lead_name: String,
    pub phone: String,
    pub action: String,
    pub result: String,
    pub done: usize,
    pub total: usize,
    pub is_finished: bool,
}

// ============================================================
// Sidecar Process Manager
// ============================================================

pub struct ZaloSidecar {
    child: Child,
    stdin: std::process::ChildStdin,
    reader: BufReader<std::process::ChildStdout>,
    counter: u64,
}

impl ZaloSidecar {
    pub fn start(session_dir: &Path) -> Result<Self, String> {
        let sidecar_dir = find_sidecar_dir()?;

        // Use pre-bundled file if available (production), otherwise fall back to index.mjs (dev)
        let bundle_path = sidecar_dir.join("bundle.cjs");
        let index_path  = sidecar_dir.join("index.mjs");
        let script_path = if bundle_path.exists() {
            bundle_path
        } else if index_path.exists() {
            index_path
        } else {
            return Err(format!(
                "Không tìm thấy zalo-sidecar tại: {}",
                sidecar_dir.display()
            ));
        };

        // FIX: macOS .app bundles don't inherit shell PATH, so 'node' command fails.
        // We must resolve the full path to node binary explicitly.
        let node_bin = find_node_binary()?;

        let mut cmd = Command::new(&node_bin);
        cmd.arg(&script_path)
            .arg(session_dir.to_string_lossy().to_string())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        #[cfg(target_os = "windows")]
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW

        let mut child = cmd.spawn()
            .map_err(|e| {
                format!(
                    "Không thể khởi động Node.js ({}): {}. Hãy đảm bảo Node.js đã được cài đặt.",
                    node_bin, e
                )
            })?;

        let stdin = child.stdin.take().ok_or("Cannot get stdin")?;
        let stdout = child.stdout.take().ok_or("Cannot get stdout")?;
        let mut stderr = child.stderr.take().ok_or("Cannot get stderr")?;
        let mut reader = BufReader::new(stdout);

        // Wait for ready signal
        let mut ready_line = String::new();
        match reader.read_line(&mut ready_line) {
            Ok(0) => {
                // EOF reached, process exited. Read stderr to find out why.
                use std::io::Read;
                let mut err_msg = String::new();
                let _ = stderr.read_to_string(&mut err_msg);
                return Err(format!("Node.js sidecar crashed on startup: {}", err_msg));
            }
            Ok(_) => {
                if !ready_line.contains("_ready") {
                    return Err(format!("Sidecar invalid ready signal: {}", ready_line));
                }
            }
            Err(e) => return Err(format!("Sidecar failed to start: {}", e)),
        }

        Ok(Self {
            child,
            stdin,
            reader,
            counter: 0,
        })
    }

    pub fn send_command(&mut self, cmd: &serde_json::Value) -> Result<serde_json::Value, String> {
        self.counter += 1;
        let id = format!("cmd_{}", self.counter);

        let mut payload = cmd.clone();
        payload["id"] = serde_json::Value::String(id.clone());

        let line = serde_json::to_string(&payload).map_err(|e| e.to_string())?;
        writeln!(self.stdin, "{}", line).map_err(|e| format!("Write failed: {}", e))?;
        self.stdin
            .flush()
            .map_err(|e| format!("Flush failed: {}", e))?;

        // Read response lines until we find our id
        loop {
            let mut resp_line = String::new();
            match self.reader.read_line(&mut resp_line) {
                Ok(0) => return Err("Sidecar process terminated unexpectedly".to_string()),
                Ok(_) => {
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&resp_line) {
                        if val["id"].as_str() == Some(&id) {
                            return Ok(val);
                        }
                        // else: different id or broadcast, skip
                    }
                }
                Err(e) => return Err(format!("Read failed: {}", e)),
            }
        }
    }

    pub fn kill(&mut self) {
        let _ = self.child.kill();
    }
}

impl Drop for ZaloSidecar {
    fn drop(&mut self) {
        self.kill();
    }
}

fn find_sidecar_dir() -> Result<PathBuf, String> {
    // Check if a directory contains a valid sidecar (bundle.cjs takes priority over index.mjs)
    let is_valid_sidecar = |p: &PathBuf| p.join("bundle.cjs").exists() || p.join("index.mjs").exists();

    // ── Dev mode: check source tree paths first ─────────────────
    if let Ok(cwd) = std::env::current_dir() {
        let root_dev = cwd.join("src-tauri").join("zalo-sidecar");
        if is_valid_sidecar(&root_dev) { return Ok(root_dev); }
        let tauri_dev = cwd.join("zalo-sidecar");
        if is_valid_sidecar(&tauri_dev) { return Ok(tauri_dev); }
    }

    if let Ok(exe) = std::env::current_exe() {
        let exe_dir = exe.parent().unwrap_or(Path::new("."));

        // ── Dev mode: walk up ancestors to find src-tauri ───────
        let dev_candidate = exe_dir.ancestors().find_map(|p| {
            if p.file_name() == Some(std::ffi::OsStr::new("src-tauri")) {
                let c = p.join("zalo-sidecar");
                if is_valid_sidecar(&c) { return Some(c); }
            }
            None
        });
        if let Some(c) = dev_candidate { return Ok(c); }

        // ── Production macOS .app bundle ─────────────────────────
        // Executable: Contents/MacOS/map-scraper
        // Resources:  Contents/Resources/zalo-sidecar/   ← correct path
        if let Some(contents_dir) = exe_dir.parent() {
            let macos_resources = contents_dir.join("Resources").join("zalo-sidecar");
            if is_valid_sidecar(&macos_resources) { return Ok(macos_resources); }
        }

        // ── Production Linux/Windows: next to executable ─────────
        let candidate = exe_dir.join("zalo-sidecar");
        if is_valid_sidecar(&candidate) { return Ok(candidate); }
    }

    Err("Không tìm thấy thư mục zalo-sidecar. Hãy đảm bảo ứng dụng được cài đặt đúng cách.".to_string())
}

/// Find the Node.js binary path.
///
/// Priority order:
/// 1. Bundled binary shipped with app (via Tauri externalBin) — works on ANY machine
/// 2. Common system installation paths (dev mode convenience)
/// 3. nvm user directory
fn find_node_binary() -> Result<String, String> {
    // ── 1. Bundled binary (added via Tauri externalBin) ─────────────────────
    // Tauri places externalBin next to the main executable:
    //   macOS .app: Contents/MacOS/node-aarch64-apple-darwin
    //   Linux/Win:  <exe_dir>/node-<target_triple>
    // We also check for a plain "node" symlink which Tauri may create.
    if let Ok(exe) = std::env::current_exe() {
        let exe_dir = exe.parent().unwrap_or(std::path::Path::new("."));

        // Check with target-triple suffix (Tauri 2 externalBin naming)
        let target_triple = std::env::consts::ARCH.to_string() + "-" +
            if cfg!(target_os = "macos") { "apple-darwin" }
            else if cfg!(target_os = "linux") { "unknown-linux-gnu" }
            else { "pc-windows-msvc" };

        let variants = [
            exe_dir.join(format!("node-{}", target_triple)),
            exe_dir.join("node"),
        ];
        for candidate in &variants {
            if candidate.exists() {
                return Ok(candidate.to_string_lossy().to_string());
            }
        }
    }

    // ── 2. Common system installation paths ─────────────────────────────────
    let system_paths = [
        "/opt/homebrew/bin/node",        // Homebrew Apple Silicon
        "/usr/local/bin/node",           // Homebrew Intel / system
        "/usr/bin/node",                 // Linux system
        "/usr/bin/nodejs",               // Ubuntu/Debian
        "/opt/local/bin/node",           // MacPorts
        "/usr/local/nvm/versions/node/v20/bin/node",
        "/usr/local/nvm/versions/node/v18/bin/node",
        "/usr/local/nvm/versions/node/v22/bin/node",
        r"C:\Program Files\nodejs\node.exe",
        r"C:\Program Files (x86)\nodejs\node.exe",
    ];
    for path in &system_paths {
        if std::path::Path::new(path).exists() {
            return Ok(path.to_string());
        }
    }

    // ── 3. nvm / volta / fnm user-local installs ────────────────────────────
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"));
    if let Some(home) = home {
        let home = std::path::Path::new(&home);
        let search_dirs = [
            home.join(".nvm").join("versions").join("node"),
            home.join(".volta").join("tools").join("image").join("node"),
            home.join(".fnm").join("node-versions"),
        ];
        for base in &search_dirs {
            if base.exists() {
                if let Ok(entries) = std::fs::read_dir(base) {
                    let mut versions: Vec<std::path::PathBuf> = entries
                        .flatten()
                        .map(|e| e.path().join("bin").join("node"))
                        .filter(|p| p.exists())
                        .collect();
                    versions.sort();
                    if let Some(latest) = versions.last() {
                        return Ok(latest.to_string_lossy().to_string());
                    }
                }
            }
        }
        // Volta bin symlink
        let volta = home.join(".volta").join("bin").join("node");
        if volta.exists() { return Ok(volta.to_string_lossy().to_string()); }
    }

    // ── 4. PATH fallback (works in terminal/dev mode) ────────────────────────
    if std::process::Command::new("node").arg("--version").output().is_ok() {
        return Ok("node".to_string());
    }

    Err("Không tìm thấy Node.js. Ứng dụng cần Node.js để hoạt động.\n\
         Vui lòng cài đặt từ: https://nodejs.org/en/download (chọn LTS)\n\
         Sau đó khởi động lại ứng dụng.".to_string())
}

// ============================================================
// Global Sidecar State
// ============================================================

lazy_static::lazy_static! {
    static ref SIDECAR: Arc<Mutex<Option<ZaloSidecar>>> = Arc::new(Mutex::new(None));
}

fn with_sidecar<F, R>(session_dir: &Path, f: F) -> Result<R, String>
where
    F: FnOnce(&mut ZaloSidecar) -> Result<R, String>,
{
    let mut guard = SIDECAR.lock().map_err(|e| e.to_string())?;

    // Start sidecar if not running
    if guard.is_none() {
        *guard = Some(ZaloSidecar::start(session_dir)?);
    }

    let sidecar = guard.as_mut().unwrap();
    match f(sidecar) {
        Ok(r) => Ok(r),
        Err(e) => {
            // If sidecar crashed, clear it so next call restarts
            if e.contains("terminated") || e.contains("Write failed") {
                *guard = None;
            }
            Err(e)
        }
    }
}

// ============================================================
// Public API — Called from lib.rs Tauri commands
// ============================================================

/// Login with cookie + imei + userAgent
pub fn zalo_login(
    session_dir: &Path,
    cookie: &str,
    imei: &str,
    user_agent: &str,
) -> Result<ZaloStatus, String> {
    let cookie_val: serde_json::Value =
        serde_json::from_str(cookie).unwrap_or(serde_json::Value::String(cookie.to_string()));

    let resp = with_sidecar(session_dir, |sc| {
        sc.send_command(&serde_json::json!({
            "cmd": "login",
            "cookie": cookie_val,
            "imei": imei,
            "userAgent": user_agent,
        }))
    })?;

    if resp["ok"].as_bool() == Some(true) {
        Ok(ZaloStatus {
            logged_in: true,
            user: Some(ZaloUser {
                name: resp["name"]
                    .as_str()
                    .unwrap_or("Đã đăng nhập")
                    .to_string(),
                avatar: String::new(),
            }),
            message: "Đăng nhập Zalo thành công!".to_string(),
            session_file: session_dir.join("zalo_credentials.json").to_string_lossy().to_string(),
        })
    } else {
        Err(resp["error"]
            .as_str()
            .unwrap_or("Login failed")
            .to_string())
    }
}

/// Login with QR Code — sends QR image back via Tauri events
pub fn zalo_login_qr(
    app: &AppHandle,
    session_dir: &Path,
) -> Result<ZaloStatus, String> {
    let mut guard = SIDECAR.lock().map_err(|e| e.to_string())?;

    // Start sidecar if not running
    if guard.is_none() {
        *guard = Some(ZaloSidecar::start(session_dir)?);
    }

    let sidecar = guard.as_mut().unwrap();
    sidecar.counter += 1;
    let id = format!("cmd_{}", sidecar.counter);

    // Send login_qr command
    let payload = serde_json::json!({
        "id": id,
        "cmd": "login_qr",
    });
    let line = serde_json::to_string(&payload).map_err(|e| e.to_string())?;
    writeln!(sidecar.stdin, "{}", line).map_err(|e| format!("Write failed: {}", e))?;
    sidecar.stdin.flush().map_err(|e| format!("Flush failed: {}", e))?;

    // Read responses: we'll get intermediate QR messages and a final login result
    loop {
        let mut resp_line = String::new();
        match sidecar.reader.read_line(&mut resp_line) {
            Ok(0) => return Err("Sidecar process terminated unexpectedly".to_string()),
            Ok(_) => {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&resp_line) {
                    let resp_id = val["id"].as_str().unwrap_or("");

                    // QR image message
                    if resp_id == format!("{}_qr", id) {
                        if let Some(qr_image) = val["qrImage"].as_str() {
                            let _ = app.emit("zalo_qr", qr_image.to_string());
                        }
                        continue;
                    }

                    // Status message (generating QR)
                    if resp_id == format!("{}_status", id) {
                        continue;
                    }

                    // Final result
                    if resp_id == id {
                        if val["ok"].as_bool() == Some(true) && val["loggedIn"].as_bool() == Some(true) {
                            return Ok(ZaloStatus {
                                logged_in: true,
                                user: Some(ZaloUser {
                                    name: val["name"].as_str().unwrap_or("Đã đăng nhập").to_string(),
                                    avatar: String::new(),
                                }),
                                message: "Đăng nhập QR thành công!".to_string(),
                                session_file: session_dir.join("zalo_credentials.json").to_string_lossy().to_string(),
                            });
                        } else {
                            return Err(val["error"].as_str().unwrap_or("QR login failed").to_string());
                        }
                    }
                }
            }
            Err(e) => return Err(format!("Read failed: {}", e)),
        }
    }
}


/// Try to login with saved credentials
pub fn zalo_login_saved(session_dir: &Path) -> Result<ZaloStatus, String> {
    let creds_path = session_dir.join("zalo_credentials.json");
    if !creds_path.exists() {
        return Ok(ZaloStatus {
            logged_in: false,
            user: None,
            message: "Chưa có thông tin đăng nhập. Vui lòng nhập cookie/imei/userAgent.".to_string(),
            session_file: creds_path.to_string_lossy().to_string(),
        });
    }

    let resp = with_sidecar(session_dir, |sc| {
        sc.send_command(&serde_json::json!({ "cmd": "login_saved" }))
    })?;

    if resp["ok"].as_bool() == Some(true) && resp["loggedIn"].as_bool() == Some(true) {
        Ok(ZaloStatus {
            logged_in: true,
            user: Some(ZaloUser {
                name: resp["name"]
                    .as_str()
                    .unwrap_or("Đã đăng nhập")
                    .to_string(),
                avatar: String::new(),
            }),
            message: "Đã kết nối Zalo thành công.".to_string(),
            session_file: creds_path.to_string_lossy().to_string(),
        })
    } else {
        Ok(ZaloStatus {
            logged_in: false,
            user: None,
            message: resp["error"]
                .as_str()
                .unwrap_or("Phiên đăng nhập hết hạn")
                .to_string(),
            session_file: creds_path.to_string_lossy().to_string(),
        })
    }
}

/// Check session still valid
pub fn zalo_check(session_dir: &Path) -> Result<ZaloStatus, String> {
    let creds_path = session_dir.join("zalo_credentials.json");
    
    let resp = with_sidecar(session_dir, |sc| {
        sc.send_command(&serde_json::json!({ "cmd": "check" }))
    });

    match resp {
        Ok(r) if r["loggedIn"].as_bool() == Some(true) => Ok(ZaloStatus {
            logged_in: true,
            user: Some(ZaloUser {
                name: "Đã đăng nhập".to_string(),
                avatar: String::new(),
            }),
            message: "Phiên đăng nhập còn hiệu lực.".to_string(),
            session_file: creds_path.to_string_lossy().to_string(),
        }),
        _ => Ok(ZaloStatus {
            logged_in: false,
            user: None,
            message: "Chưa đăng nhập hoặc phiên hết hạn.".to_string(),
            session_file: creds_path.to_string_lossy().to_string(),
        }),
    }
}

/// Find user by phone number → returns uid
pub fn zalo_find_user(session_dir: &Path, phone: &str) -> Result<serde_json::Value, String> {
    with_sidecar(session_dir, |sc| {
        sc.send_command(&serde_json::json!({
            "cmd": "find_user",
            "phone": phone,
        }))
    })
}

/// Send friend request
pub fn zalo_send_friend(
    session_dir: &Path,
    uid: &str,
    message: &str,
) -> Result<serde_json::Value, String> {
    with_sidecar(session_dir, |sc| {
        sc.send_command(&serde_json::json!({
            "cmd": "send_friend",
            "uid": uid,
            "message": message,
        }))
    })
}

/// Send message
pub fn zalo_send_message(
    session_dir: &Path,
    uid: &str,
    message: &str,
) -> Result<serde_json::Value, String> {
    with_sidecar(session_dir, |sc| {
        sc.send_command(&serde_json::json!({
            "cmd": "send_message",
            "uid": uid,
            "message": message,
        }))
    })
}

// ============================================================
// Outreach Engine
// ============================================================

#[derive(Debug, Clone)]
pub struct OutreachConfig {
    pub db_path: PathBuf,
    pub session_dir: PathBuf,
    pub message_template: String,
    pub delay_min_secs: u64,
    pub delay_max_secs: u64,
    pub daily_limit: i64,
    pub action: String,
}

pub async fn run_outreach(
    app: AppHandle,
    leads: Vec<(i64, String, String)>,
    config: OutreachConfig,
) -> Result<(), String> {
    use crate::database::Database;

    let db = Database::open(&config.db_path).map_err(|e| e.to_string())?;

    if db.count_today_sent() >= config.daily_limit {
        let _ = app.emit(
            "zalo_outreach_progress",
            OutreachProgress {
                lead_id: 0,
                lead_name: String::new(),
                phone: String::new(),
                action: "RATE_LIMIT".into(),
                result: format!("Đã đạt giới hạn {}/ngày", config.daily_limit),
                done: 0,
                total: leads.len(),
                is_finished: true,
            },
        );
        return Ok(());
    }

    let total = leads.len();
    let mut done = 0;

    for (id, name, phone) in &leads {
        if db.count_today_sent() >= config.daily_limit {
            let _ = app.emit(
                "zalo_outreach_progress",
                OutreachProgress {
                    lead_id: *id,
                    lead_name: name.clone(),
                    phone: phone.clone(),
                    action: "RATE_LIMIT".into(),
                    result: format!("Đã đạt giới hạn {}/ngày", config.daily_limit),
                    done,
                    total,
                    is_finished: true,
                },
            );
            break;
        }

        let message = config.message_template.replace("[Tên]", name);
        let result = perform_action(&config.session_dir, phone, &message, &config.action);

        done += 1;

        match &result {
            Ok(msg) => {
                match config.action.as_str() {
                    "FRIEND" | "FRIEND_AND_MSG" => {
                        let _ = db.mark_friend_sent(*id);
                    }
                    "MESSAGE" => {
                        let _ = db.mark_msg_sent(*id);
                    }
                    _ => {}
                }
                let _ = db.log_activity(*id, &config.action, "SUCCESS", msg);
                let _ = app.emit(
                    "zalo_outreach_progress",
                    OutreachProgress {
                        lead_id: *id,
                        lead_name: name.clone(),
                        phone: phone.clone(),
                        action: config.action.clone(),
                        result: "SUCCESS".into(),
                        done,
                        total,
                        is_finished: done >= total,
                    },
                );
            }
            Err(e) => {
                let _ = db.log_activity(*id, &config.action, "FAILED", e);
                let _ = app.emit(
                    "zalo_outreach_progress",
                    OutreachProgress {
                        lead_id: *id,
                        lead_name: name.clone(),
                        phone: phone.clone(),
                        action: config.action.clone(),
                        result: format!("FAILED: {}", e),
                        done,
                        total,
                        is_finished: done >= total,
                    },
                );
            }
        }

        if done < total {
            let delay = random_delay(config.delay_min_secs, config.delay_max_secs);
            tokio::time::sleep(tokio::time::Duration::from_secs(delay)).await;
        }
    }

    Ok(())
}

fn perform_action(
    session_dir: &Path,
    phone: &str,
    message: &str,
    action: &str,
) -> Result<String, String> {
    // Step 1: Find user by phone
    let find_resp = zalo_find_user(session_dir, phone)?;

    if find_resp["ok"].as_bool() != Some(true) || find_resp["found"].as_bool() != Some(true) {
        return Err(format!(
            "Không tìm thấy tài khoản Zalo với SĐT {}",
            phone
        ));
    }

    let uid = find_resp["uid"]
        .as_str()
        .ok_or("Thiếu uid")?
        .to_string();
    let display_name = find_resp["displayName"]
        .as_str()
        .unwrap_or("")
        .to_string();

    // Step 2: Perform action
    match action {
        "FRIEND" => {
            let r = zalo_send_friend(session_dir, &uid, message)?;
            if r["ok"].as_bool() == Some(true) {
                Ok(format!("Đã gửi lời mời kết bạn cho {}", display_name))
            } else {
                Err(r["error"].as_str().unwrap_or("Lỗi gửi kết bạn").to_string())
            }
        }
        "MESSAGE" => {
            let r = zalo_send_message(session_dir, &uid, message)?;
            if r["ok"].as_bool() == Some(true) {
                Ok(format!("Đã gửi tin nhắn cho {}", display_name))
            } else {
                Err(r["error"]
                    .as_str()
                    .unwrap_or("Lỗi gửi tin nhắn")
                    .to_string())
            }
        }
        "FRIEND_AND_MSG" => {
            // Send friend request first, then message
            let fr = zalo_send_friend(session_dir, &uid, message);
            let mr = zalo_send_message(session_dir, &uid, message);
            match (fr, mr) {
                (Ok(_), Ok(_)) => Ok(format!("Đã kết bạn + nhắn tin cho {}", display_name)),
                (Ok(_), Err(e)) => Ok(format!("Đã kết bạn, nhưng tin nhắn lỗi: {}", e)),
                (Err(e), _) => Err(format!("Lỗi kết bạn: {}", e)),
            }
        }
        _ => Err("Unknown action".to_string()),
    }
}

fn random_delay(min: u64, max: u64) -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let t = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos() as u64;
    if max <= min {
        return min;
    }
    min + (t % (max - min))
}
