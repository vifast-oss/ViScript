# PHA 1 ĐÓNG CHÍNH THỨC — `pha1-rc1` (2026-11-19, VD-237)

> Tệp đóng khung pha 1. **Không** trỏ tới `/tmp`. Mọi số thuộc máy đo, ghi một lần kèm HEAD.

**Kết luận**: pha 1 dòng ViScript **đóng chính thức**. Tag `pha1-rc1` (annotated, `48c1da6`) trỏ tới commit `05e42d01483d255987750656fa6cba6c434831c9` (`[VD-238]`). Ratify bởi A0 ngày 2026-11-19 (VD-241 §1).

## Số đo đóng (2026-11-19)

| Máy | Kết quả |
|---|---|
| `ps` (Linux) | clone sạch @ `pha1-rc1`: **gate PASS 10/10**; sem 469/469 interp + 469/469 VM; workspace **147/147** binary, 0 đỏ |
| `mps` (arm64 Apple M6, macOS 27.0.1 Build 26A434, Darwin 27.0.0 RELEASE_ARM64_T8152, 12 lõi; 33Gi trống trước clone; git 2.54.0; rustc/cargo 1.100.0 stable-aarch64-apple-darwin; clone sạch + `checkout --detach pha1-rc1`) | **gate PASS 10/10** · sem **interp 469/469 · vm 469/469** · workspace **147/147** (`0 FAILED`) |

Đủ 4 điều kiện VD-237: (a) cả hai máy cùng tag, cùng 147 binary; (b) VD-231 + VD-232 DONE; (c) clone sạch ps gate PASS @ tag; (d) báo cáo đã sửa 6 mục VD-233. R30 §2/§5 cập nhật dạng addendum (VD-236), `docs/R30-MAU-2026-11-18.md` giữ nhãn nháp.

**Pha 2** (chưa bắt đầu lúc ghi tệp này): VD-240 hướng **(b)** — test grid một luồng cho async/kênh; hướng (a) (đa luồng thật) hoãn, nếu mở lại ưu tiên deep-copy/byte-serialize qua kênh (không chuyển `Value` sang `Arc+Mutex`, VD-178 QĐ-1).
