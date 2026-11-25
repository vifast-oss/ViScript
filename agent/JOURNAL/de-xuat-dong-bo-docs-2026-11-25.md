# Đề xuất đồng bộ `docs/SPEC.md` + `docs/ViScript-thiet-ke.md` (2026-11-25, low-peak)

> **Từ**: Agent 2 · **Cho**: Agent 0 · **Trạng thái**: đề xuất, **chưa sửa** hai tệp này.
> **Lý do không sửa trực tiếp**: `AGENTS.md` §1 — `SPEC.md` chỉ sửa qua Agent 0; tài liệu thiết kế
> cũng đang do A0 đồng bộ. Các dòng dưới đây đã đối chiếu cây thật tại `aaf4fa83` (docs không đổi
> từ `d8857694`). Đề nghị A0 áp dụng/duyệt.

## 1. `docs/SPEC.md`

| Dòng | Hiện tại | Đề xuất |
|---|---|---|
| 611 | `không có \`select\`/\`timeout\`;` | `không có \`select\` tổng quát; kênh có \`recv_timeout\`/\`send_timeout\` + \`select_recv*\`/\`select_send*\`;` |
| 683 | ô macOS kết thúc `… host 54/54); chỉ arm64` | thêm `; **chưa chạy lại full/clippy trên \`0.6.0\`** (\`mps\` bỏ qua 2026-11-25)` |
| 930 | `\`vis 0.5.0\` hoặc \`{"name":"vis","version":"0.5.0"}\`` | `\`vis 0.6.0\` hoặc \`{"name":"vis","version":"0.6.0"}\`` |
| 1016 | `(VD/R1..R24)` | `(VD-01..VD-290, R1..R37)` |
| 1022 | `đã kiểm tra tại \`c4337ce\`:` | **cân nhắc**: chỉ đổi thành `90fd24de` nếu A0 đã thực sự rà lại §12 tại `0.6.0`; nếu §12 vẫn là mốc kiểm tra gốc thì **giữ nguyên** (header ghi bản gốc sinh từ `c4337ce`). |
| 1031 | `Không có \`select\`/\`timeout\`/cancel cây` | `Có timeout kênh + \`select_recv*\`/\`select_send*\`; không có \`select\` tổng quát/cancel cây` |

## 2. `docs/ViScript-thiet-ke.md`

| Dòng | Hiện tại | Đề xuất |
|---|---|---|
| 11 | link `../QUYET-DINH-sau-review.md`; `docs/viscript/SPEC-lex.md` | `` `docs/QUYET-DINH-sau-review.md` (repo ViFastOs) ``; `docs/SPEC-lex.md` |
| 852 | `vis-host-posix`, `vis-host-windows`, `vis-host-wasm`, `vis-host-vifast`, `vis-host-mock` | `vis-host-native` (Linux/macOS/Windows), `vis-host-linux` (Landlock), `vis-host-wasm` (workspace riêng, `wasm32-wasip1`); `vis-host-vifast` ở repo ViFastOs |
| 925 | `Tầng 0 (nền tảng thuần, 0 OS, 0 unsafe)` | `Tầng 0 (nền tảng thuần, 0 unsafe; riêng vish-term có dep termios/crossterm theo target)` |
| 932 | `viscript/ chỉ chứa crate thuần (lõi + 3 backend còn lại + mock)` | `repo này chỉ có lõi/thư viện thuần + vis-host-native/vis-host-linux + workspace riêng vis-host-wasm` |
| 935 | `unsafe chỉ trong danh sách crate đóng (vis-vm cho heap, vis-plugin cho ABI)` | `unsafe bị cấm toàn workspace (#![forbid(unsafe_code)], R2 danh sách đóng rỗng, 0 unsafe tại 0.6.0)` |
| 1081 | `lập lịch tất định theo seed trong vis-host-mock` | `lập lịch tất định theo seed trong crates/vis-runtime/src/rt/frame_test.rs + crates/vis-conformance/tests/async_api_it` |
| 1097 | `L0 crate bị chạm` | `fmt, clippy, luật R1..R9, self-test, report-check, build, doc-examples (không chạy L0)` |
| 1098 | `fmt, clippy, build wasm32 cho crate lõi, …, L0, L1` | `vis-dev check + L0 test (nextest + doc-test); không build wasm32, chưa có L1` |
| 1099 | `vis-dev gate \| + L2` | `vis-dev gate \| không tồn tại — chỉ gate --fast (thiếu --fast ⇒ exit 2)` |
| 1100 | `L4 + benchmark + báo cáo` | `gate --fast + fuzz parser (--fuzz-minutes, mặc định 2) + vòng tròn fmt; không L4/benchmark` |
| 1104 | `docs/viscript/doc-examples-allow.txt` | `docs/doc-examples-allow.txt` |
| 1113, 1133, 1137 | `viscript/agent/QUEUE.md`, `viscript/agent/` | `agent/QUEUE.md`, `agent/` |
| 1222 | `docs/viscript/SPEC-lex.md` | `docs/SPEC-lex.md` |
| 1614 | `crates/vis-cli/src/pkg.rs` | `crates/vis-pkg/src/pkg.rs`; `vis-cli` tái xuất `vis_pkg::pkg` |
| 1641 | `Mục này mô tả ngữ nghĩa đã hiện thực, không chứa số đo.` | `Mục này mô tả ngữ nghĩa đã hiện thực; số đo chi tiết nằm trong agent/reports + JOURNAL (chỉ giữ tại chỗ mốc kiểm chứng tối thiểu).` |

## 3. Đã kiểm tra là **khớp** (không cần sửa)

- 23 workspace member / 5 tầng DAG; `vis-hir`, `vis-http`, `vis-runtime`, `vis-conformance`,
  `vis-host-linux`; `vis-host-wasm` workspace riêng.
- `StdId` 167, `stdlib/` chia miền (0 tệp `_a.._e`), R8 **39 (38+1) ≤ 40**.
- WASI **390/390**, suite **28/28**, conformance **299×2**; `#193`/`#194`/`#196` đóng.
- Không còn `vis-cli::net`, “HTTP client trong vis-core”, “12 crate”, “chưa công bố Tier 1”.

**Đã kiểm chứng lại tại `ae9600a6`**: `./vis version` → `vis 0.6.0`; `./vis version --json` →
`{"name":"vis","version":"0.6.0"}`; `recv_timeout`/`send_timeout`/`select_recv*`/`select_send*` có
trong `crates/vis-types/src/tyck/expr/kenh.rs:36,58,64,83-85`; báo cáo mới nhất `R37`, quyết định
mới nhất `VD-290`; `docs/doc-examples-allow.txt` tồn tại (không phải `docs/viscript/...`).

## 4. Ghi chú

- Các dòng `0.4.5` trong `docs/SPEC.md`/`HOST-BAO-DAM.md` là **mốc lịch sử Windows Tier 1**, giữ nguyên.
- `agent/HOST-BAO-DAM.md` đã được Agent 2 cập nhật trực tiếp ở low-peak (ma trận 23 crate/5 tầng,
  47 ô/cấu hình, số đo 3.729/0, 390/390, 28/28, 299×2) — không nằm trong phạm vi A0-only.
