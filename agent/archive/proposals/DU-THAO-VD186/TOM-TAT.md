<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# TÓM TẮT · Bản thảo diff VD-186 (2 tài liệu Agent 0 sở hữu)

**Ngày**: 2026-11-18 · **Người soạn**: Agent 1 (nhánh `docs-vd186`) · **Nguồn**: `A0-QUYET-DINH-VD182-196-2026-11-18.md` **VD-186** (Q5 = **B**: Agent 1 soạn diff, A0 duyệt)
**Trạng thái**: **CHỜ A0 DUYỆT** — **chưa** áp vào tệp đích, **chưa** commit. Hai tệp đích thuộc quyền Agent 0.

| Tệp patch | Tệp đích | Dòng patch | Thay đổi |
|---|---|---|---|
| [`HOST-BAO-DAM-4.patch`](HOST-BAO-DAM-4.patch) | `agent/HOST-BAO-DAM.md` | **99** | **+48 / −5** |
| [`ViScript-thiet-ke.patch`](ViScript-thiet-ke.patch) | `docs/ViScript-thiet-ke.md` | **114** | **+89 / −2** |

Kiểm: `git apply --check` **sạch** cho cả hai (chạy từ gốc repo, HEAD `6a32233`). Đã áp thử vào cây nháp rồi
so byte: **khớp 100 %** bản đã sửa. sha256: `b3a0d16a…f8743` (HOST-BAO-DAM), `bc947bcc…13954` (thiết kế).

---

## 1. `agent/HOST-BAO-DAM.md` — §4 (mục **FsGuard**)

**Vì sao**: VD-186 yêu cầu tài liệu bảo đảm **mô tả mã đang chạy**, không mô tả mã tương lai. Bản hiện tại
đã có sẵn dòng "Đường ghi: giới hạn đã biết" và mục hardlink (ST-174), nhưng **thiếu** ba thứ A0 đòi:
(i) câu "macOS còn khe hở" **thu hẹp vào đường GHI** kèm điều kiện xoá, (ii) phát biểu **#146 đã đóng ⇒
đường đọc sạch**, (iii) **Windows mức bảo đảm thấp hơn** thành một mục đọc được.

| # | Chỗ sửa | Nội dung | Yêu cầu VD-186 |
|---|---|---|---|
| 1 | Dòng "Cập nhật" đầu tệp | `2026-11-17` → `2026-11-18`, thêm vế **VD-186** tóm tắt 3 điểm | (chung) |
| 2 | Ngay dưới bảng nền tảng | Thêm blockquote **Windows (VD-186)**: `FsGuard` **có chạy** nhưng **chỉ kiểm theo đường dẫn**, **không có danh tính tệp** ⇒ mức thấp hơn; **chỉ đo**, chưa hỗ trợ | mục 3 (Windows) |
| 3 | §"Cấm ngôn từ" | Thêm 2 gạch đầu dòng **Sai/Đúng (VD-186)**: cấm "macOS không còn khe hở", "Windows được hỗ trợ", "hardlink là lỗi đua" | mục 1 (nguyên tắc) |
| 4 | Đầu mục **FsGuard** | Thêm **bảng "Trạng thái hiện tại"** 4 dòng: Đọc **đã đóng** (`#146`, ST-174) · Ghi **còn khe hở** trên **mọi nền Unix** (`#140`/`#109b`) · Hardlink **giới hạn chính sách** · Windows **mức thấp hơn** | mục 2, cả 4 gạch |
| 5 | Ngay sau bảng đó | **Giữ** câu **"macOS còn khe hở — nhưng chỉ ở đường GHI"**, tới khi **VS3-QĐ-4** merge; ghi rõ **chỉ được xoá trong cùng commit merge QĐ-4** | mục 2, gạch 3 (**nguyên văn yêu cầu**) |
| 6 | Gạch "Đường đọc" | Thêm vế đầu: **nợ `#146` ĐÃ ĐÓNG (ST-174, VD-176 §1): không còn đua symlink trên macOS và Linux** | mục 2, gạch 1 |
| 7 | Gạch "Đường ghi" | Viết lại: **KHE HỞ HIỆN CÓ trên mọi nền Unix (`#140`/`#109b`), chưa vá**; nêu rõ `#146` **chỉ** đóng đường đọc; chỉ đích **VS3-QĐ-4** (VD-191: `openat` + `O_NOFOLLOW`/`O_DIRECTORY`, Linux ≥ 5.6 `openat2(RESOLVE_BENEATH\|RESOLVE_NO_SYMLINKS)`) | mục 2, gạch 2 |
| 8 | Mục hardlink | Thêm một câu vào "Kết luận một dòng": sau `#146`, đây là **giới hạn còn lại duy nhất** của **đường đọc** | mục 2, gạch 4 |
| 9 | Mục mới `### Windows — mức bảo đảm thấp hơn, chỉ đo` | 5 gạch: không có danh tính tệp (`(dev, ino)` = `(0,0)`, nhánh `#[cfg(not(unix))]` của `quyen.rs`) · chỉ kiểm theo đường dẫn ⇒ khe hở **rộng hơn** · không Landlock · **số đo** (host 39/54, `cargo test` 1053/100, L0 dừng ở `cap_proc`, targets 18/18, self-test 12/12, cổng 9/10) · điều kiện để nói khác đi | mục 2, gạch 5 |

**Không** sửa: bảng nền tảng (chỉ thêm blockquote), mục Landlock, mục "Cây nguồn để đo phải SẠCH", mọi số
đo cũ, mọi câu về `#146`/`#140`/`#109b` đã có.

---

## 2. `docs/ViScript-thiet-ke.md` — mục gói

**Vì sao**: mục 8 còn câu **"hoặc registry sau này"** và **"`vis.lock` khoá phiên bản + hash nội dung"** —
đúng nguyên văn mô hình **tập trung** mà VD-180/VD-181 đã **thay thế**. Mục "Mức hiện thực VS2" lại ghi
"registry cục bộ … chưa có registry mạng và chữ ký", dễ đọc thành "sẽ có registry mạng".

| # | Chỗ sửa | Nội dung | Yêu cầu VD-186 |
|---|---|---|---|
| 1 | §8, gạch "Gói" (dòng 575) | **Bỏ** "registry sau này" + "`vis.lock` khoá phiên bản + hash nội dung"; **thay** bằng một dòng tóm tắt mô hình Go + nhãn **`[thiết kế VS3-6, chưa triển khai]`** + trỏ **Phụ lục C** | mục 3, gạch 1 |
| 2 | **Phụ lục C** (mới, chèn sau Phụ lục B) | Toàn bộ mô hình, **nhãn ở tiêu đề + blockquote đầu mục** | mục 3, gạch 2 + 3 |
| 3 | **C.1** | Bảng 8 dòng (tên = đường dẫn · tên chính thức `vifast.org/vis/<tên>` · tag semver + `/v2` · **MVS** · `vis.toml` · `vis.lock` sha256 · tin cậy · proxy) + 10 gạch chi tiết: `vis.toml` ví dụ · meta tag/vanity · MVS (ca `v1.2`/`v1.4` ⇒ `v1.4`) · `pkg update X` (#142) · **`vis.lock` = sha256 cây chuẩn hoá** + **`sum.vifast.org`** chỉ ghi thêm + **VS0707** + **`VIS_SUMDB=off`** + **`VIS_PRIVATE`** · **`proxy.vifast.org` chỉ thiết kế** + **`VIS_PROXY=direct`** · **bảng host → vai trò** · bất biến · 4 ca đỏ-trước · ngoài phạm vi VS3 | mục 3, gạch 2 |
| 4 | **C.2** | **Trạng thái hiện tại của `vis pkg`** (mã đang chạy): 5 lệnh, **registry CỤC BỘ**, `vis.lock` SHA-256, **bộ giải ràng buộc** (không phải MVS), danh sách **chưa có**, nợ #142/#143/#125 | mục 3, gạch 3 ("một dòng trạng thái hiện tại") |
| 5 | "Mức hiện thực VS2", gạch "Gói" (dòng 1488) | Viết lại cho khớp mã đang chạy + trỏ nhãn `[thiết kế VS3-6, chưa triển khai]` | mục 1 (nguyên tắc) |

**Ràng buộc kỹ thuật đã giữ** (quan trọng, xem §3.1): **không** thêm khối ` ```vis ` / ` ```vish-session `
nào, và **không** chèn dòng nào **trước** dòng 1470 ⇒ **24 khối `vis` + 2 khối `vish-session` giữ nguyên số
dòng** ⇒ `test/doc-examples/doc-*` **không** phải sinh lại, cổng tài liệu **không** đỏ.

---

## 3. Chỗ tôi **chưa chắc** — xin A0 quyết

### 3.1. Vị trí nội dung gói: **Phụ lục C** thay vì viết thẳng trong §8 (quan trọng nhất)

Cổng `doc-examples` đặt **tên fixture theo số dòng** của hàng ` ```vis ` (`doc-ViScript-thiet-ke-<dòng>.vis`).
Chèn ~90 dòng vào §8 sẽ **đổi số dòng của 10 khối** (578, 617, 669, 679, 691, 705, 748, 1423, 1462, 1470)
⇒ phải chạy `python3 test/doc-examples/doc_examples.py --regen` và commit **thêm** 10 tệp fixture đổi tên —
việc đó **ngoài 2 tệp** được giao và sẽ làm cổng đỏ cho tới khi regen xong.

Vì vậy bản thảo chọn cách **không đụng số dòng**: §8 còn **một dòng trỏ**, toàn văn ở **Phụ lục C**.
- **Nếu A0 chấp nhận**: áp nguyên 2 patch, **không** cần regen, cổng xanh ngay.
- **Nếu A0 muốn viết thẳng trong §8**: tôi soạn lại bản thứ hai (chèn vào §8) và **kèm** danh sách 10 fixture
  phải đổi tên + lệnh `--regen`; khi đó cần A0 cho phép sửa `test/doc-examples/` trong **cùng commit**.
- Hệ quả phụ của cách đang chọn: **"Mục lục"** (dòng 58–59) **không** liệt kê Phụ lục C (thêm 1 dòng ở đó
  cũng làm lệch số dòng ⇒ cùng vấn đề). Nếu A0 muốn có trong Mục lục thì phải đi kèm regen.
- **Quan sát lúc soạn**: cây làm việc đang có `test/doc-examples/doc-ViScript-thiet-ke-214.vis` và
  `-435.vis` **bị sửa bởi việc khác** (R2 đổi cú pháp `none`) ⇒ đợt này **đằng nào cũng** phải chạy
  `--regen` cho R2. Nếu vậy, chi phí của phương án "viết thẳng trong §8" **thấp hơn** tôi ước lượng: chỉ
  cần regen **một lần** chung với R2. A0 chọn phương án nào cũng được, tôi soạn lại theo.

### 3.2. "§4" của `HOST-BAO-DAM.md` là mục nào

Tệp **không đánh số mục**; "§4" là cách gọi cũ trong `SO-NO.md` (#109b, #140), `STATE.md`, `QUEUE.md`.
Theo nội dung (đường đọc/đường ghi/hardlink/Windows) thì "§4" = mục **`### FsGuard`**. Bản thảo sửa **mục
FsGuard** và **không** đổi tiêu đề thành "§4" (sợ khẳng định một cách đánh số không khớp 12 mục `##` hiện
có). **Xin A0 chốt**: (a) giữ nguyên như bản thảo, hay (b) cho phép thêm số mục vào tiêu đề để "§4" trong
sổ nợ trỏ đúng chỗ.

### 3.3. Ba chỗ sửa **ngoài** §4 trong `HOST-BAO-DAM.md`

A0 chỉ định "§4", nhưng bản thảo sửa thêm 3 chỗ vì để nguyên thì tài liệu **tự mâu thuẫn**:
1. **dòng "Cập nhật"** đầu tệp (đổi ngày + tóm tắt VD-186);
2. **blockquote Windows** ngay dưới bảng nền tảng — bảng đang ghi `Windows / wasm | Chưa có host tương ứng |
   … | Chưa đo`, trong khi Windows **đã đo** (39/54, VD-146) và `FsGuard` **có chạy**; tôi **không** sửa
   hàng bảng (sợ đụng số đo) mà thêm blockquote đính chính;
3. **2 gạch "Sai/Đúng (VD-186)"** ở mục "Cấm ngôn từ" — cơ chế để 3 câu A0 muốn giữ **không bị viết sai**
   ở lần sau.
**Xin A0 chốt**: giữ cả 3, hay cắt về đúng §4 (khi đó bảng nền tảng vẫn sai về Windows).

### 3.4. Câu "macOS còn khe hở" — bản hiện tại **không có** câu đó

`CHO-A0-QUYET-DINH-2026-11-18.md` §231 đề nghị **bỏ** câu "macOS còn khe hở bản chất"; A0 đáp **giữ**,
thu hẹp vào đường ghi. Nhưng tệp hiện tại **không còn** câu ấy (ST-174 đã sửa thành "cửa sổ symlink của
đường đọc ĐÃ ĐÓNG"). Bản thảo vì vậy **viết lại câu đó** dưới dạng gạch đầu dòng mới, đúng phạm vi A0
muốn, kèm điều kiện xoá. **Xin A0 xác nhận** đây là ý A0 (giữ **ý** "macOS còn khe hở", phạm vi = đường
ghi), chứ không phải tìm lại một câu nguyên văn đã bị xoá.

### 3.5. Số liệu Windows trong mục mới

Tôi lấy từ `agent/reports/R26-viscript.md` §3.3 (host 39/54, `cargo test` 1053/100, L0 150/4 dừng ở
`cap_proc`, targets 18/18, self-test 12/12, cổng 9/10). Đây là **số của R26**, **chưa** đo lại hôm nay.
Nếu A0 muốn "chỉ nêu giới hạn, không nêu số" thì cắt 1 gạch.

### 3.6. Nhãn `[thiết kế VS3-6, chưa triển khai]` — phạm vi

Bản thảo gắn nhãn ở **tiêu đề Phụ lục C** + **blockquote đầu mục** + **dòng trỏ ở §8** + **gạch "Gói" của
mục VS2**. Không gắn lên từng câu (sẽ rất nặng). Nếu A0 muốn nhãn **từng đoạn** thì báo, tôi thêm.

### 3.7. Hai chỗ **không** sửa (ghi để A0 biết, không phải xin duyệt)

- §16 "Công cụ" còn dòng `| vis new, vis add | tạo gói, thêm phụ thuộc |` — lệnh thật là `vis pkg add`;
  `vis new`/`vis add` **không có** trong `crates/vis-cli/src/main.rs`. Ngoài phạm vi VD-186.
- §19 "Lộ trình" ghi VS2 có "gói" — vẫn đúng (đã có `vis pkg` cục bộ).

---

## 4. Cách áp (khi A0 duyệt)

```bash
cd /Users/mps/dvchd/ViScript
git apply --check agent/DU-THAO-VD186/HOST-BAO-DAM-4.patch
git apply --check agent/DU-THAO-VD186/ViScript-thiet-ke.patch
git apply agent/DU-THAO-VD186/HOST-BAO-DAM-4.patch agent/DU-THAO-VD186/ViScript-thiet-ke.patch
```

Sau khi áp: **không** cần `--regen` doc-examples (xem §3.1). Cổng tài liệu chỉ cần `vis-dev check` bình
thường. **Không** commit trong bản thảo này.

**Nhắc lại ràng buộc đã tuân**: không sửa trực tiếp 2 tệp đích · không chạy `cargo` · không chạy
`./vis-dev` · không commit · tệp `.md` mới có header SPDX.
