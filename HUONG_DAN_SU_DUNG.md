# Hướng Dẫn Sử Dụng Ứng Dụng "Stepwell Scraper"

Chào mừng bạn đến với **Stepwell Scraper** — Công cụ trích xuất dữ liệu Google Maps và quản lý phễu tiếp thị Zalo tự động. Tài liệu này sẽ hướng dẫn chi tiết cách sử dụng từng tính năng trong ứng dụng.

---

## MỤC LỤC
1. [Quét Dữ Liệu Từ Google Maps](#1-quét-dữ-liệu-từ-google-maps)
2. [Quản Lý & Xuất Dữ Liệu (Lịch sử DB)](#2-quản-lý--xuất-dữ-liệu-lịch-sử-db)
3. [Đăng Nhập Zalo CRM](#3-đăng-nhập-zalo-crm)
4. [Thực Hiện Chiến Dịch Gửi Tin Nhắn / Kết Bạn Tự Động](#4-thực-hiện-chiến-dịch-gửi-tin-nhắn--kết-bạn-tự-động)

---

## 1. Quét Dữ Liệu Từ Google Maps

Phần này giúp bạn thu thập các doanh nghiệp / địa điểm cụ thể trên Google Maps và lưu thẳng vào hệ thống.

1. **Mở tab "Cào Google Maps":** Tại thanh điều hướng chính, nhấn vào mục đầu tiên.
2. **Nhập từ khóa cần tìm:** Nhập ngành nghề hoặc địa điểm (Ví dụ: `xưởng giày dép hồ chí minh`, `bệnh viện đa khoa đà nẵng`).
3. **Tuỳ chỉnh cấu hình:** 
   - **Khu vực (Region):** Đặt tên nhóm khu vực để sau này dễ quản lý (Ví dụ: `Hồ Chí Minh`).
   - **Độ sâu cuộn trang (Scroll Depth):** Nhập số lần ứng dụng sẽ kéo chuột xuống để tải thêm dữ liệu (Càng lớn càng tìm được nhiều nhưng sẽ mất thời gian hơn, mặc định `5`).
   - **Số luồng song song (Concurrency):** Xử lý đồng thời bao nhiêu trang con cùng lúc (Mặc định `5`).
4. **Bấm "🚀 Bắt Đầu Cào":** App sẽ tự động tiến hành trích xuất số điện thoại, tên, và địa chỉ.
5. **Theo dõi Log:** Các dòng thông báo màu xanh/đỏ sẽ cho bạn biết những SĐT nào được tìm thấy và lưu lại thành công.

---

## 2. Quản Lý & Xuất Dữ Liệu (Lịch sử DB)

Tại tab **"Lịch sử DB"**, bạn có thể xem lại những gì đã được thu thập ở bước 1.

1. **Xem danh sách:** Ứng dụng nhóm dữ liệu theo từng **Khu vực (Region)** mà bạn đã đặt tên.
2. **Xóa dữ liệu:** Có thể xóa toàn bộ dữ liệu của một khu vực hoặc "Xóa toàn bộ DB" nếu muốn làm sạch hệ thống.
3. **Xuất Excel:** 
   - Bấm **"Xuất Excel khu vực này"** để tải file Excel `.xlsx` gồm danh sách những SĐT đã được quét.
   - Bấm **"Xuất tất cả ra Excel"** ở góc phải để gom mọi dữ liệu có trong máy thành một file Excel duy nhất.

---

## 3. Đăng Nhập Zalo CRM

Tab **"Zalo CRM"** là trung tâm để giao tiếp tự động với khách hàng mà bạn đã tìm được. Lần đầu sử dụng, bạn cần phải đăng nhập. 

**Tính năng nổi bật:** Bạn chỉ cần đăng nhập 1 lần duy nhất, các lần tắt mở ứng dụng sau, phần mềm sẽ tự động vào mà không cần đăng nhập lại.

Có 2 cách để đăng nhập, vui lòng chọn cách phù hợp tại tab **"Kết nối Zalo"**:

### CÁCH A: Quét Mã QR (Khuyên Dùng - Nhanh Chóng)
1. Bấm qua thẻ **"📱 Quét mã QR"**.
2. Nhấn nút màu xanh **"📱 Tạo mã QR đăng nhập"**.
3. Chờ vài giây, một hình ảnh Mã QR sẽ hiện lên trên phần mềm.
4. Mở ứng dụng Zalo trên điện thoại cá nhân -> Chọn nút Quét Mã QR ở góc trên cùng bên phải.
5. Quét mã hiển thị trên màn hình máy tính -> Nhấn **"Đăng nhập"** trên điện thoại.
6. Màn hình phần mềm sẽ báo **"✅ Đã kết nối Zalo thành công"**.

### CÁCH B: Nhập Thông Tin (Dành Cho Căn Bản Trình Duyệt)
Nếu bạn không muốn Zalo điện thoại bị phiền, bạn có thể lấy cookie đang chạy ngầm trên trình duyệt Web (Chrome).
1. Bấm qua thẻ **"🔑 Đăng nhập Cookie"**.
2. Đăng nhập Zalo Web trên Chrome (`chat.zalo.me`).
3. Mở F12 (DevTools) -> Console.
4. Lấy IMEI (z_uuid) bằng cách gõ: `localStorage.getItem('z_uuid')`.
5. Lấy UserAgent bằng cách gõ: `navigator.userAgent`.
6. Sao chép Cookie bằng extension `J2TEAM Cookies`.
7. Dán 3 thông tin này vào 3 ô tương ứng trên phần mềm và bấm **"Đăng nhập Zalo"**.

---

## 4. Thực Hiện Chiến Dịch Gửi Tin Nhắn / Kết Bạn Tự Động

Sau khi cào dữ liệu và đăng nhập Zalo thành công, bạn đã sẵn sàng tiếp cận tự động.

### 4.1. Cài Đặt Nội Dung
1. Tại tab Zalo CRM, chuyển sang mục phụ **"⚙️ Cài đặt"**.
2. **Thiết lập Delay:** Ứng dụng cần nghỉ ngơi ngẫu nhiên giữa các lần nhắn tin để Zalo không đánh dấu bạn là spam. Nhập thông số nhỏ nhất (Ví dụ: 60 giây) và lớn nhất (Ví dụ: 120 giây).
3. **Nội dung tin nhắn:** Điền kịch bản bạn muốn gửi (Ví dụ: *"Dạ chào anh/chị, em đến từ nhà máy Stepwell..."*).
4. Nhấn **"Lưu cài đặt"**.

### 4.2. Khởi Chạy Chiến Dịch (Outreach)
1. Chuyển sang mục phụ **"📋 CRM Leads"**. Bạn sẽ thấy giao diện phễu khách hàng.
2. Tại bảng danh sách phía dưới, tích chọn vào **Ô vuông** ở các khách hàng (hoặc bấm nút "Chọn tất cả").
3. Nhấn vào nút thao tác phía trên danh sách:
   - **"🤝 Gửi kết bạn"**: Ứng dụng sẽ tìm số điện thoại này trên Zalo và gửi lời mời kết bạn kèm nội dung tin nhắn bạn vừa cài đặt. (Trạng thái chuyển thành *Đã gửi KBạn*).
   - **"💬 Gửi tin nhắn"**: Ứng dụng sẽ nhắn tin trực tiếp đến khách hàng (Trạng thái chuyển thành *Đã nhắn tin*).
4. **Theo dõi tiến độ:** Một bảng tóm tắt sẽ hiện ra phía trên báo hiệu ứng dụng đang tự động gửi, bạn vui lòng không đóng ứng dụng cho đến khi chạy xong (bạn có thể hạ phần mềm xuống thanh Taskbar và làm việc khác).

### 4.3. Quản Lý Trạng Thái (CRM Funnel)
- Nếu khách hàng đồng ý lời mời, bạn có thể chuyển trạng thái thủ công thành **"Khách hàng"** hoặc **"Đã phản hồi"** trong bảng.
- Bấm vào nút **"+ Ghi chú"** ở mỗi dòng để lưu lại các thông tin đàm phán quan trọng với từng đầu mối.
