# ST-174 — Vá nợ `#146` (phương án A) + chứng minh **TẤT ĐỊNH** (VD-176 §1–§3)

Ngày: 2026-11-17 (JST) · Agent 2 dòng ViScript · Máy đo: `mac` (mps, macOS 27.0.1 arm64, 12 core)
và `ps` (Ubuntu 26.04.1, Linux `7.0.0-43-generic`) · Ngân sách 3 h (dừng mềm 17:10 JST).

## 0. Kết luận một dòng

`quyen.rs:513` đổi **`std::fs::metadata` → `std::fs::symlink_metadata`** (đúng **1 dòng logic**, không
đổi thiết kế/kỳ vọng) + **điểm chèn chỉ `#[cfg(all(test, unix))]`** tại `quyen.rs:505`: trên **mã cũ**
ca tất định **đỏ 100/100** (đọc được `NGOAI` ngoài root), sau vá **100/100 bị chặn** trên **mac và ps**;
ca thứ hai **100/100 đọc đúng `TRONG`**; ca âm báo đúng **“không chạy được điểm chèn”**; **6 000 000
vòng** ngẫu nhiên trên mac **0 đỏ**; cổng mac **PASS 10/10 ba lần liên tiếp** (L0 **1184/0**, 131 suite).
⇒ **`#146` đủ điều kiện đóng** (mục 3–6 đạt) — Agent 1 đóng sổ, không tự sửa `SO-NO.md`.

## 1. Thay đổi (nguồn)

| Tệp | Thay đổi |
|---|---|
| `viscript/crates/vis-cli/src/quyen.rs` | **:513** `metadata` → **`symlink_metadata`** (bản vá, kèm 1 chú thích); **:503-507** gọi **điểm chèn** `chen::chay(&sau, p)` (`#[cfg(all(test, unix))]`); **`mod chen`** (63 dòng, `#[cfg(all(test, unix))] pub(crate)`); cập nhật doc của `xac_nhan_sau_mo` (cơ chế + giới hạn hardlink). Tệp **1356** dòng (R1 ≤ 1500). |
| `viscript/crates/vis-cli/src/main.rs` | thêm `#[cfg(test)] mod quyen_st174;` (theo mẫu `mod audit_span`). |
| `viscript/crates/vis-cli/src/quyen_st174.rs` **(mới)** | 324 dòng — 2 ca tất định + host mô phỏng macOS (`KhongCo`) + ca âm. **Chỉ test** (`#![cfg(unix)]`). |
| `viscript/agent/HOST-BAO-DAM.md` | sửa gạch đầu dòng **macOS** (cửa sổ symlink **đã đóng**, ST-174) + **mục mới** “Hardlink của tệp ngoài vào trong root — giới hạn **chính sách**, KHÔNG phải lỗi đua”. |

Diff logic (rút gọn):

* `quyen.rs:513`: `let md = std::fs::metadata(&sau)…` → `let md = std::fs::symlink_metadata(&sau)…`
* `quyen.rs:505`: `#[cfg(all(test, unix))] chen::chay(&sau, p);` — nằm **giữa** `let sau = Self::chuan(…)`
  và bước kiểm `symlink_metadata`.
* `mod chen`: hook **thread-local** `Box<dyn FnMut(&Path, &str)>` + bộ đếm `so_lan()`; chưa đăng ký hook
  ⇒ **no-op**.

## 2. Vì sao bản vá đóng đúng cửa sổ của `#146`

Chuỗi sự kiện lọt (ST-163, mac: **4/8 100 000** vòng):

1. kiểm trước: `chuan(p)` → tên **trong root**;
2. mở: tên đã bị đổi thành **symlink ra ngoài** ⇒ handle = tệp **ngoài** (nội dung `NGOAI`, `(dev,ino)` = N);
3. xác nhận sau khi mở: `chuan(p)` lại ra **tên trong root** `sau = root/tep`;
4. `std::fs::metadata(&sau)` **đi theo symlink** ⇒ trả **(dev,ino) = N** ⇒ **trùng** handle ⇒ `kiem_canon`
   xanh ⇒ `Ok("NGOAI")`.

Nay bước 4 dùng `symlink_metadata`: `(dev,ino)` là của **chính mục tên** `root/tep` (inode của symlink),
**không thể** trùng N ⇒ `Err(VS0604 … “bị thay đổi trong khi mở”)`. Khi **không** có thay đổi trong khe
thì `sau` là đường dẫn **canonical** (không phải symlink) ⇒ hành vi như cũ, không chặn oan.

## 3. Đỏ-trước (mục 3) — log `st174-do-truoc-100.log`

Dựng: giữ nguyên điểm chèn + 2 ca, **chỉ** đổi dòng kiểm về bản cũ (`metadata`). `sha256 quyen.rs` của
lần chạy: ghi trong header log.

| Ca | Kết quả | Ý nghĩa |
|---|---|---|
| **A** — điểm chèn đổi `root/tep` → **symlink ra ngoài** | `lap=100 ro_ri=100 chan=0 diem_chen=100` ⇒ **FAILED** `RÒ RỈ (#146): đọc được nội dung NGOÀI root 100/100 lần` | **đỏ 100/100** — tái hiện tất định `#146` |
| **B** — điểm chèn đổi **symlink (trong root) về tệp thật** | `lap=100 ok_trong=100 loi=0` ⇒ ok | **đối chứng xanh** (không “chặn tất”) |

`[exit=101]` (có 1 ca đỏ — đúng kỳ vọng).

## 4. Sau vá (mục 4, 5) — log `st174-sau-mac-ps.log`

Cùng mã: `sha256 quyen.rs = b5747ad2…`, `quyen_st174.rs = 6495ce37…` trên **cả hai** máy.

| Máy | Ca A (symlink ra ngoài) | Ca B (symlink → tệp thật) | exit |
|---|---|---|---|
| **mac** | `lap=100 ro_ri=0 chan=100 chan_thay_doi=100 khac=0 diem_chen=100` | `lap=100 ok_trong=100 loi=0 khac=0 diem_chen=100` | 0 |
| **ps** | `lap=100 ro_ri=0 chan=100 chan_thay_doi=100 khac=0 diem_chen=100` | `lap=100 ok_trong=100 loi=0 khac=0 diem_chen=100` | 0 |

Thông điệp chặn (trích log): `VS0604:[quyền] …/root/tep bị thay đổi trong khi mở (chống kiểm-rồi-mở:
không đọc lại được đường dẫn)` — đúng lý do TOCTOU, không phải “thiếu cap”.

## 5. Ca âm cho điểm chèn (mục 6) — log `st174-ca-am-diem-chen.log`

Dựng: **giữ nguyên bản vá**, bỏ đúng 2 dòng gọi điểm chèn (`#[cfg(all(test, unix))]` + `chen::chay(&sau, p);`)
⇒ **cả 2 ca FAILED** với `không chạy được điểm chèn (vòng 0/100): điểm chèn chưa chạy — ca tất định vô
hiệu`; `[exit=101]`. Không có đường “xanh im lặng”.

## 6. Bằng chứng phụ 6 000 000 vòng (mục 7) — log `st174-6m-mac.log`

`VIS_TOCTOU_LAP=6000000 cargo test … quyen::tests::dua_doi_symlink_lap_co_gioi_han --nocapture`:

```
TOCTOU dua: lap=6000000 ok_trong=1581842 ro_ri=0 chan=4418158 vong_doi=2212191 giay=1006.3
test result: ok. 1 passed; 0 failed  ·  [exit=0]
```

**Số đỏ = 0** ⇒ không phải BLOCKED. (Trước vá, ST-163 đo **4 đỏ / 8,1 triệu vòng** trên mac.)

## 7. Cổng (mục 9) — `st174-gate-macos-{1,2,3}.log` + `st174-ps.log`

| Lần | Kết quả | L0 | Thời gian | Log |
|---|---|---|---|---|
| 1 | **PASS 10/10** (`[exit=0]`) | **1184/0**, 131 suite | 429 s | `st174-gate-macos-1.log` |
| 2 | **PASS 10/10** (`[exit=0]`) | **1184/0**, 131 suite | 322 s | `st174-gate-macos-2.log` |
| 3 | **PASS 10/10** (`[exit=0]`) | **1184/0**, 131 suite | 320 s | `st174-gate-macos-3.log` |

3 lần **liên tiếp**, **không retry**; L0 **1184 = 1182 (ST-173) + 2 ca `quyen_st174::*`** (đúng dự kiến).
`report-check: PASS (28 báo cáo, 196 hàng QUEUE)` cả 3 lần.

`ps` (ảnh hưởng Linux — dòng kiểm nằm trong `#[cfg(unix)]`): `cargo test -p vis-cli` **77 suite
`test result: ok`**, 0 đỏ, `[exit=0]` — `st174-ps.log`.

## 8. `cfg(test)` **không** lọt build thường

- `cargo build --workspace --bin vis` ⇒ OK (và bước `--- build: OK` trong cả 3 lần cổng).
- `strings target/debug/vis | grep -c "ST174 ca-A\|không chạy được điểm chèn\|quyen_st174\|chen::chay"`
  ⇒ **0**; `nm … | grep -c "quyen_st174\|chen"` ⇒ **0**. Điểm chèn chỉ tồn tại trong `#[cfg(test)]`.

## 9. Giới hạn đã biết — **hardlink** (mục 8, đã ghi `HOST-BAO-DAM.md`)

**Hardlink của tệp ngoài root đặt trong root được coi là nằm trong root** (đọc được). Đây là **giới hạn
chính sách**, **không** phải lỗi đua: không có khe thời gian nào để đóng — hardlink là **cùng inode**,
`canonicalize`/`symlink_metadata`/`metadata` đều trả cùng `(dev, ino)`, nên “vùng = cây tên” coi đó là
tên trong vùng. Cần quyền ghi trong root + cùng hệ tệp; Landlock không chặn; muốn chặn phải đổi định
nghĩa vùng sang “vật” hoặc `openat2`/`RESOLVE_BENEATH` — **VS3** (#140/#109b), không làm trong VS2.

## 10. Tệp đã đổi/thêm (chính xác, từ gốc repo) — **chưa commit**

Sửa:
- `viscript/crates/vis-cli/src/quyen.rs`
- `viscript/crates/vis-cli/src/main.rs`
- `viscript/agent/HOST-BAO-DAM.md`

Thêm:
- `viscript/crates/vis-cli/src/quyen_st174.rs`
- `viscript/agent/JOURNAL/st174-do-truoc-100.log`
- `viscript/agent/JOURNAL/st174-ca-am-diem-chen.log`
- `viscript/agent/JOURNAL/st174-sau-mac-ps.log`
- `viscript/agent/JOURNAL/st174-6m-mac.log`
- `viscript/agent/JOURNAL/st174-gate-macos-1.log`
- `viscript/agent/JOURNAL/st174-gate-macos-2.log`
- `viscript/agent/JOURNAL/st174-gate-macos-3.log`
- `viscript/agent/JOURNAL/st174-ps.log`
- `viscript/agent/JOURNAL/st174-ghi-chu.md` (tệp này)

Log **nháp** giữ lại làm lịch sử (mã trung gian, **không** phải bằng chứng chính — khác bản cuối chỉ ở
1 dòng chú thích/`type Hook`):
`st174-do-truoc-100-truoc-alias.log`, `st174-ca-am-diem-chen-truoc-alias.log`,
`st174-do-truoc-100-nhap.log`, `st174-ca-am-diem-chen-nhap.log`,
`st174-sau-mac-ps-nhap.log`, `st174-ps-nhap.log`, `st174-6m-mac-lan1-huy.log`, `st174-6m-mac-lan2-huy.log`.

## 11. Bất thường / ghi chú

1. **Lệch số dòng so với VD-176**: VD-176 ghi `:465`/`:472` (bản `quyen.rs` lúc soạn), hiện là
   **`:505`** (điểm chèn) và **`:513`** (bước kiểm) — cùng một cặp bước, chỉ do tệp đã dài thêm.
2. **Host của ca tất định** mô phỏng **đúng phần macOS thiếu**: đọc **từ handle**, báo `(dev,ino)` **thật**,
   nhưng `DuongDanHandle::KhongCo` (không có fd→path). Nhờ vậy cùng một ca chạy được trên **mac và ps**
   và đi đúng nhánh `xac_nhan_sau_mo`; trên Linux production nhánh này **không** dùng (có `/proc/self/fd`).
3. **Ca B là ca đối chứng** (xanh ở **cả** mã cũ và mới) — vai trò: bản vá **không** “chặn tất”, đọc trong
   root vẫn đúng đặc tả; ca **đỏ-trước** là **ca A**.
4. **`ps` đang bẩn** (nợ **ST-175**, không do task này): `git status` của `ps` có nhiều tệp `D` ở gốc cây.
   Task này chỉ `bin/vis-linux sync` các cây của dòng ViScript (`.git` + `viscript/` + …), rồi chứng minh
   **cùng mã** bằng `sha256`, không dựa vào HEAD của `ps`.
5. **Không commit/push** (hook `vifast-guard` chặn agent) — phần commit để **Agent 1**; index đang **sạch**
   (`git diff --cached` rỗng), mọi thay đổi nằm ở working tree.
6. **Không** sửa `SO-NO.md`, `DECISIONS.md`, `QUEUE.md`, `STATE.md`, `spec/`, `tests/quyen.rs`, không đổi
   kỳ vọng/ngưỡng, không `unsafe` mới, không crate mới, không `#[ignore]`/`#[should_panic]`.

## 12. Đóng `#146` (mục 10)

Điều kiện của QUEUE: mục **3–6** đạt ⇒ **đề nghị Agent 1 đóng `#146`** (sổ `SO-NO.md` do Agent 1 ghi),
kèm trích dẫn: `st174-do-truoc-100.log` (100/100 đỏ), `st174-sau-mac-ps.log` (100/100 xanh × 2 máy),
`st174-ca-am-diem-chen.log`, `st174-6m-mac.log` (0/6 000 000), 3 log cổng.
