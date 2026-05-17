/// fb_scraper.rs — Facebook Pages Scraper for Stepwell Scraper V1.2
///
/// Features:
///   - Smart Browser Login: headed Chrome for manual login, headless for scraping
///   - Persistent profile: keeps session across app restarts
///   - Anti-detection: random delays, mobile Facebook, human-like behavior
///   - Shared `leads` table with Google Maps data (source = 'facebook')

use crate::database::Database;
use chromiumoxide::browser::{Browser, BrowserConfig};
use chromiumoxide::page::Page;
use futures::StreamExt;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter};
use tokio::time::{sleep, Duration};

// ============================================================
// Data Structures
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FbLoginStatus {
    pub logged_in: bool,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FbScrapeConfig {
    pub keyword: String,
    pub location: String,
    pub limit: usize,
    pub deduplicate: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FbScrapeProgress {
    pub message: String,
    pub found: usize,
    pub skipped: usize,
    pub done: bool,
    pub error: Option<String>,
}

// Internal struct for extracted page data
struct FbPageData {
    name: String,
    phone: String,
    email: String,
    address: String,
    website: String,
    category: String,
    follower_count: String,
    fb_url: String,
}

// ============================================================
// Anti-Detection Constants
// ============================================================

const FB_PAGE_DELAY_MIN: u64 = 4;
const FB_PAGE_DELAY_MAX: u64 = 10;
const FB_SCROLL_DELAY_MIN: u64 = 2;
const FB_SCROLL_DELAY_MAX: u64 = 4;
const FB_MAX_PER_SESSION: usize = 50;
const FB_LOGIN_TIMEOUT_SECS: u64 = 300; // 5 minutes

// ============================================================
// Chrome Configuration
// ============================================================

/// Get persistent profile directory for Facebook
/// Located at: ~/.stepwell-scraper/fb-profile/
pub fn get_fb_profile_dir() -> PathBuf {
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    let profile_dir = home.join(".stepwell-scraper").join("fb-profile");
    let _ = std::fs::create_dir_all(&profile_dir);
    profile_dir
}

fn find_chrome_path() -> String {
    let candidates = [
        "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
        r"C:\Program Files\Google\Chrome\Application\chrome.exe",
        r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe",
        "/usr/bin/google-chrome",
        "/usr/bin/google-chrome-stable",
        "/usr/bin/chromium-browser",
        "/usr/bin/chromium",
    ];
    for path in &candidates {
        if std::path::Path::new(path).exists() {
            return path.to_string();
        }
    }
    "chrome".to_string()
}

fn chrome_config(profile_dir: &Path, headed: bool) -> Result<BrowserConfig, String> {
    let mut builder = BrowserConfig::builder()
        .chrome_executable(find_chrome_path())
        .no_sandbox()
        .window_size(1280, 900)
        .user_data_dir(profile_dir);

    if headed {
        builder = builder.with_head();
    }

    builder
        .arg("--no-first-run")
        .arg("--disable-sync")
        .arg("--disable-background-networking")
        .arg("--lang=vi")
        .build()
        .map_err(|e| format!("Chrome config error: {}", e))
}

// ============================================================
// Helpers
// ============================================================

fn emit_fb_progress(app: &AppHandle, msg: &str, found: usize, skipped: usize, done: bool, error: Option<String>) {
    let _ = app.emit("fb_scrape_progress", FbScrapeProgress {
        message: msg.to_string(),
        found,
        skipped,
        done,
        error,
    });
}

fn random_delay(min: u64, max: u64) -> Duration {
    use std::time::SystemTime;
    let seed = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos() as u64;
    let range = max.saturating_sub(min).max(1);
    Duration::from_secs(min + (seed % range))
}

// ============================================================
// Public API
// ============================================================

/// Check if user is logged into Facebook using persistent profile
pub async fn check_fb_login() -> Result<FbLoginStatus, String> {
    let profile_dir = get_fb_profile_dir();
    let config = chrome_config(&profile_dir, false)?;

    let (mut browser, mut handler) = Browser::launch(config)
        .await
        .map_err(|e| format!("Không thể khởi động Chrome: {}", e))?;

    tokio::spawn(async move {
        while let Some(_) = handler.next().await {}
    });

    let page = browser
        .new_page("https://m.facebook.com")
        .await
        .map_err(|e| format!("Không thể mở trang: {}", e))?;

    sleep(Duration::from_secs(4)).await;

    let url = page.url().await.ok().flatten().unwrap_or_default();

    let status = if url.contains("/login") || url.contains("checkpoint") || url.contains("/recover") {
        FbLoginStatus {
            logged_in: false,
            name: String::new(),
        }
    } else {
        // Try to extract user name from mobile Facebook
        let name = page
            .evaluate(
                r#"
                (function() {
                    // Try multiple selectors for profile name on mobile FB
                    let el = document.querySelector('header a[href*="/profile"]');
                    if (el) return el.textContent.trim();
                    el = document.querySelector('[data-sigil="MTopBlueBarProfileImage"]');
                    if (el) return el.getAttribute('alt') || '';
                    // Fallback: look for profile link text
                    let links = document.querySelectorAll('a[href*="profile.php"], a[href*="/me"]');
                    for (let l of links) { if (l.textContent.trim()) return l.textContent.trim(); }
                    return 'Facebook User';
                })()
                "#,
            )
            .await
            .ok()
            .and_then(|v| v.into_value::<String>().ok())
            .unwrap_or_else(|| "Facebook User".to_string());

        FbLoginStatus {
            logged_in: true,
            name,
        }
    };

    let _ = browser.close().await;
    Ok(status)
}

/// Show a headed Chrome window for user to manually log into Facebook.
/// Emits 'fb_login_result' event when login succeeds or times out.
pub async fn show_fb_login_browser(app: &AppHandle) -> Result<(), String> {
    let profile_dir = get_fb_profile_dir();
    let config = chrome_config(&profile_dir, true)?; // HEADED: visible window

    let (mut browser, mut handler) = Browser::launch(config)
        .await
        .map_err(|e| format!("Không thể mở trình duyệt: {}", e))?;

    tokio::spawn(async move {
        while let Some(_) = handler.next().await {}
    });

    let page = browser
        .new_page("https://www.facebook.com/login")
        .await
        .map_err(|e| format!("Không thể mở trang đăng nhập: {}", e))?;

    let start = std::time::Instant::now();
    let timeout = Duration::from_secs(FB_LOGIN_TIMEOUT_SECS);

    loop {
        sleep(Duration::from_secs(2)).await;

        match page.url().await.ok().flatten() {
            Some(url) => {
                // If URL no longer contains /login, user has logged in
                if !url.contains("/login")
                    && !url.contains("checkpoint")
                    && !url.contains("/recover")
                    && (url.contains("facebook.com") || url.contains("fb.com"))
                {
                    // Login successful!
                    let name = page
                        .evaluate(
                            r#"document.querySelector('[aria-label="Your profile"]')?.textContent || 
                               document.querySelector('[data-pagelet="ProfileBrowser"]')?.textContent ||
                               'Facebook User'"#,
                        )
                        .await
                        .ok()
                        .and_then(|v| v.into_value::<String>().ok())
                        .unwrap_or_else(|| "Facebook User".to_string());

                    let _ = app.emit(
                        "fb_login_result",
                        FbLoginStatus {
                            logged_in: true,
                            name,
                        },
                    );

                    // Close browser — cookies are saved in persistent profile
                    let _ = browser.close().await;
                    return Ok(());
                }
            }
            None => {
                // Browser was closed by user
                let _ = app.emit(
                    "fb_login_result",
                    FbLoginStatus {
                        logged_in: false,
                        name: "Đã đóng trình duyệt".to_string(),
                    },
                );
                return Ok(());
            }
        }

        if start.elapsed() > timeout {
            let _ = app.emit(
                "fb_login_result",
                FbLoginStatus {
                    logged_in: false,
                    name: "Hết thời gian chờ (5 phút)".to_string(),
                },
            );
            let _ = browser.close().await;
            return Ok(());
        }
    }
}

/// Main Facebook Pages scraping function.
/// Uses persistent profile (already logged in) in headless mode.
pub async fn start_fb_scraping(
    app: AppHandle,
    config: FbScrapeConfig,
    db_path: PathBuf,
) -> Result<usize, String> {
    let profile_dir = get_fb_profile_dir();
    let limit = config.limit.min(FB_MAX_PER_SESSION);

    emit_fb_progress(&app, "🔄 Đang khởi động trình duyệt Facebook...", 0, 0, false, None);

    let browser_config = chrome_config(&profile_dir, false)?;
    let (mut browser, mut handler) = Browser::launch(browser_config)
        .await
        .map_err(|e| format!("Không thể khởi động Chrome: {}", e))?;

    tokio::spawn(async move {
        while let Some(_) = handler.next().await {}
    });

    // Step 1: Verify we're logged in
    emit_fb_progress(&app, "🔐 Kiểm tra đăng nhập Facebook...", 0, 0, false, None);

    let page = browser
        .new_page("https://m.facebook.com")
        .await
        .map_err(|e| format!("Không thể mở trang: {}", e))?;

    sleep(Duration::from_secs(3)).await;

    let url = page.url().await.ok().flatten().unwrap_or_default();
    if url.contains("/login") || url.contains("checkpoint") {
        let _ = browser.close().await;
        return Err("Phiên Facebook đã hết hạn. Vui lòng đăng nhập lại.".to_string());
    }

    emit_fb_progress(&app, "✅ Đã đăng nhập Facebook. Bắt đầu tìm kiếm...", 0, 0, false, None);

    // Step 2: Search for pages
    let search_query = if config.location.is_empty() {
        config.keyword.clone()
    } else {
        format!("{} {}", config.keyword, config.location)
    };

    let search_url = format!(
        "https://m.facebook.com/search/pages/?q={}&ref=content_filter&source=filter",
        url_encode(&search_query)
    );

    let _ = page.goto(&search_url).await;
    sleep(Duration::from_secs(4)).await;

    emit_fb_progress(
        &app,
        &format!("🔍 Tìm kiếm: \"{}\"", search_query),
        0, 0, false, None,
    );

    // Step 3: Scroll and collect page URLs
    let mut page_urls: Vec<String> = Vec::new();
    let max_scrolls = 15;

    for scroll in 0..max_scrolls {
        if page_urls.len() >= limit {
            break;
        }

        // Extract page links from search results
        let links: Vec<String> = page
            .evaluate(
                r#"
                (function() {
                    let results = [];
                    // Mobile FB search result links
                    let links = document.querySelectorAll('a[href*="/pages/"]');
                    links.forEach(a => {
                        let href = a.href;
                        if (href && !href.includes('/search/') && !href.includes('?')) {
                            results.push(href);
                        }
                    });
                    // Also try direct page links from search
                    document.querySelectorAll('a[href^="/"]').forEach(a => {
                        let href = a.href;
                        // Filter for page-like URLs (exclude common paths)
                        if (href && !href.includes('/search/') && !href.includes('/login')
                            && !href.includes('/settings') && !href.includes('/help')
                            && !href.includes('/hashtag') && !href.includes('/story')
                            && a.querySelector('img') && a.textContent.trim().length > 2) {
                            results.push(href);
                        }
                    });
                    return [...new Set(results)];
                })()
                "#,
            )
            .await
            .ok()
            .and_then(|v| v.into_value::<Vec<String>>().ok())
            .unwrap_or_default();

        for link in links {
            if !page_urls.contains(&link) && page_urls.len() < limit {
                page_urls.push(link);
            }
        }

        emit_fb_progress(
            &app,
            &format!("📜 Scroll {} — Tìm thấy {} pages", scroll + 1, page_urls.len()),
            0, 0, false, None,
        );

        // Scroll down
        let _ = page
            .evaluate("window.scrollTo(0, document.body.scrollHeight)")
            .await;

        sleep(random_delay(FB_SCROLL_DELAY_MIN, FB_SCROLL_DELAY_MAX)).await;

        // Check if no new results after scroll
        if scroll > 3 && page_urls.is_empty() {
            emit_fb_progress(
                &app,
                "⚠️ Không tìm thêm kết quả mới. Dừng scroll.",
                0, 0, false, None,
            );
            break;
        }
    }

    if page_urls.is_empty() {
        let _ = browser.close().await;
        return Err("Không tìm thấy Facebook Page nào. Thử từ khóa khác.".to_string());
    }

    emit_fb_progress(
        &app,
        &format!("📋 Tìm thấy {} pages. Bắt đầu trích xuất dữ liệu...", page_urls.len()),
        0, 0, false, None,
    );

    // Step 4: Visit each page and extract data
    let db = Database::open(&db_path).map_err(|e| format!("Lỗi mở database: {}", e))?;
    let mut found_count = 0usize;
    let mut skipped_count = 0usize;

    for (i, page_url) in page_urls.iter().enumerate() {
        // Dedup check
        if config.deduplicate && db.fb_url_exists(page_url) {
            skipped_count += 1;
            emit_fb_progress(
                &app,
                &format!("⏭ [{}/{}] Bỏ qua (đã có): {}", i + 1, page_urls.len(), page_url),
                found_count, skipped_count, false, None,
            );
            continue;
        }

        emit_fb_progress(
            &app,
            &format!("📘 [{}/{}] Đang trích xuất: {}", i + 1, page_urls.len(), page_url),
            found_count, skipped_count, false, None,
        );

        // Navigate to page's About section
        match extract_page_info(&page, page_url).await {
            Some(data) => {
                // Insert into database
                match db.insert_fb_lead(
                    &data.name,
                    &data.phone,
                    &data.address,
                    &config.location,
                    &config.keyword,
                    &data.website,
                    &data.email,
                    &data.fb_url,
                    &data.category,
                    &data.follower_count,
                ) {
                    Ok(true) => {
                        found_count += 1;
                        let info = format!(
                            "✅ [{}/{}] {} | SĐT: {} | Email: {}",
                            i + 1,
                            page_urls.len(),
                            data.name,
                            if data.phone.is_empty() { "—" } else { &data.phone },
                            if data.email.is_empty() { "—" } else { &data.email },
                        );
                        emit_fb_progress(&app, &info, found_count, skipped_count, false, None);
                    }
                    Ok(false) => {
                        skipped_count += 1;
                    }
                    Err(e) => {
                        emit_fb_progress(
                            &app,
                            &format!("⚠️ Lỗi lưu DB: {}", e),
                            found_count, skipped_count, false, None,
                        );
                    }
                }
            }
            None => {
                emit_fb_progress(
                    &app,
                    &format!("⚠️ [{}/{}] Không trích xuất được dữ liệu", i + 1, page_urls.len()),
                    found_count, skipped_count, false, None,
                );
            }
        }

        // Anti-detection delay
        if i < page_urls.len() - 1 {
            let delay = random_delay(FB_PAGE_DELAY_MIN, FB_PAGE_DELAY_MAX);
            emit_fb_progress(
                &app,
                &format!("⏳ Chờ {}s...", delay.as_secs()),
                found_count, skipped_count, false, None,
            );
            sleep(delay).await;
        }

        // Check if session expired mid-scrape
        let current_url = page.url().await.ok().flatten().unwrap_or_default();
        if current_url.contains("/login") {
            emit_fb_progress(
                &app,
                "❌ Phiên Facebook hết hạn giữa chừng!",
                found_count, skipped_count, true,
                Some("Phiên đăng nhập hết hạn. Vui lòng đăng nhập lại.".to_string()),
            );
            let _ = browser.close().await;
            return Ok(found_count);
        }
    }

    let _ = browser.close().await;

    emit_fb_progress(
        &app,
        &format!(
            "🎉 Hoàn tất! Thu thập được {} doanh nghiệp từ Facebook (bỏ qua {} trùng).",
            found_count, skipped_count
        ),
        found_count,
        skipped_count,
        true,
        None,
    );

    Ok(found_count)
}

// ============================================================
// Page Data Extraction (mobile Facebook)
// ============================================================

async fn extract_page_info(page: &Page, page_url: &str) -> Option<FbPageData> {
    // Navigate to the About section of the page
    let clean_url = page_url.trim_end_matches('/');
    let about_url = if clean_url.contains("/about") {
        clean_url.to_string()
    } else {
        format!("{}/about", clean_url)
    };

    // Use mobile version for simpler DOM
    let mobile_url = about_url
        .replace("www.facebook.com", "m.facebook.com")
        .replace("facebook.com", "m.facebook.com")
        .replace("m.m.facebook.com", "m.facebook.com"); // avoid double replace

    page.goto(&mobile_url).await.ok()?;
    sleep(Duration::from_secs(3)).await;

    let html = page.content().await.ok()?;

    // Extract page name
    let name = page
        .evaluate(
            r#"
            document.querySelector('h1')?.textContent?.trim() ||
            document.querySelector('[data-sigil="event-name"]')?.textContent?.trim() ||
            document.title.replace(' | Facebook', '').trim()
            "#,
        )
        .await
        .ok()
        .and_then(|v| v.into_value::<String>().ok())
        .unwrap_or_default();

    if name.is_empty() {
        return None;
    }

    // Extract phone numbers (Vietnamese format)
    let phone = extract_phone(&html);

    // Extract email
    let email = extract_email(&html);

    // Extract address
    let address = extract_near_label(&html, &["Địa chỉ", "Address", "Location", "Vị trí"]);

    // Extract website
    let website = extract_website(&html);

    // Extract category
    let category = page
        .evaluate(
            r#"
            (function() {
                // Category is usually shown below the page name
                let spans = document.querySelectorAll('span');
                for (let s of spans) {
                    let t = s.textContent.trim();
                    // Common FB page categories
                    if (t.match(/Store|Shop|Restaurant|Spa|Clinic|Service|Agency|Company|Brand|Cửa hàng|Nhà hàng|Dịch vụ/i)) {
                        return t;
                    }
                }
                return '';
            })()
            "#,
        )
        .await
        .ok()
        .and_then(|v| v.into_value::<String>().ok())
        .unwrap_or_default();

    // Extract follower count
    let follower_count = page
        .evaluate(
            r#"
            (function() {
                let text = document.body.innerText;
                let m = text.match(/(\d[\d,.]*[KkMm]?)\s*(người theo dõi|followers|likes|lượt thích)/i);
                return m ? m[1] : '';
            })()
            "#,
        )
        .await
        .ok()
        .and_then(|v| v.into_value::<String>().ok())
        .unwrap_or_default();

    // Clean fb_url (normalize to canonical form)
    let fb_url = clean_url
        .replace("m.facebook.com", "facebook.com")
        .replace("/about", "");

    Some(FbPageData {
        name,
        phone,
        email,
        address,
        website,
        category,
        follower_count,
        fb_url,
    })
}

// ============================================================
// Text Extraction Helpers
// ============================================================

/// Extract Vietnamese phone numbers from HTML
fn extract_phone(html: &str) -> String {
    let re = Regex::new(
        r"(?:(?:\+84|84|0)[\s.-]?(?:\d[\s.-]?){8,9})"
    ).unwrap();

    for mat in re.find_iter(html) {
        let phone = mat.as_str().replace(|c: char| !c.is_ascii_digit() && c != '+', "");
        // Validate: must be 10-12 digits
        let digits: String = phone.chars().filter(|c| c.is_ascii_digit()).collect();
        if digits.len() >= 9 && digits.len() <= 12 {
            // Normalize to 0xxx format
            if phone.starts_with("+84") {
                return format!("0{}", &digits[2..]);
            } else if phone.starts_with("84") && digits.len() > 9 {
                return format!("0{}", &digits[2..]);
            }
            return phone;
        }
    }
    String::new()
}

/// Extract email from HTML
fn extract_email(html: &str) -> String {
    let re = Regex::new(r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}").unwrap();
    for mat in re.find_iter(html) {
        let email = mat.as_str().to_string();
        let lower = email.to_lowercase();
        // Skip image files and Facebook internal emails
        if !lower.ends_with(".png")
            && !lower.ends_with(".jpg")
            && !lower.ends_with(".gif")
            && !lower.contains("facebook.com")
            && !lower.contains("fbcdn")
            && !lower.contains("sentry.io")
        {
            return email;
        }
    }
    String::new()
}

/// Extract website URL from HTML
fn extract_website(html: &str) -> String {
    let re = Regex::new(r#"https?://(?:www\.)?[a-zA-Z0-9][a-zA-Z0-9.-]+\.[a-zA-Z]{2,}(?:/[^\s"'<>]*)?"#).unwrap();
    for mat in re.find_iter(html) {
        let url = mat.as_str().to_string();
        let lower = url.to_lowercase();
        // Skip Facebook and social media URLs
        if !lower.contains("facebook.com")
            && !lower.contains("fb.com")
            && !lower.contains("fbcdn.net")
            && !lower.contains("instagram.com")
            && !lower.contains("google.com")
            && !lower.contains("youtube.com")
            && !lower.contains("twitter.com")
        {
            return url;
        }
    }
    String::new()
}

/// Extract text near a label (e.g., "Địa chỉ: 123 ABC Street")
fn extract_near_label(html: &str, labels: &[&str]) -> String {
    for label in labels {
        // Try: "Label: value" or "Label\nvalue"
        let pattern = format!(
            r"(?i){}[\s:：]+([^\n<]{{3,100}})",
            regex::escape(label)
        );
        if let Ok(re) = Regex::new(&pattern) {
            if let Some(m) = re.captures(html) {
                if let Some(value) = m.get(1) {
                    let text = value.as_str().trim().to_string();
                    if !text.is_empty() {
                        return html_decode(&text);
                    }
                }
            }
        }
    }
    String::new()
}

/// Basic HTML entity decode
fn html_decode(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ")
}

/// Simple URL encoding
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
