# PLAN — VD Windows L0 test-suite portability (nháp, 2026-11-19 ~21:30)

> Agent 1 → A0 (plan) · **KHÔNG triển khai trong đợt này** · VD-winport-po
> **Trạng thái 2026-11-20 ~00:55 @ HEAD `be3322f`**: **VD đã mở 2026-11-19** (lệnh A0-dispatch ủy thác); đang triển khai. Nhóm A xong (test-child + 7 suite migrate); Linux workspace xanh 150 khối; gate/nightly PASS. **Windows verify @ `be3322f`: ĐỎ — 53 ca** (`/c/Users/ps/vislogs/be3322f-ws.out`, `vislogs/be3322f-failed.txt`): PATH-sep trong test grant/echo (`:` thay vì `;` trên Windows), `/etc/hosts` không tồn tại, CRLF (`\r\n`) trong chan_doan_da_tep, WSL-stub (`sync_list`, `help_co_check_sync`), WinSock (`tcp_tu_choi`, `server_*`), UNIX mode (`long_tren_thu_muc_that`), span/json fixtures. **Không đổi ngữ nghĩa sản phẩm; Tier 2 giữ; pha1-rc1 nguyên vẹn; pha 2 chưa mở**.

## 1. Hiện trạng đo được (psvn HEAD `8355022`)

**~101 ca đỏ** trong **25 suite** (90 tên test duy nhất). Phân loại theo báo cáo ban đầu trên `13c31cf`:

| Loại | Số ca | Ví dụ cụ thể | Bản chất |
|---|---|---|---|
| **A — POSIX binaries** | ~44 (+4 WSL-stub) | `cap_proc*`, `cap_proc_grant`, `extern`, `pipeline`, `vish_main`, `vsh1b`, `ngoai_vm`, `vm_main_argv`, `sem` (4 `vish/ngoai-*`), `siet_caps_pkg`, `soak_script`, `sync_list` (4, WSL-stub) | Test gọi `/bin/echo`, `/bin/sh`, `/usr/bin/grep`, `/usr/bin/printf`, `/usr/bin/true|false`, `/usr/bin/sleep`, `cat`, `lm` |
| **B — Windows path/CRLF/JSON** | ~45 | `chan_doan_da_tep`, `diag_cmp` (some), `json_ra`, `span_tep`, `std_fs_json (7)`, `strict_caps (2)`, `quyen`, `siet_caps_pkg`, `host_conformance` | Timeout/backslash `path.display()` lọt `\"` → VS0102 escape thô; `\r\n` ở `output` so với `\n` ở `.out` fixture |
| **D — WinSock loopback** | 4–5 | `net::tcp_tu_choi`, `net_server::server_*` | Windows drop SYN op OS: `connection timed out` (không `connection refused`) ⇒ test expecting refused panic |
| **E — Unix file mode** | 1 | `vsh1b.long_tren_thu_muc_that` | `mode -` nãoas để `ntstatus` khác |
| **C — WSL stub** | 4 | `sync_list::*` (4 ca) | `/bin/bash` là WSL stub, `execvpe(/bin/bash) failed` |

**Không phải lỗi sản phẩm**: tất cả chỉ là giả đặc test harness; `report-check`/`vis-dev check` PASS; `doc-examples` PASS.

## 2. Phạm vi VD PO-Windows portability (nếu A0 mở)

Mốc #1 (chất lượng tối thiểu):
- **Mỗi API process đa nền tảng** phải có **ít nhất một ca thành công trên Windows** (ca I/O process/pipe/cap grant/động quên).
- Có coverage: stdout/stderr/exit code; capability grant/deny boundary; pipeline hai đầu.
- **Không giảm tổng số test process xuất hiện trên Windows chỉ để xanh.**

Mốc #2 (Tier 1 điều kiện):
- `./vis-dev gate --fast` chạy Windows xanh (L0 không required).
- Continuous CI cho `x86_64-pc-windows-msvc` với same coverage as Linux.
- `Windows Tier 2 → Tier 1` chỉ sau khi cửa sổ định mức này + VD freeze狭 hẹp → đầy đủ.

## 3. Phương án cho phân nhóm A — POSIX binaries

### A.1 Test fixture binary chưng cất

Tạo `crates/vis-cli/tests/test_child.rs` (binary riêng `test-child`) emulate các thao tác POSIX phổ biến:

```
test-child echo <args…>       → in các args, 1 nl cuối
test-child echo-no-nl <args…> → in các args, không NL
test-child cat [file]         → in file hoặc stdin
test-child grep <pattern>[file|/@]<stdin…>
test-child printf <fmt> [args…]
test-child true / false       → exit 0 / exit 1
test-child sh                 → exit 3 do argv[1] = '3' hoặc parse "-c exit N"
test-child sleep <ms>
```

Binary **Rust**, không shell/không phụ thuộc external — chạy trên mọi nền tảng với MSVC/GNU.

Cargo.toml thêm:

```toml
[[test]] name = "test_child"
[[bin]] name = "test-child" path = "tests/test_child.rs"
```

Test file dùng `env!("CARGO_BIN_EXE_test-child")` thay `/bin/echo` để gọi child. `.vish` fixture cũng dùng child (qua `--grant p=<path-to-test-child>`).

### A.2 POSIX-only test compatibility layer

Trong `crates/vis-cli/tests/chay.rs` (helper chung):

```rust
pub struct HostCommand(PathBuf, Vec<String>);
impl HostCommand {
    pub fn echo(args: impl IntoIterator<Item = &str>) -> Self { … env!("CARGO_BIN_EXE_test-child") … }
    pub fn cat(args…) -> Self …
    pub fn grep(args…) -> Self …
    pub fn printf(args…) -> Self …
    pub fn sleep_ms(ms) -> Self …
    pub fn sh(args…) -> Self …
    pub fn true_() -> Self …
    pub fn false_() -> Self …
}
```

Mỗi call `Command::new("/bin/echo")` → `HostCommand::echo(…)`. Trên Linux `test-child` cùng inline `/bin/echo` semantics; test mới sideffect không đổi.

### A.3 Test suite thay đổi

Suite cần sửa (cấp_proc, grok, vsh1b, vish_main, vm_main_argv, extern, pipeline, …): thay bốn hàng gọi với `HostCommand::echo(…)` + có thể thêm một số ca mới verify child properties.

### A.4 WSL stub (sync_list — 4 ca)

`sync_list` dùng `bash -lc` WSL stub (`execvpe(/bin/bash) failed`). Hai phương án:
- Bỏ WSL stub: kết nối chỉ đến Git Bash path (`C:\Program Files\Git\bin\bash.exe`) — thật PATH đã có sẵn ở Win10/11 với Git Bash; hoặc
- Hoặc rewrite sang spawn direct binary — `sync_list` `vis-linux` script is pure bash: dùng `"$VIS_BIN""$ARGS…"` trực tiếp.

Đề xuất: **c invoce trực tiếp** `vis-linux --check-sync` through `Command::new("./vis-linux")`.

## 4. Phương án cho phân nhóm B — path/CRLF/JSON

- `std_fs_json`/`strict_caps`/`quyen` fixtures: path literal `.vis` dùng `p"<forward-slash path>"` hoặc `/windows/fory/ward/slash` (Windows accept cả hai); nơi tương đối:
  - `.vis` fixture được sinh ra với `format!("{}", path.display())` → sử dụng `path.display().to_string().replace('\\', "/")`;
  - hoặc đặt tất cả fixture trong `/tmp/temp_dir` với path ASCII (`C:/Users/ps/…`) — yêu thích; Windows natively accept.
- CRLF assertions: test helper `assert_buffered` đọc file, normalize `\r\n→\n` trước khi so sánh; không sửa .out fixture.

## 5. Phương án cho phân nhóm D — WinSock loopback

`tcp_tu_choi`: patched test expected message:
- `"connection refused"` (Linux)
- `"quá thời gian chờ nối tới"` (Windows)
→ assert timeout message OR match 2 outcomes; server port đã đóng, kernel refused/timeout cả hai là hợp lệ\_OS-specific.

`net_server::server_*`: TCP bind → connect → accept; trên Windows loopback works fine. These shouldn't really tick over with syn gap. May need to disable Windows branch for now or note the `listen` needs `SO_REUSEADDR` for Windows. To investigate – may need Windows testing box + cargo locally.

## 6. Phương án cho phân nhóm E — UNIX file mode

`vsh1b::long_tren_thu_muc_that`: assert `mode` field of FileEntry: trên Windows không có permission bits (trả `-` như hiện tại macA theo code). Sửa assertion thành kiểm `is_file` + `size` + `mode == "-" || mode == "100600"` (unix/bootstraps).

## 7. Estimate

| Loại | Số ca | Ước lượng |
|---|---|---|
| A — test-child + harness | ~44 | **lớn** — 1 file Rust + 20 dòng harness; test suite đổi qua `s/\/bin\/*/Ho$/\ =/g` |
| B — path/CRLF | ~45 | **trung bình** — 10-20 test redoutine |
| D — WinSock | 4–5 | **nhỏ** |
| E — UNIX mode | 1 | **nhỏ** |
| C — WSL stub | 4 | **nhỏ** |

Tổng: 1-2 phiên; không đổi ngữ nghĩa API; không đổi ABI/đóng băng.

## 8. Rủi ro

- **`test-child` mới phải được biết hết**: hành vi echo/grep/cat must match exactly POSIX command; `test-child grep -E "^title" file` must find lines starting with "title"; `test-child sleep` block for milliseconds only.
- **Binary mới được build**: `cargo test --workspace` builds both interp and VM tests using `CARGO_BIN_EXE_test-child`; ensures it's called before all tests.
- **Windows POSIX tools expectation**: test Child's `cat file` reads bytes to stdout — all valid ASCII/UTF-8 since fixtures are ASCII.
- **CHẶN `test-child`** **không cho allowlist arbiterál**: không có `/bin/*` trong `--grant` — chỉ grant allowlisted program path; sau đó test-child là allowlisted tool path caller bang with `--grant p=<path>/test-child`.

## 9. Gates sau porting hoàn thành

- `cargo test --workspace` trên Windows → **0 fail** (trừ ca tiếp tục POSIX-only nếu được cfg-gate).
- `./vis-dev gate --fast` trên Windows → **PASS** (L0 không required).
- `vis test test/sem` trên Windows → **525/525**.
- `vis-dev nightly` trên Linux — unchanged; fuzz counts unchanged.

## 10. Trạng thái A0 quyết định mở

Co-pilots need: `agent/QUEUE.md` 1994 line change từ BLOCKED → READY nếu A0 duyệt mở vd
**VD-PO-Windows-portability**; implementation theo kế hoạch này.

Tương ứng với quy tắc `ST-126` (CONSTANT-API gate blocked) — khác kiểu: không phải API; không tự mở VBC1/freeze; chỉ porting test.

## 11. Ref assets

- `agent/WINDOWS-BASELINE-2026-11-19.md` — baseline đỏ + addendum by HEAD.
- `agent/BAO-CAO-AUDIT-DOCS-2026-11-19.md` — doc audit (HEAD `8d68a0c`).
- `RL: agent/BAO-CAO-PIPELINE-PORTABILITY-2026-11-19.md` — sẽ sinh sau cho từng bucket.
- `agent/SNAPSHOT-2026-11-19.md`+`2026-11-20.md` — baseline reports.

---

**Trạng thái tài liệu**: nháp, chưa đụng mã. Đợi A0 quyết mở VD-portability; không merge cho đến khi có VD chính thức + phê duyệt A0 về tên child binary + Cargo.toml entry + cấp phép `--grant` policy (⇒ approval của A0 cho mọi lệnh entry dựng từ Rust).
