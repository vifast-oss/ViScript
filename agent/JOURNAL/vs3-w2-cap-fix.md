<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3-W2 — sửa hot-spin cap 8 MiB async-h2 (sau Windows/Nguon)

**Trạng thái**: sửa production tối thiểu; feature `async-h2` vẫn **default-off**, không đổi API
sync/default/dependency, không GitHub Actions.
**Nhánh**: `vs3/w2-tls`; merge main **`a8100ab`** (W4 Windows async net + `Nguon`).
**Log**: [`vs3-w2-cap-fix.log`](vs3-w2-cap-fix.log).

## 1. Root cause

Có **hai** lỗi ghép lại trong đăng ký readiness của backend async:

1. **Interest tích luỹ theo nguồn/fd**: `AsyncNet::register` (bản cũ `fd_wants`, bản W4
   `nguon_wants`) chỉ **OR** hướng chờ và **không bao giờ thu hẹp**. Sau lần đầu cần WRITABLE
   (TCP connect/TLS), nguồn kẹt `READABLE|WRITABLE` mãi.
2. **`ReactorOp::ReadWrite` luôn đăng ký cả hai chiều**, dù `H2Async` chỉ tắc ở READ sau khi đã
   gửi request. Với epoll **level-trigger**, socket gần như luôn WRITABLE ⇒ mỗi vòng scheduler
   được đánh thức, `h2_poll` không có dữ liệu mới, re-arm, lặp tới **200.000 turn** (VS0501) hoặc
   tranh chấp state làm rơi về đường h1 (`phản hồi HTTP không đủ header`).

## 2. Sửa tối thiểu

- `AsyncNet` (và `Nguon` sau W4):
  - thay union theo nguồn bằng **`token_wants: HashMap<token,(read,write)>`**;
  - `interest_cua_nguon(key)` = hợp **hiện tại** của các token còn sống;
  - `register`: chèn token + wants rồi mới tính interest; lỗi thì `bo_token`;
  - `deregister`: xoá token, nếu còn token khác thì `reregister` với interest **thu hẹp**;
  - `doi_khoa_nguon`/`bo_sach_nguon` cập nhật sổ token, không giữ union theo khoá.
- `ReactorOp::ReadWrite`: lấy hướng từ **`H2Async::interest()`** (`muon_doc`/`muon_ghi`), không
  mặc định `(true,true)`.
- Port lên W4 `Nguon` (`async_net_nguon.rs`): **không** đưa `SourceFd`/`RawFd` trở lại
  `async_net.rs`; h2 vẫn dùng đường Unix riêng có sẵn (`h2_async.rs`, default-off).
- Test-only: snapshot binary `vis` cho cap/soak (tránh cargo build đồng thời ghi đè
  `target/debug/vis` — nguồn của lỗi h1 giả trong log 2.000 vòng debug).

## 3. Kiểm chứng

- **10.000 vòng cap release** (trước port W4, cùng logic interest): **10.000/10.000 PASS**,
  2151 s; log `vs3-w2-cap-fix.log`.
- **2.000 × 3 lần release sau port W4/Nguon**: **2.000/2.000 mỗi lần**, 0 fail
  (260,77 s / 112,48 s / 347,78 s) — tổng **6.000/6.000**.
- Smoke debug sau port: 200/200 PASS (74,7 s).
- Log 2.000 debug trước port có 1 fail ở vòng 1843 (`phản hồi HTTP không đủ header`). Nguyên nhân
  đã khoanh vùng là **test harness**: cap loop spawn `target/debug/vis` dùng chung, cargo build
  đồng thời (feature khác) ghi đè binary ⇒ một số vòng chạy binary không có `async-h2` (ALPN h2
  server, client h1 ⇒ EOF header). Đã sửa test-only bằng **snapshot binary** trước loop; release
  10k/6k không còn lỗi này. Đây **không** phải fallback production.
- Tuy nhiên full `vs3_http2` **song song** sau merge W4 vẫn fail **cold-start** (1/4 task timeout
  10 s), trong khi chạy `--test-threads=1` **34 pass + 1 ignored**. Đây là **residual của W4/Nguon
  + chia sẻ handshake canonical**, chưa phải cap; xem §5.

## 4. Regression / R1 / df

| Cấu hình | Kết quả |
|---|---|
| feature `vs3_http2` `--test-threads=1` | **34 pass, 0 fail, 1 ignored** |
| feature `vs3_http2` song song (mặc định) | **33 pass, 1 fail**: cold 20 vòng, 1 task timeout |
| default `vs3_http2` | **19/19** |
| feature `vs3_default` | **22/22** |
| feature `vs3_tls` | **21/21** |
| fmt / clippy default+feature / `./vis-dev check` | **OK / PASS** |

R1: `async_net.rs` **1500**, `async_net_nguon.rs` 312, `async_net_host.rs` 111,
`h2_async_spike.rs` 987 — tất cả ≤ 1500. `df`: **22 G trống**.

## 5. Quyết định merge / default-on

- **Cap 8 MiB: đã sửa và kiểm chứng** (10k trước port + 6k sau port, 0 fail; hot-spin VS0501
  không còn; interest thu hẹp đúng chiều).
- **Quyết định: CHƯA merge** vì required `vs3_http2` chạy song song còn **1 cold-start timeout
  / 20 vòng** sau khi merge W4/Nguon. Đây là regression chia sẻ handshake canonical nhiều frame
  (không phải cap), cần khoanh vùng `doi_khoa_nguon`/`Wait::Reactor` trước khi merge.
- Feature vẫn **default-off**; **không** bật default-on.
- Hướng tiếp: log token/`doi_khoa_nguon` cho frame bị mất wake; có thể phải đăng ký lại reactor
  key khi nguồn đổi khoá `T:…` → `H<n>` (W4 mới thêm `doi_khoa_nguon`) hoặc giữ handle nguồn ổn
  định cho h2. Sau đó chạy lại 2.000×3 + full song song.

## 6. Còn mở

- Cancel task giữa request (nợ chung h1+h2).
- `async-h2` trên Windows chưa port (W4 mới port TCP/TLS; h2 vẫn Unix-only).
- 10k sau rebase nếu chưa chạy đủ; soak release dài.
