use crate::database::Database;
use chromiumoxide::browser::{Browser, BrowserConfig};
use chromiumoxide::page::Page;
use futures::StreamExt;
use rust_xlsxwriter::{Color, Format, Workbook};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::{AppHandle, Emitter};
use tokio::time::{sleep, Duration};

// ============================================================
// Data Structures
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lead {
    pub stt: usize,
    pub name: String,
    pub phone: String,
    pub address: String,
    pub region: String,
    pub website: String,
    pub email: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrapeConfig {
    pub keyword: String,
    pub region: String,
    pub limit: usize,
    pub deduplicate: bool,
    pub deep_crawl: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrapeProgress {
    pub message: String,
    pub found: usize,
    pub skipped: usize,
    pub done: bool,
    pub error: Option<String>,
}

// ============================================================
// Public API (called from lib.rs)
// ============================================================

pub async fn start_scraping(
    app: AppHandle,
    config: ScrapeConfig,
    db_path: PathBuf,
) -> Result<Vec<Lead>, String> {
    run_scraper(app, config, db_path).await
}

pub async fn export_excel(leads: Vec<Lead>, file_path: String) -> Result<String, String> {
    generate_excel(&leads, &file_path).map_err(|e| e.to_string())?;
    Ok(format!("Xuất Excel thành công: {}", file_path))
}

// ============================================================
// Core Scraping Logic
// ============================================================

async fn run_scraper(app: AppHandle, config: ScrapeConfig, db_path: PathBuf) -> Result<Vec<Lead>, String> {
    emit_progress(&app, "Đang khởi động trình duyệt ẩn (Headless Chrome)...", 0, 0, false, None);

    let chrome_path = find_chrome_path();

    let browser_config = BrowserConfig::builder()
        .chrome_executable(chrome_path)
        .no_sandbox()
        .window_size(1280, 900)
        .build()
        .map_err(|e| format!("Không thể cấu hình trình duyệt: {}", e))?;

    let (mut browser, mut handler) = Browser::launch(browser_config)
        .await
        .map_err(|e| format!("Không thể khởi động Chrome: {}. Hãy đảm bảo Google Chrome đã được cài đặt.", e))?;

    // Spawn browser event handler
    tokio::spawn(async move {
        while let Some(_) = handler.next().await {}
    });

    let search_query = format!("{} tại {}", config.keyword, config.region);
    let maps_url = format!(
        "https://www.google.com/maps/search/{}",
        url_encode(&search_query)
    );

    emit_progress(&app, &format!("🔍 Tìm kiếm: {}", search_query), 0, 0, false, None);

    let page = browser
        .new_page(&maps_url)
        .await
        .map_err(|e| format!("Không thể mở trang: {}", e))?;

    sleep(Duration::from_secs(4)).await;
    emit_progress(&app, "Đang tải danh sách kết quả...", 0, 0, false, None);

    // Open database for global cross-session deduplication
    let db = Database::open(&db_path).map_err(|e| format!("Lỗi mở database: {}", e))?;

    let mut leads: Vec<Lead> = Vec::new();
    let mut skipped_count = 0;

    // Step 1: Collect all place URLs from the list first
    emit_progress(&app, "Đang thu thập danh sách địa điểm...", 0, 0, false, None);
    let mut all_urls: Vec<String> = Vec::new();
    let mut scroll_rounds = 0;
    let max_scroll_rounds = 20;

    loop {
        let urls = collect_place_urls(&page).await;
        let prev_count = all_urls.len();

        for u in urls {
            if !all_urls.contains(&u) {
                all_urls.push(u);
            }
        }

        emit_progress(
            &app,
            &format!("Tìm thấy {} địa điểm trong danh sách...", all_urls.len()),
            0, 0, false, None,
        );

        if all_urls.len() >= config.limit || scroll_rounds >= max_scroll_rounds {
            break;
        }

        scroll_results_panel(&page).await;
        sleep(Duration::from_secs(2)).await;
        scroll_rounds += 1;

        let new_count = collect_place_urls(&page).await.len();
        if new_count <= prev_count && scroll_rounds > 3 {
            emit_progress(&app, "Đã tải hết danh sách.", 0, 0, false, None);
            break;
        }
    }

    // Limit to requested amount
    all_urls.truncate(config.limit * 3); // collect more to account for dedup

    // Step 2: Visit each URL and extract details
    emit_progress(&app, &format!("Bắt đầu trích xuất chi tiết {} địa điểm...", all_urls.len()), 0, 0, false, None);

    for url in &all_urls {
        if leads.len() >= config.limit {
            break;
        }

        match navigate_and_extract(&page, url).await {
            Ok(lead_data) => {
                let normalized_phone = normalize_phone(&lead_data.phone);

                // Global dedup: check DB first (catches cross-session duplicates)
                // Then check in-session list (catches same phone found twice in this crawl)
                let is_dup_in_db = config.deduplicate
                    && !normalized_phone.is_empty()
                    && db.phone_exists(&normalized_phone);

                let is_dup_in_session = config.deduplicate
                    && !normalized_phone.is_empty()
                    && leads.iter().any(|l| normalize_phone(&l.phone) == normalized_phone);

                if is_dup_in_db || is_dup_in_session {
                    skipped_count += 1;
                    let source = if is_dup_in_db { "DB toàn hệ thống" } else { "phiên này" };
                    emit_progress(
                        &app,
                        &format!("⊘ Bỏ qua (trùng {} - SĐT: {}): {}", source, normalized_phone, lead_data.name),
                        leads.len(),
                        skipped_count,
                        false,
                        None,
                    );
                    continue;
                }

                let region = extract_region(&lead_data.address, &config.region);
                let stt = leads.len() + 1;

                let mut email = String::new();
                if config.deep_crawl && !lead_data.website.is_empty() {
                    emit_progress(
                        &app,
                        &format!("Bắt đầu quét Website tìm Email: {}...", lead_data.website),
                        leads.len(),
                        skipped_count,
                        false,
                        None,
                    );
                    email = extract_email_from_website(&lead_data.website).await;
                }

                // Save to SQLite immediately
                let _ = db.insert_lead(
                    &lead_data.name,
                    &normalized_phone,
                    &lead_data.address,
                    &region,
                    &config.keyword,
                    &lead_data.website,
                    &email,
                );

                emit_progress(
                    &app,
                    &format!("✓ [{}/{}] {} | {}", stt, config.limit, lead_data.name, lead_data.phone),
                    leads.len() + 1,
                    skipped_count,
                    false,
                    None,
                );

                leads.push(Lead {
                    stt,
                    name: lead_data.name,
                    phone: lead_data.phone,
                    address: lead_data.address,
                    region,
                    website: lead_data.website.clone(),
                    email,
                });

                sleep(Duration::from_millis(800)).await;
            }
            Err(e) => {
                emit_progress(
                    &app,
                    &format!("⚠ Bỏ qua 1 địa điểm: {}", e),
                    leads.len(),
                    skipped_count,
                    false,
                    None,
                );
            }
        }
    }

    let _ = browser.close().await;

    let total = leads.len();
    emit_progress(
        &app,
        &format!("🎉 Hoàn tất! Thu thập được {} khách hàng (bỏ qua {} trùng lặp).", total, skipped_count),
        total,
        skipped_count,
        true,
        None,
    );

    Ok(leads)
}

// ============================================================
// Page Interaction Helpers
// ============================================================

struct LeadData {
    name: String,
    phone: String,
    address: String,
    website: String,
}

/// Collect all place detail URLs from the current list view.
async fn collect_place_urls(page: &Page) -> Vec<String> {
    let script = r#"(function() {
        const anchors = document.querySelectorAll('[role="feed"] a[href*="/maps/place/"]');
        const urls = [];
        anchors.forEach(a => {
            const href = a.href;
            if (href && href.includes('/maps/place/') && !urls.includes(href)) {
                urls.push(href);
            }
        });
        return urls;
    })()"#;

    match page.evaluate(script).await {
        Ok(val) => val.into_value::<Vec<String>>().unwrap_or_default(),
        Err(_) => vec![],
    }
}

/// Navigate directly to a place URL and extract its details.
async fn navigate_and_extract(page: &Page, url: &str) -> Result<LeadData, String> {
    page.goto(url)
        .await
        .map_err(|e| e.to_string())?;

    // Wait for detail page to load - poll until h1 appears
    let mut waited = 0;
    loop {
        sleep(Duration::from_millis(500)).await;
        waited += 500;
        let has_h1 = page
            .evaluate("!!document.querySelector('h1')")
            .await
            .and_then(|v| v.into_value::<bool>().map_err(|e| e.into()))
            .unwrap_or(false);
        if has_h1 || waited >= 6000 {
            break;
        }
    }

    let extract_script = r#"(function() {
        // Business name
        const nameEl = document.querySelector('h1.DUwDvf')
            || document.querySelector('h1[class*="fontHeadlineLarge"]')
            || document.querySelector('h1');

        // Phone: try multiple selectors Google Maps uses
        const phoneSelectors = [
            '[data-tooltip="Copy phone number"]',
            '[aria-label*="Phone:"]',
            '[aria-label*="Điện thoại:"]',
            '[data-item-id^="phone"]',
            'button[data-tooltip*="phone"]'
        ];
        let phoneEl = null;
        for (const sel of phoneSelectors) {
            phoneEl = document.querySelector(sel);
            if (phoneEl) break;
        }

        // Address: try multiple selectors
        const addrSelectors = [
            '[data-item-id="address"]',
            '[aria-label*="Address:"]',
            '[aria-label*="Địa chỉ:"]',
            '[data-tooltip="Copy address"]',
            'button[data-tooltip*="address"]'
        ];
        let addrEl = null;
        for (const sel of addrSelectors) {
            addrEl = document.querySelector(sel);
            if (addrEl) break;
        }

        function cleanLabel(el) {
            if (!el) return '';
            const label = el.getAttribute('aria-label') || el.innerText || '';
            return label
                .replace(/^Phone:\s*/i, '')
                .replace(/^Điện thoại:\s*/i, '')
                .replace(/^Address:\s*/i, '')
                .replace(/^Địa chỉ:\s*/i, '')
                .trim();
        }

        // Website
        const webSelectors = [
            'a[data-item-id="authority"]',
            'a[data-tooltip*="Open website"]',
            'a[data-tooltip*="Mở trang web"]',
            'a[aria-label*="Website"]'
        ];
        let webEl = null;
        for (const sel of webSelectors) {
            webEl = document.querySelector(sel);
            if (webEl) break;
        }
        let website = '';
        if (webEl) {
            website = webEl.getAttribute('href') || '';
            if (website && website.includes('/url?q=')) {
                try {
                    let urlParams = new URLSearchParams(website.split('?')[1]);
                    website = urlParams.get('q') || website;
                } catch(e) {}
            }
        }

        return {
            name: nameEl ? nameEl.innerText.trim() : '',
            phone: cleanLabel(phoneEl),
            address: cleanLabel(addrEl),
            website: website
        };
    })()"#;

    let data = page
        .evaluate(extract_script)
        .await
        .map_err(|e| e.to_string())?
        .into_value::<serde_json::Value>()
        .map_err(|e| e.to_string())?;

    let name = data["name"].as_str().unwrap_or("").to_string();
    if name.is_empty() {
        return Err("Không tìm thấy tên địa điểm".to_string());
    }

    Ok(LeadData {
        name,
        phone: data["phone"].as_str().unwrap_or("").to_string(),
        address: data["address"].as_str().unwrap_or("").to_string(),
        website: data["website"].as_str().unwrap_or("").to_string(),
    })
}

async fn scroll_results_panel(page: &Page) {
    let script = r#"(function() {
        const feed = document.querySelector('[role="feed"]');
        if (feed) { feed.scrollTop += 3000; return; }
        const main = document.querySelector('[role="main"]');
        if (main) { main.scrollTop += 3000; }
    })()"#;
    let _ = page.evaluate(script).await;
}

// ============================================================
// Excel Export
// ============================================================

pub fn generate_excel(leads: &[Lead], file_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();
    worksheet.set_name("Danh sách khách hàng")?;

    let header_format = Format::new()
        .set_bold()
        .set_background_color(Color::RGB(0x1D4ED8))
        .set_font_color(Color::White)
        .set_font_size(12);

    let row_alt_format = Format::new()
        .set_background_color(Color::RGB(0xEFF6FF));

    let headers = ["STT", "Tên khách hàng", "Số điện thoại", "Địa chỉ", "Khu vực", "Website", "Email"];
    for (col, header) in headers.iter().enumerate() {
        worksheet.write_with_format(0, col as u16, *header, &header_format)?;
    }

    worksheet.set_column_width(0, 8)?;
    worksheet.set_column_width(1, 35)?;
    worksheet.set_column_width(2, 18)?;
    worksheet.set_column_width(3, 55)?;
    worksheet.set_column_width(4, 25)?;
    worksheet.set_column_width(5, 35)?;
    worksheet.set_column_width(6, 30)?;

    for (row_idx, lead) in leads.iter().enumerate() {
        let row = (row_idx + 1) as u32;
        if row_idx % 2 == 1 {
            worksheet.write_with_format(row, 0, lead.stt as f64, &row_alt_format)?;
            worksheet.write_with_format(row, 1, &lead.name, &row_alt_format)?;
            worksheet.write_with_format(row, 2, &lead.phone, &row_alt_format)?;
            worksheet.write_with_format(row, 3, &lead.address, &row_alt_format)?;
            worksheet.write_with_format(row, 4, &lead.region, &row_alt_format)?;
            worksheet.write_with_format(row, 5, &lead.website, &row_alt_format)?;
            worksheet.write_with_format(row, 6, &lead.email, &row_alt_format)?;
        } else {
            worksheet.write(row, 0, lead.stt as f64)?;
            worksheet.write(row, 1, &lead.name)?;
            worksheet.write(row, 2, &lead.phone)?;
            worksheet.write(row, 3, &lead.address)?;
            worksheet.write(row, 4, &lead.region)?;
            worksheet.write(row, 5, &lead.website)?;
            worksheet.write(row, 6, &lead.email)?;
        }
    }

    worksheet.set_freeze_panes(1, 0)?;
    workbook.save(file_path)?;
    Ok(())
}

// ============================================================
// Utility Functions
// ============================================================

fn emit_progress(
    app: &AppHandle,
    message: &str,
    found: usize,
    skipped: usize,
    done: bool,
    error: Option<String>,
) {
    let progress = ScrapeProgress {
        message: message.to_string(),
        found,
        skipped,
        done,
        error,
    };
    let _ = app.emit("scrape_progress", progress);
}

fn normalize_phone(phone: &str) -> String {
    phone.chars().filter(|c| c.is_ascii_digit()).collect()
}

fn extract_region(address: &str, fallback: &str) -> String {
    let parts: Vec<&str> = address.split(',').collect();
    if parts.len() >= 3 {
        let district = parts[parts.len() - 2].trim();
        let city = parts[parts.len() - 1].trim();
        format!("{}, {}", district, city)
    } else if parts.len() >= 2 {
        parts[parts.len() - 1].trim().to_string()
    } else {
        fallback.to_string()
    }
}

fn url_encode(s: &str) -> String {
    let mut encoded = String::new();
    for c in s.chars() {
        match c {
            ' ' => encoded.push('+'),
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' | '~' => encoded.push(c),
            _ => {
                for byte in c.to_string().as_bytes() {
                    encoded.push_str(&format!("%{:02X}", byte));
                }
            }
        }
    }
    encoded
}

fn find_chrome_path() -> String {
    let candidates = [
        r"C:\Program Files\Google\Chrome\Application\chrome.exe",
        r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe",
        "/usr/bin/google-chrome",
        "/usr/bin/chromium-browser",
        "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
    ];
    for path in &candidates {
        if std::path::Path::new(path).exists() {
            return path.to_string();
        }
    }
    "chrome".to_string()
}


async fn extract_email_from_website(url: &str) -> String {
    let mut target_url = url.to_string();
    if !target_url.starts_with("http") {
        target_url = format!("http://{}", target_url);
    }
    
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) Chrome/114.0.0.0 Safari/537.36")
        .build() {
            Ok(c) => c,
            Err(_) => return String::new(),
        };

    let response = match client.get(&target_url).send().await {
        Ok(res) => res,
        Err(_) => return String::new(),
    };

    let html = match response.text().await {
        Ok(text) => text,
        Err(_) => return String::new(),
    };

    let re = Regex::new(r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}").unwrap();
    
    for mat in re.find_iter(&html) {
        let email = mat.as_str().to_string();
        let lower = email.to_lowercase();
        if !lower.ends_with(".png") && !lower.ends_with(".jpg") && !lower.ends_with(".gif") && !lower.contains("sentry.io") {
            return email;
        }
    }
    String::new()
}

