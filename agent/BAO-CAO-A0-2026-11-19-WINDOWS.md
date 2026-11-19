# BAO-CAO-A0 — Windows/psvn: kết quả `349af67` và câu hỏi quyết định về L0 test

Gửi A0 · từ ps (Agent đang trực) · 2026-11-19 · **A0 ĐÃ RA QUYẾT ĐỊNH (2026-11-19) — xem addendum cuối tệp**
Phạm vi: **chỉ máy Windows (psvn)** — không đo, không đóng bất cứ mục nào về mac (#147/#152 vẫn chờ mps).

---

## 1. Bối cảnh & câu hỏi cần A0 quyết

Sau khi A0 duyệt hướng Windows (fix psvn guard cho pha 1), chúng tôi đã sửa các lỗi mã thật do
psvn bắt được, đưa `main` lên `ba11a90` rồi `349af67`. Lần chạy lại **toàn bộ** suite + gate trên
Windows tại `349af67` cho kết quả: **sản phẩm xanh hoàn toàn; L0 test đỏ ở 101 ca / 25 suite —
toàn bộ thuộc các lớp portability của bộ test (tiền tồn, VS3), không có hồi quy mã sản phẩm.**

**A0 ĐÃ RA QUYẾT ĐỊNH (2026-11-19)**: **Phương án C** — dừng tại `349af67`; Windows xếp **Tier 2 — product verified**; không port 101 ca L0 trong đợt đóng pha 1; mở VD riêng "Windows test-suite portability" sau khi pha 1 đóng chính thức (base `349af67`). Xem addendum §6 (cuối tệp) — các mục §4–§5 dưới đây được giữ làm lịch sử của bản đề xuất.

---

## 2. Việc đã làm trong đợt này

### 2.1 Ba commit trên nhánh rev, fast-forward lên `main` (push đầy đủ)

| Commit | Nội dung |
|---|---|
| `ba11a90` | A1–A5: span-CRLF (3 tệp), `doc_examples.py newline="\n"`, bareword Str escape, `la_tep_thuc_thi` non-unix → `la_tep_lenh`, LSP URI `file:///c:/...`; cache `PHIEN_BAN` re-pin |
| `349af67` | Helper URI test **bỏ `#[cfg(windows)]`** → dùng `std::env::consts::OS` (code type-check giống nhau trên mọi nền; tránh lỗi compile chỉ xuất hiện trên Windows — psvn bắt được) |

Cả hai đã fast-forward `main` (origin/main = `349af67`), push xong. `pha1-rc1` nguyên vẹn tại
`05e42d01483d255987750656fa6cba6c434831c9`.

### 2.2 Truy cập psvn từ ps (thay đổi hạ tầng duy nhất, đã được phép)

- ps chưa từng có private key để ssh psvn → tạo keypair **mới** trên ps: `~/.ssh/psvn_ed25519`
  (fingerprint `SHA256:5mrepWzhBggpvxhk7cHsAn8AW77PSLdT/Gkvkrl9DaI`), private key **không rời máy ps**.
- `ps` là **Administrator** trên psvn ⇒ public key phải vào `C:\ProgramData\ssh\administrators_authorized_keys`
  (Windows OpenSSH bỏ qua `%USERPROFILE%\.ssh\authorized_keys` với tài khoản admin).
- Dùng relay `ps → root@mps (Tailscale SSH) → sudo -u mps ssh psvn` để: backup file cũ về
  `/c/Users/ps/vislogs/administrators_authorized_keys.bak-2026-11-19`, **append** 1 dòng public key
  của ps (4 → 5 dòng). ACL giữ nguyên (`NT AUTHORITY\SYSTEM:(F)` + `BUILTIN\Administrators:(F)`), không xóa, không sửa gì khác.
- `~/.ssh/config` trên ps: `Host psvn → user ps + IdentityFile psvn_ed25519`.
  Từ đó ps ssh/vis-remote psvn trực tiếp (không cần relay).
- mps không bị đụng chạm (chỉ đọc; cây repo mps vẫn ở `6fb72a7`).

---

## 3. Kết quả đo trên Windows @ `349af67`

**Môi trường (psvn)**: Windows 11 build 26300, x86_64, 6 lõi; Git Bash 5.2.37;
cargo/rustc **1.100.0** (vis-dev tự chọn `/c/Users/ps/.cargo/bin/cargo`, yêu cầu ≥ 1.99.1 ✓);
cây clone `/c/Users/ps/viscript-moi`, `core.autocrlf=false`, **dirty = 0**; `LC_ALL=C.UTF-8`.

### 3.1 Những bước **PASS** (bằng chứng đầy đủ trong log)

| Bước | Kết quả |
|---|---|
| fmt | OK |
| clippy (`-D warnings`) | **OK** |
| luật R1..R6 | OK |
| self-test âm (13 ca + fixture PASS) | OK |
| vis explain | OK |
| report-check | **PASS** (30 báo cáo, 243 hàng QUEUE phủ đủ) |
| build | OK |
| doc-examples | **OK** (24 khối: 19 xanh + 5 ngoại lệ đỏ đúng; 12 fixture QD §7.1..7.3) |
| `vis-dev check` tổng | **PASS** |
| `vis test test/sem` (interp) | **483 / 483 ✓** (4 ca đỏ duy nhất = `vish/ngoai-{echo,printf,raw,true}`, VS3) |
| `vis test --vm test/sem` | **483 / 483 ✓** (cùng 4 ca) |
| `cargo test -p vis-cli --test crlf` | **3/3 PASS** (import std.json, record thiếu field, test fn→main) |
| LSP qua `file:///c:/...` | URI được chấp nhận; module-aware `publishDiagnostics` công bố đúng (1 frame/1 tệp) |

### 3.2 Những bước **L0 test FAIL** — 25 suite, 101 ca (90 tên test duy nhất)

`cargo test --workspace --no-fail-fast` — toàn bộ các ca đỏ rơi vào **bốn lớp portability tiền tồn**
của bộ test (đã có trong báo cáo psvn lần đầu ở HEAD `13c31cf`, khi đó 104 ca):

| Lớp | Số ca (ước lượng) | Bản chất | Ví dụ tiêu biểu |
|---|---|---|---|
| **A. POSIX binaries** (`/bin/echo`, `/bin/sh`, `/usr/bin/grep`, `printf`, `cat`, `true`, `false`) — VS0501 `os error 3`; và `bash` trúng WSL stub (`execvpe`) | ~44 | Test hermetic POSIX (VD-146), không có tương đương trên Windows | `cap_proc.*` (4), `cap_proc_grant.*` (7), `extern.*`, `pipeline.*`, `vis_test.*` (3), `sem` (4), `ngoai_vm`, `vish_main.main_rong_va_pipeline_hop_le`, `soak_script`, `sync_list.*` (4 — WSL stub) |
| **B. Windows path/CRLF/JSON trong fixture & expectation** — đường dẫn thô `C:\...` nhúng vào literal `.vis` → VS0102/VS0201; JSON output đúng escape (`\\`) khác với chuỗi kỳ vọng single-`\`; so sánh chuỗi cần normalize `\r\n` | ~45 | Test-suite tự tạo fixture/mong đợi trên Linux; chưa từng chạy trên Windows | `json_ra.check_json_*` (7), `span_tep.json_bao…`, `chan_doan_da_tep.loi_*` / `b_dai_van_dung_a`, `quyen.*` (8), `std_fs_json.*` (7), `strict_caps.strict_cap_dir_*` (2), `vm_main_argv.argv_dau_cap_sau`, `host_conformance_suite` case 13 |
| **D. WinSock/loopback** — kết nối 127.0.0.1:port đóng port ⇒ Windows drop SYN ⇒ "quá thời gian chờ nối tới" thay vì refused ngay | 4–5 | Hành vi nền tảng, không phải lỗi sản phẩm | `net.tcp_tu_choi`, `net_server.server_mot_lan`, `server_hai_tien_trinh`, `server_hai_ket_noi_lien_tiep` |
| **E. Unix file mode** — `long` hiển thị `mode -` trên Windows (không có bit x) | 1 | Kỳ vọng unix | `vsh1b.long_tren_thu_muc_that` |

> Ghi chú đếm: 101 = tổng `failed` ghi trong từng khối `test result: FAILED` (25 suite); 90 = các tên
> test duy nhất (một số tên lặp ở hai engine). Log đầy đủ: `/c/Users/ps/vislogs/windows-349af67-ws.log`,
> verify stream: `/tmp/opencode/psvn-windows-verify-stream.log` (máy ps), gate: `/c/Users/ps/vislogs/windows-349af67-gate.log`.

### 3.3 Những gì **đã xanh thật** (các lớp fix trước đó)

- **A1 span trên văn bản chuẩn hoá CRLF→LF**: `use std.json` / `use std.list` không còn bị cắt tên module
  (nguyên nhân của 74 ca sem đỏ + toàn bộ đường `use` trên Windows) → **không còn ca nào đỏ vì span-CRLF**.
- **A2 `doc_examples.py newline="\n"`** → `doc-examples` PASS (trước đó fail `VS0705` do fixture CRLF).
- **A3 bareword `\` escape** → fixture có `C:\...` không còn lỗi "escape không hợp lệ ở block lệnh"
  khi tên thư mục/duỗi trong .vish (thông qua `chuyen_doi_text`).
- **A4 `la_tep_thuc_thi` non-unix → `is_file`** → `cap[Dir]` không âm thầm nhận tệp thực thi nữa.
- **A5 LSP URI** → `file:///c:/...` được chấp nhận, module-aware diagnostics đầy đủ.
- **Bổ sung**: `#[cfg(windows)]` dead code trong test helper URI (bị E0599 trên Windows) đã bị gỡ
  bằng cách viết helper không dùng cfg — `cargo check --workspace --tests --target x86_64-pc-windows-msvc` = 0 lỗi.

**Kết luận được A0 điều chỉnh** (nguyên văn): *"Không phát hiện hồi quy mã sản phẩm trong các lớp đã được kiểm chứng thành công; 101 ca L0 còn đỏ đã được phân loại sơ bộ là vấn đề portability của test suite hoặc khác biệt nền tảng, nhưng chưa được port và xác nhận độc lập từng ca."*

---

## 4. Khuyến nghị của Agent (cho A0): phương án C — **A0 đã chọn C (2026-11-19)**

Lý do, theo thứ tự:

1. **Pha 1 giữ nguyên**: không thêm một dòng diff nào ngoài `ba11a90`/`349af67` vào đường merge.
   Port ~101 test là công work lớn (4 lớp, mỗi lớp một cách xử lý khác nhau), không được phép
   trộn vào đợt đóng pha 1 / VD-241.
2. **Lớp B không thể sửa "cho xong" một cách máy móc**: muốn xanh phải normalize CRLF khi so chuỗi
   hoặc sinh expectation theo nền tảng — chuẩn hoá sai có thể **che giấu lỗi CRLF thật của sản phẩm**
   (chính lớp lỗi đã sửa ở A1). Mỗi expectation phải được stem/viết lại theo hợp đồng nền tảng.
3. **Lớp A**: lựa chọn `#[cfg(unix)]` hay port sang lệnh tương đương Windows là **quyết định chính sách**
   (POSIX-only tests được phép biến mất khỏi Windows L0? hay phải có Windows equivalents?) — A0/owner quyết,
   không tự ý rút bớt test coverage.
4. **D và E** tương tự: WinSock drop-SYN và unix mode là ngữ nghĩa nền tảng — cần ratify cách kỳ vọng.
5. Giá trị của việc dừng: bằng chứng **sản phẩm** trên Windows đã đủ (sem 483/483 ×2, doc-examples xanh,
   gate các bước trước L0 xanh), nội dung quyết định còn lại hoàn toàn là phạm vi/chính sách test.

### 4.1 Nếu A0 chọn A hoặc B — scope đề xuất

- VD riêng: **"Windows test-suite portability"**, base `349af67`.
- Lớp B (~45 ca): sinh/kiểm expectation theo nền tảng (path JSON-escape; `\r\n` normalize có ghi lý do;
  fixture path dùng forward-slash hoặc literal `p"..."`); review chéo Agent 2 bắt buộc.
- Lớp A (~44 ca): `#[cfg(unix)]` hoặc tương đương native; liệt kê từng test; ghi rõ trong CHEATSHEET.
- Lớp D (5): kỳ vọng theo nền tảng (refused vs timeout) hoặc cfg gate; lớp E (1): cfg(unix).
- Cổng đóng: gate PASS trên **cả ps và psvn**; review chéo; không đổi ngữ nghĩa sản phẩm.
- Ước lượng: 1 buổi tập trung + review (không chớm tiến độ #152/#147/B-3).

---

## 5. Câu hỏi cho A0 — **đã được A0 trả lời (2026-11-19); xem addendum §6**

1. **Chọn C (khuyến nghị), B, hay A?** Nếu B/A: mở VD "Windows test-suite portability" ngay sau đóng pha 1?
2. **Chính sách** cho lớp A/D/E: `#[cfg(unix)]` (bỏ khỏi Windows L0) hay tương đương native? Có trần coverage không?
3. **Windows tier**: tier-2 "product verified" (ghi trong ROADMAP/HẠNG-MỤC) hay tier-1 "gate L0 xanh"?
4. **CI**: có thêm Windows vào CI/nightly sau VD-port (nếu có) không, hay vẫn thủ công qua psvn?

---

## 6. Trạng thái & ràng buộc còn hiệu lực (không đổi)

- `pha1-rc1` = `05e42d0…` — nguyên vẹn.
- **#147-mac / #152-vế-mac**: vẫn chờ số đo trên máy rảnh (mps @ HEAD mới + và ps cùng HEAD đo
  `--gate --fast`); không dùng số Linux để kết luận. A0-3 (lambda-annotation) còn ratify cho #155.
- **Phase B-3** (impl xuyên module + bound generic enforce): chờ ratify riêng; chưa bắt đầu.
- **Q-OPTION-1/2** (Option-long `a–f`): vẫn treo chờ A0 chốt.
- VM `MakeSome`: dead code, không được emit — giữ opcode frozen, không dùng đến.
- Windows access: ps ↔ psvn đã thông (key mới ở `administrators_authorized_keys`, backup ở vislogs);
  revert được bằng cách xóa 1 dòng `ps-host-to-psvn`.

---

## Phụ lục A — Tái lập trên psvn

```bash
ssh psvn                       # user ps (ps có key ~/.ssh/psvn_ed25519; config ~/.ssh/config)
cd /c/Users/ps/viscript-moi && git fetch && git checkout main && git pull --ff-only
git status --porcelain          # kỳ vọng: trống
LC_ALL=C.UTF-8 cargo test --workspace --no-fail-fast   # ~25 suite đỏ: 101 ca (VS3 classes)
LC_ALL=C.UTF-8 ./vis-dev gate --fast                   # fmt..doc-examples PASS; dừng ở L0
./target/debug/vis test test/sem; ./target/debug/vis test --vm test/sem   # 483/483 ×2
cargo test -p vis-cli --test crlf                      # 3/3
```

Log: `/c/Users/ps/vislogs/windows-349af67-{ws,gate}.log` · verify stream: `/tmp/opencode/psvn-windows-verify-stream.log` ·
backup authorized_keys: `/c/Users/ps/vislogs/administrators_authorized_keys.bak-2026-11-19`.

## Phụ lục B — Danh sách 25 suite đỏ (101 ca)

`cap_proc, cap_proc_grant, chan_doan_da_tep, diag_cmp, extern, grant_proc_canon, host_conformance,
json_ra, net, net_server, ngoai_vm, pipeline, quyen, sem (4 vish/ngoai), siet_caps_pkg, soak_script,
span_tep, st83_no_nho, std_fs_json (7), strict_caps (2), sync_list (4), vis_test (3), vish_main (1),
vm_main_argv (1), vsh1b (1)`

Chi tiết 90 tên test: `grep -E '^---- [a-z0-9_]+ stdout ----' windows-349af67-ws.log`.

## Phụ lục C — So sánh lớp trước/sau fix

| Lớp (HEAD 13c31cf, báo cáo đầu) | Sau ba11a90 + 349af67 |
|---|---|
| CRLF/span cắt tên module (`std.jso`, `ist\nuse`) | **Đã hết** (A1) |
| `doc-examples` fixture CRLF → VS0705 | **Đã hết** (A2) |
| bareword path `\t` cắn literal | **Đã hết** (A3) |
| `cap[Dir]` nhận tệp thực thi im lặng | **Đã hết** (A4) |
| LSP mất `file:///c:/...` | **Đã hết** (A5) |
| Test helper `#[cfg(windows)]` không type-check | **Đã hết** (349af67) |
| POSIX binaries / WSL stub / WinSock / unix mode | **Còn (lớp A/D/E) — tiền tồn** |
| Đường dẫn Windows thô/JSON escape/CRLF trong fixture | **Còn (lớp B) — tiền tồn** |

## Phụ lục D — Tiền sử

- Báo cáo Windows đầu: Agent psvn tại HEAD `13c31cf` (104 ca đỏ; khoanh 5 lớp lỗi mã A1–A5, phần còn lại là VS3).
- Quy trình đóng lệnh `use std.json` bị CRLF: `crates/vis-cli/src/modules.rs` (span slice trên văn bản chuẩn hoá),
  `quyen.rs`, `vistest.rs`, dùng `vis_syntax::lex::normalize_source` thống nhất.
- Không có thay đổi ngữ nghĩa ngôn ngữ, không đổi mã lỗi, không động pha 2.


---

## 6. Addendum — Quyết định của A0 (2026-11-19) & hồi phục chi tiết

> Nguồn: lệnh của A0 "Quyết định A0 về Windows tại 349af67". Bản đầy đủ ghi ở `agent/DECISIONS.md`
> (block "Quyết định Agent 0 · 2026-11-19 (Windows @ 349af67)").

**Chốt Tier 2 @ `349af67`**: Windows = product verified. Được: compiler, interpreter, VM,
module loading, CRLF handling, tài liệu thực thi, LSP cơ bản — kiểm chứng trên máy thật.
Không được: tuyên bố workspace/gate PASS trên Windows; Windows không phải cổng bắt buộc L0.
**Không** ghi "full Windows gate PASS" / "Windows fully supported".

**`pha1-rc1` giữ nguyên** @ `05e42d0…`. Không port 101 ca trong đợt đóng pha 1.

**Sau khi pha 1 đóng chính thức**: mở VD riêng **"Windows test-suite portability"**, base `349af67`.
Chính sách A0 (tóm tắt nghĩa vụ):

1. **Lớp A (POSIX binaries)** — không `#[cfg(unix)]` hàng loạt:
   - (A1) test process/pipeline/capability đa nền tảng ⇒ tương đương **native**, ưu tiên **fixture executable do workspace build** (không ánh xạ `/bin/* → cmd/PowerShell/findstr`); phải có coverage Windows cho mọi API process đa nền tảng: ca thành công, stdout/stderr, exit code ≠ 0, grant/deny capability.
   - (A2) test Unix-integration thật (`/bin/sh`, quoting/executable bit/signal/tiện ích POSIX) ⇒ `#[cfg(unix)]` được phép **nhưng** phải ghi rõ là Unix integration, không tính vào coverage đa nền tảng.
   - (A3) test thiếu-chương-trình-dùng dụ error ⇒ fixture executable không tồn tại / đường dẫn sinh trong thư mục tạm; không dùng `/bin/...`.
2. **Lớp B (path/JSON/CRLF)** — normalize **theo hợp đồng từng tầng**: fixture dùng path literal chính thức ViScript hoặc helper escape dùng chung duy nhất; JSON assertion parse thành dữ liệu cấu trúc, không sửa expected để khớp  hiển thị; CRLF chỉ normalize khi hợp đồng là dòng logic (console/diagnostic), không normalize byte-accuracy tests (span/lexer/serializer/round-trip); helper normalization có tên mục đích (`normalize_console_newlines()`), không helper mơ hồ.
3. **Lớp D (WinSock)** — chấp nhận **lỗi semantic tương đương** (refused/reset/timeout) với **timeout hữu hạn**; không khóa nguyên văn thông báo hệ điều hành. Test server tự dựng: listener **port động** + **đồng bộ server-ready** + **shutdown có timeout**; `server_mot_lan`/`server_hai_tien_trinh`/`server_hai_ket_noi_lien_tiep` xem xét riêng xem có phải race fixture (nếu là race ⇒ lỗi harness cần sửa, không phải WinSock semantics).
4. **Lớp E (Unix mode)** — bit x/`#[cfg(unix)]` cho test mode Unix; **`long` trên Windows trình A0 riêng** (ẩn cột mode / placeholder tài liệu hóa / thuộc tính Windows phù hợp); **chưa** đổi output sản phẩm.
5. **CI**: chưa đưa L0 đỏ thành required CI; tối đa informational/nightly (không `continue-on-error` trình bày như pass; lưu danh sách đỏ; cảnh báo regression bước sản phẩm đã xanh). Sau VD portability: Tier 1 + required CI (hoặc Windows full gate nightly + smoke PR); Unix-only tests không tính Windows count.
6. **Không gộp** VD Windows với Phase B-3 / Option-long / lambda-annotation / #147 / #152.
7. **`#147-mac`, `#152-vế-mac`**: tiếp tục chờ mps; **không** dùng Windows/Linux thay bằng chứng mac. **Phase B-3**: chờ ratify riêng. **Lambda annotation / #155**: quy trình riêng. **Q-OPTION-1/2**: [ĐÍNH CHÍNH 2026-11-19 — lệnh tiếp theo của A0]: A0 làm rõ "thiếu lớp ngoài/trong" mô tả **giá trị vắng mặt hợp lệ**, không phải sai kiểu — `None: int??` **hợp lệ** khi kiểu đích rõ, `Some(None): int??` **hợp lệ**; ba trạng thái phân biệt được, đều hợp lệ ("lồng thật, không flatten"). **Không** VS0412-cho-f, **không** đổi `long-f.vis` thành `.err`. Engine đang từ chối `let e: int?? = None` (VS0412, VD-238) ⇒ **finding cần tái lập** — Agent 2 đang probe checker/interp/VM; chưa sửa engine/golden. Xem `agent/QD-OPTION-LONG-2026-11-19.md` §4 + `agent/DECISIONS.md` đính chính 2026-11-19.
8. **Quyền truy cập psvn**: `agent/PSVN-ACCESS-2026-11-19.md` — owner, fingerprint, vị trí dòng, cách thu hồi (xóa **đúng dòng** `ps-host-to-psvn`), ngày review `2026-12-19` (hoặc khi pha 1 đóng / VD portability khởi động).
