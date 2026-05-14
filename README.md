# Stepwell Scraper (Tidu Map Scraper)

Stepwell Scraper là một hệ thống Desktop Application mạnh mẽ kết hợp giữa **Khai thác dữ liệu Google Maps (Scraping)** và **Quản lý khách hàng Zalo (Zalo CRM)**. Ứng dụng giúp tự động hóa quá trình tìm kiếm khách hàng tiềm năng B2B và thực hiện các chiến dịch tiếp cận tự động qua Zalo.

## 🚀 Tính năng chính

1. **Google Maps Scraper:**
   - Tìm kiếm và trích xuất dữ liệu địa điểm (Tên, SĐT, Địa chỉ, Khu vực) tự động.
   - Hỗ trợ lưu trữ trực tiếp vào CSDL SQLite (`leads.db`).
   - Xuất dữ liệu đa dạng theo từng khu vực hoặc toàn bộ danh sách ra định dạng Excel (`.xlsx`).

2. **Zalo CRM & Automation:**
   - **Đăng nhập Zalo linh hoạt:** Hỗ trợ đăng nhập qua quét mã QR (trực tiếp trên app) hoặc nhập Cookie thủ công (Dành cho tài khoản muốn giữ nguyên phiên làm việc trên trình duyệt).
   - **Bền vững:** Giữ phiên đăng nhập để các lần mở ứng dụng sau không cần phải thao tác lại.
   - **Chiến dịch tự động (Outreach):** Gửi lời mời kết bạn và nhắn tin hàng loạt tự động với thời gian chờ (delay) ngẫu nhiên để tránh bị khóa tài khoản.
   - **Phễu CRM:** Theo dõi trạng thái khách hàng (Mới -> Đã gửi kết bạn -> Đã nhắn tin -> Khách hàng).

## 🛠 Kiến trúc công nghệ

- **Frontend:** React, TypeScript, TailwindCSS 4, Vite.
- **Backend:** 
  - **Tauri (Rust):** Quản lý giao diện Desktop, Hệ thống File, Database (SQLite), HTTP Client scraping.
  - **Node.js Sidecar (`zca-js`):** Process chạy ngầm giao tiếp độc lập với Tauri qua stdin/stdout để thao tác với API không chính thức của Zalo một cách nhẹ nhàng và ổn định (thay thế hoàn toàn việc phải giả lập trình duyệt nặng nề).

## 💻 Hướng dẫn Cài đặt & Phát triển

### Yêu cầu môi trường:
- [Node.js](https://nodejs.org/en/) (khuyên dùng v18+)
- [Rust](https://www.rust-lang.org/tools/install) (cargo)

### 1. Cài đặt Dependencies
Mở terminal tại thư mục gốc của dự án:
```bash
npm install
cd src-tauri/zalo-sidecar
npm install
cd ../..
```

### 2. Chạy môi trường phát triển (Dev)
```bash
npm run tauri dev
```

### 3. Build Desktop App (Sản phẩm cuối)
Để đóng gói thành file `.exe` (NSIS Installer) cho Windows:
```bash
npm run tauri build -- --bundles nsis
```
File cài đặt sẽ nằm ở: `src-tauri/target/release/bundle/nsis/`

---

## 📖 Tài liệu liên quan
- Chi tiết hướng dẫn thao tác ứng dụng dành cho người dùng: Tham khảo file [HUONG_DAN_SU_DUNG.md](./HUONG_DAN_SU_DUNG.md)
