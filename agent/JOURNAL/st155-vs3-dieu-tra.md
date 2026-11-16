<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-155 — điều tra CHỈ ĐỌC để chuẩn bị tài liệu thiết kế VS3

**Task**: ST-155 (`QUEUE.md`, 2 h) · **Ngày** 2026-11-17 · **HEAD khi đọc** `93e1d418` · **Căn cứ mở**: VD-153, VD-148/VD-151,
`JOURNAL/quyet-dinh-a0-2026-11-17.md` §5. **Đã giữ**: không sửa `crates/`/`docs/`/`spec/`/`v2/`, không thêm crate, không
`cargo add`, không nightly/soak. **Nhãn**: `[ĐO]` = số đã có log trong `JOURNAL/`; `[ƯL]` = ước lượng (kèm cơ sở);
`[GT]` = giả thuyết phải đo. **Không** con số nào dưới đây là số tôi đo mới.

**Kết luận một dòng**: VS3 để lại sáu lỗ hổng đúng như hiến chương, nhưng ba trong sáu (async · kênh · quyền-ghi) **chặn ở
cùng một chỗ**: `Host` là `&mut dyn` một luồng, `Value` dùng `Rc`/`RefCell`, và `FsHost` định danh tệp bằng **tên**, không
bằng handle — nên không mục nào vá cục bộ được.

---

## 1. Async **song song thật**

**Hiện trạng.** `crates/vis-interp/src/rt.rs:2` — "bộ lập lịch **một luồng, tất định**"; `rt.rs:10` ghi nợ nguyên văn
"**không** nhường giữa thân ⇒ chưa có song song thật; **không** GC (chu trình `Rc`)". `rt.rs:216-218`: `await` chạy thân
**một phát** (`host.goi_than(…)`) rồi đặt `Xong` ⇒ **không có điểm dừng** để chèn suspend/resume; `rt.rs:149-173` `chay_mot`
chỉ rút task `Moi` khỏi hàng FIFO; `rt.rs:239-247` join theo **thứ tự tạo**, gọi ở `interp.rs:207-209` (interp) và
`vm.rs:189-192` (VM) — **một** hiện thực cho hai đường (VD-95). Chỗ chặn `Send`: `value.rs:67-70` (`Task(Rc<…>)`,
`Chan(Rc<RefCell<…>>)`), `rt.rs:49,51` (`Cell` + `RefCell`), `builtin.rs:239` (`fn sched(&self) -> &RefCell<Scheduler>`).
Thiết kế đích ngược lại: `docs/viscript/ViScript-thiet-ke.md:776` `pub trait Host: Send + Sync` + method `&self`; `:724`
(`parallel.map`/`thread.spawn` trên luồng OS); `:562` (mỗi luồng heap + bộ đếm **không atomic** riêng). `unsafe`: **0** dòng
trong sản phẩm — 10 lần chữ "unsafe" đều ở chú thích (`vis-types/src/lib.rs:5`, `vis-bytecode/src/lib.rs:4`). `rt.rs:24`
`MAX_TASK_NEST = 256` (mỗi `await` là khung gọi thật) — mô hình máy trạng thái sẽ **gỡ** chặn này và đổi ngữ nghĩa `VS0501`.

**Việc cần cho VS3.** Scheduler đa luồng/work-stealing + suspend–resume + wait-set, **kèm** quyết định mô hình bộ nhớ/GC:
(a) giữ `Rc` + **heap riêng mỗi luồng** như §7.4 (giá trị qua luồng phải `Send`); (b) `Arc`+`Mutex` toàn cục (đắt, phá tất
định); (c) thread-per-task với `Host` chia sẻ được. Chốt `Send`/`Sync` cho `Value`, `Task`, `Chan`, `Host`.

**Kích thước ước lượng `[ƯL]`.** 5 tệp lõi: `rt.rs` 247 + `interp.rs` 1 247 + `vm.rs` 1 076 + `value.rs` 604 + `builtin.rs`
1 455 = **4 629 dòng** bề mặt ⇒ **4–6 task × ≤3 h**, **+10–20 ca**, đổi **800–1 800 dòng**. Cơ sở: mỗi điểm chạm `Host`/`Value`
lan ~1 call-site/40–60 dòng. **Chưa đo.**

**Rủi ro.** (1) Mất tất định ⇒ đỏ hợp đồng interp↔VM "0 lệch" (`crates/vis-vm/tests/diff_100k.rs:2`, 100k seed `[ĐO]`
lech=0) và L0 **1175** ca `[ĐO]`; (2) chạm **LSP** (nạp module), **soak** 10 vòng, **cổng** — cả ba giả định một luồng;
(3) `Rc`+`RefCell` chia sẻ giữa luồng = UB nếu không `unsafe` ⇒ phải đổi kiểu hoặc xin **ngoại lệ D2-17**.

**Câu hỏi cho Agent 0.** (1) Giữ **heap riêng mỗi luồng** (§7.4) hay chuyển `Arc`/`Mutex` toàn cục? (2) Có cấp **ngoại lệ
`unsafe`** trong `vis-interp`/`vis-vm` cho work-stealing (D2-17 hiện cấm)? (3) `Host: Send + Sync` (§12) là **cam kết** VS3
hay đổi `Host` thành `&self` + khoá nội bộ?

## 2. Kênh: rendezvous `chan(0)`, `select`, timeout

**Hiện trạng.** `rt.rs:54-63` `Chan { cap, buf: VecDeque, closed }` — **không** wait-set, không danh sách người chờ.
`rt.rs:130-136`: `cap == 0` **từ chối lúc chạy** (`DEADLOCK`/`VS0505`); ca `kenh_cap0.rs:57-68` khoá hành vi literal
`chan[int](0)` ⇒ **exit 1**, không treo; `tyck/expr/kenh.rs:16-33` **không** kiểm `cap` ⇒ chú thích `rt.rs:128-129` ("literal
`0` đã bị tyck chặn, exit 2") **lệch mã**. `builtin.rs:1229-1292`: `send`/`recv` khi chặn thì **drain** hàng sẵn sàng
(`chay_mot`) rồi thử lại, hết queue ⇒ `VS0505` (`:1259-1263`, `:1278-1282`); thứ tự đánh thức = FIFO `ready` (`rt.rs:157-158`),
**không** theo kênh. `builtin.rs:1292` chỉ có `send`/`recv`/`close` ⇒ **không** `select`, **không** timeout (nợ #17). Tên
`select` **đã dùng** cho lệnh chiếu cột (`builtin.rs:333-337`, `tyck/expr.rs:648`) ⇒ phải giải xung đột theo thiết kế §9.4.
Nợ đã cấp số: **#16** (close không đánh thức ai), **#17**, **#19/#71** (rendezvous), **#72** (`agent/SO-NO.md` §5).

**Việc cần cho VS3.** Rendezvous thật cho `cap = 0`, **wait-set** trên kênh, `close` **đánh thức** người chờ, `select` nhiều
kênh + `timeout(ms, …)`, và **thứ tự đánh thức tất định** để ca test còn so byte được.

**Kích thước ước lượng `[ƯL]`.** `rt.rs` +80–150, `builtin.rs` (`chan_method`) +60–120, `tyck/expr/kenh.rs` +20–40; nếu
`select` là **cú pháp mới** thì thêm `vis-syntax` + `hir.rs` + `lower/expr.rs` ⇒ **6–10 tệp, 600–1 200 dòng, 2–4 task**.
Cơ sở: mỗi phương thức kênh hiện 18–25 dòng (`builtin.rs:1240-1290`); `select` đi qua **4 tầng**. **Chưa đo.**

**Rủi ro.** (1) Bất định thứ tự đánh thức — khuôn hiện có (`kenh_cap0.rs:51-54`) chạy **cả** interp và VM rồi so byte;
(2) trùng tên `select` với lệnh chiếu cột ⇒ quyết định ngữ pháp là của Agent 0; (3) đổi `VS0505` (exit 1 → 2) phá ca đã khoá.

**Câu hỏi cho Agent 0.** (1) `select` là **cú pháp mới** hay **hàm std** (`chan.select([c1,c2])`)? (2) Rendezvous phải
**tất định** (FIFO theo id tác vụ) hay chấp nhận bất định (khi đó miễn trừ ca so byte)? (3) `chan(0)` giữ exit **1** hay đổi **2**?

## 3. Net: TLS, HTTP/2, keep-alive

**Hiện trạng.** `crates/vis-interp/src/net.rs:8-10` — phạm vi nguyên văn "**chỉ** `http://`, **chỉ loopback**; **không**
TLS/HTTP2, **không** keep-alive"; `:20-24` cap 8 MiB / header 64 KiB; `:27-38` `tach_url` **từ chối** host ≠
`127.0.0.1`/`localhost`. `crates/vis-cli/src/net.rs:2` — `NetHost` CLI là TCP thật **chỉ loopback**, `:119-121` `chi_loopback()`
trước `TcpStream::connect_timeout`. `builtin.rs:186-200` trait `NetHost` 7 method, handle **chuỗi mờ**, body nhị phân;
`quyen.rs:590-604` `NetGuard` kiểm `cap[Net]` **theo chuỗi host**. Cây phụ thuộc: **31 package** (`Cargo.lock`), **không**
crate TLS; crate mã hoá duy nhất `sha2` 0.10 (`crates/vis-bytecode/Cargo.toml:10-11`). Thiết kế: `ViScript-thiet-ke.md:822`
(`net` có "TLS (thuần Rust)"), `:1484` (mức VS2). Luật: QD §9.3 allowlist **MIT/Apache-2.0/BSD-2/3/ISC/0BSD/Zlib/Unicode**,
**cấm MPL/GPL/LGPL** + D2-21 + AGENTS §4.9.

**Việc cần cho VS3 + kiểm allowlist.** (1) **rustls** — Apache-2.0 OR ISC OR MIT (thuộc allowlist) `[GT: xác nhận bằng
cargo metadata]`. (2) Kho CA **`webpki-roots`** — giấy phép **đổi theo phiên bản** (cũ **MPL-2.0**, 1.x
**CDLA-Permissive-2.0**) `[GT]` ⇒ **cả hai KHÔNG có trong allowlist** ⇒ dùng là **phải xin ngoại lệ QD §9.3**, hoặc lấy CA
hệ thống. (3) **ring/aws-lc-rs** — chứa mã gốc OpenSSL nên giấy phép **tổng hợp** (ISC/MIT/Apache **+ OpenSSL**) `[GT]` ⇒
**rủi ro allowlist cao nhất** của mục này. (4) Build: **+15–30 package** `[ƯL]`, +30–60 s `[ƯL]`; ring cần **C toolchain**
(psvn đã có VS Build Tools, VD-140), một số nền cần clang/perl `[GT]`.

**Kích thước ước lượng `[ƯL]`.** TLS 1 task (~300–600 dòng, 8–12 ca) · keep-alive 1 task (~200–400 dòng: đổi vòng đời
`tay`) · HTTP/2 2–3 task (800–1 500 dòng: khung + HPACK + luồng) ⇒ **4–5 task, 7–10 tệp**. **Chưa đo.**

**Rủi ro.** (1) **Giấy phép** — cổng cứng, không có "tạm thời"; (2) **chứng chỉ hệ thống** (Keychain / cert store ≠
webpki-roots) ⇒ hành vi khác nhau giữa 3 nền; (3) **SNI/proxy/redirect**: `cap[Net]` so khớp host chuỗi (`quyen.rs:596`),
TLS bắt tay theo host sau redirect ⇒ **lỗ hổng cấp quyền**; (4) `--sandbox`/Landlock **không** chặn mạng.

**Câu hỏi cho Agent 0.** (1) Kho CA: `webpki-roots` (**xin ngoại lệ giấy phép**) hay CA hệ thống (thêm `unsafe`/crate theo nền)?
(2) TLS là **điều kiện** của VS3 hay chỉ `https://` trên HTTP/1.1, **hoãn HTTP/2** sang VS3b? (3) `cap[Net]` so khớp host
**trước** hay **sau** redirect?

## 4. Quyền: đường GHI nguyên tử (`openat`/`O_NOFOLLOW`), macOS fd→path

**Hiện trạng.** `crates/vis-cli/src/quyen.rs:568-570` — **đường GHI vẫn kiểm-rồi-mở**: `kiem_fs(p, true)?` rồi
`self.trong.write(p, s)` ⇒ giữa hai bước tên có thể bị đổi; đường ĐỌC đã "mở-rồi-kiểm" (`quyen.rs:552-566`, `:492-510`),
đóng ở ST-137 `[ĐO]` 0/50 000 trên Linux. `quyen.rs:465-490` `xac_nhan_sau_mo` là **hai syscall trên tên** (giới hạn đã biết);
`:399-410` `kiem_fs_canon`. `builtin.rs:144-184` `FsHost` định danh bằng **`&str` đường dẫn** (`read(p)`, `write(p,s)`,
`read_theo_handle(p)`) ⇒ **không** có API handle/thư mục ⇒ `openat` **không thể** làm nếu không đổi trait. `ngoai.rs:268-278`
Linux lấy đường dẫn thật qua `/proc/self/fd/<fd>`; `:280-292` macOS **`KhongCo`** (chú thích: `fcntl(F_GETPATH)` cần
`libc`/`unsafe`, **ngoài VS2**). `agent/HOST-BAO-DAM.md:102-106`: Linux bảo đảm theo cơ chế, macOS **giới hạn văn bản**
(VD-145), đường GHI "sửa triệt để cần `openat` + `O_NOFOLLOW`… việc VS3", hardlink cũng là giới hạn.
`libc` **đã có** trong `Cargo.lock` (0.2.191) nhưng **chỉ bắc cầu** qua `cpufeatures` — **0** dòng `libc::` trong mã ta;
**8** `impl FsHost` (`main.rs:74`, `ngoai.rs:299`, `fsmock.rs:148`, `quyen.rs:551,684,802,870,924`). Nợ: **#109b/#140**
(đường GHI), **#137** (macOS). **Lệch số cần đính chính**: hiến chương §5 ghi "`#142/#109b`" nhưng `#142` = "`pkg update`
ghim mọi gói khác" (VD-149) ⇒ nhiều khả năng muốn nói **`#140`** (câu hỏi §8).

**Việc cần cho VS3.** (a) Đổi cách định danh: thêm API **handle** (`dir_open`/`openat`) vào `FsHost`, hoặc thêm
`write_theo_handle` trả `(dev, ino, đường dẫn thật)` như đường đọc; (b) `O_NOFOLLOW`/`RESOLVE_NO_SYMLINKS`; (c) macOS fd→path
cho **cả** đọc và ghi; (d) chốt `libc` (đã trong lock) vs `rustix`/`cap-std` (crate **mới**).

**Kích thước ước lượng `[ƯL]`.** **5–8 tệp** (`builtin.rs`, `quyen.rs`, `ngoai.rs`, `fsmock.rs` + 3 host giả trong
`quyen.rs`), **400–800 dòng, 2–3 task**, 1 ca **đỏ-trước** kiểu đua (khuôn `quyen::tests::dua_doi_symlink_lap_co_gioi_han`,
50 000 vòng `[ĐO]`). Cơ sở: 8 impl `FsHost` sửa cùng lúc, mỗi impl 10–40 dòng. **Chưa đo.**

**Rủi ro.** (1) `unsafe`/crate mới là cổng D2-17 + QD §9.3 — cần văn bản **trước** dòng mã đầu tiên; (2) đổi `FsHost` chạm
**mọi** host (mock, wasm, windows) và **54/54** ca hợp chuẩn `[ĐO]`; (3) macOS/Windows không có `/proc` ⇒ 3 nền **3 cơ chế**;
(4) Landlock chỉ có trên Linux ⇒ ca macOS **không** được dựa vào nó.

**Câu hỏi cho Agent 0.** (1) `libc` (đã trong lock, 0 dòng dùng) + `unsafe` tối thiểu, hay `rustix`/`cap-std` (crate mới)?
(2) Được **đổi API `FsHost`** (thêm method bắt buộc) hay chỉ thêm method **default** để 8 impl cũ vẫn biên dịch?
(3) macOS fd→path làm trong VS3 hay VS4 — và nếu làm, đường GHI macOS có được miễn ca đua không?

## 5. Windows: "hỗ trợ thật"

**Hiện trạng.** Hợp chuẩn host **39/54** `[ĐO]` (`JOURNAL/st144-host-suite.log:18-32`, sha256 `f48f340a…`), **15 ca `not ok`**
chia **2 nhóm**: (a) **9 ca** `ngoai_*` gọi chương trình POSIX tuyệt đối — `/bin/echo`, `/bin/cat`, `/usr/bin/grep`,
`/usr/bin/printf`, `/usr/bin/false` (`test/host/suite.txt:38-50`); (b) **6 ca** đường dẫn tuyệt đối `{tmp}` (`suite.txt:28`
và `:62-66`: `cap_trong/cap_ngoai/cap_thieu_write/cap_grant/cap_khong_khai`). **2 ca "xanh" không chứng minh gì**: `suite.txt:45`
(`/bin/sh -c "exit 3"`) và `:46` (`/khong/co/lenh`) **kỳ vọng lỗi** (exit 1 + `VS0501`) ⇒ xanh vì chương trình **không tồn tại**
`[GT]`. `cargo test` **1053/100** `[ĐO]` (`st144-cargo-test.log`); L0 fail-fast **150/4**, dừng ở suite `cap_proc` vì `/bin/echo`
(`R26 §3.3`; ca nguồn `crates/vis-cli/tests/cap_proc.rs:62,70,96,112,130`); cổng **9/10 bước** rồi dừng **L0** `[ĐO]`. Đã đóng
bằng hạ tầng: self-test **12/12** (VD-150) và ca 7 (VD-152, `check.rs` tách tên gốc theo cả `\` và `/`). **Không có CI**: không
`.github/`, không `.yml`; runner là **SSH tới `ps`/`psvn`** (`infra/windows/setup.md` 82 dòng, `measure.sh` 88 dòng,
`setup.ps1` 126 dòng, VD-136). Cổng và mọi script là **bash** (`bin/vis`, `bin/vis-dev`: `#!/usr/bin/env bash`) ⇒ Windows
**bắt buộc Git Bash** (log psvn: `MINGW64_NT-10.0-26300`).

**Việc cần cho VS3.** Chốt **định nghĩa "hỗ trợ thật"**, rồi: (a) lớp đường dẫn (`/`↔`\`, ổ đĩa, UNC, `\\?\`); (b) **bộ chương
trình POSIX** (đóng gói kèm / shim lệnh native / bắt buộc Git Bash); (c) CRLF (đã chuẩn hoá ở `pkg.rs:5`, chưa áp cho `fs`);
(d) tiến trình (`cmd`/`cmd //c`, `| complete`, job control); (e) quyền (ACL ≠ POSIX, `(write)` không ánh xạ thẳng).

**Kích thước ước lượng `[ƯL]`.** **3–5 task**: (a) lớp đường dẫn + 15 ca host — 1–2 task, 300–600 dòng; (b) bộ POSIX — 1 task,
200–400 dòng (+quyết định kích thước gói); (c) ~100 ca `cargo test` — 2 task, 400–800 dòng. **Tổng 9–15 h.** Cơ sở: 15 ca host
và 100 ca cargo test **đã có tên** trong log, mỗi ca ước 3–8 dòng. **Chưa đo.**

**Rủi ro.** (1) Sửa ca = chạm AGENTS §4.2/§4.3 (cấm sửa kỳ vọng, cấm ignore) ⇒ phải có **VD mở đường** kiểu VD-132 (mới nới
**2** ca); (2) không CI ⇒ số Windows phụ thuộc máy psvn + SSH, flake = `T-INFRA`, **không** retry (D2-07); (3) "shim bằng Git
Bash" rất dễ bị đọc thành "hỗ trợ thật" ⇒ phải định nghĩa bằng **ca đo được**; (4) đóng gói GNU coreutils = **GPL**, trái QD §9.3.

**Câu hỏi cho Agent 0.** (1) "Hỗ trợ thật" = (a) đóng gói bộ POSIX riêng, (b) shim sang lệnh native, hay (c) chỉ sửa đường
dẫn/CRLF và **giữ** yêu cầu Git Bash? (2) Có được **sửa kỳ vọng** 15 ca host + ~100 ca `cargo test` theo nền (mở rộng VD-132)?
(3) Runner: chấp nhận **SSH psvn** hay bắt buộc CI thật trước khi tuyên bố hỗ trợ?

## 6. Gói: registry mạng + chữ ký + GC store

**Hiện trạng.** `crates/vis-cli/src/pkg.rs:2` — "registry **local** (không mạng), cài vào `.vis/pkg/<tên>/<ver>/`"; `:7-8`
`install` **chỉ** đọc `vis.lock` + kho blob `.vis/store/<sha256>` ("không cần registry"); `:860` `thu_muc_registry` = **thư mục**;
`:914` `install` không mở registry; `:953` đọc blob. **Chữ ký: không có** — VD-125 nguyên văn "Không mạng, không chữ ký (VS3)";
băm dùng `sha2` qua `vis_bytecode::source_hash` (`pkg.rs:10-12`, `vis-bytecode/Cargo.toml:10-11`). **GC store** chỉ có nhánh
**thủ công**: `pkg.rs:1119` ".vis/store chỉ xoá khi **không** gói còn lại nào dùng", `:1145` `remove_file` trong `lenh_remove`;
không mark-sweep, không ai đo kích thước store. **#142** `pkg.rs:1008-1020` (`pkg update <tên>` ghim mọi gói khác `=<version>`);
**#143** `NamePolicy::Strict` (`crates/vis-types/src/lower/options.rs:10,60`) vẫn opt-in; **#118** (registry mạng + chữ ký),
**#119** (lock không ghi quan hệ phụ thuộc) — `agent/SO-NO.md` §5. Kích thước mã: `pkg.rs` 1 198 + `pkg/resolver.rs` 708 +
`pkg/semver.rs` 631 = **2 537 dòng** `[ĐO]`.

**Việc cần cho VS3.** (a) Giao thức registry (index JSON + blob) trên `std.http` hiện có; (b) **chữ ký số** (khoá tin cậy +
manifest ký) — Ed25519 (crate mới) hay `sha256`+HMAC (không thêm crate nhưng **không phải** chữ ký công khai); (c) **GC store**
mark-sweep theo `vis.lock`; (d) **#142** chính sách update không ghim; (e) **#143** Strict mặc định cho tệp có `use`.

**Kích thước ước lượng `[ƯL]`.** registry 1–2 task (400–800 dòng + server giả loopback) · chữ ký 1 task (200–400 dòng,
8–12 ca) · GC 0,5 · #142 0,5–1 · #143 0,5–1 ⇒ **4–5 task, 6–9 tệp, 900–1 600 dòng**. Cơ sở: 2 537 dòng hiện có cho **toàn bộ**
pkg ⇒ thêm ~40–60 %. **Chưa đo.**

**Rủi ro.** (1) Chữ ký cần **crate mới** (`ed25519-dalek` thường kéo `curve25519-dalek`/`rand_core`/`zeroize` `[GT]`) ⇒ allowlist
+ cây phụ thuộc; (2) registry **qua mạng** phá bất biến VS2 "pkg không mạng" và cần `cap[Net]` — không có TLS (mục 3) thì
**không** được ra Internet; (3) GC xoá nhầm blob dùng chung giữa hai lock; (4) **#142** đổi chính sách ⇒ đổi `vis.lock`
(VD-125/VD-129 đã đóng băng hành vi); (5) **#143** bật Strict mặc định sẽ đỏ **toàn bộ** fixture + 54 ca host.

**Câu hỏi cho Agent 0.** (1) Chữ ký: Ed25519 (crate mới) hay `sha256`+HMAC tự viết? (2) Registry **bắt buộc** TLS (phụ thuộc
mục 3) hay chấp nhận "registry nội bộ loopback" như VS2? (3) `vis.lock` mở **định dạng 3** cho #119/#142 hay giữ định dạng 2?

## 7. Bảng tổng hợp

| # | Mục | Kích thước `[ƯL]` | Rủi ro chính | Phụ thuộc |
|---|---|---|---|---|
| 1 | Async song song thật | 4–6 task · 800–1 800 dòng · 5 tệp lõi (4 629 dòng bề mặt) | mất tất định (diff 100k, L0 1175); `Rc`/`RefCell` `!Send`; cần ngoại lệ `unsafe` (D2-17) | **chặn 2, 3, 4** (mô hình bộ nhớ + `Host: Send + Sync`) |
| 2 | Kênh: rendezvous/select/timeout | 2–4 task · 600–1 200 dòng · 6–10 tệp | bất định thứ tự đánh thức; trùng tên lệnh `select`; đổi `VS0505` phá ca | 1 (wait-set + scheduler) |
| 3 | Net: TLS/HTTP2/keep-alive | 4–5 task · 1 300–2 500 dòng · 7–10 tệp + **+15–30 package** | **allowlist giấy phép** (webpki-roots/ring); CA hệ thống; `cap[Net]` + SNI | 1, 4 (đọc tệp CA), 6 (registry cần TLS) |
| 4 | Quyền ghi `openat`/`O_NOFOLLOW`; macOS fd→path | 2–3 task · 400–800 dòng · 5–8 tệp (8 impl `FsHost`) | cần `unsafe`/crate mới; chạm 54/54 ca; 3 nền 3 cơ chế | 1 (nếu `Host` đổi `&self`) |
| 5 | Windows "hỗ trợ thật" | 3–5 task · 900–1 800 dòng · 15 ca host + ~100 ca cargo test | phải sửa kỳ vọng test (AGENTS §4.2); **không CI**; POSIX = GPL | 1, 4 |
| 6 | Gói: registry + chữ ký + GC | 4–5 task · 900–1 600 dòng · 6–9 tệp (nền 2 537 dòng) | crate chữ ký mới; định dạng lock; Strict mặc định đỏ 54 ca | 3 (TLS) |

**Tổng `[ƯL]`: 19–28 task × ≤3 h ≈ 57–84 h** (≥ 6–9 phiên 3 h cho cả VS3). Hiến chương §5 đòi thiết kế **trước từng mục**
⇒ đề nghị Agent 0 ra **6 quyết định riêng**, không một tài liệu gộp.

## 8. Top 5 câu hỏi Agent 0 phải trả lời trước khi viết thiết kế

1. **Mô hình bộ nhớ/GC** của VS3 (heap riêng mỗi luồng như §7.4 vs `Arc`/`Mutex`; có GC chu trình?) — quyết định `Send`/`Sync`
   của `Value`/`Host`/`Task`/`Chan` và **chặn cả 6 mục**.
2. **Chính sách crate**: có mở **ngoại lệ giấy phép** (webpki-roots MPL/CDLA, ring OpenSSL) và **ngoại lệ `unsafe`** (D2-17) cho
   `vis-interp`/`vis-vm`/`vis-cli`? Trả lời "không" thì mục 1, 3, 4 **phải** đổi thiết kế.
3. **Windows "hỗ trợ thật"** nghĩa là gì, và có được **sửa kỳ vọng** 15 ca host + ~100 ca `cargo test` (mở rộng VD-132)?
4. **`select` trên kênh**: cú pháp mới hay hàm std (tên `select` đã là lệnh chiếu cột, `builtin.rs:333-337`)?
5. **Số nợ của đường GHI**: hiến chương §5 ghi "`#142/#109b`" nhưng `#142` = `pkg update` ghim (VD-149) — xác nhận **`#140`**
   thay `#142`, và chốt ma trận phụ thuộc giữa 6 mục (1 → 2/3/4; 3 → 6).

## 9. Đề nghị **KHÔNG** làm trong VS3

1. **HTTP/2** — TLS + keep-alive đã đủ giá trị; HTTP/2 (khung + HPACK) là 2–3 task độc lập, tách **VS3b**.
2. **Siết caps mặc định toàn cục** (#33/#43) — đỏ toàn bộ fixture + 54 ca host; giữ opt-in `--strict-caps` như VS2.
3. **Runtime wasm** (VD-121) — VS2 chỉ "biên dịch kiểm tra"; đưa vào VS3 làm loãng ngân sách.
4. **pty/tty server** (thiết kế §12 "chưa có ở bản đầu") — không thuộc 6 mục hiến chương.
5. **Registry trần HTTP ra Internet** — chưa có TLS (mục 3) thì chỉ cho registry **nội bộ/loopback**.
6. **Đóng gói GNU coreutils** cho Windows — **GPL**, trái QD §9.3.

---

*ST-155 — chỉ đọc. Không dòng mã nào bị sửa; mọi số `[ĐO]` dẫn log trong `agent/JOURNAL/`, mọi số `[ƯL]` ghi kèm cơ sở.*
