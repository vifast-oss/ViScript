# ST-131 · Chạy đầy đủ trên Linux — đo và liệt kê lệch (2026-11-16)

Người làm: lead (đợt 23). Căn cứ: `JOURNAL/quyet-dinh-a0-2026-11-16.md` §6 **ST-131**.
Runner: `bin/vis-linux` (backend `ssh`) → host `ps`, Ubuntu 26.04.1, kernel 7.0.0-43-generic x86_64,
12 core, rustc/cargo **1.99.1**, Landlock ABI 8.

Cây đo: **`/tmp/vf-gate` ở commit `6fb8c3de`** (worktree sạch, không lẫn thay đổi đang làm của
agent khác), đồng bộ sang `/root/vifast-linux` bằng rsync **trừ `target/`**.

**Task này CHỈ đo.** Không sửa gì trong lúc đo. Hai lỗi portability đã sửa **trước** khi đo là của
**ST-121** (fixture ca 5 + `check_report.py` trỏ `v2/agent/BLOCKED.md`); chúng nằm trong `6fb8c3de`
nên số dưới đây là số **sau** khi vá.

## 1. Số đo đặt cạnh macOS

| Phép đo | macOS (máy dev) | Linux (runner `ps`) | Kết luận |
|---|---|---|---|
| `cargo test --workspace --no-fail-fast` | **1066 passed / 0 failed** | **1062 passed / 4 failed** (122 suite) | **4 lệch** — liệt kê §3 |
| `vis-dev gate --fast` (dòng cuối) | **PASS** | **FAIL** ở `--- L0 test` (đúng 1 test, cùng ca #1 §3) | 10/11 bước PASS; chỉ L0 đỏ vì 1 ca |
| `bin/vis-host-suite` (54 ca) | 54 PASS | **54 PASS** | **không lệch** |
| `vis-dev check` (fmt/clippy/R1..R4/self-test/explain/báo cáo/doc-examples) | PASS | **PASS** toàn bộ | **không lệch** |

Ghi chú: 1066 − 1062 = 4, khớp đúng số ca đỏ ⇒ **không** có ca nào "biến mất" hay bị bỏ qua trên Linux.

## 2. Lệnh đã chạy (tái lập)

```bash
bash viscript/bin/vis-linux gate                       # = vis-dev gate --fast trên Linux
VIS_LINUX_NO_SYNC=1 bash viscript/bin/vis-linux run bash -c 'cd viscript && cargo test --workspace --no-fail-fast'
VIS_LINUX_NO_SYNC=1 bash viscript/bin/vis-linux run bash -c 'cd viscript && bash bin/vis-host-suite'
```

## 3. Bốn lệch — mỗi lệch: ca, macOS, Linux, phỏng đoán

### Lệch 1 · `complete_giu_stderr_va_code_khac_0` (`crates/vis-cli/tests/pipeline.rs:133`)

- **macOS**: `grep` với cờ lạ in ra `grep: unrecognized option ...` ⇒ chuỗi bắt đầu `grep:` ⇒ ca xanh.
- **Linux**: `/usr/bin/grep: unrecognized option '--khong-co' ...` ⇒ chuỗi bắt đầu `/usr/bin/grep:`
  ⇒ assert `o.contains("stderr  [grep:")` đỏ.
- **Sản phẩm KHÔNG sai**: record vẫn giữ đủ `stderr [...]` và `code    2` (đúng ngữ nghĩa `complete`).
- **Phỏng đoán**: ca kiểm bám vào **tiền tố thông điệp của GNU/BSD grep**; GNU grep in `argv[0]`,
  BSD grep in `grep:`. Sửa sau: so `stderr` **chứa** "unrecognized option"/"invalid option" hoặc so
  theo `code 2` thay vì tiền tố. (Không sửa trong task này.)

### Lệch 2 · `sandbox_check_luon_in_cuong_che` (`crates/vis-cli/tests/sandbox.rs:59`)

- **macOS**: `--sandbox-check` ⇒ exit **3** (không có Landlock) ⇒ ca xanh.
- **Linux**: exit **0** (Landlock có, probe đọc được `lsm` chứa `landlock`) ⇒ ca đỏ (`left: 0, right: 3`).
- **Sản phẩm KHÔNG sai**: exit 0 trên Linux là **đúng hợp đồng** (0 = có landlock, 3 = không, 4 = không rõ).
- **Phỏng đoán**: ca kiểm viết cho macOS, chưa `cfg` theo nền. Sửa sau: kỳ vọng theo `cfg!(target_os)`.

### Lệch 3 · `targets_wasm_vis_types` (`crates/vis-cli/tests/targets.rs:28`)

- **macOS**: `cargo check --target wasm32-unknown-unknown` xanh (target đã cài trong rustup).
- **Linux**: check **đỏ** ngay từ phụ thuộc ⇒ `wasm32-unknown-unknown` **chưa cài** trên runner.
- **Phỏng đoán**: môi trường, không phải mã. Khắc phục đúng cách: `rustup target add
  wasm32-unknown-unknown` trên runner (là **thành phần toolchain**, không phải phần mềm hệ thống) —
  cần Agent 0 cho phép vì ST-121 cấm tự cài; **chưa** làm trong task này.

### Lệch 4 · `than_lenh_tren_thu_muc_that` (`crates/vis-cli/tests/vish_cmd_body.rs:312`)

- **macOS**: `.vish` có thân `cmd` trên thư mục thật ⇒ exit 0.
- **Linux**: `error VS0501: tên 'tmp' chưa hỗ trợ ở VS0` tại `…/l.vish:3:7`, exit 1.
- **Phỏng đoán**: ca sinh mã nguồn có **đường dẫn thư mục tạm**; `std::env::temp_dir()` trên Linux là
  `/tmp/…` (ngắn, không dấu cách) còn macOS là `/var/folders/…` (dài). Một nhánh của ca có thể đang
  **không trích dẫn** đường dẫn khi nó "trông an toàn", khiến `/tmp/...` bị đọc thành chuỗi lệnh bắt
  đầu bằng `/` rồi tách từ `tmp`. Cần một task riêng để xác định (đọc `vish_cmd_body.rs:280-320`).
  **Không** kết luận sản phẩm sai — chưa đủ bằng chứng.

## 4. Điều **không** lệch (quan trọng để không nói quá)

- 54/54 ca **hợp chuẩn host** chạy thật trên Linux, **0 lệch** ⇒ hạng mục "Hợp chuẩn host" có **nền
  thứ hai** thật, không chỉ macOS.
- Toàn bộ `vis-dev check` (fmt, clippy `-D warnings`, R1..R4, self-test âm, `vis explain`, report-check,
  doc-examples 24 khối) **PASS** trên Linux ⇒ harness không phụ thuộc macOS.
- `vis-host-suite` và `doc-examples` không cần sửa gì ⇒ các số khoá về host giữ nguyên trên nền mới.

## 5. Nợ mới (đánh số tiếp từ 105 — sau ST-132 đã dùng 105/106/107, nên bắt đầu 108)

- **108** — `pipeline.rs:133` bám tiền tố thông điệp grep (lệch 1).
- **109** — `sandbox.rs:59` chưa `cfg` theo nền (lệch 2).
- **110** — runner Linux thiếu `wasm32-unknown-unknown` ⇒ `targets.rs` đỏ (lệch 3).
- **111** — `vish_cmd_body.rs:312` đường dẫn thư mục tạm trên Linux (lệch 4) — cần điều tra riêng.
