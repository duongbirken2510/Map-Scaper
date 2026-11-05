# 📖 Hướng Dẫn Sử Dụng Chi Tiết — StepWell Scraper

Chào mừng bạn đến với **StepWell Scraper** — Ứng dụng Desktop đa năng giúp **thu thập khách hàng tiềm năng (Leads)** tự động từ **Google Maps** & **Facebook Pages**, tích hợp **CRM quản lý phễu khách hàng**, **Zalo Outreach tự động** và **Trạm gửi Cold Email hàng loạt (Spintax)**.

---

## 📑 MỤC LỤC

1. [Tổng Quan & Yêu Cầu Hệ Thống](#1-tổng-quan--yêu-cầu-hệ-thống)
2. [Cài Đặt & Khởi Chạy Ứng Dụng](#2-cài-đặt--khởi-chạy-ứng-dụng)
3. [Bố Cục Giao Diện Chính](#3-bố-cục-giao-diện-chính)
4. [Quét Dữ Liệu Từ Google Maps](#4-quét-dữ-liệu-từ-google-maps)
5. [Quét Dữ Liệu Từ Facebook Pages](#5-quét-dữ-liệu-từ-facebook-pages)
6. [Quản Lý Kết Quả, Lịch Sử DB & Xuất Excel](#6-quản-lý-kết-quả-lịch-sử-db--xuất-excel)
7. [Kết Nối Tài Khoản Zalo CRM](#7-kết-nối-tài-khoản-zalo-crm)
8. [Quản Lý Phễu Khách Hàng (CRM Leads) & Chăm Sóc Tự Động](#8-quản-lý-phễu-khách-hàng-crm-leads--chăm-sóc-tự-động)
9. [Trạm Gửi Email Chào Hàng Hàng Loạt (Cold Email & Spintax)](#9-trạm-gửi-email-chào-hàng-hàng-loạt-cold-email--spintax)
10. [Cài Đặt An Toàn Cho Zalo Outreach](#10-cài-đặt-an-toàn-cho-zalo-outreach)
11. [Vị Trí Lưu Trữ Dữ Liệu Trên Máy Tính](#11-vị-trí-lưu-trữ-dữ-liệu-trên-máy-tính)
12. [Xử Lý Sự Cố Thường Gặp (Troubleshooting)](#12-xử-lý-sự-cố-thường-gặp-troubleshooting)

---

## 1. Tổng Quan & Yêu Cầu Hệ Thống

### ✨ Tính năng cốt lõi
- **Đa nguồn thu thập (Multi-source Scraping):** Cào dữ liệu từ **Google Maps** (kèm chế độ quét sâu vào Website để lấy Email) và **Facebook Pages**.
- **Chống trùng lặp toàn cục (Global Deduplication):** Tự động đối chiếu với cơ sở dữ liệu SQLite nội bộ để bỏ qua các SĐT / Fanpage đã từng thu thập ở các phiên trước.
- **Tích hợp CRM & Phễu bán hàng:** Theo dõi trạng thái từng khách hàng (`Mới` → `Đã gửi Email` / `Đã KBạn` / `Đã nhắn` → `Đã chấp nhận` → `Đã phản hồi` → `Khách hàng` / `Từ chối`) và ghi chú trực tiếp.
- **Tiếp cận đa kênh (Omnichannel Outreach):**
  - **Zalo:** Tự động tìm SĐT, gửi lời mời kết bạn hoặc nhắn tin trực tiếp với độ trễ (delay) ngẫu nhiên an toàn.
  - **Cold Email:** Gửi email chào hàng qua SMTP (Gmail/Outlook) hỗ trợ HTML, biến cá nhân hóa (`[Tên]`, `[Website]`) và trộn nội dung **Spintax** chống vào mục Spam.

### 💻 Yêu cầu hệ thống
1. **Trình duyệt Google Chrome:** Bắt buộc cài đặt sẵn trên máy tính (ứng dụng điều khiển nhân Chrome để trích xuất dữ liệu).
2. **Hệ điều hành:** macOS (Apple Silicon / Intel) hoặc Windows 10/11 (64-bit).
3. *(Dành cho lập trình viên chạy từ mã nguồn)*:
   - **Node.js** `>= 18`
   - **Rust & Cargo** (cài đặt qua `rustup.rs`)

---

## 2. Cài Đặt & Khởi Chạy Ứng Dụng

### Cách 1: Dành cho người dùng cuối (Bản cài đặt sẵn)
- **Trên macOS:** Mở file `.dmg`, kéo biểu tượng **Stepwell Scraper** vào thư mục `Applications` và mở ứng dụng từ Launchpad / thanh Dock.
- **Trên Windows:** Chạy file cài đặt `.exe` hoặc `.msi` và mở **Stepwell Scraper** từ màn hình Desktop.

### Cách 2: Dành cho nhà phát triển (Chạy từ mã nguồn)

```bash
# 1. Cài đặt thư viện giao diện (Frontend) và Zalo sidecar
npm install
cd src-tauri/zalo-sidecar && npm install && cd ../..

# 2. Khởi chạy ứng dụng ở chế độ phát triển (Development)
npm run tauri dev

# 3. Đóng gói thành file cài đặt hoàn chỉnh (.dmg / .app / .exe)
npm run tauri build
```

> **⚠️ Lưu ý quan trọng khi chạy `npm run tauri dev`:**
> - Giao diện phát triển chạy tại cổng mặc định `http://localhost:1422`.
> - Bạn **bắt buộc phải thao tác trên cửa sổ ứng dụng Desktop "Stepwell Scraper"** tự động bật lên, **KHÔNG** mở `http://localhost:1422` trên trình duyệt Chrome/Safari (vì trình duyệt web thông thường không có quyền truy cập vào lõi Rust và cơ sở dữ liệu của ứng dụng).

---

## 3. Bố Cục Giao Diện Chính

Giao diện **StepWell Scraper** được chia thành 3 khu vực trực quan:

1. **Thanh trạng thái trên cùng (Header):**
   - Hiển thị thống kê tổng quan thời gian thực: **DB tổng** (tổng số khách hàng đã lưu), **SĐT unique** (tổng số điện thoại duy nhất), **Phiên này** (số lượng tìm thấy trong lần quét hiện tại) và **Bỏ qua** (số lượng bị trùng hoặc không hợp lệ).
2. **Cột điều khiển bên trái (Sidebar — `CẤU HÌNH CÀO`):**
   - Nơi chọn nguồn dữ liệu (`📍 Google Maps` / `📘 Facebook`), nhập từ khóa, khu vực, giới hạn số lượng, bật/tắt lọc trùng, quét sâu Email, theo dõi thanh tiến trình và các nút bấm hành động (**▶ Bắt đầu cào**, **📊 Xuất phiên này**, **🗄 Xuất toàn bộ DB**).
3. **Khu vực làm việc chính bên phải (4 Tab):**
   - **📋 Logs:** Màn hình nhật ký thời gian thực hiển thị từng bước tìm kiếm và trích xuất.
   - **📊 Kết quả:** Bảng danh sách khách hàng vừa thu thập được trong phiên hiện tại.
   - **🗄 Lịch sử DB:** Kho dữ liệu tổng hợp toàn bộ khách hàng đã lưu trong máy tính qua mọi phiên làm việc.
   - **🤝 Zalo CRM:** Trung tâm kết nối Zalo, quản lý phễu khách hàng (CRM Leads), gửi Cold Email và cài đặt thông số tự động.

---

## 4. Quét Dữ Liệu Từ Google Maps

Tính năng này **không cần đăng nhập bất kỳ tài khoản nào**. Ứng dụng sẽ khởi chạy trình duyệt Chrome ẩn (Headless Chrome) để tự động tìm kiếm và bóc tách dữ liệu.

### Các bước thực hiện:
1. Tại cột **Cấu hình cào** bên trái, mục **📡 Nguồn dữ liệu**, chọn **📍 Google Maps**.
2. **🔍 Từ khóa:** Nhập ngành nghề, lĩnh vực hoặc loại hình kinh doanh cần tìm (Ví dụ: `giày dép`, `xưởng may đồng phục`, `nha khoa`, `spa làm đẹp`).
3. **📍 Khu vực:** Nhập tên tỉnh/thành phố hoặc quận/huyện (Ví dụ: `Hồ Chí Minh`, `Quận 7 Hồ Chí Minh`, `Đà Nẵng`).
   - *Mẹo:* Hệ thống sẽ tự động ghép thành truy vấn tìm kiếm `"{Từ khóa} tại {Khu vực}"` trên Google Maps và dùng chính tên Khu vực này để phân nhóm trong Database.
4. **🎯 Số lượng tối đa:** Nhập số lượng khách hàng mục tiêu muốn thu thập trong phiên (`1` – `500`, mặc định `50`).
5. **Tùy chọn nâng cao:**
   - **Lọc trùng (toàn DB):** *(Khuyên bật)* Tự động bỏ qua các địa điểm có số điện thoại đã tồn tại trong Database từ các lần quét trước.
   - **Quét sâu (Lấy Email) `CHẬM`:**
     - **Khi tắt:** Chỉ lấy nhanh Tên, SĐT, Địa chỉ, Website trực tiếp trên Google Maps (tốc độ rất nhanh).
     - **Khi bật:** Nếu địa điểm có Website, ứng dụng sẽ tự động truy cập vào trang chủ và trang Liên hệ của Website đó để dò tìm địa chỉ **Email** doanh nghiệp. *(Quá trình sẽ chậm hơn nhưng thu được dữ liệu Email chất lượng để chạy Cold Email).*
6. Nhấn nút màu xanh **▶ Bắt đầu cào**.
7. Theo dõi tiến trình trực tiếp tại tab **📋 Logs**. Khi hoàn tất, ứng dụng sẽ tự động chuyển sang tab **📊 Kết quả** và lưu toàn bộ dữ liệu vào Database.

---

## 5. Quét Dữ Liệu Từ Facebook Pages

Tính năng này giúp bạn tìm kiếm các Fanpage bán hàng / doanh nghiệp theo từ khóa và khu vực, sau đó tự động vào mục **Giới thiệu (About)** của từng Fanpage để lấy **Tên Page, SĐT, Email, Địa chỉ, Website, Danh mục và Số người theo dõi**.

### Bước 1: Đăng nhập Facebook (Chỉ cần làm 1 lần)
1. Tại mục **📡 Nguồn dữ liệu** bên trái, chuyển sang **📘 Facebook**.
2. Ứng dụng sẽ tự động kiểm tra trạng thái đăng nhập:
   - Nếu hiện `📘 Chưa đăng nhập Facebook`, hãy bấm nút **🔑 Đăng nhập Facebook**.
   - Một cửa sổ trình duyệt Google Chrome sẽ hiện lên mở trang đăng nhập Facebook.
   - Bạn đăng nhập tài khoản Facebook của mình như bình thường (hỗ trợ cả xác thực 2 yếu tố). Bạn có tối đa **5 phút** để hoàn tất đăng nhập.
   - Ngay khi đăng nhập thành công, cửa sổ Chrome sẽ tự đóng lại và ứng dụng báo: **✅ Đã kết nối: [Tên Facebook của bạn]**.
3. **Lưu phiên tự động:** Hồ sơ đăng nhập được lưu cố định tại `~/.stepwell-scraper/fb-profile/`, các lần mở app sau bạn không cần đăng nhập lại (trừ khi phiên hết hạn thì bấm **🔄 Đăng nhập lại**).

### Bước 2: Tiến hành cào Fanpage
1. Nhập **🔍 Từ khóa** (Ví dụ: `giày dép`, `thời trang nữ`) và **📍 Khu vực** (Ví dụ: `Hồ Chí Minh`).
2. Nhập **🎯 Số lượng tối đa** *(Lưu ý: Để bảo vệ tài khoản Facebook khỏi bị đánh dấu bất thường, hệ thống tự động giới hạn an toàn tối đa **50 Pages/phiên** kèm thời gian nghỉ ngẫu nhiên 4–10 giây giữa mỗi Page)*.
3. Bật **Lọc trùng (toàn DB)** để không quét lại các Fanpage đã có trong cơ sở dữ liệu.
4. Nhấn **▶ Bắt đầu cào** và theo dõi kết quả tại tab **📋 Logs** / **🗄 Lịch sử DB**.

---

## 6. Quản Lý Kết Quả, Lịch Sử DB & Xuất Excel

### 6.1. Xuất file Excel (`.xlsx`)
Ở góc dưới cùng cột bên trái có 2 nút xuất dữ liệu chuyên biệt:
- **📊 Xuất phiên này (`N`):** Chỉ xuất danh sách những khách hàng vừa quét được trong phiên hiện tại ra file Excel (tên gợi ý mặc định: `khachhang_{Từ khóa}_{Khu vực}.xlsx`).
- **🗄 Xuất toàn bộ DB (`N`):** Xuất toàn bộ khách hàng đang có trong cơ sở dữ liệu của máy ra một file Excel tổng hợp duy nhất (`toan_bo_khachhang.xlsx`).

### 6.2. Tra cứu & Lọc trong tab "🗄 Lịch sử DB"
1. Chuyển sang tab **🗄 Lịch sử DB** ở thanh điều hướng chính.
2. **Tìm kiếm theo khu vực:** Nhập tên khu vực vào ô *"Lọc theo khu vực (Enter để tìm)..."* và nhấn phím `Enter` hoặc nút **Tìm**. Để xem lại toàn bộ, xóa trắng ô tìm kiếm và nhấn **Tìm**.
3. **Làm sạch dữ liệu:** Nếu muốn xóa toàn bộ dữ liệu cũ để làm mới hệ thống, bấm nút **🗑 Xóa tất cả** ở góc phải thanh công cụ → Nhấn **Xóa** để xác nhận.

---

## 7. Kết Nối Tài Khoản Zalo CRM

Chuyển sang tab chính **🤝 Zalo CRM**, chọn mục phụ đầu tiên **🔗 Kết nối Zalo**.
Tại đây, phần mềm giao tiếp trực tiếp với Zalo mà không cần mở trình duyệt, đồng thời **tự động lưu phiên đăng nhập** cho các lần sử dụng sau.

Bạn có thể chọn 1 trong 2 phương thức đăng nhập:

### Cách A: 📱 Quét mã QR (Khuyên dùng — Nhanh & Đơn giản nhất)
1. Chọn thẻ **📱 Quét mã QR**.
2. Nhấn nút **📱 Tạo mã QR đăng nhập**.
3. Đợi vài giây để mã QR xuất hiện trên màn hình.
4. Mở ứng dụng **Zalo trên điện thoại** → Nhấn biểu tượng **Quét mã QR** ở góc trên bên phải → Quét mã trên màn hình máy tính và bấm **Đăng nhập**.
5. Giao diện sẽ chuyển sang trạng thái **✅ Đã kết nối Zalo thành công** kèm tên tài khoản của bạn ở góc phải trên cùng.

### Cách B: 🔑 Đăng nhập bằng Cookie (Dành cho người dùng nâng cao)
1. Chọn thẻ **🔑 Đăng nhập Cookie**.
2. Mở trình duyệt Chrome trên máy tính và đăng nhập vào [https://chat.zalo.me](https://chat.zalo.me).
3. Nhấn phím `F12` (hoặc chuột phải chọn *Inspect / Kiểm tra*) → Chuyển sang tab **Console**:
   - **Lấy IMEI (`z_uuid`):** Gõ lệnh `localStorage.getItem('z_uuid')` rồi nhấn `Enter`, sao chép chuỗi kết quả (bỏ dấu ngoặc kép) dán vào ô **IMEI (z_uuid)**.
   - **Lấy User Agent:** Gõ lệnh `navigator.userAgent` rồi nhấn `Enter`, sao chép chuỗi kết quả dán vào ô **User Agent**.
4. **Lấy Cookie (JSON):** Sử dụng tiện ích mở rộng **J2TEAM Cookies** hoặc **ZaloDataExtractor** trên Chrome để xuất Cookie của trang `chat.zalo.me` dưới dạng JSON và dán vào ô **Cookie (JSON)**.
5. Nhấn **🔐 Đăng nhập Zalo**.

---

## 8. Quản Lý Phễu Khách Hàng (CRM Leads) & Chăm Sóc Tự Động

Tại tab **🤝 Zalo CRM**, chuyển sang mục phụ thứ hai: **📋 CRM Leads**.

### 8.1. Theo dõi thống kê Phễu (Funnel)
Thanh thống kê phía trên tự động tổng hợp số lượng khách hàng theo từng giai đoạn:
- **Mới (`NEW`):** Khách hàng vừa cào về, chưa tương tác.
- **Đã gửi Email (`SENT_EMAIL`):** Đã gửi email chào hàng thành công.
- **Đã gửi KBạn (`SENT_FRIEND`):** Đã gửi lời mời kết bạn Zalo.
- **Đã nhắn tin (`SENT_MSG`):** Đã gửi tin nhắn Zalo đầu tiên.
- **Đã chấp nhận (`FRIEND_ACCEPTED`) / Đã phản hồi (`REPLIED`) / Khách hàng (`CUSTOMER`) / Từ chối (`REJECTED`):** Các trạng thái chuyển đổi trong quá trình chăm sóc.
- Góc phải hiển thị bộ đếm **Hôm nay đã gửi: `X`/ngày** để bạn kiểm soát hạn mức an toàn.

### 8.2. Lọc, Ghi chú & Cập nhật trạng thái từng khách hàng
- **Lọc theo trạng thái:** Sử dụng menu thả xuống ở góc trái (`Tất cả`, `Mới`, `Đã gửi Email`, `Đã KBạn`...) để lọc đúng tệp khách hàng cần xử lý.
- **Đổi trạng thái thủ công:** Nhấn trực tiếp vào nhãn trạng thái màu sắc ở cột **Trạng thái** của bất kỳ dòng nào để chuyển sang trạng thái mới (ví dụ: chuyển sang `Đã phản hồi` hoặc `Khách hàng`).
- **Thêm ghi chú đàm phán:** Nhấn vào chữ `+ Ghi chú` (hoặc nội dung ghi chú hiện tại) ở cột **Ghi chú** → Nhập nội dung → Nhấn nút **✓** màu xanh để lưu lại.

### 8.3. Chạy chiến dịch Kết bạn / Nhắn tin Zalo / Gửi Email hàng loạt
1. Tích chọn vào ô vuông ở đầu dòng các khách hàng muốn tiếp cận (hoặc bấm nút **Chọn tất cả** để chọn toàn bộ danh sách đang lọc).
2. Nhấn một trong 3 nút hành động ở góc phải thanh công cụ:
   - **📧 Gửi Email (`N`):** Gửi email chào hàng hàng loạt tới các khách hàng đã chọn (dựa trên cấu hình ở mục *Email Outreach*).
   - **🤝 Gửi kết bạn (`N`):** Tự động tìm tài khoản Zalo qua SĐT và gửi lời mời kết bạn kèm lời chào đã cài đặt.
   - **💬 Gửi tin nhắn (`N`):** Tự động gửi tin nhắn Zalo trực tiếp tới các SĐT đã chọn.
3. **Theo dõi hộp nhật ký Outreach:** Ngay bên dưới thanh công cụ sẽ xuất hiện khung log thời gian thực hiển thị kết quả từng khách hàng (`✓` thành công hoặc `⚠` lỗi/bỏ qua). Bạn có thể thu nhỏ cửa sổ ứng dụng xuống thanh Taskbar/Dock để làm việc khác trong lúc hệ thống tự động chạy ngầm.

---

## 9. Trạm Gửi Email Chào Hàng Hàng Loạt (Cold Email & Spintax)

Tại tab **🤝 Zalo CRM**, chuyển sang mục phụ thứ ba: **📧 Email Outreach**.

### 9.1. Cấu hình Máy chủ gửi thư (SMTP)
1. **Tài khoản Email (Gmail/Outlook):** Nhập địa chỉ email dùng để gửi thư (Ví dụ: `kinhdoanh@gmail.com`).
2. **Mật khẩu ứng dụng (App Password):** Nhập chuỗi mật khẩu ứng dụng gồm 16 ký tự do Google/Microsoft cấp *(Lưu ý: Tuyệt đối không nhập mật khẩu đăng nhập Gmail thông thường)*.
   - **Cách lấy Mật khẩu ứng dụng Gmail:**
     1. Truy cập phần Quản lý Tài khoản Google ([myaccount.google.com](https://myaccount.google.com)) → mục **Bảo mật (Security)**.
     2. Bật tính năng **Xác minh 2 bước (2-Step Verification)**.
     3. Trên thanh tìm kiếm của trang Quản lý tài khoản Google, gõ **"Mật khẩu ứng dụng" (App passwords)**.
     4. Tạo một mật khẩu ứng dụng mới, sao chép **16 chữ cái** hiện ra trên khung màu vàng và dán vào phần mềm.
3. Nhấn nút **Lưu cấu hình**.

### 9.2. Soạn thảo nội dung với Spintax & Biến cá nhân hóa
Để tránh bị bộ lọc của Gmail đánh dấu là thư rác (Spam) khi gửi số lượng lớn, ứng dụng hỗ trợ công nghệ **Spintax** giúp tạo ra hàng trăm biến thể bức thư khác nhau:
- **Cú pháp Spintax `{Lựa chọn 1|Lựa chọn 2|Lựa chọn 3}`:** Hệ thống sẽ chọn ngẫu nhiên 1 cụm từ trong dấu ngoặc nhọn cho mỗi email gửi đi.
  - *Ví dụ Tiêu đề:* `{Hợp tác kinh doanh|Thư ngỏ hợp tác|Chào hàng sỉ tận xưởng} - Kính gửi cửa hàng [Tên]`
- **Biến cá nhân hóa tự động:**
  - `[Tên]` → Tự động thay thế bằng Tên cửa hàng/doanh nghiệp của khách hàng.
  - `[Website]` → Tự động thay thế bằng địa chỉ Website của khách hàng.
- **Định dạng HTML:** Ô **Nội dung** hỗ trợ đầy đủ thẻ HTML (`<p>`, `<b>`, `<ul>`, `<li>`, `<a href="...">`) giúp thư chào hàng trình bày đẹp mắt, chuyên nghiệp.
- **Kiểm tra trước khi gửi:** Nhấn nút **🔄 Xáo trộn (Spin) thử** để xem các biến thể nội dung hiển thị ngay tại khung **👀 Xem trước (Preview)** bên dưới.
- Sau khi ưng ý, đừng quên nhấn **Lưu cấu hình** trước khi quay sang tab **📋 CRM Leads** để bấm **📧 Gửi Email**.

---

## 10. Cài Đặt An Toàn Cho Zalo Outreach

Tại tab **🤝 Zalo CRM**, chuyển sang mục phụ thứ tư: **⚙️ Cài đặt**.

1. **💬 Template tin nhắn đầu tiên:**
   - Nhập nội dung lời chào kết bạn hoặc tin nhắn đầu tiên gửi tới khách hàng.
   - Hỗ trợ biến `[Tên]` để tự động điền tên cửa hàng/khách hàng (Ví dụ: *"Dạ chào anh/chị chủ cửa hàng [Tên], bên em là xưởng sản xuất giày dép..."*).
2. **Cấu hình thời gian nghỉ & Giới hạn an toàn:**
   - **Delay tối thiểu (giây):** Thời gian nghỉ ngắn nhất giữa 2 lần gửi liên tiếp (Mặc định: `60` giây).
   - **Delay tối đa (giây):** Thời gian nghỉ dài nhất giữa 2 lần gửi liên tiếp (Mặc định: `120` giây). Ứng dụng sẽ chọn ngẫu nhiên một con số giữa Min và Max ở mỗi lượt gửi để mô phỏng thao tác người thật.
   - **Giới hạn kết bạn/ngày:** Số lượng tối đa trong 1 ngày (Khuyến nghị an toàn: `15` – `25` người/ngày).
3. Nhấn **💾 Lưu cài đặt** để áp dụng.

---

## 11. Vị Trí Lưu Trữ Dữ Liệu Trên Máy Tính

Mọi dữ liệu khách hàng, cài đặt và phiên đăng nhập đều được lưu cục bộ (Offline) ngay trên máy tính của bạn, đảm bảo tính bảo mật tuyệt đối:

- **Cơ sở dữ liệu SQLite (`leads.db`) & Phiên đăng nhập Zalo:**
  - **macOS:** `~/Library/Application Support/com.detau.map-scraper/`
  - **Windows:** `%LOCALAPPDATA%\com.detau.map-scraper\`
- **Hồ sơ trình duyệt Facebook (Persistent Chrome Profile):**
  - **macOS / Linux:** `~/.stepwell-scraper/fb-profile/`
  - **Windows:** `C:\Users\<Tên_User>\.stepwell-scraper\fb-profile\`

---

## 12. Xử Lý Sự Cố Thường Gặp (Troubleshooting)

| Triệu chứng / Thông báo lỗi | Nguyên nhân | Cách khắc phục |
| :--- | :--- | :--- |
| `TypeError: undefined is not an object (evaluating 'window.__TAURI_INTERNALS__.invoke')` | Bạn đang mở địa chỉ `http://localhost:1422` trên trình duyệt Chrome/Safari thay vì dùng cửa sổ ứng dụng Desktop. | Đóng tab trình duyệt web và chuyển sang sử dụng trực tiếp **cửa sổ ứng dụng Stepwell Scraper** trên máy tính. |
| `Error: Port 1420 / 1422 is already in use` khi chạy lệnh `tauri dev` | Cổng dev đang bị một ứng dụng khác chiếm dụng. | Đóng ứng dụng đang chiếm cổng hoặc đổi số cổng trong `vite.config.ts` và `src-tauri/tauri.conf.json`. |
| `Không thể khởi động Chrome... Hãy đảm bảo Google Chrome đã được cài đặt.` | Máy tính chưa cài Google Chrome hoặc cài ở thư mục tùy chỉnh. | Tải và cài đặt **Google Chrome** vào thư mục mặc định (`/Applications/Google Chrome.app` trên Mac hoặc `C:\Program Files\Google\Chrome` trên Windows). |
| Quét Facebook báo `Phiên đăng nhập hết hạn` hoặc không tìm thấy Page nào | Cookie Facebook đã hết hạn hoặc tài khoản bị yêu cầu xác minh lại. | Chọn nguồn **📘 Facebook** ở cột trái → Bấm **🔄 Đăng nhập lại** để mở cửa sổ Chrome và đăng nhập lại tài khoản. |
| Gửi Email báo lỗi xác thực SMTP (`535-5.7.8 Username and Password not accepted`) | Đang điền mật khẩu đăng nhập Gmail thông thường hoặc gõ sai App Password. | Bật Xác minh 2 bước trên Google, tạo **Mật khẩu ứng dụng (16 ký tự)** và dán lại vào mục **📧 Email Outreach** → **Lưu cấu hình**. |
| Gửi Zalo báo lỗi hoặc trạng thái chuyển sang `Chưa kết nối` | Tài khoản Zalo trên điện thoại vừa đăng xuất hoặc thu hồi phiên Web. | Vào mục **🔗 Kết nối Zalo** → Bấm nút làm mới `↻` hoặc quét lại mã QR để kết nối lại. |
