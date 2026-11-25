# Đề xuất đồng bộ `docs/SPEC.md` + `docs/ViScript-thiet-ke.md` (2026-11-25, low-peak)

> **Từ**: Agent 2 · **Cho**: Agent 0 · **Trạng thái**: đề xuất, **chưa sửa** hai tệp này.
> **Lý do không sửa trực tiếp**: `AGENTS.md` §1 — `SPEC.md` chỉ sửa qua Agent 0; tài liệu thiết kế
> cũng đang do A0 đồng bộ. Các dòng dưới đây đã đối chiếu cây thật tại `aaf4fa83` (docs không đổi
> từ `d8857694`). Đề nghị A0 áp dụng/duyệt.

## 1. `docs/SPEC.md`

| Dòng | Hiện tại | Đề xuất |
|---|---|---|
| 609 | `Mô hình hiện tại **một luồng, run-to-completion**` | `Mô hình mặc định một luồng; chương trình `async` dùng runtime frame suspend/resume + reactor/timer` |
| 611 | `không có \`select\`/\`timeout\`;` | `không có \`select\` tổng quát; kênh có \`recv_timeout\`/\`send_timeout\` + \`select_recv*\`/\`select_send*\`;` |
| 617 | `\`chan(0)\` bị từ chối ...` | `\`chan(0)\` bị từ chối ở đường thường; đường frame hỗ trợ rendezvous (xem §12/thiết kế)` |
| 618 | §5.2 chỉ liệt kê `send`/`recv`/`close` | bổ sung `recv_timeout`/`send_timeout`/`select_recv*`/`select_send*` (khớp §611/§1031) |
| 683 | ô macOS kết thúc `… host 54/54); chỉ arm64` | thêm `; clippy **PASS @311d2288**; **chưa chạy lại full gate trên \`0.6.0\`** (mps bỏ qua theo chỉ đạo)` |
| 930 | `\`vis 0.5.0\` hoặc \`{"name":"vis","version":"0.5.0"}\`` | `\`vis 0.6.0\` hoặc \`{"name":"vis","version":"0.6.0"}\`` |
| 1016 | `(VD/R1..R24)` | `(VD-01..VD-290, R1..R37)` |
| 1022 | `đã kiểm tra tại \`c4337ce\`:` | **cân nhắc**: chỉ đổi thành `90fd24de` nếu A0 đã thực sự rà lại §12 tại `0.6.0`; nếu §12 vẫn là mốc kiểm tra gốc thì **giữ nguyên**. |
| 1031 | `Không có \`select\`/\`timeout\`/cancel cây` | `Có timeout kênh + \`select_recv*\`/\`select_send*\`; không có \`select\` tổng quát/cancel cây` |
| 1032 | `\`chan(0)\` rendezvous | Bị từ chối \`VS0505\`` | `\`chan(0)\`: đường thường \`VS0505\`; đường frame hỗ trợ rendezvous` |

## 2. `docs/ViScript-thiet-ke.md`

| Dòng | Hiện tại | Đề xuất |
|---|---|---|
| 11 | link `../QUYET-DINH-sau-review.md`; `docs/viscript/SPEC-lex.md` | `` `docs/QUYET-DINH-sau-review.md` (repo ViFastOs) ``; `docs/SPEC-lex.md` |
| 852 | `vis-host-posix`, `vis-host-windows`, `vis-host-wasm`, `vis-host-vifast`, `vis-host-mock` | `vis-host-native` (Linux/macOS/Windows), `vis-host-linux` (Landlock), `vis-host-wasm` (workspace riêng, `wasm32-wasip1`); `vis-host-vifast` ở repo ViFastOs |
| 925 | `Tầng 0 (nền tảng thuần, 0 OS, 0 unsafe)` | `Tầng 0 (nền tảng thuần, 0 unsafe; riêng vish-term có dep termios/crossterm theo target)` |
| 932 | `viscript/ chỉ chứa crate thuần (lõi + 3 backend còn lại + mock)` | `repo này chứa lõi/thư viện + \`vis-host-native\`/\`vis-host-linux\` + workspace riêng \`vis-host-wasm\`; \`vish-term\` có dep OS theo target` |
| 935 | `unsafe chỉ trong danh sách crate đóng (vis-vm cho heap, vis-plugin cho ABI)` | `unsafe bị cấm theo R2 (danh sách đóng rỗng, 0 unsafe tại 0.6.0); attribute \`#![forbid(unsafe_code)]\` mới có ở 11/23 crate — không tuyên bố attribute toàn workspace` |
| 1081 | `lập lịch tất định theo seed trong vis-host-mock` | `lập lịch tất định theo trace/seq trong \`crates/vis-runtime/src/rt/frame_test.rs\` + \`crates/vis-conformance/tests/async_api_it\`; chỉ \`lat257_fairness\` dùng seed` |
| 1097 | `L0 crate bị chạm` | `fmt, clippy, R1..R9, **DAG 5 tầng + self-test**, toolchain, release, fixtures, report-check, build, doc-examples, readme-examples (không chạy L0)` |
| 1098 | `fmt, clippy, build wasm32 cho crate lõi, …, L0, L1` | `vis-dev check + L0 test (nextest + doc-test); không build wasm32, chưa có L1` |
| 1099 | `vis-dev gate \| + L2` | `vis-dev gate \| chỉ nhận gate --fast; không có full gate/L2 (thiếu --fast ⇒ exit 2)` |
| 1100 | `L4 + benchmark + báo cáo` | `gate --fast + fuzz parser (--fuzz-minutes, mặc định 2) + vòng tròn fmt; không L4/benchmark` |
| 1104 | `docs/viscript/doc-examples-allow.txt` | `docs/doc-examples-allow.txt` |
| 1113, 1133, 1137 | `viscript/agent/QUEUE.md`, `viscript/agent/` | `agent/QUEUE.md`, `agent/` |
| 1222 | `docs/viscript/SPEC-lex.md` | `docs/SPEC-lex.md` |
| 1614 | `crates/vis-cli/src/pkg.rs` | `crates/vis-pkg/src/pkg.rs`; `vis-cli` dùng `pub(crate) use vis_pkg::pkg` (không phải re-export công khai) |
| 1641 | `Mục này mô tả ngữ nghĩa đã hiện thực, không chứa số đo.` | **CONDITIONAL**: câu này vẫn đúng nửa đầu, nhưng §1653-1655 lại chứa 54/54, 18/18, 390/390, 28/28, 167. Thống nhất: hoặc bỏ số khỏi mục này, hoặc đổi thành `Mục này mô tả ngữ nghĩa đã hiện thực; số đo chi tiết nằm trong agent/reports + JOURNAL (chỉ giữ tại chỗ mốc kiểm chứng tối thiểu).` |
| 914–915 | `vis-host`, `vis-host-mock`, `vis-host-{posix,windows,wasm}` | `vis-host-native`, `vis-host-linux`, `vis-host-wasm` (workspace riêng); `Host` ở `crates/vis-runtime/src/builtin/mod.rs:38`, mock FS thật ở `crates/vis-host-native/src/fsmock.rs`, frame test dùng fake reactor event (`rt/frame_test.rs`) |
| 1063, 1080 | `L4 ... Miri` / `Miri` | bỏ/tách: `vis-dev nightly` không có L4/Miri; Miri chưa cấu hình trong repo |
| 1115, 1117 | `vis-host-posix`/`vis-host-windows` | `vis-host-native` (Linux/macOS/Windows) |
| 23 | `vis-host-vifast` ngoài `viscript/` | `vis-host-vifast` ở repo ViFastOs (bỏ tiền tố `viscript/` đã không còn) |
| 1145 | `Workspace \`viscript/\` + \`viscript/agent/\`` | `repo ViScript (gốc) + \`agent/\`` |
| 1574 | `(nợ **#142**)` | `#142 đã đóng ở ST-288 (xem :1637)` |
| 1653 | `nợ chạy bù clippy Sóng 2–3` | `clippy macOS **PASS @311d2288**; chỉ full gate bị bỏ qua theo chỉ đạo` |

## 3. Đã kiểm tra là **khớp** (không cần sửa)

- 23 workspace member / 5 tầng DAG; `vis-hir`, `vis-http`, `vis-runtime`, `vis-conformance`,
  `vis-host-linux`; `vis-host-wasm` workspace riêng.
- `StdId` 167, `stdlib/` chia miền (0 tệp `_a.._e`), R8 **39 (38+1) ≤ 40**.
- WASI **390/390**, suite **28/28**, conformance **299×2**; `#193`/`#194`/`#196` đóng.
- 23 crate / 5 tầng; không còn “12 crate”, “chưa công bố Tier 1”. **Lưu ý**: `vis-cli::net` **vẫn còn**
  như **re-export phẳng** `pub use vis_host_native::net` (`crates/vis-cli/src/lib.rs:15`); HTTP/1.1
  sans-IO ở `vis-http`, còn HTTP/2 + keep-alive vẫn ở `vis-host-native`; `vis-core` không còn protocol.

**Đã kiểm chứng lại tại `ae9600a6`**: `./vis version` → `vis 0.6.0`; `./vis version --json` →
`{"name":"vis","version":"0.6.0"}`; `recv_timeout`/`send_timeout`/`select_recv*`/`select_send*` có
trong `crates/vis-types/src/tyck/expr/kenh.rs:36,58,64,83-85`; báo cáo mới nhất `R37`, quyết định
mới nhất `VD-290`; `docs/doc-examples-allow.txt` tồn tại (không phải `docs/viscript/...`).

## 4. Ghi chú

- Các dòng `0.4.5` trong `docs/SPEC.md`/`HOST-BAO-DAM.md` là **mốc lịch sử Windows Tier 1**, giữ nguyên.
- `agent/HOST-BAO-DAM.md` đã được Agent 2 cập nhật trực tiếp ở low-peak (ma trận 23 crate/5 tầng,
  47 ô/cấu hình, số đo 3.730/0/4, 390/390, 28/28, 299×2) — không nằm trong phạm vi A0-only.
