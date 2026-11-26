<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# REVIEW ST-318b — kiểm toán độc lập đóng `#202` (chuẩn hoá `ms` trong `host_parity::net_http`)

> **Người kiểm**: Gatekeeper (Agent 1 độc lập), 2026-11-27 (giờ thấp điểm).
> **Đối tượng**: nhánh `plan/st318b`, HEAD **`a2b489391d838d9914cec4a1d0609b62f203b637`**,
> nền **`2b5ccb512522c80ac0170a92e68168820dc24729`**; 2 commit `68aeb4b3` / `a2b48939`.
> **`main` đối chiếu**: `2c7dbef6` (`[ST-00] VD-296`).
> **Worktree**: `/home/ps/viswt/st318b` — **không sửa mã**, không push, không `cargo clean`.
> **Cấu hình đo**: `CARGO_BUILD_JOBS=1`, cargo **1.100.0** (`$HOME/.cargo/bin/cargo`; cargo trên
> `PATH` là 1.93.1 nên `vis-dev` tự chọn đúng theo `rust-toolchain.toml`). Đĩa `/home` **9,2 GB
> trống** (> 6 GB) trước/sau. Log ngoài repo tại `/home/ps/vislogs/` (R7).
>
> **Kết luận: PASS.** **0 P1, 0 P2 lỗi mã.** 2 ghi chú P3 (giới hạn nhận biết của bộ chuẩn hoá —
> chấp nhận được theo mục tiêu `#202`, xem §6). Đủ điều kiện A0 đóng `#202` + `ST-318b` sau khi
> cập nhật `SO-NO.md`/`QUEUE.md`.

## 0. TL;DR

| # | Hạng mục | Kết quả | Bằng chứng chính |
|---|---|---|---|
| 1 | Phạm vi git | **PASS** | `git status --short` rỗng; 2 commit; **3 tệp** = 1 test + journal + `STATE.md`; **0 tệp sản phẩm** |
| 2 | Nội dung sửa | **PASS** | `chuan_hoa_ms` chỉ thay dãy chữ số ASCII ngay trước `" ms"` → `<N>`; 7 dòng / 4 dòng byte-exact / 2 dòng timeout / `Err` cổng đóng / 12 request **giữ nguyên** |
| 3 | Ca đích 20× | **PASS** | **20/20**, 0 flake (`--test-threads=1`, `jobs=1`); thêm **24/24** khi 8 process song song × 3 đợt |
| 4 | `host_parity::net_http` | **PASS** | **4/4** (2 parity + unit test mới + `chuan_hoa_cong` của `net`) |
| 5 | `gate --fast` | **PASS** | exit 0; **3902/3902** test (4 skipped); `lib_api_it` **396 chạy + 3 ignored = 399**; fmt/clippy/R1..R10/self-test/DAG/doc-examples OK |
| 6 | `rules` | **PASS** | `--- rules: OK`, exit 0 |
| 7 | Merge-tree `main` | **PASS (dự kiến)** | cây `c9ef8825`; **chỉ** xung đột `agent/STATE.md`, 0 tệp khác |
| 8 | R8 / R1 | **PASS** | cấp 1 **39/40** (`vis-cli` **8/15**), **0 tệp test mới**, **0 `#[ignore]`**; tệp sửa **484** dòng (< 1.500) |
| 9 | Rủi ro | **PASS** | Không nới test thành rỗng; không che lỗi cấu trúc; 2 P3 ghi nhận (§6) |

## 1. Phạm vi git

```
git log --oneline 2b5ccb51..HEAD
a2b48939 [ST-318b] journal/STATE: dong #202 (20/20, gate fast + rules PASS)
68aeb4b3 [ST-318b] #202: on dinh output host_parity net_http (bo ms that)
```

`git diff --stat 2b5ccb51..HEAD` — **3 tệp, +131/−3**:

| Tệp | Thay đổi | Loại |
|---|---|---|
| `crates/vis-conformance/tests/misc_api_it/host_parity/net_http.rs` | +55/−3 | **test only** |
| `agent/JOURNAL/st318b-on-dinh-net-http.md` | +74 | journal |
| `agent/STATE.md` | +5 | trạng thái |

`git diff --name-only` chỉ có 3 đường trên ⇒ **0 tệp sản phẩm** (`crates/*/src/**` không chạm),
0 `Cargo.toml`, 0 tệp test cấp 1 mới, 0 nhánh cấm. `git status --short` **rỗng** trước và sau
toàn bộ phiên kiểm.

## 2. Nội dung sửa — không nới test

### 2.1 `chuan_hoa_ms` (net_http.rs:366–389)

Thuật toán: lặp `find(" ms")`, lùi qua **dãy byte chữ số ASCII** liền trước; nếu có số thì thay
dãy đó bằng `<N>` rồi giữ nguyên `" ms"`; nếu không có số thì giữ nguyên cả cụm và đi tiếp.

Kiểm từng điểm:

- **Kết thúc**: nhánh không-số tiến `i + 3` (≥ 3 byte); nhánh có-số đặt `con = &con[i..]` với
  `i > 0` (vì `i == 0` ⇒ `b` rỗng ⇒ nhánh không-số) ⇒ mỗi vòng luôn tiến, không lặp vô hạn.
- **Biên UTF-8**: `find` trả byte index; `i + 3` cắt sau `" ms"` (3 byte ASCII); `j` lùi trong
  dãy `is_ascii_digit()` nên luôn dừng ở đầu một ký tự ⇒ mọi slice đều đúng biên, không panic.
  Chuỗi tiếng Việt (`quá thời gian chờ dữ liệu từ`) đi qua nguyên vẹn.
- **Độ hẹp**: chỉ xoá **giá trị số** của token ngay trước `" ms"`; toàn bộ chữ, tên handle
  (`C9`/`C10`), tên hàm, dấu câu, `ms`, URL, cổng vẫn so **từng byte** qua `hai_duong_chuan_hoa`
  (`o1 == o2`, `e1 == e2`, exit bằng nhau và bằng 0).
- **Chỉ dùng cho ca đích**: `git grep chuan_hoa_ms` = 1 định nghĩa + 1 unit test + 1 lời gọi
  trong `http_bon_ham_parity`; ca `net` vẫn dùng `chuan_hoa_cong`, không bị ảnh hưởng.

### 2.2 Ca `http_bon_ham_parity` (net_http.rs:408–489)

Đối chiếu với bản nền, **không assert nào bị bỏ**:

| Kiểm | Bản nền | HEAD |
|---|---|---|
| Đúng **7 dòng** stdout | có | **giữ** (`dong.len() == 7`) |
| 4 dòng OK/HEAD/`get_timeout`/`head_timeout` | `&dong[..4]` byte-exact | **giữ nguyên văn** |
| `slow=` hết giờ | `starts_with` + `contains("quá thời gian")` | **giữ** + thêm `ends_with(" sau <N> ms")` (**chặt hơn**) |
| `slowhead=` hết giờ | `starts_with` + `contains("quá thời gian")` | **giữ** + thêm `ends_with(" sau <N> ms")` (**chặt hơn**) |
| Cổng đóng | `starts_with("closed=")` + khác thông điệp Ok | **giữ nguyên** |
| Server thấy request | đúng **12** dòng method/path (6 × 2 engine) | **giữ nguyên**, so cả vec |
| exit hai engine | bằng nhau và = 0 | **giữ** (trong `hai_duong_chuan_hoa`) |

Server vẫn bind `http_server(12)`, `/cham*` ngủ 300 ms > timeout 80 ms, cổng đóng bind-rồi-thả
thật. Vì `ends_with(" sau <N> ms")` mới thêm, thông điệp không còn khả năng "rỗng hoá": phần
`quá thời gian chờ dữ liệu từ `C9`` vẫn phải khớp từng byte sau khi bỏ số.

### 2.3 Unit test khoá bộ chuẩn hoá

`chuan_hoa_ms_chi_thay_so_do_thuc` kiểm đúng 3 tính chất: hai dòng `78`/`79` → `<N>` (kèm giữ
nguyên `slow=`/`slowhead=`, handle, `ms`); chuỗi `"không có số ms"` **không đổi** (nhánh không-số);
`"123 ms"` → `"<N> ms"`. Đây là test của chính hàm chuẩn hoá, không thay thế ca parity.

## 3. Bằng chứng chạy lại (độc lập)

| Lần chạy | Lệnh | Kết quả |
|---|---|---|
| Build test | `cargo test -p vis-conformance --test lib_api_it --no-run` | exit 0, 44,9 s |
| **20 lần tuần tự** | `cargo test ... host_parity::net_http::http_bon_ham_parity -- --exact --test-threads=1` × 20 | **20/20 PASS**, mỗi lần `1 passed; 0 failed; 398 filtered out` |
| **24 lần song song** | 8 process × 3 đợt (cùng lệnh) | **24/24 PASS**, 0 panic/0 timeout |
| `host_parity::net_http` | `cargo test ... 'host_parity::net_http::' -- --test-threads=1` | **4/4 PASS**, 0,88 s |

Tổng cộng **44/44** lần ca đích xanh trong phiên kiểm này, không lần nào đỏ. Log:
`/tmp/opencode/st318b-run-{1..20}.log`, `/tmp/opencode/st318b-par-{1..3}-{1..8}.log`,
`/tmp/opencode/st318b-net_http.log`.

## 4. Cổng máy

### 4.1 `./vis-dev gate --fast`

Log: `/home/ps/vislogs/review-st318b-gate.log` — **exit 0**; sha256 hiện tại
**`4df7765f88d413edec37f7f81384e4fc55c113e7495196f2198a34b3e92d7149`**
(dòng `EXIT=0` + sha được nối cuối log sau khi cổng kết thúc).

- `vis-dag-check: OK — 23 crate / 5 tầng, 0 cạnh cấm`
- `report-check: PASS (35 báo cáo, 365 hàng QUEUE phủ đủ)`
- `--- luật R1..R10: OK` (self-test 20 ca, gồm R1 1.501 dòng, R3 SPDX, R10 API-CODES)
- `vis-dev check: PASS` (fmt + clippy + build + doc-examples)
- `Summary [94,951 s] 3902 tests run: 3902 passed, 4 skipped` — **0 failed, 0 retry**
  (`.config/nextest.toml` đặt `retries = 0`).
- `lib_api_it`: **396 PASS** in log + **3 test `#[ignore]`** thuộc `async_api_it` được `#[path]`
  vào (`lat256_budget::measure_workloads`, `lat257_b_vm_steps::measure_vm_instruction_counts`,
  `lat257_fairness::quet_q_phan_bo_khong_chon_default`) = **399** — khớp con số builder/`STATE.md`.
- `--- L0 test: OK`; `vis-dev gate --fast: PASS`.

### 4.2 `./vis-dev rules`

Log: `/home/ps/vislogs/review-st318b-rules.log` — **exit 0**, `--- rules: OK`;
sha256 **`36846bda2389faf4d8bf7efd4dba6aecbdfa7ea9e2c1744668c90a19b8dffc96`**.

## 5. Merge-tree với `main` (`2c7dbef6`, có `VD-296`)

```
git merge-tree --write-tree main HEAD
c9ef8825bb12302ff3271114e754dce8f41d5181
100644 ec4d6926... 1  agent/STATE.md
100644 08fb54aa... 2  agent/STATE.md
100644 294cf966... 3  agent/STATE.md
Auto-merging agent/STATE.md
CONFLICT (content): Merge conflict in agent/STATE.md
```

Exit 1 do **đúng một** xung đột nội dung `agent/STATE.md` — cả hai bên cùng chèn mục vào
`## 3. Nợ và quyết định` (`VD-296` bên `main` vs `ST-318b` bên nhánh). **Không** tệp nào khác bị
xung đột; `net_http.rs` và journal không nằm trong danh sách ⇒ A0 chỉ cần gộp tay hai đoạn
`STATE.md` khi nghiệm thu. Đúng dự kiến, không phải lỗi lát này.

## 6. Rủi ro và finding

**P1: không có. P2: không có.**

- **P3-1 — giá trị ms bị xoá nên `0 ms` cũng xanh.** Nếu một engine hồi quy kiểu "deadline hết
  ngay" và in `sau 0 ms`, ca parity vẫn PASS (miễn hai engine cùng in một dạng). Đây là cái giá
  bắt buộc của việc bỏ số đo thật; test vẫn đòi **đúng loại lỗi** `quá thời gian`, đúng handle,
  đúng cấu trúc đuôi. Nếu muốn chặt hơn: thêm assert `40 ≤ N ≤ 200` cho `dong[4]`/`dong[5]` sau
  khi tách số — đề xuất, không chặn.
- **P3-2 — chuẩn hoá áp cả stderr.** `hai_duong_chuan_hoa` chuẩn hoá `o` **và** `e`. Hiện stderr
  của ca này rỗng nên không ảnh hưởng; nếu sau này chẩn đoán có `"<số> ms"` thì giá trị đó cũng
  bị che. Phạm vi hẹp, không phải hồi quy của lát này.
- **Flake do nguyên nhân khác**: đã căng 44 lần (20 tuần tự + 24 song song) — 0 đỏ. Các yếu tố
  không ổn định còn lại (cổng ephemeral, thứ tự request) đã được xử lý từ `ST-315d`
  (`chuan_hoa_cong` + server đọc request trong vòng accept), không thấy dấu hiệu mới.
- **So sánh mất nghĩa?** Không: phần còn lại so byte-exact, 4 dòng OK/HEAD vẫn nguyên văn, hai
  dòng timeout còn **chặt hơn** nhờ `ends_with(" sau <N> ms")`, 12 request vẫn kiểm đủ.

## 7. Kết luận

**PASS.** Lát sửa **test-only**, đúng một nguyên nhân (`#202`: số ms là phần deadline còn lại đo
thật, hai engine chạy tuần tự lệch 1 ms), không nới test thành rỗng, không chạm sản phẩm, R8 giữ
39/40, cổng `gate --fast` + `rules` xanh, 44/44 lần ca đích xanh trong kiểm toán độc lập.

Đề nghị A0: đóng `#202` trong `agent/SO-NO.md`, đánh `DONE` hàng `ST-318b` trong `agent/QUEUE.md`,
và khi merge chỉ cần gộp tay đoạn `agent/STATE.md` (§5).
