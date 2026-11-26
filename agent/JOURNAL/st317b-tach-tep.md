<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-317b — Tách tệp sản phẩm > 1.100 LOC + thiết kế khử trùng lặp worker

- **Ngày**: 2026-11-26/27 (giờ thấp điểm) · **Vai**: Builder (Agent 2) · **Nhánh**: `plan/st317b` · **Nền**: `main` @ `3e828393` (Sóng 5 `DONE` + `VD-293`)
- **Worktree**: `/home/ps/viswt/st317b` — không chạm repo chính `/home/ps/ViScript`.
- **Cổng đã chạy**: `./vis-dev check` PASS · `./vis-dev gate --fast` PASS (`3880/3880`, 4 skipped) · `./vis-dev rules` R1..R9 OK (**R8 = 39/40**) · `vis-dag-check` OK (23 crate / 5 tầng / 0 cạnh cấm) · `VIS_DIFF_N=100000 ./bin/vis-diff` **`lech=0`** (100.000 chương trình, 106,2 s).

---

## 0. Lane điều chỉnh (theo phân tích xung đột độc lập, áp dụng trong lượt này)

| Hạng mục | Quyết định | Lý do |
|---|---|---|
| `crates/vis-host-native/src/db/sqlite/mod.rs` (1.130) | **HOÃN** — không chạm | `plan/st317a` đang sửa 4 điểm `Value::Enum` **tại chính tệp này**; tách bây giờ sẽ xung đột merge. Làm sau khi 317a hợp nhất (A0 báo). |
| `crates/vis-types/src/tyck/pat.rs` (1.140) | **LÀM NGAY** — xong | Đã kiểm 0 điểm `Value`, 0 xung đột với W1/W3; tách trước sqlite theo chỉ đạo. |
| `crates/vis-runtime/src/worker/proto.rs` | **KHÔNG move** | 317a đang sửa 5 điểm trong tệp; giữ nguyên. |
| `crates/vis-interp/src/frame/worker.rs`, `crates/vis-vm/src/vm/worker.rs` | **KHÔNG move** — chỉ thiết kế + danh sách test đặc tả (§4) | Hai tệp dựng/so `Value` (`Value::Unit`, `Value::Task`, `Value::Str`, `to_send`/`to_value`) và gọi `Task`/`TrangThai` — vùng 317a đang đổi `Value` 64→32 B. Move lúc này tạo xung đột cơ học trên cùng dòng. Đánh dấu **chờ 317a**. |
| `crates/vis-core/src/value/**`, `crates/vis-lsp/**`, `crates/vis-hir/src/stdlib/**`, `crates/vis-diag/**` | **KHÔNG chạm** | Lane W1/W3. |

---

## 1. Bảng tệp trước/sau (số dòng thật, `wc -l`)

### 1a. `crates/vis-pkg/src/pkg.rs` → `crates/vis-pkg/src/pkg/`

| Trước | Sau | Dòng | Trách nhiệm |
|---|---|---|---|
| `pkg.rs` | **1151** | — | — |
| | `pkg/mod.rs` | **54** | doc + khai báo module + re-export (không logic) |
| | `pkg/manifest.rs` | **178** | manifest + `tree_sha` (`bam_cay`) + validate `files[]`; 2 test cũ giữ nguyên |
| | `pkg/goi.rs` | **170** | `TepCai`/`GoiCai`, `doc_goi`, giải đồ thị, ghi cây `.vis` |
| | `pkg/khoa.rs` | **213** | `vis.lock` định dạng 2: ràng buộc hiệu lực, "còn thoả", dựng/ghi lock |
| | `pkg/nguon.rs` | **228** | đối số CLI, registry cục bộ/HTTP, khoá công khai Ed25519, `nap_goi` |
| | `pkg/lenh.rs` | **377** | `run` + 6 lệnh `add/install/update/list/remove/migrate` + `dung` |
| — | `pkg/{gc,registry_http,resolver,tar_b64}.rs` | 446/431/777/266 | **giữ nguyên**, chỉ dùng lại re-export `super::…` |

**API giữ nguyên**: `vis_pkg::pkg::run`, `pub mod resolver`, `pub use semver`, `pub(crate) use vis_loader::lock_meta::{…}`, các tên `pub(crate)` nội bộ (`GoiCai`, `Manifest`, `doc_tat_ca`, `giai_roots`, `doc_manifest`, `cwd`, `dung`, `bam`, …) — 4 tệp `gc/registry_http/resolver/tar_b64` **không phải sửa một dòng nào** vì `mod.rs` giữ đúng đường `super::`.

### 1b. `crates/vis-runtime/src/rt/sched.rs` → `crates/vis-runtime/src/rt/sched/`

| Trước | Sau | Dòng | Trách nhiệm |
|---|---|---|---|
| `sched.rs` | **1189** | — | — |
| | `sched/mod.rs` | **22** | doc + khai báo + `pub use Scheduler` + `pub(crate) use` 3 trigger |
| | `sched/scheduler.rs` | **120** | struct `Scheduler`, `new/tao/spawn`, `diag_snapshot` |
| | `sched/trigger.rs` | **16** | 3 `thread_local!` (`FRAME_CANCEL_TEST/TURNS/BYTES`) |
| | `sched/frame_api.rs` | **109** | `frame_enable_for_test`, re-arm, `frame_await`, `frame_register` |
| | `sched/frame_next.rs` | **113** | `frame_next` (FIFO ready + ngân sách nhường event) |
| | `sched/frame_su_kien.rs` | **216** | ngân sách ready, deliverable, `frame_fire`, `frame_wake_one` |
| | `sched/frame_ket_thuc.rs` | **195** | `frame_stuck_trap`, `frame_wake_chan`, task done, drain, final join, reclaim |
| | `sched/frame_huy.rs` | **215** | huỷ hợp tác (request/process/one + đếm byte) |
| | `sched/frame_reactor.rs` | **279** | reactor giả: load/poll/fire/idle/shutdown |

**API giữ nguyên**: `rt::Scheduler`, `rt::FRAME_CANCEL_*` (re-export `pub(crate)` — `rt.rs` và `frame_test.rs` không đổi), mọi method `pub`/`pub(crate)` giữ đúng tên/chữ ký. Chỉ nới visibility nội bộ (`fn → pub(super)`/`pub(crate)`) cho các method gọi chéo tệp: `frame_fire`, `frame_wake_one`, `frame_after_ready_budget`, `wake_reactor`, `frame_dem_byte_va_yeu_cau_huy`.

### 1c. `crates/vis-types/src/tyck/pat.rs` → `crates/vis-types/src/tyck/pat/`

| Trước | Sau | Dòng | Trách nhiệm |
|---|---|---|---|
| `pat.rs` | **1140** | — | — |
| | `pat/mod.rs` | **21** | doc + khai báo + `pub(crate) use sym::{arm_uses, than_uses}` |
| | `pat/gan.rs` | **321** | `bind_pat` + `kiem_ctor_args` (gán kiểu binding) |
| | `pat/xet_het.rs` | **169** | `xet_het` (match xét hết theo kiểu chủ thể) |
| | `pat/phu.rs` | **98** | helper phủ dùng chung: `phu_tat_ca`, `cap_dau`, `ctor_names`, `bool_names`, `record_cov`, `tuple_phu_het`, `bat_kha_bac` |
| | `pat/or_bind.rs` | **204** | luật or-pattern `VS0412` + `BindTypes`/`pat_children`/`pat_binds_typed`/`ctor_field_types` |
| | `pat/sym.rs` | **210** | duyệt `sym` biểu thức/thân (`expr_syms` … `than_uses`, `arm_uses`) |
| | `pat/phu_het.rs` | **177** | phủ hết cho `let`/`for` (`kiem_let_phu_het`, `kiem_for_phu_het`, `pat_phu_het`, `or_phu_het`, `phu_bien_the`) |

**API giữ nguyên**: `crate::tyck::pat::than_uses`, `arm_uses` (dùng ở `expr.rs`, `mod.rs`, `phuong_thuc.rs` — **không sửa**); mọi method `pub(crate)` (`bind_pat`, `xet_het`, `kiem_or_bind`, `kiem_let_phu_het`, `kiem_for_phu_het`) giữ tên/chữ ký; `BindTypes` là type nội bộ.

### 1d. Tổng

| Tệp gốc | Trước | Sau (max tệp con) | Hệ số |
|---|---|---|---|
| `vis-pkg/src/pkg.rs` | 1151 | 377 | 3,1× |
| `vis-runtime/src/rt/sched.rs` | 1189 | 279 | 4,3× |
| `vis-types/src/tyck/pat.rs` | 1140 | 321 | 3,6× |

Mọi tệp sản phẩm mới **< 400 dòng** (mục tiêu < 1.000, trần cứng 1.500). `mod.rs` chỉ doc + `mod` + re-export. 0 `unsafe`, SPDX đầy đủ ở mọi tệp mới.

---

## 2. Test chạy (số thật)

| Cổng | Kết quả | Log + sha256 |
|---|---|---|
| `cargo test -p vis-pkg` | 27 passed (lib) + `sig_it` như cũ | — |
| `cargo test -p vis-runtime` | 288 passed | — |
| `cargo test -p vis-types` | 221 passed + 2 doc-test | — |
| `./vis-dev check` | **PASS** | `/home/ps/vislogs/st317b-check.log` · `989d8aba…780096` |
| `./vis-dev gate --fast` | **PASS** — nextest `3880/3880 passed, 4 skipped` | `/home/ps/vislogs/st317b-gate.log` · `c5322284…63287d` |
| `./vis-dev rules` | **OK** R1..R9 — R8 **38+1 = 39 ≤ 40**, `vis-cli/tests` 8 ≤ 15 | `/home/ps/vislogs/st317b-rules.log` · `812aacb2…793414` |
| `./bin/vis-dag-check` | **OK** — 23 crate / 5 tầng / 0 cạnh cấm | `/home/ps/vislogs/st317b-dag.log` · `5ee0d7f4…c4af4d` |
| `VIS_DIFF_N=100000 ./bin/vis-diff` | **`lech=0`** — 100.000 chương trình, 106,2 s, 941/s | `/home/ps/vislogs/st317b-diff.log` · `3fd0463e…155beb` |

Không tạo tệp test cấp 1 mới, không `#[ignore]` mới, không sửa test. Test hiện có chạy nguyên chỗ (cùng module path logic, chỉ đổi tệp vật lý).

**Lưu ý dòng lệnh**: máy có `/usr/bin/cargo` 1.93.1 (không đạt MSRV); mọi lệnh dùng `PATH="$HOME/.cargo/bin:$PATH"` (cargo/rustc 1.100.0). `./vis-dev` tự chọn đúng.

---

## 3. Commit

| Commit | Nội dung |
|---|---|
| `f247567c` | `[ST-317b]` tách `pkg.rs` 1151 → mod 54 + 5 tệp |
| `987c5585` | `[ST-317b]` tách `rt/sched.rs` 1189 → mod 22 + 8 tệp |
| `7ae2f96e` | `[ST-317b]` tách `tyck/pat.rs` 1140 → mod 21 + 6 tệp |
| (commit checkpoint) | journal + `STATE.md` |

---

## 4. Worker orchestration — **THIẾT KẾ + TEST ĐẶC TẢ** (chưa move code; **chờ `ST-317a`**)

### 4.1 Hiện trạng & trùng lặp thật

| Mảnh logic | `vis-interp/src/rt/exec.rs` (461) | `vis-vm/src/vm/worker.rs` (586) | Trạng thái |
|---|---|---|---|
| `Proxy`/task↔pool | `vis-runtime/src/rt/exec_state.rs` (`TaskProxy`, `ExecState`, `PendingJob`) | `struct Proxy` **riêng** trong `worker.rs` | **đã tách một nửa** |
| tạo pool + board | `ExecState::ensure_pool` (runtime) | `ensure_pool(w: &mut WorkerState)` | trùng ý tưởng |
| `apply_outcome` | `exec.rs:189` | `worker.rs:231` | **gần bản sao** (~60 dòng) |
| `flush_proxy` (+`VS0509`/`VS0506`) | `exec.rs:216` | `worker.rs:309` | **gần bản sao** (~40 dòng) |
| `pump` (board 50 ms) | `exec.rs:256` | `worker.rs:255` | **gần bản sao** (~40 dòng) |
| `in_flight`/`is_worker_task`/`cancel_worker` | `exec.rs:297–314` | `worker.rs:291–306` | **bản sao** |
| `await` (frame suspend + pump loop + flush) | `exec.rs:315` | `worker.rs:348` | **bản sao** (~50 dòng) |
| `final_join` (join + outcome + flush theo spawn order) | `exec.rs:367` | `worker.rs:396` | **gần bản sao** (~90 dòng) |
| `code_cross`/`loi_task` + message `VS0509`/`VS0506` | `exec.rs:174–188` | `worker.rs:213–228` | **bản sao** (literal) |
| runner thân (interp cây vs VM module) | `chay_than_worker` | `chay_than_vm` | **khác bản chất — giữ riêng** |
| state máy engine (globals, `Value`/`SendValue`) | `interp` | `vm` | **giữ riêng** |

Ước lượng sau gom: `exec.rs` ~460 → ~180; `vm/worker.rs` ~586 → ~330; `vis-runtime/src/worker/` thêm 1 tệp `orchestrate.rs` ~260 (hoặc 2 tệp < 180).

### 4.2 Thiết kế API tối thiểu (đề xuất cho lượt sau)

Đặt tại `crates/vis-runtime/src/worker/orchestrate.rs`, **thuần**, không biết engine:

```rust
/// Kết quả một job đã xong, đã chuẩn hoá khỏi TaskOutcome của pool.
pub struct KetQuaJob { pub output: String, pub dirty: Vec<String>, pub cancelled: bool,
                       pub ket: Result<Value, Trap> }   // Trap đã bọc VS0506 bởi caller

/// Áp 1 outcome vào proxy + cache Task + TrangThai::Xong; KHÔNG flush.
/// Caller truyền `frame_task_done: Option<&mut dyn FnMut(u32, Result<Value, Trap>)>`
/// để engine tự gọi scheduler của mình.
pub fn ap_dung_outcome<S: ProxyStore>(store: &mut S, o: TaskOutcome,
                                      bao_loi: &dyn Fn(&Task, String) -> Trap,
                                      done: &mut dyn FnMut(u32, Result<Value, Trap>)) -> Option<u32>;

/// Rút output theo đúng một lần; trả lỗi VS0509 (dirty) / VS0506 (cancelled) + output.
pub fn flush_proxy<S: ProxyStore>(store: &mut S, id: u32, span: Span) -> Result<String, Trap>;

/// Một vòng board: `block` ⇒ chờ tối đa 50 ms; áp mọi outcome sẵn sàng. Trả số outcome.
pub fn pump<S: ProxyStore>(store: &mut S, block: bool, ...) -> Result<usize, Trap>;

pub trait ProxyStore {  // hiện thực bởi ExecState (interp) và WorkerState (VM)
    fn proxy(&self, id: u32) -> Option<&TaskProxyMeta>;
    fn proxy_mut(&mut self, id: u32) -> Option<&mut TaskProxyMeta>;
    fn board(&self) -> Option<Arc<OutcomeBoard>>;
    fn pool_id_to_task(&self) -> &HashMap<TaskId, u32>;
}
```

Nguyên tắc:
1. **`TaskProxyMeta` một nguồn** (task, pool_id, handle, flushed, done, output, dirty, cancelled) trong `worker/proto.rs` (hoặc `worker/proxy.rs` mới) — thay `struct Proxy` riêng của VM.
2. **Message + mã lỗi một nguồn** (`VS0509` ghi chéo, `VS0506` huỷ, `VS0508` không Sendable) — hiện literal lặp ở 2 tệp.
3. **Engine giữ**: runner thân (`chay_than_worker`/`chay_than_vm`), cách đọc/ghi globals, `Host`/`Vm::out`, suspend frame (interp) vs block pump (VM), budget charge.
4. **`final_join`**: hàm chung nhận iterator `(pool, proxies, map)` + callback flush; engine chỉ khác nguồn `output`.
5. Không đổi `Value::Task(Rc<Task>)`, không đổi `WorkerPool`/`SendValue`/`TaskId` công khai.

### 4.3 Vì sao chưa move trong lượt này

- `ST-317a` đang đổi `Value` (`Cap`/`Record`/`Enum` boxing, `Rc<VisRecord>`) và **5 điểm trong `worker/proto.rs`** (`to_value`/`from_value`/`to_send`/`ProxyValue`); `frame/worker.rs` + `vm/worker.rs` dựng/so `Value::Unit`/`Value::Task`/`Value::Str` và `SendValue::to_value`.
- Move code bây giờ = sửa cùng dòng với 317a ở 2 nhánh ⇒ xung đột cơ học chắc chắn, không có giá trị kiến trúc.
- Đã **chốt thiết kế + danh sách test** để lượt sau chỉ việc hiện thực.

### 4.4 Test đặc tả (viết khi `317a` hợp nhất; 0 binary R8 mới)

Tất cả đặt trong aggregator đã có (gợi ý `crates/vis-interp/tests/frame_it.rs` đã khai báo `#[path]` hoặc tệp con `tests/frame_it/worker_parity.rs`), **không** tạo `.rs` cấp 1:

| # | Ca | Kỳ vọng |
|---|---|---|
| W1 | spawn 2 task song song, output xen kẽ | output merge **theo spawn order**, không theo thời điểm xong |
| W2 | task ghi biến global giữa 2 worker | `VS0509` (ghi chéo), message nêu tên biến + tên task |
| W3 | task bị huỷ trước khi await | `VS0506` "tác vụ bị huỷ"; `Task.trang_thai == Xong`, `kq = Err` |
| W4 | task trả `Value` chứa `Rc`/`RefCell` (không Sendable) | `VS0508` tại **spawn** (trước khi tạo job) và tại **kết quả** |
| W5 | await cùng một task 2 lần | output flush **đúng một lần** (proxy `flushed`), kết quả cache ổn định |
| W6 | final join có 1 task lỗi không ai await | trap `VS0506` surfaced từ `final_join` (không exit 0) |
| W7 | frame mode: await task worker khi chưa xong | trả `Pending`, không block; `frame_next` wake khi outcome tới |
| W8 | non-frame `Off`/`Local` | đường cũ `spawn`/`join_all_off` không đổi; 0 job lên pool |
| W9 | 2 engine cùng kịch bản (interp vs VM) | cùng stdout + exit + mã lỗi (parity) |
| W10 | huỷ idempotent: cancel 2 lần + cancel task đã xong | không panic, không nhân đôi log/outcome |

Điều kiện nghiệm thu lượt sau: `gate --fast` PASS, `vis-diff 100k lech=0`, `frame_it` + `worker_leak_it` xanh, **R8 vẫn 39/40**.

---

## 5. Việc còn lại (đã đăng ký, không tự ý mở)

| Việc | Điều kiện mở | Ghi chú |
|---|---|---|
| Tách `vis-host-native/src/db/sqlite/mod.rs` (1.130) | sau khi `ST-317a` hợp nhất (4 điểm `Value::Enum` trong tệp) | theo đúng lane điều chỉnh |
| Tách `crates/vis-core/src/value/mod.rs` | sau `ST-317a` (chính nó) | A0 báo |
| Tách `crates/vis-lsp/src/lib.rs` | sau `ST-317c` | A0 báo |
| Hiện thực gom worker orchestration (`worker/orchestrate.rs`) + 10 test §4.4 | sau `ST-317a` | thiết kế đã chốt trong tệp này |
| `vis-interp` sync↔frame khử trùng lặp (phần còn lại của mô tả `ST-317b`) | sau `ST-317a` | lượt này mới làm phần tách tệp + worker design; `vis-interp/src/{interp,frame}/**` chưa bị đụng vì trùng vùng `Value` |

---

# Phần 2a — tách `crates/vis-lsp/src/lib.rs` (mở khoá sau `ST-317c` PASS, `main` @ `aecb266c`)

- **Nhánh** `plan/st317b` đã `git merge --ff-only main` ⇒ nền `aecb266c` (gồm ST-317b phần 1 `e1fe1733` + ST-317c `53df9090` PASS + ST-316c prep).
- **Phạm vi**: chỉ `crates/vis-lsp/src/lib.rs` → `server/`. **Không sửa** `analysis/host.rs` (317c vừa audit PASS), không đổi wire protocol/JSON, không đổi `vis_lsp::run`/`AnalysisHost`/`ThongKeCache`.

## 2a.1 LOC trước/sau (`wc -l` thật)

| Trước | Sau | Dòng | Trách nhiệm |
|---|---|---|---|
| `vis-lsp/src/lib.rs` | **1056** | — | — |
| | `src/lib.rs` | **236** | doc + `#![forbid(unsafe_code)]` + khai báo `mod` + `pub use host::{AnalysisHost, ThongKeCache}` + `pub use server::run` + re-export `pub(crate) uri_thanh_duong/duong_thanh_uri/mode_cua` + test URI `vd258_tests` (3 test cũ) |
| | `src/server/mod.rs` | **11** | khai báo + `pub use run::run` |
| | `src/server/run.rs` | **651** | `run`: cờ `--stdio`, vòng lặp frame stdin, dispatch JSON-RPC (initialize/initialized/shutdown/exit + 24 method `textDocument/*`/`workspace/*`), publish chẩn đoán phụ thuộc |
| | `src/server/khoi_tao.rs` | **60** | `ket_qua_khoi_tao` — capabilities cố định (snapshot tất định) |
| | `src/server/uri.rs` | **143** | `uri_thanh_duong(_voi)`, `duong_thanh_uri`, `percent_giai_ma`, `mode_cua` (giữ nguyên luật ST-80/ST-297/VD-258) |

`lib.rs` 1.056 → **236** (< 500, mục tiêu 500–800 cho tệp lớn nhất: `server/run.rs` 651). Mọi tệp < 1.000, R1 ≤ 1.500.

**API giữ nguyên**: `vis_lsp::run` (re-export từ `server::run`, cùng chữ ký `fn(&[String]) -> ExitCode`), `pub use host::{AnalysisHost, ThongKeCache}`, các module `protocol/features/analysis` giữ nguyên `#[path]` + tên phẳng. `analysis/*` dùng `super::uri_thanh_duong`/`super::duong_thanh_uri`/`super::mode_cua` vẫn chạy qua re-export `pub(crate)` ở `lib.rs`. **0 dòng logic bị sửa** — move nguyên văn, chỉ thêm `use`/`pub(crate)`/`pub(crate) mod uri`.

## 2a.2 Bằng chứng test/cổng

| Cổng | Kết quả | Log + sha256 |
|---|---|---|
| `cargo test -p vis-lsp --lib` | **29 passed**, 0 failed | — |
| `cargo test -p vis-lsp --test lsp_it` | **158/158 passed**, 0 failed | — |
| `./vis-dev gate --fast` | **PASS** — nextest **3885/3885 passed, 4 skipped** (51,2 s) | `/home/ps/vislogs/st317b-p2a-gate.log` · `0e67dde2…984f3` |
| `./vis-dev rules` R1..R9 | **OK** — R8 **38 + 1 = 39/40**, `vis-cli/tests` 8/15; R1 mọi tệp ≤ 1.500 | `/home/ps/vislogs/st317b-p2a-rules.log` · `812aacb2…793414` |
| `./bin/vis-dag-check` | **OK** — 23 crate / 5 tầng / 0 cạnh cấm | `/home/ps/vislogs/st317b-p2a-dag.log` · `5ee0d7f4…c4af4d` |
| `vis-diff 100k` | **không chạy** — thuần move code LSP, 0 đổi hành vi/wire protocol (theo chỉ đạo) | — |

**Commit**: `[ST-317b] tach vis-lsp/lib.rs …` (cùng commit với mục này).

## 2a.3 Còn lại (chờ `ST-317a`)

| Việc | Điều kiện |
|---|---|
| `crates/vis-core/src/value/mod.rs` (1.0xx dòng) | `ST-317a` hợp nhất |
| `crates/vis-host-native/src/db/sqlite/mod.rs` (1.130) | `ST-317a` hợp nhất (4 điểm `Value::Enum` trong tệp) |
| `crates/vis-runtime/src/worker/proto.rs` + worker orchestration (`worker/orchestrate.rs`, 10 test W1..W10 §4.4) | `ST-317a` hợp nhất |
| Khử trùng lặp `vis-interp/src/{interp,frame}/**` | `ST-317a` hợp nhất |

---

# Phần 2b — mở khoá toàn bộ sau `ST-317a` PASS (`main` @ `426b83a3`)

- **Nền**: `plan/st317b` fast-forward lên `426b83a3` (317a Value 32 B merged). Không chạm lại `worker/proto.rs` trừ `codes.rs`/orchestrate (xem §2b.3).
- **Thứ tự thi công**: value → sqlite → worker orchestration → dedup interp → cổng cuối.

## 2b.1 `crates/vis-core/src/value/mod.rs` — 1.415 → **748**

| Tệp | Dòng | Trách nhiệm |
|---|---:|---|
| `value/mod.rs` | **748** | `enum Value` + payload (`VisRecord`/`EnumArgs`/`VisEnum`/`CapVal`/`VmFunc`), constructor/public API, `Env`/`Closure`, `truthy`, `exit_code/show/eq/cmp`, test layout |
| `value/lit.rs` | **403** | hiển thị/đọc literal (`show_*`, `parse_*`, `pct_hop_le`, `PCT_GIOI_I64`, escape) + `tests_show_char` |
| `value/ten.rs` | **291** | `Fnv`, `Name` + bảng tên thread-local + cache, `Tags`, singleton `None`/map rỗng + `tests_name` |

- Re-export giữ API: `value::{Name, pct_hop_le, parse_*, PCT_GIOI_I64}`; `show_eq.rs` vẫn dùng helper qua glob `super::*` (`pub(crate) use lit::{show_*}`); `map.rs`/`show_eq.rs` dùng `Fnv` qua `pub(crate) use ten::Fnv`.
- **`size_of::<Value>() == 32` vẫn xanh** (`tests_value_32`), `cargo test -p vis-core` 35 test pass. Commit **`3dd51026`**.

## 2b.2 `crates/vis-host-native/src/db/sqlite/mod.rs` — 1.132 → **282**

| Tệp | Dòng | Trách nhiệm |
|---|---:|---|
| `sqlite/mod.rs` | **282** | doc hợp đồng + `MAX_*` + `GioiHan`/`Muc` + `Db` + khai báo/re-export |
| `sqlite/host.rs` | **261** | `impl DbHost for Db` (open/close/execute/query/batch/metadata) |
| `sqlite/truy_van.rs` | **309** | kiểm/chuyển param, `Lenh`/cache statement, `thuc_thi`, `truy_van`, cap hàng/ô/cột |
| `sqlite/token.rs` | **78** | token `db:<32 hex>` + nguồn OS/fallback |
| `sqlite/chuyen_doi.rs` | **90** | ô↔`Value`, param `SqlValue`→rusqlite, `SQLITE_LIMIT_LENGTH` |
| `sqlite/loi.rs` | **106** | map lỗi SQLite (không lộ raw SQL/bound/path) |
| `sqlite/guard.rs` | **67** | `DbGuard` (chặn file-backed khi có cap) |

- `Db`/`DbGuard`/`MAX_*` vẫn qua `pub use sqlite::*`; `schema.rs` chỉ đổi 1 dòng import (`super::loi::loi_sqlite`); test cũ `use super::*` chạy nguyên (thêm `#[cfg(test)] use` cho tên chỉ test dùng). `cargo test -p vis-host-native` 128 test pass. Commit **`7a33d996`**.

## 2b.3 Worker orchestration — **đã move dùng chung + 10 test W1..W10**

`crates/vis-runtime/src/worker/orchestrate.rs` (**135 dòng**) — một nguồn:
`ma_ghi_cheo()` (nay `codes::CROSS` = `RUNTIME_CODES[8]`), `loi_task` (`VS0506` bọc mã gốc), `ap_dung_outcome`
(proxy + cache `Task` + `TrangThai::Xong`, trả `(main_id, ket)`), `flush_proxy` (output đúng một lần;
`VS0509`/`VS0506`), `ap_va_flush` (final join), `rut_outcome` + `pump` (board 50 ms, closure áp outcome —
**không** giữ borrow `RefCell` qua lúc chờ).

| Tệp | Trước | Sau | Ghi chú |
|---|---:|---:|---|
| `vis-interp/src/rt/exec.rs` | 461 | **368** | `apply_outcome`/`flush_proxy`/`pump`/`final_join` gọi nguồn chung; xoá `code_cross`/`loi_task` cục bộ |
| `vis-vm/src/vm/worker.rs` | 586 | **485** | xoá `struct Proxy` (dùng `TaskProxy` chung), `apply_outcome`/`flush_proxy`/`pump`/`final_join` gọi nguồn chung |
| `vis-runtime/src/codes.rs` | — | +3 | thêm `pub const CROSS` |

**10 test đặc tả W1..W10** tại `crates/vis-conformance/tests/async_api_it/st317b_worker_orchestrate.rs` (405 dòng), khai `#[path]` trong `lib_api_it.rs` (**0 binary R8 mới**):

| # | Ca | Kết quả |
|---|---|---|
| W1 | output nhiều task theo spawn order + parity interp/VM W=2/4 | PASS |
| W2 | ghi top-level W>1 ⇒ `VS0509` cả 2 engine; W=1 đúng | PASS |
| W3 | huỷ giữa chừng (`cancel_after`) ⇒ `VS0506` cả 2 engine | PASS |
| W4 | **program-level** `VS0508`: đối số là giá trị hàm (spawn) + kết quả task là hàm (await) | PASS |
| W5 | await cùng task 2 lần ⇒ chạy/flush một lần, cache ổn định | PASS |
| W6 | **lỗi thân task không await** (`%0`): W=1 surface cả 2 engine; W≥2 parity hiện hữu (exit 0 — khoá không hồi quy) | PASS |
| W7 | **frame mode** thật: `run_async_workers_with_hosts_caps` W=2/4 suspend/resume, byte-identical W=1, `steps>0` | PASS |
| W8 | `Local`/`Off` giữ đường cũ (cùng stdout/exit) | PASS |
| W9 | parity interp↔VM W=1/2/4 (stdout/exit/codes/giá trị cuối) | PASS |
| W10 | **cancel idempotent trên cùng handle**: 2 lần sau khi xong (giữ kết quả) + 2 lần trước join (một outcome `cancelled`) | PASS |

Commit **`9fc06efb`**. *Ghi nhận*: W6 cho thấy `run_workers_parallel*` W≥2 **không** surface lỗi task không ai await ở **cả hai** engine (exit 0) — hành vi có trước, test khoá parity; nợ riêng, không sửa trong lát này.

## 2b.4 Dedup interp

- `opt_depth_ty`: **một nguồn** `interp::goi::opt_depth_ty` (`pub(crate)`); `frame/params.rs` re-export `pub(super)` cho `frame.rs` — xoá bản sao.
- `call_value`: thân khác bản chất (frame dựng `Work` vs interp trả `Value`) nên **không** gộp thân; gom phần dùng chung thật: `loi_arity` + `loi_khong_phai_ham` là **free fn `pub(crate)`** trong `interp/goi_ham.rs`, re-export qua `goi` (đúng chỗ plan cảnh báo), dùng ở cả hai đường — message/span không đổi.
- `thu_duong`: **đã một nguồn từ trước** (`interp::gan_long::thu_duong`); `frame/expr.rs` chỉ là wrapper chuyển `HExpr` → `NutGan` (env semantics khác) — **không gộp thêm** theo plan.

Commit **`a92fdb2f`**.

## 2b.5 Cổng cuối (số thật)

| Cổng | Kết quả | Log + sha256 |
|---|---|---|
| `./vis-dev check` | **PASS** | `st317b-p2b-check-final.log` · `0e64c7dd…c2b0` |
| `./vis-dev gate --fast` | **PASS** — nextest **3901/3901**, 4 skipped (41,1 s) | `st317b-p2b-gate-final.log` · `ec9a3719…665b` |
| `VIS_DIFF_N=100000 ./bin/vis-diff` | **`lech=0`** — 100.000 chương trình, 56,5 s, 1.771/s | `st317b-p2b-diff.log` · `050f34a0…29f5` |
| `./vis-dev rules` | **OK** — R1..R9, **R8 = 38+1 = 39/40** | `st317b-p2b-rules.log` · `812aacb2…3414` |
| `vis-dag-check` | **OK** — 23 crate / 5 tầng / 0 cạnh cấm | `st317b-p2b-dag.log` · `5ee0d7f4…af4d` |

## 2b.6 Còn lại

- `ST-317b` **hoàn tất phần tách tệp + worker + dedup** trong phạm vi Sóng 6. Không còn tệp sản phẩm > 1.100 LOC thuộc lane W2.
- Nợ ghi nhận (ngoài phạm vi): (a) W≥2 không surface lỗi task không await (parity interp/VM); (b) `thu_duong` wrapper frame vẫn cần vì env semantics khác.
- `Worktree`/nhánh dọn theo `AGENTS.md` §6 sau khi A0 duyệt `DONE`.
