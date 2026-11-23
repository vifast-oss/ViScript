# ST-134 · Vá runner + ca theo nền + điều tra ca (iv) (2026-11-16)

Người làm: Agent 2 dòng ViScript. Căn cứ: `KE-HOACH-CHOT-23f.md` §2 **ST-134**, **VD-132** (ca phụ
thuộc nền), **VD-136** (script tái lập + log môi trường mỗi lần đo), **VD-116** (điều tra trước, sửa
sau), **VD-141/143** (không lật DONE sớm). Runner: `bin/vis-linux` backend `ssh` → host `ps`.

## 0. Kết luận một dòng

`bin/vis-targets` **18/18 `check` PASS** trên `ps` sau khi chạy script tái lập
`viscript/infra/linux/setup.sh` (Vòng 1 thiếu `wasm32` **và** `x86_64-pc-windows-msvc` + thiếu
`vis-host-wasm/`); hai ca theo VD-132 xanh **cả macOS và `ps`**; ca (iv) kết luận **(b) lỗi sản phẩm,
sửa được ≤150 dòng** (68 dòng: phân loại thân `cmd` một dòng) — đã sửa, `vish_cmd_body` **16/16 hai
nền**; `gate --fast` **PASS**, `check_report --all` **PASS**.

## 1. 2a — script tái lập toolchain trong repo (VD-136)

| Tệp | Việc | Dòng/sha256 |
|---|---|---|
| `viscript/infra/linux/setup.sh` (mới) | kiểm + cài **rustup target** còn thiếu, in **log môi trường + digest** mỗi lần chạy, chỉ `rustup target add` | sha256 `a527a5038cef11d444869b4d1273ec604f28175b0c5f4d4b7ada6144878ce510` |
| `viscript/infra/linux/setup.md` (mới) | cách gọi (trực tiếp / `bin/vis-linux setup` / tự gọi trong `run`), cái gì **không** cài, mã thoát | sha256 `eec0caec9454ce62abc9403113cfe6578b271b28ceb7c394befaf7fba414afe0` |
| `viscript/bin/vis-linux` | thêm `setup [--log]`; `run`/`gate` **tự gọi** setup sau sync (idempotent, tắt bằng `VIS_LINUX_NO_SETUP=1`); sync thêm `vis-host-wasm/` | sha256 `1b7c79b44c176c91d6c09f4f12bbb481e54f051cb7df0129e6991baca30a2cf3` |

**Hai target, không phải một** (xem §7 lệch 1): `wasm32-unknown-unknown` (R23 §4.4 lệch 5) **và**
`x86_64-pc-windows-msvc` — ma trận `bin/vis-targets` có **8 ô** `cargo check --target
x86_64-pc-windows-msvc`; thiếu std của target thì 8 ô đó đỏ, 18/18 là bất khả thi. Cả hai là **rustup
target** (thành phần toolchain), không phải phần mềm hệ thống ⇒ đúng phạm vi VD-136.

Mỗi lần chạy, script in/nối vào `${VIS_SETUP_LOG:-$HOME/.cache/vis-setup.log}`: `hostname`, `os`,
`kernel`, `rustc -Vv` (đầy đủ), `cargo -V`, `rustup --version`, `active-toolchain`, `git HEAD`,
`setup.sh sha256`, `digest-toolchain` (sha256 của rustc+cargo+rustup+active+target+HEAD), target
trước/sau, và dòng `=== setup kết thúc: HEAD=… digest=… ===`.

### Đo trên `ps`

Log: `JOURNAL/st134-setup-ps.log` — sha256 **`33a96e52be194ed4bfc9a038f2eafc76c662f30c8a1e968a318d6fa146f3967e`**.

```
hostname: ps
os: Ubuntu 26.04.1 LTS
kernel: Linux 7.0.0-43-generic x86_64
rustc 1.99.1 (58cf8f9c3 2026-10-13)
active-toolchain: 1.99.1-x86_64-unknown-linux-gnu (default)
git HEAD: ee73989a… (lần cài đầu)
digest-toolchain: d677c675… → sau: 200bcc113feaa2f43f729cf8ddadbd637a7c380e971bf682e6f2f0e6c0ea1388
target đang có (trước): x86_64-unknown-linux-gnu
target đang có (sau): wasm32-unknown-unknown x86_64-pc-windows-msvc x86_64-unknown-linux-gnu
setup: OK (2/2 target; rustc 1.99.1 (58cf8f9c3 2026-10-13))
```

Lần chạy lại (trong `run`) chứng minh **idempotent**: `target …: đã cài (không cài lại)` và
`digest-toolchain (sau)` **không đổi** (`200bcc11…`).

### `bin/vis-targets` — 18/18 trên `ps`

Log: `JOURNAL/st134-vis-targets-ps.log` sha256
**`bc356a5a45647f347ba7e6d4187cd403300a5819dfbcc6853fbab477fd2a2767`** — 18 dòng `PASS`
(8 crate × 2 target + `vis-host-wasm × wasm32` + `vis-host-linux × x86_64-linux-gnu`), dòng cuối
`vis-targets: tất cả ô kỳ vọng xanh (chỉ CHECK, chưa chạy trên Windows/wasm)`.
macOS: `JOURNAL/st134-vis-targets-macos.log` sha256
**`a617adb1ef96e0889fe2fda454d81f99356e7010ef61d634139a81750fd30484`** — cũng 18/18 PASS.
Mức bằng chứng vẫn **chỉ CHECK biên dịch chéo** (VD-121), không tuyên bố chạy Windows/wasm.

## 2. 2b — hai ca theo VD-132 (không xoá, không ignore, không cfg bỏ ca)

### 2b.1 `crates/vis-cli/tests/pipeline.rs` — `complete_giu_stderr_va_code_khac_0`

Đỏ-trước trên `ps` (log `JOURNAL/st134-red-2ca-ps.log`, sha256 **`f249bdda341d4c834f8acf298659df67f597bf0d2c8166140b66eabfe1421a1a`**):

```
panicked at crates/vis-cli/tests/pipeline.rs:133:5:
stderr phải có trong record: stdout  []
stderr  [/usr/bin/grep: unrecognized option '--khong-co', …]
code    2
```

Sau sửa (log `JOURNAL/st134-green-2ca-ps.log`, sha256 **`c9c3372508d61247da5a1d5d939cdabf2f574d7eecef768ba8e5bab8f57d598b`**):

```
test complete_giu_stderr_va_code_khac_0 ... ok
test result: ok. 1 passed; 0 failed; …
```

### 2b.2 `crates/vis-cli/tests/sandbox.rs` — `sandbox_check_luon_in_cuong_che`

Đỏ-trước trên `ps` (cùng log đỏ):

```
panicked at crates/vis-cli/tests/sandbox.rs:59:5:
assertion `left == right` failed: macOS:
  left: 0
 right: 3
```

Sau sửa: `test sandbox_check_luon_in_cuong_che ... ok` (cùng log xanh), trong đó nhánh Linux khẳng
định thêm **dòng báo ABI** bằng một lần chạy `--sandbox` trong **cùng ca**:
`err_sb.contains("sandbox: landlock ABI=")` và `err_sb.contains("mức=đầy đủ")`.

### 2b.3 Diff hai ca (dán nguyên văn)

```diff
diff --git a/viscript/crates/vis-cli/tests/pipeline.rs b/viscript/crates/vis-cli/tests/pipeline.rs
@@ -130,9 +130,19 @@ fn complete_giu_stderr_va_code_khac_0() {
     assert_eq!(c, 0, "stderr: {e}");
     // **SỬA KÈM LÝ DO (ST-41)**: `complete` trả **một bản ghi** ⇒ giá trị cuối pipeline in **khối
     // khoá–giá trị** (`Render`, §9.6): `stderr  […]` (hai dấu cách), không còn `stderr: […]`.
+    //
+    // **VD-132 (ST-134) — ca phụ thuộc nền**: thông điệp lỗi của `grep` in **tên chương trình** theo
+    // chuẩn của nền: GNU grep (Linux) in `argv[0]` nguyên văn — ta gọi `/usr/bin/grep` nên là
+    // `/usr/bin/grep:`; BSD grep (macOS) in tên chương trình `grep:`. Mỗi nhánh khẳng định **giá trị
+    // cụ thể**; record vẫn phải mang đủ stderr + `code 2` (sản phẩm không đổi).
+    let tien_to = if cfg!(target_os = "linux") {
+        "stderr  [/usr/bin/grep:"
+    } else {
+        "stderr  [grep:"
+    };
     assert!(
-        o.contains("stderr  [grep:"),
-        "stderr phải có trong record: {o}"
+        o.contains(tien_to),
+        "stderr phải có trong record với tiền tố {tien_to:?}: {o}"
     );
@@ -50,15 +50,58 @@ fn sandbox_check_macos() {
-/// Ca 2 — **luôn** in `cưỡng chế runtime:` và `cap điều kiện:`.
+/// Ca 2 — **luôn** in `cưỡng chế runtime:` và `cap điều kiện:`; exit theo **nền** (VD-132, ST-134).
+///
+/// macOS: không có Landlock ⇒ **3**. Linux **có** Landlock (runner `ps`) ⇒ **0**, kèm dòng báo LSM;
+/// đồng thời `--sandbox` phải áp ruleset thật và in **dòng báo ABI** (`sandbox: landlock ABI=…`).
 #[test]
 fn sandbox_check_luon_in_cuong_che() {
     let d = tmp_dir("in");
     ghi(&d.join("f.vis"), "fn main():\n    print(1)\n");
     let (code, out, err) = chay(&d, &["run", "--sandbox-check", "f.vis"]);
-    assert_eq!(code, 3, "macOS: {err}");
     assert!(out.contains("cưỡng chế runtime:"), "stdout: {out}");
     assert!(out.contains("cap điều kiện:"), "stdout: {out}");
+
+    // **VD-132**: mỗi nhánh nền khẳng định một **giá trị cụ thể**; không xoá ca, không cfg bỏ ca.
+    #[cfg(target_os = "macos")]
+    {
+        assert_eq!(code, 3, "macOS không có Landlock ⇒ 3: {err}\nstdout: {out}");
+        assert!(
+            out.contains("landlock: không hỗ trợ trên macos"),
+            "stdout: {out}"
+        );
+    }
+    #[cfg(target_os = "linux")]
+    {
+        let lsm = std::fs::read_to_string("/sys/kernel/security/lsm").unwrap_or_default();
+        let co_landlock = lsm.split(',').any(|x| x.trim() == "landlock");
+        if co_landlock {
+            // Linux **có** Landlock (máy đo `ps`): probe chỉ đọc LSM ⇒ exit 0 + dòng "có (LSM: …)".
+            assert_eq!(code, 0, "Linux có Landlock ⇒ 0: {err}\nstdout: {out}");
+            assert!(out.contains("landlock: có (LSM: "), "stdout: {out}");
+            // **Dòng báo ABI**: `--sandbox` áp ruleset thật, in `sandbox: landlock ABI=… mức=… nnp=…`.
+            let (code_sb, _out_sb, err_sb) = chay(&d, &["run", "--sandbox", "f.vis"]);
+            assert_eq!(
+                code_sb, 0,
+                "--sandbox trên chương trình `print(1)` phải chạy được: {err_sb}"
+            );
+            assert!(
+                err_sb.contains("sandbox: landlock ABI="),
+                "thiếu dòng báo ABI: {err_sb}"
+            );
+            assert!(
+                err_sb.contains("mức=đầy đủ"),
+                "kernel phải cưỡng chế đầy đủ: {err_sb}"
+            );
+        } else {
+            // Linux **không** có Landlock: hợp đồng là 3 (LSM không có) / 4 (không đọc được).
+            assert!(
+                code == 3 || code == 4,
+                "Linux không có Landlock ⇒ 3/4, thật {code}: {err}"
+            );
+            assert!(out.contains("landlock: "), "stdout: {out}");
+        }
+    }
     let _ = std::fs::remove_dir_all(&d);
 }
```

Toàn bộ **hai tệp** sau sửa xanh trên **cả hai nền** (log `JOURNAL/st134-green-files-ps.log`, sha256
**`c5edae41572da788fb57c5722157ab483f9d46a953cdc6eaf50aafc9d6c7c0e3`**): `pipeline` **16/16**,
`sandbox` **6/6**; macOS đo cùng ngày: `pipeline` **16/16**, `sandbox` **6/6**.

## 3. 2c — điều tra ca (iv): `than_lenh_tren_thu_muc_that` (`vish_cmd_body.rs:312`)

### 3.1 Kết luận: **(b) lỗi sản phẩm, sửa được ≤150 dòng** — đã sửa (68 dòng)

Không phải lỗi ca kiểm. Đỏ vì **phân loại thân `cmd`** trong `vis-types` chọn **biểu thức script**
khi một dòng vừa parse được ở cả hai chế độ:

- Thân `cmd` một dòng `ls /tmp/…/that`:
  - parse **script** (ST-44 đường "thân biểu thức") đọc `/tmp/…` thành **phép chia**
    `ls / tmp / vt / that` ⇒ HIR `Var("tmp")` ⇒ lúc chạy `VS0501: tên 'tmp' chưa hỗ trợ ở VS0`;
  - parse **lệnh** đọc đúng `ls` + đường dẫn tuyệt đối.
- Trên macOS, `std::env::temp_dir()` là `/var/folders/…`; `var` là **từ khoá** ⇒ parse script
  **thất bại** ⇒ rơi về nhánh **lệnh** ⇒ ca **xanh may mắn**, không phải vì sản phẩm đúng trên Linux.

Bằng chứng tách bạch (log `JOURNAL/st134-2c-do-truoc-ps.log`, sha256
**`46f24909dcd022685678034d6c0bd96886bc0a1a9131024022c051bb7c8ef109`**) — **cùng một binary CŨ**
(sha256 `c327862cf5f991a596bdb7cd3e7fb38d083fbb9f05a54786cb5b1241bcc11afb`) trên `ps`:

```
--- chạy trong thân cmd ---
/tmp/than-iv/l.vish:3:7: error VS0501: tên `tmp` chưa hỗ trợ ở VS0
EXIT=1
--- cùng đường dẫn, ls ở CẤP TỆP (ngoài thân cmd) ---
name   size               modified  is_file
x.txt    3b  …  true
EXIT=0
```

⇒ Lỗi nằm ở **phân loại thân `cmd`**, không phải ở `ls`/đường dẫn tuyệt đối.

### 3.2 Sửa (68 dòng: +65/−3, không `unsafe`, không crate mới)

| Tệp | Việc |
|---|---|
| `crates/vis-types/src/cmdsig.rs` | `pub fn ten_mac_dinh() -> Vec<String>` — tên **12 lệnh chuẩn** (`ls`, `open`, …) |
| `crates/vis-types/src/lower/khai.rs` | `ten_cmd_khai(text)` quét tên `cmd` khai cột 1; `than_cmd(text, ten_lenh)`; khi `script && lenh` và thân **một dòng**, nếu **từ đầu** là **tên lệnh đã biết** ⇒ `KieuThan::Command` (ST-48: "thân = đúng một pipeline") |
| `crates/vis-types/src/lower.rs` | truyền danh sách tên lệnh (bảng chuẩn + `cmd` khai trong tệp) vào `than_cmd` |

Hunk chính:

```diff
@@ -321,7 +350,21 @@ pub(crate) fn than_cmd(text: &str, ten_lenh: &[String]) -> Vec<ThanLenh> {
-                if dong_dedent.len() == 1 || co_let {
+                // **ST-134 (lỗi ca iv)**: … nếu **từ đầu** dòng là **tên lệnh đã biết** thì thân là
+                // **lệnh** (ST-48), không phải biểu thức. Ví dụ `ls /tmp/x`: script đọc thành phép
+                // chia `ls / tmp / x` (`tmp` chưa có ⇒ VS0501 lúc chạy) — phải chọn lệnh.
+                let dau = dong_dedent
+                    .first()
+                    .map(|l| l.trim_start())
+                    .and_then(|l| l.split_whitespace().next())
+                    .unwrap_or("");
+                let la_lenh_da_biet =
+                    !co_let && dong_dedent.len() == 1 && ten_lenh.iter().any(|t| t == dau);
+                if la_lenh_da_biet {
+                    KieuThan::Command
+                } else if dong_dedent.len() == 1 || co_let {
                     KieuThan::Script
                 } else {
                     KieuThan::Mixed
```

Sau sửa (log `JOURNAL/st134-sau-ps.log`): repro in đúng `x.txt`, `EXIT=0`; `vish_cmd_body` **16/16**
trên `ps` và **16/16** trên macOS; `test/sem` **447 ok, 0 `not ok`** (interp **và** `--vm`); gate
`--fast` PASS (L0 **1159/0** trên macOS); differential 100k seed `20261115` **lech=0** (`172,1 s`).

## 4. 2d — đủ **sáu** ca đỏ Linux của R23 §4.4 (không phải 4)

| # | Ca | Loại | Xử lý ở đâu |
|---|---|---|---|
| 1 | `complete_giu_stderr_va_code_khac_0` | **lỗi ca kiểm** (GNU in `argv[0]`, BSD in `grep:`) | **ST-134 (2b)** — áp **VD-132**, đã xanh hai nền |
| 2 | `quyen::tests::dua_doi_symlink_lap_co_gioi_han` | **lỗi sản phẩm** — rò rỉ TOCTOU **798/50 000** trên Linux, macOS 0/50 000 | **ST-137** (VD-141) — **không** sửa ở ST-134 |
| 3 | `sandbox_check_luon_in_cuong_che` | **lỗi ca kiểm** (viết cứng kỳ vọng macOS) | **ST-134 (2b)** — áp **VD-132**, đã xanh hai nền |
| 4 | `symlink_canonicalize_hai_phia` | **môi trường**: `echo` trên runner là symlink coreutils multicall ⇒ `argv[0]` khác | chưa sửa; cần quyết (chấp nhận "môi trường" hoặc đổi ca) |
| 5 | `targets_wasm_vis_types` | **môi trường/toolchain**: thiếu `wasm32-unknown-unknown` | **ST-134 (2a)** — `setup.sh` cài target; `vis-targets` 18/18 trên `ps` |
| 6 | `than_lenh_tren_thu_muc_that` (`.vish` + `/tmp`) | **lỗi sản phẩm** (không phải ca kiểm — xem §3) | **ST-134 (2c)** — sửa 68 dòng; 16/16 hai nền |

Tóm lại: **2 lỗi ca kiểm** (đều áp VD-132), **2 lỗi sản phẩm** (#2 ⇒ ST-137 đang xử lý; #6 ⇒ sửa
trong ST-134), **2 môi trường** (#4 chưa xử lý, #5 ⇒ setup.sh).

## 5. Hai nền — số đo đặt cạnh nhau

Cây đo: `ps` HEAD `e369698a` (nhánh chung sau ST-137/ST-139) + thay đổi **ST-134 chưa commit**;
sha256 các tệp trong log `st134-sau-ps.log`. macOS: cùng cây nguồn, `rustc 1.99.1`.

| Phép đo | macOS | Linux (`ps`) |
|---|---|---|
| `bin/vis-targets` (mức CHECK) | **18/18 PASS** | **18/18 PASS** (sau `setup.sh`) |
| `pipeline.rs` (16 ca) | **16/16** | **16/16** |
| `sandbox.rs` (6 ca) | **6/6** | **6/6** |
| `vish_cmd_body.rs` (16 ca) | **16/16** | **16/16** |
| `cargo test --workspace` | **1159 passed / 0 failed / 129 suite** (L0 trong `gate --fast`, cây cuối) | chưa đo trong task (ST-135) |
| `vis test test/sem` (interp) | **447 ok / 0 `not ok`** | chưa đo (ST-135) |
| `vis test --vm test/sem` | **447 ok / 0 `not ok`** | chưa đo (ST-135) |
| differential 100k seed `20261115` | **lech = 0**, 172,1 s | chưa đo (ST-135) |
| `gate --fast` | **PASS** (`fmt`, `clippy -D warnings`, R1..R4, self-test, explain, report, fixtures, build, doc-examples, **L0 1159/0**) — log `JOURNAL/st134-gate-fast-macos.log`, sha256 `d4336de7826cf306bcbb2233ce3e5708373741d08b5a2834f68bc6058b42460c` | — |
| `check_report.py --all` | **PASS** (nằm trong gate) | — |
| toolchain | rustc 1.99.1 | rustc 1.99.1, digest-toolchain `200bcc11…`, target: wasm32 + windows-msvc + linux-gnu |

Ghi chú: 1159 = 1153 (R23) **+6 test** do các commit **ST-130b**, **ST-137**, **ST-139** của agent khác
đã hợp nhất trước khi đo cổng cuối (lần đo `cargo test --workspace` riêng ở base `1097a81c` cho **1155/0**, log
`JOURNAL/st134-workspace-macos.log`) — **không** phải test của ST-134 (task này không thêm test nào).

## 6. Lệnh tái lập

```bash
# 1) toolchain + log môi trường (idempotent)
bash viscript/bin/vis-linux setup
# 2) ma trận nền tảng
bash viscript/bin/vis-linux run bash -c 'cd viscript && bash bin/vis-targets'
# 3) hai ca theo VD-132 + ca (iv) trên Linux
bash viscript/bin/vis-linux run bash -c 'cd viscript && cargo test -p vis-cli --test pipeline \
  && cargo test -p vis-cli --test sandbox && cargo test -p vis-cli --test vish_cmd_body'
# 4) cổng trên máy dev
bash viscript/bin/vis-dev gate --fast
python3 viscript/test/report/check_report.py --all
```

## 7. Chỗ lệch kế hoạch

1. **`setup.sh` cài 2 target, không chỉ `wasm32`**: QUEUE chỉ nêu `wasm32`, nhưng `bin/vis-targets`
   có **8 ô Windows** (`x86_64-pc-windows-msvc`); thiếu target đó thì tiêu chí "18/18" bất khả thi.
   Vẫn đúng phạm vi VD-136 (rustup target = thành phần toolchain).
2. **Thiếu `vis-host-wasm/` trong sync** (không phải target): lần đo đầu 17/18 vì
   `manifest path /root/vifast-linux/vis-host-wasm/Cargo.toml does not exist`; đã thêm thư mục vào
   `sync_tree` của `bin/vis-linux`. Đây là "vá runner" ngoài dự kiến.
3. **"Dòng báo ABI" của VD-132**: `vis run --sandbox-check` **không** in ABI (chỉ in dòng LSM
   `landlock: có (LSM: …)`); ABI chỉ in ở `--sandbox` (`sandbox: landlock ABI=… mức=…`). Ca giữ
   nguyên exit 0 + dòng LSM, và khẳng định **dòng báo ABI** bằng **một lần chạy `--sandbox` trong
   cùng ca** — **không** sửa sản phẩm, không nới ngưỡng. Nếu Agent 0 muốn ABI xuất hiện ngay trong
   `--sandbox-check` thì cần một quyết định riêng (đổi hợp đồng lệnh chẩn đoán).
4. **Ca (iv) vượt "điều tra"**: kết luận là **(b) lỗi sản phẩm ≤150 dòng** nên đã sửa 68 dòng ngay
   trong task (thay vì chỉ lập kế hoạch). R23 ghi "chưa đủ bằng chứng kết luận" — nay có bằng chứng
   tách bạch (thân `cmd` vs cấp tệp, cùng binary).
5. **macOS không đo được `vis-host-linux`** (crate chỉ biên dịch trên Linux) — dùng `ps` cho toàn bộ
   bằng chứng Landlock; macOS chỉ đo `vis` CLI. Không phải thiếu sót mới (từ ST-132).

## 8. Nợ mới (đánh số tiếp sau 131; ST-132b đã dùng 132–133)

- **134** — `setup.sh` cài thêm `x86_64-pc-windows-msvc` (ngoài `wasm32` mà hàng QUEUE nêu) để
  `bin/vis-targets` đạt 18/18; cần Agent 0 chuẩn thuận việc cài target Windows trên runner Linux.
- **135** — `bin/vis-linux` phải sync thêm `vis-host-wasm/`; chưa có ca/kiểm tự động cho danh sách
  thư mục sync (thêm thư mục mới cho ma trận nền tảng là sửa tay).
- **136** — phân loại thân `cmd` một dòng dựa trên **tên lệnh đã biết**; nếu người dùng có **biến
  trùng tên lệnh** (`let ls = …`) và muốn thân là biểu thức thì bị hiểu là lệnh — hiện chỉ có đường
  thoát `( … )`; chưa có ca khoá và chưa có cú pháp tường minh.
