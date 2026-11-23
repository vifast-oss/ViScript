<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-178 — Sửa tài liệu theo VD-178 + VD-180 + VD-181 (báo cáo)

**Ngày**: 2026-11-17 · **Người làm**: Agent 2 dòng ViScript · **Ngân sách**: 3 h (dừng mềm 01:15 JST)
**Nguồn quyết định**: `JOURNAL/quyet-dinh-a0-2026-11-17-vd178.md` (duyệt CÓ SỬA 6 tài liệu VS3 + `st169` + `vs-ergo.md`) · `…-vd180.md` (mô hình gói phi tập trung) · `…-vd181.md` (tên gói `vifast.org/vis/<tên>`, `sum.`/`proxy.`, Traefik không phải hợp đồng)
**Trạng thái**: **DONE phần tài liệu** · **0 dòng mã** · **chưa commit** (Agent 1 commit) · **1 mục chờ A0** (§6)

| | |
|---|---|
| Repo | `/Users/mps/dvchd/ViScript` (KHÔNG phải ViFastOs) |
| HEAD lúc làm | `505deaa` (`[ST-171k]`) — Agent 1 đã commit **trạng thái trung gian** của ST-178 trong commit này (xem §7) |
| Tệp sửa | 6 tài liệu VS3 + `vs-ergo.md` + `st169` + `CHEAT-SHEET-ViScript.md` (mới) |
| `check_report --all` | **PASS (29 báo cáo, 197 hàng QUEUE phủ đủ)** — log `JOURNAL/st178-check-report.log` |
| Mã | **0 dòng** (`crates/`, `bin/`, `test/` không đổi) |

---

## 1. Bảng mỗi [SỬA] → tệp:dòng đã sửa

### 1.1. `agent/JOURNAL/vs3-qd-1.md` (QĐ-1)
| [SỬA] VD-178 | Nội dung đã áp | đã sửa |
|---|---|---|
| QĐ-1 mục 1 | Đo song song bằng **task tính toán (CPU)**, bỏ sleep/IO ảo; ngưỡng **`0,6 × min(nproc,N,4)`**; ghi `nproc` + load + khoá đo | `agent/JOURNAL/vs3-qd-1.md:6` |
| QĐ-1 mục 2 | **Bỏ cấm await task ở worker khác**; luật Sendable + sao chép sâu lúc spawn/await; **VS0508** | `agent/JOURNAL/vs3-qd-1.md:20` |

### 1.2. `agent/JOURNAL/vs3-qd-2.md` (QĐ-2)
| [SỬA] VD-178 | Nội dung đã áp | đã sửa |
|---|---|---|
| QĐ-2 mục 1 | `chan(0)` = **rendezvous hợp lệ**; #145 = **xoá chú thích sai** `rt.rs:128-129` + **bỏ chặn runtime**, **không** thêm kiểm tyck; ca đỏ-trước 2c đổi thành “chan(0) chạy rendezvous” | `agent/JOURNAL/vs3-qd-2.md:6` |
| QĐ-2 mục 2 (áp dụng) | `recv_any` trả **`(int, T)?`**; hết giờ ⇒ **`None`**; mọi kênh cùng kiểu `T` | `agent/JOURNAL/vs3-qd-2.md:8` |
| QĐ-2 mục 3 (áp dụng) | **Công bằng vòng xoay** khi nhiều kênh cùng sẵn sàng; test bằng bộ đếm tất định | `agent/JOURNAL/vs3-qd-2.md:9` |

### 1.3. `agent/JOURNAL/vs3-qd-3.md` (QĐ-3)
| [SỬA] VD-178 | Nội dung đã áp | đã sửa |
|---|---|---|
| QĐ-3 mục 1 | Thêm **`http.client(ca: p"fixtures/ca.pem")`**; mặc định kho chứng chỉ HĐH (`rustls-platform-verifier`); **không** có insecure; nhánh dự phòng `rustls`+`webpki` nếu platform-verifier không thêm root | `agent/JOURNAL/vs3-qd-3.md:6` |
| QĐ-3 mục 2 (áp dụng) | Mặc định an toàn: `timeout_connect` **10 s**, `timeout_read` **30 s**, `max_redirect` **5**, `max_body` **8 MiB** — **đề xuất, chờ A0 chốt** | `agent/JOURNAL/vs3-qd-3.md:14` |

### 1.4. `agent/JOURNAL/vs3-qd-4.md` (QĐ-4)
| [SỬA] VD-178 | Nội dung đã áp | đã sửa |
|---|---|---|
| QĐ-4 mục 1 | **§4b là thiết kế chính** cho Linux **và** macOS: đi **từng thành phần** `openat`+`O_NOFOLLOW`; `openat2` đường nhanh + fallback; `..` theo từ vựng; macOS **hết khe hở bản chất**; còn lại **hardlink** | `agent/JOURNAL/vs3-qd-4.md:6` · `agent/JOURNAL/vs3-qd-4.md:20` (§3) |
| QĐ-4 mục 2 | Chứng minh bằng **ca tất định** 100/100 đỏ trước / 100/100 xanh sau; 500k chỉ là **bằng chứng phụ** | `agent/JOURNAL/vs3-qd-4.md:34` |
| QĐ-4 mục 3 (áp dụng) | **`rustix` được duyệt** (giấy phép MIT, xác minh `cargo metadata`, API an toàn, crate giữ `forbid(unsafe_code)`) | `agent/JOURNAL/vs3-qd-4.md:26` |
| QĐ-4 mục 4 (áp dụng) | **Windows** kiểm theo đường dẫn, **mức bảo đảm thấp hơn** (không hứa cùng mức sandbox) | `agent/JOURNAL/vs3-qd-4.md:11` |

### 1.5. `agent/JOURNAL/vs3-qd-5.md` (QĐ-5)
| [SỬA] VD-178 | Nội dung đã áp | đã sửa |
|---|---|---|
| QĐ-5 mục 1 | Số liệu **48/15/42 = 105**; thêm hàng `host_conformance` (b=1) vào bảng §3b | `agent/JOURNAL/vs3-qd-5.md:47` |
| QĐ-5 mục 2 | **VS3-5b chạy SAU R6**; luật test mới: không chèn đường dẫn vào chuỗi thường, dùng `p"…"`/argv, golden/TAP **tương đối**, mục tiêu danh sách nền **về 0** | `agent/JOURNAL/vs3-qd-5.md:29` |
| QĐ-5 mục 3 | Helper **chỉ** `echo/cat/grep/printf/true/false/sleep`; **không** `sh -c`; test dùng `sh -c` viết lại bằng lệnh con | `agent/JOURNAL/vs3-qd-5.md:19` |
| QĐ-5 mục 4 | Nhóm b **mặc định lỗi sản phẩm**; lsp 5, net_server 3, net 1, json_ra 5, host_conformance 1; cổng đóng Windows phải báo **“bị từ chối”** | `agent/JOURNAL/vs3-qd-5.md:22` |
| QĐ-5 mục 5 | VS3-5a/5d **xen kẽ pha 1** | `agent/JOURNAL/vs3-qd-5.md:35` |

### 1.6. `agent/JOURNAL/vs3-qd-6.md` (QĐ-6 — **viết lại theo VD-180/VD-181**)
| Quyết định | Nội dung đã áp | đã sửa |
|---|---|---|
| VD-178 QĐ-6 mục 1 (bị thay) | Mô hình **phi tập trung kiểu Go**, không kho tập trung; tên = **đường dẫn**; tag semver; major ≥ 2 có `/v2`; **MVS**; `pkg update X` (#142) | `agent/JOURNAL/vs3-qd-6.md:9` |
| VD-178 QĐ-6 mục 2 (bị thay) | **VS0707** = hash không khớp `vis.lock`/sổ (thay “chữ ký sai”); **VS0706** giữ “pkg chưa hỗ trợ” | `agent/JOURNAL/vs3-qd-6.md:22` |
| VD-180 mục 3 | `vis.lock` = **sha256 cây tệp chuẩn hoá**; **`sum.vifast.org`** chỉ ghi thêm (commit ký, khoá ghim); **`VIS_SUMDB=off`** (cảnh báo); **`VIS_PRIVATE`** | `agent/JOURNAL/vs3-qd-6.md:20` |
| VD-180 mục 4 | **`proxy.vifast.org` chỉ thiết kế** (GET theo đường dẫn@phiên bản); **`VIS_PROXY=direct`** mặc định | `agent/JOURNAL/vs3-qd-6.md:79` |
| VD-180 mục 5 | **4 ca đỏ-trước bắt buộc** (tag đổi nội dung ⇒ VS0707; MVS v1.2/v1.4 ⇒ v1.4; vanity ⇒ tải đúng; offline + cache ⇒ build) | `agent/JOURNAL/vs3-qd-6.md:88` |
| VD-180 mục 6 | **Ngoài phạm vi VS3**: proxy thật, `pkg.vifast.org`, yank | `agent/JOURNAL/vs3-qd-6.md:26` |
| VD-181 mục 1 | **`vifast.org/vis/<tên>`** (path, không subdomain); `<tên>` `[a-z0-9-]`; không `/x/`; docs ở `/docs/vis/…` | `agent/JOURNAL/vs3-qd-6.md:15` |
| VD-181 mục 2 | `sum.`/`proxy.vifast.org` **ngang hàng**, không lồng dưới `vis.` | `agent/JOURNAL/vs3-qd-6.md:42` |
| VD-181 mục 3 | `vis.vifast.org` **không** làm tiền tố module (chỉ website/redirect) | `agent/JOURNAL/vs3-qd-6.md:43` |
| VD-181 mục 4 | `viscript.org` chỉ **thương hiệu**, không vào đường dẫn gói 1.0 | `agent/JOURNAL/vs3-qd-6.md:44` |
| VD-181 mục 5 | **Bảng host → vai trò** (4 hàng) | `agent/JOURNAL/vs3-qd-6.md:34` |
| VD-181 mục 6 | **Traefik không phải hợp đồng**; hợp đồng là **nội dung meta tag**; `PathPrefix(/vis/)` → static origin | `agent/JOURNAL/vs3-qd-6.md:45` |
| VD-181 mục 7 | Tên **bất biến** sau gói đầu tiên; **quyết định không đảo** | `agent/JOURNAL/vs3-qd-6.md:6` |

### 1.7. `agent/JOURNAL/vs-ergo.md` (C1 + C2 + C3)
| [SỬA] VD-178 | Nội dung đã áp | đã sửa |
|---|---|---|
| C1 | Bảng mã thoát **0/1/2/101**; VS0502/VS0503 đổi kỳ vọng sang 101 (làm trong R9) | `agent/JOURNAL/vs-ergo.md:7` |
| R1 + R2 | Làm **chung một khối**; bỏ “xoá builtin `None`” riêng ở R1c; `None` là biến thể ở R2a; từ khoá `none` ⇒ **VS0203** + gợi ý + fmt | `agent/JOURNAL/vs-ergo.md:53` · `agent/JOURNAL/vs-ergo.md:62` |
| R3 | `if let`/`while let` nhận **pattern**; `if let v = o:` ⇒ **lỗi + gợi ý `Some(v)`**; một quy tắc, không dạng tắt | `agent/JOURNAL/vs-ergo.md:75` |
| R5 | `{ten: …}` trần ⇒ **VS0205**; method chỉ cho tham số đầu **kiểu cụ thể**; `str(x)` giữ là chuyển đổi | `agent/JOURNAL/vs-ergo.md:95` |
| R6 | Thêm ví dụ `p"C:\thu muc\"` **hợp lệ** | `agent/JOURNAL/vs-ergo.md:106` |
| R7 | Duyệt đổi **4 test** (run.rs:98,100,112; interp_v2.rs:77); thêm `-7 % 2 ⇒ 1`; đơn vị chia số ⇒ đơn vị, làm tròn xuống | `agent/JOURNAL/vs-ergo.md:114` |
| R9 | **VS0203** = từ khoá đã bỏ; **VS0204** = const cần kiểu khi không literal; thêm **VS0508**, **VS0707**; lỗi chạy có mã thoát **101** | `agent/JOURNAL/vs-ergo.md:130` |
| R10 | Cộng **cùng loại** hợp lệ; chỉ **khác loại** lỗi; in ghép/thập phân + `.fmt_iec()` | `agent/JOURNAL/vs-ergo.md:164` |
| R11 (c3) | Sửa chú thích probe `c3` (là **khai báo**, không phải “gọi kèm tiền tố”) | `agent/JOURNAL/vs-ergo.md:225` |
| R12 | `alias`/`newtype` ⇒ **VS0203** | `agent/JOURNAL/vs-ergo.md:254` |
| R14 | **Trùng tên ⇒ lỗi** (bỏ ưu tiên ngầm); `impl P:` chỉ trong cùng module khai báo `P` | `agent/JOURNAL/vs-ergo.md:268` |
| R19 | Panic tràn i64 thoát **101** theo C1 | `agent/JOURNAL/vs-ergo.md:354` |
| R22 | Bỏ “dùng `match` nếu cần” để ép ngược; ghi nợ sau 1.0 | `agent/JOURNAL/vs-ergo.md:289` |
| R23 | Ca đỏ-trước viết lại bằng `U(addr: A(city: "A"))` | `agent/JOURNAL/vs-ergo.md:300` |
| R24 | **Đảo**: `filter/map` giữ collection; `.lazy()` trả `Iterator[T]`; `range`/`fs.lines` lười; **duyệt lại được**; R20 **không** chờ R24 | `agent/JOURNAL/vs-ergo.md:308` |
| C3 | Thứ tự pha 1 mới: P0 → R9+C1 → R6 → R17 → R1+R2 → R3 → R8 → R4 → R5 → R11 → R12 → R16 → R7 → R19b → R19c | `agent/JOURNAL/vs-ergo.md:175` |

### 1.8. `agent/JOURNAL/st169-bang-phan-loai-windows.md`
| [SỬA] VD-178 | Nội dung đã áp | đã sửa |
|---|---|---|
| QĐ-5 mục 1 (bảng Windows) | Số liệu chốt **48/15/42 = 105**; thêm ca `host_conformance_suite` (nhóm b) với `file:dòng`; ghi rõ **nguồn** | `agent/JOURNAL/st169-bang-phan-loai-windows.md:16` |

### 1.9. `CHEAT-SHEET-ViScript.md` (mới, gốc repo)
| Mục | Nội dung đã áp | Vị trí |
|---|---|---|
| C1 | Bảng **mã thoát** 0/1/2/**101** + nhãn trạng thái | `CHEAT-SHEET-ViScript.md:31` |
| R3 | `if let Some(v) = x:` **[DỰ KIẾN]**; `if let v = x:` ⇒ lỗi + gợi ý | `CHEAT-SHEET-ViScript.md:96` |
| R10 | Đơn vị: in ghép/thập phân, `.fmt_iec()`, cộng cùng loại hợp lệ | `CHEAT-SHEET-ViScript.md:136`–`138` |
| R24 | `.lazy()` + `Iterator[T]` duyệt lại **[DỰ KIẾN]** | `CHEAT-SHEET-ViScript.md:187` |
| Nhãn | **[ĐÃ CÓ] / [ĐANG LÀM] / [DỰ KIẾN]** + quy ước + phụ lục trạng thái | `CHEAT-SHEET-ViScript.md:15` · `:245` |

---

## 2. Tóm tắt thay đổi từng tài liệu

| Tệp | Thay đổi chính |
|---|---|
| `vs3-qd-1.md` | Ngưỡng đo song song mới (task CPU, `0,6×min(nproc,N,4)`, ghi load + khoá đo); bỏ luật cấm await worker khác, thay bằng luật **Sendable + sao chép sâu**; mã **VS0508**; cập nhật bảng task 1b/1c/1d, ca đỏ-trước, cổng |
| `vs3-qd-2.md` | `chan(0)` = rendezvous **hợp lệ** (bỏ mâu thuẫn tyck/runtime); #145 = xoá chú thích sai + bỏ chặn runtime; `recv_any` trả `(int,T)?`, hết giờ `None`; **công bằng vòng xoay** + ca bộ đếm |
| `vs3-qd-3.md` | Thêm tuỳ chọn **`ca:`** cho CA riêng; mặc định kho HĐH; **không insecure**; mục 4 **mặc định an toàn** (10 s / 30 s / 5 / 8 MiB — đề xuất); ghi chú platform-verifier + `ring` Windows; HTTP server TLS ngoài phạm vi |
| `vs3-qd-4.md` | **§3 viết lại** theo §4b (đi từng thành phần, `openat`+`O_NOFOLLOW`, `openat2` đường nhanh + fallback, `..` từ vựng); macOS hết khe hở bản chất, còn **hardlink**; **rustix duyệt** (MIT); Windows mức thấp hơn; ca tất định 100/100 (500k phụ) |
| `vs3-qd-5.md` | Bảng §3b thêm `host_conformance` ⇒ **48/15/42 = 105**; **5b sau R6** + luật test mới; helper **7 lệnh con**, bỏ `sh -c`; nhóm b **mặc định lỗi sản phẩm** (lsp 5, net_server 3, net 1, json_ra 5, host_conformance 1); 5a/5d xen kẽ pha 1 |
| `vs3-qd-6.md` | **Viết lại toàn bộ** theo VD-180/181: mô hình phi tập trung kiểu Go, tên = đường dẫn, tag semver + `/v2`, **MVS**, `pkg update X`; `vis.lock` sha256 cây chuẩn hoá; `sum.vifast.org` chỉ ghi thêm; **VS0707**; `VIS_SUMDB=off`/`VIS_PRIVATE`/`VIS_PROXY=direct`; proxy **chỉ thiết kế**; **bảng host**; Traefik không phải hợp đồng; **4 ca đỏ-trước**; ngoài phạm vi; bảng phụ thuộc + task 6a–6d |
| `vs-ergo.md` | Thêm **C1** (mã thoát); R1+R2 gộp; R3 pattern; R5 VS0205 + method + `str(x)`; R6 ví dụ `p"…\"`; R7 4 test + `-7%2` + đơn vị; R9 VS0203/0204/0508/0707 + 101; R10 đính chính; R11 c3; R12 VS0203; R14 trùng tên; R19 101; R22 bỏ match; R23 ca mới; **R24 đảo**; **C3** thay thứ tự cũ; bảng tổng hợp cuối tệp |
| `st169-…windows.md` | Sửa hàng 49 (`host_conformance::host_conformance_suite`, `crates/vis-cli/tests/host_conformance.rs:109:5`); ghi chú b: lsp **5**, json_ra **5**, tổng **15**; thêm dòng nguồn + “đã sửa” |
| `CHEAT-SHEET-ViScript.md` | Chép từ `/Users/mps/dvchd/ViFastOs/CHEAT-SHEET-ViScript.md` về gốc repo mới; sửa đường dẫn (`target/debug/vis`, `docs/SPEC-lex.md`, `test/sem/**`, `agent/API-STD.txt`); thêm **mã thoát C1**, **R3**, **R10**, **R24**, nhãn **[ĐÃ CÓ]/[ĐANG LÀM]/[DỰ KIẾN]**, phụ lục trạng thái; `pkg add` theo mô hình mới |

---

## 3. Kết quả `check_report`

```
$ python3 test/report/check_report.py --all
report-check: PASS (29 báo cáo, 197 hàng QUEUE phủ đủ)
[exit=0]
```

Log đầy đủ: `agent/JOURNAL/st178-check-report.log` (không ghi đè log cũ).

---

## 4. Bằng chứng probe (để gán nhãn trạng thái cheat sheet)

Chạy `target/debug/vis` @ `505deaa` (macOS arm64):

| Probe | Kết quả | Kết luận |
|---|---|---|
| `1 / 0` | `VS0502`, **exit 1** | C1 (101) là **[DỰ KIẾN]** |
| `if let Some(v) = o:` | `VS0412` pattern ctor không khớp `int` | R3 là **[DỰ KIẾN]** |
| `if let v = o:` | chạy, in `5` | R3 sẽ **đổi** ⇒ **[ĐANG LÀM]** |
| `print(150m)` / `print(1h+30m)` / `print(1kb+1kib)` | `150m` / `90m` / `2024b` | R10 là **[DỰ KIẾN]** |
| `xs.lazy()` | `VS0403` không có method `lazy` | R24 là **[DỰ KIẾN]** |
| `p"C:\x"` | `VS0102` escape không hợp lệ | R6 là **[ĐANG LÀM]** |
| `assert false, "hong"` | `VS0201` | R20 là **[DỰ KIẾN]** |
| `fn main() -> Result[(), str]: Err("hong")` | **exit 0** | R4 (exit 1) là **[DỰ KIẾN]** |
| `let x: int? = none` | chạy, in `none` | R2 (bỏ `none`) là **[DỰ KIẾN]** |

---

## 5. Tệp để Agent 1 commit

**Sửa (9 tệp)** — phần điền `tệp:dòng` + sửa đường dẫn cũ, sau commit `505deaa`:
1. `agent/JOURNAL/vs3-qd-1.md`
2. `agent/JOURNAL/vs3-qd-2.md`
3. `agent/JOURNAL/vs3-qd-3.md`
4. `agent/JOURNAL/vs3-qd-4.md`
5. `agent/JOURNAL/vs3-qd-5.md`
6. `agent/JOURNAL/vs3-qd-6.md`
7. `agent/JOURNAL/vs-ergo.md`
8. `agent/JOURNAL/st169-bang-phan-loai-windows.md`
9. `CHEAT-SHEET-ViScript.md` (đã nằm trong `505deaa`; bản này chỉ sửa 2 đường dẫn cũ `viscript/…` còn sót)

**Mới (2 tệp)**:
10. `agent/JOURNAL/st178-bao-cao.md` (báo cáo này)
11. `agent/JOURNAL/st178-check-report.log`

**Không đổi**: `crates/`, `bin/`, `test/`, `agent/QUEUE.md`, `agent/DECISIONS.md`, `agent/SO-NO.md`, `docs/`.

---

## 6. Việc còn chờ A0

1. **Số mặc định an toàn của QĐ-3** (đề xuất, chờ A0 chốt khi duyệt ST-178):
   `timeout_connect = 10 s` · `timeout_read = 30 s` · `max_redirect = 5` (chỉ GET) · `max_body = 8 MiB`.
   Đã ghi rõ “**đề xuất, chờ A0 chốt**” trong `vs3-qd-3.md:14` — chưa dùng để “đạt” gì.
2. **`agent/HOST-BAO-DAM.md` §4** (Agent 0 sở hữu, **ngoài danh sách tệp ST-178 được giao**): cần bỏ câu “macOS còn khe hở bản chất”, ghi **hardlink** là giới hạn còn lại, ghi **Windows mức bảo đảm thấp hơn**. Đã ghi chú tại `vs3-qd-4.md` §8.
3. **`docs/ViScript-thiet-ke.md`** (Agent 0 sở hữu): còn mô tả mô hình gói cũ — `:572` (“registry sau này”, “vis.lock khoá phiên bản + hash nội dung”) và `:1485` (“registry cục bộ… chưa có registry mạng và chữ ký”). Cần cập nhật theo VD-180/181 (bảng host → vai trò, mô hình phi tập trung). **ST-178 không sửa** (ngoài phạm vi tệp).
4. **Xác nhận cách hiểu “9 mục [SỬA]”**: “9 mục” = **9 nhóm quyết định** (QĐ-1…QĐ-6 + C1 + C2 + C3); bên trong có **16 dấu [SỬA]** (QĐ-1×2, QĐ-2×1, QĐ-3×1, QĐ-4×2, QĐ-5×1, QĐ-6×1, C2×8). Báo cáo này liệt kê **đủ 16 dấu** + C1/C3 + các mục “áp dụng” (QĐ-2 mục 2–3, QĐ-3 mục 2, QĐ-4 mục 3–4, QĐ-5 mục 5) để A0 đối chiếu.

---

## 7. Ghi chú về commit `505deaa` (Agent 1)

- Agent 1 đã commit **trạng thái trung gian** của ST-178 trong `505deaa` (`[ST-171k]`, 22:22 JST) — commit này chứa nội dung 6 tài liệu VS3 + `vs-ergo.md` + `st169` + `CHEAT-SHEET-ViScript.md` **trước khi** điền `tệp:dòng` (còn placeholder `@@L…@@`).
- Sau đó Agent 2 đã điền **toàn bộ** `tệp:dòng` và thêm báo cáo + log; phần này **chưa commit** (Agent 1 commit).
- Đề nghị Agent 1 commit tiếp với thông điệp `[ST-178] sửa tài liệu theo VD-178/180/181 (0 dòng mã)` và **không** amend `505deaa` (đã push).
