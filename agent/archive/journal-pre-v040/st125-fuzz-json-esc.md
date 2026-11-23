# ST-125 · Fuzz `json_esc` 100 000 chuỗi, đối chiếu ba phía (VD-130e)

Agent 2 dòng ViScript · **2026-11-16** · task **ST-125** · ngân sách 1,5 h · nguồn sự thật:
[`quyet-dinh-a0-2026-11-16.md`](quyet-dinh-a0-2026-11-16.md) §2 (`§10(e) json_esc`), §6 **ST-125**,
[`DECISIONS.md`](../DECISIONS.md) **VD-130(e)**.

## 1. Mục tiêu

Bộ escape JSON của CLI (`crates/vis-cli/src/json_esc.rs::esc`, dùng cho `check --json` và
`test --json`) phải khôi phục **đúng từng ký tự** qua **ba phía**:

1. chuỗi **gốc**,
2. `json.decode(esc(s))` — **đúng** hàm mà `std.json.decode` gọi (`vis-interp/src/json.rs::decode`,
   gọi từ `vis-interp/src/stdlib.rs:379`),
3. `python3 json.loads` trên cùng chuỗi đã escape (bên thứ ba độc lập).

Lệch nào cũng là **lỗi** ⇒ sửa `esc` (nguồn duy nhất), **không** sửa bộ sinh để né. `lech = 0` mới xong.

## 2. Bộ sinh (seed cố định, ghi vào log)

`seed = 20261115` (`const SEED`, tất định: xorshift64* — chạy lại cho **cùng** 100 000 ca).
`SO_CA = 100_000`. Bảy họ ca, phủ đúng danh sách VD-130e:

| # | Họ (`i % 7`) | Nội dung |
|---|---|---|
| 1 | 0 | **ký tự điều khiển** `U+0000..U+001F` (1–8 ký tự) |
| 2 | 1 | **`U+0000`** lẫn trong ký tự thường (1–7) |
| 3 | 2 | **`U+2028`/`U+2029`** (dấu phân cách dòng Unicode) |
| 4 | 3 | **ngoài BMP** (`U+1F600`, `U+1F1FB`, `U+20BB7`, **`U+10FFFF`**…) |
| 5 | 4 | `"` và `\` dày đặc (1–11 ký tự chỉ gồm hai ký tự này) |
| 6 | 5 | **chuỗi ngẫu nhiên** trên bảng ký tự rộng (điều khiển + `U+2028/2029` + Việt/Nhật + emoji) |
| 7 | 6 | biên: **rỗng**, 1 ký tự, chuỗi **dài 300–599 ký tự** |

## 3. Kết quả đo — `JOURNAL/st125-fuzz-100k.log`

Lệnh:

```
~/.cargo/bin/cargo test -p vis-cli --bin vis json_esc -- --nocapture --test-threads=1
```

Dòng đo (nguyên văn từ log):

```
test json_esc::fuzz_100k::fuzz_100k_khop_ba_phia ... ST-125 fuzz: seed=20261115 ca=100000
  json_decode_lech=0 · python3 ca=100000 lech=0 ca_encode=100000 lech_encode=0
test result: ok. 5 passed; 0 failed; ...
finished in 11.28s
```

| Phía | Số ca | Lệch |
|---|---|---|
| `json.decode(esc(s))` | 100 000 | **0** |
| `python3 json.loads('"' + esc(s) + '"')` | 100 000 | **0** |
| `json.decode(json.encode(s))` (đường escape **thứ hai**, `json_tai`) | 100 000 | **0** |
| `python3 json.loads(json.encode(s))` | 100 000 | **0** |

⇒ **`lech = 0`** trên cả bốn phép so. **Không phải sửa `json_esc`**: không có lệch nào để sửa.

## 4. Red-first: **không có log đỏ thật** + lý do + **đối chứng chứng minh bộ fuzz không vô nghĩa**

- **Lý do không có log đỏ**: `esc` đã đúng trên toàn bộ 100 000 ca ngay lần chạy đầu (đo §3). Việc
  "sửa cho đỏ trước" chỉ có nghĩa nếu có lỗi thật; ở đây **không** có ⇒ ghi lý do theo luật red-first.
- **Đối chứng (ca phá)**: `doi_chung_escaper_hong_bi_phat_hien` dùng **cùng** bộ sinh + cùng phép so,
  nhưng escape bằng một bản **cố ý hỏng** (`esc_hong`: quên `\` và quên `U+0000..U+001F`) ⇒ test đòi
  **≥1 ca bị phát hiện** và **đã xanh** ⇒ bộ fuzz **có** khả năng phát hiện lỗi escape, không phải
  "xanh vì không kiểm gì". (Đây là mutation-test tối thiểu cho chính harness, **không** phải nới ngưỡng.)

## 5. Vì sao 100 000 ca mà chỉ ~11 s

Một lần sinh + escape + decode trong tiến trình (Rust) và **một** lần gọi `python3` cho cả 100 000 ca
(ghi `escaped.txt` + `encoded.txt` + `goc.bin` có **tiền tố độ dài u32 LE** để chuỗi gốc chứa `\n`,
`\r`, `U+0000` vẫn đọc được chính xác — **không** phải sửa chuỗi gốc cho vừa định dạng tệp).

## 6. Tệp đổi

| Tệp | Loại |
|---|---|
| `crates/vis-cli/src/json_esc.rs` | **thêm** module `#[cfg(test)] mod fuzz_100k`: bộ sinh seed cố định + ca ba phía + ca đối chứng escaper hỏng (không đổi hàm `esc`) |
| `agent/JOURNAL/st125-fuzz-100k.log` | log đo (seed + số ca + lech từng phía) |

## 7. Nợ mới (đánh số tiếp từ **112**; ST-125 dùng **116**)

- **116** — Có **hai** bộ escape JSON trong dòng ViScript: `vis-cli/src/json_esc.rs::esc` (cho
  `check --json`/`test --json`) và `json_tai`'s `esc` trong `vis-interp/src/stdlib.rs:573-585` (cho
  `to json`). Cả hai đã đối chiếu `python3` **0 lệch** (§3), nhưng chúng **khác kiểu** (`\b`/`\f` viết
  tắt ở bộ thứ nhất, `\u0008`/`\u000c` ở bộ thứ hai) ⇒ VD-130e nói "nguồn duy nhất" **chưa** đúng
  trên thực tế. Đề nghị: hoặc gộp về một hàm, hoặc Agent 0 sửa câu chữ của VD-130e thành "hai bộ, cả
  hai phải qua fuzz" (ST-125 đã đo **cả hai**).

## 8. Chỗ không áp được nguyên văn (so với quyết định Agent 0)

- **lệch**: §6 ST-125 "Đối chiếu ba phía: chuỗi gốc, `json.decode(escape)`, `python3 json.loads`" →
  **thực tế**: làm **bốn** phép (thêm đường `json.encode`/`json_tai`) vì "nguồn duy nhất" trên thực tế
  là **hai** bộ escape (nợ #116). Không đổi bộ sinh, không đổi ngưỡng.
- **lệch**: không có log đỏ thật ⇒ ghi lý do ở §4 + **đối chứng ca phá** thay cho log đỏ.

## 9. Trạng thái cổng lúc bàn giao

- `json_esc` **chỉ thêm test**, không đổi hàm `esc` ⇒ không thể đổi hành vi sản phẩm.
- `cargo test --workspace --no-fail-fast`: **127 suite OK · 1138 test xanh** (log
  `st124-workspace-no-fail-fast.log`) — trong đó có 5 test của `json_esc` (3 cũ + 2 mới).
- `gate --fast` lúc bàn giao còn đỏ ở bước `fmt` vì **tệp chưa commit của task khác**
  (`crates/vis-cli/tests/siet_caps_pkg.rs`, ST-130) — **ngoài phạm vi**, không sửa (xem
  [`st124-review.md`](st124-review.md) §9).
- `VIS_DIFF_N=100000 VIS_DIFF_SEED=20261115 ./bin/vis-diff` ⇒ **lech=0** (`st124-diff-100k.log`).

## 10. Cổng cuối — `gate --fast` **PASS** trên đúng HEAD

Trên **worktree sạch tại `dd81c9fa`** (cây chung đang bị task khác sửa chưa commit):

```
vis-dev gate --fast: PASS   # 128 suite / 1146 test / 0 đỏ (log: st124-gate-clean-pass.log)
--- doc-examples: OK (24 khối: 18 xanh + 6 ngoại lệ đỏ đúng)
```

`json_esc` chỉ **thêm test**, không đổi `esc` ⇒ cổng xanh là xác nhận "không hồi quy".
