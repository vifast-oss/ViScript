# ST-135 · Chạy lại đầy đủ trên Linux sau ST-134 — chỉ đo (2026-11-16)

Người làm: lead (đo trên runner `ps`). Căn cứ: `KE-HOACH-CHOT-23f.md` **§2 mục ST-135**.
Task này **chỉ đo**, không sửa gì.

## 1. Cây đo

`viscript/bin/vis-linux run` (sync + không chạy setup, `VIS_LINUX_NO_SETUP=1`) ⇒ **HEAD `7881eb30`**
(đã gồm ST-137 `e47cfb2f`, ST-130b `eab366d7`, ST-132b `a47a1ffe`, ST-134 `6e6673bd`).
Máy: Ubuntu 26.04.1, kernel 7.0.0-43-generic, 12 core, rustc/cargo **1.99.1**, Landlock ABI **8**.

## 2. Số đo

| Lệnh | Kết quả |
|---|---|
| `cargo test --workspace --no-fail-fast` | **1161 passed / 1 failed**, **129** suite |
| `bin/vis-host-suite` | **54/54** (exit 0) |
| `bin/vis-targets` (sau `infra/linux/setup.sh` của ST-134) | **18/18 PASS** |
| `bash viscript/bin/vis-dev gate --fast` | 10/11 bước PASS; **L0 đỏ đúng 1 ca** (dưới) |

So với R23: Linux **1147/6 → 1161/1**. Năm ca đỏ đã hết nhờ ST-134 (2 ca theo VD-132 + wasm target +
ca `.vish`/`/tmp`) và ST-137 (TOCTOU).

## 3. Ca đỏ duy nhất — môi trường, không phải sản phẩm

`crates/vis-cli/tests/grant_proc_canon.rs::symlink_canonicalize_hai_phia`:

```
error VS0501: chương trình ngoài `/tmp/vis-grant-canon-symlink-…/ln-echo` thoát mã 1:
Security violation: Requested utility `ln-echo` does not match executable name:
  /usr/lib/cargo/bin/coreutils/echo
```

- Trên `ps`, `/usr/bin/echo` là **symlink vào coreutils multicall**; binary multicall **từ chối**
  chạy khi `argv[0]` khác tên nó. Ca tạo một symlink tên `ln-echo` ⇒ đúng cơ chế bị multicall chặn.
- **Sản phẩm đúng**: nó canonicalize rồi exec **đúng đường dẫn thật** (chính là điều ST-123 kiểm).
  Đây là **khác biệt môi trường**, không phải lỗi quyền.
- **Không sửa** trong task này (ST-135 chỉ đo). VD-132 hiện cho phép **2** ca; đây là ca thứ **3** ⇒
  cần Agent 0 mở VD mới, hoặc cài `echo` không-multicall trên runner. Đã ghi nợ **#139** ở R24 §5.

## 4. Lệnh tái lập

```bash
bash viscript/bin/vis-linux run bash -c 'cd viscript && cargo test --workspace --no-fail-fast'
bash viscript/bin/vis-linux run bash -c 'cd viscript && bash bin/vis-host-suite'
bash viscript/bin/vis-linux run bash -c 'cd viscript && bash bin/vis-targets'
bash viscript/bin/vis-linux gate
```
