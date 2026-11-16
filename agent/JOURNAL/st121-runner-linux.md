# ST-121 · Runner Linux — thăm dò và dựng nền (2026-11-16)

Người làm: lead (đợt 23). Căn cứ: `JOURNAL/quyet-dinh-a0-2026-11-16.md` §6 **ST-121** + **VD-120**.

## 0. Kết luận một dòng

**Có nền Linux thật**: host `ps` qua tailscale/ssh — Ubuntu 26.04.1, x86_64, kernel 7.0.0-43,
12 core, **cargo/rustc 1.99.1** (đúng `rust-version` của workspace), `/sys/kernel/security/lsm`
có **landlock**, và syscall `landlock_create_ruleset` trả **ABI = 8** (không bị seccomp chặn).
⇒ **ST-131** (đo đầy đủ trên Linux) và **ST-132** (Landlock thật) **mở khoá**, không cần BLOCKED-ENV.

## 1. Thăm dò công cụ (theo đúng lệnh ST-121)

| Máy | docker | podman | colima | nerdctl | lima | Kết luận |
|---|---|---|---|---|---|---|
| macOS (máy dev) | không | không | không | không | `/opt/homebrew/bin/lima` | **không có container runtime**; lima chưa có VM nào được tạo, và ST-121 cấm tự cài phần mềm hệ thống |
| `ps` (Linux, tailscale) | **29.9.3** (có server) | không | — | — | — | có docker, **và** có rustup 1.99.1 chạy trực tiếp |

## 2. Lệch so với §6 ST-121 (ghi rõ, không giấu)

§6 giả định backend là **container trên máy dev**. Máy dev **không có** docker/podman/colima nên
giả định đó không dựng được trong ngân sách 1 h mà không cài phần mềm hệ thống (bị cấm).

Cách làm đã chọn: **backend `ssh` tới một host Linux thật** (`VIS_LINUX_SSH`, mặc định `root@ps`),
đồng bộ cây nguồn bằng rsync rồi chạy lệnh ở đó. Backend `docker` **vẫn được viết** kèm
`infra/linux/Dockerfile` (ghim `rust:1.99.1-bookworm`) để dùng khi máy có docker/lima.

Đây là **mạnh hơn** yêu cầu, không phải đường tắt: kernel thật, **không** seccomp container ⇒
bằng chứng Landlock ở ST-132 không bị container che. Mặt yếu đã biết: backend `docker`
**chưa chạy được** trên máy dev hiện tại (không có runtime) ⇒ mọi số đo Linux trong đợt 23 đến từ
backend `ssh`.

## 3. Log dò (nguyên văn, `bin/vis-linux probe --log …`)

```
== nền ==
Linux 7.0.0-43-generic x86_64
os: Ubuntu 26.04.1 LTS
nproc: 12
== công cụ ==
cargo: cargo 1.99.1 (5f94df478 2026-08-27)
rustc: rustc 1.99.1 (58cf8f9c3 2026-10-13)
python3: Python 3.14.4
cargo (đường dẫn): /root/.cargo/bin/cargo
== lsm ==
lsm: lockdown,capability,landlock,yama,apparmor,bpf,ima,evm
landlock trong lsm: CÓ
== syscall landlock ==
landlock syscall: CÓ (ABI=8)
== quyền ghi ==
ghi được: /root/.cargo
ghi được: /tmp
```

`sha256(st121-probe.log) = 65a07e70e79f788f1412e5676ff21a0271bb1fa27ff957d336974687ebc02ef7`

Cách dò syscall: `landlock_create_ruleset(NULL, 0, LANDLOCK_CREATE_RULESET_VERSION)` = syscall **444**
(giống nhau trên x86_64 và aarch64); trả số dương = ABI version, `ENOSYS`/`EOPNOTSUPP` = nền không có,
errno khác = **bị chặn** (seccomp/LSM) và **không tự nới**.

## 4. Đã dựng

| Tệp | Vai trò |
|---|---|
| `bin/vis-linux` | `probe` / `sync` / `run` / `gate` / `shell`; backend `ssh` (mặc định, đã đo) và `docker`. Mã thoát 3 = BLOCKED-ENV. Chỉ rsync `viscript/`, `docs/viscript/`, `.git`, `AGENTS.md` — **không** đụng `crates/`, `v2/` của dòng OS |
| `infra/linux/Dockerfile` | backend `docker`: cây nguồn mount **chỉ đọc** ở `/src`, build vào volume `/target`; ghim `rust:1.99.1-bookworm` |
| `agent/JOURNAL/st121-probe.log` | log dò + sha256 |

Lệnh tái lập:

```bash
bash viscript/bin/vis-linux probe --log viscript/agent/JOURNAL/st121-probe.log
bash viscript/bin/vis-linux gate          # = bash viscript/bin/vis-dev gate --fast, chạy trên host Linux
VIS_LINUX_NO_SYNC=1 bash viscript/bin/vis-linux run bash -c 'cd viscript && cargo test --workspace'
```

## 5. Ba lệch thật do chạy trên Linux phát hiện (đã sửa, có lý do)

Chạy cổng trên Linux lần đầu **đỏ** ở ba chỗ — cả ba đều là **lỗi thật của harness**, không phải
"máy Linux khác máy Mac":

1. **`test/fixtures/run_fixtures.sh` ca 5** ("môi trường sạch không có cargo") dựng môi trường bằng
   `PATH=/usr/bin:/bin`, ngầm giả định cargo **không** nằm ở đó (đúng trên macOS). Trên Ubuntu,
   cargo của distro ở `/usr/bin/cargo` ⇒ vẫn thấy cargo ⇒ rc=101
   (`failed to acquire package cache lock`, vì `HOME=/nonexistent`). Kỳ vọng của ca **không đổi**;
   nay PATH được dựng theo máy: một thư mục tạm chỉ có `dirname` (thứ `bin/vis` cần), không có cargo.
   Chứng minh ca còn bắt được vi phạm: một bản `bin/vis` sửa thành `exit 127` ⇒ ca báo
   `not ok 5 - thiếu cargo vẫn nổ 127` (đúng nhánh đầu).
2. **`test/report/check_report.py`** trỏ `v2/agent/BLOCKED.md` — tệp của **dòng OS**. Trên cây chỉ có
   `viscript/` (đúng cách một runner dòng ViScript nên được đồng bộ), nó nổ
   `FileNotFoundError: /root/vifast-linux/v2/agent/BLOCKED.md` ⇒ cổng ViScript **phụ thuộc** dòng kia.
   Nay trỏ `viscript/agent/BLOCKED.md`. Không đổi kết luận: **không** báo cáo ViScript nào dùng id `YC-`
   (`grep -ho 'YC-[0-9]*' reports/*.json` = rỗng), nên nhánh kiểm đó không bao giờ chạy với dòng này.
3. **cargo của distro cũ hơn `rust-version`**: `/usr/bin/cargo` = **1.93.1**, workspace đòi **1.99.1**.
   `bin/vis-linux` export `PATH=$HOME/.cargo/bin:…` (rustup) nên lệnh qua runner luôn dùng 1.99.1.
   **Không** sửa `find_cargo` để "ưu tiên rustup": người dùng đặt PATH thế nào là quyền của họ, và
   cargo tự báo lỗi rõ khi toolchain cũ. Đây là điều kiện môi trường, ghi ở đây để người sau không đoán.

Ngoài ra `test/doc-examples` ghim **số dòng** làm tên fixture; 4 dòng trỏ của ST-120 làm dịch số dòng
⇒ đã `--regen` (nội dung khối không đổi) và cập nhật `DOC_EXCLUDED` trong `crates/vis-cli/tests/check.rs`
(610→614, 662→666, 737→745, 1412→1420). Việc này ghi ở commit `[ST-120b]`.

## 6. Ảnh hưởng tới QUEUE

- **ST-131**: `READY` — runner đã có, không còn BLOCKED-ENV.
- **ST-132**: `READY` — `lsm` có `landlock` **và** syscall cho ABI=8; điều kiện "lsm có landlock" trong
  hàng QUEUE đã thoả **trước khi** thêm crate.
- **Q1/Q2 của Agent 0**: điều kiện "sau khi runner Linux chạy được" đã đạt; phần còn lại của Q1 là
  crate `landlock` (duyệt có điều kiện ở VD-120) — làm ở ST-132.
