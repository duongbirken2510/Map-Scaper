# Stepwell Scraper V1.1.1

Stepwell Scraper là một Desktop Application mạnh mẽ kết hợp **Khai thác dữ liệu Google Maps**, **Tiếp thị Zalo (Zalo CRM)** và **Gửi thư hàng loạt (Email Outreach)**. Ứng dụng tự động hóa quá trình tìm kiếm khách hàng tiềm năng B2B và thực hiện chiến dịch tiếp cận đa kênh.

---

## 🆕 Changelog

### V1.1.1 — Bản vá ổn định (17/05/2026)

#### 🔐 Bảo mật
- **Fix SQL Injection:** Toàn bộ câu lệnh lọc trong `get_crm_leads` chuyển sang Parameterized Queries (`params![]`)
- **Bật CSP (Content Security Policy):** Thay thế `null` bằng chính sách bảo mật cụ thể trong `tauri.conf.json`
- **XSS Sanitization:** Lọc HTML độc hại trong phần xem trước email

#### ⚡ Hiệu năng
- **Fix N+1 Query:** `get_crm_stats()` gộp từ 8 queries thành 1 câu `GROUP BY`
- **Thêm DB Index:** Cột `status`, `region`, `friend_sent_at` được đánh index tăng tốc lọc dữ liệu lớn

#### 🐛 Sửa lỗi nghiêm trọng
- **Chrome SingletonLock:** Mỗi phiên cào giờ dùng profile Chrome riêng biệt (unique `--user-data-dir`), không còn crash khi chạy liên tiếp
- **Zalo QR Đơ App:** Chuyển QR login sang background thread (fire-and-forget), UI không bị block trong khi chờ quét QR
- **Node.js không tìm thấy:** Bundle Node.js v20 LTS vào trong app → hoạt động trên mọi máy Mac không cần cài Node.js
- **`zca-js` không tìm thấy:** Dùng `esbuild` đóng gói toàn bộ sidecar thành `bundle.cjs` tự chứa → không phụ thuộc `node_modules`
- **Zalo Sidecar path sai:** Fix đường dẫn tìm sidecar trong macOS `.app` bundle (`Contents/Resources/` thay vì `Contents/MacOS/`)
- **Email dùng sai delay:** Email Outreach giờ dùng `email_delay_min/max` thay vì dùng chung với cài đặt của Zalo

#### 🎨 Giao diện & UX
- **Fix Stale Closure:** `loadStats` trong `App.tsx` bọc bằng `useCallback`, stats luôn cập nhật sau khi cào xong
- **Log Outreach:** Không còn reset log khi chạy action mới — log được nối tiếp có tiêu đề phân cách
- **Interface CRM đồng bộ:** Thêm `last_reply_at`, `converted_at`, `rejected` vào TypeScript interface khớp với Rust struct

#### ⚙️ Cài đặt
- Thêm `email_delay_min` / `email_delay_max` mặc định (30s–60s) trong Settings

---

### V1.1.0 — Deep Crawl & Email (15/05/2026)
- **Deep Crawling:** Tự động vào website khách hàng để truy quét Email liên hệ
- **Email Outreach (Cold Email):** Gửi email hàng loạt qua SMTP
- **Spintax Engine:** Trộn nội dung ngẫu nhiên tránh bộ lọc spam (`{Chào|Xin chào} [Tên]`)
- **Cơ chế UPSERT:** Cập nhật dữ liệu cũ mà không gây trùng lặp
- **Preview Email:** Xem trước nội dung email trước khi gửi

---

## 🌟 Tính năng chính

### 1. Google Maps Scraper
- Tìm kiếm và trích xuất: Tên, SĐT, Địa chỉ, Khu vực, Website, Email
- Lưu trực tiếp vào SQLite — lọc trùng toàn DB tự động
- Xuất Excel (`.xlsx`) theo khu vực hoặc toàn bộ
- **Tự dừng thông minh:** Dừng khi đủ số lượng, hết kết quả Maps, hoặc scroll tối đa 20 vòng

### 2. Zalo CRM & Automation
- Đăng nhập qua **QR Code** (không block UI) hoặc **Cookie thủ công**
- Gửi lời mời kết bạn & nhắn tin hàng loạt với delay ngẫu nhiên
- Phễu CRM: `Mới → Đã gửi kết bạn → Đã nhắn tin → Khách hàng`
- Lưu phiên đăng nhập — không cần login lại

### 3. Email Outreach
- Gửi email SMTP với Spintax và placeholder (`[Tên]`, `[Website]`)
- Cấu hình delay riêng cho email (tách biệt với Zalo)

---

## 🛠 Kiến trúc công nghệ

| Layer | Công nghệ |
|---|---|
| **Frontend** | React 19, TypeScript, TailwindCSS 4, Vite |
| **Backend Desktop** | Tauri 2 (Rust) — SQLite, file system, HTTP |
| **Zalo Engine** | Node.js Sidecar (`zca-js`) — giao tiếp qua stdin/stdout |
| **Database** | SQLite (bundled trong binary, không cần DLL) |

---

## 💻 Cài đặt & Phát triển

### Yêu cầu (Dev machine)
- [Node.js](https://nodejs.org/en/) v18+
- [Rust](https://www.rust-lang.org/tools/install) (stable)
- Google Chrome (cho scraper)

> **Máy người dùng cuối:** Không cần cài Node.js — đã bundle sẵn trong app.

### 1. Cài đặt Dependencies
```bash
npm install
cd src-tauri/zalo-sidecar && npm install && cd ../..
```

### 2. Chạy Dev Mode
```bash
npm run tauri dev
```

### 3. Build Production

**macOS (Apple Silicon):**
```bash
npm run tauri build
# Output: src-tauri/target/release/bundle/dmg/Stepwell Scraper_1.1.1_aarch64.dmg
```

**Linux (qua GitLab CI/CD):**  
Push code → GitLab pipeline tự build `.deb` + `.AppImage`

### 4. Cài đặt trên macOS (máy khác)
```bash
# Sau khi cài từ DMG, chạy lệnh bypass Gatekeeper:
xattr -cr "/Applications/Stepwell Scraper.app"
```

---

## 📁 Cấu trúc dự án

```
Map-Scaper-clone/
├── src/                        # React Frontend
│   ├── App.tsx                 # Layout chính, scraper controls
│   └── ZaloTab.tsx             # Zalo CRM, Email Outreach, Settings
├── src-tauri/
│   ├── src/
│   │   ├── lib.rs              # Tauri commands
│   │   ├── scraper.rs          # Google Maps crawler
│   │   ├── database.rs         # SQLite operations
│   │   ├── zalo.rs             # Zalo sidecar bridge
│   │   └── mailer.rs           # Email SMTP
│   ├── zalo-sidecar/
│   │   ├── index.mjs           # Node.js sidecar source
│   │   └── bundle.cjs          # Esbuild bundle (production)
│   ├── binaries/
│   │   └── node-aarch64-apple-darwin  # Bundled Node.js v20 LTS
│   └── tauri.conf.json
└── README.md
```

---

## 📖 Tài liệu liên quan
- Hướng dẫn sử dụng chi tiết: [HUONG_DAN_SU_DUNG.md](./HUONG_DAN_SU_DUNG.md)
