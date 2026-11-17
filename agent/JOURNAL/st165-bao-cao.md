# ST-165 — nợ **#138**: `/proc` đọc được trên Linux (nếu không ⇒ `VS0604`, người vận hành phải biết)

Người ghi: **Agent 2** dòng ViScript · 2026-11-17 · HEAD lúc đo: mac/ps `6f8f63a5` + **một tệp sửa chưa commit**
(`viscript/crates/vis-cli/src/quyen.rs`) · ngân sách 1 h (dừng mềm 13:15 JST).

Nguồn: `agent/SO-NO.md` hàng `| 138 |` · `reports/R24-viscript.md` §5 #138 · lịch sử **ST-137** (VD-141) ·
`crates/vis-cli/src/quyen.rs:500-503` (`KhongLayDuoc` ⇒ từ chối) · `quyen.rs:1068-1112` (ca "không đọc được
`/proc/self/fd/3`") · `crates/vis-cli/src/ngoai.rs:286-289` (macOS `KhongCo`).

---

## 1. Kết luận trước (tóm tắt)

**Không có lỗi "đọc `/proc` bị `VS0604` oan" trên Linux khi `/proc` đọc được.** Đo thật trên `ps`:

| Ca (cap[Dir] `/proc`) | Trước ST-165 | Sau ST-165 |
|---|---|---|
| `fs.read("/proc/self/status")` | **OK** (không `VS0604`) | **OK** (không đổi) |
| `fs.read("/proc/self/cmdline")` | **OK** | **OK** |
| `fs.read("/proc/version")` | chưa chạy tới (ca dưới trap trước) | **OK** |
| `fs.read("/proc/self/fd/0")` (stdin ⇒ `/dev/null`) | `VS0604` + **chỉ** "thiếu `cap[Dir]` cho `/proc/self/fd/0`" | `VS0604` + **nêu đích thật**: "`/proc/self/fd/0` phân giải tới `/dev/null` — ngoài mọi `cap[Dir]`/`cap[File]`" |
| `/proc` **không** đọc được (mô phỏng `unshare -m` + tmpfs lên `/proc`) | `VS0604` "không xác định được đường dẫn thật … `/proc/self/fd/3`: ENOENT" | thêm hướng dẫn: "Trên Linux cần procfs đọc được tại `/proc/self/fd` … hãy kiểm/mount procfs" |

**Đỏ-trước là chẩn đoán, không phải quyền đọc**: cả hai ca từ chối ở trên đều **đúng** theo thiết kế ST-137
(từ chối khi không xác định được **vật đã mở**; không quay về kiểm theo tên). Nhưng thông điệp cũ **không cho
người vận hành biết vì sao** — đúng nội dung nợ #138 ("hành vi đúng, nhưng người vận hành phải biết"). Sửa
**tối thiểu 2 chỗ thông điệp** trong `quyen.rs`, **không** đổi logic quyết định, không ngưỡng, không `unsafe`,
không crate mới.

---

## 2. Đỏ-trước (bằng chứng)

### 2.1 Trên `ps` (Linux thật, `bin/vis-remote`, log ngoài repo + bản sao JOURNAL)

`JOURNAL/st165-proc-do-truoc.log` — lượt 1 **sai công cụ** (script thiếu `use std.str` nên `str.contains`
không có trong phạm vi ⇒ `VS0501`), **giữ nguyên không ghi đè**; kết quả dùng được ghi ở lượt 2.

`JOURNAL/st165-proc-do-truoc-lan2.log` — `ps`, binary build từ HEAD `f2d2e72d` (**trước** khi sửa), `cap[Dir] @ p"/proc"`:

```
A-STATUS-OK            ← /proc/self/status đọc được, KHÔNG VS0604
true
B-CMDLINE-OK
proc2.vis:18:11: error VS0604: [quyền] thiếu `cap[Dir]` cho `/proc/self/fd/0`
EXIT=2
```

⇒ (a) `/proc` **đọc được** thì đọc status/cmdline **được** — tức là không có lỗi "VS0604 oan" cho tệp procfs
thường; (b) ca `/proc/self/fd/0` (stdin của tiến trình = `/dev/null`) bị từ chối **đúng** nhưng thông điệp
**không nêu** đích `/dev/null` ⇒ người khai `cap[Dir]` cho `/proc` sẽ tưởng cap của mình khai sai.

### 2.2 Unit test đỏ-trước (mac)

`JOURNAL/st165-do-truoc-test.log` — bản "trước" = `git show HEAD:…/quyen.rs` **+ 3 ca mới của ST-165**:

```
quyen::tests::doi_tuong_khong_phai_duong_dan_bao_ro_ly_do --- FAILED
quyen::tests::thong_diep_khong_doc_duoc_proc_co_huong_dan_van_hanh --- FAILED
quyen::tests::thong_diep_tu_choi_neu_dich_that_khac_ten --- FAILED
…
phải nói rõ vật không có đường dẫn: VS0604:[quyền] thiếu `cap[Dir]` cho `/proc/self/fd/1`
phải hướng dẫn kiểm/mount procfs: … ⇒ từ chối (không quay về kiểm theo tên; ST-137)
thông điệp phải nói tên đã phân giải: VS0604:[quyền] thiếu `cap[Dir]` cho `…/root/link`
test result: FAILED. 3 passed; 3 failed
[exit=101]
```

Lượt ghi log đầu tiên bị sai `[exit=]` (đặt `$?` sau `echo` rỗng) — **giữ** ở
`JOURNAL/st165-do-truoc-test-lan1-sai-exit.log`, không ghi đè.

---

## 3. Sửa tối thiểu (chỉ `crates/vis-cli/src/quyen.rs`, +145 dòng kể cả chú thích/ca)

1. **`kiem_canon`** — thêm hai nhánh **chẩn đoán**:
   - `c` **không phải đường dẫn tuyệt đối** (vd `/proc/self/fd/1` → `pipe:[…]`): báo rõ
     "`p` trỏ tới đối tượng không phải đường dẫn (`pipe:[…]`) ⇒ không xác định được vật để kiểm quyền ⇒ từ chối
     (ST-165 · nợ #138)", **không** báo nhầm "thiếu `cap[Dir]`".
   - `p` tuyệt đối nhưng phân giải ra `c` khác: nêu **đích thật** "(`p` phân giải tới `c` — ngoài mọi
     `cap[Dir]`/`cap[File]` đã khai)".
   - Tách các nhánh cũ thành `return` sớm, **giữ nguyên** câu chữ cũ cho 3 ca cũ (strict-thiếu-cap,
     thiếu `(write)`, thiếu `cap[Dir]` khi `p` không đổi) ⇒ không đổi kỳ vọng test cũ.
2. **`xac_nhan_handle`** (`KhongLayDuoc`) — giữ nguyên quyết định **từ chối**, thêm một câu hướng dẫn vận hành:
   "Trên Linux cần procfs đọc được tại `/proc/self/fd`: thiếu procfs thì **mọi** lần đọc đều bị từ chối — hãy
   kiểm/mount procfs (ST-165 · nợ #138)".

**Không** đổi: thứ tự mở→lấy handle→đọc, `xac_nhan_sau_mo`, ngưỡng, `MAX_*`, kỳ vọng cũ, `#[ignore]`; không
`unsafe` mới; không crate mới. Tệp `quyen.rs` **1 284** dòng (< 1 500).

---

## 4. Xanh-sau (bằng chứng)

- **Unit (mac)**: `JOURNAL/st165-sau-test.log` — `cargo test -q -p vis-cli --bin vis -- quyen::tests` ⇒
  **6 passed / 0 failed**, `[exit=0]`.
- **`ps` E2E, binary sau sửa** (cùng cây nguồn đã sync): `JOURNAL/st165-proc-sau.log` —

```
A-STATUS-OK / true / B-CMDLINE-OK / C-VERSION-OK
proc3.vis:24:11: error VS0604: [quyền] thiếu `cap[Dir]` cho `/proc/self/fd/0`
  (`/proc/self/fd/0` phân giải tới `/dev/null` — ngoài mọi `cap[Dir]`/`cap[File]` đã khai)
EXIT=2
== thuong.vis (cap[Dir] /tmp/st165) ==
Ok(NOI-DUNG-TRONG)
EXIT=0
```

  ⇒ `/proc` đọc được ⇒ đọc status/cmdline/version **không** `VS0604` (đúng yêu cầu nợ); ca `/proc/self/fd/0`
  vẫn từ chối **đúng** nhưng nay **người vận hành biết** đích là `/dev/null`; tệp thường trong root vẫn đọc được.
- **Mô phỏng `/proc` không đọc được** (`unshare -m --propagation private` + `mount -t tmpfs tmpfs /proc`,
  **chỉ trong namespace của tiến trình**, không ảnh hưởng máy): `JOURNAL/st165-proc-khong-doc-duoc.log` —

```
PROC_MUC=2
thuong.vis:4:11: error VS0604: [quyền] không xác định được đường dẫn thật của tệp đã mở cho `/tmp/st165/a.txt`
  (không đọc được `/proc/self/fd/3`: No such file or directory (os error 2)) ⇒ từ chối (không quay về kiểm theo tên; ST-137).
  Trên Linux cần procfs đọc được tại `/proc/self/fd`: thiếu procfs thì **mọi** lần đọc đều bị từ chối — hãy kiểm/mount procfs (ST-165 · nợ #138)
EXIT=2
```

  ⇒ đúng hành vi đã ghi trong nợ ("`/proc` phải đọc được trên Linux, nếu không ⇒ `VS0604`"), nay có **hướng dẫn
  vận hành** ngay trong thông điệp.

---

## 5. Đóng nợ **#138** (đề nghị Agent 1)

- Hành vi **không đổi** (vẫn từ chối khi không xác định được vật đã mở) ⇒ **không** cần đổi cơ chế/thiết kế,
  **không** BLOCKED.
- Yêu cầu của nợ đã được kiểm **hai chiều** trên Linux thật: `/proc` đọc được ⇒ đọc được; `/proc` không đọc
  được ⇒ `VS0604` **kèm hướng dẫn** mount/kiểm procfs; ca "đích không phải đường dẫn" (`pipe:`/`socket:`) và
  "tên phân giải ra đích khác" nay có chẩn đoán rõ ràng.
- Đề nghị Agent 1: đổi hàng `| 138 |` trong `agent/SO-NO.md` sang **`đóng ở ST-165`** + ghi 3 tệp log bằng
  chứng (mục 2/4). **Agent 2 không sửa** `SO-NO.md`/`QUEUE.md`/`DECISIONS.md` theo luật.

## 6. Cổng, commit

- **macOS** (`JOURNAL/st165-gate-macos.log`, chạy `../vis-dev gate --fast` tại HEAD `6f8f63a5` + 1 tệp sửa):
  **PASS** `[exit=0]`, **10/10** bước `--- …: OK` (fmt · clippy · luật R1..R4 · self-test case âm · vis explain ·
  báo cáo `report-check: PASS (28 báo cáo, 192 hàng QUEUE phủ đủ)` · báo cáo fixtures · build · doc-examples
  24 khối · L0), L0 **1182 passed / 0 failed** (**131** suite = 1179 cũ + **3** ca mới chạy trên mac; ca
  `doc_proc_status_…` là `cfg(target_os = "linux")` nên không chạy trên mac).
- **`ps` (Linux thật, qua `bin/vis-remote`)**: `JOURNAL/st165-gate-ps.log` ⇒ **PASS** `[exit=0]`, **10/10** bước,
  L0 **1186 passed / 0 failed** (**131** suite = 1182 cũ + **4** ca mới, gồm ca Linux), log có header VD-136 +
  sha256 bản trên máy đích. Trước khi chạy đã `bin/vis-linux sync` (8/8 mục) + build lại `vis` bằng toolchain
  `1.99.1` (`bash -lc`, `/root/.cargo/bin/cargo`).
- **Tệp đã sửa**: `crates/vis-cli/src/quyen.rs` (1 284 dòng; sha256 `b5f1f9cd0c65c2251e262c501ca9330534cb0a983b22ef4fbaab7a4a01c56cf0`)
  + 8 tệp bằng chứng trong `agent/JOURNAL/` (`st165-*.log`) + tệp báo cáo này. **Không** sửa `QUEUE.md`,
  `DECISIONS.md`, `SO-NO.md`, `spec/`, `docs/`, `v2/`; **không** `unsafe`/crate mới/`#[ignore]`/nới ngưỡng.
- **Commit**: `[ST-165]` ngay sau báo cáo này (sha do Agent 2 gửi Lead trong câu trả lời cuối) · **Push**:
  `git push origin main` (nếu bị từ chối: `git fetch` + `git merge --ff-only origin/main` rồi push lại).
