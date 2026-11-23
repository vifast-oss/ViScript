# Mức bảo đảm host của ViScript (ST-73, VD-107)

> **ĐÍNH CHÍNH macOS — 2026-11-21 (ST-242 / VD-242 Q7).** Số “macOS 27.0.1 arm64” cũ đã được **đo lại trên Darwin thật** ngày 2026-11-23:
> `mps` = macOS **27.0.1 arm64**, gate **3631/3**, workspace **3636/0/3**, host **54/54**,
> WASI suite **28/28**, `vis-wasm-sem` **388/390 = 99,5%** (xem `JOURNAL/mac-v045-verify.md`).
> **Vẫn chưa tuyên bố hỗ trợ macOS** (A0 quyết định tier); bằng chứng đang hiệu lực là **Linux (`ps`)**: host suite **54/54**,
> `vis-host-linux` **18/18** (ST-295 thêm 3 ca `Execute`/lib). Windows: **Tier 2**, verify lại trên `psvn` @ tag **`v0.3.5`** (`03bb7b9`):
> workspace **3528 passed / 0 failed / 3 ignored** (debug + release), P2 10/10, P3 10/10 — **vẫn chưa
> tuyên bố hỗ trợ**. Xem `STATE.md` addendum 20/11, `DECISIONS.md` VD-242, `agent/SNAPSHOT-2026-11-20.md` §6.

> **Cập nhật 2026-11-22 (ST-295 / VD-276)** — **đường ghi** trên Unix đã đóng khe kiểm-rồi-mở
> bằng `openat`/`openat2` (`rustix =1.1.6`, feature `fs`; ca đua **100/100 → 0/100**,
> `JOURNAL/st295-race.log`). Landlock `#125`: ruleset xử lý `Execute`, cấp chỉ-đọc + Execute cho
> thư viện chuẩn và binary trong grant — `vis-host-linux` **18/18**. Windows vẫn **Tier 2**
> (không có `openat`); macOS dùng cùng mã, đã verify `v0.4.5` trên Darwin 27 arm64 ngày 2026-11-23.

> **Cập nhật 2026-11-22 (ST-289, `v0.3.6`)** — phiên bản workspace là **một nguồn** (`Cargo.toml`
> `version = "0.3.6"`, crate dùng `version.workspace = true`); `vis --version` = `vis 0.3.6`.
> Tệp này **chỉ** sửa phần Windows đã có bằng chứng tag `v0.3.5`; các tuyên bố macOS/wasm giữ nguyên
> (macOS **không** có máy thật, wasm **chưa có runner**). **R32 chờ ST-284..288** trước khi chốt lại.

> **Windows Tier 1 — CHÍNH THỨC (2026-11-23, ST-298 / VD-283)** — đã đủ điều kiện; giữ caveat `FsGuard`.
> Bằng chứng đã có: **3× `bin/vis-gate-windows --full` PASS liên tiếp** @ `f4cb136`
> (**3611 passed / 0 failed / 3 ignored** mỗi lần; host 54/54; targets 18/18 + 17/1; dirty 0).
> Soak 4h cùng worktree đang chạy; số liệu do A0/W4 điền sau:
> **`985` vòng / fail `0` / flake `1`; fd/handle `free RAM 8204–8766 MB; proc/threads 0/0 sau mỗi vòng; fd/handle không đo được sau soak (NO_PROC; không còn tiến trình vis/cargo/rustc); dirty cuối 0`;
> free RAM `8204–8766 MB`; dirty cuối `0`; ETA `đã xong`**.
> Điều kiện đã đạt: soak 4h **0 lỗi sản phẩm**, RAM phẳng, 3× full xanh; A0 uỷ quyền tại `VD-283`.
> Tier 1 là mức hỗ trợ sản phẩm, **không** phải sandbox kernel; `FsGuard` chỉ kiểm path, không file identity.

Tài liệu **ngắn** về cái gì **thật sự** được bảo đảm trên từng nền. Cập nhật: 2026-11-20 ~02:30 — **lưu ý**: VD-winport đang triển khai; L0 Windows verify: @ `6ccf367` → 7 suite đỏ; @ `f9430b9` → **còn 1 ca** (`server_hai_ket_noi_lien_tiep` — đã fix @ `c7cef71`, verify nền); Windows **vẫn chưa được tuyên bố hỗ trợ**; Tier 2 giữ; `pha1-rc1` nguyên vẹn. Nội dung nền cũ giữ nguyên; chi tiết ở `agent/SNAPSHOT-2026-11-20.md`.
Cập nhật phụ: 2026-11-18 (giữ nguyên) — 2026-11-20 xem dòng đầu.
(ST-173: Landlock **chỉ Linux** — xem mục cuối tệp. **ST-295/VD-276**: **đường đọc và đường ghi**
đều đã đóng khe kiểm-rồi-mở trên Unix bằng `openat`/`openat2` (`#109b`/`#140`); macOS **chưa verify**
lại, **Windows mức bảo đảm thấp hơn**).

## Bảng nền tảng ↔ bảo đảm

| Nền tảng | Cưỡng chế **runtime** (ST-72b) | Landlock (kernel) | Bằng chứng |
|---|---|---|---|
| macOS (mps) — **ĐÃ verify `v0.4.5`** (chưa tuyên bố hỗ trợ) | **Có theo mã** — `FsGuard`/`NetGuard`, `VS0604`, `--grant` | **Không có** — nhân Darwin không có Landlock; suite `vis-host-linux` **cfg-out** (0 ca, ST-173) | **Darwin 27.0.1 arm64, 2026-11-23**: gate **3631/3**, workspace **3636/0/3**, host **54/54**, WASI **28/28**, `vis-wasm-sem` **388/390**; `JOURNAL/mac-v045-verify.md`. |
| Linux | **Có** — cùng mã `vis-cli` | **Có** — crate `landlock =0.4.7` cưỡng chế bằng kernel (ST-132); suite **18/18 ca** trên `ps` (ST-295 thêm 3 ca `Execute`/lib) | `cargo test -p vis-host-linux --test landlock` → `18 passed` (log `agent/JOURNAL/st295-verify.log`) |
| Windows / wasm | Chưa có host tương ứng | Không áp dụng | Chưa đo |

> **Windows (VD-186)**: `FsGuard` **có chạy** trên Windows nhưng **chỉ kiểm theo đường dẫn** và **không có
> danh tính tệp** ⇒ mức bảo đảm **thấp hơn** macOS/Linux; Windows trong VS2 **chỉ đo**, **chưa hỗ trợ**
> (VD-121/VD-146) — chi tiết ở mục **FsGuard**. wasm **chưa đo**.

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
| `vis-host-*` chạy thật | **đo, chưa hỗ trợ** (39/54; 15 ca `not ok` trùng danh sách ST-136 — thiếu chương trình POSIX, VS3) | **chưa có runner** (chỉ check) | Windows: **đo**, Linux: **chạy thật** (`vis-host-linux` 13/13 tại ST-137, **15/15** từ ST-150/ST-166/ST-173, **18/18** từ ST-295 — xem mục cuối; host suite 54/54) |

Bảng đầy đủ (18 ô) + log: `agent/JOURNAL/st74-matrix.log`. **Cấm** viết “chạy trên Windows/wasm”;
đúng là “**check** biên dịch chéo xanh, **chưa chạy**”.

## Bộ hợp chuẩn host — chạy THẬT trên Linux; macOS là số cũ (ST-75, ST-131/ST-142)

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
Log: `agent/JOURNAL/st75-suite.log` (nhãn macOS **cũ**, ST-75 — xem đính chính đầu tệp);
`st137-host-suite-macos.log` (nhãn macOS **cũ**), `st137-host-suite-ps.log`, `st142-*`,
`st144-host-suite.log` (Windows). **Linux: đã đo** (54/54, ST-142 — bằng chứng hiện hành).
**Windows: đã đo, chỉ đo** (39/54, VD-146/VD-121 — chưa hỗ trợ; workspace xanh @ `c7cef71` là cổng
`cargo test`, **không** phải tuyên bố hỗ trợ). **wasm: chưa đo** (chỉ check, VD-121).

## Cấm ngôn từ

- **Sai**: “đã cưỡng chế bằng Landlock” khi nói về **macOS/Windows**; “Landlock đã bảo vệ chương trình” trên nền không phải Linux; “cách ly kernel đã bật” chung chung.
- **Đúng**: “**cưỡng chế ở tầng runtime** (ST-72b)” trên **mọi** nền; “**Landlock cưỡng chế chỉ trên Linux** — đo **18/18** ca (ST-132/ST-150/ST-166/ST-173/ST-295)”.
- `vis-host-linux` **chỉ biên dịch trên Linux**: `src/lib.rs` và `tests/landlock.rs` có `#![cfg(target_os = "linux")]`, `landlock` nằm trong `[target.'cfg(target_os = "linux")'.dependencies]` (ST-173). Trên mac/Windows đây là crate **rỗng, 0 ca** — **giới hạn nền**, không phải thiếu sót; xem mục cuối tệp.

- Cấm nói "async song song", "chạy đồng thời", "rendezvous", "HTTPS", "sandbox kernel" khi mô tả VS2 (VD-122, VD-123, VD-120).
- Cấm nói "hỗ trợ Windows/wasm/Linux" khi chỉ có cargo check (VD-104, VD-121).
- **Sai (ST-295/VD-276)**: “macOS đã an toàn” (macOS **chưa verify** lại); “Windows được hỗ trợ” /
  “Windows có sandbox kernel”; “hardlink là lỗi đua”; “đường ghi còn kiểm-rồi-mở trên Unix” (đã đóng
  từ ST-295 bằng `openat`/`openat2`).
- **Đúng (ST-295, sửa 22/11)**: “**đường đọc + đường ghi** đã đóng khe symlink trên **Unix/Linux**
  (`#146`/`#140`/`#109b`; ST-174 + ST-295; macOS **chưa verify** lại — xem đính chính đầu tệp)”;
  “**hardlink** là **giới hạn chính sách**”; “**Windows** mức bảo đảm **thấp hơn**, chỉ **đo**”.

## Điều kiện để có Landlock thật (VD-107 (4))

Cần **một** trong hai, kèm runner Linux để **đo**:

1. Agent 0 duyệt crate `landlock` (MIT/Apache) vào allowlist; hoặc
2. Agent 0 duyệt `unsafe` **đóng** trong `vis-host-linux` + ca âm.

Hiện cả hai đều **chưa** được duyệt (R2 cấm `unsafe`, allowlist crate đóng).

**Cập nhật 2026-11-17 (ST-132/ST-166/ST-173):** điều kiện **1 đã đạt** — VD-120 duyệt crate
`landlock =0.4.7` (MIT OR Apache-2.0) **chỉ** trong crate `vis-host-linux`; suite kernel **18/18**
trên `ps`, nợ **#132 đóng** (ST-166). Điều kiện 2 (`unsafe`) **không** dùng.

## Thụt lề `.vish` — **có ngữ nghĩa ở cấp tệp** (ST-85, VD-109)

Đo ngày 2026-11-14 (log `agent/JOURNAL/st85-do.log`): trong **thân `cmd`**, 2 vs 4 dấu cách cho **cùng**
stdout/exit (interp = VM); TAB bị lexer từ chối (`VS0107`). Nhưng ở **cấp tệp**, thụt lề **đổi hành vi**:
tệp mà **mọi** dòng thụt lề ⇒ chương trình **không in gì**, exit 0; dòng cấp tệp thụt 1 dấu cách đứng
**đầu** bị bỏ (`topmix` chỉ in `y`); đứng **sau** lại được chạy (`topmix2` in `y` rồi `x`).

⇒ **Không** chuẩn hoá thụt lề trong `vis fmt` (giữ nguyên văn). `vis fmt --check` chỉ **cảnh báo**
(không đỏ) và `vis fmt --indent-report` **đếm** dòng lệch chuẩn `4 × độ sâu khối` (ước lượng theo dòng
khi thân `cmd` chỉ là lệnh). Tại thời điểm ST-85: **8 tệp / 10 dòng** trong `test/` lệch (không sửa).

## HTTP client (ST-104, ST-109, VS3-4a) — bảo đảm và giới hạn

- `http://` **chỉ loopback** (`localhost`/`127.0.0.0/8`/`::1`); `https://` (VS3-4a) cho host bất kỳ, xác
  thực chứng chỉ theo bundle gốc Mozilla (`webpki-roots`), vẫn qua `cap[Net]`. Mặc định HTTP/1.1;
  HTTP/2 chỉ khi bật feature `http2` (xem dưới).
- **HTTP/2 (VS3-4b2, feature `http2` default-off)**: TLS + ALPN `h2`; server không chọn h2 ⇒
  fallback HTTP/1.1 **trên cùng kết nối**. Một request/kết nối (không multiplex/push/pool h2),
  body cap 8 MiB, timeout chung; `cap[Net]` vẫn kiểm trước khi mở TLS.
- **Phân giải loopback**: `localhost` thử **mọi** địa chỉ phân giải (ưu tiên IPv4, luôn có fallback
  `127.0.0.1`) trong cùng deadline; chỉ địa chỉ `is_loopback()` được nối. `[::1]`/`127.0.0.x` được
  phép; host ngoài loopback bị từ chối trước khi nối.
- **TLS**: `rustls` + `ring`; lỗi chứng chỉ/handshake là `Err` dữ liệu có chữ `TLS`, không panic;
  handshake nằm trong **cùng deadline** với connect/redirect/đọc. Không cho hạ cấp `HTTPS → HTTP`.
  Root store **release** là `webpki-roots`; bản **debug** có hook `VIS_TLS_TEST_ROOT_DER` (một cert
  DER) **chỉ cho test hermetic** — release không biên dịch hook.
- **Chunked**: ghép chunk (bỏ `;ext`), dừng ở chunk `0`; **chunk trailer bị bỏ qua**.
- **Redirect**: **chỉ GET**, tối đa **5** lần; theo `Location` tuyệt đối/tương đối, có `.`/`..`; query-only
  giữ path; fragment bị bỏ. `HEAD` **không** theo redirect (trả mã gốc + `Location`).
- **Keep-alive (VS3-4b)**: request HTTP/1.1 gửi `Connection: keep-alive`; response HTTP/1.1 không
  `close` (hoặc HTTP/1.0 + `keep-alive`) và thân đã đọc hết ⇒ kết nối về **pool** theo
  `scheme|host:port`; tối đa **8** idle, hết hạn sau **15 s**; server `close`/EOF ⇒ đóng. Host không
  có pool (mặc định trait) vẫn one-shot. Kết nối idle chết được **thử lại một lần** (GET/HEAD).
- **Không keep-alive qua process**: pool sống trong một lần chạy `vis`; không ghi ra đĩa.
- **Cookie (VS3-4b)**: jar **nội bộ** theo `Net`, không ra đĩa/không API ngôn ngữ; parse
  `Set-Cookie` (`name=value`, `Path`, `Domain`, `Secure`, `HttpOnly`, `Max-Age`, `Expires` IMF-fixdate);
  gửi `Cookie` theo domain/path, **Secure chỉ qua https**, không gửi cross-host; cookie set ở hop
  redirect dùng được ở hop sau cùng host. Không SameSite/public-suffix/prefix/persistent store.
- **Basic auth (VS3-4b)**: URL `user:pass@host` ⇒ `Authorization: Basic …`; credential **không** vào
  log/`Host`/thông điệp lỗi; **không** tự chuyển Authorization sang redirect khác host (redirect
  tương đối cùng host giữ userinfo). Không có API header/challenge 401 tự động.
- **`Content-Length`** đọc đúng N byte; **timeout đọc 2 s/lần** (server im lặng ⇒ `Err` rõ).
- **`http.head`**: gửi `HEAD`, **không đọc thân** dù server gửi `Content-Length`/chunked; trả
  `{status, headers}` (headers khoá chữ thường).

### FsGuard: mức bảo đảm chống đổi tệp giữa kiểm và dùng (sau ST-124)

**Trạng thái hiện tại (VD-186, 2026-11-18)** — đọc bảng này trước khi trích dẫn mục này:

| Đường | Trạng thái trong **mã đang chạy** | Nguồn |
|---|---|---|
| **Đọc** | **Đã đóng** khe symlink trên **Unix/Linux** — nợ `#146` đóng ở ST-174; với `cap[Dir]`/`cap[File]`, ST-295 mở **từ fd gốc** (`openat`/`openat2`) (macOS **chưa verify** lại) | VD-176 §1 · VD-276 §1 · `JOURNAL/st174-*.log`, `st295-*.log` |
| **Ghi** | **Đã đóng** trên **Unix/Linux**: `openat`/`openat2(RESOLVE_BENEATH|RESOLVE_NO_SYMLINKS)` + `O_NOFOLLOW`; ca đua **100/100 → 0/100** (macOS **chưa verify** lại) — nợ `#140`/`#109b` đóng ở ST-295 | VD-276 §1 · `JOURNAL/st295-*.log` |
| **Hardlink** | **Giới hạn chính sách**, **không** phải lỗi đua — mục riêng bên dưới | VD-176 §3 |
| **Windows** | **Mức bảo đảm thấp hơn**: chỉ kiểm theo **đường dẫn**, không có danh tính tệp | VD-146 · VD-178 QĐ-4 |


- **Đường đọc — nợ `#146` ĐÃ ĐÓNG (ST-174, VD-176 §1): không còn đua symlink trên macOS và Linux.** Mở handle trước, quyết định quyền trên đường dẫn thật của chính handle, rồi đọc nội dung từ chính handle đó. Bất biến: nội dung trả về là nội dung của chính tệp mà quyết định quyền đã áp.
  - **Linux**: đường dẫn thật lấy qua /proc/self/fd/<fd>. /proc không đọc được thì từ chối, không quay về kiểm theo tên. Bảo đảm theo cơ chế (ST-137: 0/50 000).
  - **macOS (số cũ, CHƯA verify lại): cửa sổ symlink của đường đọc ĐÃ ĐÓNG (ST-174, VD-176 §1).** Std an toàn không có cách lấy đường dẫn của handle, nên vẫn giữ bước xác nhận `(dev, ino)`; từ ST-174 bước kiểm dùng `std::fs::symlink_metadata` (**không đi theo symlink**) thay cho `std::fs::metadata`. Vì `canonicalize` chỉ trả **tên** và bước kiểm chạy sau đó, mục tên bị đổi thành symlink trong khe giữa hai syscall nay bị **từ chối** (`VS0604`, "bị thay đổi trong khi mở"): `(dev, ino)` của **symlink** không thể trùng `(dev, ino)` của handle đã mở. Bằng chứng **tất định** (không dựa số vòng): ca đỏ-trước 100/100 trên mã cũ, sau vá 100/100 chặn trên mac **và** `ps`, 6 triệu vòng ngẫu nhiên trên mac 0 đỏ — `agent/JOURNAL/st174-*.log`. Giới hạn **còn lại** của đường đọc: **hardlink** (mục dưới). Cách triệt để (mở từng thành phần bằng `openat` + `O_NOFOLLOW`, bỏ hẳn kiểm theo đường dẫn) **đã làm ở ST-295** cho đường đọc/ghi có `cap` trên Unix (xem mục Đường ghi).
- **Đường ghi — nợ `#140`/`#109b` ĐÃ ĐÓNG (ST-295, VD-276 §1): không còn kiểm-rồi-mở trên Unix/Linux.** `FsGuard` mở **từ fd gốc** bằng `openat` từng thành phần với `O_NOFOLLOW`/`O_DIRECTORY`; Linux ≥ 5.6 dùng `openat2(RESOLVE_BENEATH|RESOLVE_NO_SYMLINKS)`; kernel cũ (`ENOSYS`) rơi về đi từng thành phần. Áp cho `write`/`append`/`write_bytes`/`mkdir`/`mkdir_all`/`remove`/`copy`/`rename`, đọc `read`/`read_bytes` cũng đi cùng hàm. Ca đua **tất định** (điểm chèn chỉ test giữa kiểm và mở): nhánh cũ **100/100** rò ra ngoài, nhánh mới **0/100** — `JOURNAL/st295-baove.md`, `quyen_st295.rs`. **Giới hạn còn lại**: Windows không có `openat` (chỉ kiểm theo đường dẫn); macOS dùng cùng mã `openat` nhưng **chưa verify** lại; **hardlink** vẫn là giới hạn chính sách (mục dưới).
- **Host không báo danh tính tệp** (--fs-mock, Windows): chỉ so danh tính trước và sau có khác nhau không. Chặn được "đổi rồi để nguyên", không chặn được "đổi rồi đổi lại".
- **Hardlink: giới hạn đã biết.** Hardlink nằm trong vùng được cấp nhưng trỏ tới cùng inode với tệp ngoài vùng thì đọc được, vì về mặt hệ tệp đó là cùng một tệp. Landlock cũng không chặn trường hợp này. Muốn tạo hardlink như vậy cần quyền ghi vào thư mục trong vùng và cùng hệ tệp với tệp đích.
- exists và list không trả nội dung tệp.

### Hardlink của tệp ngoài vào trong root — **giới hạn chính sách**, KHÔNG phải lỗi đua (ST-174, VD-176 §3)

**Kết luận một dòng:** một **hardlink** trỏ tới tệp **ngoài** root nhưng **nằm trong** root được **coi
là nằm trong root** — đọc được; bản vá ST-174 **không** (và không thể) chặn bằng `symlink_metadata`,
vì đây là **giới hạn chính sách**, **không** phải lỗi đua (TOCTOU). Sau khi `#146` đóng (ST-174), đây là
**giới hạn còn lại duy nhất** của **đường đọc** (VD-186).

- **Vì sao không phải lỗi đua:** kể cả khi **không** có khe thời gian nào, hardlink trong root vẫn là
  một **tên hợp lệ** trong vùng được cấp và trỏ tới **cùng inode** với tệp ngoài — hệ tệp coi đó là
  **cùng một vật** (hardlink không có "đích" để phân giải như symlink). `canonicalize` không đổi gì,
  `symlink_metadata` và `metadata` trả **cùng** `(dev, ino)` ⇒ quyết định "trong root" là **đúng chính
  sách hiện tại** ("vùng = cây tên"), **không** phải "lọt do đổi tên trong khe".
- **Điều kiện tạo được:** cần **quyền ghi** vào một thư mục **trong root** và tệp đích **cùng một hệ
  tệp** (hardlink không qua hệ tệp). Người chỉ có `cap[Dir]` **đọc** không tạo được hardlink.
- **Landlock cũng không chặn:** Landlock áp trên **đường dẫn lúc mở**, không trên nguồn gốc inode ⇒ ca
  hardlink lọt cả khi có `--sandbox`.
- **Đường ghi đã đóng khe đua (ST-295)**; hardlink ở trên chỉ ảnh hưởng **đường đọc** theo chính sách
  "cây tên", không mở lại khe kiểm-rồi-ghi.
- **Muốn chặn theo chính sách (không làm trong VS2):** đổi định nghĩa vùng từ "cây tên" sang "vật" (so
  `(dev, ino)` của mọi tên trong root với tệp đích), hoặc chuyển sang `openat`/`openat2` +
  `RESOLVE_BENEATH` + kiểm quyền trên **handle** ⇒ là **chính sách tương lai** (không cùng #109b — nợ đó
  đã đóng ở ST-295). **Không** tự đổi chính sách trong VS2.

### Windows — mức bảo đảm **thấp hơn**, chỉ **đo** (VD-146, VD-178 QĐ-4, VD-186)

**Kết luận một dòng:** trên Windows, `FsGuard` **chỉ kiểm theo đường dẫn** và **không có danh tính tệp**
(`(dev, ino)` = `(0, 0)`), nên mức bảo đảm **thấp hơn** macOS/Linux; Windows trong VS2 **chỉ đo**, **chưa
hỗ trợ** (VD-121/VD-146).

- **Không có danh tính tệp:** `xac_nhan_sau_mo` chỉ so **đường dẫn trước ≠ đường dẫn sau** (nhánh
  `#[cfg(not(unix))]` trong `crates/vis-cli/src/quyen.rs`) ⇒ chặn được ca "đổi sang đường dẫn khác rồi để
  nguyên", **không** chặn được ca "đổi rồi đổi lại".
- **Chỉ kiểm theo đường dẫn:** Windows **không** có `openat`/`O_NOFOLLOW` (bản vá ST-295 chỉ bật trên
  Unix) ⇒ **đường ghi** vẫn kiểm-rồi-mở, **cộng** việc không xác nhận được danh tính ⇒ khe hở **rộng
  hơn** Unix, không hẹp hơn.
- **Không có Landlock:** Landlock **chỉ Linux** (mục cuối tệp) ⇒ không có lớp kernel nào bù.
- **Số đo — chỉ để biết, KHÔNG phải tuyên bố hỗ trợ:** host suite **39/54**; `cargo test` **1053/100**;
  L0 dừng ở suite `cap_proc` (`/bin/echo`, `/bin/sh` không tồn tại); `bin/vis-targets` **18/18**;
  self-test **12/12**; cổng đi qua **9/10** bước rồi dừng ở L0 — `agent/reports/R26-viscript.md` §3.3,
  log `st144-host-suite.log`, `st153-gate-psvn.log`.
- **Điều kiện để nói khác đi:** chỉ khi VS3-5 (VD-188) xong và có runner Windows thật; **không** suy rộng
  từ "cổng đi qua 9/10 bước" thành "Windows được hỗ trợ".

## Landlock chỉ Linux — **giới hạn nền**, không phải thiếu sót (ST-173, VD-175 §2)

**Kết luận một dòng:** Landlock là cơ chế **chỉ có trong nhân Linux**; `vis-host-linux` vì vậy **chỉ
biên dịch và chỉ có ca kiểm trên Linux**. Trên macOS/Windows crate này **rỗng (0 ca)**, và **mọi** tuyên
bố “kernel chặn” chỉ có bằng chứng từ Linux — đây là **giới hạn nền**, **không** phải lỗi, không phải
test bị bỏ.

### Cách cfg-hoá (ST-173)

- `vis-host-linux/Cargo.toml`: `landlock = "=0.4.7"` nằm trong
  `[target.'cfg(target_os = "linux")'.dependencies]` — trên nền khác crate **không** được kéo vào.
- `vis-host-linux/src/lib.rs` và `vis-host-linux/tests/landlock.rs` có `#![cfg(target_os = "linux")]`
  (có từ ST-132; ST-173 chỉ chuyển dependency, **không** sửa thân test).
- **Không** `#[ignore]`, **không** bỏ ca, **không** đổi kỳ vọng/ngưỡng. Trên Linux dependency **vẫn bắt
  buộc**: thiếu crate `landlock` ⇒ build Linux **đỏ** — không có đường “âm thầm bỏ bảo vệ”.

### Hệ quả (đọc kỹ trước khi trích dẫn)

1. **macOS/Windows không chạy ca Landlock.** `cargo test -p vis-host-linux` trên mac: **0 ca**
   (trước ST-173: **đỏ biên dịch** crate `landlock`, 6 lỗi, `[exit=101]`; sau ST-173: `[exit=0]`, 0 ca).
   Crate rỗng **không** phải “đã kiểm” — chỉ là “không áp dụng”.
2. **Hành vi bảo vệ chỉ đo trên Linux.** Số ca trên `ps` (Linux `7.0.0-43-generic`, LSM có `landlock`):
   **trước 15 passed / sau 15 passed** (`0 failed`, `[exit=0]`) — **KHÔNG đổi**. Log:
   `agent/JOURNAL/st173-ps-truoc.log`, `st173-ps-sau.log`.
3. **Cổng trên mac không phát hiện hồi quy Landlock.** Muốn biết Landlock còn chặn thật phải chạy
   `cargo test -p vis-host-linux --test landlock` trên **Linux** (runner `ps`).
4. Trên mac/Windows, bảo đảm duy nhất là **cưỡng chế runtime** (`FsGuard`/`NetGuard`, ST-72b) —
   **không** được suy rộng thành “có sandbox kernel”.

| Nền | Biên dịch `vis-host-linux` | Số ca Landlock chạy | Bằng chứng |
|---|---|---|---|
| Linux (`ps`, kernel `7.0.0-43-generic`) | Có — dependency **bắt buộc** | **18/18** | `st173-ps-truoc.log`, `st173-ps-sau.log`; `st295-verify.log` |
| macOS — **số cũ, chưa verify** | Có, nhưng crate **rỗng** | **0** (không áp dụng) | `st173-mac-truoc.log`, `st173-mac-sau.log` — **không** dùng làm bằng chứng macOS (ST-242) |
| Windows | — | **0** (không áp dụng) | chưa có runner |

**Không** dùng câu: “Landlock bảo vệ trên mac/Windows”; “không biên dịch được nên bỏ qua”; “mac thiếu
sandbox kernel”. Đúng: “**Landlock chỉ Linux** — mac/Windows chỉ có `FsGuard`/`NetGuard` ở tầng runtime”.

## Cây nguồn để đo phải SẠCH (ST-175, VD-177 §4)

- `bin/vis-remote` kiểm `git -C <repo> status --porcelain` trên `ps`/`psvn` **trước khi chạy**; cây bẩn ⇒
  **từ chối**, in số mục bẩn + 5 mục đầu + gợi ý, mã thoát **6**. `mac` **không** kiểm mặc định (cây làm việc
  của agent; chính công cụ ghi log vào repo) — bật bằng `--require-clean`.
- `--allow-dirty` **chỉ để chẩn đoán**, KHÔNG dùng cho phép đo. Bằng chứng: `JOURNAL/st175-chan-cay-ban.log`
  (bẩn ⇒ `[exit=6]`), `st175-cho-phep-cay-ban.log` (`--allow-dirty` ⇒ `[exit=0]`), `st175-kiem-che-do.log`.
- `git rev-parse HEAD:viscript` là hash **của commit**, KHÔNG phải cây làm việc ⇒ không trích dẫn nó như
  bằng chứng “cây sạch”; bằng chứng đúng là `git status --porcelain` rỗng + dòng `# ST-175: cay SACH` trong log.

## macOS — verify `v0.4.5` (2026-11-23)

- `mps`: macOS **27.0.1** (Darwin 27.0.0) **arm64**, Rust **1.100.0**, Node **26.12.1**.
- `./vis-dev check` PASS; gate **3631/3**; workspace **3636/0/3**; host **54/54**; WASI suite **28/28**; `vis-wasm-sem` **388/390 = 99,5%** (2 stack-overflow interp known, VM pass).
- Fix test-only/tooling: `fix/macos-v045-tests` (APFS non-UTF-8 skip, `st295` cfg Linux, harness `CARGO_TARGET_DIR`, Python 3.9).
- **Chưa tuyên bố hỗ trợ macOS**; cần A0 quyết định tier.
