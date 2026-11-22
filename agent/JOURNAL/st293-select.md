<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-293 — `select_recv*` / `select_send*` (VD-274/VD-280)

> **Người làm**: W3 (Builder), 2026-11-22 · **baseline** `a308cd8` (merge `main`); branch
> `plan/w3-platform`. **Không** đổi parser/EBNF, **không** `unsafe`, không push/merge/tag.

## 1. API đã cài

Bốn builtin có kiểm kiểu tĩnh (không thêm từ khoá/cú pháp):

```text
select_recv(chans: list[Chan[T]]) -> {index:int, value:T?}
select_recv_timeout(chans, ms) -> {index:int, value:T?}?   # ms<0 VS0501; ms=0 thăm dò; hết giờ None
select_send(chans, val) -> int?                            # None nếu tất cả đóng
select_send_timeout(chans, val, ms) -> int?
```

- `select_recv*` trả **record** runtime (`Value::Record` với `index`/`value`); tầng kiểu dùng
  pseudo-record `SelectRecv[T]` trong `tyck/expr/kenh.rs` + `tyck/truong.rs` để `.index`/`.value`
  (và `?.` trên kết quả timeout) có kiểu thật mà **không** đổi `Type`/parser.
- Danh sách rỗng ⇒ `VS0501`; `ms < 0` ⇒ `VS0501`; tất cả kênh đóng: recv trả record value `None`,
  send trả `None`; tất cả treo không còn tác vụ ⇒ `VS0505` (qua `frame_stuck_trap` hiện có).
- Thêm tên vào `builtin_sig::SIGS`, `builtin_is_value`, `vis-bytecode::util::la_builtin`; runtime
  ở `crates/vis-interp/src/rt/select.rs`, kiểu ở `crates/vis-types/src/tyck/expr/kenh.rs`.

## 2. Fairness & wait-set

- `FrameTest.select_turn` tăng mỗi lần gọi; điểm bắt đầu = `turn % n`, giữ nguyên cho **retry**
  của cùng lần chọn ⇒ xoay vòng tất định giữa các lần gọi.
- Claim chung `Rc<RefCell<Option<usize>>>` theo frame: mọi đường `send`/`recv` hiện có hỏi
  `claim_recv`/`claim_send` trước khi nhận/giao; kênh thua **không** nhận giá trị lần hai.
- `select_recv` đăng ký frame vào `recv_waiters` của mọi kênh mở; `select_send` đăng ký
  `ChanSend { select: true }`. Khi một kênh thắng, `don_waiters_cu`/`don_dep_kenh` gỡ toàn bộ
  waiter/giá trị nhánh thua **một lần**; `huy_frame` dọn state khi task bị huỷ.
- Timer chọn dùng reactor timer hiện có; mỗi lần chọn timeout đăng ký một `Wait::Reactor` +
  event; retry dùng lại đúng `(key, gen)`, chỉ đăng ký mới khi chưa có.

## 3. Test

`crates/vis-cli/tests/vs3_select.rs` — **25 ca**, mỗi ca chạy **interp + VM**, so
`stdout/exit/diagnostics/snapshot/log` byte-identical:

- recv: kênh đầu, kênh thứ hai, kênh đóng (value `None`), đệm trước close;
- timeout: rỗng `None`, `ms=0` thăm dò, `ms<0` `VS0501`, nhận sau khi chờ, đóng trả record `None`;
- send: receiver chờ, đệm, chọn kênh thứ hai, tất cả đóng `None`, timeout không receiver;
- fairness: 2 lần/2 kênh, 3 lần/3 kênh; cleanup: không `dup-ready`/`chanmiss`, snapshot sạch;
- `VS0501` list rỗng, `VS0505` recv/send treo; cancel task đang select không in `post`;
- type: `select_recv([1])` ⇒ `VS0412` tĩnh.

Kết quả:

```
cargo test -p vis-cli --test vs3_select   → 25 passed / 0 failed
vs3_chan0 / vs3_chan_cap / vs3_chan_timeout → 13 / 6 / 17 pass (không hồi quy)
cargo clippy -p vis-cli -p vis-interp -p vis-types -p vis-bytecode -p vis-vm --all-targets -- -D warnings → OK
cargo fmt --all -- --check → OK
./vis-dev gate --fast → PASS — 3632 passed (2 slow) / 4 skipped
VIS_DIFF_N=100000 VIS_DIFF_SEED=202611222100 ./bin/vis-diff → tong=100000, lech=0 (199,1 s)
```

R1: `interp.rs`/`stdlib.rs` **không đụng**; tệp lớn nhất đã sửa: `rt.rs` 1339, `builtin.rs` 1330,
`chan.rs` 779, `select.rs` 651, `kenh.rs` 148 — đều ≤ 1.450.

## 4. Blocker

Không có blocker. Không đổi API cũ/default ngoài 4 builtin; runtime đồng bộ vẫn từ chối `select_*`
bằng lỗi rõ; không `unsafe`.
