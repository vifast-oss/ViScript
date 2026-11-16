# ST-130b · Ca thứ 8 — chương trình có `use` (module nội bộ + `std.fs`) mà **không** `pkg.*` phải giữ hành vi caps cũ (VD-133)

Agent 2 dòng ViScript · **2026-11-16** · task **ST-130**, phần bổ sung theo
[`KE-HOACH-CHOT-23f.md`](../../KE-HOACH-CHOT-23f.md) §2 (GO cho Agent 2) · nguồn sự thật:
[`DECISIONS.md`](../DECISIONS.md) **VD-124**, **VD-133**, **VD-143**, VD-119; hàng **ST-130** +
dòng **XEM LẠI** ngay dưới trong [`QUEUE.md`](../QUEUE.md).

## 1. Mục tiêu (nguyên văn từ 23f §2 + VD-133)

Thêm vào `crates/vis-cli/tests/siet_caps_pkg.rs` **ca thứ 8**: chương trình có **`use` module nội bộ** +
**`use std.fs`**, **không** `pkg.*`, **không** khai `cap`, **đọc một tệp** ⇒ **chạy được, KHÔNG
`VS0604`**, cả **interp** và **`--vm --no-cache`**.

Đây là **ca khoá** cho câu của VD-133: *"chương trình có `use` nhưng không dùng `pkg.*` phải giữ hành
vi cũ về caps; có ca khoá"* — nợ **#114** chỉ được chuyển VS3 khi ca khoá này xanh.

## 2. Đỏ-trước: **không có log đỏ thật vì hành vi đã đúng — ca khoá hồi quy**

Chương trình của ca chạy trên HEAD `926022e4` (trước khi thêm ca) **đã xanh cả hai đường**: chế độ siết
chỉ bật khi `co_pkg` (`main.rs`: `co_pkg = loaded.files.iter().any(|f| f.rel.starts_with("pkg/"))`), mà ca
này **không** có `pkg.*` ⇒ `siet = false` ⇒ hành vi cũ. Vì vậy **không có log đỏ để dán**; ca này là **ca
khoá hồi quy**, không phải bằng chứng sửa lỗi. Không sửa một dòng sản phẩm nào.

Bằng chứng xanh + **đối chứng** (log [`st130b-do-xanh-doi-chung.log`](st130b-do-xanh-doi-chung.log),
`sha256 8c22c4036243289b29837f9e8cc156eae6f251d7e7bc7f9727ee202fc5b6c607`):

| lệnh (cùng chương trình) | stdout | stderr | exit |
|---|---|---|---|
| `vis run f.vis` (interp) | `x\n7\n` | — | **0** |
| `vis run --vm --no-cache f.vis` | `x\n7\n` | — | **0** |
| **đối chứng** `vis run --strict-caps f.vis` | — | `error VS0604: [quyền] cần khai cap[Dir] cho a.txt` | **2** |
| **đối chứng** `vis run --vm --no-cache --strict-caps f.vis` | — | `error VS0604: …cap[Dir]…` | **2** |

⇒ Ca **không vô nghĩa**: cùng chương trình (cùng đường đọc tệp), chỉ bật `--strict-caps` là bị chặn
`VS0604` thật. Đối chứng này **được nhúng ngay trong `#[test]`** (vế thứ hai, §3) — nếu sau này ai làm
caps ngừng siết thì chính ca thứ 8 đỏ, không phải chỉ dựa vào log tay.

## 3. Ca thứ 8 — `crates/vis-cli/tests/siet_caps_pkg.rs`

**Đếm:** `grep -c '#\[test\]' crates/vis-cli/tests/siet_caps_pkg.rs` = **8** (7 ca cũ + ca mới; file
264 dòng, **không** `#[ignore]`, không nới ngưỡng, không sửa kỳ vọng ca cũ).

Ca mới: `use_module_noi_bo_va_std_nhung_khong_pkg_giu_hanh_vi_cu` — chú thích `(h)`, chạy qua helper
`ca()` có sẵn (chạy **cả** interp và `--vm --no-cache`, đòi exit + stdout **giống nhau** giữa hai đường):

```vis
use ./util
use std.fs

fn main():
    match fs.read("a.txt"):
        ok(s):
            print(s)
            print(g())
        err(e):
            print("err")
```

`util.vis` = `pub fn g() -> int:` trả `7` (module nội bộ **thật**, không phải import rỗng), `a.txt` = `x`.

| vế | khẳng định |
|---|---|
| chạy (2 đường) | exit **0**, stdout **`x\n7\n`**, stderr **không** chứa `VS0604` |
| **đối chứng** (2 đường) | cùng chương trình + `--strict-caps` ⇒ exit **2**, `VS0604`, nêu `cap[Dir]`, stdout rỗng |

Lý do ca này khác ca (c): ca (c) chỉ có `use std.fs`; ca này thêm **`use ./util`** — chính `use` module
nội bộ làm tệp rơi `NamePolicy::Strict` (`modules.rs`: `trong_module = !con.is_empty() || …`), nên nó
khoá đúng câu "có `use` (kể cả `use` module nội bộ) mà không `pkg.*` thì **không** siết caps".

## 4. Lệnh + số đo

| lệnh | kết quả | log (`agent/JOURNAL/`) + `sha256` |
|---|---|---|
| `cargo test -p vis-cli --test siet_caps_pkg` | **8 passed / 0 failed** | [`st130b-siet-caps-8ca.log`](st130b-siet-caps-8ca.log) `a6d3bc66…` |
| `cargo test -p vis-cli --test sem` | 1 passed (harness khoá **447 ca / 22 nhóm** — không đổi) | [`st130b-sem-cargo.log`](st130b-sem-cargo.log) `39908f15…` |
| `./bin/vis test test/sem` | **447 ok**, exit 0 | [`st130b-vis-test-interp.log`](st130b-vis-test-interp.log) `eb0e0e98…` |
| `./bin/vis test --vm test/sem` | **447 ok**, exit 0 | [`st130b-vis-test-vm.log`](st130b-vis-test-vm.log) `e6363412…` |
| `VIS_DIFF_N=100000 VIS_DIFF_SEED=20261115 ./bin/vis-diff` | `tong=100000 loi_tinh=6230 chay=93770 **lech=0** giay=163.9` | [`st130b-diff-100k.log`](st130b-diff-100k.log) `81bf140e…` |
| `bash bin/vis-dev gate --fast` (cây sạch, xem §6.3) | dòng cuối **`PASS`**; L0 **1154 passed / 0 failed** (129 suite) | [`st130b-gate-fast.log`](st130b-gate-fast.log) `52592bf7…` |
| `python3 test/report/check_report.py --all` | `report-check: PASS (23 báo cáo, 155 hàng QUEUE phủ đủ)` | trước khi lật hàng: [`st130b-report-check-truoc-queue.log`](st130b-report-check-truoc-queue.log) `b0bc8c9d…`; chạy lại **sau** khi lật hàng ST-130 → DONE: [`st130b-report-check.log`](st130b-report-check.log) `1a6cace9…` (commit `[ST-130q]`) |

Số khoá **không đổi**: sem **447 / 22 nhóm**, `vis test` **447 ok** / `--vm` **447 ok**, diff 100k
**lech=0**. Không thêm crate, không `unsafe`, R1 của tệp test = 264 dòng (≤1.500).

## 5. Tệp đổi

- `crates/vis-cli/tests/siet_caps_pkg.rs` — **thêm 1 `#[test]`** (8 tổng), không sửa ca cũ.
- `agent/JOURNAL/st130b-*` — journal này + 9 log bằng chứng.
- `agent/STATE.md` — mục ST-130b (lịch sử).
- `agent/QUEUE.md` — **chỉ hàng ST-130**: `READY` → `DONE` + dòng CHỜ DUYỆT Agent 1 (giữ dòng CHỜ
  DUYỆT cũ và dòng XEM LẠI làm lịch sử; **không** ghi APPROVED — VD-143).

## 6. Chỗ **lệch** kế hoạch / ghi chú (không giấu)

1. **Không có log đỏ** (khác kịch bản "đỏ-trước" của 23f): hành vi đã đúng từ ST-130 gốc, nên theo đúng
   yêu cầu của task tôi ghi rõ "không có log đỏ thật vì hành vi đã đúng — ca khoá hồi quy" và dán đối
   chứng `--strict-caps ⇒ VS0604` (§2). Không sửa mã sản phẩm ⇒ **0 dòng** thay đổi ngoài test.
2. **Cổng `gate --fast` lần 1 đỏ vì flake tải máy** — `net_server::accept_het_gio` (đo `vm=false`
   **426 ms** > 400 ms), **không** liên quan ca của tôi: log
   [`st130b-gate-fast-lan1-flake.log`](st130b-gate-fast-lan1-flake.log) `d318cc32…`; lúc đó
   `load average = 13.75` trên **12 core** do **agent khác** đang chạy `cargo test`/build trong cùng repo.
   Phân loại bằng **một** lần chạy riêng (đúng ca đỏ) — cũng đỏ (`586 ms`, log
   [`st130b-phan-loai-net-server.log`](st130b-phan-loai-net-server.log) `a9afa2fd…`) ⇒ kết luận **T-INFRA**
   (ngưỡng 400 ms là ngưỡng sản phẩm, **không** nới; ca không thuộc phạm vi ST-130, không sửa).
3. **Cổng chạy trên cây sạch**: lúc chạy lại, **cây làm việc chia sẻ** có 6 tệp của **agent khác** đang
   sửa dở (`crates/vis-cli/src/{main,ngoai,quyen}.rs`, `tests/{pipeline,sandbox}.rs`,
   `crates/vis-interp/src/builtin.rs`) ⇒ `cargo fmt --check` toàn workspace đỏ ở **tệp của họ** (không
   phải của tôi; log lần 2 [`st130b-gate-fast-lan2-fmt-cay-ban.log`](st130b-gate-fast-lan2-fmt-cay-ban.log)
   `9db236af…`, dòng `--- fmt: FAIL`). Vì vậy cổng PASS được đo trên **worktree sạch**
   `/tmp/st130b-gate-wt` tại HEAD lúc chạy `3ea7176d` + **đúng** tệp test sẽ commit (md5
   `dbfd4de055fa148decc8390f2fc67441`, khớp md5 tệp trong cây chính); toàn bộ HEAD đó **không** chứa
   thay đổi dở của agent khác. Đây là cách duy nhất đo được cổng của **đúng** bản sẽ commit mà không
   đụng vào việc của người khác (cấm stash/reset/checkout). Log:
   [`st130b-gate-fast.log`](st130b-gate-fast.log) `52592bf7…`.
