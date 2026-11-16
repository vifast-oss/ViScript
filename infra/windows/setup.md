# `viscript/infra/windows/` — tái lập máy đo Windows (ST-136, VD-136, VD-140)

Máy đo Windows của VS2 là **máy thật `psvn`** (Windows 11, truy cập qua ssh từ máy dev). Windows trong VS2 **chỉ ĐO**
(VD-136): không hỗ trợ, không sửa mã cho Windows (VD-121). Thư mục này giữ **script tái lập** để lần đo sau bắt đầu
từ cùng một toolchain, không cài tay.

## 1. Tệp

| Tệp | Vai trò |
|---|---|
| `setup.ps1` | Kiểm + bảo đảm **đúng toolchain 1.99.1** (bản `ps` đang đo) và **hai target std** của `bin/vis-targets` (VD-146(1)), in log môi trường; `-Check` thì chỉ kiểm, không cài |
| `setup.md` | Tệp này: cách gọi, cái gì được cài, cái gì **không**, và môi trường đã đo |

## 2. Vì sao phải ghim toolchain

- `ps` (Linux) đo bằng **rustc 1.99.1** (`58cf8f9c3`, 2026-10-13); repo khai `rust-version = "1.99.1"`.
- Trên `psvn`, "stable" **không** phải 1.99.1 (stable = 1.100.0 sau 2026-11-16) ⇒ nếu để rustup tự chọn
  (`rust-toolchain.toml` khai `channel = "stable"`) thì số đo **không so sánh được** với `ps`/macOS.
- Vì vậy **mọi** lệnh đo phải ghim:

```powershell
$env:RUSTUP_TOOLCHAIN = "1.99.1-x86_64-pc-windows-msvc"
$env:CARGO_TARGET_DIR = "C:\vis-target"    # target dir riêng (VD-136)
$env:CARGO_TERM_COLOR = "never"
```

> **Bẫy đã gặp (2026-11-16):** chạy `rustup -V` **trần** ở gốc repo ⇒ rustup tự cập nhật "stable" 1.96.0 → 1.100.0
> và cài thêm `rust-src`, `rust-docs`, std của `x86_64-unknown-none`, `aarch64-unknown-none`,
> `riscv64gc-unknown-none-elf`, `wasm32-unknown-unknown` **vào stable**. Không ảnh hưởng số đo (đã ghim 1.99.1),
> nhưng đừng lặp lại; `setup.ps1` đặt `RUSTUP_TOOLCHAIN` trước mọi lệnh.

## 3. `setup.ps1` cài gì / không cài gì

**Cài:** toolchain `1.99.1-x86_64-pc-windows-msvc` **nếu thiếu** (đúng phiên bản `ps`, VD-140) và **hai target std**
của `bin/vis-targets` **nếu thiếu** (VD-146(1), 2026-11-16) — mỗi lần cài ghi `rustup target list --installed` trước/sau.
**Không:** `rustup update`, không đổi toolchain mặc định, không cài target nào khác, không `sudo`, không Build Tools,
không gói OS, không reboot. `-Check` chỉ kiểm, **không cài** (thiếu ⇒ exit 1).

**Target cho `bin/vis-targets`:** toolchain 1.99.1 trên `psvn` chỉ có `x86_64-pc-windows-msvc`; **VD-146(1) cho phép
cài** std `wasm32-unknown-unknown` + `x86_64-unknown-linux-gnu` để ma trận 18/18 đo được. Lệnh (script tự ghim
`RUSTUP_TOOLCHAIN`; gọi từ gốc repo để log kèm hash):

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File ...\repo\viscript\infra\windows\setup.ps1 -Log C:\vis-setup.log
```

## 4. Chạy

```powershell
# trên máy dev
ssh psvn

# trên psvn
powershell -NoProfile -File C:\Users\ps\vifast-windows\repo\viscript\infra\windows\setup.ps1
# chỉ kiểm (thiếu ⇒ exit 1, không cài)
powershell -NoProfile -File ...\setup.ps1 -Check
```

| Mã thoát | Nghĩa |
|---|---|
| 0 | xanh (đủ toolchain + đủ 2 target std) |
| 1 | `-Check` và thiếu thành phần |
| 3 | BLOCKED-ENV (không có `rustup` trong PATH) |

## 5. Môi trường đã đo (ST-136, 2026-11-16)

| Mục | Giá trị |
|---|---|
| Máy | `PSVN` · Microsoft Windows 11 Home 64-bit · version 10.0.26300 · build 26300 |
| rustup | `1.29.2 (d8c92c676 2026-10-21)` |
| rustc | `1.99.1 (58cf8f9c3 2026-10-13)` · `x86_64-pc-windows-msvc` · LLVM 23.1.1 |
| cargo | `1.99.1 (5f94df478 2026-08-27)` |
| Toolchain có | stable (**1.100.0**), nightly, 1.90.0, 1.96, **1.99.1** |
| Target 1.99.1 | `x86_64-pc-windows-msvc` |
| Component 1.99.1 | cargo, clippy, rust-std, rustc, rustfmt |
| Build Tools | VS 2022 Community **17.14.8** · MSVC **14.44.35207** · Windows SDK 10.0.22621.0 + 10.0.26100.0 — **không cập nhật** (build/link chạy được, không có log lỗi) |
| Git Bash | `GNU bash 5.2.37(1)` tại `C:\Program Files\Git\bin\bash.exe` (bash mặc định của Windows là **WSL**, không dùng) |
| Python | `python 3.13.3`; `python3` = uv `cpython-3.12.10` (bản `vis-dev` chọn) |
| Repo / target | `C:\Users\ps\vifast-windows\repo` (`core.autocrlf false`) · `C:\vis-target` |
| Hash đã đo | `e369698adf28c4f8aa3b5883fec9af5569a93920` (`origin/main` 2026-11-16T07:53Z) |

Số đo và danh sách ca lệch: [`agent/JOURNAL/st136-windows.md`](../../agent/JOURNAL/st136-windows.md).
