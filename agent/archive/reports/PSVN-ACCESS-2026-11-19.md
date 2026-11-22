# PSVN-ACCESS — tài liệu quyền truy cập máy Windows (psvn) · 2026-11-19

Ghi theo lệnh của A0 (quyết định Windows `349af67`, §10): chủ sở hữu, fingerprint, máy được phép,
vị trí dòng public key, cách thu hồi, ngày xem xét lại.

> **Không chứa private key, không log SSH nhạy cảm, không nội dung đầy đủ của
> `administrators_authorized_keys`.**

## 1. Chủ sở hữu & mục đích

- **Chủ sở hữu key**: `ps` (máy dev Linux, user `ps`). Key chỉ dùng để **đọc/chạy kiểm thử**
  trên `psvn` từ `ps` — không dùng cho mục đích khác.
- **Mục đích**: Agent 1 (ps) thực thi các lệnh xác minh Windows (build/test/gate) trên máy thật
  `psvn` theo quy ước `bin/vis-remote`.
- **Được tạo lúc**: 2026-11-19, sau khi A0 duyệt việc ss/ssh từ ps.

## 2. Keypair

- Tên trên máy ps: `~/.ssh/psvn_ed25519` (**private**, quyền `600`, chỉ ở máy ps) và
  `~/.ssh/psvn_ed25519.pub`.
- Thuật toán: **ed25519**, 256 bit.
- Fingerprint (SHA256): **`SHA256:5mrepWzhBggpvxhk7cHsAn8AW77PSLdT/Gkvkrl9DaI`** (comment: `ps-host-to-psvn`).
- Public key bắt đầu bằng `ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAID87W7KuaYbKSxqYDllYQkMNnLcG3wOzPztjfHFKKjxs`.
- `~/.ssh/config` (ps): block `Host psvn` ghim `User ps` + `IdentityFile ~/.ssh/psvn_ed25519` +
  `IdentitiesOnly yes` + `ConnectTimeout 15` — chỉ ảnh hưởng host `psvn`.
- `known_hosts` (ps) đã có entry (TOFU, accept-new theo A0 cho phiên này):
  - `psvn` (ED25519, `SHA256:uT+MWtoqzG+HguN1T9OfaAvXwRySOTuQiD1e7dw5OpM`)
  - `psvn.tail17a72a.ts.net` (cùng fingerprint — cùng khóa, đã đối chiếu với known_hosts của mps)
  - Không có private key ở máy khác.

## 3. Máy được phép truy cập `psvn`

- **ps (10.100/…, tailscale 100.113.214.23)** — trực tiếp qua `ssh psvn`/`vis-remote psvn`.
- **mps** — đã có key riêng (`/Users/mps/.ssh/psvn_ed25519`) do mps tự quản lý từ trước đợt này
  (fingerprint ED25519 `mps@mps-to-psvn`, dòng thứ 4 của file public sau khi thêm); đợt này
  **không** đụng key của mps, chỉ dùng relay tạm thời để thực thi thao tác backup/append với sự
  cho phép của A0 (§10).
- **Không** máy nào khác được ghi thêm quyền trong đợt này.

## 4. Vị trí dòng public key trên psvn

- `ps` là **Administrator** trên `psvn` ⇒ Windows OpenSSH bỏ qua `%USERPROFILE%\.ssh\authorized_keys`
  và dùng **`C:\ProgramData\ssh\administrators_authorized_keys`**.
- Dòng mới được **append cuối file** (dòng 5 sau khi thêm, từ 4 lên 5):
  `ssh-ed25519 …ps-host-to-psvn` — không sửa, không xóa dòng nào có sẵn.
- ACL **giữ nguyên**: `NT AUTHORITY\SYSTEM:(F)` + `BUILTIN\Administrators:(F)`.
- Backup bản 4 dòng gốc (chỉ trong quá trình cài, **không commit**):
  `C:\Users\ps\vislogs\administrators_authorized_keys.bak-2026-11-19`.
- Lưu ý: tài khoản Windows `ps` cũng có thư mục `C:\Users\ps\.ssh\` chứa key `opencode_ps`/
  `opencode_mps` do toolchain trước đó của dự án — **không liên quan** tới đợt này; không đụng.

## 5. Cách thu hồi

- Xóa **đúng một dòng** kết thúc bằng `ps-host-to-psvn` trong
  `C:\ProgramData\ssh\administrators_authorized_keys` (giữ nguyên 4 dòng còn lại, giữ nguyên ACL).
- **Không** khôi phục toàn bộ file backup một cách mù — backup chỉ dùng làm đối chiếu chữ ký/diff,
  vì file thực có thể chứa thay đổi hợp lệ phát sinh sau 2026-11-19.
- Xóa `~/.ssh/psvn_ed25519{,.pub}` trên ps nếu muốn hủy hoàn toàn đường truy cập từ ps.
- Kiểm tra sau thu hồi: `ssh -o IdentitiesOnly=yes -i ~/.ssh/psvn_ed25519 ps@psvn true` phải **thất bại**.

## 6. Ngày xem xét lại quyền truy cập

- **Mốc xem xét**: `2026-12-19` (30 ngày sau khi cài) **hoặc ngay khi pha 1 đóng chính thức / VD
  "Windows test-suite portability" khởi động, mốc nào đến trước**.
- Tại mốc đó A0 quyết: giữ access hay thu hồi. Nếu pha 1 đóng và Windows tier 2 là mức cuối,
  ưu tiên thu hồi (§5) thay vì giữ key lâu dài.

## 7. Bối cảnh & giới hạn

- Access này được dùng **chỉ** cho verification tại `349af67` (pull origin/main, `cargo test
  --workspace --no-fail-fast`, `./vis-dev gate --fast`, `vis test test/sem` ×2, CRLF tests).
- `psvn` không bị thay đổi gì ngoài bước append dòng authorized_keys + đọc log + pull `git`.
- `vis-remote psvn` hoạt động bình thường với access này (kiểm: `git log --oneline -2` OK).
