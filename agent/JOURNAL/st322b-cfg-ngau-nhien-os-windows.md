<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-322b — cfg-gate import `ngau_nhien_os` trong `sqlite/mod.rs` cho Windows

- **Ngày**: 2026-11-27 03:32 UTC · **Vai**: Builder (Agent 2)
- **Nhánh**: `plan/st322b` · **Worktree**: `/home/ps/viswt/st322b`
- **Nền**: `main` @ `304cb610`
- **Trạng thái**: fix 6 dòng; gate Linux xanh. **Cần re-run `psvn-gate-full` sau merge** — Builder không SSH sang psvn, không tự nghiệm thu Windows.

## 0. TL;DR

Re-run `psvn-gate-full` sau ST-321d vẫn **FAIL exit=101**, nhưng lỗi **mới**:
`error: unused import: ngau_nhien_os` tại `crates\vis-host-native\src\db\sqlite\mod.rs:103:34`
(`-D unused-imports` implied by `-D warnings`). Log gốc:
`/home/ps/vislogs/peak2-psvn-20261127T0319Z/psvn-gate-full.log`, kết quả:
`/home/ps/vislogs/peak2-psvn-result-20261127T0320Z.md`. Lỗi `unused import Command` của ST-321d
đã hết (xác nhận trong log) — đây là defect độc lập, không phải regress ST-321d.

## 1. Điều tra — tại sao unused chỉ trên Windows

| Vị trí | Vai trò | cfg |
|---|---|---|
| `sqlite/mod.rs:103` | `#[cfg(test)] use token::{ngau_nhien_du_phong, ngau_nhien_os};` | import dùng chung cho module test |
| `sqlite/tests.rs:4` | `use super::*;` | kế thừa import của `mod.rs` |
| `sqlite/tests/sqlite_a.rs:4` | `use super::*;` | **không có import riêng** — lấy `ngau_nhien_os` qua 2 lần glob này |
| `sqlite/tests/sqlite_a.rs:71-77` | `ngau_nhien_os_hai_lan_khac_nhau()` — **nơi duy nhất dùng** `ngau_nhien_os` | **`#[cfg(unix)]`** |
| `sqlite/tests/sqlite_a.rs:81-87` | `entropy_du_phong_tach_mien_va_bo_dem()` — dùng `ngau_nhien_du_phong` | không gated (mọi nền) |
| `token.rs:24` | định nghĩa `ngau_nhien_os()` đọc `/dev/urandom` | `#[cfg(unix)]` |
| `token.rs:39` | stub `ngau_nhien_os() -> None` | `#[cfg(not(unix))]` |
| `token.rs:16` | `token_ngau_nhien()` gọi nội bộ `ngau_nhien_os()` | không cần import ở `mod.rs` |

⇒ Trên Unix: ca test `#[cfg(unix)]` biên dịch và dùng import ⇒ không cảnh báo.
Trên Windows: ca test bị loại, `token_ngau_nhien` gọi trong module `token` (không qua import),
`mod.rs` không còn chỗ nào dùng `ngau_nhien_os` ⇒ `-D warnings` đỏ.
(Lưu ý: báo cáo psvn §5 nói "sqlite_a.rs có import riêng" — không đúng; tệp dùng `use super::*`.)

## 2. Patch (tối thiểu, 0 đổi hành vi / runtime)

```diff
 #[cfg(test)]
-use token::{ngau_nhien_du_phong, ngau_nhien_os};
+use token::ngau_nhien_du_phong;
+/// Ca test duy nhất dùng `ngau_nhien_os` là `tests/sqlite_a.rs` (`#[cfg(unix)]`) ⇒ trên Windows
+/// import này là dư (`-D warnings` ⊗ exit 101). `token_ngau_nhien` gọi nội bộ trong `token.rs`,
+/// không cần import ở đây. Giữ `ngau_nhien_du_phong` cho mọi nền (ca fallback không gated).
+#[cfg(all(test, unix))]
+use token::ngau_nhien_os;
```

- Giữ nguyên test behavior: Unix vẫn import + chạy ca OS RNG; Windows bỏ import dư.
- Không đụng `token.rs`, không đụng test, không bump cache/VBC1 (test-only import).
- `cargo fmt --all` chỉ sắp xếp lại dòng `use token::token_dung_dang;` (không đổi ngữ nghĩa).

## 3. Gate Linux (đều PASS)

Worktree riêng, **không đụng repo chính** (WASI/bench đang chạy trên `main`).
Toolchain ghim **1.100.0** dùng trực tiếp
`PATH=/home/ps/.rustup/toolchains/1.100.0-x86_64-unknown-linux-gnu/bin:$PATH`
(`/usr/bin/cargo` là 1.93.1 ⇒ bị từ chối MSRV; không có `rustup` shim trên PATH),
`CARGO_BUILD_JOBS=2`, `TMPDIR=/home/ps/vislogs/tmp-st322b` (TMPDIR đĩa, tràn tmpfs `/tmp`).

| Lệnh | RC | Ghi chú |
|---|---|---|
| `cargo fmt --all` | 0 | chỉ patch này đổi |
| `cargo test -p vis-host-native` | 0 | 31 unit + 1 posix + 9 cookie + 8 net + 79 security + doc 0; **0 failed** |
| `./vis-dev gate --fast` | 0 | `PASS (L1 chưa có tới khi có sim ViScript)`, L0 OK |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 | `Finished dev`, không warning |

Log `/home/ps/vislogs/st322b-*.log`, sha256:

```
e5f299e922c3fc187bda2ab7af7a1125fd481a0a94eec0e31d272597aaeaacef  st322b-fmt.log
7452c5009d6644fa23d3e8691c2a394786ebe5dfedf8693a40dfbcc55f0238ff  st322b-test.log
ffc7275dc442ce2a0d95c79d922bd5e3a8b91b0cd4ce5f0f56bcc734945c26c0  st322b-gate.log
bdee1b60b78ed5e19cd6b19ca1a9adbddc1e3180f1349c0f0612fe7fd1cf5e74  st322b-clippy.log
```

Lần chạy test đầu (trước khi build binary `vis`) fail ca `host_posix_real` do thiếu binary —
đây là thứ tự chạy, không phải defect; sau `gate --fast` (tự build workspace) chạy lại xanh.

## 4. Điều kiện nghiệm thu / chuyển giao

- **Không SSH sang psvn**, không merge, không push — Builder only.
- **Bắt buộc sau merge**: đồng bộ sang psvn rồi chạy lại `psvn-gate-full`
  (`bin/vis-gate-windows --full`) để xác nhận Windows hết `unused import ngau_nhien_os`
  và clippy `-D warnings` xanh, mới được mở ca test/bỏ trạng thái BLOCKED.
- Disk lúc chạy: `/` còn **12 G** (> ngưỡng dừng 5 G); không bump cache/VBC1.
