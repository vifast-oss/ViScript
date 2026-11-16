# Mức bảo đảm host của ViScript (ST-73, VD-107)

Tài liệu **ngắn** về cái gì **thật sự** được bảo đảm trên từng nền. Cập nhật: 2026-11-14.

## Bảng nền tảng ↔ bảo đảm

| Nền tảng | Cưỡng chế **runtime** (ST-72b) | Landlock (kernel) | Bằng chứng |
|---|---|---|---|
| macOS (máy phát triển) | **Có** — `FsGuard`/`NetGuard`, `VS0604`, `--grant`, đã đo 8 ca interp+VM | **Không có** (không hỗ trợ) | `vis run --sandbox-check` in `landlock: không hỗ trợ trên macos`, exit 3 |
| Linux | **Có** — cùng mã `vis-cli` (chưa đo trên máy Linux) | **Chỉ probe** — đọc `/sys/kernel/security/lsm`, **chưa** cưỡng chế bằng kernel | `cargo check --target x86_64-unknown-linux-gnu` xanh (biên dịch chéo, **chưa chạy**) |
| Windows / wasm | Chưa có host tương ứng | Không áp dụng | Chưa đo |

## Cách đọc `vis run --sandbox-check`

In ra **stdout**, **không** chạy chương trình:

```
landlock: không hỗ trợ trên macos     # hoặc: landlock: có/không (LSM: …) trên Linux
cưỡng chế runtime: bật|tắt            # bật khi `main` khai ≥1 `cap[...]` (ST-72b)
cap điều kiện: fs,net                 # loại quyền đang được cưỡng chế
```

Mã thoát: `0` = Linux có `landlock`; `3` = không có / nền khác; `4` = Linux không đọc được tệp LSM.

## Đa nền tảng — ma trận `cargo check --target` (ST-74)

**Chỉ CHECK biên dịch chéo**; **KHÔNG** chạy trên Windows/wasm (chưa có runner). Lệnh:
`viscript/bin/vis-targets` (in bảng, exit 1 nếu ô **kỳ vọng xanh** bị đỏ; kỳ vọng ghi trong script).

| Nhóm | `x86_64-pc-windows-msvc` | `wasm32-unknown-unknown` | Mức bằng chứng |
|---|---|---|---|
| 7 crate lõi (`vis-diag`, `vis-syntax`, `vis-types`, `vis-bytecode`, `vis-fmt`, `vis-interp`, `vis-vm`) | check xanh | check xanh | **check** |
| `vis-cli` | check xanh | check xanh **nhưng KHÔNG chạy được**: `std::process::Command`, `std::net`, `std::fs`, thread không có OS sau wasm32-unknown-unknown | **check** — không phải hỗ trợ runtime |
| `vis-host-linux` | — | — | `x86_64-unknown-linux-gnu` check (ST-73) |
| `vis-host-wasm` | — | check xanh (khung; `probe()` = “runtime kiểm, không có sandbox OS”) | **check** |
| `vis-host-*` chạy thật | **chưa có runner** | **chưa có runner** | không có |

Bảng đầy đủ (18 ô) + log: `agent/JOURNAL/st74-matrix.log`. **Cấm** viết “chạy trên Windows/wasm”;
đúng là “**check** biên dịch chéo xanh, **chưa chạy**”.

## Bộ hợp chuẩn host — chạy THẬT trên macOS (ST-75)

**54 ca**, mỗi ca chạy **cả** `vis run` và `vis run --vm --no-cache` (**120 lần chạy**), so **byte**
stdout + mã thoát (+ stderr chứa khi khai): **0 lệch**.

| Nhóm | Số ca | Ghi chú |
|---|---|---|
| FsHost (`std.fs`, `--fs-mock`) | 22 | đọc/ghi/exists/list, UTF-8/emoji/CRLF, tên có dấu cách/Unicode, 64 KiB, ca âm |
| Ngoai (chương trình ngoài) | 13 | echo/cat/grep/`complete`/mã thoát/`--`/nhiều stage; VM **không có host Ngoai** ⇒ 13 ca khai `vm:vs0604` (kỳ vọng `VS0604`, exit 1 — đúng thiết kế) |
| Shell/`ls`/`open` | 11 | bảng `ls`, `--all`, `--long`, `where`, `sort_by`, `to json`, `take/first/count/select` |
| Quyền `cap` | 5 | trong root, ngoài root `VS0604`, thiếu `(write)`, `--grant`, không khai cap |
| Determinism | 3 | mỗi ca chạy **3 lần** mỗi đường, cùng stdout |

Lệnh: `viscript/bin/vis-host-suite` (in bảng `ca → PASS/FAIL`, exit 1 nếu thiếu/đỏ). Log:
`agent/JOURNAL/st75-suite.log`. **Windows/Linux/wasm: CHƯA ĐO** — mới chỉ **check** biên dịch chéo
(ST-74); không có runner.

## Cấm ngôn từ

- **Sai**: “đã cưỡng chế bằng Landlock”, “Landlock đã bảo vệ chương trình”, “cách ly kernel đã bật”.
- **Đúng**: “**cưỡng chế ở tầng runtime** (ST-72b)”; “**probe** Landlock (ST-73) — chỉ đọc, chưa cưỡng chế”.
- `vis-host-linux` hiện là **khung probe**; nó **không** gọi syscall và **không** hạn chế quyền.

- Cấm nói "async song song", "chạy đồng thời", "rendezvous", "HTTPS", "sandbox kernel" khi mô tả VS2 (VD-122, VD-123, VD-120).
- Cấm nói "hỗ trợ Windows/wasm/Linux" khi chỉ có cargo check (VD-104, VD-121).

## Điều kiện để có Landlock thật (VD-107 (4))

Cần **một** trong hai, kèm runner Linux để **đo**:

1. Agent 0 duyệt crate `landlock` (MIT/Apache) vào allowlist; hoặc
2. Agent 0 duyệt `unsafe` **đóng** trong `vis-host-linux` + ca âm.

Hiện cả hai đều **chưa** được duyệt (R2 cấm `unsafe`, allowlist crate đóng).

## Thụt lề `.vish` — **có ngữ nghĩa ở cấp tệp** (ST-85, VD-109)

Đo ngày 2026-11-14 (log `agent/JOURNAL/st85-do.log`): trong **thân `cmd`**, 2 vs 4 dấu cách cho **cùng**
stdout/exit (interp = VM); TAB bị lexer từ chối (`VS0107`). Nhưng ở **cấp tệp**, thụt lề **đổi hành vi**:
tệp mà **mọi** dòng thụt lề ⇒ chương trình **không in gì**, exit 0; dòng cấp tệp thụt 1 dấu cách đứng
**đầu** bị bỏ (`topmix` chỉ in `y`); đứng **sau** lại được chạy (`topmix2` in `y` rồi `x`).

⇒ **Không** chuẩn hoá thụt lề trong `vis fmt` (giữ nguyên văn). `vis fmt --check` chỉ **cảnh báo**
(không đỏ) và `vis fmt --indent-report` **đếm** dòng lệch chuẩn `4 × độ sâu khối` (ước lượng theo dòng
khi thân `cmd` chỉ là lệnh). Tại thời điểm ST-85: **8 tệp / 10 dòng** trong `test/` lệch (không sửa).

## HTTP client (ST-104, ST-109) — bảo đảm và giới hạn

- **Chỉ** `http://`, **chỉ loopback** (`127.0.0.1`/`localhost`); **không** TLS, **không** HTTP/2.
- **Chunked**: ghép chunk (bỏ `;ext`), dừng ở chunk `0`; **chunk trailer bị bỏ qua**.
- **Redirect**: **chỉ GET**, tối đa **5** lần; theo `Location` tuyệt đối/tương đối, có `.`/`..`; query-only
  giữ path; fragment bị bỏ. `HEAD` **không** theo redirect (trả mã gốc + `Location`).
- **Không keep-alive**: mỗi request một kết nối (`Connection: close`); không cookie/auth tự động.
- **`Content-Length`** đọc đúng N byte; **timeout đọc 2 s/lần** (server im lặng ⇒ `Err` rõ).
- **`http.head`**: gửi `HEAD`, **không đọc thân** dù server gửi `Content-Length`/chunked; trả
  `{status, headers}` (headers khoá chữ thường).

### FsGuard: mức bảo đảm chống đổi tệp giữa kiểm và dùng (sau ST-124)

- **Đường đọc**: mở handle trước, so (dev, ino) trước và sau canonicalize, trả nội dung đọc từ handle. Bất biến: nội dung trả về là nội dung của chính tệp mà quyết định quyền đã áp.
- **Đường ghi: giới hạn đã biết.** FsGuard kiểm đường dẫn rồi mới ghi, vì mở trước để lấy handle là đã tạo hoặc cắt tệp. Trong khoảng giữa hai bước, nếu bên khác ghi được vào thư mục cha, đường dẫn có thể bị thay bằng symlink trỏ ra ngoài vùng được cấp. Sửa triệt để cần mở theo thư mục và không theo symlink (openat, O_NOFOLLOW), tức cần unsafe hoặc crate ngoài; chưa có trong VS2. Trên Linux khi chạy với --sandbox, Landlock chặn việc ghi ra ngoài vùng được cấp ở mức kernel; không có --sandbox thì giới hạn này còn nguyên.
- **Host không báo danh tính tệp** (--fs-mock, Windows): chỉ so danh tính trước và sau có khác nhau không. Chặn được "đổi rồi để nguyên", không chặn được "đổi rồi đổi lại".
- **Hardlink: giới hạn đã biết.** Hardlink nằm trong vùng được cấp nhưng trỏ tới cùng inode với tệp ngoài vùng thì đọc được, vì về mặt hệ tệp đó là cùng một tệp. Landlock cũng không chặn trường hợp này. Muốn tạo hardlink như vậy cần quyền ghi vào thư mục trong vùng và cùng hệ tệp với tệp đích.
- exists và list không trả nội dung tệp.
