<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# Checkpoint sau Lát 0 — chờ A0 duyệt Lát 1 (VD-250, 2026-11-20)

> **Trạng thái**: ĐÃ XONG Lát 0 + #148 + 5 đặc tả. **DỪNG TẠI ĐÂY** — theo VD-250 §3/§6,
> không tự mở Lát 1, không thay runtime mặc định, không đổi tag/release (vẫn **PAUSED**),
> baseline lịch sử `dbae70a`. Không mở VS3 toàn phần/worker/GC/TLS/API mới.
>
> **Căn cứ**: VD-250 (`agent/DECISIONS.md`) · ST-264/265/266 (`agent/QUEUE.md`) ·
> phiếu `agent/DE-XUAT-KIEN-TRUC-ASYNC-2026-11-20.md`.

## 1. Lát 0 — prototype suspend/resume (ST-264)

**Sản phẩm**: `crates/vis-interp/tests/lat0_suspend.rs` (10 test) · báo cáo
`agent/JOURNAL/vd250-lat0-prototype.md`.

**6 tiêu chí A0 → bằng chứng** (đều có test riêng cho **cả hai** mô hình engine):

| # | Tiêu chí | Kết quả |
|---|---|---|
| 1 | A chạy → suspend → B chạy → A resume đúng vị trí | log `["A:pre","B:run","A:post"]`; `A:post` sau `B:run` |
| 2 | Local + capture giữ giá trị | `10 + 7 = 17` sau resume; ô capture chia sẻ đúng (writeback) |
| 3 | Resume không lặp tác dụng phụ | `A:pre` xuất hiện **đúng 1 lần** (so_full vector) |
| 4 | Kết quả **và** lỗi qua continuation | `Ok(41)+1 = 42`; `Err` short-circuit, bỏ `after-await` |
| 5 | `await` trong biểu thức lồng | log `["operand-eval","g","call=42"]` — operand trước, `g` sau resume, outer call cuối |
| 6 | Không đổi runtime/bytecode; không stack lớn/đệ quy | `max_stack ≤ 4`, `max_work ≤ 8`; `pump()` là vòng `while let … pop_front()` |

**Cách giữ trạng thái**: VM-like mirror `Frame { fi, pc, locals, captured, st_cu, writeback }`
(`vm.rs:124-135`), resume bằng tiến `pc` qua `Await`; interp-like dùng statement index + **heap
continuation stack** (`work: Vec<Work>`), không qua Rust call stack. Event là cờ giả do harness bật;
không socket thật.

**Giới hạn trung thực (phải đọc trước khi duyệt Lát 1)**: đây là **model spike**, không phải đường
`goi_than` thật. Chưa nối vào HIR/bytecode/interp `eval`/VM `run`; chưa đa-frame continuation; chưa
mọi vị trí `await` mà checker hiện chấp nhận; chưa `Trap`/`VS0506` thật; chưa event-trace; chưa
ngân sách/ownership. Runtime mặc định, bytecode phát hành, VERSION và opcode **không đổi**.

## 2. #148 — Option A (ST-265)

`crates/vis-interp/src/net.rs`: helper `con_lai_ms(han)` (`:64-75`); `doc_http_mot` nhận **một**
`han` chung (`:229-274`), mỗi lần đọc dùng thời gian còn lại, **giữ clamp 2 000 ms/đọc**; `http_get`
tạo deadline **duy nhất tại đầu lời gọi** (`:334`), connect dùng remaining (`:340-346`), **cùng `han`
cho mọi redirect hop** (`:348`), `close` trên mọi đường; tương tự cho HEAD (`:376-435`).
**Supersede** phần timeout liên quan của VD-183 ("giữa hai lần đọc, không phải tổng") — ghi rõ trong
`JOURNAL/vd250-http148.md`. Không API mới, không TLS/HTTP2.

Test đỏ-trước `crates/vis-cli/tests/http.rs:660-708` (3 redirect + response chậm, `get_timeout(...,300)`):
**RED 1,01 s** (4×250 ms) → **GREEN ~300 ms**; lặp 3 lần 0,61/0,62/0,62 s không flake.
**Đính chính đã ghi**: slow-drip **trong một response** vốn đã bị chặn (deadline tính một lần tại
`net.rs:225/357` cũ); lỗ là chuỗi redirect.

**Còn lại (không sửa ở đây)**: `NetHost::write` không timeout (có thể chờ lâu nếu peer không đọc);
deadline kiểm giữa các lần đọc, không phải ngắt cứng.

## 3. Năm đặc tả chuẩn bị cho Lát 1 (ST-266)

| Tệp | Nội dung |
|---|---|
| `JOURNAL/vd250-determinism-contract.md` | Trace/tie-break; **reactor giả ⇒ byte-identical**; mạng thật chỉ kiểm đúng đắn/tiến triển/giới hạn/thứ tự bắt buộc; bảng phân loại fixture **P/O/I/T** — **không** cập nhật golden hàng loạt |
| `JOURNAL/vd250-charging-spec.md` | Tách duyệt/chưa duyệt; 5 câu tính phí có đề xuất; hạn ngạch CPU **riêng, reset/turn**; cấm giữ `MAX_STEPS` làm giới hạn thường; watchdog phải distinct |
| `JOURNAL/vd250-fee-repr.md` | So sánh trung lập bảng phụ vs `Instr::Charge` (**side-table không tự động duyệt**); kiểm thiếu/sai vị trí/zero-tamper/nhảy bỏ charge/crafted loop/resume double-charge; **không tin artifact chỉ vì decode được**; VERSION gán **sau** khi chọn |
| `JOURNAL/vd250-ownership.md` | Thu hồi khi **hết handle VÀ xong nghĩa vụ join**; bảng trạng thái + chuyển tiếp; phân biệt tài nguyên task-owned với handle thoát task; hook `diag_snapshot()` test/debug only; khung cancellation/first-error **trước Lát 3** |
| `JOURNAL/vd250-dependency-policy.md` | Chính sách thực sự: `CONTRIBUTING.md:33` + R2 + allowlist RiScript `docs/ViScript-thiet-ke.md:926` (**không tự áp** allowlist ViFastOs); tiền lệ `sha2`/`rusqlite`; gap `deny.toml`/`THIRD-PARTY.md`; mâu thuẫn unsafe; **chưa chọn crate** |

## 4. Validation

- `./vis-dev check`: **PASS** (fmt, clippy `-D warnings`, R1..R6, self-test, build, doc-examples).
- `cargo test -p vis-interp -p vis-cli`: **998 passed / 0 failed** (987 baseline + 10 Lát 0 + 1 HTTP).
- `cargo test -p vis-interp --test lat0_suspend`: **10 passed / 0 failed**.
- Luật R1..R6 trên toàn repo sau khi thêm tài liệu: **OK**. Tag vẫn `pha1-rc1`; baseline `dbae70a`.

## 5. Đề xuất Lát 1 để A0 duyệt (CHƯA triển khai)

Lát 1 = **cheduler có frame, vẫn I/O chặn thật**: `ready` chứa frame sẵn sàng; tách `await_task`
(`rt.rs:178-230`) thành bắt đầu/tiếp tục; inject `Pending` giả từ test; **chưa đụng reactor**.
Tiêu chí đề xuất:
- Ca synthetic "task chờ giả + `MARK` spawn sau" ⇒ `MARK` in **trước** khi task chờ xong — nhưng
  theo hợp đồng tất định đã sửa (VD-250 §2.1), so bằng **event trace/reactor giả**, không bằng mạng
  thật; và theo §2.2, fixture thuần tính toán giữ nguyên, fixture có I/O **phân loại** chứ không đổi
  golden hàng loạt.
- 35 ca `test/sem/async/*.vis` + `diff_100k` không đổi; `VS0506`, cache, final join theo thứ tự tạo,
  task không await — giữ nguyên (VD-250 §2.3: **không** huỳnh task pending lúc main exit).
- B1 (20×200 ms) **vẫn ~4,11 s** ở Lát 1 — ghi rõ chưa sửa I/O; B2 vẫn đói — ghi rõ.
- Trước Lát 1 phải chốt: 5 câu tính phí + bảng phụ/opcode + ownership/diag + chính sách dependency
  (§3 và các file journal). **Không ấn VERSION** cho tới khi A0 chọn biểu diễn.

## 6. Quyết định A0 cần cho bước tiếp

1. Chốt các câu mở trong 5 đặc tả (đặc biệt: tie-break event trace, 5 câu tính phí, bảng phụ vs
   opcode, luật thu hồi 2 điều kiện, allowlist/unsafe).
2. Duyệt hay không **Lát 1** theo đề xuất §5.
3. Nhắc lại: tag/release PAUSED; không mở VS3/worker/GC/TLS/API; không dùng ước lượng cũ làm cam kết.

**Kết luận**: mục tiêu Lát 0 không phải "có event loop" mà là chứng minh hai engine dừng/tiếp tục
đúng, không lặp tác dụng phụ, không mất trạng thái — đã đạt **ở mức model**. Phần nối vào runtime thật
là việc Lát 1 và chỉ bắt đầu khi A0 duyệt.
