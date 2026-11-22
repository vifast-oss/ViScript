<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Nguyên văn quyết định Agent 0 (VD-272..VD-279) — áp trực tiếp theo VD-279 (mở rộng VD-119). -->
# QUYẾT ĐỊNH AGENT 0 · 2026-11-22 · Kế hoạch dài hạn hậu `v0.3.5` (`v0.3.6` → `v1.0.0-rc1`) & Quy trình Vai trò `VD-279`

> **Người ban hành**: Agent 0 (được người điều hành phê duyệt và uỷ quyền kiêm nhiệm `Agent 0 + Agent 1 + Agent 2` ngày 2026-11-22).
> **Baseline**: `HEAD` = `13202b6` · Tag `v0.3.5` @ `03bb7b9`.
> **Phạm vi commit `[ST-00]`**: lưu tệp này vào `agent/JOURNAL/quyet-dinh-a0-2026-11-22.md`, cập nhật `agent/DECISIONS.md` (`VD-272`..`VD-279`), `agent/QUEUE.md` (`ST-284`..`ST-309`), `AGENTS.md` và `agent/ROADMAP.md`.

---

## Khối 1 — Quyết định `VD-272`..`VD-279` (đã ghi vào `agent/DECISIONS.md`)

### VD-272 · Lộ trình dài hạn hậu `v0.3.5` & cổng chặn cứng R0 Refactor — A0, 2026-11-22

> **Người quyết định**: Agent 0 (người điều hành đã phê duyệt 2026-11-22).
> **Baseline**: `13202b6` (tag `v0.3.5` @ `03bb7b9`). `pha1-rc1` và các tag `v0.1.0`..`v0.3.5` giữ nguyên.

1. **Chuỗi 5 giai đoạn phát hành**:
   - **Giai đoạn 0 (`v0.3.6`)**: R0 Refactor giải phóng ngưỡng R1 + Hook huỷ HTTP + Khép G3 gói (`VS0708`, `#142`) + Đồng bộ `Cargo.toml` version, `SO-NO.md`, `ViScript-thiet-ke.md`, `HOST-BAO-DAM.md`, `#151`, báo cáo `R32`.
   - **Giai đoạn 1 (`v0.4.0` — G4a Core)**: Ngân sách mặc định (`VS3-1a`, `VS0510`) + Worker batch (`P2b-1/2`) + Worker Frame (`P4a..c`, `--workers`, `VS0509`) + Kênh `select` (`VS3-2c`) + `async-h2` Windows & default-on + Đóng khe TOCTOU đường ghi (`VS3-QĐ-4`, `#109b`/`#140`) & Landlock `#125`.
   - **Giai đoạn 2 (`v0.4.5` — G4b Platforms)**: Đóng `#149` + Nâng **Windows Tier 1** (`bin/vis-gate-windows`, chốt `#191`) + **`vis-host-wasm` WASI runtime thật** (`wasm32-wasip1` + `wasmtime` runner `bin/vis-wasm-suite`).
   - **Giai đoạn 3 (`v0.5.0` — Pha 2 Ngôn ngữ, CLI & ViShell MVP)**: Chốt `T??` + Lambda type annotation + `#155` + `async` method/`cmd` (`#13`) + `derives Json` codegen (`#8`) + `vis new`/`vis doc`/`vis repl` + ViShell tương tác (`VSH1`, `crates/vish-edit`) + đóng gói VS Code extension mỏng.
   - **Giai đoạn 4 (`v0.6.0` → `v1.0.0-rc1` — G5 Hệ sinh thái & Đóng băng)**: `vis-plugin-abi` (static + dynamic tắt trong strict-caps/sandbox) + `Terminal` trait & giao thức ViTerm có cấu trúc + `docs/SPEC.md` + kiểm toán bảo mật & tổng nghiệm thu 24h soak.
2. **Cổng chặn cứng R0 Refactor**: Cấm mở bất kỳ task tính năng nào chạm `vm.rs`, `frame.rs`, `value.rs`, `codegen.rs`, `tyck/expr.rs`, `interp/goi.rs`, `async_net.rs`, `quyen.rs`, `db/sqlite/mod.rs` trước khi `ST-284`..`ST-286` đưa toàn bộ tệp lõi về **≤ 1.250 dòng** với 0 thay đổi hành vi.
3. **Đồng bộ phiên bản & quản trị**: Từ `v0.3.6`, mọi crate trong workspace dùng `version.workspace = true` khớp bản phát hành; mọi lát mới đánh số `ST-284` trở đi trong `agent/QUEUE.md`; mỗi mốc phát hành xuất đủ báo cáo `R<n>` (`R32` tại `v0.3.6`).
4. **Ràng buộc giữ nguyên**: macOS tiếp tục **ngoài phạm vi** (chờ phần cứng Darwin thật); **cấm thêm `.github/workflows/`** (verify local `ps` + `psvn`); nhịp **batch test 4 giờ**, tối đa 4 luồng worktree riêng.

### VD-273 · Chốt Ngân sách mặc định (`VS3-1a`), 3 mã lỗi mới & Hook Huỷ HTTP (`VS3-1b`) — A0, 2026-11-22

1. **Ngân sách mặc định (`v0.4.0`)**: Bật ngân sách ngữ nghĩa thống nhất interp↔VM mặc định `BUDGET_DEFAULT = 40_000_000`, giữ `VM_SAFETY_FUEL = 100_000_000` (> 2× trần ngân sách). Cung cấp cờ CLI `--max-budget <N>` và `--unlimited-budget` cho workload hợp lệ dài hơn. Đổi semantics mặc định ⇒ bắt buộc bump `PHIEN_BAN` cache + re-pin.
2. **Cấp đích danh 3 mã lỗi mới** vào `crates/vis-diag/src/lib.rs` và `agent/API-CODES.txt` (đủ 3 ngôn ngữ `vi/en/ja`, đúng quy trình `VD-129`):
   - **`VS0509`** (Error): Vi phạm ghi biến cấp tệp chéo worker ở chế độ song song (`W > 1`).
   - **`VS0510`** (Error): Chương trình vượt ngân sách thực thi ngữ nghĩa (tách hẳn khỏi `VS0501` trap nội bộ / fuel safety net).
   - **`VS0708`** (Error): Lỗi xác minh chữ ký số / khoá tin cậy registry gói (`VS3-5b/5c`).
3. **Hook huỷ giữa request HTTP (chung h1 + h2)**: Mở `FrameReactorBackend::cancel_task_handles(frame_id)` gọi `HttpAsync::dong()` + `RST_STREAM` (h2) / đóng socket (h1) khi task bị huỷ tại safe point. Đây là điều kiện bắt buộc trước khi bật mặc định `async-h2`.

### VD-274 · Kênh `select` (`VS3-2c`): API hàm tĩnh trước, không đổi EBNF ở `v0.4.0` — A0, 2026-11-22

1. Trong `v0.4.0`, triển khai `select` dưới dạng 4 hàm dựng sẵn có kiểm kiểu tĩnh (không thêm từ khoá khối, không đụng `SPEC-lex`/parser/`vis-fmt`):
   - `select_recv(chans: list[Chan[T]]) -> {index: int, value: T?}`: chờ chặn tới khi một kênh sẵn sàng hoặc đóng; danh sách rỗng ⇒ `VS0501`; tất cả treo không có nguồn ⇒ `VS0505`.
   - `select_recv_timeout(chans: list[Chan[T]], ms: int) -> {index: int, value: T?}?`: `ms < 0` ⇒ `VS0501` (nhất quán `recv_timeout`), `ms == 0` ⇒ thăm dò 1 lần, hết giờ ⇒ `None`.
   - `select_send(chans: list[Chan[T]], val: T) -> int?`: trả `Some(index)` của kênh nhận được `val`, hoặc `None` nếu toàn bộ kênh đã đóng.
   - `select_send_timeout(chans: list[Chan[T]], val: T, ms: int) -> int?`: `ms < 0` ⇒ `VS0501`, hết giờ hoặc toàn bộ đóng ⇒ `None`.
2. **Bất biến bắt buộc**: (a) xoay vòng vị trí bắt đầu tất định theo turn để chống bỏ đói (fairness); (b) khi một nhánh thắng/timeout/cancel, gỡ đăng ký waiter khỏi mọi kênh còn lại **đúng một lần** (không rò waiter, không double-wake); (c) chạy trên frame runtime, byte-identical giữa interp và VM.
3. Cú pháp khối `select:` cho kênh **khác kiểu** để xét riêng sau `v0.5.0` nếu có nhu cầu thực tế.

### VD-275 · Chốt triển khai Worker Pool `P2b` + `P4` & cờ CLI `--workers` (`VS3-3b`) — A0, 2026-11-22

1. **Thứ tự bắt buộc**: `ST-284` (R0 refactor `vm/host.rs`, `frame/worker.rs`) → `ST-291` (`P2b-1` lazy/batched submit + `P2b-2` compact `Vec` proxy) → `ST-292` (`P4a → P4b → P4c` theo thiết kế `agent/JOURNAL/vs3-w5-p4-frame-plan.md`).
2. **Chính sách `P4a` tại `v0.4.0`**:
   - Chỉ main frame (`current == 0`) offload `spawn` lên pool; frame task (`current != 0`) chạy local trong driver.
   - `chan(0)` và host/async I/O bên trong worker task bị **từ chối rõ** (không treo, không hứa wire channel qua pool ở `v0.4.0`).
   - Output worker đệm theo task, flush tại điểm tiêu thụ (`await`) và theo thứ tự spawn ở `final_join` để `W=1/2/4` byte-identical.
   - Cross-write top-level ở `W > 1` báo **`VS0509`**.
3. **Giao diện CLI**: Mở cờ tường minh **`vis run --workers <N>`** (mặc định `1` = single-thread giữ nguyên hành vi cũ; `N == 0` là lỗi dùng exit 2). Không đổi mặc định sang `N > 1` trong `v0.4.0`.

### VD-276 · Đóng khe hở TOCTOU đường ghi Unix (`VS3-QĐ-4`, `#109b`/`#140`) & Landlock `#125` — A0, 2026-11-22

1. **Đường ghi + đọc dùng chung `openat`**: Thay đường ghi kiểm-rồi-mở của `FsGuard` trên Unix bằng hàm mở từng thành phần từ fd gốc (`openat` + `O_NOFOLLOW`/`O_DIRECTORY`, ưu tiên `openat2(RESOLVE_BENEATH | RESOLVE_NO_SYMLINKS)` trên Linux ≥ 5.6).
2. **Phê duyệt phụ thuộc có điều kiện (giữ 0 `unsafe` trong mã ViScript — luật R2 nguyên vẹn)**:
   - Duyệt **Phương án A**: dùng crate **`rustix`** (chỉ bật feature tối giản `fs`, ghim phiên bản tuyệt đối `=…`, chỉ đặt trong `[target.'cfg(unix)'.dependencies]` của `vis-cli` hoặc `vis-host-linux`).
   - Trước khi merge, Agent 2 bắt buộc chạy kiểm tra giấy phép toàn bộ cây bắc cầu bằng `cargo metadata` (mọi phụ thuộc phải thuộc allowlist `MIT / Apache-2.0 / Zlib / Unicode-3.0`, ghi log vào `agent/JOURNAL/st295-license.log`); nếu có phụ thuộc ngoài allowlist ⇒ `BLOCKED` báo A0 ngay.
3. **Landlock `#125`**: Khi `--sandbox` chạy kèm `cap[Proc]` đã được `--grant`, cấp quyền chỉ-đọc + `AccessFs::Execute` cho các đường dẫn thư viện động chuẩn (`/lib`, `/lib64`, `/usr/lib`, `/usr/bin` nếu tồn tại) và đúng binary đã canonicalize trong grant; kèm ca âm chứng minh vẫn chặn ghi vào `/lib*` và chặn chạy binary ngoài grant.
4. **Xoá cảnh báo**: Chỉ được xoá câu cảnh báo “đường ghi còn khe hở kiểm-rồi-mở” trong `agent/HOST-BAO-DAM.md` trong **cùng commit** mà ca đua symlink ghi đỏ-trước **100/100 → sau vá 0/100** đã xanh.

### VD-277 · Tiêu chí công nhận Windows Tier 1 & WASI Runtime (`VS3-6`, `v0.4.5`) — A0, 2026-11-22

1. **Windows Tier 1**: Chỉ được nâng nhãn từ Tier 2 lên **Tier 1** khi đồng thời thoả 5 điều kiện trên máy `psvn` thật:
   - `async-h2` bỏ `#![cfg(unix)]`, chạy xanh trên Windows qua `Nguon`/`mio` và được bật mặc định non-wasm (`ST-294`);
   - Đóng toàn bộ điểm tách đường dẫn còn lại trong `PATH-SPLIT-AUDIT-2026-11-18.md` (`#149`, `ST-297`);
   - Xác minh lại `cargo nextest` (`#191`): nếu hết flake sau vá `54f813f` thì đóng `#191`; nếu còn đỏ thì phân lập nguyên nhân gốc hoặc chốt chính thức `cargo test --workspace --no-fail-fast` là đường gate Windows chuẩn;
   - Có script một lệnh `bin/vis-gate-windows` chạy xanh **3 lần liên tiếp** + soak 4 giờ (debug + release + host 54/54 + targets + CLI smoke);
   - `agent/HOST-BAO-DAM.md` vẫn ghi trung thực giới hạn `FsGuard` trên Windows (kiểm theo đường dẫn, không có Landlock kernel).
2. **WASI Runtime (`wasm32-wasip1`)**:
   - Duyệt công cụ host `wasmtime` để chạy bộ kiểm thử (không thêm `wasmtime` vào `Cargo.toml` của `vis-cli`).
   - Triển khai host WASI thật trong `vis-host-wasm` (FS preopened dirs, clock, stdout/stderr; từ chối rõ `net`/`proc`/`sqlite`, không panic).
   - Nghiệm thu: `bin/vis-wasm-suite` chạy bộ lọc thuần ngôn ngữ + stdlib lõi của `test/sem` (≥ 85% tổng ca) trên **cả interp và VM**, **0 lệch byte** với Linux native; phần `net`/`proc`/`sqlite` gắn nhãn `EXPECTED_UNSUPPORTED` tường minh. Nợ `#156` giữ nguyên cho cấu hình `sqlite`/`wasm32-unknown-unknown`.

### VD-278 · Phạm vi Gói, Ngôn ngữ Pha 2, ViShell & Plugin ABI tới `v1.0` — A0, 2026-11-22

1. **Gói (`Phụ lục C`)**: Trong vòng đời `v1.0`, giữ mô hình **Registry HTTP(S) + Ed25519 + Lock v2 + Semver resolver** làm mô hình chính thức. Hoãn mô hình phi tập trung kiểu Go (`vifast.org/vis/*`, `sum.vifast.org`, MVS) sang hậu-`v1.0`; giữ nguyên nhãn `[thiết kế hậu-v1.0, chưa triển khai]` trong Phụ lục C.
2. **Ngôn ngữ Pha 2 (`v0.5.0`)**:
   - **Chốt `T??` hợp lệ** (`Option[Option[T]]` khớp code R2 hiện hành): sửa các dòng mâu thuẫn trong `docs/ViScript-thiet-ke.md` §4.4 và EBNF Phụ lục A.
   - Mở cú pháp chú thích kiểu tham số lambda `(x: int?, y: str) => expr` + auto-lift tham số closure ở cả hai engine (đóng `#155`).
   - Hỗ trợ `async fn` trong `impl` và `async cmd` (`#13`), và sinh mã serialize/deserialize thật cho `derives Json` (`#8`).
3. **Công cụ & ViShell (`v0.5.0`)**: Triển khai `vis new`, `vis doc`, `vis repl`, và ViShell tương tác MVP (`VSH1`, `crates/vish-edit`: soạn dòng đa nền, tô màu CST, tab completion theo kiểu, lịch sử append-only, job control POSIX `&`/`jobs`/`fg`/`bg`/`kill`, Ctrl-C exit 130, panic giữ phiên).
4. **Plugin Rust ABI & ViTerm (`v0.6.0`)**:
   - Tạo `crates/vis-plugin-abi` (`VIS_PLUGIN_ABI_V1`, liên kết tĩnh mặc định tắt trong release chuẩn) và `crates/vis-plugin-macros` (`#[vis::module]`/`#[vis::func]`).
   - Nạp thư viện động bị **tắt cứng** khi bật `--strict-caps` hoặc `--sandbox`; mọi quyền truy cập tài nguyên của plugin bắt buộc truyền tường minh qua `cap[...]`.
   - Thiết kế `Terminal` trait (`AnsiPosix`, `WinConsole`, `ViTermFramed`) độc lập với `ViFastOs` (giữ luật R4, không kéo `vis-host-vifast` vào cây nguồn).

### VD-279 · Chuẩn hoá phân vai theo Quyền hạn (độc lập môi trường, mở rộng `VD-119`/`VD-137`) — A0, 2026-11-22

> **Mục đích**: Tách **quyền hạn kỹ thuật** khỏi **năng lực công cụ môi trường** (có hay không có quyền truy cập repo). Chấm dứt nút cổ chai dán tay khi Agent 0 có repo và bỏ nhu cầu lập cơ chế "A0-proxy" tạm bợ, đồng thời giữ tương thích 100% với lịch sử `Agent 0 / Agent 1 / Agent 2`.

1. **Ba vai trò độc lập môi trường**:
   - **Kiến trúc & Đặc tả (`Architect` — nhãn `Agent 0`)**: Sở hữu `docs/SPEC*.md`, `agent/DECISIONS.md`, `agent/QUEUE.md` (mở hàng & duyệt), phê duyệt `agent/API-*.txt`, crate ngoài, ngoại lệ `unsafe` và quyền gắn tag phát hành.
     - *Khi có quyền truy cập repo (`Repo-enabled`)*: Ghi trực tiếp quyết định vào repo, chạy `./vis-dev rules` + `check_report.py --all`, tạo commit riêng **`[ST-00]`** và đẩy lên `origin`.
     - *Khi chỉ trò chuyện (`Chat-only`)*: Giữ nguyên `VD-119` — xuất tệp `QUYET-DINH-A0-<ngày>.md` để `Builder` hoặc `Gatekeeper` dán nguyên văn thành commit **`[ST-00]`**.
   - **Thực thi (`Builder` — nhãn `Agent 2` + luồng `W1..W3`)**: Sở hữu mã nguồn `crates/`, `vis-host-*`, test đỏ-trước, `agent/STATE.md`, `agent/BLOCKED.md`, `agent/JOURNAL/`, ghi nợ mới vào `agent/SO-NO.md`. Làm trên worktree/nhánh riêng theo bảng sở hữu tệp; không tự đổi SPEC, golden cũ, ngưỡng cổng hay thêm crate/`unsafe` chưa có VD.
   - **Nghiệm thu & Cổng (`Gatekeeper` — nhãn `Agent 1` + luồng `W4 QA`)**: Đối chiếu độc lập tiêu chí `DONE` với bằng chứng máy (`fresh-clone`, `gate --fast`, `diff 100k`, `soak`, `psvn` Windows), rà lệch tài liệu/sổ nợ, xuất báo cáo `R<n>` và ký `APPROVED`.
2. **Chế độ Kiêm nhiệm (`Unified Agent` — 1 tác nhân kiêm `Agent 0 + Agent 1 + Agent 2`)**:
   - Khi được người điều hành uỷ quyền kiêm nhiệm nhiều vai trò, một agent được phép quyết định, triển khai, kiểm chứng và đẩy code trực tiếp, nhưng **bắt buộc giữ 3 ranh giới cứng**:
     1. **Tách commit theo vai trò**: Commit quyết định/SPEC/QUEUE (`[ST-00]`) **tách riêng hoàn toàn** khỏi commit mã/test (`[ST-###]`).
     2. **Bằng chứng máy thắng lời nói**: Bước `Gatekeeper` không tự duyệt bằng nhận định chủ quan; bắt buộc chạy cổng thật và lưu log/số đo tái lập được (kèm commit hash) trước khi đóng task hoặc gắn tag.
     3. **Cấm tự nới luật khi gặp lỗi mã**: Đang ở vai `Builder` gặp test đỏ hoặc đụng ngưỡng R1..R6 thì phải sửa mã; nếu thật sự cần đổi luật/ngữ nghĩa phải dừng nhánh mã, lập VD mới có lý do và commit `[ST-00]` riêng trước khi tiếp tục.
3. **Liên kết lịch sử**: `VD-119`, `VD-137`, `VD-196` tiếp tục có hiệu lực như trường hợp riêng của `Agent 0 (Chat-only)` và mô hình đa-agent tách máy.

---

## Khối 2 — Hàng việc `ST-284`..`ST-309` (đã ghi vào `agent/QUEUE.md`)

Xem bảng đầy đủ trong `agent/QUEUE.md` mục **“Chương trình Dài hạn hậu `v0.3.5` — `VD-272`..`VD-278`”**:
- **Giai đoạn 0 (`v0.3.6`)**: `ST-284`..`ST-289` (`READY`) — R0 Refactor (≤ 1.250 dòng/tệp lõi) + HTTP cancel hook + `VS0708`/`#142` + đồng bộ version/`SO-NO.md`/docs/`#151`/`R32`.
- **Giai đoạn 1 (`v0.4.0`)**: `ST-290`..`ST-296` (`BLOCKED` chờ GĐ 0) — Ngân sách `40M`/`VS0510`, Worker `P2b` + `P4`/`--workers`/`VS0509`, `select`, `async-h2` Windows + default-on, TOCTOU đường ghi `rustix`/`#125`, `R33`.
- **Giai đoạn 2 (`v0.4.5`)**: `ST-297`..`ST-300` (`BLOCKED` chờ `v0.4.0`) — `#149`, Windows Tier 1 (`bin/vis-gate-windows`, `#191`), `vis-host-wasm` WASI + `wasmtime` runner `bin/vis-wasm-suite`, `R34`.
- **Giai đoạn 3 (`v0.5.0`)**: `ST-301`..`ST-305` (`BLOCKED` chờ `v0.4.5`) — `T??` + Lambda annot + `#155`, `async` method/`cmd` (`#13`), `derives Json` (`#8`), `vis new/doc/repl`, ViShell MVP (`VSH1`), VS Code extension, `R35`.
- **Giai đoạn 4 (`v0.6.0` → `v1.0.0-rc1`)**: `ST-306`..`ST-309` (`BLOCKED` chờ `v0.5.0`) — `vis-plugin-abi`, `vis-plugin-macros`, `Terminal`/ViTerm protocol, `docs/SPEC.md`, kiểm toán bảo mật + soak 24h, `R36`.

---

## Khối 3 — Hướng dẫn khởi chạy cho `Builder` (`Agent 2`) ở máy khác

1. `git pull --ff-only origin main` (nhận commit `[ST-00]` đã chứa `VD-272`..`VD-279` và `ST-284`..`ST-309`).
2. Tạo 4 worktree tách biệt cho Giai đoạn 0 (`ST-284`..`ST-289`):
   - `W1` (`plan/w1-runtime`): `ST-284` → `ST-285`
   - `W2` (`plan/w2-net-sec`): `ST-286` → `ST-287`
   - `W3` (`plan/w3-platform`): `ST-288`
   - `W4` (`plan/w4-qa-docs`): sau khi W1/W2/W3 merge tuần tự vào `main`, chạy `ST-289` và xuất `R32`.
3. Giữ nguyên nhịp kiểm chứng: test đỏ-trước từng lát, `./vis-dev gate --fast` + `./bin/vis-host-suite` + `./bin/vis-targets` mỗi mốc 4 giờ trên `main`, không GitHub Actions, macOS ngoài phạm vi, `psvn` verify ghi log ngoài repo (`/c/Users/ps/vislogs/`).
