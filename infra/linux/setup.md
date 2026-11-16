# `viscript/infra/linux/` — tái lập runner Linux (ST-134, VD-136)

Runner Linux của VS2 là **máy thật `ps`** (Ubuntu, truy cập qua ssh từ máy dev), không phải container
(VD-136 sửa VD-134). Thư mục này giữ **script tái lập** để lần đo nào cũng bắt đầu từ cùng một môi
trường, không cài tay.

## 1. Tệp

| Tệp | Vai trò |
|---|---|
| `setup.sh` | Kiểm + cài **thành phần toolchain** còn thiếu (`rustup target`), in **log môi trường + digest** mỗi lần chạy |
| `setup.md` | Tệp này: cách gọi, cái gì được cài, cái gì **không** |
| `Dockerfile` | Phương án container cũ; **chưa build**, không phải đường đo của VS2 (VD-138). Giữ làm tham chiếu |

## 2. `setup.sh` cài gì

Chỉ hai **target của rustup** (thành phần toolchain, không phải phần mềm hệ thống):

```
wasm32-unknown-unknown
x86_64-pc-windows-msvc
```

Vì sao **hai** (không chỉ `wasm32`):

- `wasm32-unknown-unknown` — R23 §4.4 lệch 5: ca `targets_wasm_vis_types` và các ô `wasm32` của
  `bin/vis-targets` đỏ trên runner vì thiếu target này.
- `x86_64-pc-windows-msvc` — ma trận `bin/vis-targets` có **8 ô** `cargo check --target
  x86_64-pc-windows-msvc`; thiếu std của target thì cả 8 ô đỏ, nên `bin/vis-targets` không thể 18/18.
  Đây vẫn là **rustup target** (không phải Windows SDK/Build Tools): `cargo check` chỉ cần std, không
  link.

Script **không** làm gì khác: không `rustup update`, không đổi toolchain mặc định, không `sudo`,
không cài gói OS. Thiếu `rustup`/`rustc` ⇒ **exit 3** (BLOCKED-ENV).

## 3. Chạy

```bash
# trên máy dev, đẩy sang runner rồi chạy (khuyến nghị — đồng bộ cây nguồn trước):
bash viscript/bin/vis-linux setup

# hoặc trên chính runner, trong cây đã đồng bộ:
cd /root/vifast-linux
bash viscript/infra/linux/setup.sh

# chỉ kiểm, không cài (CI/pre-flight):
VIS_SETUP_CHECK=1 bash viscript/infra/linux/setup.sh
```

`bin/vis-linux run` và `bin/vis-linux gate` **tự gọi** bước setup sau khi đồng bộ (idempotent: target
đã có thì chỉ in "đã cài (không cài lại)"). Tắt tự gọi bằng `VIS_LINUX_NO_SETUP=1` khi cần đo đúng
môi trường hiện có.

Mã thoát của `setup.sh`: **0** xanh · **1** thiếu target (chế độ `VIS_SETUP_CHECK=1`) · **3**
BLOCKED-ENV (không có rustup/rustc, hoặc `rustup target add` thất bại).

## 4. Log mỗi lần chạy (VD-136)

Script **luôn** in ra stdout và nối vào `${VIS_SETUP_LOG:-$HOME/.cache/vis-setup.log}` một khối:

```
=== setup toolchain (VD-136) <UTC> ===
hostname: <tên máy>
os: <PRETTY_NAME>
kernel: <uname -srm>
rustc: <dòng đầu rustc -Vv>
host-target: <dòng host: của rustc -Vv>
<toàn bộ rustc -Vv>
<cargo -V>
<rustup --version>
active-toolchain: <rustup show active-toolchain>
git HEAD: <git rev-parse HEAD của cây đang đo>
setup.sh sha256: <sha256 của chính script>
digest-toolchain: <sha256(rustc -Vv + cargo -V + rustup -V + active + target đang có + git HEAD)>
target cần: …
target đang có (trước): …
```

Cuối mỗi lần chạy in `digest-toolchain (sau)`, `setup.sh sha256` và một dòng
`=== setup kết thúc: HEAD=… digest=… ===`. Nhờ vậy mỗi log đều trả lời được: máy nào, kernel nào,
rustc nào, target nào, **hash commit nào**, và **script phiên bản nào** (sha256) — đúng danh sách
VD-136.

## 5. Sau khi chạy: đo lại

```bash
bash viscript/bin/vis-linux run bash -c 'cd viscript && bash bin/vis-targets'
```

Kỳ vọng: **18/18 ô `check` PASS** (8 crate × 2 target + `vis-host-wasm` × wasm32 + `vis-host-linux` ×
linux-gnu). Mức bằng chứng vẫn là **chỉ CHECK biên dịch chéo** (VD-121) — không chạy trên
Windows/wasm.
