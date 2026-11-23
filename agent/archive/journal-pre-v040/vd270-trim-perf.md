<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VD-270 / trim-perf — bảng tra tập scalar cho `str.trim*_matches`

> **Phạm vi**: tối ưu hiệu năng, **không đổi ngữ nghĩa, chữ ký, API, dispatch**. Chỉ sửa
> `crates/vis-interp/src/stdlib/strtrim.rs`, thêm `test/bench/trim-matches.vis`, thêm unit test
> trong chính module. Không đụng parser/fee/db/cache/frame/LSP/SPEC, không thêm dependency.
>
> **Cây đo**: worktree `vd270-trim-perf`, nhánh cùng tên, fast-forward từ `0bcbed7` lên
> **`254c03f`** (main HEAD lúc bắt đầu) trước khi dựng binary nền.
> **Binary**: profile `dev` (`cargo build -p vis-cli --bin vis`), không phải release.
> **Máy**: Linux x86_64, 12 lõi; **máy có tải** — load average dao động **~7–28** trong lúc đo
> (các agent khác chạy build/`vis` song song). Vì vậy số đo là **tham khảo có đối chứng xen kẽ**,
> không phải khung đo máy rảnh.
>
> **Kết luận**: trên bench này, bản tối ưu nhanh hơn **~71× (interp)** và **~84× (VM)** theo
> trung vị; checksum đầu ra **giống byte** ở mọi lần chạy.

## 1. Thay đổi

`trước` — mỗi scalar của `s` duyệt tuyến tính toàn bộ `chars`:

```rust
let trong = |c: char| chars.chars().any(|x| x == c);
s.trim_matches(trong)
```

`O(|s| × |chars|)`; với `|s| = 10^6`, `|chars| = 1000` là ~10^9 phép so sánh.

`sau` — ngưỡng **8 scalar**:

- `chars` rỗng ⇒ trả `s` nguyên vẹn (nhánh sớm giữ nguyên).
- `≤ 8` scalar ⇒ giữ **nguyên** đường duyệt tuyến tính, **không cấp phát**.
- `> 8` scalar ⇒ `Vec<char>` + `sort_unstable` + `dedup` **một lần**, rồi
  `binary_search` (`O(log |chars|)`) cho mỗi scalar.

Bảng tra vẫn là **tập** scalar: sắp xếp/bỏ trùng không đổi quan hệ thành viên; `char` của Rust
là scalar Unicode nên astral (`😀`, `\u{10FFFF}`) là **một** phần tử, không bị tách byte.
Không có chuẩn hoá Unicode nào được áp thêm. Vì vậy kết quả trùng khít bản cũ cho mọi input,
kể cả `chars` có lặp, thứ tự bất kỳ, hoặc rỗng.

Đã cân nhắc `HashSet<char>` (đề xuất trong phiếu): đo cùng bench, cùng máy, 5 cặp xen kẽ:

| Bản | 5 lần (ms) | Trung vị |
|---|---|---|
| `HashSet<char>` | 618, 1048, 1106, 1099, 1331 | **1099** |
| `Vec<char>` sắp xếp + nhị phân | 391, 651, 729, 659, 580 | **651** |

Giữ bản sắp xếp vì nhanh hơn ~1,7× trên binary debug này (SipHash của `HashSet` đắt trong
debug), không cần hash seed, và cùng độ phức tạp logarit.

## 2. Bench

Tệp: [`test/bench/trim-matches.vis`](../../test/bench/trim-matches.vis) — script thuần, không
dependency mới, 29 dòng.

- `bo` = **1000 scalar phân biệt** U+4E00..U+51E7 (CJK, NFC ổn định — tránh việc lexer NFC
  toàn tệp sắp xếp lại dấu tổ hợp như lần thử đầu với U+0300..).
- `s` = 500 000 scalar thuộc `bo` + `"MID"` + 500 000 scalar thuộc `bo` = **1 000 003 scalar**
  (3 000 003 byte); 1 000 000 scalar ở hai đầu đều thuộc tập ⇒ trường hợp xấu nhất.
- Ba lời gọi: `trim_matches` (bỏ 10^6 scalar), `trim_start_matches` và `trim_end_matches`
  (mỗi lời bỏ 5×10^5).
- Có `assert` kết quả chính xác (`ca == "MID"`, độ dài, `starts_with`/`ends_with`) và in
  checksum cố định:
  `1000009 3 1500003 1500003 3000003` (tổng char_count; số `M`; byte_len đầu; byte_len cuối;
  byte_len `s`).

Lệnh đo (binary nền và binary tối ưu **xen kẽ** trong từng vòng, cùng một tệp bench):

```bash
/tmp/opencode/vis-base-254c03f run --no-cache --time      test/bench/trim-matches.vis
/tmp/opencode/vis-final-vd270  run --no-cache --time      test/bench/trim-matches.vis
/tmp/opencode/vis-base-254c03f run --no-cache --time --vm test/bench/trim-matches.vis
/tmp/opencode/vis-final-vd270  run --no-cache --time --vm test/bench/trim-matches.vis
```

- Binary nền: dựng từ cây **chưa sửa** tại `254c03f`; sha256
  `7832097efa8dd873e517678c1143c850c046155ebdbc102480c0343af0002877`.
- Binary tối ưu: sha256
  `0c68a5a3255dd34ebdea6ca58357f7f1a7f653e32d774141d87ddddc1c594c5d`.
- `--time` đo **đồng hồ tường quanh đúng lời gọi runtime** (không tính nạp/parse/tyck/codegen/
  cache); `--no-cache` để không dùng artifact cũ.

## 3. Số đo trước/sau (3 cặp xen kẽ, `vis run --time`)

| Vòng | Engine | Nền `254c03f` (ms) | Tối ưu (ms) | Tỉ lệ |
|---|---|---:|---:|---:|
| 1 | interp | 16 247 | 214 | 75,9× |
| 2 | interp | 17 778 | 251 | 70,8× |
| 3 | interp | 28 300 | 340 | 83,2× |
| 1 | VM | 23 553 | 227 | 103,8× |
| 2 | VM | 19 045 | 226 | 84,3× |
| 3 | VM | 17 957 | 225 | 79,8× |

| Engine | Trung vị nền | Trung vị tối ưu | Tỉ lệ trung vị | Min nền / min tối ưu |
|---|---:|---:|---:|---:|
| interp | **17 778 ms** | **251 ms** | **~70,8×** | 16 247 / 214 ms |
| VM | **19 045 ms** | **226 ms** | **~84,3×** | 17 957 / 225 ms |

Mọi 12 lần chạy đều exit 0 và in **cùng** checksum
`1000009 3 1500003 1500003 3000003`.

**Đối chứng phần khung đo**: một bản sao bench bỏ hẳn ba lời gọi trim (giữ tạo chuỗi + assert +
checksum) chạy **10–14 ms** trên binary tối ưu ⇒ phần còn lại ~200–330 ms đúng là chi phí tra
tập, không phải chi phí dựng chuỗi/đo.

**Cảnh báo tải**: cùng binary nền, trước khi đo xen kẽ, có lần cho 15 654 ms (interp) và
16 958 ms (VM) lúc tải thấp, có lần 46 918/49 333 ms lúc tải cao. Vì vậy chỉ bảng xen kẽ ở trên
làm căn cứ so sánh; các con số tuyệt đối chỉ có nghĩa trên máy này, ở thời điểm này.

## 4. Đúng đắn

- `cargo test -p vis-interp --lib strtrim`: **6/6** (3 test cũ + 3 test mới):
  - `tap_lon_trung_va_astral` — tập > ngưỡng có 20 lần lặp `'x'` + `'😀'` + `'đ'`; cả ba hàm,
    cả hai phía, rỗng ⇒ nguyên vẹn.
  - `tap_lon_tuong_duong_ban_tuyen_tinh` — tập 300 scalar + trùng + astral, 9 chuỗi đầu vào,
    so **từng byte** với bản tham chiếu tuyến tính viết lại nguyên văn trong test.
  - `nguong_bang_tra_bien` — đúng biên 8 (tuyến tính) và 9 (bảng tra), cả ba hàm.
- `cargo test -p vis-cli --test vd270_str_trim` (**không sửa tệp test**): **8/8**.
- `cargo test --workspace`: **2425 passed / 0 failed / 3 ignored** (gồm doc-test).
- `./vis-dev gate --fast`: **PASS** (1 m 51 s; `cargo-nextest` không có trên máy này nên L0 dùng
  `cargo test --workspace`).

## 5. Khoảng trống / ghi chú

- **Chỉ đo debug**. Chưa đo release; ở release cả hai cách tra đều nhanh hơn debug, nhưng tỉ lệ
  so với bản tuyến tính chắc chắn vẫn dương. Không tuyên bố số release.
- Máy đo **có tải** (agent khác chạy song song); đã giảm thiểu bằng cách xen kẽ nền/tối ưu
  trong cùng vòng, nhưng chưa có khung máy rảnh theo chuẩn VD-220.
- Ngưỡng 8 lấy theo phiếu; **chưa quét** ngưỡng 4/8/16. Với `|chars|` lớn, chi phí dựng bảng
  amortize rõ; với `|chars|` nhỏ, đường tuyến tính tránh cấp phát.
- Bảng tra dựng **mỗi lời gọi** (stdlib thuần, không cache trạng thái). `O(|chars| log |chars|)`
  một lần là không đáng kể so với `O(|s| log |chars|)`.
- Bench **không** nằm trong `gate --fast` (giữ cổng nhanh); đây là script đo tay, không phải
  test hồi quy hiệu năng có ngưỡng.
