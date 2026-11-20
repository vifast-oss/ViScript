# R31 — ViScript, vòng 31 (baseline bàn giao VD-242, 2026-11-20)

## 1. Phạm vi

R31 là báo cáo **baseline** cho đợt “ổn định sau pha 1” theo VD-242. Không mở pha 2 hay VS3. Không tuyên bố macOS; Windows giữ Tier 2.

## 2. Baseline được đo

- HEAD sản phẩm: `31b51d2` (= `origin/main` lúc bàn giao).
- Tag `pha1-rc1` = `05e42d0` — không di chuyển.
- Linux: `gate --fast` PASS 12/12; 150 suite / 1349 test / 0 đỏ; `sem` 525/525 cả interp và VM.
- Nightly: PASS 10 lần; fuzz ~18,8 triệu ca, 0 vi phạm; 475 tệp fmt bất động.
- Fresh-clone gate: PASS @ `9c3f804` (ST-232).
- Windows: workspace xanh @ `c12979f`/`da24c7d` (150 khối, 1334 test, 0 failed); HEAD hiện tại chưa verify lại; Tier 2.
- `vis-targets`: 16/18 ở cấu hình cũ (trước khi tách feature `sqlite` tùy chọn).

## 3. Lỗi đã biết (không bị xóa bởi số đo xanh)

- 10 phát hiện parity interp ↔ VM: #157..#166 (đều có repro `JOURNAL/st234-repro/`).
- 18 phát hiện std/CLI: #167..#184 (5 ca chạy lại trong phiên audit; 13 ca có dẫn chứng `tệp:dòng`).
- Không dùng differential 0 lệch để phủ nhận 10 ca trên.
- Không dùng Windows xanh tại commit cũ để tuyên bố Tier 1.

## 4. Quyết định áp dụng

VD-242 (Agent 0, 2026-11-20): mở đợt ổn định sau pha 1; duyệt xử lý cả 10 ca parity và triage đủ 18 ca std/CLI; chốt ngữ nghĩa optional chaining, Option coercion, map trùng khóa, JSON/sort, `sqlite.query` chỉ đọc; đính chính macOS + host identity; ratify proxy có chọn lọc; pha 2/VS3 chưa mở.

## 5. Cổng

- `fast`: PASS tại baseline.
- `full`: — (không chạy trong lượt ghi sổ này).
- `nightly`: PASS (số ở §2).

## 6. Ghi chú sổ sách

`head` ở trên là **baseline sản phẩm được đo**, không phải commit chứa báo cáo này (tránh tự tham chiếu hash). Mọi hàng QUEUE tại baseline được phủ trong `R31-viscript.json`. Các hàng ST-236 trở đi của đợt VD-242 sẽ được phủ bởi báo cáo vòng sau.
