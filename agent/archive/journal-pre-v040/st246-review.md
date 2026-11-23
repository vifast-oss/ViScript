<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-246 — review Agent 1 (VD-244 blocker tương thích bytecode/cache)

**Nguồn**: worktree `vd244-st246-compat`, commit Agent 2 `8bf73b0`. **Người review**: Agent 1. **Ngày**: 2026-11-20.

## 1. Đối chiếu 4 bằng chứng VD-244 §2

| Yêu cầu | Kết quả | Ghi chú review |
|---|---|---|
| Runtime mới từ chối artifact cũ | **Đạt** | `st246_version_compat.rs`: artifact v2 patch `version=1` ⇒ `decode → BadVersion(1)`, `load_verified → LoadError::Decode(BadVersion(1))`, `verify → BadVersion(1)`, `is_well_formed()==false`; patch lại `version=2` ⇒ decode OK, **chỉ khác đúng 2 byte** ⇒ chính `VERSION` là nguyên nhân. |
| Runtime cũ từ chối artifact mới | **Đạt theo mô phỏng gate** | Agent 2 trích mã tại `6eda337`: `decode.rs:248` và `verify.rs:307` đều `if version != VERSION` với `VERSION=1`; test `runtime_cu_tu_choi_artifact_v2` tái tạo hai gate đó (`OLD_VERSION=1`) trong cùng repo. **Không build riêng crate v1 thật** — ghi rõ để A0 quyết nếu cần mạnh hơn. Điều kiện chặn “runtime cũ không kiểm VERSION” **không xảy ra**. |
| Cache v1 không tái dùng; source recompile đúng 2 engine | **Đạt** | `khoa_multi_voi_version` tường minh; khoá v1 ≠ v2 (cả 0/3/99 đều khác), đường dẫn khác. E2E: cài artifact v1 **in sai `999`** vào path cache ⇒ VM vẫn in `42`, cache ghi lại v2; interp `42`; VM `--no-cache` `42`. |
| Bytecode v1 không source bị từ chối rõ | **Đạt** | `load_verified` ⇒ `Err(LoadError::Decode(BadVersion(1)))`; không có đường tương thích (VD-244 không duyệt). |

## 2. Phạm vi thay đổi (đã kiểm diff)

- `VERSION 1 → 2` trong `format.rs`; giữ `MAGIC`, bố cục, opcode, `API-VBC1.txt`, `API-CODES.txt`.
- `decode.rs`/`encode.rs`: **chỉ comment**; **`verify.rs` nguồn không đụng** (quyền ST-247 giữ nguyên).
- `cache.rs`: thêm `khoa_multi_voi_version` (chỉ để test), re-pin khoá ghim `61490404… → 66c332a9…` kèm lý do ST-246/VD-244.
- **Re-pin 2 test cũ bắt buộc**: `tests/decode_neg.rs` và `tests/verify.rs` dùng `2` làm sentinel “version lạ”; sau khi `VERSION=2` sentinel thành hợp lệ ⇒ đổi sang `3`. Không phải đổi ngữ nghĩa, không đụng nguồn verifier.

## 3. Kết luận review

**SHIP.** Đây là blocker **có thể đóng** theo bằng chứng đã nêu, với **caveat** chiều “runtime cũ” là mô phỏng gate trong cùng repo (có trích commit), không phải build crate v1 thật. Agent 1 sẽ ghi caveat này khi báo A0; nếu A0 yêu cầu build cũ thật, mở task bổ sung — không làm cản việc tích hợp hiện tại.

**Không** được suy ra “global-slot đã khép”: ST-247 (miền global suy từ lệnh ghi) vẫn mở. Không tag, không fresh-clone/release verification cho tới khi ST-247 xong.
