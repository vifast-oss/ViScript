<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-235 — Audit std/builtins + CLI: 18 phát hiện (chỉ đo, KHÔNG sửa)

**Ngày**: 2026-11-20 · **HEAD**: `c1be50f` · **Binary**: `target/debug/vis` (mã sản phẩm không đổi từ
`4ee06c3`) · **Máy**: Linux x86_64, kernel `7.0.0-43-generic`, rustc `1.99.1`.

Nguồn: hai agent audit độc lập (đọc mã + chạy binary), rồi phiên này **chạy lại 5 ca** bằng lệnh
dưới đây. Các ca còn lại có dẫn chứng `tệp:dòng` + repro của agent; **chưa** chạy lại trong phiên này
(ghi rõ ở cột “đo lại”).

## 0. Lệnh đo lại (5 ca đã xác nhận trong phiên)

```bash
V=/home/ps/ViScript/target/debug/vis
$V run  sort.vis        # use std.list; list.sort([3.0, 0.0/0.0, 1.0, 2.0])
$V run  jp.vis          # use std.json; json.encode(1e310pct)
$V run  rt/a.vis        # a.vis use ./b; b.vis có xs[9] ở dòng 2
$V check crlf.vis       # cùng nội dung với lf.vis nhưng CRLF
$V fmt --check test/fmt/bad.vis   # tệp sai cú pháp nằm trong test/fmt/
```

Kết quả thật (dán nguyên văn):

```
sort:  [3, NaN, 1, 2]                      (exit 0; không sắp xếp, không lỗi)
jp:    inf                                  (exit 0; JSON không hợp lệ)
rt:    rt/a.vis:4:1: error VS0503: chỉ số 9 ngoài phạm vi (độ dài 2)   (đúng phải là rt/b.vis:2:5)
check: lf.vis  → VS0412 19:18 ; crlf.vis → VS0412 18:18              (lệch 1 dòng)
fmt:   test/fmt/bad.vis → exit 0, không output ; plain.vis → VS0201 exit 2
```

## 1. Nhóm std/builtins (10)

| # | Phát hiện | Dẫn chứng | Thực tế → mong đợi | Đo lại | Sửa |
|---|---|---|---|---|---|
| 167 | `net.read` cấp phát theo `max_bytes` người dùng ⇒ **abort** (exit 134) | `stdlib.rs:477-489`, `net.rs:185-192` | `net.read(h, i64::MAX, 500)` ⇒ `memory allocation of 9223372036854775807 bytes failed`; mong đợi `Err`/cap 8 MiB | audit | ~5 dòng |
| 168 | `std.list.sort` **im lặng không sắp** khi không so được (NaN/bản ghi) | `stdlib.rs:266-271` `unwrap_or(Equal)` | `[3.0, NaN, 1.0, 2.0]` ⇒ `[3, NaN, 1, 2]`; mong đợi sort đúng hoặc `VS0501` (như `sort_by`) | **đã chạy** | 10–15 |
| 169 | `json.encode` xuất `inf`/`NaN` cho `percent` vô cực ⇒ JSON không hợp lệ | `stdlib.rs:704-705` | `json.encode(1e310pct)` ⇒ `inf`; mong đợi `null` (như `Float`) | **đã chạy** | 3 |
| 170 | `eq_value` map sai khi khoá trùng; map literal giữ khoá trùng, `map.get` lấy đầu, `json.decode` lấy cuối | `value.rs:292-296`, `interp.rs:796-803`, `vm.rs:736` | `{"x":1,"x":1} == {"x":1,"x":2}` ⇒ `true`; mong đợi `false` (hoặc dedupe literal — **cần A0**) | audit | 10 (eq) / A0 (dedupe) |
| 171 | `ls --long`: mtime > 2262 tràn `u128→i64` ⇒ in năm 1861 | `ngoai.rs:347-352` `as i64` | `touch -d 2446-01-01` ⇒ `modified = -3425585673709551616`; mong đợi 2446 hoặc `-` | audit | 3 |
| 172 | Số học `percent` **bão hoà im lặng**; `show` cũng bão hoà | `builtin.rs:419-455`, `value.rs:421-428` | `200pct * i64::MAX` ⇒ `i64::MAX`; `print(1e300pct)` ⇒ `9223372036854775807pct`; mong đợi `VS0507`/đúng giá trị | audit | ~15 |
| 173 | `json.encode` mất dấu float (`1.0` → `1`) ⇒ round-trip đổi kiểu | `stdlib.rs:697-700`, `818-826` | decode(encode(1.0)) là Int; mong đợi giữ float (**cần A0** vì golden) | audit | 5 |
| 174 | `std.path` sai ở gốc `/` | `stdlib.rs:935-955` | `basename("/") = ""`, `dirname("/") = "."`; POSIX: `"/"`, `"/"` | audit | 6 |
| 175 | `json.decode` nhận `01`/`-01` (JSON không hợp lệ) | `json.rs:240-291` | `json.decode("01")` ⇒ Ok; Python từ chối | audit | 5 |
| 176 | `-9223372036854775808` không viết được (lexer tràn trước khi áp `-`) | tầng literal | phải viết `-9223372036854775807 - 1` | audit | lexer |

## 2. Nhóm CLI/LSP (8)

| # | Phát hiện | Dẫn chứng | Thực tế → mong đợi | Đo lại | Sửa |
|---|---|---|---|---|---|
| 177 | Trap trong **module** bị gán sai tệp + dòng/cột (báo cuối tệp chính) | `interp.rs:56-64`, `main.rs:520-548` | `rt/a.vis:4:1` thay vì `rt/b.vis:2:5` | **đã chạy** | 10–20 |
| 178 | CRLF: chẩn đoán `tyck` lệch dòng/cột (`check`, `--json`, `run`) | `modules.rs:861` dùng text gốc, span trên text đã chuẩn hoá | CRLF `18:18` vs LF `19:18` | **đã chạy** | 2–5 |
| 179 | LSP `publishDiagnostics` range sai với tệp CRLF trên đĩa | `lsp/chan_doan.rs:51` | range `line:18` thay vì `19` (0-based) | audit | ~2 |
| 180 | `vis check --json`: warning mềm (`VS0211`) làm `ok:false` + exit 1, khác TAP | `cli/check.rs:251-253,285-293` | TAP exit 0 vs JSON exit 1; mong đợi exit 0 | audit | ~3 |
| 181 | `vis run --dry` / `--sandbox-check`: tệp sai cú pháp vẫn exit 0, in “không khai báo quyền” | `main.rs:298-305`, `quyen.rs:101-190` | `vis check` exit 1 nhưng `run --dry` exit 0; mong đợi VS0201 exit 2 | audit | ~10 |
| 182 | `vis fmt --check/--write` **bỏ qua im lặng** tệp người dùng nêu tên nếu nằm trong `test/fmt|check|doc-examples` hoặc có sibling `.err` | `cli/fmt.rs:149-155` | `test/fmt/bad.vis` exit 0; `plain.vis` VS0201 exit 2 | **đã chạy** | ~8 |
| 183 | `vis fmt` nhiều tệp: chẩn đoán không nêu tên tệp | `cli/fmt.rs:185,200` | `# VS0201 1:8 …` ×2, không biết tệp nào | audit | 2–3 |
| 184 | LSP framing: header cụt **không có** `Content-Length` bị coi là EOF sạch (exit 0) | `lsp/khung.rs:31-36` | `printf 'Content-Type: …\r\n' \| vis lsp --stdio` exit 0; mong đợi lỗi exit 1 | audit | ~3 |

Ghi chú ngoài bảng (từ audit, chưa xác nhận độc lập): `vis fmt --indent-report <không tồn tại>` exit 0;
`env -u HOME vis cache --path` in lỗi nhưng exit 0; LSP nhận request sau `shutdown`; `net.write`
không write-timeout; record equality bỏ qua tên kiểu; `sqlite.query` chạy INSERT (cần A0 xác nhận chủ ý).

## 3. Kết luận

- **Không sửa dòng mã nào** trong ST-235. 18 mục ghi thành nợ **#167..#184** (đều **mở**); ưu tiên:
  #167 (abort) > #177/#178 (sai vị trí) > #168/#169 (sai kết quả/JSON hỏng) > #180–#184 (im lặng) >
  còn lại.
- Ca cần **A0 chốt ngữ nghĩa**: #170 (khoá trùng), #173 (float trong JSON), `sqlite.query` INSERT.
- Mọi repro đã chạy nằm ở `/tmp/opencode/v234/` và `/tmp/opencode/audit-{std,cli}/` (ngoài repo);
  các biểu thức chính đã dán ở §0/§1/§2 để tái lập không cần các thư mục đó.
