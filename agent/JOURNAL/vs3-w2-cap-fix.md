<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3-W2 — cap hot-spin + cold-wake h2 (interest theo token, force-wake peer)

**Trạng thái**: sửa production tối thiểu, **đủ điều kiện merge** (chưa push/merge/tag theo lệnh).
Feature `async-h2` vẫn **default-off**; không đổi API sync/default/dependency; không GitHub Actions.
**Nhánh**: `vs3/w2-tls`; merge main **`fdd9e88`** (`Nguon`/Windows async net).
**Target dir (bắt buộc từ nay)**: `$PWD/target/w2` — **không** dùng target chung
`/home/ps/ViScript/target` (đã từng làm lẫn rmeta với main).
**Log**: [`vs3-w2-cap-fix.log`](vs3-w2-cap-fix.log).

## 1. Root cause (hai lỗi độc lập)

### 1.1 Cap 8 MiB — hot-spin interest

- `AsyncNet::register` (cũ `fd_wants`, W4 `nguon_wants`) **OR tích luỹ** hướng chờ theo nguồn và
  không thu hẹp. Sau lần đầu cần WRITABLE (TCP/TLS), nguồn kẹt `READABLE|WRITABLE`.
- `ReactorOp::ReadWrite` luôn đăng ký `(true,true)`, dù h2 sau khi gửi request chỉ tắc READ.
  epoll level-trigger + socket luôn WRITABLE ⇒ scheduler quay tới **200.000 turn** (VS0501),
  hoặc state rối rơi về h1 (`phản hồi HTTP không đủ header`).

### 1.2 Cold-start — mất wake sau `doi_khoa_nguon` (W4)

- `doi_khoa_nguon` `T:… → H<n>` **không làm mất token**, nhưng lộ một lỗi protocol:
  một frame gọi `h2_poll` có thể **drain response của mọi stream** trên session chung. Khi frame
  đó nhận response của chính nó (`ReactorH2Outcome::Response`), các frame khác đã được phục vụ hộ
  nhưng **không còn readiness socket** để đánh thức ⇒ 1/4 task ngủ tới timeout.
- Đây là lý do cold-start chỉ fail khi chạy **song song** (timing khác), còn serial xanh.

## 2. Sửa tối thiểu

1. **Per-token interest**: `token_wants: HashMap<token,(read,write)>`; interest nguồn = hợp
   **hiện tại** của các token còn sống; `deregister` thu hẹp bằng `reregister`; port lên
   W4 `Nguon` (`async_net_nguon.rs`), **không** đưa `SourceFd`/`RawFd` trở lại `async_net.rs`.
2. **`ReactorOp::ReadWrite`** lấy hướng từ `H2Async::interest()` (`muon_doc`/`muon_ghi`).
3. **Force-wake peer**: `waker` + `wake_nguon`; `h2_poll` sau **mọi tiến triển** (`tien`, kể cả
   khi frame hiện tại đã có `Response`) gọi `danh_thuc_nguon(H<n>)` nếu nguồn còn >1 token;
   `wait` phát event tổng hợp cho mọi token cùng nguồn. Không spin vì chỉ phát khi có tiến triển.
4. Test-only: snapshot binary cho cap/soak (tránh cargo build khác feature ghi đè binary).
5. Đóng/mở h2/TLS chuyển vào `async_net_h2.rs` (`dong_h2`) để giữ `async_net.rs` ≤1500 dòng.

## 3. Kiểm chứng (target riêng `$PWD/target/w2`)

| Hạng mục | Kết quả |
|---|---|
| Cap 2.000 × 2 (release, debug-assertions cho TLS root test) | **2.000/2.000** mỗi lần — **4.000/4.000**, 305 s / 376 s |
| Cold-start **1.000 vòng song song** (`VS3_W2_COLD_ROUNDS=1000`, full suite parallel) | **PASS**, 44,6 s; mỗi vòng **1 connection / 4 stream** |
| Feature `vs3_http2` song song | **34 pass + 1 ignored** × **3 lần** (16,0–16,1 s) |
| Feature `vs3_http2` serial | **34 pass + 1 ignored**, 51 s |
| Default `vs3_http2` | **19/19** |
| Feature `vs3_default` | **22/22** |
| Feature `vs3_tls` | **21/21** |
| fmt / clippy default + feature / `./vis-dev check` | **OK / PASS** |
| R1 | `async_net.rs` **1490**, `async_net_h2.rs` 176, `async_net_nguon.rs` 312, host 111, support 987 — tất cả ≤1500 |
| df | **12 G trống** (>5 G) |

Lưu ý: các số 10k/6k của đợt trước dùng target chung đã **không được tính**; kết quả trên là
chạy lại sạch trên target riêng. Debug 2.000 từng fail vòng 1843 do binary debug bị build khác
feature ghi đè — đã khoá bằng snapshot; không tái hiện trên target riêng.

## 4. Quyết định

- **Đủ điều kiện merge** theo tiêu chí: cap 4.000 vòng sạch, cold 1.000 vòng song song, feature
  suite 3× + serial xanh, default/TLS/default async xanh, fmt/clippy/check PASS.
- Feature vẫn **default-off**; **không** bật default-on trong đợt này (còn cancel task + h2
  Windows + soak release dài).
- Không push/merge/tag; commit trên nhánh `vs3/w2-tls`.

## 5. Còn mở

- Cancel task giữa request (nợ chung h1+h2).
- `async-h2` Windows chưa port (W4 mới port TCP/TLS).
- Soak release 30 phút sau thay đổi cold-wake (nên chạy trước khi merge).
