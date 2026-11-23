# ST-130 · Siết caps khi chương trình dùng gói (VD-124, Q8c)

Agent 2 dòng ViScript · **2026-11-16** · task **ST-130** · ngân sách 3 h · nguồn sự thật:
[`quyet-dinh-a0-2026-11-16.md`](quyet-dinh-a0-2026-11-16.md) §2 Q8 + §6 ST-130,
[`DECISIONS.md`](../DECISIONS.md) **VD-124**, hàng ST-130 trong [`QUEUE.md`](../QUEUE.md).

## 1. Mục tiêu (nguyên văn + cách đo)

- Đồ thị import có **ít nhất một** module `pkg.*` ⇒ chế độ strict-caps bật **mặc định**.
- Cờ thoát **tường minh** `--no-strict-caps` ⇒ chạy được + **một dòng cảnh báo ra stderr** (không ra
  stdout).
- Chương trình **không** dùng gói: hành vi **giữ nguyên** (sem **447** ca không đổi).
- **≥6 ca × interp/`--vm`**. Nếu ước lượng >150 dòng ⇒ kế hoạch VD-116 và dừng.

## 2. Việc đã làm (đo được **37 dòng thêm / 9 dòng bớt** — dưới 150 nên làm luôn, không cần kế hoạch)

| tệp | thay đổi | dòng |
|---|---|---|
| `crates/vis-cli/src/main.rs` | cờ `--no-strict-caps`; cấm dùng đồng thời với `--strict-caps`; **dời** khối `quyen::tao` xuống **sau** `modules::load_with`; `co_pkg = loaded.files.iter().any(rel.starts_with("pkg/"))`; `siet = strict_caps \|\| (co_pkg && !no_strict_caps)`; cảnh báo **một** dòng ra stderr khi `co_pkg && --no-strict-caps`; trợ giúp | +28/−8 |
| `crates/vis-types/src/lower.rs` | **sửa lỗi chặn tính năng** (xem §4): tên module `std` đã `use` (`fs`, `net`, …) **không** phát `VS0301` dưới `NamePolicy::Strict` | +9/−1 |

Không thêm crate, không `unsafe`, không nới ngưỡng. Không sửa `quyen.rs` (tệp ST-124 đang sửa song
song) — chỉ đọc `Quyen.siet` sẵn có.

## 3. Red-first (log + sha256) — [`st130-red-truoc-sau.log`](st130-red-truoc-sau.log) `ef2f334428ad43c991f984ef3f7c0c12dfe02dc6b324a71877684cd25ff7bcd6`

Cùng một chương trình (`use std.fs` + `use pkg.http` + `fs.exists` **không** khai `cap`), chạy bằng
binary **ở HEAD trước ST-130** (worktree tạm, HEAD `4336ba5d`) và binary **có thay đổi**:

| | trước ST-130 | sau ST-130 |
|---|---|---|
| `vis run f.vis` | `error VS0301: tên không tồn tại: fs`, **exit 2** (chương trình dùng gói **không** chạm được fs ⇒ tính năng không kiểm được) | `error VS0604: [quyền] cần khai cap[Dir] cho a.txt`, **exit 2** (siết **mặc định**, đúng VD-124) |
| `vis run --no-strict-caps f.vis` | `vis run: không hỗ trợ cờ --no-strict-caps`, **exit 2** | **cảnh báo 1 dòng ra stderr** + `true`/`100` ra **stdout**, **exit 0** |

## 4. Phát hiện **chặn tính năng** và cách sửa (ghi rõ, không giấu)

Trước ST-130, tệp nào có **bất kỳ** `use` nào đều bị hạ với `NamePolicy::Strict`
(`crates/vis-cli/src/modules.rs:606` `let trong_module = !con.is_empty() || …`); mà `use pkg.*` **là**
một `use` ⇒ chính chương trình "dùng gói" rơi vào Strict. Dưới Strict, `lower_var_use`
(`crates/vis-types/src/lower.rs:662`) phát `VS0301` cho **mọi** tên không phải ký hiệu — kể cả tên
**module std** (`fs`, `net`) mà tệp đã `use`. Hệ quả: `use pkg.*` + `use std.fs` **không biên dịch
được**, tức VD-124 không thể kiểm (đo ở §3, dòng 3). Sửa: coi tên trong `opts.std_mods` là **hợp lệ**
⇒ không phát `VS0301`. **2 dòng logic**, không đổi phân giải tên khác.

## 5. Ca kiểm — `crates/vis-cli/tests/siet_caps_pkg.rs` (**7 ca**, mỗi ca chạy **cả** interp và `--vm --no-cache`, so exit + stdout)

| # | ca | kết quả |
|---|---|---|
| a | có `pkg` + `fs.exists` **không** cap | `VS0604` + `cap[Dir]`, exit **2**, stdout rỗng |
| b | như (a) + `--no-strict-caps` | exit **0**, stdout `true\n100\n`, stderr **đúng 1** dòng cảnh báo |
| c | **không** pkg + `fs.exists` (không cờ / có cờ) | exit **0**, **0** cảnh báo — hành vi **giữ nguyên** |
| d | có `pkg` + `--strict-caps` tường minh | vẫn `VS0604`, exit 2 |
| e | có `pkg`, **không** chạm fs/net, có `--no-strict-caps` | exit 0, **1** cảnh báo (cờ là lựa chọn tường minh) |
| f | có `pkg` + khai đủ `cap[Dir]` + đọc **trong** root | exit 0, `true\n100\n`, **0** cảnh báo |
| g | `--strict-caps` **và** `--no-strict-caps` | exit **2**, "mâu thuẫn", không chạy |

## 6. Lệnh + số đo

```
cd viscript
~/.cargo/bin/cargo test -p vis-cli --test siet_caps_pkg      → 7 passed
~/.cargo/bin/cargo test -p vis-cli --test sem                → 1 passed (447 ca ngữ nghĩa / 22 nhóm; không đổi)
./bin/vis test test/sem                                      → 447 ok
./bin/vis test --vm test/sem                                 → 447 ok (0 SKIP)
~/.cargo/bin/cargo test -p vis-cli --test strict_caps        → 7 passed
~/.cargo/bin/cargo test -p vis-cli --test cap_proc           → 6 passed
~/.cargo/bin/cargo test -p vis-cli --test pkg                → 24 passed
~/.cargo/bin/cargo test -p vis-cli --test cache_khong_che    → 5 passed (26,4 s)
VIS_DIFF_N=100000 VIS_DIFF_SEED=20261115 ./bin/vis-diff      → tong=100000 loi_tinh=6230 chay=93770 lech=0 giay=161.2
```

**Cổng `bash bin/vis-dev gate --fast` = PASS** (dòng cuối `vis-dev gate --fast: PASS (L1 chưa có tới khi
có sim ViScript)`) — log [`st130-gate-fast.log`](st130-gate-fast.log)
(`2d84b18db90ef348ea3bdef50b110d38d34f204499fc023cbe93baf4bd7538da`); `report-check: PASS (22 báo cáo,
149 hàng QUEUE phủ đủ)`; `doc-examples` 24 khối (18 xanh + 6 ngoại lệ). Đây là lần **đầu** cổng xanh
trong chuỗi ST-126..ST-130 — các lần trước bị chặn bởi tệp **đang sửa dở của agent khác**
(`vis-host-linux/**`, `json_esc.rs`, `quyen.rs`), ghi rõ ở JOURNAL từng task.
Lượt này `cargo test --workspace`: **1153 passed, 0 failed** (129 khối `test result: ok`).

Log phạm vi: [`st130-tests-pham-vi.log`](st130-tests-pham-vi.log)
(`c488682e15f9b53fe9027063a2e5d31a2b74b0d53d8236c7c3db9be4fea8f5cd`); differential:
[`st130-diff-100k.log`](st130-diff-100k.log)
(`2db6df0074f09c8842a23bb202abf1702f9edce039202f1faebbdb1b18dd4986`) — **lech=0**.
Số khoá **không đổi**: sem **447/22**, parity **398+49**, FUNCS **52**, `all_codes` **33**, `VBC1` 53,
allowlist 6.

## 7. Tệp đổi

`crates/vis-cli/src/main.rs`, `crates/vis-types/src/lower.rs`,
`crates/vis-cli/tests/siet_caps_pkg.rs` (**mới**), `agent/JOURNAL/st130-*`, `agent/STATE.md`,
`agent/QUEUE.md`.

## 8. Chỗ **lệch** so với quyết định / tự chọn (kèm dòng thực tế)

1. **Dời `quyen::tao` xuống sau `load_with`** (`crates/vis-cli/src/main.rs`): bắt buộc, vì "có dùng gói
   hay không" chỉ biết **sau** khi nạp module. Hệ quả **thứ tự lỗi** đổi: lỗi `--grant` (grant cho
   `cap` không khai) giờ báo **sau** lỗi nạp module/chẩn đoán tĩnh. Không test nào phụ thuộc thứ tự
   này (đo: `cap_proc_grant` 6/6 + `strict_caps` 7/7 xanh).
2. **Hai cờ mâu thuẫn ⇒ exit 2**: VD-124 không nói; tôi chọn **lỗi rõ** thay vì im lặng cho cờ này
   thắng cờ kia (ghi để Agent 0 quyết nếu muốn "cờ sau thắng").
3. **Cảnh báo chỉ khi `co_pkg`**: chương trình **không** dùng gói mà truyền `--no-strict-caps` ⇒
   **không** cảnh báo (không có gì để thoát) — ca (c) khoá điều này.
4. **Sửa `lower.rs` nằm ngoài câu chữ của ST-130** nhưng **bắt buộc** để VD-124 kiểm được (§4). Đây là
   lỗi **có sẵn** (từ ST-71 khi `use pkg.*` làm tệp rơi vào Strict), không phải hồi quy của tôi; sửa
   **9 dòng**, có ca (a)/(b) chứng minh trước/sau (log §3).
5. **Siết mặc định toàn cục** vẫn là **nợ VS3** — không làm (đúng VD-124).

## 9. Nợ mới (đánh số tiếp từ **114**)

| # | Nợ | Mức |
|---|---|---|
| **114** | `NamePolicy::Strict` được bật cho **mọi** tệp có `use` (`modules.rs:606`), không chỉ tệp trong module thật; hệ quả là `use pkg.*` (một import) làm **toàn bộ** tệp chuyển Strict — chính sách này nên xét lại (Strict nên theo "tệp nằm trong cây module", không theo "có import"), vì nó từng che mất lỗi VD-124. Chưa đổi vì ảnh hưởng rộng (447 ca + host suite) | VS3 |
| **115** | `vis test`/host suite **không** áp strict mặc định cho chương trình có pkg (chỉ `vis run` được sửa). Nếu muốn "mọi đường chạy đều siết", phải sửa `vistest.rs` + host suite và đo lại 447 ca | VS3 |
