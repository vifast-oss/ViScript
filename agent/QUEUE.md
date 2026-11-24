# QUEUE — ViScript, vòng 1

Người ghi: **Agent 0** (qua người điều phối). Người đọc/thi hành: **Agent 2** dòng ViScript.
Nguồn: `docs/QUYET-DINH-sau-review.md` (QD) §7, `review/PATCH-BRIEF.md` §2, `docs/viscript/ViScript-thiet-ke.md` §20.2.
Trạng thái hợp lệ: `READY | BLOCKED | REVIEW | DONE`.
**Tệp này tự chứa**: đọc hết 5 khối dưới đây là đủ để chạy một task mà không cần mở tài liệu khác.

---

## Khối A — Luật chơi (bắt buộc)

**Vòng lặp 8 bước cho mỗi task**

1. **Đọc** `STATE.md`, `QUEUE.md`, `DECISIONS.md`, phần SPEC liên quan. Không đọc lại toàn repo.
2. **Lập kế hoạch nhỏ** trong `STATE.md`: các bước, tệp sẽ chạm, test sẽ thêm.
3. **Test đỏ trước**: viết test/kịch bản thể hiện yêu cầu; chạy, xác nhận đỏ **đúng lý do**.
4. **Cài đặt** theo bước nhỏ; sau mỗi bước chạy `./vis-dev check`.
5. **Cổng**: `./vis-dev gate --fast` (luôn).
6. **Commit**: `[ST-##] <tóm tắt>` + kết quả cổng + `BUILD_ID`.
7. **Cập nhật** `STATE.md`, `JOURNAL/YYYY-MM-DD.md`; `vifast-agentctl --project viscript done <id>`.
8. **Nén ngữ cảnh** nếu lớn, rồi quay lại bước 1 với task kế.

**Cấm (9 điều)**

1. Sửa `SPEC-lex.md` / `SPEC.md` ngoài phạm vi `ST-02a`/`ST-02b`.
2. Sửa `accept()` / kỳ vọng / snapshot golden để test xanh.
3. `#[ignore]`, tắt test, bỏ case khỏi danh sách chạy, dùng `...` trong fixture (QD §7.4 cấm).
4. Nới ngưỡng (thời gian fuzz, số golden, số fixture).
5. Retry để qua cổng; flake là `T-INFRA`, ghi log, chạy lại tối đa **một** lần chỉ để phân loại.
6. Đổi ngữ nghĩa ngôn ngữ (cú pháp, quy tắc kiểu) mà không qua Agent 0 — bắt buộc `BLOCKED` + đề xuất + ví dụ (VD-08).
7. Thêm `unsafe` mà không có case âm; vượt **1.500 dòng/tệp**.
8. Đưa số đo viết tay vào tài liệu (số đo do `./vis-dev nightly` sinh).
9. Sửa `DECISIONS.md` hoặc QUEUE của dòng OS.

**Bắt buộc**: hồi quy ⇒ **revert trước**, điều tra sau; lỗi từ fuzz/vi sai ⇒ thu nhỏ, ghi `tests/regress/`, test đỏ trước rồi mới sửa; `cargo fmt` trước commit; SPDX theo loại tệp (`#` cho `.vis .vish .vidl .toml`, `//` cho Rust); `vis-host-vifast` **không** nằm trong `viscript/`.

## Khối B — Ngân sách và điều kiện dừng (3 giờ/task)

| Tình huống | Hành động |
|---|---|
| Task vượt **3 giờ** (ST-02a: 3 giờ / **2 phiên**) | Dừng, ghi tiến độ + lý do, chuyển task kế không phụ thuộc |
| Kẹt **hai lần** cùng một ý | `BLOCKED` + câu hỏi cho Agent 0, đổi nhánh (quy tắc 2.5) |
| SPEC mâu thuẫn/thiếu | `BLOCKED` kèm đề xuất + ví dụ cụ thể, **không** tự đổi SPEC |
| Cổng đỏ do lỗi không thuộc task | Mục lỗi trong `JOURNAL`, **không** sửa lan |
| Hết task khả dụng | Chạy `./vis-dev nightly`, viết báo cáo, dừng |

**Mỗi task ≤ 3 giờ. Task lớn hơn phải tách.**

## Khối C — Định nghĩa "xong" (DoD)

1. Tiêu chí trong bảng dưới đạt, **có bằng chứng** (đầu ra lệnh, số golden, TAP, snapshot).
2. Test mới **đã từng đỏ** (dán log đỏ vào `JOURNAL`).
3. Cổng tương ứng **xanh, không retry**.
4. `STATE.md` + `JOURNAL/` cập nhật; không còn TODO không có task.

## Khối D — Lệnh cổng

```bash
./vis-dev check          # fmt + clippy + luật cấu trúc (≤1.500 dòng/tệp, unsafe có case âm, SPDX)
./vis-dev gate --fast    # BẮT BUỘC cuối mỗi task: check + L0 + golden hiện có
./vis-dev nightly        # fuzz dài, snapshot toàn bộ, benchmark — không chạy trong vòng task
```

- Dòng ViScript **không có** công cụ cho tới khi `VT01`/`ST-01` dựng khung: giai đoạn đầu ghi "cổng chưa có" vào `STATE.md`.
- Fuzz 10 phút là **một phần tiêu chí** của ST-04, không phải cổng thường ngày.

## Khối E — Định dạng báo cáo

`reports/R<n>-viscript.md` — 6 mục: (1) tóm tắt 5 dòng; (2) bảng task `ID | trạng thái | commit | cổng`; (3) lỗi tìm thấy (fixture/`.regress`); (4) số đo thay đổi; (5) golden/fixture đã thêm; (6) `BLOCKED` + câu hỏi + đề xuất task kế.

`reports/R<n>-viscript.json` — cùng schema với dòng OS, **có `project: "viscript"`**:

```json
{
  "project": "viscript",
  "round": 1,
  "head": "ca2eaf5",
  "build_id": "—",
  "tasks": [{"id": "ST-01", "status": "DONE", "commits": ["…"], "gates": ["fast"]}],
  "gates": {"fast": "PASS", "full": "—", "nightly": "—"},
  "sim": {"seeds": 0, "failures": 0},
  "coverage": {"tokens": "—", "fixtures": "—"},
  "mutants": {"killed": 0, "alive": 0},
  "perf": {},
  "blocked": []
}
```

Agent 0 đọc **JSON trước**, MD sau. JSON thiếu `project` bị trả lại.

---

## Bảng task vòng 1

| ID | Task | Phụ thuộc | Ngân sách | Trạng thái | Tiêu chí xong |
|---|---|---|---|---|---|
| ST-01 | workspace `viscript/` + `viscript/agent/` (không ghi đè) + SPDX theo loại tệp + cổng 1.500 dòng/unsafe có case âm + `vis-host-vifast` **không** nằm trong `viscript/` | — | 3 h | **DONE** | cổng chạy; 3 case âm |
| ST-02a | **Áp errata mục 3 (S1..S11, L1..L8) + APPROVED**; SPEC-lex + Phụ lục A + bảng ưu tiên do Agent 1 soạn | — | 3 h (2 phiên) | **DONE** | errata áp xong; **APPROVED 2026-11-12 Agent 0** |
| ST-02b | sổ mã lỗi VS + 4 chữ số + quy tắc cấp | ST-01 | 2 h | **DONE** | bảng mã; `vis explain` được |
| ST-03 | `vis-diag`: cột theo Unicode scalar, render, bảng vi/en/ja | ST-01 | 3 h | **DONE** | snapshot 20 ca, có tiếng Việt/emoji |
| ST-11 | `vis-types` nền: `Type`, HIR, giải tên, hạ CST→HIR (script) | ST-05 | 3 h | **DONE** | HIR cho 15 khối tài liệu + 30 golden biểu thức; lỗi tên `VS03xx`; đỏ trước |
| ST-12 | `vis-types`: kiểm kiểu VS0 + `match` xét hết | ST-11 | 3 h | **DONE** | `VS04xx`; ≥ 80 ca kiểu (đúng/sai đều có) |
| ST-12b | sửa 7 lỗi kiểm kiểu (review ST-12) + coverage | ST-12 | 1,5 h | **DONE** | 7 mục PHẢI SỬA + tệp `tests/tyck_v2.rs`; đỏ trước |
| ST-13 | `vis-interp`: thông dịch cây trên HIR + builtins lõi | ST-12 | 3 h | **DONE** | chạy được chương trình VS0; `VS05xx` runtime |
| ST-14 | `vis run` + TAP harness ngữ nghĩa + 100 ca đầu | ST-13 | 3 h | **DONE** | `vis run <tệp.vis>`, exit 0/1/2 |
| ST-15b | sửa 4 lệch thiết kế (push/CoW, closure, `{x,..}`, pattern trần) | ST-15 | 2 h | **DONE** | 4 ca sem mới + 1 kỳ vọng cũ sửa có lý do (ca thứ 5) |
| ST-15 | thêm test ngữ nghĩa tới **≥ 300 ca** | ST-14 | 3 h | **DONE** | tổng ≥ 300 `.vis` + kỳ vọng stdout |
| ST-16 | fuzz parser **1 giờ** + `vis-dev nightly` chạy được (bỏ exit 2) | ST-14 | 2 h | **DONE** | không panic; nightly PASS |
| ST-18 | `cmd_lower`: chế độ lệnh `.vish` hạ xuống HIR + chạy được | ST-17 | 3 h | **DONE** | bảng chữ ký tối thiểu; `vis run t.vsh` chạy; `VS06xx`? (hỏi A0) |
| ST-19 | `vis-bytecode`: định dạng VBC1 + sinh mã từ HIR | ST-18 | 3 h | **DONE** | magic/phiên bản; sinh mã cho subset VS0; test khứ hồi |
| ST-20b | differential về **0 lệch** (sửa 19 ca) | ST-20 | 2 h | **DONE** | 272/272 khớp (stdout+exit+mã,dòng,cột); KNOWN_LECH rỗng |
| ST-20 | `vis-vm`: máy ảo thực thi bytecode, ngữ nghĩa trùng interp | ST-19 | 3 h | **DONE** | chạy 309 ca sem qua VM = interp |
| ST-21 | verifier bytecode + fuzz không panic | ST-20 | 2 h | **DONE** | bytecode hỏng bị từ chối; fuzz 10 phút |
| ST-22 | differential interp vs VM **≥ 100.000 chương trình, 0 lệch** | ST-21 | 3 h | **DONE** | sinh chương trình có seed; so stdout+mã lỗi |
| ST-23 | cache bytecode theo hash closure + khởi động ấm < 20 ms | ST-22 | 2 h | **DONE** | đo thật, không ước lượng |
| ST-25 | module: `use ./util`, đa tệp, cấm vòng, hash closure gồm phụ thuộc | ST-24 | 3 h | **DONE** | đa tệp chạy được; `VS07xx` cho vòng/không thấy |
| ST-26 | `vis-std` lõi: `std.str/list/map/path/time/json` | ST-25 | 3 h | **DONE** | ≥ 60 ca; `use std.x` chạy |
| ST-27 | `vis-host-posix`: chương trình ngoài (§9.3), variadic, `--x=v` | ST-26 | 3 h | **DONE** | chạy `ls`/`echo` thật; quyền tối thiểu |
| ST-28 | generic hàm + kiểu `[T]` | ST-27 | 3 h | **DONE** | ≥ 40 ca; differential vẫn 0 lệch |
| ST-29 | trait + `derives` (Eq/Show/Json) | ST-28 | 3 h | **DONE** | ≥ 30 ca |
| ST-30 | `vis test` + SHA-256 cho hash closure/consent | ST-29 | 2 h | **DONE** | `sha2` duyệt allowlist; thay FNV |
| ST-30b | đồng bộ chẩn đoán interp↔VM + siết differential (so đủ) | ST-30 | 2 h | **DONE** | 312/312 khớp, 0 lệch |
| ST-32 | harness so chẩn đoán **biên dịch** hai đường (đóng nợ 26 ca) | ST-31 | 2 h | **DONE** | 0 lệch; không nới |
| ST-33 | `json.decode` + `std.fs` tối thiểu (qua `Host`) | ST-32 | 3 h | **DONE** | đối xứng encode; interp/vm không chạm OS |
| ST-34 | `\| complete` + pipe→stdin cho chương trình ngoài | ST-33 | 3 h | **DONE** | `grep` exit 1 hết là lỗi; 2 tiến trình nối |
| ST-35 | generic **thật**: `Hop[int] ≠ Hop[str]` | ST-34 | 3 h | **DONE** | sai kiểu ⇒ VS0412; differential 0 lệch |
| ST-36 | báo cáo **R5** | ST-32..ST-35 | 1 h | **DONE** | MD+JSON, số liệu tái lập |
| ST-37 | `CmdSig` **generic** + khe ống **hợp nhất** (bỏ `==`); `take`/`lines` generic; seam `SigSource` | ST-36 | 3 h | **DONE** | `( [1,2] ) \| take 1` xanh; 338 ca sem không đổi; differential 0 lệch |
| ST-38 | hạ **closure-arg** (`CmdClosure` → `Lambda`, cộng offset span) + `where`/`each`/`first`/`count` | ST-37 | 3 h | **DONE** | 8 ca dương + 4 âm; lỗi trong thân closure đúng `dòng:cột`; `VS0412` cho thân sai kiểu |
| ST-39 | `sort_by` + `--desc` (mở `list.sort_by` cho khoá `int/size/duration/str`, giữ ổn định) | ST-38 | 3 h | **DONE** | 6 ca; ca cũ `list.sort_by` vẫn xanh; khoá `bool` ⇒ `VS0412` |
| ST-40a | **Prelude bản ghi** `FileEntry`/`ProcEntry` + `it` + **viết tắt trường**; mã `VS0605` | ST-39 | 3 h | **DONE** | `where size > 5mb` xanh trên `list[FileEntry]`; 6 ca mơ hồ; sổ `06xx` 4 → 5 mã |
| ST-40b | `get`/`select` chiếu theo trường (hạ thành `map` trên `it.<field>`) | ST-40a | 2 h | **DONE** | `\| get name` ⇒ `list[str]`; trường không có ⇒ `VS0605` |
| ST-41 | **`Render`**: bảng `list[bản ghi]` + khối khoá–giá trị; cột theo scalar sau **NFC** | ST-40b | 3 h | **DONE** | 15 snapshot (Việt NFD/emoji/CJK); `print` cũ cho `list[str]` không đổi |
| ST-42 | `ls`/`open` qua `FsHost` (`list_entries` + metadata), `--all/--long`, cờ **`--fs-mock`** | ST-41 | 3 h | **DONE** | 12 ca tất định; pipeline §9.7 dòng 672 chạy được; allowlist **8 → 7** |
| ST-43 | báo cáo **R6** (VSH1a) | ST-42 | 1,5 h | **DONE** | MD + JSON `project: "viscript"`, số liệu tái lập, 8 hàng |
| ST-44 | `.vish` **khai báo kiểu**: `type` / `let x: T` / `cmd f(...) -> T` | ST-43 | 3 h | **DONE** | prelude dùng bằng khai báo thật; `VS0302` khi trùng; differential 0 lệch |
| ST-45 | **`where` kiểm tĩnh đầy đủ** (vị từ không `bool` ⇒ lỗi tĩnh) | ST-44 | 2 h | **DONE** | hết nợ `VS0501` lúc chạy; kiểu `{error}` ⇒ vẫn kiểm động |
| ST-46 | `ls --long` thật + `( … )` làm **đối số** + chẩn đoán **tuyệt đối** trong ngoặc | ST-45 | 3 h | **DONE** | 3 nợ VSH1b hết; allowlist không tăng |
| ST-47 | báo cáo **R7** | ST-44..ST-46 | 1,5 h | **DONE** | MD+JSON, số liệu tái lập |
| ST-48 | thân `cmd` trong `.vish` bằng **pipeline lệnh** (VSH1b-2) | ST-47 | 3 h | **DONE** | `cmd f() -> list[FileEntry]:` + khối `ls \| where …` chạy 2 đường |
| ST-49 | **nhóm-lệnh có cờ** `( ls … --long )` + **nhóm nhiều dòng** | ST-48 | 2,5 h | **DONE** | 2 nợ VSH1b hết; `( ls … )` không còn rơi về lệnh ngoài |
| ST-50 | **thống nhất mã field-access** + `where ( x => x )` kiểm **tĩnh** | ST-49 | 2,5 h | **DONE** | một mã duy nhất; snapshot vi/en/ja kèm lý do |
| ST-51 | **stage đầu là biểu thức** (`[1,2] \| take 1` không ngoặc) | ST-50 | 2 h | **DONE** | không phá văn phạm §9.5; `( … )` vẫn chạy |
| ST-52 | báo cáo **R8** | ST-48..ST-51 | 1,5 h | **DONE** | MD+JSON, số liệu tái lập |
| ST-53 | `select` (bản ghi ẩn danh) đi `to json` + **hợp nhất hai `sort_by`** | ST-52 | 3 h | **DONE** | một hiện thực; `std.list.sort_by` cũ không phá; ổn định + `--desc` |
| ST-54 | `CODEGEN_TIN_TYCK`: codegen **từ chối** khi HIR còn kiểu `Error` | ST-53 | 3 h | **DONE** | chẩn đoán rõ; hằng số 21 phải **giảm**; diff 0 lệch |
| ST-55 | báo cáo **R9** + kế hoạch **VS2** có tiêu chí đo | ST-53..ST-54 | 1,5 h | **DONE** | MD+JSON; nêu rõ khoảng cách VS2 |
| ST-56 | nợ lõi **A**: khoá `str` cho `std.list.sort_by`; `&self`/`&expr` (hết `Unsupported` ở codegen); `Hop[Hop[int]]` đóng nốt | ST-55 | 3 h | **DONE** | 3 nợ đóng, mỗi nợ ≥2 ca; interp ↔ `--vm` cùng byte |
| ST-56b | chốt **gán** một ngữ nghĩa hai đường: `self.x = v` khi `self` bất biến ⇒ `VS0412`; `var self`/`var xs` gán được ở **interp** (VM đã làm); `run --vm` lỗi codegen ⇒ exit ≠ 0 | ST-56 | 2,5 h | **DONE** | ≥6 ca; interp = `--vm` cùng byte; không nới ngưỡng |
| ST-57 | nợ lõi **B**: mồ côi **đa tệp**, **siêu trait** kiểm được, **`derives` sinh mã** (`Show`/`Eq`) | ST-56 | 3 h | **DONE** | mỗi mục ≥2 ca đỏ→xanh; derives sinh phương thức **chạy được** |
| ST-58 | nợ lõi **C**: `Ty::Any` → chữ ký **generic thật** cho `std.list.*`/`std.map.*` | ST-57 | 3 h | **DONE** | không còn `Ty::Any`; sai kiểu ⇒ `VS04xx` **tĩnh** |
| ST-59 | nợ shell: thân `cmd` dùng **tham số**; body "vừa script vừa lệnh" có luật rõ; **biến** làm stage đầu | ST-58 | 3 h | **DONE** | ≥6 ca; `x \| take 1` chạy 2 đường |
| ST-60 | **VS2 #1 — async**: `async fn`/`await`/`spawn` + bộ lập lịch **tất định một luồng** | ST-59 | 3 h | **DONE** | ≥20 ca sem interp = `--vm` cùng byte; 10.000 task không tràn |
| ST-60b | hotfix sau review độc lập: gán trong **closure** (captured/param) hồi quy parity; đích lồng `self.xs[0]`; `.vish` shell-let `let out = ls -l`; `sort_by --desc` bị bỏ im lặng | ST-60 | 2,5 h | **DONE** | ≥8 ca; interp = `--vm` cùng byte; không nới ngưỡng |
| ST-60c | derives/siêu trait: impl **sinh** vào `impl_pairs`; `enum Show` phải **lỗi tĩnh**; `Eq` chặn **sâu** (list/map/opt của float); SPDX 19 tệp dữ liệu + header `diag_cmp` | ST-60b | 2 h | **DONE** | ≥4 ca; gate xanh |
| ST-61 | **VS2 #2 — kênh**: `chan[T]`, `send/recv/close`, **deadlock có chẩn đoán** | ST-60 | 3 h | **DONE** | ≥15 ca; ca deadlock **không treo**, mã lỗi rõ |
| ST-62 | báo cáo **R10** | ST-56..ST-61 | 1,5 h | **DONE** | MD+JSON; số liệu tái lập; nêu nợ còn lại |
| ST-63 | **VS2 #3a** — `NetHost` + TCP client (`std.net.connect/read/write/close`, timeout) | ST-62 | 2,5 h | **DONE** | 5 ca e2e loopback hai đường; không host ⇒ `VS0604` |
| ST-64 | **VS2 #3b** — `http.get`/`get_timeout` (giao thức trong `vis-interp`) | ST-63 | 2,5 h | **DONE** | 6 ca e2e: 200/404/timeout/refused/8 MiB/sai kiểu |
| ST-65 | **VS2 #3c** — server: `net.listen/local_addr/accept(_timeout)` + e2e 2 tiến trình | ST-64 | 3 h | **DONE** | 4 ca; bàn giao cổng qua tệp; ≤3 s, không flake 3 lần |
| ST-66 | **VS2 #3d** — ca biên net/http + `--explain-perms` NET + `--vm` cache/no-cache | ST-65 | 2 h | **DONE** | ≥13 ca tổng; `gate` xanh |
| ST-67 | **VS2 #5a** — `vis lsp`: khung Content-Length + initialize/shutdown/exit + lỗi framing | ST-66 | 2,5 h | **DONE** | 6 snapshot; stdout sạch; thoát khi EOF |
| ST-68 | **VS2 #5b** — LSP diagnostics + vị trí **UTF-16** (emoji, tab, CRLF) | ST-67 | 3 h | **DONE** | 6 snapshot + 6 unit vị trí |
| ST-69 | **VS2 #5c** — LSP completion/hover + ≥13 phản hồi snapshot | ST-68 | 2,5 h | **DONE** | ≥13 frame; prefix/hover/null |
| ST-70 | **VS2 #6a** — `vis pkg add/install/list`, manifest/lock/SHA-256, registry local | ST-69 | 3 h | **DONE** | 6 ca + e2e tất định; không mạng |
| ST-71 | **VS2 #6b** — `use pkg.*` nạp gói + kiểm sha lúc chạy | ST-70 | 2,5 h | **DONE** | 4 ca; lock không có đường tuyệt đối |
| ST-72 | **VS2 #4a** — kiểu `cap[Dir/File/Net/Proc]` trong tyck + `vis run --dry` liệt kê quyền khai báo | ST-71 | 3 h | **DONE** | `cap[Dir] @ p"./x" (write)` parse+check được; ≥5 ca sem; `--dry` in |
| ST-72b | **VS2 #4b** — cưỡng chế quyền ở `HostFs`/`HostNet` + `--grant name=path` | ST-72 | 3 h | **DONE** | ≥8 ca cho phép/từ chối (`VS0604` nêu quyền thiếu); không khai ⇒ không hạn chế (ghi rõ) |
| ST-73 | **VS2 #4c** — `vis run --sandbox-check` (probe Landlock Linux, không `unsafe`) + khung `vis-host-linux` **ngoài** `viscript/` + tài liệu **mức bảo đảm theo nền** | ST-72b | 2,5 h | **DONE** | ≥6 ca probe; `cargo check --target x86_64-unknown-linux-gnu` xanh; **không** tuyên bố đã cưỡng chế bằng Landlock |
| ST-74 | **VS2 #7** — `vis-host-windows`/`wasm`: check chéo `x86_64-pc-windows-msvc` + `wasm32-unknown-unknown` | ST-73 | 2,5 h | **DONE** | cả hai target `check` xanh; nêu rõ mức bằng chứng |
| ST-75 | **VS2 #8** — bộ hợp chuẩn host `host-conformance` ≥50 ca chạy **trên macOS** + khung 4 nền | ST-74 | 3 h | **DONE** | ≥50 ca xanh; 3 nền còn lại ghi rõ chưa đo |
| ST-76 | báo cáo **R11** | ST-63..ST-75 | 1,5 h | **DONE** | MD+JSON; nêu rõ hạng mục VS2 nào **chưa** đo được |
| ST-77 | **nhất quán #1** — `vis fmt` cho cú pháp mới: `cap[...]` + `@`/`(write)`, `async fn`, `spawn`, `chan[T]`, `use pkg.*` | ST-76 | 2,5 h | **DONE** | ≥8 ca; lossless + idempotent; `fmt --check` trên toàn `test/` xanh |
| ST-78 | **nhất quán #2** — `vis explain`/sổ mã: `VS0504`/`VS0505`/`VS0506` đủ vi/en/ja + `--list` + phụ lục | ST-77 | 2 h | **DONE** | 3×3 thông điệp; `explain --list` phủ 32 mã; snapshot |
| ST-79 | **nhất quán #3** — `vis test` với `async`/`await`/`chan` (khối `test` chờ task) | ST-78 | 2,5 h | **DONE** | ≥6 ca; TAP đúng; interp = `--vm` |
| ST-80 | **nhất quán #4** — LSP **module-aware**: giải `use ./x` + `std.*` khi chẩn đoán | ST-79 | 3 h | **DONE** | ≥6 ca; hết chẩn đoán giả cho `std.*`; vẫn một tệp nếu không `use` |
| ST-81 | **nhất quang #5** — `--strict-caps`: bắt buộc khai `cap` cho mọi truy cập fs/net | ST-80 | 2,5 h | **DONE** | ≥6 ca; mặc định **không đổi** (ghi rõ); fixture chuyển dần |
| ST-82 | báo cáo **R12** | ST-77..ST-81 | 1,5 h | **DONE** | MD+JSON; nêu rõ phần chưa làm được và **vì sao** |
| ST-83 | **nợ nhỏ** — `fs.exists` ngoài quyền ⇒ `VS0604` (hết im lặng), `assert <đk>` dạng **câu lệnh**, `usage_explain` theo `--lang` | ST-82 | 2,5 h | **DONE** | ≥6 ca; không đổi hành vi đã khoá |
| ST-84 | **LSP** completion/hover **module-aware** (ký hiệu từ `use ./x` + `std.*`) | ST-83 | 2,5 h | **DONE** | ≥4 ca; vẫn tất định |
| ST-85 | **`vis fmt`** chuẩn hoá **thụt lề khối `cmd`** trong `.vish` + cập nhật fixture | ST-84 | 3 h | **DONE** | `--check test/` xanh; 68 fixture cập nhật **có lý do** |
| ST-85b | **lỗi thật phát hiện ở ST-85**: dòng cấp tệp **thụt lề** trong `.vish` bị **bỏ im lặng** (in ra rỗng, exit 0) — phải **chẩn đoán** hoặc xử lý đúng | ST-85 | 2,5 h | **DONE** | ≥6 ca; không còn "im lặng bỏ dòng"; hành vi cũ hợp lệ không đổi |
| ST-86 | **fuzz differential** mở rộng cho cú pháp mới (`async`/`chan`/`cap`/`derives`) — 20.000 chương trình | ST-85 | 3 h | **DONE** | 0 lệch; seed ghim; sinh được cả ca lỗi |
| ST-87 | **cache không che lỗi**: chạy toàn bộ suite ở 3 chế độ (`run`, `--vm`, `--vm --no-cache`) so byte | ST-86 | 2,5 h | **DONE** | 0 lệch trên ≥500 ca; sửa nếu lệch |
| ST-87b | **lỗi thật phát hiện ở ST-87**: fast-path cache **dead code** (`main.rs` đọc khoá v1 `khoa`, ghi khoá v2 `khoa_multi`) ⇒ tệp một-module **không bao giờ** cache hit | ST-87 | 2 h | **DONE** | ≥4 ca: TAP lần 2 có `(cache)`, một tệp 2 lần chạy nhanh hơn, đa tệp không đổi |
| ST-88 | báo cáo **R13** | ST-83..ST-87 | 1,5 h | **DONE** | MD+JSON; nợ còn lại đánh số |
| ST-89 | **parity cuối** — host `Ngoai` cho **VM** (chương trình ngoài trong `.vish` chạy cả hai đường) | ST-88 | 3 h | **DONE** | `--vm` SKIP 7 → **0**; parity 398 → **405**; suite ngoài chạy 2 đường cùng byte |
| ST-90 | **pkg** — lưu `entry` trong `vis.lock` + `vis pkg remove` | ST-89 | 2,5 h | **DONE** | ≥5 ca; lock cũ vẫn đọc được (tương thích) |
| ST-91 | **LSP** — completion **sau `.`** theo kiểu (field bản ghi + phương thức `impl`) | ST-90 | 2,5 h | **DONE** | ≥5 ca; tất định; không phá 34 ca LSP |
| ST-92 | báo cáo **R14** | ST-89..ST-91 | 1,5 h | **DONE** | MD+JSON; nợ còn lại đánh số |
| ST-93 | **`cap[Proc]`** — chương trình ngoài phải có quyền `cap[Proc]`; `--strict-caps` cưỡng chế | ST-92 | 2,5 h | **DONE** | ≥6 ca; hai đường cùng byte; `VS0604` nêu `cap[Proc]` |
| ST-93b | **im lặng mới phát sinh ở ST-93**: `.vish` có `fn main` **có thân** ⇒ thân `main` **không chạy** (trước đây cũng vậy nhưng im lặng) — phải **chẩn đoán** | ST-93 | 1,5 h | **DONE** | ≥4 ca; `main` chỉ khai `cap` (thân rỗng) vẫn hợp lệ |
| ST-94 | **kênh** — `chan[T](0)` rendezvous + ngữ nghĩa `close` (recv sau close, send sau close) + drain `Moi` | ST-93 | 3 h | **DONE** | ≥8 ca; không treo; interp = VM |
| ST-95 | báo cáo **R15** | ST-93..ST-94 | 1,5 h | **DONE** | MD+JSON; nợ đánh số |
| ST-96 | **nhất quán sổ mã**: mọi `VS0xxx` xuất hiện trong mã nguồn đều **có** trong `all_codes()`, và mọi mã trong sổ **được dùng** (hoặc ghi rõ là dự phòng) | ST-95 | 2 h | **DONE** | test tự động quét `crates/`; ≥3 ca; sửa lệch nếu có |
| ST-97 | **`--grant` cho `cap[Proc]`** (allowlist lệnh) — đóng nợ "cap[Proc] không allowlist" | ST-96 | 2,5 h | **DONE** | ≥5 ca; hai đường cùng byte; mặc định không đổi |
| ST-98 | **soak 30 phút**: cổng ×2 + diff 100k ×2 + host ×2 + cache ×2 tuần tự, ghi log, phát hiện flake | ST-97 | 1,5 h | **DONE** | ≥8 lần chạy; mọi lần xanh hoặc ghi `T-INFRA` có log |
| ST-99 | **R16 + ROADMAP bàn giao** (`agent/ROADMAP.md`: 8 hạng mục VS2 × mức × điều kiện để đóng + cách tiếp tục) | ST-98 | 2 h | **DONE** | MD+JSON; `report-check` PASS; ROADMAP ≤120 dòng |
| ST-100 | **LSP** — hover cho **trường** bản ghi + `didClose` dọn cache + cache nạp module giữa `didChange` | ST-99 | 2,5 h | **DONE** | ≥5 ca; phân tích < 2 ms sau lần đầu; không phá 39 ca LSP |
| ST-101 | **`vis check --json`** + `vis test --json` (đầu ra máy đọc) | ST-100 | 2,5 h | **DONE** | ≥6 ca; JSON ổn định (khoá cố định); TAP giữ nguyên khi không cờ |
| ST-101b | **lỗ hổng JSON**: chuỗi chưa được **escape** (`"` `\` newline, ký tự < 0x20) ⇒ JSON **không hợp lệ** khi message chứa chúng | ST-101 | 1,5 h | **DONE** | ≥4 ca; parse lại bằng `json.decode` của chính ViScript; không đổi định dạng |
| ST-102 | **`vis pkg update`** + `pkg list --json` (không mạng) | ST-101 | 2 h | **DONE** | ≥5 ca; lock/store nhất quán; `update` không đổi sha nếu không đổi nguồn |
| ST-103 | báo cáo **R17** | ST-100..ST-102 | 1,5 h | **DONE** | MD+JSON; nợ đánh số |
| ST-104 | **HTTP** — `Transfer-Encoding: chunked` + theo **redirect 3xx** (tối đa 5) + `Connection: close` | ST-103 | 3 h | **DONE** | ≥6 ca với server loopback; không treo; hai đường cùng byte |
| ST-105 | **`vis test --timeout <ms>`** — ca treo bị **cắt** và đánh `not ok` rõ | ST-104 | 2,5 h | **DONE** | ≥4 ca; không rò tiến trình; mặc định không đổi |
| ST-106 | **R1 headroom** — tách `lower/cmd.rs` (1.491 → ≤1.200) không đổi hành vi | ST-105 | 2 h | **DONE** | R1 max giảm; 447 sem + parity không đổi |
| ST-107 | báo cáo **R18** | ST-104..ST-106 | 1,5 h | **DONE** | MD+JSON; nợ đánh số |
| ST-108 | **R1 headroom #2** — tách `lower.rs` (1.482 → ≤1.200), thuần refactor, checksum chứng minh | ST-107 | 2,5 h | **DONE** | R1 max ≤1.200; 1045 test + diff sha256 trùng |
| ST-109 | **HTTP** — resolve `Location` đầy đủ (`..`, query-only) + tài liệu chunk trailer + `HEAD` | ST-108 | 2,5 h | **DONE** | ≥5 ca loopback; không hồi quy 16 ca http |
| ST-110 | **`check --json`**: thêm `severity` + `file` **đúng tệp phụ** cho chẩn đoán module | ST-109 | 2 h | **DONE** | ≥4 ca; khoá cố định; TAP không đổi |
| ST-111 | báo cáo **R19** | ST-108..ST-110 | 1,5 h | **DONE** | MD+JSON; nợ đánh số |
| ST-112 | **nợ correctness**: chẩn đoán **đa tệp** — câu lệnh cấp tệp chính có thể bị gắn sang module (span mỗi tệp bắt đầu từ 0). Chẩn đoán → sửa tối thiểu hoặc **kế hoạch** nếu >150 dòng | ST-111 | 3 h | **DONE** | tái hiện + chỉ đúng chỗ; nếu sửa: 1056 test xanh, ca mới cho cả 2 chiều; nếu không: kế hoạch ≤30 dòng |
| ST-113 | **soak cuối** (≥10 vòng tuần tự) + **R20** + cập nhật ROADMAP | ST-112 | 3 h | **DONE** | 10/10 xanh hoặc T-INFRA có log; R20 `report-check` PASS |
| ST-114a | **Span file-id (1/2)**: thêm `Span.file: u16` (mặc định 0 = y hệt), luồn file-id qua lex/parse/lower; **chưa** đổi chẩn đoán | ST-113 | 3 h | **DONE** | 1056 test y hệt; `Span::new` cũ giữ nguyên; checksum + diff sha256 trùng |
| ST-114b | **Span file-id (2/2)**: `modules.rs` map chẩn đoán **theo `file`** + `vi_tri` dùng **văn bản tệp đúng**; TAP/JSON đúng tệp | ST-114a | 3 h | **DONE** | ≥5 ca (cấp tệp chính, fn chính, module, cả hai, `.vish`); parity/sem/host không đổi |
| ST-115 | báo cáo **R21** | ST-114a..ST-114b | 1,5 h | **DONE** | MD+JSON; `report-check` PASS |
| ST-116 | **R1 headroom #3** — tách `parse/mod.rs` (1.452 → ≤1.200), thuần refactor + checksum | ST-115 | 2,5 h | **DONE** | R1 max giảm; 1062 test + diff sha256 trùng |
| ST-116b | **R1 headroom #4** — tách `crates/vis-cli/src/main.rs` (1.254 → ≤1.000): tách `run`/`check`/`test`/`fmt` sang `cli/*.rs`, thuần refactor | ST-116 | 2,5 h | **DONE** | R1 max ≤1.000; checksum 1062 test + diff sha256 trùng |
| ST-117 | **LSP** — chẩn đoán **đa tệp** cho tài liệu đang mở dùng `file` (ca a+b cùng lỗi; tài liệu mở là module) | ST-116 | 2 h | **DONE** | ≥4 ca; publish **chỉ** cho tài liệu mở; sửa nếu sai |
| ST-118 | **soak cuối 10 vòng + R22 + ROADMAP chốt** | ST-117 | 3 h | **DONE** | 10/10 xanh hoặc T-INFRA có log; `report-check` PASS |
| ST-119 | **Văn bản quản trị**: AGENTS.md (root, chỉ phần ViScript) + viscript/AGENTS.md theo Phụ lục A; STATE.md viết lại header ≤60 dòng (G1/G2/G3) | ST-118 | 1,5 h | **DONE**** | `grep -c 'VD-88' AGENTS.md viscript/AGENTS.md` = 0; header STATE.md có hash c1587398, vòng 23, task kế; check_report PASS; gate PASS |
> CHỜ DUYỆT Agent 0 · 324d1da9→af69a4ae (ST-119) · VD-88 = 0 trong cả 2 AGENTS.md; header STATE 35 dòng
APPROVED 2026-11-16 Agent 0 · duyệt theo QUYET-DINH-A0-2026-11-16b §4 · Agent 2 đã đối chiếu đủ tiêu chí
| ST-120 | **Đồng bộ thiết kế**: docs/viscript/ViScript-thiet-ke.md + ROADMAP.md + HOST-BAO-DAM.md theo Phụ lục B/C/D (G5/G6, Q3..Q7, Q10, Q12) | ST-119 | 2,5 h | **DONE**** | 4 dòng trỏ G6 đã chèn; cổng doc-examples PASS; không có số đo viết tay mới trong docs/; gate PASS |
> CHỜ DUYỆT Agent 0 · cfeee4c5 (ST-120) + 26e5f82c (ST-120b) · doc-examples OK (24 khối: 18 xanh + 6 ngoại lệ); 4 dòng trỏ đã chèn
APPROVED 2026-11-16 Agent 0 · duyệt theo QUYET-DINH-A0-2026-11-16b §4 · Agent 2 đã đối chiếu đủ tiêu chí
| ST-121 | **Thăm dò runner Linux** (Q2a): phát hiện docker/podman/colima; nếu có: Dockerfile + bin/vis-linux chạy `uname -r`, `cat /sys/kernel/security/lsm`, build | ST-118 | 1 h | **DONE**** | log thăm dò có sha256; có công cụ ⇒ container build xanh + in lsm; không có ⇒ BLOCKED-ENV + lệnh người dùng cần chạy |
> CHỜ DUYỆT Agent 0 · 627c698a (ST-121) · lsm CÓ landlock, syscall ABI=8, rustc 1.99.1; 2 lỗi portability đã vá
XEM LẠI Agent 0 · thiếu: log container build xanh (DoD nhánh "có công cụ"); backend đã dùng là ssh, mạnh hơn nhưng không phải artefact DoD yêu cầu
APPROVED 2026-11-16 Agent 0 · theo VD-138 (backend SSH thay container) · Agent 1 đối chiếu
| ST-122 | **Span tổng hợp mang file** (Q11, VD-127): test duyệt HIR fixture đa tệp + sửa các chỗ sinh node | ST-118 | 3 h | **DONE** | test duyệt: 0 node module phụ có file=0; ≥4 ca mới chẩn đoán rơi trên node tổng hợp ở module phụ, JSON+TAP đúng tệp/dòng, cả interp và --vm; parity giữ phân hoạch |
> CHỜ DUYỆT Agent 0 · b0189956 (ST-122) · audit 23 đơn vị đa tệp/321 node HIR: **65 → 0** node sai `file`; 5 ca `mod.vish:3:*` đúng ở `check --json` + TAP + `run` + `run --vm`; sem **447**, `vis test` **447 ok/0 SKIP**, parity **398+49**, `cargo test` **1072**, gate **PASS**, diff **lech=0** |
APPROVED 2026-11-16 Agent 0 · duyệt theo QUYET-DINH-A0-2026-11-16b §4 · Agent 2 đã đối chiếu đủ tiêu chí
| ST-123 | **--grant Proc theo đường dẫn tuyệt đối** (VD-130c) | ST-118 | 2 h | **DONE** | red: bản sao `echo` ở thư mục tạm chạy được khi grant /bin/echo; sau sửa: VS0604 exit 2; ≥6 ca (symlink, PATH, tương đối, trùng tên) × interp/--vm |
> CHỜ DUYỆT Agent 0 · 4c87cd59 (code) · 7 ca × interp/--vm xanh (VS0604 exit 2 cho bản sao trùng tên; symlink/PATH/tương đối/không tồn tại đều đúng); host suite 54/54; gate PASS; vis-diff 100.000 lech=0; đỏ trước sha256 5ffafd6e
APPROVED 2026-11-16 Agent 0 · duyệt theo QUYET-DINH-A0-2026-11-16b §4 · Agent 2 đã đối chiếu đủ tiêu chí
| ST-124 | **Review FsGuard TOCTOU + Value::Cap** (VD-130a/b) | ST-118 | 3 h | **DONE** | JOURNAL/st124-review.md: mỗi phát hiện có ca tái hiện hoặc lý do không tái hiện được; sửa không cần unsafe ⇒ sửa + ca; cần unsafe ⇒ ghi giới hạn vào HOST-BAO-DAM.md theo Phụ lục D2 |
> CHỜ DUYỆT Agent 0 · 706dba00 (code) · TOCTOU tái hiện **1661/50 000** (3,32 %) trước sửa → **0/50 000** sau sửa (mở-rồi-kiểm bằng `(dev,ino)` của handle, **không `unsafe`**); 7 ca Value::Cap xanh (json.decode/đọc tệp/đối số/`to json`=null/`print`+`==` = VS0412); đề nghị 1 dòng giới hạn **đường ghi** (#115) thay cho Phụ lục D2 chung
APPROVED 2026-11-16 Agent 0 · duyệt theo QUYET-DINH-A0-2026-11-16b §4 · Agent 2 đã đối chiếu đủ tiêu chí
| ST-125 | **Fuzz json_esc 100k** (VD-130e) | ST-118 | 1,5 h | **DONE** | seed cố định; 100.000 chuỗi; roundtrip json.decode và python3 json.loads; lech=0; có ký tự điều khiển, U+0000, U+2028/2029, ngoài BMP, `"` `\` |
> CHỜ DUYỆT Agent 0 · 2a1ff16a (code) · seed **20261115**, **100 000** ca × 4 phép (json.decode, python3 json.loads, + đường `to json`/`json_tai` cả hai phía) ⇒ **lech = 0** hết; không phải sửa `esc`; có ca đối chứng escaper hỏng (fuzz không vô nghĩa) · nợ #116: thực tế có **2** bộ escape
APPROVED 2026-11-16 Agent 0 · duyệt theo QUYET-DINH-A0-2026-11-16b §4 · Agent 2 đã đối chiếu đủ tiêu chí
| ST-126 | **Đóng băng API + tương thích** (Q14/Q15, VD-129): tạo API-STD.txt, API-CODES.txt, API-VBC1.txt + test so + kiểm khoá cache | ST-118 | 2,5 h | **DONE** | 3 tệp sinh từ code, đã sắp xếp; test đỏ khi xoá/đổi tên/thêm không khai; báo cáo: khoá cache có đổi khi bảng thẻ đổi không (đo bằng ca) |
> CHỜ DUYỆT Agent 0 · 1a1614cd (ST-126) · API 52/32/53 mục sinh từ code, test chặn bắt THÊM/XOÁ/ĐỔI đích danh (nhắc VD-129); khoá cache **KHÔNG** phủ bảng thẻ trước sửa (`799713b8…b7cd5`) → **CÓ** sau sửa (`6d11672c…4ccf`), 6 chiều ca kiểm xanh; `cargo test --workspace` 1077/0
APPROVED 2026-11-16 Agent 0 · duyệt theo QUYET-DINH-A0-2026-11-16b §4 · Agent 2 đã đối chiếu đủ tiêu chí
| ST-127 | **Semver: kiểu Version + VersionReq + so khớp** (Q9a, VD-125) | ST-126 | 2,5 h | **DONE** | ≥40 ca đơn vị phủ mọi toán tử + ^0.x + từ chối prerelease; không crate mới |
> CHỜ DUYỆT Agent 0 · 769331ee (ST-127) · `Version`/`VersionReq` 7 toán tử + `,`=AND + không-toán-tử⇒`^` (luật `^0.x` theo Cargo); 19 test/**114 assert** + 27 ca từ chối; prerelease/build + `||`/`*`/`!=`/khoảng `a-b` bị từ chối có chẩn đoán; không crate mới; clippy/fmt OK
APPROVED 2026-11-16 Agent 0 · duyệt theo QUYET-DINH-A0-2026-11-16b §4 · Agent 2 đã đối chiếu đủ tiêu chí
| ST-128 | **Resolver tất định + registry nhiều phiên bản** (VD-125) | ST-127 | 3 h | **DONE** | ≥12 ca: chọn cao nhất, quay lui, xung đột ⇒ mã VS07xx mới (cập nhật đủ ≥7 chỗ đếm + API-CODES.txt theo VD-125); chạy 2 lần cùng byte |
> CHỜ DUYỆT Agent 0 · e20eca56 (ST-128) · mã mới **VS0704** (khối 07xx, số kế tiếp) 3 ngôn ngữ, **all_codes = 33** (7 khối; 7 chỗ đếm + API-CODES.txt đã cập nhật); resolver **17 ca** + CLI **7 ca** (chọn cao nhất, quay lui 1–2 tầng, xung đột nêu **chuỗi ràng buộc** + nguồn `a@1.0.0`, 2 thư mục ⇒ lock **byte y hệt**); `next_block()` vẫn 801 (nợ #109); cổng toàn workspace bị chặn bởi `json_esc.rs` của agent khác (log có sha256)
APPROVED 2026-11-16 Agent 0 · duyệt theo QUYET-DINH-A0-2026-11-16b §4 · Agent 2 đã đối chiếu đủ tiêu chí
| ST-129 | **pkg CLI dùng resolver + migration lock cũ** (VD-125) | ST-128 | 2,5 h | **DONE** | install không đổi lock khi còn thoả; update giải lại; lock cũ ⇒ lock mới, ghi lại 2 lần cùng byte; pkg list --json có version_req |
> CHỜ DUYỆT Agent 0 · 13aa18f2 (ST-129) · **đo trước**: lock cũ 510 byte `{lock:1,packages}` (không roots/version_req); định dạng **2** `{lock:2,roots,packages[version_req]}`; `pkg_lock` **8 ca** (install giữ byte, migrate + 2 lần **cùng byte** `95da92fe…`, lock không còn thoả ⇒ exit 2 + `pkg update`, update **nâng 1.10.0→1.11.0** trong `^1.0`, `list --json` có `version_req`, remove bỏ roots); clippy/fmt OK (1 lint của `quyen.rs` — tệp ST-124 — đã loại trừ minh bạch)
APPROVED 2026-11-16 Agent 0 · duyệt theo QUYET-DINH-A0-2026-11-16b §4 · Agent 2 đã đối chiếu đủ tiêu chí
| ST-130 | **Siết caps khi dùng gói** (Q8c, VD-124) | ST-129 | 3 h | **DONE** | ≥6 ca × interp/--vm: có pkg + thiếu cap ⇒ VS0604 exit 2; --no-strict-caps ⇒ chạy + 1 dòng cảnh báo stderr; không pkg ⇒ hành vi cũ, 447 sem không đổi; >150 dòng ⇒ kế hoạch (VD-116) |
> CHỜ DUYỆT Agent 0 · 742971f2 (ST-130) · **37 dòng thêm/9 bớt** (<150 ⇒ làm, không cần kế hoạch); `siet = --strict-caps || (co_pkg && !--no-strict-caps)`, cảnh báo **1 dòng stderr**; **7 ca × interp/--vm** (VS0604 exit 2, thoát tường minh, không pkg ⇒ cũ, hai cờ mâu thuẫn); sửa **9 dòng** `lower.rs` (tên module std dưới Strict — lỗi có sẵn từ ST-71 chặn VD-124, có log trước/sau `ef2f3344…`); sem **447/22 không đổi**, diff **100k lech=0**, `cargo test --workspace` **1153/0**, **`gate --fast` PASS** (`2d84b18d…`)
XEM LẠI Agent 0 2026-11-16 · thiếu ca thứ 8 theo QUYET-DINH-A0-2026-11-16b §2 (tests/siet_caps_pkg.rs đang 7 #[test]). Trạng thái thực tế: READY cho phần bổ sung; chưa APPROVED.
> CHỜ DUYỆT Agent 1 · eab366d7 (ST-130b) · ca thứ 8 `use_module_noi_bo_va_std_nhung_khong_pkg_giu_hanh_vi_cu`: `use ./util` + `use std.fs`, **không** `pkg.*`, **không** `cap`, đọc tệp ⇒ chạy được (exit 0, `x\n7\n`), **KHÔNG `VS0604`**, cả interp và `--vm --no-cache`; **đối chứng nhúng trong ca**: cùng chương trình + `--strict-caps` ⇒ `VS0604` exit 2; **không có log đỏ thật** vì hành vi đã đúng — **ca khoá hồi quy** (log `st130b-do-xanh-doi-chung.log` `8c22c403…`); **8 #[test]**, sem **447/22 không đổi**, `vis test` **447 ok** / `--vm` **447 ok**, diff 100k **lech=0**, **`gate --fast` PASS** (worktree sạch HEAD `3ea7176d`, L0 **1154/0**, `52592bf7…`), report-check PASS
APPROVED 2026-11-16 Agent 0 · theo QUYET-DINH-A0-2026-11-16g §1 D1 · Agent 1 đối chiếu: siet_caps_pkg.rs 8 #[test]; ca thứ 8 chạy interp và --vm --no-cache, exit 0, có đối chứng --strict-caps ⇒ VS0604; sem 447/22 không đổi
| ST-131 | **Chạy đầy đủ trên runner Linux**: cargo test, gate --fast, host suite 54 | ST-121 | 3 h | **DONE** | số đo Linux đặt cạnh macOS; mọi lệch được liệt kê, KHÔNG sửa trong task này |
> CHỜ DUYỆT Agent 0 · Linux 1062/4 so với macOS 1066/0; host suite 54/54 = 0 lệch; gate PASS 10/11 bước (chỉ L0 đỏ vì 1 ca) · 4 lệch đã liệt kê trong JOURNAL/st131-linux.md §3 (không sửa trong task)
APPROVED 2026-11-16 Agent 0 · duyệt theo QUYET-DINH-A0-2026-11-16b §4 · Agent 2 đã đối chiếu đủ tiêu chí
| ST-132 | **Landlock thật** (Q1a, VD-120) | ST-131 | 3 h | **DONE** | log giấy phép phụ thuộc; ≥8 ca cfg(linux) xanh trên runner, trong đó ≥3 ca âm chứng minh kernel chặn ở tiến trình con; macOS: --sandbox exit 3, ghi "chưa đo" |
> CHỜ DUYỆT Agent 0 · vis-host-linux 12/12 ca trên runner (LSM có `landlock`, syscall ABI=8); macOS `--sandbox` exit 3 (chưa đo) · **cần quyết**: giấy phép `unicode-ident` (build-time) `(MIT OR Apache-2.0) AND Unicode-3.0` ngoài allowlist hẹp (JOURNAL/st132-landlock.md §5)
XEM LẠI Agent 0 · thiếu: ca âm thứ 3 ở tiến trình con chứng minh kernel chặn (hiện có 2: ca 10 đọc ngoài, ca 11 ghi ngoài; ca 12 là đối chứng dương)
APPROVED 2026-11-16 Agent 0 · theo QUYET-DINH-A0-2026-11-16g §1 D1 · Agent 1 đối chiếu: landlock.rs 13/13 trên ps; 3 ca âm ở tiến trình con (đọc, ghi, read_dir) có đỏ trước và đối chứng dương; macOS --sandbox exit 3
| ST-132b | **Ca âm Landlock thứ 3**: tiến trình con mở thư mục ngoài phạm vi (read_dir) | ST-132 | 1 h | **DONE** | chạy trên ps; con nhận PermissionDenied/EACCES ở read_dir; có đối chứng dương read_dir trong phạm vi cùng ca; đỏ trước (ca chạy khi không --sandbox phải liệt kê được thư mục); 12 ca cũ vẫn xanh; log môi trường theo VD-136 |
> CHỜ DUYỆT Agent 0 · `a47a1ffe` (ST-132b) · **13/13 ca** trên `ps` (12 cũ + `con_read_dir_ngoai_pham_vi_bi_kernel_chan`): con `read_dir` ngoài ⇒ `Permission denied` (EACCES), con `read_dir` trong ⇒ liệt kê được; **đỏ-trước** `exit=0 stdout="bi-mat.txt"` (không ruleset); log + sha256 `24502ed0…` (JOURNAL/st132b-do-truoc.log); HEAD đo `926022e4`, tệp đo sha256 `fa570a60…`; đủ **3 ca âm tiến trình con** theo VD-138
ĐÍNH CHÍNH 2026-11-16 Agent 0 · các dòng APPROVED theo QUYET-DINH-A0-2026-11-16b §4 do Agent 1 đối chiếu, không phải Agent 2 (VD-137)
APPROVED 2026-11-16 Agent 0 · theo QUYET-DINH-A0-2026-11-16g §1 D1 · Agent 1 đối chiếu: landlock.rs 13/13 trên ps; 3 ca âm ở tiến trình con (đọc, ghi, read_dir) có đỏ trước và đối chứng dương; macOS --sandbox exit 3
| ST-133 | **R23 + soak 10 + ROADMAP chốt VS2** (VD-131) | tất cả trên | 1,5 h | **DONE** | R23 có đủ số khoá, 5 câu bắt buộc, trạng thái 8 hạng mục theo 3 loại; soak 10/10; check_report PASS |
> CHỜ DUYỆT Agent 0 · R23 + soak **10/10 = 1447 s** · macOS 1153 test/129 suite/0 đỏ, Linux 1147/6 (6 lệch liệt kê ở R23 §4.4, trong đó 1 lỗi sản phẩm: rò rỉ TOCTOU 798/50 000 trên Linux)
ĐÍNH CHÍNH 2026-11-16 Agent 0 · R23 chỉ chốt đợt 23, không chốt VS2 (VD-143). Hai câu sai trong R23: (1) ST-132b không phải "Agent 2 tự thêm" mà theo QUYET-DINH-A0-2026-11-16d §4.5, Agent 1 dán, chủ trương đã duyệt; nợ #131 đóng. (2) Nợ #126 unicode-ident đã có đáp án theo QD §9.3 + VD-33, phụ thuộc build-time; đóng. R24 mang mục "Đính chính R23" (VD-142).
APPROVED 2026-11-16 Agent 0 · theo QUYET-DINH-A0-2026-11-16g §1 D6 · R23 duyệt như báo cáo đợt 23, KHÔNG chốt VS2 (VD-143); 2 câu sai đã đính chính tại QUEUE và R24 §1 (VD-142)
| ST-134 | **Vá runner + ca theo nền**: rustup target ghi vào script tái lập trong viscript/infra/linux/ (VD-136); 2 ca theo VD-132; điều tra ca (iv) 1 h | ST-131 | 2 h | **DONE** | digest mới trong log; 2 ca có nhánh nền khẳng định giá trị cụ thể, diff trong báo cáo; ca (iv) có kết luận 1 trong 3 loại hoặc kế hoạch; macOS gate PASS |
> CHỜ DUYỆT Agent 0 · `6e6673bd` (ST-134) · `vis-targets` **18/18** trên `ps` sau `infra/linux/setup.sh` (cài **2** target: `wasm32-unknown-unknown` + `x86_64-pc-windows-msvc`; sync thêm `vis-host-wasm/`); `pipeline` 16/16, `sandbox` 6/6, `vish_cmd_body` 16/16 **cả macOS và `ps`**; ca (iv) = **(b) lỗi sản phẩm 68 dòng** đã sửa (phân loại thân `cmd` một dòng, `ls /tmp/...` không còn bị đọc thành phép chia `tmp`), đỏ-trước tách bạch bằng binary cũ trên `ps`; 6 ca đỏ Linux phân loại đủ ở `JOURNAL/st134-runner-ca-nen.md` §4; mac `gate --fast` **PASS** (L0 **1159/0**), `check_report --all` PASS; log+sha256 trong JOURNAL
APPROVED 2026-11-16 Agent 0 · theo QUYET-DINH-A0-2026-11-16g §1 D1 · Agent 1 đối chiếu: setup.sh trong repo; 2 ca theo VD-132 có nhánh nền khẳng định giá trị cụ thể; ca (iv) lỗi sản phẩm sửa 68 dòng (≤150), vish_cmd_body 16/16 hai nền
| ST-135 | **Chạy lại đầy đủ trên Linux** sau ST-134 | ST-134 | 1 h | **DONE** | số đo cargo test, gate --fast, host suite trên Linux đặt cạnh macOS; còn đỏ thì liệt kê, không sửa |
> CHỜ DUYỆT Agent 1 · Linux **1161/1** (129 suite) so với R23 1147/6; host **54/54**; targets **18/18**; gate đỏ chỉ 1 ca môi trường (coreutils multicall, nợ #139 — không sửa trong task)
APPROVED 2026-11-16 Agent 0 · theo QUYET-DINH-A0-2026-11-16g §1 D1 · Agent 1 đối chiếu: chỉ đo; Linux 1161/1, host 54/54, targets 18/18, gate 10/11; ca đỏ duy nhất là môi trường, xử lý ở ST-140
| ST-136 | **Đo trên Windows (psvn) qua SSH**, chỉ đo theo VD-136 | ST-135 | 2 h | **DONE** | log môi trường đủ mục VD-136; số đo cargo test và host suite đặt cạnh macOS/Linux; mỗi ca lệch: tên ca, đầu ra mong đợi, đầu ra Windows, nhóm nguyên nhân phỏng đoán (đường dẫn / CRLF / tiến trình / quyền / khác), ghi rõ "phỏng đoán"; không có diff code ngoài infra/ và JOURNAL; thiếu Build Tools thì BLOCKED-ENV kèm hướng dẫn cho người dùng |
Ghi chú Agent 0 2026-11-16: psvn qua SSH tailscale đã sẵn, có Rust MSVC + Build Tools; cập nhật theo VD-140 (rustc cùng phiên bản ps, Build Tools chỉ khi lỗi, không reboot).
> CHỜ DUYỆT Agent 1 · 042cf378 (ST-136) · Windows (`psvn`, rustc **1.99.1** khớp `ps`) @ **`632e1fba`**: **cargo test 1051/102** (129 suite, 100 suite xanh; 102 tên ca đỏ + hai đầu ra + nhóm phỏng đoán trong `JOURNAL/st136-windows.md` §4), **host suite 39/54**, **`vis-targets` 8/18**; **`gate --fast` KHÔNG qua**: tại `632e1fba` dừng ở **`clippy`** (`crates/vis-cli/tests/sandbox.rs:61` — ST-134 để `code`/`err` không dùng khi hai nhánh `cfg(target_os)` bị loại trên Windows; chỉ hiện trên Windows), tại `e369698a` dừng ở self-test fixture (`python3.12` đọc ống bằng `cp1252` ⇒ `UnicodeDecodeError`, lỗi script kiểm); 91/102 lệch thuộc nhóm **đường dẫn** (thiếu `/bin/*`, `\` đi vào literal/URI/JSON, dạng đường dẫn JSON/TAP), 5 ca **CRLF** do `python3` text mode, 1 ca **môi trường** (thiếu std `wasm32`), **không ca nào lệch logic ngôn ngữ** (phỏng đoán); **không sửa mã/kỳ vọng/cfg**, không cài gì (1.99.1 đã có sẵn từ 2026-10-31; Build Tools không cần cập nhật vì build/link chạy), **không reboot**; **lưu ý**: `stable` bị rustup tự cập nhật **1.96.0 → 1.100.0** khi chạy `rustup -V` ở gốc repo (ngoài ý muốn, không ảnh hưởng số đo — mọi lệnh đo ghim `RUSTUP_TOOLCHAIN=1.99.1`); script tái lập `infra/windows/setup.ps1` (đã chạy thử `-Check` ⇒ exit 1 đúng thiết kế); Windows vẫn **"chưa hỗ trợ"** (VD-121); đề nghị Agent 0 quyết: cho phép `rustup target add wasm32-unknown-unknown x86_64-unknown-linux-gnu`, sửa 1 dòng `doc_examples.py` (encoding), xử lý `sandbox.rs:61`, và các nhóm ca `/bin/*` + escape đường dẫn (VS3)
APPROVED 2026-11-16 Agent 0 · theo QUYET-DINH-A0-2026-11-16g §1 D1 · Agent 1 đối chiếu: chỉ đo; log môi trường đủ VD-136, rustc khớp ps; 102 ca lệch có tên, hai đầu ra, nhóm phỏng đoán; không sửa code hay kỳ vọng
| ST-137 | **Sửa rò rỉ TOCTOU đường đọc trên Linux** (VD-141) | ST-135 | 3 h | **DONE** | JOURNAL/st137-phantich.md có cửa sổ thật + 1 lần lọt tái hiện; đỏ trước 798/50 000 trên ps có log; sau sửa 0/50 000 trên ps và macOS; ca /proc không đọc được ⇒ từ chối; host suite 54/54 hai nền; diff 100k lech=0; không unsafe, không crate mới; hướng ưu tiên không đóng được ⇒ kế hoạch + BLOCKED |
> CHỜ DUYỆT Agent 1 · e47cfb2f (ST-137) · **cửa sổ thật**: `quyen.rs:458` canonicalize trả **CHUỖI** + `:465` metadata theo **TÊN** + `:469` quyết định trên chuỗi cũ (chứng minh tất định: JOURNAL/st137-cuaso-that.log `dcf8b87a…`); **đỏ-trước 92/50 000** trên `ps` (R23 đo **798/50 000** trên cùng commit — tỉ lệ phụ thuộc lịch trình, không phải hằng số; JOURNAL/st137-red-ps.log `70f6e07f…`); **SAU: 0/50 000 + `ok_trong > 0` trên CẢ `ps` (34 142) và macOS (13 852)**; **4 ca mới**: handle ra ngoài root ⇒ chặn **tất định**, đọc từ **handle** khi tên đã bị đổi, `/proc` không đọc được ⇒ **VS0604**, host báo đúng đường dẫn handle; host suite **54/54** trên **cả hai nền**; diff 100k **lech=0**; `gate --fast` **PASS** (worktree sạch tại **HEAD đã push `a85193bd`**, L0 **1159/0**; lần 1 trên `1097a81c` cũng PASS, L0 1155/0); report-check PASS; **không `unsafe`, không crate mới**; **macOS giữ bước xác nhận ST-124** vì không có `/proc` và std an toàn không có fd→path (`/dev/fd` `readlink`=EINVAL; `fcntl(F_GETPATH)` cần `libc`/`unsafe`) ⇒ **nợ mới #132** |
APPROVED 2026-11-16 Agent 0 · theo QUYET-DINH-A0-2026-11-16g §1 D1 · Agent 1 đối chiếu: cửa sổ thật có phân tích; đỏ trước 92/50 000 trên ps; sau 0/50 000 trên ps (2 lần) và macOS; 4 ca mới gồm /proc không đọc được ⇒ từ chối; không unsafe, không crate mới; host 54/54 hai nền; diff 100k lech=0. macOS bảo đảm theo đo, không theo cơ chế (VD-145)
| ST-138 | **R24 + chốt VS2** (VD-131, VD-142, VD-143) | ST-130, ST-132b, ST-134, ST-135, ST-136, ST-137 | 1,5 h | **DONE** | danh sách 6 hàng phụ thuộc kèm trạng thái ở đầu báo cáo; mục "Đính chính R23"; 5 câu bắt buộc VD-131; 8 hạng mục theo 3 trạng thái; Linux và Windows mỗi nền một mục số đo; soak 10/10; check_report PASS |
> DONE 2026-11-16 · lật theo QUYET-DINH-A0-2026-11-16g §5.2: 6 hàng phụ thuộc APPROVED, gate --fast tại 69112900 PASS (L0 1159/0, log JOURNAL/st138b-gate-fast.log) · R24 chốt VS2 kỹ thuật 7/8 hạng mục; hạng mục 7 Linux "chờ" (VD-148)
> ĐÃ XONG PHẦN VIỆC · R24 viết xong (soak 10/10 = 1352 s) · **chưa lật DONE** theo VD-143: chờ Agent 1 đối chiếu 6 hàng phụ thuộc rồi dán APPROVED
ĐÍNH CHÍNH 2026-11-16 Agent 0 · R24 §4 ghi hạng mục 7 "Linux chạy thật" trái VD-134 (Linux 1161/1, gate 10/11). Trạng thái đúng: Linux "chờ" tới ST-140 (VD-148). R24 JSON ghi ST-135 READY là đúng thiết kế check_report (đối chiếu QUEUE tại head báo cáo); R24 MD §0 ghi DONE. R24 thiếu hash đo cho macOS và Linux. Cả ba đính chính trong R25 (VD-142).
APPROVED 2026-11-16 Agent 0 · theo QUYET-DINH-A0-2026-11-16g §2.2 · Agent 1 đối chiếu: 6 hàng phụ thuộc đều APPROVED; gate --fast tại HEAD sau R24 PASS L0 1159/0; R24 chốt VS2 về kỹ thuật cho 7/8 hạng mục, hạng mục 7 Linux "chờ" (VD-148)
| ST-139 | **`vis test --timeout` không báo quá hạn oan khi test in nhiều** (nợ #89, issue #23, PR #24) | ST-105 | 1 h | **DONE** | 4 ca đỏ-trước đúng lý do (200 KB stdout, 200 KB stderr, in nhiều rồi fail, cháu giữ ống) rồi xanh; 5 ca ST-105 giữ nguyên; TAP/`--json` không đổi; không crate mới, không `unsafe`, không `#[ignore]`, không nới ngưỡng; `gate --fast` PASS |
> DONE Agent 0 2026-11-16 · commit `ef6489d1` (PR #24 của khoajunior — Agent 0 đọc, gắn mã ST-139, merge) · đỏ-trước trên mã cũ: unit `stderr_nhieu_khong_bi_qua_han_oan` «báo quá hạn oan sau 3.115 s», `chau_giu_ong_khong_lam_treo` 10.088 s ≥ 3 s; tích hợp `in_nhieu_khong_bi_qua_han_oan` «not ok 1 - in_nhieu / quá hạn 3000 ms», `in_nhieu_roi_fail_bao_dung_loi` «báo quá hạn oan» (`JOURNAL/st139-do-truoc.log`) · sau sửa **3/3 unit + 7/7 tích hợp xanh** · `gate --fast` **PASS** (129 suite, **1158 test, 0 đỏ**; `JOURNAL/st139-gate-fast.log`) · Agent 0 thêm `#[cfg(unix)]` cho 3 mục dùng `/bin/sh` để cổng không đỏ trên Windows (ST-136) · ngoài phạm vi: nợ #88 (kill cây cháu)
> Hàng đợt 24g theo QUYET-DINH-A0-2026-11-16g §4. **ĐÍNH CHÍNH ID (Agent 1 2026-11-16, ghi theo VD-147(2))**: §4 đánh số hàng gate là **ST-139**, nhưng ST-139 đã dùng cho PR #24 (hàng ngay trên, DONE). Hàng gate đổi thành **ST-138b** (bước xác nhận của ST-138); nội dung, ngân sách, tiêu chí giữ nguyên văn; ST-140/141/144 phụ thuộc ST-138b. Agent 0 đổi lại được ở tệp quyết định kế tiếp.
| ST-138b | **Gate tại HEAD sau R24** (16g §2.2) | ST-138 | 0,5 h | **DONE** | ./vis-dev gate --fast tại HEAD có R24; dòng cuối + số L0 dán dưới hàng ST-138; đỏ ⇒ XEM LẠI, không tự lật ST-138 |
> CHỜ DUYỆT Agent 1 · 69112900 (ST-138b) · gate --fast PASS, L0 1159/0; 10 bước OK; report-check PASS (24 báo cáo, 164 hàng QUEUE); log JOURNAL/st138b-gate-fast.log sha256 1c6e3d18b4eeb435a4a38cad09138c39169322ec85010590cc08f51916cc58d6; không retry
APPROVED 2026-11-17 Agent 0 · theo QUYET-DINH-A0-2026-11-16g §2.2 · Agent 1 đối chiếu: gate --fast tại HEAD sau R24 PASS, L0 1159/0, 10/10 bước, report-check 24/164; log st138b-gate-fast.log sha256 1c6e3d18…
CHUẨN THUẬN 2026-11-17 Agent 0 · ID ST-138b thay cho ST-139 trong QUYET-DINH-A0-2026-11-16g §4 (ST-139 đã dùng cho PR #24); nội dung/ngân sách/tiêu chí giữ nguyên văn; các hàng phụ thuộc dùng ST-138b
| ST-140 | **Ca coreutils theo VD-144** | ST-138b | 1,5 h | **DONE** | đỏ trước trên ps có log; sau sửa: ca xanh trên ps và macOS; cargo test --workspace trên ps 0 đỏ; gate --fast trên ps PASS 11/11; diff ca đúng một tệp test |
> CHỜ DUYỆT Agent 1 · 02fb7320 (ST-140) · **đỏ-trước trên `ps`** (hash `9e616960`): ca `symlink_canonicalize_hai_phia` **FAILED** — `c.vish:5:1: error VS0501: chương trình ngoài …/ln-echo thoát mã 1: Security violation: Requested utility `ln-echo` does not match executable name: /usr/lib/cargo/bin/coreutils/echo` (cargo test exit **101**; `JOURNAL/st140-red-ps.log` sha256 `3b7987c04072b7fb808851a325a232e05e777576f77308c288681520b375b8de`) · **sửa đúng VD-144**: đích symlink `/bin/echo` → **`/bin/sh`** chạy `-c "echo a"` (gói `vis-cli` chỉ lộ **một** binary qua `CARGO_BIN_EXE_` là `vis`; `vis` không in lại đối số tất định, không có `--version` ⇒ nhánh `/bin/sh -c` của VD-144; trên `ps` `/bin/sh` = `/usr/bin/dash`, **không** multicall); giữ **hai chiều** (grant `p=/bin/sh` + chạy qua symlink; grant `p=<symlink>` + chạy `/bin/sh` thật), giữ stdout `[a]\n`/`[b]\n` và parity interp/`--vm`; **không** đụng ca `sao_chep_trung_ten_bi_chan` hay ca nào khác; chú thích ca ghi lý do + VD-144 · **SAU: xanh hai nền** — macOS `grant_proc_canon` **7/7**; `ps` `cargo test --workspace` **1162 passed / 0 failed** (129 suite, exit 0; ST-135 đo **1161/1** — đúng ca này đỏ; `JOURNAL/st140-cargo-test-ps.log` sha256 `733a77b8bc9b64dd3550de2ce69b78efae2fc9bd4246279823ec6a92b4857219`) · `gate --fast` **PASS cả hai nền**: macOS **L0 1159/0** (`JOURNAL/st140-gate-macos.log` sha256 `62a25edeee8e0308bc3fd2684dff3945010baa00272d39dc7e9bcd49dffe3b63`), `ps` **L0 1162/0**, 12/12 dòng `--- …: OK`, exit 0 (`JOURNAL/st140-gate-ps.log` sha256 `647c925be791f17cce4e70207bff173e0dd6c3d1270663865e567b3e0bbdc627`) · **diff đúng một tệp test** (`crates/vis-cli/tests/grant_proc_canon.rs`, +20/−5) · **bất thường**: lần 1 lệnh gate `ps` chép nguyên văn kế hoạch (`cd viscript && ../vis-dev`) trả **127** vì `viscript/bin/vis-linux` chỉ rsync `viscript/`, `docs/viscript/`, `vis-host-linux/`, `vis-host-wasm/`, `.git/`, `AGENTS.md` — shim gốc repo `<repo>/vis-dev` **không có** trên `ps`; **không** phải cổng đỏ, đã chạy lại **một** lần bằng `bin/vis-dev`; log lần 1 giữ nguyên (`JOURNAL/st140-gate-ps-lan1-sai-duong-dan.log` sha256 `9c1272114295588394970aef9389a459ae5d13651c8f2fdf24ef10469970507c`)
APPROVED 2026-11-17 Agent 0 · theo QUYET-DINH-A0-2026-11-16g §7 · Agent 1 đối chiếu: đỏ-trước trên ps (exit 101, coreutils multicall); sửa đúng 1 tệp test theo VD-144 (hai chiều giữ nguyên); ps cargo test 1162/0 + gate PASS (L0 1162/0); macOS gate PASS 1159/0
| ST-141 | **Log không ghi đè** (VD-147 (5)): vis-soak nhận VIS_SOAK_LOG/--log, mặc định tên theo ID task; khôi phục st113-soak.log từ 36ed5922; tách soak đợt 24 sang st138-soak.log; vis-host-suite in nền thật (uname -sm) | ST-138b | 1 h | **DONE** | git show 36ed5922:<đường dẫn> trùng byte với st113-soak.log đã khôi phục; st138-soak.log trùng byte nội dung ở a9f80d83; chạy vis-host-suite trên ps in "Linux"; không xoá log nào |
> CHỜ DUYỆT Agent 1 · `f447f1b0` (ST-141) · **log không ghi đè (VD-147(5))**: `bin/vis-soak` nhận `--log <tệp>` > `VIS_SOAK_LOG` > mặc định `agent/JOURNAL/st<VIS_TASK_ID>-soak.log` (không có ID ⇒ `soak-<YYYYmmddTHHMMSSZ>.log`); tệp đích đã có và khác rỗng ⇒ **exit 2**, 2 dòng stderr, **không truncate**; `--append`/`VIS_SOAK_APPEND=1` mới ghi tiếp; `--dry-run` in đúng 10 tên vòng, không ghi gì (đỏ-trước: bản cũ xoá 54 byte → 0 byte dù đặt `VIS_SOAK_LOG`, `JOURNAL/st141-red-overwrite.log` sha256 `7d2cade2b340518dfd8bc153e678f0f40de5a0d78bec27efba837acb9fd333bf`) · **khôi phục + tách log**: `st113-soak.log` = `d2840412c1b663ae99a85ac6db7c49ee1ac08cd7c896a98a8cd3ee691ca49c98` (trùng byte `git show 36ed5922:…`), `st138-soak.log` = `d79df3c2cf25f6ad6d37ba2cf418273d3fa8375b1935a8e6b2dacfaf610628f0` (trùng byte `git show a9f80d83:…`), không xoá log nào (`JOURNAL/st141-khoi-phuc-log.log` sha256 `3fce19d970a5494ad9834f1f89c098a2e099db6b996b761c8c3cbc3e6cd18250`) · `cargo test -p vis-cli --test soak_script` **1 passed / 0 failed** (đúng 10 vòng) · `vis-host-suite` in nền thật `uname -sm`: macOS **54/54** `(nền: Darwin arm64)` (`JOURNAL/st141-host-suite-macos.log` sha256 `60e3cc70…`), `ps` **54/54** dòng cuối `vis-host-suite: 54/54 ca xanh (nền: Linux x86_64; các nền khác chưa đo)`, exit 0 (`JOURNAL/st141-host-suite-ps.log` sha256 `6c1e985d…`, chạy qua `bin/vis-linux run`, sha256 script trên `ps` trùng macOS `1bcc8df0…`) · macOS `gate --fast` **PASS** L0 **1159/0** (129 suite), report-check PASS (24 báo cáo, 164 hàng QUEUE), doc-examples OK (24 khối) (`JOURNAL/st141-gate-macos-lan2.log` sha256 `d62cb72ed5218a0981191fc646cbedf743a6bcd5d0964c63d7f6f1f3411b20de`); lần 1 đỏ **T-INFRA** `vistest::tests::stderr_nhieu_khong_bi_qua_han_oan` (stderr về 0/220000 byte trong L0 đủ tải; chạy riêng **5/5 xanh**), giữ nguyên log `JOURNAL/st141-gate-macos.log` sha256 `9e0fe26d…`, chạy lại **một** lần theo D2-07 để phân loại, không sửa ngưỡng/ca · chỉ 2 tệp script + log/tài liệu, không đụng `crates/`
APPROVED 2026-11-17 Agent 0 · theo QUYET-DINH-A0-2026-11-16g §7 · Agent 1 đối chiếu: st113-soak.log = d2840412…, st138-soak.log = d79df3c2… (byte-identical git show); vis-soak cấm ghi đè (exit 2) + nhận --log/VIS_SOAK_LOG; vis-host-suite in nền thật, ps 54/54
| ST-142 | **Số Linux đầy đủ** (nợ #130): vis test test/sem interp và --vm, diff 100k seed 20261115, cache, trên ps | ST-140 | 1,5 h | **DONE** | 447/447 cả hai đường, 0 SKIP; lech=0; số đặt cạnh macOS kèm hash commit; lệch thì liệt kê, không sửa |
> CHỜ DUYỆT Agent 1 · `ee58a518` (ST-142) · **cùng một HEAD trên cả hai nền**: macOS `ee58a518` (`git rev-parse HEAD`), `ps` `ee58a518` (`git rev-parse HEAD` trong `/root/vifast-linux` **sau** `vis-linux sync`) · **macOS**: `vis test test/sem` **447 ok / 0 not ok / 0 SKIP** (6 s; `JOURNAL/st142-mac-vis-test.log` sha256 `2667692dba0ef8b9a77c3a5d203cfa91a35b24427982d208e4b245e59ac1570c`) · `--vm` **447 ok / 0 not ok / 0 SKIP** (6 s; `st142-mac-vis-test-vm.log` `c71bf764667c00f2e65fcdd57e54412984ae5d0e164513005235e51c90ac8c12`) · diff 100k seed `20261115` **lech=0** (157,4 s, 635/s, `loi_tinh=6230`; `st142-mac-diff-100k.log` `f5842c13a06e47920d1c21f4c9f4b19b4cf0f22d9dcb701f4450d029859c7915`) · cache 520×4 **5 passed / 0 failed, 0 lệch** (25,43 s; `st142-mac-cache.log` `d942e2228e09dcc72c78e0cc2453e26a6dab6c11da4b0db60cfb200b4d6a6e66`) · **ps** (Linux 7.0.0-43-generic, Ubuntu 26.04.1, rustc/cargo `1.99.1` tại `/root/.cargo/bin`): `vis test` **447/0/0 SKIP** (4 s; `st142-ps-vis-test.log` `0f6d717883c0aa7b2cef78ab845ee285c281559990b1a8aef906668ef6673663`) · `--vm` **447/0/0 SKIP** (4 s; `st142-ps-vis-test-vm.log` `88a7bc6adedb646e1b6fe787f4e4bc83e265112982a09440a43d356cae9b9b8b`) · diff 100k **lech=0** (40,9 s, 2448/s, `loi_tinh=6230`; `st142-ps-diff-100k.log` `2b88adec765da11a41563955a7335391f74c63516b9766d61f9b94a511630722`) · cache 520×4 **5 passed / 0 failed, 0 lệch** (6,33 s; `st142-ps-cache.log` `5e9d551220057a70b6b65813e7500654aa97dddfbd7b62d226b897fb43b74451`) · build `ps` `st142-ps-build.log` `6eae1bdfa070119d5f956448eb300167801c0e4d1d3b1941a2703b1c57d7201a` · **lệch đã liệt kê**: chỉ **thời gian** — diff 157,4 s vs 40,9 s, cache 25,43 s vs 6,33 s (~3,9–4×; cùng 93770 ca chạy, chi phí spawn tiến trình mỗi nền; `loi_tinh` **trùng** 6230 ⇒ cùng bộ sinh ca), `vis test` 6 s vs 4 s (log `ps` gồm ~2 s sync+setup); **không lệch số đúng/sai, không SKIP nền nào**, không sửa mã/test/kỳ vọng/ngưỡng · gate macOS **PASS** `exit 0`, L0 **1159/0** (129 suite), report-check PASS (24 báo cáo, 164 hàng QUEUE), doc-examples OK (24 khối), **10/10** dòng `--- …: OK`, 295 s (`st142-gate-macos.log` `676f1ee24db86415560a872949398241ef1b3a07500163cc59428d2ea338e0c3`) · không chạy gate trên `ps` (đã có ST-140); shim `<repo>/vis-dev` vẫn không sync
APPROVED 2026-11-17 Agent 0 · theo QUYET-DINH-A0-2026-11-16g §7 · Agent 1 đối chiếu: cùng HEAD ee58a518 hai nền — vis test 447/447 ×2 đường 0 SKIP, diff 100k lech=0, cache 520×4 0 lệch; lệch chỉ là thời gian (3,85–4,02× do spawn tiến trình)
| ST-143 | **Soak 10 trên ps** | ST-142 | 1 h | **DONE** | log st143-soak.log; 10/10 hoặc liệt kê vòng đỏ + phân loại T-INFRA (chạy lại tối đa 1 lần chỉ để phân loại) |
> CHỜ DUYỆT Agent 1 · `2866f8c2` (ST-143) · **shim**: `sync_tree` thêm 2 dòng `rsync -a` cho hai shim gốc (`vis-dev`, `vis`), không `--delete`, không nới quyền (`e4062cd6`); trên `ps`: `cd viscript && ../vis-dev rules` **exit 0**, in `--- rules: OK`, sha256 hai phía trùng — `vis-dev` `53707e34…`, `vis` `05d97c4f…`, `bin/vis-linux` `60b01e49…` (`JOURNAL/st143-shim-ps.log` sha256 `94e3ee6363225ebe9e1d0967611e22ffc4af0a197c987697080c6d9735085cd9`) · **soak 10/10 = 250 s trên ps** ở HEAD `2866f8c2` (`head_do` = `head_host`, đo sau `vis-linux sync`); `JOURNAL/st143-soak.log` **228281 byte**, sha256 **`faf765e1332fdc38a4096aa5ca52c9625fc659d8a6a36f185ca17dad165ebdbe` khớp hai phía** (local = host; `ssh cat` thu về không sửa byte); các vòng: gate-1/2/3 **38/38/38 s** (L0 **1162/0** mỗi vòng, 129 suite, report-check PASS ×3), diff-1/2/3 **41/40/41 s** (**lech=0**, 93770 ca chạy, 2455–2462 ca/s), host-1/2 **1/0 s** (**54/54**, nền Linux x86_64), cache-1/2 **7/6 s** (5 passed / 0 failed) · **lần 1 đỏ — T-INFRA, không phải flake**: dùng đúng giao diện `VIS_SOAK_LOG=…` thì dừng ở **gate-1 sau 31 s** exit 1 (L0 **1161/1**) vì biến rò vào `cargo test` ⇒ ca `soak_script` gọi lại `vis-soak --dry-run` thấy tệp log đang ghi ⇒ nhánh chống ghi đè VD-147(5) exit 2; **sửa gốc** ở `2866f8c2` (`unset VIS_SOAK_LOG VIS_SOAK_APPEND`; `--dry-run` vẫn in đúng 10 tên, `soak_script` **1 passed / 0 failed**), rồi chạy lại **một** lần; log lần 1 giữ nguyên `JOURNAL/st143-soak-lan1-gate-1-do.log` sha256 `f3500cad30b312cbf08497fb3b5ab1fc6095755ddeadafe06af6c691f1b0032c` · log môi trường VD-136 `JOURNAL/st143-soak-env.log` sha256 `9adf2e31b1baa38dcd4f3f9cf8ca085ed16f1fe9de03ab58a35c1744a851ff81` · gate macOS **PASS** `exit 0`, L0 **1159/0** (129 suite), **10/10** dòng `--- …: OK`, report-check PASS (24 báo cáo, 164 hàng QUEUE), doc-examples OK (24 khối), 289 s (`JOURNAL/st143-gate-macos.log` sha256 `718af70e750aaff8726fde5bbe1097f5d2db57d23cef702ec64d779feaa5e40a`) · chỉ tệp `viscript/`: `bin/vis-linux`, `bin/vis-soak`, log + sổ; **không** đụng `crates/`, `spec/`, `docs/`, `v2/`
APPROVED 2026-11-17 Agent 0 · theo QUYET-DINH-A0-2026-11-16g §7 · Agent 1 đối chiếu: soak 10/10 = 250 s trên ps; log sha256 local = host; shim vis-dev/vis đã sync (../vis-dev chạy được trên ps); VIS_SOAK_LOG không rò (unset 2 dòng)
| ST-144 | **Windows: 3 việc VD-146 + đo lại** | ST-138b | 2,5 h | **DONE** | target add có log trước/sau; doc_examples.py 1 dòng; sandbox.rs tối thiểu; đo lại cargo test, host suite, vis-targets, gate --fast trên psvn; số đặt cạnh ST-136; KHÔNG sửa ca lệch nào khác |
> CHỜ DUYỆT Agent 1 · `3dc13342` · target **2/2 đã cài** (log trước/sau: `target-truoc=x86_64-pc-windows-msvc` → `target-sau=wasm32-unknown-unknown,x86_64-pc-windows-msvc,x86_64-unknown-linux-gnu`; `C:\vis-setup-st144.log` sha256 `a8e8f7c0…` = bản sao `JOURNAL/st144-setup-psvn.log`); **gate Windows: dừng bước 5/10** — `self-test case âm` **7/12** (7 fixture `doc-examples` "bị bắt nhưng sai lý do" vì stderr `doc_examples.py` trên Windows là **cp1252 + backslashreplace**, chuỗi kỳ vọng UTF-8 ⇒ lệch byte; **không còn `UnicodeDecodeError`**; sửa tiếp ngoài 3 việc VD-146 ⇒ VS3), trước đó `fmt`/`clippy`/`luật R1..R4` **OK** (hết lỗi `sandbox.rs:61` của ST-136); **cargo test 1053/100** (ST-136: 1051/102; +`targets_wasm_vis_types` nhờ std wasm32, +`treo_bi_giet_trong_3s` ca thời gian không sửa gì); **host 39/54** (đúng 15 tên như ST-136); **targets 18/18** (ST-136: 8/18); `doc_examples.py` +2 dòng `encoding="utf-8"` và `sandbox.rs` +5 dòng (1 khối `cfg(not(any(macos,linux)))`) sửa tối thiểu; **không sửa ca lệch nào**; gate macOS **PASS** L0 **1159/0** (`st144-gate-macos.log` `c0d71639…`); đo trên `psvn` @ `3dc13342` (cây sạch), 4 log khớp sha256 hai phía; không cài gì khác, không reboot |
APPROVED 2026-11-17 Agent 0 · theo QUYET-DINH-A0-2026-11-16g §7 · Agent 1 đối chiếu: VD-146 đúng 3 việc tối thiểu (không đổi kỳ vọng/ca); psvn targets 18/18 (từ 8/18), cargo test 1053/100, host 39/54 (15 ca trùng ST-136), clippy hết đỏ; gate Windows dừng bước 5/10 (cp1252 — VS3); Windows vẫn "chưa hỗ trợ"
| ST-145 | **Sổ nợ duy nhất** (VD-147 (3)) | ST-141 | 2 h | **DONE** | viscript/agent/SO-NO.md gom mọi nợ từ R1..R24 + JOURNAL; mỗi số một nghĩa; #109 tách #109a/#109b; mỗi mục có nguồn; nợ VS3 đánh dấu rõ; check_report PASS |
> CHỜ DUYỆT Agent 1 · 4f1c4d9f (ST-145) · `viscript/agent/SO-NO.md` **142 mục** (**92 mở** / **28 đóng** / **20 VS3** / **1 không làm** / 1 điều hướng #141); **trùng số 0**; **#109a/#109b tách** (#109a = `next_block` ST-128 → VS3, #109b = đường ghi, mở) + 2 nợ đợt 23 chưa có số chuẩn ghi ở §7.3 (`#113b`, `#114b`); trích **R1..R24** (22 báo cáo có bảng/danh sách nợ; **R1, R20 không có dòng đánh số**) + JOURNAL đợt 23 + QUEUE/DECISIONS để chốt trạng thái; `check_report --all` **PASS** (24 báo cáo, 164 hàng QUEUE); `gate --fast` macOS **PASS** **L0 1159/0** (129 suite), 10/10 dòng `--- …: OK`, 316 s; log `JOURNAL/st145-trich-no.log` (sha256 `f5047551…`), `st145-kiem-trung.log` (PASS, 0 trùng), `st145-gate-macos.log` (sha256 `8767e547…`, `[exit=0]`); **không** sửa báo cáo R1..R24, không đụng mã/test/kỳ vọng/ngưỡng, không `unsafe`, không crate mới |
APPROVED 2026-11-17 Agent 0 · theo QUYET-DINH-A0-2026-11-16g §7 · Agent 1 đối chiếu: SO-NO.md 142 mục (mở 92/đóng 28/VS3 20/không làm 1/điều hướng 1), trùng số 0, #109a/#109b tách, mỗi mục có nguồn
| ST-146 | **R25** | ST-140..ST-145 | 1,5 h | **DONE** | đầu báo cáo: danh sách hàng phụ thuộc + trạng thái (VD-143); mục "Đính chính R24" (hạng mục 7 Linux, ST-135 JSON/MD, hash từng nền); hash đo từng nền; hạng mục 7 cập nhật theo kết quả ST-140; 5 câu VD-131; nợ trích theo SO-NO.md; check_report PASS |
> CHỜ DUYỆT Agent 1 · `43b19dfe` (ST-146) · **R25 xong**: `reports/R25-viscript.{md,json,json.license}`; `check_report --all` **PASS (25 báo cáo, 164 hàng QUEUE)**; gate macOS `--fast` **PASS** `[exit=0]` **L0 1159/0** (129 suite), **10/10** dòng `--- …: OK`, 328 s, chạy **một** lần (`JOURNAL/st146-gate-macos.log` sha256 `47b40553…`); **sổ nợ 142 mục** (mở **92**/VS3 **20**) trích theo `SO-NO.md`, không trích `#113b`/`#114b` bằng số; **hash đo từng nền** (macOS `43b19dfe`, `ps` `02fb7320`/`ee58a518`/`2866f8c2`, `psvn` `3dc13342`); **hạng mục 7 Linux "chạy thật"** (ST-140: `cargo test 1162/0` + gate PASS trên `ps`), Windows chỉ đo (VD-146), wasm chỉ check (VD-121); **đính chính R24 đủ 3 điểm** (VD-142); **5 câu VD-131** nguyên văn; T-INFRA 3 lần ghi đủ (ST-141 flake, ST-143 rò env, ST-140 sai đường dẫn) — không retry lấy xanh

APPROVED 2026-11-17 Agent 0 · theo QUYET-DINH-A0-2026-11-16g §7 · Agent 1 đối chiếu: R25 đủ 7 hàng phụ thuộc + 3 điểm đính chính R24 + 5 câu VD-131 + hash từng nền; check_report PASS (25 báo cáo, 164 hàng); gate macOS PASS L0 1159/0
| ST-147 | **Windows gate bước 5**: đặt UTF-8 cho mọi lần gọi python của cổng (VD-150) | ST-144 | 1,5 h | **DONE** | gate macOS PASS (1159/0, cùng số bước) ✔; trên psvn: self-test **11/12** (chưa 12/12), cổng **dừng ở bước self-test**, chưa đi qua bước 5; không sửa ca lệch ✔; diff chỉ script ✔; log st147-* ✔ |
> ĐO (ST-147, chưa lật DONE) · `fd16d7d1` (hạ tầng) · **macOS**: `gate --fast` **PASS** `[exit=0]`, **10/10** bước, L0 **1159/0** (129 suite), report-check PASS (25 báo cáo, 170 hàng QUEUE), doc-examples OK (24 khối), 306 s, một lần — `JOURNAL/st147-gate-macos.log` sha256 `ba3b9dd37e997e04523de2302022db62cfc469696e9c2c4cb962eff6b4851e51` · **psvn** @ `fd16d7d1` (rustc 1.99.1, Python 3.13.3): fmt OK · clippy OK · luật R1..R4 OK · **self-test case âm 11/12 FAIL** ⇒ dừng ở bước self-test (5/10), `exit=1`, 98 s — `JOURNAL/st147-gate-psvn.log` 3511 byte, sha256 **hai phía khớp** `25e57182828198e8accdc62f8eb003c1a9156869bf42009cf88e07aa7c94fe46`; log môi trường VD-136 `JOURNAL/st147-psvn-env.log` · **encoding đã xong**: ST-144 đỏ 7 ca (6,7,8,9,10,11,12), nay chỉ còn **ca 7**, 6 ca kia in đúng câu kỳ vọng **bằng UTF-8** · **ca 7 KHÔNG phải encoding**: `crates/vis-cli/src/cli/check.rs:82` tách basename bằng `path.rsplit('/')` ⇒ trên Windows `phu` luôn khác rỗng ⇒ nhánh `--syntax` (dòng 97) **giữ** chẩn đoán kiểu ⇒ khối `x = 1` đỏ ⇒ câu `ngoại lệ cũ: …` không bao giờ in; chứng minh: cùng tệp gọi bằng **tên tương đối** trong cwd ⇒ `ok 1 - lf.vis` rc 0, còn đường dẫn tuyệt đối `C:\…` ⇒ `not ok … # VS0301`; `--syntax` trên Windows không khác gì không có cờ, LF/CRLF như nhau, build lại rc 0 không đổi (`JOURNAL/st147-chan-doan-psvn.log` sha256 `5e2123c27083d8951db2255ff4f0a45adf031be10e53d345070604a062dc613c` = psvn) · **CẦN ĐỂ MỞ**: Agent 0 cấp VD/task VS3 cho **một dòng** sửa `check.rs` (dùng `Path::file_name`/tách cả `'/'` và `'\\'`) — VD-150 chỉ cho hạ tầng cổng, VD-146 cấm sửa sản phẩm cho Windows nên **không sửa**; sau đó ca 7 xanh ⇒ self-test 12/12 và cổng đi qua bước 5 (dự kiến dừng ở doc-examples hoặc L0, ~100 ca đỏ Windows thuộc VS3) · **bất thường**: 4 log untracked `st144-*` trên psvn trùng byte bản đã commit (`b1ee0b33`) chặn `git pull --ff-only` ⇒ đã **chuyển** sang `/c/Users/ps/backup-st144-logs-20261117/` (không xoá) rồi pull `3dc13342..fd16d7d1`; `load` không đo được (Git Bash psvn không có `uptime`) · **không** retry cổng, không sửa ca/ngưỡng/src
ĐÓNG HÀNG 2026-11-17 Agent 0 · ST-147 DONE theo phạm vi VD-150 (UTF-8: Windows 7/12 → 11/12, 6 ca encoding xanh; macOS gate PASS 10/10, L0 1159/0). Ca 7 không phải encoding — lỗi sản phẩm check.rs, chuyển ST-153 (VD-152). BLOCKED cũ của hàng được giải toả bởi việc tách phạm vi này.
APPROVED 2026-11-17 Agent 0 · 17b §1 · ST-147 DONE theo phạm vi VD-150 (Windows 7/12→11/12; 6 ca encoding xanh; macOS gate PASS 10/10 L0 1159/0); ca 7 chuyển ST-153
| ST-148 | **Sổ nợ theo VD-149**: thêm #142/#143 + ánh xạ #113b/#114b; đánh dấu nợ đã đóng trong đợt 24g (#130, #139, #89, #109b còn mở) | ST-146 | 0,5 h | **DONE** | SO-NO.md có #142/#143; trùng số 0; mỗi mục mới có nguồn; check_report PASS; gate macOS PASS |
> CHỜ DUYỆT Agent 1 · `cfce6b86` (ST-148) · **SO-NO theo VD-149**: thêm **#142** (`#113b` — `pkg update <tên>` ghim mọi gói khác) và **#143** (`#114b` — `NamePolicy::Strict` bật cho mọi tệp có `use`), cả hai **mở** (VS2 không sửa); **ánh xạ §7.3** `#113b → #142`, `#114b → #143` — giữ số cũ trong ngoặc, **không tái dùng số**, không đánh lại số; **đóng trong đợt 24g, đối chiếu lại có bằng chứng trong repo**: `#89` (ST-139 · PR #24 · commit `ef6489d1` · `JOURNAL/st139-do-truoc.log`), `#130` (ST-142 · `JOURNAL/st142-ps-vis-test{,-vm}.log` + `st142-ps-diff-100k.log` + `st142-ps-cache.log`: 447/447 hai đường, `lech=0`), `#139` (ST-140 · **VD-144** · `JOURNAL/st140-red-ps.log` + `st140-cargo-test-ps.log`), `#126` (ST-132 · **VD-135**), `#131` (ST-00 · đính chính R23/R24 §1) — đã đúng trạng thái `đóng` từ ST-145, nay **kèm nguồn log**; **giữ mở** `#109b`/`#140` + ghi chú "**dòng giới hạn đã có trong `HOST-BAO-DAM.md` §4**; sửa kỹ thuật thuộc VS3" (không đóng không căn cứ) · sổ **144 mục** (**94 mở** / 28 đóng / 20 VS3 / 1 không làm / 1 điều hướng), dải **1..143** (`#109` tách a/b) · **trùng số 0** + dải + nguồn + ánh xạ + khớp §8: `JOURNAL/st148-kiem-trung.log` (PASS, sha256 `852c2d0632a4a524a90e7976bd5a174ef5fbb30482480d1ca82f069e6ec9816c`) · `python3 viscript/test/report/check_report.py --all` **PASS (25 báo cáo, 170 hàng QUEUE)** (`JOURNAL/st148-check-report.log` sha256 `6789cc76071bc9e959d7df06c79d14c52c8bcaf68f2686269b88fd40b4c41260`) · `cd viscript && ../vis-dev gate --fast` **PASS** `[exit=0]`, **10/10** bước `--- …: OK`, L0 **1159/0** (129 suite), report-check PASS (25/170), doc-examples OK (24 khối), chạy **một lần** (`JOURNAL/st148-gate-macos.log` sha256 `ba80ea698067bba8ab749fbe3d2a11d85ada9b7ec61899b952f6474b52b19114`, header VD-136 + sha256 + `[exit=0]`) · **không** sửa báo cáo R1..R25 (VD-142), không đụng mã/test/kỳ vọng/ngưỡng, không `unsafe`/crate mới/`#[ignore]`; commit chỉ tệp `viscript/` (thay đổi chưa commit của `v2/` giữ nguyên)
APPROVED 2026-11-17 Agent 0 · 17 §6 · Agent 1 đối chiếu: SO-NO #142/#143 thêm, ánh xạ #113b/#114b, nguồn #89/#130/#139; trùng số 0; check_report PASS
| ST-149 | **Nợ #136**: ca khoá phân loại thân `cmd` khi biến TRÙNG TÊN LỆNH (đường thoát `( … )`) | ST-146 | 1,5 h | **DONE** | ≥3 ca: biến trùng tên lệnh dùng như biểu thức; `( … )` là biểu thức; lệnh thật vẫn là lệnh. Nếu hành vi sai: sửa ≤150 dòng hoặc ghi kế hoạch + BLOCKED phần sửa; không đổi kỳ vọng ca cũ |
> CHỜ DUYỆT Agent 1 · `6ce972ef` (ST-149) · **nợ #136 đóng**: `crates/vis-types/src/lower/khai.rs` **+37/−6** — helper `ten_bien_bind_toi` (biến `let`/`var`/`const` cột 1 + tham số `cmd`/`fn`; **không** gồm tên `cmd`/`fn`/`type` hay tên dựng sẵn) và `la_lenh_da_biet` **loại trừ** tên đã bị biến che ⇒ thân là **Script**; không bind ⇒ luật ST-134 nguyên vẹn; **4 ca mới** trong `crates/vis-cli/tests/vish_cmd_body.rs` (20 ca): bind cấp tệp trùng tên lệnh ⇒ `3`; `( … )` ⇒ `3`; **không** bind + `ls <thư mục thật>` ⇒ vẫn in bảng (khuôn `than_lenh_tren_thu_muc_that`); `let` trong thân ⇒ `3` · **đỏ-trước** (HEAD `456e8a4a`): ca 1 in **bảng `ls`** thay vì `3` (`JOURNAL/st149-do-truoc.log` sha256 `63928844…`) · tái hiện sau sửa in **`3`** (`JOURNAL/st149-tai-hien.log` sha256 `6beba733…`) · `cargo test --workspace` **1163/0** (129 suite; 1159 + 4 ca) — `JOURNAL/st149-cargo-test.log` sha256 `96ab2705…` · gate macOS **PASS** `[exit=0]` **10/10** bước, L0 **1163/0** (129 suite), report-check PASS (25 báo cáo, 171 hàng QUEUE), doc-examples OK (24 khối) — `JOURNAL/st149-gate-macos.log` sha256 `ad574c8e…` · **lật ST-147 BLOCKED→DONE** theo `ĐÓNG HÀNG 2026-11-17 Agent 0` (giữ nguyên dòng ghi chú) · không `#[ignore]`/ngưỡng/`unsafe`/crate mới, không đổi kỳ vọng ca cũ; commit chỉ tệp `viscript/`
APPROVED 2026-11-17 Agent 0 · 17 §6 · đỏ-trước in bảng ls thay vì 3; khai.rs +37/−6 (chỉ biến che tên lệnh; cmd/fn/type giữ nguyên); 4 ca; #136 đóng; gate mac PASS L0 1163/0
| ST-150 | **Nợ #133**: ca âm **GHI** cho thư mục chỉ-đọc trong `vis-host-linux` | ST-146 | 1 h | **DONE** | chạy trên ps: con ghi vào thư mục chỉ-đọc ⇒ EACCES (kernel), đối chứng dương ghi trong thư mục cho phép; 13 ca cũ xanh; log st150-* |
> CHỜ DUYỆT Agent 1 · `9eeb936b` (ST-150) · **nợ #133 đóng**: thêm ca **14** `con_ghi_vao_thu_muc_chi_doc_bi_kernel_chan` trong `vis-host-linux/tests/landlock.rs` (**+101** dòng, không sửa ca cũ) — **đỏ-trước** (chưa áp ruleset) cùng helper `ghi` ⇒ `exit=0`, tệp nội dung `truoc ruleset`; **sau ruleset** `ap(&[ChoPhep{trong,ghi:false}, ChoPhep{trong-ghi,ghi:true}])` (2 luật, MỘT ruleset) cùng helper `ghi <trong>/x.txt` ⇒ **`exit=7`**, stderr `loi: Permission denied (os error 13)` = `EACCES`, và `x.txt` **không được tạo**; **đối chứng dương cùng ca** cùng helper `ghi <trong-ghi>/x.txt` ⇒ `exit=0`, nội dung `duoc ghi` ⇒ luật ghi **phân biệt đúng thư mục**, không "chặn tất"; helper gọi thẳng `std::fs::write`, **không** qua `FsGuard` ⇒ **kernel** chặn (VD-120); chạy `root` trên `ps` nên DAC không giải thích được · **`ps` 14/14 xanh** (13 ca cũ vẫn xanh, `test result: ok. 14 passed; 0 failed; 0 ignored`, 0,33 s; kernel `7.0.0-43-generic`, Ubuntu 26.04.1, rustc 1.99.1, Landlock trong `lsm`) — `JOURNAL/st150-landlock-ps.log` (4 699 byte, sha256 `614b124afc8c64a377deb35f3b29c445ae585a3cc14f24bf0e73cee57df9c980`) · **sha256 tệp đo HAI PHÍA khớp**: local `fa2aab0fd1bfd122fb11eee1783adee50639e935b8dde324b6b64d37deaa1461` = `ssh root@ps sha256sum` `fa2aab0f…` · header VD-136 đủ (máy, kernel, OS, rustc `-Vv`, rustup targets, HEAD `8abb0c81` hai phía, lệnh, load `1.38 0.87 0.52`) · gate macOS **PASS** `[exit=0]` **10/10** bước, L0 **1163/0** (129 suite; = mức ST-149, `vis-host-linux` **không** thuộc workspace `viscript/`), report-check PASS (25 báo cáo, 171 hàng QUEUE), doc-examples OK (24 khối) — `JOURNAL/st150-gate-macos.log` sha256 `6aa8b4f835e46b61ffc5e19c8c0f5a3c161768115dc4dec9e9191f5b39f47894` · **không** `#[ignore]`/`cfg`-bỏ/nới ngưỡng/`unsafe`/crate mới, không sửa `src/`, không đụng `spec/`, `docs/`, `v2/`, `DECISIONS.md`; commit chỉ tệp `viscript/` + `vis-host-linux/` |
APPROVED 2026-11-17 Agent 0 · 17 §6 · ca 14 (helper ghi thư mục chỉ-đọc ⇒ EACCES, tệp không tạo; đối chứng dương ghi được); ps 14/14; #133 đóng
| ST-151 | **Nợ #135**: kiểm tự động danh sách sync của `bin/vis-linux` | ST-146 | 1 h | **DONE** | ✔ `SYNC_DIRS`/`SYNC_FILES` **một nguồn** ở đầu tệp (sau `set -u`), `sync_tree()` **chỉ** đọc hai biến (bỏ 5 chỗ rải trong vòng lặp + `rsync` `AGENTS.md` + vòng `vis-dev`/`vis`) và **không đổi hành vi**: 8 lời gọi `rsync` bản cũ = bản mới (thứ tự, `--delete`, exclude, đích), `AGENTS.md` vẫn best-effort, hai shim vẫn `die 3`; ✔ **`--check-sync`** in 8/8 mục `có|thiếu`, exit **0**/đủ · **2**/thiếu, **không** ssh/rsync; ✔ **đỏ khi bỏ một mục** (`vis-host-wasm` ⇒ 4 ca đỏ; rải lại danh sách tệp ⇒ 1 ca đỏ) và **xanh khi đủ** `viscript docs/viscript vis-host-linux vis-host-wasm .git AGENTS.md vis-dev vis`; ✔ gate macOS **PASS** `[exit=0]` 10/10, L0 **1175/0** (lần 1 đỏ T-INFRA ca thời gian) |
> CHỜ DUYỆT Agent 1 · `9bd7d022` (ST-151) · **nợ #135 đóng**: `bin/vis-linux` **+44/−10** — `SYNC_DIRS="viscript docs/viscript vis-host-linux vis-host-wasm .git"` + `SYNC_FILES="AGENTS.md vis-dev vis"` ở đầu tệp (sau `set -u`); `sync_tree()` chỉ đọc hai biến · **không đổi hành vi**: 8 lời gọi `rsync` của bản cũ và bản mới trên cùng cây giả (stub `rsync`/`ssh`) **giống hệt** (cùng thứ tự, `--delete`, exclude, đích); `AGENTS.md` vẫn best-effort (`2>/dev/null || true`), hai shim vẫn `die 3` (`JOURNAL/st151-do-truoc.log` §0) · **`--check-sync`** (mới): in 8/8 mục `có|thiếu`, exit **0** khi đủ · **2** khi thiếu (`thiếu n/8`), **không** gọi ssh/rsync (không `pick_backend`; ca tĩnh khoá `cmd_check_sync` không chứa `ssh`/`rsync`) · **9 ca** `crates/vis-cli/tests/sync_list.rs` (khuôn `soak_script.rs`): `sync_dirs_dung_tap`/`sync_files_dung_tap` so **tập**, `sync_tree_chi_dung_hai_bien` (không còn `$REPO_ROOT/<mục>` trong thân hàm), `check_sync_tai_repo_that_in_du_8_muc_va_exit_0`, `check_sync_thieu_mot_muc_thi_exit_2` (cây giả thiếu `vis-host-wasm` ⇒ **exit 2**, chỉ đúng mục đó `thiếu`), `check_sync_du_het_thi_exit_0_trong_cay_gia`, `help_co_check_sync` · **ĐỎ-TRƯỚC**: chưa có tệp test ⇒ `error: no test target named sync_list` (exit **101**) · **XANH-SAU** 9/9 · **bỏ `vis-host-wasm` khỏi `SYNC_DIRS`** ⇒ **4 ca đỏ** · **rải lại `for d in AGENTS.md vis-dev vis`** ⇒ ca `sync_tree_chi_dung_hai_bien` đỏ · hoàn nguyên sha256 `aedf781a…` trùng lúc gate (`JOURNAL/st151-do-truoc.log` sha256 `2bf3b11655627b6b7eeb6bf6d33f394445fd7eb1d824c17eee87b9f4946fd30f`) · **macOS gate lần 1 ĐỎ** ở L0 vì ca **THỜI GIAN** `lsp::module::tests::hieu_nang_nap_module_5_tep` (`116 ms ≥ 100 ms` dưới tải `7.39`/12 core) ⇒ **T-INFRA** (D2-07: chạy riêng **5/5 xanh**, `JOURNAL/st151-lsp-hieu-nang.log`), log đỏ giữ nguyên `st151-gate-macos.log` sha256 `8626ab3b3b7573b0b117078932215129b8972c638fba39c9dc1d92debbc618ee`; **lần 2 PASS** `[exit=0]` **10/10** bước, L0 **1175/0** (**130** suite; = 1166 + 9 ca), report-check PASS (25 báo cáo, 171 hàng QUEUE), doc-examples OK (24 khối) — `JOURNAL/st151-gate-macos-lan2.log` sha256 `a53c38a829727829256a783982487f297f24aecf52208a518f6bbb6959aedd2e` · `--check-sync` tại chỗ trên macOS `exit=0` 8/8 (`st151-check-sync-macos.log` `935fb81f…`) · **không** `unsafe`/crate mới/`#[ignore]`/`cfg`-bỏ/nới ngưỡng; không đụng `spec/`, `docs/`, `v2/`, `DECISIONS.md`, `crates/vis-cli/src/`; commit chỉ tệp `viscript/` (thay đổi của dòng OS do họ commit, không đụng)
APPROVED 2026-11-17 Agent 0 · 17 §6 · SYNC_DIRS/SYNC_FILES một nguồn + --check-sync (0/2) + 9 ca (bỏ 1 mục ⇒ 4 ca đỏ); #135 đóng; gate mac PASS L0 1175/0
| ST-152 | **R26** + cập nhật ROADMAP/STATE (chốt đợt 25) | ST-147..ST-151 | 1,5 h | **DONE** | ✔ danh sách **6 hàng** phụ thuộc + trạng thái (VD-143) ở §0; ✔ **đính chính/ghi chú R25** 4 điểm (a) L0 1159→1163→1166→**1175**, (b) Windows **bước 5 → bước 10**, self-test 11/12 → **12/12**, (c) `ps` 1169 là số **trước** 9 ca ST-151 (**chưa** đo lại), (d) ST-147 **tách phạm vi** (17b §1) — không còn BLOCKED; ✔ số đo **từng nền** + hash + sha256 log (§3); ✔ nợ trích theo `SO-NO.md`: **144** (91 mở/31 đóng/**20 VS3**/1 không làm/1 điều hướng); ✔ **5 câu VD-131** nguyên văn + **8 hạng mục** VS2 (cập nhật 4 và 7); ✔ **check_report PASS (26 báo cáo, 171 hàng QUEUE)**; ✔ gate macOS **PASS** L0 **1175/0** (130 suite) |
> CHỜ DUYỆT Agent 1 · b1c07802 · R26 xong: 26 báo cáo, check_report PASS; gate macOS PASS L0 1175/0; sổ nợ 144 (91 mở/20 VS3); VS2 đóng 8/8 (VD-151)

APPROVED 2026-11-17 Agent 0 · 17 §6 · R26 26 báo cáo/171 task/11 khoá/head b1c07802/blocked []; 5 câu VD-131 đủ; 8 hạng mục; check_report PASS; gate mac PASS L0 1175/0
| ST-153 | **check.rs: tên gốc đúng cả `\` và `/`** (VD-152, nợ mới từ ST-147) | ST-147 | 1 h | **DONE** | ≥3 ca khoá chạy mọi nền; macOS gate PASS 10/10 (L0 1159/0); psvn: self-test 12/12 + cổng đi QUA bước 5 (ghi bước dừng thật); không đổi hành vi Unix; log st153-* |
> CHỜ DUYỆT Agent 1 · `4a805115` (ST-153) · **VD-152 xong**: `crates/vis-cli/src/cli/check.rs` **+30/−1** — MỘT hàm `ten_goc` tách tên gốc theo **cả** `/` và `\` (`path.rsplit(['/', '\\'])`), dùng đúng chỗ cũ (`let chinh = ten_goc(path);`); `phu`/bộ lọc `--syntax` **không** đổi gì khác; **3 ca unit** `ten_goc_windows` (`C:\x\a.vis` ⇒ `a.vis`), `ten_goc_unix` (`d/a.vis` ⇒ `a.vis`), `ten_goc_khong_co_dau` (`a.vis` ⇒ `a.vis`) — xanh trên **cả ba** nền · **ca 7 của ST-147 đóng**: trên psvn @ `4a805115`, `vis check --syntax` với đường dẫn tuyệt đối **có** `\` ⇒ `ok 1` `RC=0` (trước ST-153: `not ok … VS0301`), `vis check` mặc định vẫn `VS0301` (kiểm kiểu không đổi), tên tương đối vẫn `ok` — `JOURNAL/st153-psvn-ca7.log` sha256 `19bb0ff7…` · **macOS**: gate lần 1 **ĐỎ** ở L0 vì ca đua `quyen::tests::dua_doi_symlink_lap_co_gioi_han` (`ro_ri=1` trên 50 000 lần) — phân loại **T-INFRA** theo D2-07: chạy **riêng** ca đó **5/5 xanh** (`JOURNAL/st153-symlink-lap.log` sha256 `bcc9bcba…`), không sửa ca/ngưỡng · gate lần 2 (sau phân loại) **PASS** `[exit=0]` **10/10** bước `--- …: OK`, L0 **1166/0** (129 suite; = **1163** + 3 ca ST-153) — `JOURNAL/st153-gate-macos-lan2.log` sha256 `0aa9a016…`; log đỏ giữ nguyên `JOURNAL/st153-gate-macos.log` (sha256 `fdc42745…`) · **Linux (`ps`)**: gate **PASS** `[exit=0]` **10/10** bước, L0 **1169/0** (129 suite; = **1162** + 4 ca ST-149 + 3 ca ST-153) — `JOURNAL/st153-gate-ps.log` sha256 `f38a2b88…`; sha256 `check.rs` **hai phía khớp** `9761b5dd…` (ps = tại chỗ) · **đối chiếu độc lập** `cargo test --workspace -- --list`: macOS **1166** / ps **1169** dòng `: test`, chênh **+3** = 4 ca `cfg(target_os = "linux")` (`quyen::tests::doc_lay_tu_handle_khi_ten_da_bi_doi`, `…doc_theo_handle_bao_dung_duong_dan_cua_proc`, `…handle_ngoai_root_bi_chan_tat_dinh`, `sandbox_check_linux_doc_lsm`) − 1 ca riêng macOS (`sandbox_check_macos`) — **đúng kỳ vọng** của đề bài (ps = mac + 3) · **psvn**: self-test case âm **12/12** (`ok 1..ok 12`), cổng **đi QUA bước 5** (`--- vis explain: OK`) và dừng ở **bước 10/10 `--- L0 test: FAIL`** (fail-fast ở suite `cap_proc`: **150/4**, lý do `/bin/echo` không tồn tại trên Windows; ~100 ca lệch Windows còn lại **thuộc VS3**, không sửa) — `JOURNAL/st153-gate-psvn.log` sha256 **hai phía khớp** `a229d743…`, môi trường VD-136 `JOURNAL/st153-psvn-env.log` sha256 `cc65bbfc…` · **không** `unsafe`/crate mới/`#[ignore]`/nới ngưỡng, không đổi kỳ vọng ca cũ, không đụng `spec/`, `docs/`, `v2/`, `DECISIONS.md`; commit chỉ tệp `viscript/` |

APPROVED 2026-11-17 Agent 0 · 17b §3 · ten_goc tách cả \ và / (3 ca unit); ca 7 ST-147 đóng; mac 1166/0, Linux gate PASS 1169/0; psvn self-test 12/12, cổng qua 9/10 bước dừng L0
| ST-154 | **Nợ #144**: ca dương `read_dir` trên thư mục **có quyền ghi** (đóng #144) | ST-152 | 0,75 h | **DONE** | 15/15 trên ps; ca âm cũ vẫn xanh; gate macOS PASS L0 1175/0; log st154-* |
GHI CHÚ 2026-11-17 Agent 0 (17c §5) · ST-154 gộp thêm việc sổ: `#143` → VS3 (VD-154(b)), thêm `#145` (VD-154(c)), đếm lại SO-NO (90 mở/31 đóng/21 VS3); R27 đính chính 91/20 của R26
> CHỜ DUYỆT Agent 1 · `d203b664` (ST-154) · **nợ #144 đóng**: ca **15** `con_read_dir_thu_muc_co_quyen_ghi_van_liet_ke_duoc` trong `vis-host-linux/tests/landlock.rs` (**+137** dòng, **không** sửa ca nào cũ, không `#[ignore]`/`cfg`-bỏ) — `co.txt` ghi **trước** khi áp ruleset; **MỘT** ruleset **HAI** luật (`trong` `ghi:false` + `trong-ghi` `ghi:true`, `so_luat == 2` như ca 14); **dương**: helper `ls trong-ghi` ⇒ **`exit 0`** + danh sách `co.txt`; **đối chứng luật ghi có hiệu lực**: helper `ghi trong-ghi/moi.txt` ⇒ `exit 0`, `ls` lại thấy **`co.txt` + `moi.txt`**; **âm cùng ca ngoài phạm vi**: helper `ls ngoai` ⇒ **`exit 7`** + `loi: Permission denied (os error 13)` (`EACCES`); đỏ-trước in `exit=0` + `co.txt`; helper gọi thẳng `std::fs::read_dir`, **không** qua `FsGuard` ⇒ **kernel** chặn (VD-120) · **`ps` 15/15 xanh** (`test result: ok. 15 passed; 0 failed; 0 ignored`, 0,35 s; 14 ca cũ vẫn xanh; kernel `7.0.0-43-generic`, Ubuntu 26.04.1, rustc 1.99.1, `lsm` có `landlock`) — `JOURNAL/st154-landlock-ps.log` (5 278 byte, sha256 `cb4ab4ab1b57f5d653b19addd21cced7dab877a74eb9557876ecb7865b95dbca`, header VD-136 + `### EXIT_VIS_LINUX_RUN=0`); **sha256 tệp đo HAI PHÍA khớp** `7102e90b3a484a9128fe981240090e7ccd43a7e9f96c6c05f4576ac57e871627` (local = `ssh root@ps sha256sum`), 25 454 byte; lần đo **1** (trước `cargo fmt`, cũng 15/15) giữ nguyên ở `JOURNAL/st154-landlock-ps-lan1-truoc-fmt.log` (sha256 `c9b79b9088f074478a7dbacdbae1ff6ed001ac0e7428c3e8de4b64cdcda7645d`) · **sổ theo VD-154**: `#143` → **chuyển VS3** (trạng thái ghi `VD-154(b): chuyển VS3 vì ca st130b xanh`, **giữ** dòng lịch sử; §5 lên **21 mục**; §7.3 ghi chú), `#144` **đóng ở ST-154** (vế 2 của `#133`), thêm **`#145`** (**mở** — `rt.rs:128-129` nói `chan(0)` bị **tyck** chặn, thực tế tyck **không** kiểm `cap` (`tyck/expr/kenh.rs:16-33`), **runtime** chặn `exit 1` (`kenh_cap0.rs:57-68`); xử khi có task VS3 kênh) ⇒ §8 **146 mục = **91 mở** / **32 đóng** / **21 VS3** / 1 không làm / 1 điều hướng**, trùng số **0**, dải **1..145** — `JOURNAL/st154-kiem-trung.log` (sha256 `ffd9b22f3b68edaa0a60270f11d8a97092a6407def2bb3787bd5a282738d4ae7`, PASS 10/10 mục kiểm, 0 FAIL — bản chạy trên SO-NO CUỐI, có §9) · **đối chiếu số VD-154(b) (ghi lại thay vì giấu)**: `90 mở / 31 đóng / 21 VS3` là số của bảng **144 dòng** *sau* bước đổi `#143` và *trước* khi thêm `#144`/`#145` (90+31+21+1+1 = 144) — luật sổ (§3: không xoá dòng; VD-147(3): mỗi nợ một dòng) ⇒ sổ cuối **146 dòng**; đề nghị Agent 0 đính chính nếu R27 trích số cuối · **macOS gate**: lần **1 ĐỎ** `--- fmt: FAIL` (rustfmt đòi xuống dòng **một** `assert!` của ca 15 — không phải `T-INFRA`, không retry lấy xanh) ⇒ sửa **chỉ khoảng trắng** rồi chạy **lần 2**: **PASS** `[exit=0]`, **10/10** bước `--- …: OK`, L0 **1175/0** (**130** suite), report-check PASS (**26** báo cáo, **173** hàng QUEUE), doc-examples OK (24 khối) — `JOURNAL/st154-gate-macos-lan2.log` (76 725 byte, sha256 `aae5447557dc20f71b8ebfcf9eea31b42bf74f538d957ed98941e0d04ae9053b`); log đỏ giữ nguyên `JOURNAL/st154-gate-macos.log` (sha256 `3b0a12649934c5df71b5f5e2bd6c4d264b8d193cb3d541ae117c94704cea2429`) · `python3 viscript/test/report/check_report.py --all` ⇒ **PASS (26 báo cáo, 173 hàng QUEUE phủ đủ)** — `JOURNAL/st154-check-report.log` (sha256 `342f3c26080fad7fb7e58fe3ab8ec11a7d391b621ba53de5eb878cbe62d8a650`) · **không** `unsafe`/crate mới/`#[ignore]`/`cfg`-bỏ/nới ngưỡng; không đụng `crates/`, `spec/`, `docs/`, `v2/`, `DECISIONS.md`; commit chỉ tệp `viscript/` + `vis-host-linux/tests/landlock.rs`; **task kế: KHÔNG** — chờ Agent 0 ban hành tài liệu thiết kế VS3
APPROVED 2026-11-17 Agent 0 · 17c §5 · ca 15 (read_dir thư mục có quyền ghi vẫn liệt kê; ngoài phạm vi EACCES); ps 15/15; #144 đóng, #143→VS3, #145 thêm; sổ 146
| ST-155 | **Điều tra VS3 (chỉ đọc)** cho tài liệu thiết kế: async · kênh · TLS · quyền ghi · Windows | ST-152 | 2 h | **DONE** | ✔ `agent/archive/journal-pre-v040/st155-vs3-dieu-tra.md` **222 dòng**: **6 mục** × đúng 5 khối (Hiện trạng · Việc cần cho VS3 · Kích thước `[ƯL]` · Rủi ro · Câu hỏi cho Agent 0), mỗi mục **≤31 dòng**; ✔ **dẫn chứng `tệp:dòng`**: mục 1 **15** · 2 **12** · 3 **7** · 4 **10** · 5 **6** · 6 **6** (yêu cầu ≥5 cho mục 1–4, ≥3 cho 5–6); ✔ **§7 bảng tổng hợp** (6 mục × kích thước/rủi ro/phụ thuộc) + **§8 Top 5 câu hỏi** + **§9 6 việc đề nghị KHÔNG làm**; ✔ nhãn: `[ĐO]` **11** (đều dẫn log có sẵn), `[ƯL]` **12** (mỗi chỗ kèm cơ sở: số dòng tệp/số ca), `[GT]` **7** — **0** số đo mới chưa chạy; ✔ **không** sửa mã/`docs/`/`spec/`/`v2/`, không crate mới, không nightly/soak; `git status -- viscript/` chỉ **1** tệp mới |
> CHỜ DUYỆT Agent 1 · 93e1d418 · tài liệu điều tra VS3 (6 mục + bảng + câu hỏi), không sửa mã; gate macOS PASS L0 1175/0 — log `JOURNAL/st155-gate-macos.log` (76 461 B, sha256 `551c671251976021352a8e4f29b4a4f189b8fb8bf1be857c8ac29fbd6c38347c`, header VD-136 + `[exit=0]`), **10/10** bước, report-check PASS (**26** báo cáo, **173** hàng QUEUE), doc-examples OK (24 khối); 3 việc cần Agent 0 chốt ngay: **mô hình bộ nhớ/GC** (chặn mục 1→2/3/4), **allowlist giấy phép cho kho CA TLS** (webpki-roots/ring), **số nợ đường GHI** (`#142` trong hiến chương §5 vs `#140` trong `SO-NO.md`) |

APPROVED 2026-11-17 Agent 0 · 17c §5 · ST-155 chỉ đọc (222 dòng, 57 dẫn chứng, 0 dòng mã); gate mac PASS L0 1175/0
| ST-156 | **R27** + cập nhật ROADMAP/STATE (chốt đêm 17/11: ST-154, ST-155, đính chính sổ 146) | ST-154, ST-155 | 1,25 h | **DONE** | đủ hàng phụ thuộc (ST-154/155/152) + trạng thái; đính chính sổ **146/91/32/21**; số 3 nền; 5 câu VD-131; check_report PASS (27 báo cáo, 173 hàng) |
> CHỜ DUYỆT Agent 1 · dd088322 · R27 xong: 27 báo cáo, check_report PASS; gate macOS PASS L0 1175/0; sổ 146 (91 mở/32 đóng/21 VS3)

APPROVED 2026-11-17 Agent 0 · VD-155 · R27 27 báo cáo/174 task/11 khoá/head dd088322/blocked []; 6 điểm đính chính; check_report PASS; gate mac PASS L0 1175/0
| ST-157 | **Nợ #110**: VM đưa `argv` vào tham số không-`cap` của `main` (parity với interp) | ST-156 | 1 h | **DONE** | ✔ **ca đỏ-trước** `crates/vis-cli/tests/vm_main_argv.rs` (**4** ca, so **byte** `vis run` vs `vis run --vm --no-cache`): trước sửa **3/4 ĐỎ** — interp `[a, b]` vs VM `()` (`JOURNAL/st157-do-truoc.log`, `[exit=101]`); ✔ sau sửa **4/4 xanh** và **cùng byte** `[a, b]`, exit 0 (cả ca `cap` đứng trước/sau `argv`: `Value::Cap` đúng chỉ số, ST-72b); ✔ `main()` không tham số giữ nguyên; ✔ sửa **`crates/vis-vm/src/vm.rs` +40/−6** (bỏ qua "vỏ" `Call(main, 0); Halt` của codegen khi `main` có tham số, đặt `argv` vào mọi tham số không-`cap`; **không** `unsafe`/crate mới/`#[ignore]`/nới ngưỡng); ✔ `cargo test -p vis-cli` **778/0** (77 suite); ✔ gate macOS **PASS 10/10**, L0 **1179/0** (131 dòng `test result`; cơ sở 1175 + 4 ca mới); ✔ `vis test test/sem` **447/447** (interp) và **447/447** (`--vm`); ✔ sổ: **#110 đóng ở ST-157**, **146 = 90 mở/33 đóng/21 VS3/1/1** (`JOURNAL/st157-kiem-trung.log`); log st157-* |
> CHỜ DUYỆT Agent 1 · `567350e1` (ST-157) · **nợ #110 đóng**: VM đưa `argv` (`list[str]`) vào tham số **không-`cap`** của `main` như interp (`interp.rs:221-234`) — codegen bọc `fn main` trong "vỏ" `Call(main, 0); Halt` **0 tham số** (`vis-bytecode/src/codegen.rs`), nên VM cũ chỉ đặt `argv` khi `arity == 0` ⇒ `fn main(args: list[str])` nhận `()` trên `--vm`; sửa **`crates/vis-vm/src/vm.rs`** (+40/−6): khi entry là vỏ `[Call(fi, 0), Halt]` và `fi` tên `main` có tham số ⇒ chạy **thẳng** `main`, đặt `Value::Cap` đúng **chỉ số** (ST-72b) rồi `argv` vào **mọi** tham số còn lại; `main()` không tham số giữ nguyên đường cũ · **ca đỏ-trước** `crates/vis-cli/tests/vm_main_argv.rs`: **4** ca so **byte** hai đường (interp vs `--vm --no-cache`), truyền `a b` — **3/4 ĐỎ** trước sửa (`interp (0, "[a, b]\n")` vs `vm (0, "()\n")`, `[exit=101]`, `JOURNAL/st157-do-truoc.log`), **4/4 xanh** sau sửa (`[a, b]`, exit 0); ca `cap` dùng `fs.read` **qua cap** để chứng minh `Value::Cap` không bị `argv` ghi đè · **ca cũ** `cap_khong_lo::doi_so_dong_lenh_la_str` vốn **ghim đúng sự lệch** (VM `VS0501` + `` `()` ``) — sau khi đóng #110 buộc phải đỏ, **cập nhật** để khẳng định **parity** (`[cap[Dir] @ p"/"]` + `1`, exit 0), giữ bất biến ST-124 (không đường nào tạo `Cap`); **ghi rõ** đây là ca duy nhất phải đổi kỳ vọng và lý do (VD-156) · `cargo test -p vis-cli` **778/0** (77 suite, gồm `vm_main_argv` 4/4) · **gate macOS** `../vis-dev gate --fast` ⇒ **PASS** `[exit=0]`, **10/10** bước `--- …: OK`, L0 **1179/0** (**131** dòng `test result` = cơ sở 1175 + 4 ca mới), doc-examples OK (24 khối) — `JOURNAL/st157-gate-macos.log` (header VD-136 + sha256 tệp sửa + `[exit=0]`) · `vis test test/sem` **447/447** interp (`JOURNAL/st157-vis-test.log`) và **447/447** `--vm` (0 `not ok`, `JOURNAL/st157-vis-test-vm.log`) · **sổ**: `#110` **mở → đóng ở ST-157**, §8 **146 mục = 90 mở / 33 đóng / 21 VS3 / 1 không làm / 1 điều hướng**, trùng số 0, dải 1..145 — `JOURNAL/st157-kiem-trung.log` (PASS) · **không** `unsafe`/crate mới/`#[ignore]`/nới ngưỡng; không đụng `spec/`, `docs/`, `v2/`, `DECISIONS.md`; commit chỉ tệp `viscript/`; **task kế: ST-158**
APPROVED 2026-11-17 Agent 0 · 17d §1 (VD-156) · đỏ-trước vm "()" vs interp "[a, b]"; vm.rs +40/−6; 4 ca cùng byte; #110 đóng; L0 1179/0; ca cap_khong_lo đổi kỳ vọng là BẮT BUỘC (ghim sự lệch cũ)
| ST-158 | **Nợ #129**: gom danh sách số dòng khối tài liệu về **một nguồn** (4 tệp test) | ST-156 | 0,75 h | **DONE** | ✔ **một nguồn** `test/doc_excluded.rs` (48 dòng, SPDX `//`): `const DOC_EXCLUDED: [usize; 7] = [305, 432, 445, 614, 666, 745, 1420]` — **không đổi số nào**, 0 dependency; tệp dùng `//` **không** `//!` vì `include!` chèn ở **vị trí item** (inner doc ⇒ `E0753`) · ✔ **4 tệp test** bỏ bản chép, `include!("../../../test/doc_excluded.rs")` (đúng cấp `crates/<crate>/tests/` → `viscript/test/`, kiểm `ls`): `check.rs` (2 bộ lọc, dùng `DOC_EXCLUDED`; −8/+4) · `fmt.rs` (include ở vị trí **item** — lồng trong `fn` báo "non-statement macro in statement position") · `doc_fixtures.rs` (bỏ tuple `(dòng, lý do)`; **lý do** giữ trong `LY_DO`, số dòng lấy từ `DOC_EXCLUDED_SYNTAX`) · `hir.rs` (5 chỗ `EXCLUDED` → `DOC_EXCLUDED`, bỏ `EXCLUDED_COUNT`) · ✔ số kỳ vọng **không đổi**: vis-syntax loại trừ **6** khối (khối 666 `extern cmd git(…, ...rest: str)` parse sạch từ ST-27) nên nguồn dẫn xuất `DOC_EXCLUDED_SYNTAX: [usize; 6]` = 7 khối trừ khối 666 bằng **const block** (không chép số); `#[allow(dead_code)]` trên const (mỗi tệp dùng một phần) · ✔ **trước/sau giống hệt** 4 target: `check` **13/0** · `fmt` **13/0** · `doc_fixtures` **3/0** · `hir` **10/0** (`JOURNAL/st158-truoc.log` / `st158-sau.log`) · ✔ gate macOS **PASS** `[exit=0]` **10/10** bước, L0 **1179/0** (**131** dòng `test result`, **không đổi**); check_report PASS (**27** báo cáo, **177** hàng) · ✔ sổ: **#129 đóng ở ST-158**, §8 **146 = 89 mở / 34 đóng / 21 VS3 / 1 / 1**; **không** `unsafe`/crate mới/`#[ignore]`/nới ngưỡng; không đụng `spec/`, `docs/`, `v2/`, `DECISIONS.md`, `crates/*/src/`; **task kế: ST-159** |
> CHỜ DUYỆT Agent 1 · `8036b0a2` (ST-158) · **nợ #129 đóng**: danh sách 7 khối tài liệu bị loại trừ (VD-36) nay ở **một nguồn** `viscript/test/doc_excluded.rs` (48 dòng; SPDX `//` — dùng `//` **không** `//!` vì `include!` chèn ở vị trí **item**, inner doc ⇒ `E0753`): `const DOC_EXCLUDED: [usize; 7] = [305, 432, 445, 614, 666, 745, 1420]` — **không đổi số nào**, không dependency mới · **4 tệp test** bỏ bản chép và dùng `include!("../../../test/doc_excluded.rs")` (đúng cấp từ `crates/<crate>/tests/`), giữ nguyên logic/khẳng định: `check.rs` (cả 2 bộ lọc `!contains`/`contains`; −8/+4) · `fmt.rs` (include phải ở **vị trí item** — lồng trong `fn` báo "non-statement macro in statement position"; −4/+6) · `doc_fixtures.rs` (bỏ 7 mục tuple `(dòng, lý do)`: **lý do** còn trong `LY_DO`, số dòng lấy từ `DOC_EXCLUDED_SYNTAX`; −34/+29) · `hir.rs` (5 chỗ `EXCLUDED.iter()` → `DOC_EXCLUDED.contains`/`to_vec`/`len`, bỏ `EXCLUDED_COUNT`; −32/+11) · **dị biệt vis-syntax được ghi rõ, không giấu**: khối 666 parse **sạch** từ ST-27 (variadic) ⇒ vis-syntax loại trừ **6** khối, ba tệp còn lại **7**; nguồn dẫn xuất `DOC_EXCLUDED_SYNTAX: [usize; 6]` bằng **const block** trừ vị trí khối 666 — **không chép số**; `#[allow(dead_code)]` trên const (mỗi tệp dùng một phần danh sách) · **chứng minh không đổi hành vi**: 4 target `cargo test` **trước** = **sau** — `check` **13/0** · `fmt` **13/0** · `doc_fixtures` **3/0** · `hir` **10/0** (`JOURNAL/st158-truoc.log` / `st158-sau.log`, sha256 4 tệp + `[exit=]`) · **gate macOS** `../vis-dev gate --fast` ⇒ **PASS** `[exit=0]`, **10/10** bước `--- …: OK`, L0 **1179/0** (**131** dòng `test result` — **không đổi**) — `JOURNAL/st158-gate-macos.log` (header VD-136 + sha256 tệp sửa + `[exit=0]`) · `check_report.py --all` ⇒ **PASS (27 báo cáo, 177 hàng)** — `JOURNAL/st158-check-report.log` · **sổ**: `#129` **mở → đóng ở ST-158**, §8 **146 mục = 89 mở / 34 đóng / 21 VS3 / 1 không làm / 1 điều hướng** · **không** `unsafe`/crate mới/`#[ignore]`/nới ngưỡng; không đụng `spec/`, `docs/`, `v2/`, `DECISIONS.md`, `crates/*/src/`; commit chỉ tệp `viscript/` (thay đổi chưa commit của dòng OS `v2/` giữ nguyên; `git add` tường minh) · **task kế: ST-159** (R28)
APPROVED 2026-11-17 Agent 0 · 17d §1 (VD-156) · test/doc_excluded.rs 7 số một nguồn + 4 include!; trước/sau 13/13,13/13,3/3,10/10; #129 đóng; L0 1179/0 không đổi
| ST-159 | **R28** (chốt phần sau R27) | ST-157, ST-158 | 0,75 h | **DONE** | ✔ **3 tệp** `reports/R28-viscript.{md,json,json.license}`: **28** báo cáo, **177** hàng QUEUE (176 `DONE` + **ST-159 `READY`** tại `head` `fdb21d15`); JSON **11 khoá**, `tasks` **177** đúng thứ tự, `commits[]` hash thật; ✔ `check_report.py --all` ⇒ **PASS (28 báo cáo, 177 hàng QUEUE phủ đủ)** — `JOURNAL/st159-check-report.log`; ✔ gate macOS `../vis-dev gate --fast` ⇒ **PASS** `[exit=0]`, **10/10** bước `--- …: OK`, L0 **1179/0** (**131** suite), doc-examples OK (24 khối), report-check **28/177** — `JOURNAL/st159-gate-macos.log` (header VD-136 + sha256 tệp JSON/MD + `[exit=0]`); ✔ §2 đính chính 4 mục (L0 **1175→1179**; **1 ca cũ** `cap_khong_lo::doi_so_dong_lenh_la_str` đổi kỳ vọng **bắt buộc** theo VD-156; `doc_fixtures` loại trừ **6** khối so nguồn **7** số; `ps`/`psvn` **chưa đo lại**); ✔ §4 **5 câu VD-131** nguyên văn; §5 **8 hạng mục VS2** (hạng mục 4 = `vis-host-linux` **15/15**, hạng mục 7 = Windows **9/10 bước** dừng L0); §7 VS3 **5 câu hỏi** chờ Agent 0; ✔ sổ **146 = 89 mở / 34 đóng / 21 VS3 / 1 / 1** (#110, #129 đóng); ✔ **không** sửa R1..R27/mã/test/kỳ vọng/ngưỡng; không nightly/soak; log st159-* |
> CHỜ DUYỆT Agent 1 · fdb21d15 · R28 xong: 28 báo cáo, check_report PASS; gate macOS PASS L0 1179/0; sổ 146 (89/34/21)

APPROVED 2026-11-17 Agent 0 · VD-156 · R28 28 báo cáo/177 task/11 khoá/head fdb21d15/blocked []; 4 đính chính; check_report PASS; gate mac PASS L0 1179/0
| ST-160 | **Dán quyết định đợt 29** (Agent 1): 13 APPROVED + VD-157..VD-164 + mở nợ #146 | ST-159 | 0,5 h | **DONE** | check_report PASS; sổ **147** mục, 0 trùng; quyết định đầy đủ trong `JOURNAL/quyet-dinh-a0-2026-11-17-dot29.md` |
APPROVED 2026-11-17 Agent 0 · ST-160 đạt: đủ 13 APPROVED, đủ VD-157…VD-164, sổ 147 mục, thứ tự task đúng; không cần sửa
| ST-161 | **Đo lại cùng HEAD mới nhất**: `ps` gate --fast; `psvn` cargo test + self-test + cổng (chỉ đo) | ST-160 | 1,5 h | **DONE** | ✔ **cùng HEAD `0df923c2`** cả ba nền — `head_full` trong header VD-136 của **từng** log + sha256 **3 tệp đại diện khớp tuyệt đối** (`crates/vis-vm/src/vm.rs` = `7290111990a55a605404222af0271387df5d01e1b71fc1b2dc2b5d4e3c0d9e9d` (ST-157) · `crates/vis-cli/tests/sync_list.rs` = `f38474b179832a4816f837abfd0004403a255312e65e82cf561c3bf15958c4d9` (ST-151) · `crates/vis-cli/src/cli/check.rs` = `9761b5dd603d1f9cfedb4b3767b42eb961bbacc1c90b654bd7f581f334bd7a00` (ST-153)); `git status --porcelain -- viscript/` trên `ps` **0 tệp tracked đổi**, trên `psvn` **rỗng** (log ra `/c/Users/ps/vislogs/`) · ✔ **`ps` gate --fast PASS 10/10** `[exit=0]`, L0 **1182/0** **đúng dự kiến** (1169 + 9 ST-151 + 4 ST-157), **131** suite — `JOURNAL/st161-gate-ps.log` (2 059 dòng); chênh `ps`−`mac` **+3** giải thích **theo suite**: chỉ **một** suite khác trong 131 — `vis-cli` bin `src/main.rs` **66 / 63**; ba ca **chỉ có trên `ps`** (đều `ok`): `quyen::tests::{doc_lay_tu_handle_khi_ten_da_bi_doi, doc_theo_handle_bao_dung_duong_dan_cua_proc, handle_ngoai_root_bi_chan_tat_dinh}` (nhóm đọc `/proc` bằng handle, `cfg(target_os = "linux")`); **không** ca nào chỉ có trên mac · ✔ **`psvn`**: `git pull --ff-only` `3abd6328..0df923c2` `[exit=0]` — `st161-pull-psvn.log`; `cargo test --workspace --no-fail-fast` **1068 đạt / 105 đỏ**, **131** suite, `[exit=101]` (thay `1053/100 @3dc13342`) — `st161-cargo-test-psvn-nff.log`; bảng lệch **theo từng suite**: `src\main.rs` 56/0→**59/0** (+3 unit test đi cùng `check.rs`) · `tests\sync_list.rs` **mới** 5/4 (4 đỏ: `bash` phân giải ra **WSL stub** `execvpe(/bin/bash) failed`) · `tests\vm_main_argv.rs` **mới** 3/1 (đỏ `argv_dau_cap_sau`: `VS0102`/`VS0201` do `\` đường dẫn Windows) · `tests\vish_cmd_body.rs` 15/1→**18/2** (+3 ca ST-149; đỏ mới `khong_bind_van_la_lenh`: `VS0501` do `\t` đường dẫn Windows) · `tests\json_ra.rs` 6/7→**7/6** (`check_json_khong_co_tap_khong_doi` **đỏ ở ST-144 nay xanh**; tệp test **và** mã json **không đổi** trong khoảng HEAD ⇒ lần đo cũ **nhiễu**, không phải đã sửa — **không** chạy lại để đuổi xanh); 6 ca đỏ **mới** đều **môi trường Windows**, **0** ca lỗi logic ngôn ngữ; danh sách **105** ca đỏ sinh tự động từ log: `JOURNAL/st161-psvn-105-do.log` · self-test **12/12** `[exit=0]` — `st161-selftest-psvn.log` · **cổng `psvn`**: 9/10 bước OK, **dừng ở bước 10 `L0 test: FAIL`** = **150/4** (16 suite, dừng ở `cap_proc`, như R28), `[exit=1]` — `st161-gate-psvn.log` · ✔ **`mac` đo lại tại `0df923c2`**: lần 1 đỏ `luật R1..R4` **do artifact tự gây** (tệp dẫn xuất `.txt` vi phạm R3 — đã đổi `.log`, **giữ** log lần đỏ `st161-gate-macos-lan1-r3.log`), lần 2 đỏ **T-INFRA `#146`** `quyen::tests::dua_doi_symlink_lap_co_gioi_han` (`TOCTOU dua: lap=50000 ok_trong=11878 ro_ri=1 chan=38121 vong_doi=16383`, `quyen.rs:782` — `st161-gate-macos-lan2-t146.log`), lần 3 (**chạy lại một lần** để phân loại) **PASS 10/10** `[exit=0]`, L0 **1179/0** (131 suite) — `st161-gate-macos.log` · ✔ `check_report.py --all` **PASS (28 báo cáo, 188 hàng)** — `st161-check-report.log` · ✔ **không** sửa `crates/`/`spec/`/`docs/`/`v2/`/`DECISIONS.md`/ca/ngưỡng; không crate mới/`unsafe`/`#[ignore]`/retry-lấy-xanh; `git add` **tường minh** tệp `viscript/`; **task kế: ST-163** |
> CHỜ DUYỆT Agent 1 · `0df923c2` · **ba nền cùng HEAD**: `ps` L0 **1182/0** **đúng dự kiến** (131 suite, gate PASS 10/10) · `mac` **1179/0** (131 suite, gate PASS 10/10; `ps`−`mac` = **+3** = đúng ba ca `quyen::tests::*` đọc `/proc` chỉ có trên Linux — 130 suite còn lại trùng từng ca) · `psvn` `cargo test --no-fail-fast` **1068/105** (131 suite; từ `1053/100 @3dc13342` = +15 đạt, −1 cũ xanh, +6 mới đỏ — **6 ca mới đều môi trường Windows**: 4 WSL-`bash` + 2 escape `\` đường dẫn), self-test **12/12**, cổng **dừng ở L0** 150/4 · **nợ #146 tái hiện** trên mac (`ro_ri=1`/50 000 vòng, `quyen.rs:782`) ⇒ bằng chứng cho **ST-163**
| ST-162 | **Chuẩn hoá script bọc cổng từ xa** (sự cố R28 §5 #4/#5/#6): đường tuyệt đối cho cargo/log; mã thoát ra tệp `.exit`; psvn đọc `$LASTEXITCODE`; log psvn ra ngoài repo hoặc `.gitignore` | ST-160 | 0,75 h | **DONE** | ✔ **tệp mới** `bin/vis-remote` (**397** dòng, SPDX `#`): `vis-remote <mac\|ps\|psvn> [--log <tệp>] [--tag <nhãn>] -- <lệnh…>`, thêm `--check` (in môi trường + `.exit` của `true`/`false`) và `--hosts`; **`bin/vis-linux` giữ nguyên hành vi** (`--check-sync` vẫn đủ **8/8** mục) · ✔ **#4 mã thoát THẬT**: thân chạy ghi `echo $? > "$LOG.exit"` **bên trong bash trên máy đích**, rồi **đọc lại từ tệp** (`cat`) — **không** bao giờ lấy `$?`/`$LASTEXITCODE` của PowerShell; lệnh giả `bash -c 'exit 3'` ⇒ `[exit=3]` + tệp `.exit` = `3` trên **cả** `ps` (`JOURNAL/st162-exit-ps.log`) và `psvn` (`st162-exit-psvn.log`); `--check` cho `true`=**0** / `false`=**1** trên cả ba nền · ✔ **#5 ĐƯỜNG TUYỆT ĐỐI**: `cargo` dò rồi **ghim** (`mac` `/Users/mps/.cargo/bin/cargo` · `ps` `/root/.cargo/bin/cargo` · `psvn` `/c/Users/ps/.cargo/bin/cargo`), log luôn `$LOG`/`$LOG.exit` tuyệt đối, không còn `cd <tương đối> && …` · ✔ **#6 PATH**: thân chạy `export PATH="$(dirname $CARGO):…"` **trước** lệnh ⇒ trên `ps`: `bin/vis-dev check` **PASS** `[exit=0]` (`JOURNAL/st162-ps-check.log`, chép nguyên văn từ log chạy thật) và `cargo --version` = `/root/.cargo/bin/cargo` `[exit=0]` (`st162-ps-check-cargo.log`) · ✔ **#7 LOG NGOÀI REPO**: mặc định `<home máy đích>/vislogs/` (psvn `/c/Users/ps/vislogs/`); `--log` chỉ đổi **bản sao cục bộ** và **cảnh báo** khi nằm trong repo · `psvn`: tệp untracked cũ `st153-gate-psvn.log` (di sản cách làm cũ; sha256 `a229d743…` **trùng** bản đã commit ở `1263f7ea`) chuyển sang `/c/Users/ps/backup-20261117b/` (`st162-psvn-don.log`) ⇒ sau **một** lần chạy thật `git log --oneline -1` **không** sinh tệp nào trong repo (`st162-psvn-realrun.log`: `git status --porcelain` **rỗng**, log + `.exit` nằm ở `/c/Users/ps/vislogs/`) ⇒ `git pull --ff-only` **KHÔNG bị chặn**: fast-forward `4a805115..3abd6328`, `[exit=0]` (`st162-psvn-pull.log`) · ✔ **header VD-136** ở mọi log: máy/OS/nhân/nproc/shell, `cargo`+`rustc` **kèm đường dẫn tuyệt đối**, repo, HEAD đầy đủ + ngắn + nhánh + số untracked + `git status`, lệnh, UTC bắt đầu/kết thúc, `[exit=N]` **đọc từ `.exit`** · ✔ **gate macOS** `../vis-dev gate --fast` chạy **qua chính `vis-remote mac`** ⇒ **PASS** `[exit=0]`, **10/10** bước `--- …: OK`, L0 **1179/0** (**131** suite) — `JOURNAL/st162-gate-macos.log` · ✔ `check_report.py --all` ⇒ **PASS (28 báo cáo, 188 hàng QUEUE)** — `JOURNAL/st162-check-report.log` · ✔ **không** sửa `crates/`/`spec/`/`docs/`/`v2/`/`DECISIONS.md`; không crate mới/`unsafe`/`#[ignore]`/nới ngưỡng; `git add` **tường minh** tệp `viscript/`; **task kế: ST-161** |
> CHỜ DUYỆT Agent 1 · `dbd97729` · **sự cố R28 §5 #4/#5/#6 đóng**: `bin/vis-remote` (mới) chạy một lệnh bash trên `mac`/`ps`/`psvn` với (1) `cargo`+log **tuyệt đối**, (2) mã thoát ghi ra `<log>.exit` **bên trong bash máy đích** rồi đọc lại từ tệp ⇒ `exit 3` ⇒ `[exit=3]` trên cả `ps` và `psvn`, (3) log mặc định `<home>/vislogs/` **ngoài repo** ⇒ `psvn` `git pull --ff-only` **không** bị chặn sau một lần chạy thật, (4) header VD-136 ở mọi log; gate macOS **PASS 10/10** L0 **1179/0** (131 suite) chạy qua chính `vis-remote mac`; `check_report` PASS **28 báo cáo / 188 hàng**
| ST-163 | **Điều tra #146** (ca đua symlink, VD-158): **500 000** vòng trên macOS và `ps`, thu đúng assertion hỏng | ST-160 | 3 h | **DONE** | ✔ **PHÂN LOẠI (b) — TOCTOU THẬT, CHỈ macOS, khớp `VD-145`** (không phải lỗi test): mac **4 lần đỏ / 8 100 000 vòng**, nội dung lọt là **`"NGOAI"` thật (5 byte)**; `ps` **0 / 4 100 000 vòng** (quyết định trên `/proc/self/fd`) — `JOURNAL/st163-toctou-macos{,-lan2,-lan3}.log` · `st163-toctou-ps{,-lan2}.log` · ✔ **cơ chế có vết** (`VIS_TOCTOU_TRACE=1`): `xac_nhan_sau_mo(truoc=/…/root/tep, sau=/…/root/tep, danh_tinh_handle=(16777229,57186948), metadata(sau)=(16777229,57186948))` ⇒ `sau` là **tên trong root** nhưng `std::fs::metadata(sau)` (`quyen.rs:472`) **đi theo symlink** trả **inode tệp ngoài** (`ngoai.txt`) ⇒ `kiem_canon` xanh ⇒ `Ok("NGOAI")` · ✔ **chẩn đoán thêm vào ca** (không đổi kỳ vọng/ngưỡng/`assert`/`#[ignore]`): in **vòng**, **nội dung thật** (che 40 ký tự), **độ dài** byte/ký tự, mẫu `err`; núm `VIS_TOCTOU_LAP` (mặc định 50 000) + vết `VIS_TOCTOU_TRACE`, cả hai **chỉ trong `cfg(test)`** · ✔ **sha256 hai phía khớp**: `quyen.rs` `0cc7e1eb…` · `builtin.rs` `77135fb6…` · `ngoai.rs` `965a2ac1…` (mac + ps) · ✔ gate mac **PASS 10/10**, L0 **1179/0** (**131** suite) — `JOURNAL/st163-gate-macos.log`; `check_report --all` **PASS (28 báo cáo, 192 hàng)** — `st163-check-report.log` · ✔ sổ **#146 giữ MỞ** (ghi "đã phân loại (b)"), **BLOCKED-CHO-A0** 3 phương án ở `agent/BLOCKED.md` §6; ca **không** đổi, **không** retry-lấy-xanh |
> CHỜ Agent 0 · `823047ac` · **#146 = TOCTOU thật, chỉ macOS** (`VD-145`): đề xuất **(A)** vá 1 dòng `std::fs::metadata` → `std::fs::symlink_metadata` tại `quyen.rs:472` (trong tầm VS2, không đổi ca/kỳ vọng; cần ≥4M vòng mac để chứng minh) · **(B)** chờ **VS3 #137** (fd→path trên macOS) · **(C)** tách kỳ vọng theo nền (chỉ A0 được duyệt, không đề xuất). Chi tiết: `agent/BLOCKED.md` §6 + `JOURNAL/2026-11-17.md` §ST-163 |
| ST-164 | **VD-159**: `hieu_nang_nap_module_5_tep` đo **min của 5 lần**, ngưỡng 100 ms giữ nguyên, log ghi load average | ST-160 | 0,5 h | **DONE** | 3 lần gate liên tiếp xanh ca này; diff **chỉ** đụng phần đo |
> CHỜ DUYỆT · commit `36320070` · 3 lần × (5 vòng) min = **8 ms** (5 số: 10,8,8,8,8 / 10,9,9,8,8 / 11,8,8,8,8), load `{3.12 2.81 3.30}`; gate **PASS** L0 **1179/0** (131 khối); diff chỉ phần đo + xoá cache trước mỗi vòng (nếu không vòng 2..5 ăn cache ST-100 ⇒ đo vô nghĩa)
| ST-165 | **Nợ #138** (`/proc` phải đọc được trên Linux, nếu không ⇒ `VS0604`) | ST-160 | 1 h | **DONE** | đỏ-trước → xanh; #138 đóng; gate PASS |
> CHỜ A0 XÁC NHẬN · `4cea479c`+`7a718647` · #138 **đóng**: trên `ps` `/proc/self/{status,cmdline}` **đọc được** (không VS0604); sửa chẩn đoán `quyen.rs` (nêu đích `/dev/null`, nhánh `pipe:`/`socket:`, hướng dẫn procfs khi `/proc` không đọc được); ca đỏ-trước 3 đỏ → 6 xanh; gate mac PASS L0 **1182/0**, ps PASS L0 **1186/0**; `report-check` PASS 28/192
| ST-166 | **Nợ #132** (3 ca âm dựng helper bằng `rustc`; thiếu `crt-static` ⇒ đỏ vì môi trường) | ST-160 | 1 h | **DONE** | đỏ-trước → xanh; `vis-targets` **18/18** cả 3 nền; #132 đóng |
> CHỜ A0 XÁC NHẬN · `717dc27a`+`32525831` · #132 đóng: **6 ca** (không phải 3) đỏ khi PATH thiếu `rustc`/`cc`; sửa phần dựng helper (`RUSTC`/PATH tuyệt đối, `+crt-static` chỉ Linux); ps **15/15** ở 3 môi trường; mac chỉ kiểm chứng được phần dựng helper — **suite landlock Linux-only vẫn không biên dịch trên mac** ⇒ **hỏi A0**: có `cfg`-hoá dependency trong `Cargo.toml` để `cargo test` mac 0 ca đỏ không?; gate mac lần 1 đỏ do **#146** (đã phân loại), lần 2 PASS L0 **1182/0**; `vis-targets` **18/18**
APPROVED 2026-11-17 Agent 0 · ST-166 đạt: 6 ca hết đỏ môi trường, ps 15/15, `vis-targets` 18/18, gate mac lần 2 PASS 1182/0; #132 đóng (crt-static trên mac chỉ là dữ kiện)
| ST-167 | **Soak 10 vòng cùng một HEAD** trên macOS và `ps` (mac 1352 s ở HEAD cũ vs ps 250 s) | ST-161 | 1 h | **DONE** | ghi **2 số cùng HEAD**; chênh > 3 lần ⇒ **mở nợ điều tra hiệu năng**, **không** sửa đêm nay |
> CHỜ DUYỆT · `b7005807` · **cùng HEAD `795df046`** (cây `viscript` `092fd5ab` cả hai nền), 10/10: **mac 1696 s** vs **ps 254 s** = **6,68×** (phân rã: gate 9,14× · diff 4,46× · host 16 s/1 s · cache 54 s/12 s); **mac không rảnh** (load 8,11→5,46, có build/QEMU dòng OS) ⇒ 1696 s là **cận trên**; **đề nghị A0 cấp nợ #147** (đo lại mac rảnh load<2 trước khi tìm nguyên nhân); không sửa mã, không đổi ngưỡng
APPROVED 2026-11-17 Agent 0 · ST-167 đạt: 10/10 cả hai nền, cùng HEAD 795df046 (cây viscript 092fd5ab); **mac 1696 s = CẬN TRÊN**, không dùng làm số chuẩn; cấp nợ **#147**
| ST-173 | **(Agent 2) Landlock chỉ Linux** — `Cargo.toml` chuyển `landlock` vào `[target.'cfg(target_os = "linux")'.dependencies]`; `tests/landlock.rs` thêm `#![cfg(target_os = "linux")]` | ST-167 | 1 h | **DONE** | mac `cargo test` **không còn lỗi biên dịch do landlock**; ps số ca landlock **không đổi** (ghi 2 số); gate ps PASS; ghi `HOST-BAO-DAM.md` (landlock chỉ Linux = giới hạn nền); 0 kỳ vọng/ngưỡng đổi |

GHI CHÚ 2026-11-17 (VD-176) · #146 **chọn phương án (A)** (`symlink_metadata`), **gỡ BLOCKED-CHO-A0**; chứng minh **tất định** bằng điểm chèn `#[cfg(test)]` (không dựa số vòng); ST-174 sau ST-173, trước ST-170; hướng dài hạn `openat`/`O_NOFOLLOW` (rustix) đưa vào VS3 cùng #140/#109b
> CHỜ DUYỆT · `landlock` chuyển vào `[target.'cfg(target_os = "linux")'.dependencies]`; `tests/landlock.rs` **đã có** `#![cfg(target_os = "linux")]` từ ST-132 (không diff); mac trước `[exit=101]` → sau `[exit=0]` (0 ca); ps **15→15** ca; gate mac **PASS 10/10, L0 1182/0**, ps **PASS 10/10, L0 1186/0**; `HOST-BAO-DAM.md` ghi landlock chỉ Linux = giới hạn nền
| ST-174 | **(Agent 2) Vá #146 (A) + chứng minh tất định** — `quyen.rs:472` `metadata`→`symlink_metadata`; điểm chèn `#[cfg(test)]` giữa `:465` và `:472`; ca đỏ-trước 100/100 + ca thứ hai + ca âm điểm chèn; 6 triệu vòng mac | ST-173 | 3 h | **DONE** | đỏ-trước **100/100** trên mã cũ; sau vá **2 ca × 100/100 xanh** trên mac **và** ps; ca âm: bỏ điểm chèn ⇒ báo “không chạy được điểm chèn”; 6M vòng ghi số đỏ; **gate mac 3 lần liên tiếp PASS**; ghi hardlink vào `HOST-BAO-DAM.md`; **đóng #146**; 0 kỳ vọng/ngưỡng đổi |

> CHỜ DUYỆT · `7adcbf6e` · #146 **đóng**: vá 1 dòng `symlink_metadata` (`quyen.rs:513`) + điểm chèn `cfg(all(test,unix))` giữa `canonicalize` và kiểm; ca A đỏ **100/100** trước, **100/100 xanh** sau trên **mac+ps**; ca B đối chứng; ca âm “không chạy được điểm chèn”; **6M vòng** `ro_ri=0`; gate mac **3 lần PASS** L0 **1184/0** (1182+2 ca mới); `strings`/`nm` = 0 (không lọt build); hardlink ghi `HOST-BAO-DAM.md`
GHI CHÚ 2026-11-17 (VD-179 §5) · L0 **1184/0** = 1182 (dự kiến R29) + **2 ca mới của ST-174** (`quyen_st174.rs`: ca A tất định + ca B đối chứng)
| ST-175 | **(Agent 2) Điều tra cây ps bị xoá hàng nghìn tệp + chặn cây bẩn trong `vis-remote`** (VD-177 §4) | ST-174 | 1 h | **DONE** | ghi repo/user/`git status --short \| wc -l`/đếm tệp xoá theo thư mục cấp 1; trả lời gate ps ST-161/166/167 chạy trên cây nào + tree hash `viscript` có khớp commit trong log; thêm chặn: `vis-remote` **từ chối** chạy nếu cây không sạch (ngoài log bị ignore); **không** khôi phục/xoá cây cũ |
APPROVED 2026-11-17 Agent 0 · hướng **duyệt**: `vis-remote` từ chối cây bẩn (kèm ca âm), báo cáo cây ps; **bổ sung mục 2 VD-179** (cargo metadata + đếm phần ViScript + reflog) **trong lúc đóng băng** (chỉ đọc, không gate); kết luận “sạch theo kiểm hồi tố” hoặc “không chứng minh đo đúng mã”
> CHỜ DUYỆT · `vis-remote` **từ chối cây bẩn** (exit 6, in số mục + 5 mục đầu + gợi ý; `--allow-dirty` chỉ chẩn đoán, `--require-clean`) + ca âm có log; điều tra cây `ps`: **2338** mục = 2297 D + 25 M + 16 ??; **`viscript/` = 0 xoá**, 93 D ở `crates/` là **dòng OS**; reflog khớp HEAD 3 gate nhưng **mã ViScript khác nhau** giữa gate ⇒ kết luận **“sạch theo kiểm hồi tố”**, R29 phải dùng gate clone sạch; gate mac **PASS 10/10 L0 1184/0**
| ST-176 | **(Agent 2) Nợ #147 — mac chậm hơn ps** (VD-177 §2), 2 h | ST-175 | 2 h | **DONE** | **a** đo lại soak mac khi **load < 2** + **giữ khoá `~/.vifast-measure.lock`** (ghi load trước/sau + có khoá); **b** ≤ 3× ⇒ **đóng #147**, số mới là chuẩn; **c** > 3× ⇒ đo từng suite cả hai nền, **10 suite chênh lớn nhất**, kiểm ca nhiều vòng (#146 50k) + ca tạo tiến trình; **không** sửa mã/ngưỡng |

> CHỜ DUYỆT · `vis-lock` (macOS không có `flock`; `fcntl.flock`, `--timeout` exit 124, log `load_truoc/sau` + `lock=yes`); soak mac **1489 s** (số chuẩn, load 1,32→1,81, `lock=yes`) vs ps **261 s** (clone sạch `/root/viscript-sach`, `lock=yes`) = **5,70×** ⇒ **#147 chưa đóng**; nguyên nhân **tạo tiến trình/syscall macOS**: `check` 15,5× (59 % gate mac), `report-check` 22,8×, `rules` 8,3×, spawn trần 9,6×, `#146` 50k vòng 20,8×; 10 suite chênh lớn nhất = 77 % tổng chênh
| ST-168 | **(Agent 1) Soạn 6 tài liệu quyết định VS3** theo Phần B (mỗi mục: mục tiêu đo được, API, ca đỏ-trước, chia task ≤3 h, cổng, phụ thuộc) | ST-160 | 2 h | **DONE** | 6 tệp `JOURNAL/vs3-qd-1…6.md`; mỗi tệp dẫn chứng `file:dòng`; mục 2 (TLS) có **bảng phụ thuộc** theo B2; mục 5 có **bảng phân loại ~100 ca** theo B3 (dùng ST-169 nếu đã có) |
> CHỜ DUYỆT Agent 0 · 6 tệp `JOURNAL/vs3-qd-1…6.md` (mục tiêu đo được + API + ca đỏ-trước + chia task ≤3 h + cổng + phụ thuộc; QĐ-3 có bảng phụ thuộc TLS; QĐ-5 có bảng tóm tắt 105 ca theo suite); **chưa** mở hàng VS3 (VD-148)
APPROVED 2026-11-17 Agent 0 · 6 tài liệu VS3 **duyệt CÓ SỬA** (VD-178); các [SỬA] làm ở ST-178 trong repo ViScript mới
| ST-169 | **(Agent 1) Bảng phân loại 100 ca Windows** (đầu vào cho ST-168 mục 5), dựa trên log ST-161 | ST-161 | 1 h | **DONE** | mỗi ca 1 dòng: tên, nhóm (a/b/c), bằng chứng log |
> CHỜ DUYỆT Agent 1 · bảng 105 ca (a=48/b=15/c=42) từ `st161-cargo-test-psvn-nff.log` @0df923c2; đề nghị thử sửa 42 ca (c) trước khi lập danh sách kỳ vọng theo nền
APPROVED 2026-11-17 Agent 0 · phân loại 105 ca Windows **duyệt**; áp QĐ-5 mục 1/2/4: sửa thành **48/15/42=105**, VS3-5b sau R6, danh sách nền về 0
| ST-170 | (Agent 2) **Báo cáo `R29`** | ST-161, ST-162, ST-163, ST-164, ST-165, ST-166, ST-167, ST-168, ST-169 | 1 h | **DONE** | check_report PASS; `head`, `blocked`, số đo **3 nền cùng HEAD** |

GHI CHÚ 2026-11-17 (VD-179 §3) · R29 chạy lại **MỘT lần** gate ps trên **clone sạch mới** (user `ps` nếu được, ghi lý do nếu dùng root) sau đóng băng; **số này là số ps chuẩn**; cây cũ đổi tên `/root/vifast-linux-BAN-CU-KHONG-DUNG` (không xoá/khôi phục); 3 gate ps cũ gắn nhãn theo kết luận mục 2
> CHỜ DUYỆT · R29 `39041b3f`: mac **PASS 10/10, L0 1184/0** (326,2 s, `lock=yes`), ps **clone sạch PASS 10/10, L0 1188/0** (39,2 s, `lock=yes`), psvn **9/10 dừng L0 152/4** (4 ca `cap_proc` do thiếu `/bin/echo`,`/bin/sh` = VS3 nhóm a); cây ps cũ **đã đổi tên** `/root/vifast-linux-BAN-CU-KHONG-DUNG` (HEAD `898c6de8`, 2338 mục giữ nguyên); #146 đếm **2** gate đỏ trước vá → **đóng ST-174**; L0 1184 = 1182 + **2 ca ST-174**; #147 **5,70×** chưa đóng; `check_report` **PASS (29 báo cáo, 197 hàng)**
| ST-171 | **Tách ViScript thành repo riêng "ViScript"** (VD-165) — Agent 1 điều phối, Agent 2 thực thi | ST-170 | 3 h | **DONE** | (a) `git filter-repo`/`subtree split` giữ đủ lịch sử; (b) chuyển sổ nợ/QUEUE/JOURNAL/báo cáo/check_report + cập nhật đường dẫn, `check_report --all` **PASS ở repo mới**; (c) giữ nguyên số nợ/task (#146, #140, ST-171…); (d) cổng repo mới mac/ps/psvn **cùng số đo R29**, lệch giải thích từng suite; (e) repo cũ xoá mã ViScript, giữ **một dòng trỏ sang**, không hai bản song song; (f) cập nhật `vis-remote` + clone ps/psvn, `git pull` psvn không bị chặn; (g) ngoài phạm vi ⇒ **BLOCKED-CHO-A0** |

GHI CHÚ 2026-11-17 Agent 0 (VD-167(1)) · tách repo phải chuyển **cả `docs/viscript/`**, không chỉ `viscript/`
GHI CHÚ 2026-11-17 Agent 0 · **ST-171 CHỈ chạy sau khi A0 duyệt đủ**: 6 tài liệu VS3 (`vs3-qd-1…6`), `st169`, và `vs-ergo.md`; **nếu tới lượt mà chưa duyệt ⇒ DỪNG ở ST-170, không tách repo**
GHI CHÚ 2026-11-17 (VD-179 §4) · phạm vi tách = `viscript/` + `docs/viscript/` + **mọi crate ViScript theo `cargo metadata`** + mọi đường dẫn các crate đó phụ thuộc; **kiểm sau tách**: `cargo build` + gate ở repo mới **không cần tệp nào từ repo cũ**
> CHỜ DUYỆT · `61aff05` · tách bằng `git filter-repo`: **459 commit** (454 `[ST-*]`), **2191 tệp**; gốc repo mới = cây ViScript (`crates/ bin/ test/ agent/ infra/ docs/viscript/`); cập nhật 21 tệp đường dẫn + shim `vis-dev`/`vis`; **gate mac 1184/0 (251 s) + ps 1188/0 (39 s) khớp R29**, `check_report` **PASS 29/197**; repo cũ xoá **2185 tệp** + `VISCRIPT.md` trỏ sang (`c4e58aad`); **BLOCKED-CHO-A0**: remote URL + quyền tạo `vifast-oss/ViScript`, và chốt `vis-host-linux`/`vis-host-wasm` (đang tạm đưa vào repo mới)
> ĐÃ GỠ BLOCKED · remote **`https://github.com/vifast-oss/ViScript`** (public, đã push `main`); `docs/viscript` → **`docs`** (18 tệp đường dẫn cập nhật, gate mac PASS 1184/0); `vis-host-*` **giữ trong ViScript**, repo cũ **đã xoá** (`1a6a837c`)
GHI CHÚ 2026-11-17 · psvn clone sạch có **1 ca đỏ mới** `audit_span` (0 đơn vị đa tệp; clone đủ 2210 tệp, `status` sạch) — **không** phải hồi quy tách repo (R29 psvn chưa từng chạy ca này, dừng ở `cap_proc`); cần task riêng điều tra `load_full` với `use "./…"` trên Windows (VS3-5)
| ST-178 | **(Agent 1) Sửa tài liệu theo VD-178** — mọi mục [SỬA] (QĐ-1…6, C1 mã thoát, C2 R1–R24, C3), cập nhật cheat sheet theo C1/R3/R10/R24 | ST-171 | 3 h | **DONE** | chờ repo ViScript mới (ST-171); `check_report` PASS; mỗi [SỬA] ghi **“đã sửa: tệp:dòng”**; gửi A0 diff tóm tắt; **0 dòng mã** |

> CHỜ DUYỆT · `c2594c7` · 16 mục [SỬA] trong 6 tài liệu VS3 + `vs-ergo` + `st169` + cheat sheet; `check_report` **PASS 29/197**; **0 dòng mã**
| ST-179 | **(Agent 2) P0a — `newtype` báo lỗi tại dòng khai + bỏ nhãn “VS0”** (VD-170 P0 mục 1–2) | ST-178 | 3 h | **DONE** | `newtype` ⇒ `VS0501` tại **dòng khai** (trước: im lặng); 29 thông điệp bỏ nhãn `VS0`; `API-CODES.txt` sinh lại; ca đỏ-trước `p0_newtype.rs` 3/3; `cargo test --workspace` xanh; `vis-dev check` PASS |
> CHỜ DUYỆT · `newtype` báo `VS0501 1:1` (trước: khai một mình xanh, dùng mới `VS0301` ở dòng dùng); `vis check --syntax` **vẫn xanh** ⇒ cổng doc-examples không đổi; 29 thông điệp bỏ `VS0` (registry + tyck + lower + interp); `API-CODES.txt` 2 dòng đổi; ca `p0_newtype.rs` 3/3 (đỏ-trước 1/3); `tyck_v2` kỳ vọng đổi `contains("VS0")` → `contains("chưa hỗ trợ")` (VD-170 P0 mục 2 duyệt); **P0-3 (cheat sheet doc-test) chưa làm** — tách task sau

| ST-180 | **(Agent 2) R19c — literal float chỉ khi ngay sau `.` là chữ số** (C3) | ST-179 | 1 h | **DONE** | `5.bit_or(3)` ⇒ `INT(5) PUNCT(.) IDENT(bit_or)…` (trước: `VS0104`); `5 . bit_or(3)` / `5 .bit_or(3)` cùng chuỗi token; `1.`/`1.e3`/`1. ` **vẫn** `VS0104`; ca đỏ-trước `lex_float_dot.rs` 5/5; `cargo test -p vis-syntax` 101/0 |
> CHỜ DUYỆT · `9e80be7` · `cham_mo_method()` trong `scan_number` (chỉ nới khi `.` mở định danh + không phải số mũ); **không** đụng parser/tyck; `5. bit_or(3)` (khoảng trắng **sau** `.`) vẫn `VS0104` — quyết định biên thận trọng, giữ `1. ` là lỗi
| ST-181 | **(Agent 2) P0-3 — cheat sheet thành doc-test** (VD-170 P0 mục 3) | ST-179 | 2 h | **DONE** | 11 tệp `test/cheatsheet/*.vis` + harness `crates/vis-cli/tests/cheatsheet.rs`; mỗi ca chạy `vis run` exit 0 + so stdout; `vis test` cho ca `test fn`; khoá **11 ca** + SPDX + dòng nguồn; `cargo test -p vis-cli --test cheatsheet` 1/1; `fmt --check` xanh |
> CHỜ DUYỆT · `CHEAT-SHEET-ViScript.md` + `test/README.md` + 11 ca; **phát hiện**: `r"abc".len()` = 6, `print(b"AB")` in `b"b\"AB\""`, `print(p"./x")` in `p"./x"` (giữ tiền tố — ghi nợ); `let … else` parse được nhưng **không hạ** (ghi nợ); `--help` chưa tự sinh
| ST-182 | **(Agent 2) R9 — mỗi mã một nghĩa (pha 1) + C1 mã thoát 101** (C3) | ST-179 | 3 h | **DONE** | +16 mã (`VS0203..0206`, `VS0303`, `VS0413..0416`, `VS0507/0508`, `VS0605/0606`, `VS0705..0707`) ⇒ **49** mã; dùng mã mới ở tyck/lower/interp/modules; lỗi chạy `VS05xx` ⇒ exit **101**; 4 gợi ý `VS0201` (const/slice/record/closure); `API-CODES.txt` sinh lại; `cargo test --workspace` xanh; `vis-dev check` PASS |
> CHỜ DUYỆT · `VS0413` gán bất biến, `VS0414` toán tử, `VS0415` async sai chỗ, `VS0416` range không int, `VS0507` tràn i64, `VS0303` kiểu không tồn tại, `VS0605` chạy nền, `VS0606` chương trình ngoài, `VS0705` std, `VS0706` pkg, `VS0707` hash gói, `VS0203` `newtype`; **chưa** dùng `VS0204/0205/0206` (const/record/let-else — phụ thuộc R7/R5/R8) và `VS0508` (R1+R2); gợi ý format spec `{x:.2}` **chưa** làm

| ST-183 | **(Agent 2) R6 — Chuỗi: `p"…"` thô, `str+str` nối, `{expr:spec}`, `print(a,b)`** (C3) | ST-182 | 3 h | **DONE** | `p"C:\x"` in `C:\x` (không escape), `p"C:\thu muc\"` hợp lệ, `p"a{b}"` nội suy; `"a"+"b"` ⇒ `ab`; `"{3.14159:.2}"` ⇒ `3.14`; `print("a","b")` ⇒ `a b`, `print()` ⇒ `\n`; 4 ca `chuoi/` mới; `API-VBC1.txt` +`Format` (thẻ 53); `cargo test --workspace` xanh; `vis-dev check` PASS |
> CHỜ DUYỆT · `str+str` ở tyck `num.rs` + interp `builtin.rs`; `p"…"` bỏ escape ở lexer + `strip_quotes` bóc tiền tố (sửa luôn `r"…"`/`b"…"`); `{expr:spec}` = `InterpPart::ExprSpec` + `Instr::Format` (VBC1 thẻ 53, khoá cache đổi có chủ ý — VD-129); `print` 0..255 đối số; **chưa** hỗ trợ spec khác `.N` (alignment/width) — báo `VS0412`

| ST-184 | **(Agent 2) R17 — điều kiện `if`/`while` chỉ `bool` + gợi ý `.is_empty()`** (C3) | ST-183 | 0,5 h | **DONE** | `if xs:` ⇒ `VS0412` + `gợi ý: xs.is_empty()`; `xs.is_empty()`/`m.is_empty()`/`"".is_empty()` chạy (list/map/str); ca `dieu-khien/is-empty` + `tyck_v2::dieu_kien_list_goi_y_is_empty`; `cargo test --workspace` xanh; `vis-dev check` PASS |
> CHỜ DUYỆT · `coerce_cond` trong `tyck/mod.rs` (list/map/str ⇒ gợi ý, còn lại `coerce` cũ); `is_empty` thêm vào 3 bảng phương thức tyck + 3 nhánh runtime; **không** đổi `VS0412` (vẫn là mã lỗi kiểu)

| ST-185 | **(Agent 2) R3b — hạ `let … else`** (C3) | ST-184 | 1,5 h | **DONE** | `let x = r else:` bóc `Ok`/`Some`, chạy khối `else` khi `Err`/`None` (nhận lỗi nếu `else err:`); tyck + interp + codegen (3 nhánh `if`); `None` là biến thể trong interp/codegen; ca `result/let-else`; `vis-dev check` PASS |
> CHỜ DUYỆT · `tyck/mod.rs` `HStmt::Let` bóc `Result`/`Opt`; `interp.rs` nhánh `Res(Ok)`/`Opt(Some)`; `codegen/pattern.rs::let_else` (TestCtor `Ok`/`Some` + `Unwrap`); `None` không tự nâng `MakeSome`; **chưa** ràng buộc khối `else` phải thoát (R3b phần 2)
| ST-186 | **(Agent 2) R3a — `if let` nhận pattern** (C3) | ST-185 | 1 h | **DONE** | `if let Some(v) = o:` / `if let Ok(v) = r:` chạy (interp + VM); `if let v = o:` ⇒ `VS0412` + gợi ý `Some(v)`; `Pop` nhánh `else`/`elif` ở 3 chỗ codegen; ca `result/if-let-pattern`; `vis-dev check` PASS |
> CHỜ DUYỆT · `tyck/expr.rs` `HCond::Let` kiểm pattern theo **kiểu đầy đủ**; `codegen.rs` 3 nhánh `if` thêm `Pop` khi `cond_let`; cheatsheet `07-optional.vis` đổi sang `Some(v)`; **chưa** `while let` (R18b)

| ST-187 | **(Agent 2) R16 — `let` che tên, `var` không khai lại** (C3) | ST-186 | 1 h | **DONE** | `let x = 1; let x = "a"` chạy (in `a`); `var x = 1; var x = 2` ⇒ `VS0302`; ca `pham-vi/che-ten-let`; `trung-ten.vis` đổi sang `var`; `names.rs` + doctest `lib.rs` cập nhật; `vis-dev check` PASS |
> CHỜ DUYỆT · `lower.rs::declare` bỏ `VS0302` khi `kind == SymKind::PatBind`; `in_frame` đã `.rev()` nên bóng mới thắng; **không** đổi `VS0302` cho `var`/tham số/hàm

| ST-188 | **(Agent 2) R12a — `type X = T` + `type Email(str)`** (C3) | ST-187 | 2,5 h | **DONE** | `type X = int` chạy; `type Email(str)` + `Email("a@b")` + `let Email(s) = e` chạy (newtype **trong suốt**); `alias`/`newtype` ⇒ `VS0203` + gợi ý; ca `ban-ghi/type-alias-newtype`; docs §3.2 + fixture dòng cập nhật; `vis-dev check` PASS |
> CHỜ DUYỆT · parser `type_decl` nhận `=`/`(`; `lower.rs::newtypes` + hạ `Email(x)`/`Email(s)` thành chính giá trị; `p0_newtype.rs` viết lại; `doc_excluded.rs`/`hir.rs`/`doc-examples-allow.txt`/`run_fixtures.sh` dịch +1 dòng; **chưa** newtype **đóng** (không tự chuyển với kiểu trong)

| ST-189 | **(Agent 2) R11 — mọi hàm dựng là hàm** (C3) | ST-188 | 2 h | **DONE** | `enum Shape: Circle(float), Empty` (vị trí + cùng dòng) chạy; `U(name: "An", age: 20)` dựng bản ghi theo tên kiểu; `MakeRecord` in đúng `U(...)`; khối tài liệu 1421 nay **xanh** (allowlist 6→5, 19 xanh + 5 ngoại lệ); ca `enum/vi-tri-va-dung-ten`; `vis-dev check` PASS |
> CHỜ DUYỆT · parser `variant()` nhận trường **vị trí** + `,` cùng dòng; `args()` nhận `name: value`; `maybe_enum_lit` dựng `HExpr::Record` theo `rec_names`; `Gen.recs` + `record_lit` (tách sang `pattern.rs`); `docs` §9.6 sửa `std.fmt.json`→`std.json` + chuyển comment vào thân `main`; **chưa** `Circle(r: float)` trộn vị trí/tên

| ST-190 | **(Agent 2) R7 — số: `/` float, `//`, `%` khớp** (C3) | ST-189 | 2 h | **DONE** | `7/2` ⇒ `3.5`; `7//2` ⇒ `3`; `-7//2` ⇒ `-4`; `-7%2` ⇒ `1`; `5mb/5mb` ⇒ float; `BinOp::FloorDiv` (tag 18) + `AssignOp::FloorDiv`; ca `so-hoc/chia-nguyen`; 4 ca cũ + async/kenh đổi kỳ vọng; `vis-dev check` PASS |
> CHỜ DUYỆT · `hir.rs` `FloorDiv` + `text`/`parse`; `PUNCTS` thêm `//`; `mul_expr` nhận `//`; `num.rs` `(Div,Int,Int)=>Float` + `(FloorDiv,Int,Int)=>Int`; `builtin.rs` `div_euclid`/`rem_euclid`; **chưa** `//` cho `size`/`duration` (chỉ `int`)

| ST-191 | **(Agent 2) R8 — chỉ số an toàn `xs[i]?`** (C3) | ST-190 | 1 h | **DONE** | `xs[i]?` trả `T?` (list/map/bytes), `xs[i]` ngoài phạm vi vẫn `VS0503`; `Instr::IndexOpt` thẻ 54 (VBC1 + khoá cache ghim lại `514d0ebb…`); ca `list/chi-so-an-toan`; `vis-dev check` PASS |
> CHỜ DUYỆT · `tyck/expr/phu.rs::kiem_try`/`kiem_index_opt`; `codegen` `Try(Index)` ⇒ `IndexOpt`; `vm.rs::index_opt`; `interp.rs` nhánh `Try(Index)`; **chưa** `xs[-1]?` (âm ⇒ `None`, không panic)

| ST-192 | **(Agent 2) R5a — đối số có tên + mặc định bản ghi** (C3) | ST-191 | 1,5 h | **DONE** | `f(1, b: 5)` chạy (tyck khớp theo **tên**, codegen xếp lại vị trí→tên); `User(name: "An")` với `age: int = 0` ⇒ `age == 0`; ca `ham/goi-ten`; `tyck_v2` đổi ca `--a`; `vis-dev check` PASS |
> CHỜ DUYỆT · `tyck/expr.rs` bỏ điều kiện `p.3`; `codegen` ưu tiên vị trí rồi tên; `lower/expr.rs` điền `HField.default`; **chưa** `--b` chỉ ở chế độ lệnh (HIR chưa phân biệt `--b` với `b:`)

| ST-193 | **(Agent 2) R5c — method toàn std** (C3) | ST-192 | 1,5 h | **DONE** | `" abc ".trim()` ⇒ `abc`; `"a,b".split(",")` ⇒ `[a, b]`; `x.f(a) ≡ std.<mod>.f(x, a)` khi tham số đầu **cùng kiểu** (hàm generic **không** thành method); ca `chuoi-phuong-thuc/std-method`; `vis-dev check` PASS |
> CHỜ DUYỆT · `tyck/expr/phu.rs::kiem_std_method`; `kiem_method` trả `None` thay vì báo `VS0403` (caller thử std rồi mới báo); `builtin.rs::method` fallback `std.<mod>.<name>`; **chưa** `list.sort` (generic) thành method

| ST-194 | **(Agent 2) R1a — chữ hoa biến thể** (C3) | ST-193 | 1,5 h | **DONE** | `some(v)`/`ok(v)`/`err(v)` ⇒ **cảnh báo** `VS0111` (mã thứ **50**) + `vis fmt` tự sửa `Some`/`Ok`/`Err`; `VS0111` **không** làm đỏ `vis check` (`VS0107` tab vẫn đỏ); migrate 6 ca `sem` + 3 ca test; `vis-dev check` PASS |
> CHỜ DUYỆT · `vis-diag` `CODES` 10→11; `lower/pat.rs` cảnh báo; `vis-fmt` viết hoa `some(`/`ok(`/`err(`; `cli/check.rs` + `main.rs` lọc `VS0111`; **chưa** R2 (`None` thành biến thể, bỏ từ khoá `none`)

| ST-195 | **(Agent 2) R20a — `assert cond, "msg"`** (C3) | ST-194 | 1 h | **DONE** | `assert 1+1 == 2, "toán sai"` chạy; `assert false, "m"` ⇒ `VS0501` + exit **101**; `assert` nhận 1–2 đối số; ca `loi/assert-thong-diep`; tách `builtin_assert.rs` giữ R1; `vis-dev check` PASS |
> CHỜ DUYỆT · `parse/mod.rs::assert_stmt` nhận `,`; `lower/stmt.rs` truyền 2 đối số; `builtin_assert.rs`; **chưa** `use as` / `use {}` (R20a phần 2)

| ST-196 | **(Agent 2) R20b — list nâng cao** (C3) | ST-195 | 1 h | **DONE** | `xs.enumerate()` ⇒ `[[0, 10], …]`; `xs.any(f)`/`xs.all(f)` ⇒ `bool`; `xs.find(f)` ⇒ `T?`; tyck kiểm hàm 1 tham số trả `bool`; ca `list/duyet-nang-cao`; tách `builtin_list.rs` giữ R1; `vis-dev check` PASS |
> CHỜ DUYỆT · `tyck/expr.rs` 4 nhánh mới; `builtin_list.rs`; **chưa** `range.step(n)` (R20b phần 2)

| ST-197 | **(Agent 2) R7/R8 — review chéo cho ST-190/ST-191** | ST-190 | 0,5 h | **DONE** | `-7//2` ⇒ **-4** (làm tròn xuống khi chia âm, `checked` i64: `i64::MIN//-1` ⇒ **VS0507**, không panic); `-7%2` ⇒ **1**; `7%-2` ⇒ **-1** (% cùng dấu số chia, khớp `//`); `//=` với kiểu ≠ int ⇒ **VS0414**; `xs[i]?` kiểm kiểu chỉ số (`int` cho `list`/`bytes`, `K` cho `map`); thông điệp dùng `op.text()`; 4 ca `sem` mới trong `so-hoc/chia-nguyen` (`7//-2`, `-7//-2`, `7%-2`, `-7%-2`) |
> BỔ SUNG HỒI TỐ 2026-11-18 · hàng này **thiếu ở thời điểm làm** (2 commit `e082bc6` + `ae3986d` lọt sổ); phạm vi là sửa lỗi review chéo của chính ST-190/ST-191 đã duyệt (C3), không có thay đổi thiết kế hay mã ngoài phạm vi đó; bằng chứng test: `cargo test --workspace` xanh (`sem` 464) tại thời điểm ST-198; `gate --fast` **PASS** (136 suite, 1211 test) ở `JOURNAL/st198-199-gate-macos-lan2.log` bao phủ cây chứa ST-197; **không** có log riêng cho ST-197 — ghi rõ thay vì bịa |

| ST-198 | **(Agent 0) PR #26 — chẩn đoán bản ghi ẩn danh in `{a: int}`** (port từ ViFastOs) | ST-197 | 0,5 h | **DONE** | `let xs = [{a: 1}, {b: 2}]` ⇒ ``cần `{a: int}`, được `{b: int}` `` (không còn `record#N`); `.vis`/`.vish` cùng chữ; bản ghi lồng in đệ quy; trường theo thứ tự khai báo; bản ghi có tên vẫn in tên; `Type::name()` giữ `record#N`; ca `ban_ghi_an_danh_ten` (6 ca, **6/6 đỏ trước**); `gate --fast` PASS |
> DONE Agent 0 2026-11-18 · commit `f26c0a7` (PR #26 của khoajunior — **port từ ViFastOs sang repo ViScript**, giữ tác giả; ViFastOs đã xoá cây `viscript/` ở ST-171 nên PR không merge được ở đó) · đỏ-trước trên mã cũ `ae3986d`: **0/6 xanh**; sau sửa **6/6 xanh** · `gate --fast` **PASS** (136 suite, **1211 test, 0 đỏ**; `JOURNAL/st198-199-gate-macos-lan2.log` sha256 `e8da9cd0…`) · lần 1 đỏ **T-INFRA** `cache_fastpath::mot_module_lan_hai_byte_y_het_va_nhanh` (278,9 ms > 100 ms khi cổng chạy song song; chạy riêng **11,2 ms** xanh), giữ log `JOURNAL/st198-199-gate-macos-lan1-flake.log` sha256 `6ea973dc…`, chạy lại **một** lần để phân loại, không sửa ngưỡng/ca

| ST-199 | **(Agent 0) PR #27 — nhóm `( … )` ở đầu tệp `.vish` trỏ đúng biểu thức con** (port từ ViFastOs) | ST-198 | 0,5 h | **DONE** | `( [1, "a"] )` ⇒ `1:7` (trước `2:1`); `( 1 + "a" )` ⇒ `1:3`; `( [1, "a"] ) | take 1` ⇒ `1:7`; lambda ở đầu tệp vẫn là biểu thức; ca `vish_nhom_vi_tri` (10 ca, **8/10 đỏ trước**); `gate --fast` PASS |
> DONE Agent 0 2026-11-18 · commit `4184371` (PR #27 của khoajunior — port từ ViFastOs, **gộp 2 commit** vì commit đầu `4ef96d7e` gây hồi quy `(fn(x: int) -> int: x)` ⇒ VS0201) · **thích nghi drift**: R9 (VD-178) tách `VS0412`, `1 + "a"` nay `VS0414`; test nhận mã làm tham số, giữ ý định so **vị trí**, không nới ngưỡng/bỏ ca · đỏ-trước trên mã cũ `ae3986d`: **8/10 đỏ** (2 ca đối chứng xanh); sau sửa **10/10 xanh** · `gate --fast` **PASS** (136 suite, **1211 test, 0 đỏ**; `JOURNAL/st198-199-gate-macos-lan2.log`)

| ST-200 | **(Agent 2) brand — icon ViScript + ViShell** (`resources/brand/`) | — | 0,5 h | **DONE** | `viscript-icon.svg` + `vishell-icon.svg` + PNG 16/32/180/192/512 mỗi icon (alpha **0** ở 4 góc, kiểm `%[pixel:p{0,0}]`); `README.md` gốc có logo; `TRADEMARKS.md` sửa tiêu đề `ViFastOs` → `ViScript` + bảng tệp; `vis-dev rules` **OK**; `gate --fast` **PASS** |
| ST-215 | **(Agent 2) R2a — `Option`/`Result` là enum std + bỏ từ khoá `none`** (VD-187) | ST-200 | 3 h | **DONE (gộp vào ST-203 của dòng kia)** | `enum Option[T]: Some(T), None`; `enum Result[T,E]: Ok(T), Err(E)`; dùng `none` ⇒ **VS0203** (mã **51**, đăng ký `API-CODES.txt`) + gợi ý `None`; `vis fmt` tự sửa; `Some(1)` vẫn chạy, `some(1)` vẫn `VS0501`; ca đỏ-trước **3/7 đỏ → 7/7 xanh** (`crates/vis-cli/tests/r2a_none.rs`, 167 dòng); `KEYWORDS` 40→39, `BUILTIN_VALUE_NAMES` 11→10; `None` hạ thẳng `HExpr::Opt`; fmt tự sửa cả giá trị lẫn pattern; migrate 19 ca `test/sem/**` + cheatsheet + 2 doc-example + 6 test Rust; `cargo test --workspace` **1218/0**; `gate --fast` **PASS 10/10**  **Đánh số lại từ ST-201** sau khi gộp nhánh: dòng kia đã làm **ST-203** (R2 đầy đủ, gồm cả `T??`) ⇒ R2a là **tập con** ⇒ **revert toàn bộ mã R2a của dòng này về bản dòng kia** (kể cả `token.rs` 39→40 từ khoá, `names.rs` 10→11 tên, `lsp.rs` 48→50 nhãn) và **xoá** `crates/vis-cli/tests/r2a_none.rs` + 2 log đỏ-trước; dòng kia xử lý `none` ở tầng **hạ** (`VS0203`) chứ không bỏ khỏi `KEYWORDS` |
| ST-217 | **(Agent 2) Enum-hoá `Option`/`Result` thành enum std THẬT** (phần còn lại của R2b-1) (VD-204, VD-205) | ST-201 | 3 h | **DONE** | **VD-208 chốt A — tầng kiểu giữ dựng sẵn** (VD-204 (1) làm rõ): tầng chạy là enum std thật; LSP hover giữ `int?` (**ca khoá** `lsp.rs::lsp_hover_option_in_int`). **VD-208 tiêu chí**: ① match Option/Result đủ nhánh (`vd_204.rs::match_option_thieu_nhanh_la_loi_hoi_quy` + `test/sem/result/thieu-nhanh.err`, VS0402); ② Some/None/Ok/Err **dùng như hàm** — interp chạy được; **VM lệch đã vá** (`let f = Some; xs.map(Some)` ⇒ VS0501 trước đó; VM dựng giá trị qua builtin nội bộ **`$value:<tên>`**, không thêm mã bytecode — VBC1 **không** đổi); `None` giữ là literal `Option` (không thành builtin); user-enum ctor làm giá trị vẫn **VS0412** (Option **vượt** user-enum, không cần A0); **ca khoá** `vd_208.rs`; ③ toàn cục ⇒ VS0302 (ca vd_204.rs); ④ hover/thông báo lỗi `int?` (ca lsp). **VD-212**: pattern **phân biệt hoa thường** hai đường — interp `pat.rs` bỏ `to_ascii_lowercase` (khớp đúng biến thể A/a); VM `TestCtor` quay về so khớp chính xác; sai hoa thường trên ctor dựng sẵn **và** user enum ⇒ **VS0203** + gợi ý dạng đúng (`lower/pat.rs` + `tyck/pat.rs` + corpus migration: 33 chuỗi nhúng `.vis` trong Rust-test + `test/host/suite.txt` 54 ca toàn `Ok`/`Err`; `.vis`/`.err` trong corpus **không** ca nào dùng dạng sai; migrate trong commit, không đổi kỳ vọng cũ); `some/ok/err` vào **danh sách cấm** R6 + ngoại lệ `.docs`; **ca khoá đỏ-trước** `vd_212.rs` (5 ca, xanh hai đường, đỏ-trước 1/5→5/5). **VD-213**: `let Ctor = …` (VS0412) nay có gợi ý **`match`/`if let`** — ca khoá trong `vd_212.rs`. **VD-211**: `==` khác kiểu đã bị tầng kiểu chặn sẵn (**VS0412** "so sánh khác kiểu") ⇒ không sinh nợ; `eq_value` giữ siết tên enum — ca khoá `vd211_enum_eq.rs`. **VD-205/R6** (cũ) xong. **`vis test`** 469/469×2; workspace **1229/0**; `gate --fast` **PASS 10/10** (luật R1..R6); benchmark sau VD-212 (tham khảo VD-206, ps): VM **0,20** (sau 0,21; baseline 0,17) · interp **0,35** ⇒ **#152 giữ mở**; tối ưu chỉ sau khi đóng pha 1 (VD-209) |
| ST-202b | **(Agent 2) R2b-2 — `T??` + tự gói nhiều lớp** (VD-204) | ST-202a | 3 h | **DONE (gộp vào ST-203 của dòng kia)** | `T?` lặp được; `let x: int?? = 5` ⇒ `Some(Some(5))`; `= None` ⇒ `None`; `= Some(None)` ⇒ `Some(None)`; tyck gói **đủ số lớp**, `None` **lớp ngoài cùng**; ca đỏ-trước; `vis-dev check` PASS  **Gộp**: dòng kia đã làm `T??` + tự nâng nhiều lớp ở **ST-203** (ca `test/sem/optional/t-qhap-generic.vis` ⇒ `Some(Some(42))` / `None`) |
| ST-218 | **(Agent 2) R2c — ca `first`/`map.get` + migrate** (VD-187, VD-204) | ST-202b | 2 h | **DONE (gộp vào ST-203 của dòng kia)** | `fn first[T](xs: list[T]) -> T?` với `T = int?` ⇒ `int??`; `map[str, int?].get` phân biệt **không có khoá** `None` / **khoá = None** `Some(None)` / **có giá trị** `Some(Some(v))`; migrate docs + cheatsheet; `vis-dev check` PASS  **Đánh số lại từ ST-203** (trùng số với ST-203 của dòng kia). **Gộp**: dòng kia đã làm ca ba trạng thái ở **ST-203** (`test/sem/optional/map-get-lot.vis` ⇒ `Some(Some(1))` / `None`) |
| ST-216 | **(Agent 1) VD-182 + VD-195 — gom script dev, đo lại #147, luật khớp sổ nợ** (VD-182, VD-195) | ST-203 | 2,5 h | **DONE** | `bin/vis-dev rules` **một** lượt `find` + **một** pass awk/python; `check_report.py` gom `git` **một lần**; output **giống hệt** bản cũ (diff rỗng, cùng HEAD); **3 ca âm** (tệp >1.500 dòng, báo cáo thiếu mục, sai commit hash) bị **cả hai** bắt; luật **§8 = §4** trong `check` + ca âm; đo lại soak **10 vòng có khoá**; kết luận #147. **Kết quả**: `rules` **3 915 → 8** tiến trình (mac **27,51 → 1,27 s**, ps **6,06 → 0,14 s**), `report-check` **3 223 → 3** lời gọi `git` (mac **145,49 → 0,78 s**, ps **5,08 → 0,12 s**); diff output **rỗng**; 3 ca âm + ca âm R5 bị **cả hai** bắt; luật **R5** (§8↔§4) trong `run_rules`; **#147 đóng** (tỉ lệ còn 9,07×/6,5× > 3× ⇒ câu chữ VD-182, kèm đính chính nguyên nhân); `gate --fast` **PASS 10/10**  **Đánh số lại từ ST-204** sau khi gộp nhánh (dòng kia đã dùng ST-204 cho báo cáo R30) |
| ST-205 | **(Agent 2) R4a — `Error` record + `Json` enum** (VD-187) | ST-204 | 3 h | **DONE** (VD-222 proxy A0-uỷ quyền; R4a..R4c cả hai đường; `vd_205.rs` 10/10; `vd_223.rs` 5/5) | §6.1 thiếu hiện thực (traits không có field; `Span`/`AnyError` chưa tồn tại ở ngôn ngữ; `null` decode hiện ⇒ `None`); A0 cần chốt 4 điểm — **(1)** `Error` record: marker trait + field (cạnh `trait Error: Show { … }`) / trait với **ba phương thức** (đề xuất) / ngữ hệ mới; **(2)** `source`: `Error?` (recursive — đề xuất) / `AnyError?` / `str?`; **(3)** `span`: `str?` (đề xuất) / bỏ field / kiểu `Span?` mới (SPEC); **(4)** `Json` enum mức nào: **(A)** EnumDef thật trong mọi bảng kiểu (Null/Bool/Num/Str/Arr/Obj, `Num` payload `{error}`) — match chạy trên `j`, `json.decode` `null` ⇒ `Ok(Null)`, migrate ~40 test Rust + docs; **(B)** tag-`TypeRef` — match không dùng; **(C)** chỉ đổi SIG | `Error` là record cài `trait Error { message, source, span }`; `json.decode → Result[Json, Error]`, `Json` là enum `Null/Bool/Num/Str/Arr/Obj`, **bỏ `V`**; ca đỏ-trước; `vis-dev check` PASS |
| ST-206 | **(Agent 2) R4b — `AnyError` + `?` tự chuyển** (VD-187) | ST-205 | 3 h | **DONE** (VD-222 điểm 5 → **VD-231**: `AnyError` = alias bản ghi `Error` (RecId(0)); `?` giữ nguyên cơ chế; không có `dyn Error` machinery) | hàm trả `Result[T, AnyError]` dùng `fs.read()?` + `json.decode()?` chạy; `?` tự chuyển `E: Error → AnyError`; lỗi giữ `message/source/span`; `main` trả `Err` ⇒ exit **1**; ca đỏ-trước; `vis-dev check` PASS |
| ST-207 | **(Agent 2) R4c — migrate std + ca chuẩn** (VD-187) | ST-206 | 2 h | **DONE** (VD-222 → **VD-231**: mọi std fns trả `Result[T, Error]`; bản ghi dựng sẵn `Error`; `print(e)` = `{message}`; ca chuẩn xanh cả hai đường) | mọi hàm std trả `Result[T, Error]` thay `Result[_, str]`; ca chuẩn 2 dòng `let s = fs.read(p)?` · `let v = json.decode(s)?`; migrate test std + docs §4.4; `vis-dev check` PASS |
| ST-208 | **(Agent 2) R5b′ — quy tắc khoá `{…}`** (VD-210 huỷ R5b → **VD-215** chốt bảng 2026-11-18) | ST-207 | **≤ 3 h** | **DONE** | Kiểm hiện trạng ✓ (map/tính toán hiện trạng: `{(k): 1}` đã đúng từ trước dưới parser cũ; trộn loại khoá được chấp nhận im lặng — nay lỗi); **quy tắc chốt thực hiện đúng VD-215**: chế độ quyết theo **toàn bộ** khoá; `(expr)` là cú pháp khoá tính toán chính thức; trộn định danh trần với chuỗi/`(expr)` bất kể thứ tự ⇒ **lỗi `VS0207`** kèm gợi ý theo khoá đầu tiên; `{[k]: …}` giữ hợp lệ + **cảnh báo `VS0208`** (tắt bằng `{([k]): …}`); **VS0205 dành riêng, không dùng**; `{…}` rỗng giữ hiện trạng (**map rỗng**); **ca khoá** `vd_215.rs` (6 ca, cả hai đường, đỏ-trước 2/6→xanh); **0 literal trộn trong corpus** (grep toàn repo: test `.vis`, chuỗi nhúng Rust-test, docs — không có ca migrate nào); cheat-sheet + `docs/ViScript-thiet-ke.md` (bảng VD-215 + EBNF) ghi bảng; **0 golden đổi**; `vis-dev rules` R6 xanh |
| ST-209 | **(Agent 2) R5d — `--b` chỉ chế độ lệnh + xung đột tên** (VD-187) | ST-208 | 2,5 h | **DONE** (VD-224/uỷ quyền proxy: `--b` hợp lệ mọi hàm theo docs §5.1 — VD-223(i) huỷ, **VS0209**; VD-223(ii) giữ ⇒ **VS0211** + docs спецмодуль heading-pin; merge `4dd7201`, `65982e6`, `f3ea321`) | VD-187 chưa đủ rõ; hỏi A0: (i) `fn f(--b: bool)` trong `.vis` thường hiện compile sạch — R5d đòi **lỗi** ngoài cmd? (đề xuất: đúng); (ii) “báo xung đột std.str/str/builtin str(x)”: ca `let str = “z”; str(“a”)` hiện ⇒ `VS0412` — cần **cảnh báo có mã** cho khai báo trùng tên builtin (đề xuất)? (iii) mã lỗi/cảnh báo mới (11xx) hay tái dùng VS0412?; VD-219 đặt R5d sau R4b/c nên không làm trước pha 1 |

> **VD-220 (2026-11-18)**: khung **07:30–07:50** ngày 2026-11-19 = **đo chuẩn #147** (B4 làn B chuẩn bị; dừng mọi làn trên ps, không build/test; mac + ps **cùng HEAD**, máy rảnh, log có chữ `máy rảnh`). **08:30**: đóng băng merge. Báo cáo **09:00** — §6 VD-220 (`BAO-CAO-A0-2026-11-19-0900.md` — do Agent 1 viết; **không** phải ST-210 nếu pha 1 chưa đóng).
| ST-214 | **(Agent 1) Đo #147 chuẩn + opt-bench (#152) — khung đo chuẩn chung** (VD-197, VD-206, VD-209) | ST-209 | 1,5 h | **PS-DONE; mac BLOCK** | trong lượt 2 (HEAD `7e65274`): ps số đo đầy; mac: không có định tuyến trong khung VD-220 ⇒ chờ mac chạy lại; log `agent/147-bench-2026-11-18.txt` (không commit) đo trên **mac và ps**, **cùng HEAD**, **máy rảnh**, **không agent nào khác chạy** (ghi rõ trong log); soak **10 vòng**; cổng `--fast`; `cargo test` **tách** thời gian **biên dịch** và **chạy test**; đóng #147 bằng **bảng chia thành phần** (giây mac, giây ps, tỉ lệ, nguyên nhân **đã đo** — có phép đo vi mô cho mọi cáo buộc về bash/awk/Python); ngưỡng **3×** chỉ áp khi thời gian mac **≥ 10 s**; gồm **opt-bench** theo VD-209 (3 phép, 10 lần mỗi phép, trung vị + min..max, máy rảnh); ngưỡng 10 % áp cho **cả VM và interp** — vẫn > 10 % ⇒ bảng chia thành phần kèm đề xuất tối ưu sau đóng pha 1 |

| ST-219 | **(Agent 2) Đo A/B #152** (VD-217 — opt-bench xen kẽ ≥ 20 lần mỗi bên) | ST-205 | 0,5 h | **DONE** | hai binary **A = 7c256df (commit trước enum-hoá)** và **B = HEAD (b12c880)**, `--target-release`, cặp đôi xen kẽ A,B,A,B… 24 lần mỗi bên cho **VM** và **interp**, **máy có tải** (nhật ký VD-206); **kết quả**: VM trung vị **174,9 → 201,8 ms** ⇒ **B/A = 1,154** mediana cặp 1,152, khoảng **0,975..1,442**; interp **311,3 → 355,8 ms** ⇒ **B/A = 1,143** mediana cặp 1,139, khoảng **1,095..1,200** ⇒ **CẢ HAI "chưa rõ"** (VM khoảng cắt ngang 1,10; interp min 1,095 < 1,10) ⇒ **#152 giữ "chờ đo chuẩn"**; tối ưu ctor-enum **chưa** được xếp lịch (VD-217/209); số tham khảo cũ không dùng làm mốc |
| ST-220 | **(Agent 2) VS0111 ngừng phát + ca `$value:`** (VD-216, VD-218) | ST-217 | 1 h | **DONE** | SPEC-lex §7 dòng VS0111 ghi "**ngừng phát từ VD-212** … báo `VS0203`"; **giữ số**; **xoá bộ lọc VS0111** trong `vis check`/`--json` (thay bằng tổng quát: mọi `Severity::Warning` khác VS0107 mềm, **VS0107 (tab) vẫn đỏ** theo VD-12); `vis explain` 52 mã; **ca khoá `vd_218.rs`**: lỗi arity qua builtin-as-value hai đường giống nhau byte-đối-byte, không `$value:`; `print(Some)` cùng `<builtin Some>` hai đường; **disassemble ẩn `$value:`** (`<builtin Some>`); thông điệp `$value:` nội bộ không in tên tiền tố; **lưu note VM docs** (`vis-vm/src/lib.rs` đoạn "$value:<tên>" — VBC1 đóng băng, không opcode mới); `grep` chứng minh không còn chỗ nào phát VS0111 (ngoài sổ mã `vis-diag`); **0 golden đổi**; `gate --fast` **PASS 10/10** |
| ST-210 | **(Agent 1) Đóng pha 1 — R30 + 4 dấu [SỬA] + bảng old→new→mã** (VD-185, VD-187) | ST-209 | 1 h | **DONE** (VD-225, A0-proxy uỷ quyền: pha 1 đóng trên **ps** tại HEAD `3bcfede`; cổng clone sạch ps PASS 10/10; 16/16 dấu `[SỬA]` VD-194 kiểm thực sự (vs3-qd-1..6/vs-ergo/CHEAT-SHEET/st169 — checklist trong `JOURNAL/st178-bao-cao.md`, mọi mục đều có tệp:dòng khớp); R30 giữ khung cũ + addendum ở `BAO-CAO-A0-2026-11-19-0900.md` (bảng old→new→mã VD-208/212/213/222/216/218/223/224); **mac: không đo được**, ghi waiver (VD-225 điểm 3); **A0 ratify VD-221..225** | **30/30** task vs-ergo DONE; gate **PASS 10/10** mac **và** ps trên **clone sạch**, **không chạy lại**; docs xanh; **bảng old→new→mã lỗi** (ghi theo **VD-208/210/212/215** (`{[k]: v}` ghi R30 dưới dạng **bẫy JavaScript thường gặp**)); gửi A0 **trích trước/sau 4 dấu `[SỬA]`** (QĐ-1 #1, QĐ-4 #2, C2 #3 và #7) |
| ST-211 | **(Agent 2) VS3-5-pre — `check.rs:191` + ca khoá + danh sách tách đường dẫn** (VD-184) | ST-210 | 1 h | **DONE** (VD-225, proxy A0-uỷ quyền) | `Path::new(path).file_name()` thay `rsplit('/')`; ca khoá: tương đối, tuyệt đối `/…`, `#[cfg(windows)]` `C:\…` + `C:/…`, `#[cfg(unix)]` tên tệp chứa `\`; self-test **12/12** trên psvn; liệt kê mọi chỗ tách đường dẫn thủ công vào **#149** (không sửa) |
| ST-212 | **(Agent 2) VS3-QĐ-1 — async song song thật** (VD-185) | ST-211 | 3 h | **READY** | đo song song bằng task **CPU** (không sleep), `W=min(nproc,N)` nhanh hơn `W=1` ≥ **0,6×min(nproc,N,4)**; `spawn` gửi được, đối số/giá trị trả **sao chép sâu**; **VS0508** = giá trị không gửi được; ca đỏ-trước; `vis-dev check` PASS |
| ST-228 | **(Agent 1, proxy A0) `fn f() -> int?` return plain lift `T → T?` (VD-228)** | ST-212 | 2-3 h | **DONE (proxy VD-225, chưa ratify)** | Option B áp @ `1eea7ed`: interp `opt_lift_fn_ret` ở `call_hfn_trong`; VM `codegen/than.rs` (`ret_opt` + `MakeSome`); `vd_228.rs` 7/7 (5/5 @ `1eea7ed`, +2 ca @ `8726eee`) byte-identical; closure/arg-lift KHÔNG; workspace 147/147; gate PASS; **A0 ratify/revert cần duyệt** |
| ST-229 | **(Agent 1, proxy A0)** `fn g(x: int?)` argument `T → T?` lift + closure-biên `fn(x) -> T?` lift (VD-229) | ST-228 | 2 h | **DONE (proxy VD-225, chưa ratify)** | byte-identical hai engine; lambda params không parse annot ⇒ không lift; `vd_228.rs` 13/13; workspace 147/147; gate PASS |
| ST-230 | **(Agent 2, nối VD-winport)** `vis test`: fixture `.vish` tìm được trợ giúp `test-child` **cạnh binary `vis`** (PATH của **tiến trình con**) | — | 0,5 h | **DONE** | đỏ-trước `./target/debug/vis test test/sem` **exit 1 / 4 `not ok`** (`VS0501 … test-child`, PATH sạch) ⇒ sau sửa **525 ok / 0 not ok** cả interp và `--vm`, exit 0; 1 ca unit cho PATH; `gate --fast` PASS; **không** đổi PATH của `vis`, không đổi ngôn ngữ |
> DONE 2026-11-20 · `crates/vis-cli/src/vistest.rs` (`path_voi_muc_dau` + `dat_path_tro_giup`, gọi ở **2** điểm spawn: fixture `.out/.err` và `test fn`) · lý do: harness Rust `tests/sem.rs`/`diag_cmp.rs` đã thêm `CARGO_BIN_EXE_test-child` vào PATH, còn `vis test` thì không ⇒ cùng bộ `test/sem` cho hai kết quả khác nhau · đỏ-trước `JOURNAL/st230-do-truoc.log` (sha256 `748b74b2246796c20fa738f9699c87265c83a976cdabeacf3909cede0adf0440`) · sau `JOURNAL/st230-sau.log` (525/525 ×2, exit 0) · unit `JOURNAL/st230-unit.log` (4/4) · gate `JOURNAL/st230-gate.log` (**PASS**, `[exit=0]`, 12/12 dòng `--- …: OK`, L0 **1348/0** — 150 suite, +1 ca unit; sha256 `3432a6b994cb61957a19ae38366441778510d59b3ca00b53cac35ea5db50e4be`).
| ST-231 | **(Agent 2, nối VD-winport)** đo lại `bin/vis-targets` sau SQLite MVP + ghi nợ **#156** | — | 0,5 h | **DONE** | `./bin/vis-targets` **16/18** — 2 ô `vis-cli` × `x86_64-pc-windows-msvc` / `wasm32-unknown-unknown` đỏ vì `libsqlite3-sys` (rusqlite `bundled`) cần C toolchain (`cc-rs`: thiếu `lib.exe` / `cc`); **không** sửa mã/script/kỳ vọng; nợ **#156** thêm vào `SO-NO.md` (§8 **157 = 94/38/23/1/1**, luật R5 xanh); gate **PASS** (`JOURNAL/st231-gate.log`, `[exit=0]`, L0 **1348/0**, sha256 `63589328591634d83d79c0a4eceefc5001af9c8e3fedf8ed2afeddfe8bfddb4a`) |
> DONE 2026-11-20 · chỉ **đo + ghi sổ**, không đổi sản phẩm · log `JOURNAL/st231-targets.log` (sha256 `79eeefbe3084ee3df7ef44407771c1747637f8b6216ebb4ff3117c8650d359b4`) · nguồn: `bin/vis-targets` (18 ô; `EXPECTED_RED` **chưa** được hiện thực trong script) · cần A0 quyết hướng xử (feature optional / expected-red có văn bản / cross toolchain).
| ST-232 | **(Agent 2)** fresh-clone gate @ `9c3f804` (bằng chứng tái lập) | — | 0,25 h | **DONE** | clone sạch từ `origin/main` ⇒ `./vis-dev gate --fast` **PASS** `[exit=0]`, **12/12** bước, **150 suite / 1348 passed / 0 failed**; không sửa gì trong clone |
> DONE 2026-11-20 · clone `/home/ps/.tmp-clone-gate/st231` từ `https://github.com/vifast-oss/ViScript.git` tại `9c3f804` · log `JOURNAL/st232-fresh-clone.log` (sha256 `424fcf2c7cc885b0fbd6815d4ff5bf4ee4dd91d5244916d456d21eff203a113a`).
| ST-233 | **(Agent 2, hồi quy tách repo ST-171)** `bin/vis-soak` gọi `./vis-dev` ở **gốc repo** (không `../vis-dev`) | — | 0,5 h | **DONE** | đỏ-trước: chạy thật `./bin/vis-soak` ⇒ `gate-1` **exit=127** (`../vis-dev: not found`) và ca mới `vis_soak_goi_vis_dev_o_goc_repo` **0/1**; sau sửa ca **2/2**; **soak 10/10** (3 gate + 3 diff + 2 host + 2 cache) với gate PASS trong từng vòng |
> DONE 2026-11-20 · sửa **3 dòng** `bin/vis-soak` (`../vis-dev` → `./vis-dev`), ca khoá dựng cây giả có `vis-dev` ở gốc (marker đòi **3** lần gọi) · đỏ-trước: chạy thật `./bin/vis-soak --log …` ⇒ `gate-1` **exit=127** (`../vis-dev: not found`) — `JOURNAL/st233-soak-do-truoc.log` (sha256 `b586a53c3bbff5dcd639de980c6edb53daea539b1b71a158b66f995924d71bab`); ca `vis_soak_goi_vis_dev_o_goc_repo` **0/1** trước sửa ⇒ **2/2** sau sửa · lần 2 đỏ `fmt` (`st233-soak-sau.log` sha256 `5b94fb225de3bfd477c45f5e685cec5e655fb9fc5e50ab21338c77838a309aa9`; đã `cargo fmt`), lần 3 đỏ **R3** vì tệp lạ `net.vis` do agent audit tạo ở gốc repo (`st233-soak-sau2.log` sha256 `71cbe5fa03913009c0ed1b67008a82015646aff8681750a6e4a9fc1c00c9f5f8`; đã chuyển ra `/tmp/opencode/audit-vm/`) — **không** phải flake sản phẩm · lần 4 `st233-soak-sau3.log` (sha256 `cde9a19488cccd72ab2fc7ca85e7209423830913be37ab67995f60c4f6a3a021`) **10/10 vòng OK = 267 s**, mỗi gate **150 suite / 1349 passed / 0 failed** (gate-1: 41 s, gate-2: 41 s, gate-3: 41 s; diff 41/42/42 s `lech=0`; host 54/54 ×2; cache 5/5 ×2). · gate cuối `JOURNAL/st233-234-gate.log` **PASS** (`[exit=0]`, **150 suite / 1349 passed / 0 failed**, sha256 `cee657e9e33f8fd5ab7382a63fc7ce2dbcf476480e14fd287c4e1fe6569eb55d`).
| ST-234 | **(Agent 2, audit parity)** 10 lệch interp ↔ VM **xác nhận độc lập** + ghi nợ `#157..#166` | — | 1 h | **DONE** | mỗi ca có repro `.vis` (SPDX) + đầu ra **hai engine**; **không sửa mã**; `SO-NO` **167 = 104 mở/38 đóng/23 VS3/1/1**, luật R5 xanh; báo cáo `JOURNAL/st234-lech-vm-interp.md`; gate PASS |
> DONE 2026-11-20 · 10 ca: `for` lồng+break **treo/quá lâu** ở VM, `while let` VM từ chối module, `let`/`const` cấp tệp VM thấy `()`, `?.` lệch hai chiều, `match` rơi nhánh VM im lặng, closure ghi biến bắt VM reset, or-pattern bind vế đầu, `set` không dedup · **không sửa** (ca `?.` base không-optional cần A0 chốt) · repro `JOURNAL/st234-repro/*.vis` (10 tệp) · ưu tiên: treo > từ chối module > const/biến cấp tệp > im lặng > còn lại. · gate cuối `JOURNAL/st233-234-gate.log` **PASS** (`[exit=0]`, **150 suite / 1349 passed / 0 failed**, sha256 `cee657e9e33f8fd5ab7382a63fc7ce2dbcf476480e14fd287c4e1fe6569eb55d`).
| ST-235 | **(Agent 2, audit std + CLI)** 18 phát hiện, ghi nợ `#167..#184` | — | 1 h | **DONE** | 5 ca **chạy lại trong phiên** (`list.sort` NaN, `json.encode(1e310pct)`=`inf`, trap module sai tệp `rt/a.vis:4:1` thay vì `rt/b.vis:2:5`, CRLF lệch 1 dòng, `fmt` bỏ qua tệp `test/fmt/bad.vis`) + 13 ca có dẫn chứng `tệp:dòng`; **không sửa mã**; `SO-NO` **185 = 122 mở/38 đóng/23 VS3/1/1**, R5 xanh; báo cáo `JOURNAL/st235-audit-std-cli.md` |
> DONE 2026-11-20 · nguồn audit: std/builtins (10) + CLI/LSP (8); ca nặng: `net.read` **abort** exit 134, `list.sort` im lặng, `json.encode` phát `inf`, trap module sai tệp, CRLF lệch dòng, `fmt` bỏ qua tệp, `check --json` warning-đỏ; **cần A0 chốt**: #170 (khoá trùng map), #173 (float trong JSON), `sqlite.query` INSERT; report sha256 `acc487e1b7b5ad2e67b43dafa57aa9ec816248467df63efb72ae20d0c8a8e6d0`. · gate `JOURNAL/st235-gate.log` **PASS** (`[exit=0]`, 150 suite / **1349 passed / 0 failed**, sha256 `c8260b11c37705efd15d86974d550825ce1d7d09a91ca5d665a1aec9bb6848fc`).
| ST-226 | **(Agent 1) `while let` (R18b, VD-222 điểm 7)** | ST-210 | 2 h | **DONE** (VD-225/VD-222 điểm 7 — merge `ba9173e`) | semantics y hệt `if let`; **`while let` trên cả hai đường cho cùng byte**; ca khoá `vd_226.rs` (8 ca byte-identical); `vis-dev gate --fast` PASS 10/10 | `while let PAT = EXPR:` nhận **pattern**; semantics như `if let` (đường tới `None` ⇒ thoát vòng); bindings trong thân vòng; Codegen + interp qua `HCond::Let` (đã có); ca đỏ-trước; `./vis-dev gate --fast` PASS |
| ST-213 | **(Agent 1) VD-186 — diff tài liệu `HOST-BAO-DAM.md` §4 + `ViScript-thiet-ke.md`** (VD-186) | ST-200 | 2 h | **DONE** | mỗi tệp **một diff**; tài liệu mô tả **mã hiện tại**; câu tương lai gắn nhãn `[thiết kế VS3, chưa triển khai]`; §4 thu hẹp "macOS còn khe hở" vào **đường GHI** + hardlink + Windows mức thấp hơn; bỏ registry tập trung, thay mô hình VD-180/181; **Agent 0 duyệt lượt sau**. **Kết quả**: `agent/DU-THAO-VD186/` — `HOST-BAO-DAM-4.patch` (**99 dòng**, +48/−5), `ViScript-thiet-ke.patch` (**114 dòng**, +89/−2), `TOM-TAT.md` (140 dòng, 7 điểm xin A0 quyết); `git apply --check` **sạch**; **không** đụng tệp đích, **không** đổi số dòng khối ` ```vis ` (24 khối + 2 `vish-session` giữ nguyên ⇒ không phải `--regen`) |
> DONE 2026-11-18 · commit `7c7a710` · PNG sinh từ SVG bằng headless Chrome 1024 px rồi thu nhỏ Lanczos (nền trong suốt, không viền trắng); `resources/brand/README.md` ghi cách sinh lại + bảng màu; đã chép sang `ViFastSite/brand/` (192) và `ViFastOrg/profile/assets/` (512), deploy `vifast.org` + push trang tổ chức GitHub
| ST-201 | **(Agent 2) Đồng bộ tài liệu sau đợt pha 1** (R29 → tách repo → ST-200) | ST-200 | 0,5 h | **DONE** | `ROADMAP.md` thêm khối “cuối R29” (tách repo ST-171 + pha 1 ST-179..ST-200 + số đo + việc còn thiếu + sổ nợ) — giữ số cũ (VD-142); `STATE.md` header thêm dòng 18/11 cuối ngày; `SO-NO.md` đồng bộ đợt 29 (#146 đóng ST-174, mở #147, 89 mở/**35 đóng**) — commit `2f62022`; `ST-197` hàng QUEUE/JOURNAL bổ sung hồi tố — commit `ed490d2`; **0 dòng mã**, không đổi SPEC/ngưỡng/ca |
> DONE 2026-11-18 · commit `ed490d2` (ST-197), `2f62022` (SO-NO), khối ROADMAP/STATE ở commit này; **không** báo cáo R30 — chờ chốt đợt pha 1 |
| ST-202 | **(Agent 2) R19b — tràn i64 = panic (thoát 101), mã `VS0507` ở mọi đường** (C3/VD-178) | ST-200 | 0,5 h | **DONE** | Sửa **một** chỗ lệch còn sót sau R9: `un Neg` (`-i64::MIN`) báo **VS0501** thay vì **VS0507** (`crates/vis-interp/src/builtin.rs` — VM dùng chung đường này); interp **và** VM đều `VS0507` + exit **101**; ca đỏ-trước `test/sem/loi/doi-dau-tran.vis` (trước sửa: `not ok 227 … VS0501 tràn i64 khi đổi dấu`) xanh sau sửa; harness khoá số ca 464→**465** (`vis_test.rs`, `sem.rs`, `diag_cmp.rs` XANH vẫn 380/LOI_LUC_CHAY 34→**35**, `vs_interp.rs` 415/khop 39, `roundtrip.rs`); `cargo test --workspace` **1215/0** (136 suite); `gate --fast` **PASS**; `vis test` 465/465 ×2 đường; log tổng hợp `JOURNAL/st202-r19b.md` |
> DONE 2026-11-18 · ca âm cũ `tran-nhan`/`tran-cong`/`tran-i64` không đổi; `i64::MIN // -1` đã VS0507 từ ST-197; `3^...pow`/`%`/add/sub/mul đều đã qua `overflow()`; literal `9223372036854775808` vẫn lỗi biên dịch `VS0104` (khác tràn runtime) |
| ST-203 | **(Agent 2) R2 — bỏ từ khoá `none` + `T??` lồng tuỳ ý + tự nâng nhiều lớp** (C3/VD-178) | ST-202 | 2 h | **DONE** | `none` ⇒ **VS0203** mọi vị trí (biểu thức `lower/expr.rs`, pattern `lower/pat.rs`); `vis fmt` tự sửa `none` → `None` (+2 ca fmt); patterns UpperCamel — so thường nội bộ, thông điệp in hoa; **`int??` = `Opt[Opt[int]]`** (parse hậu tố `?`/`??`; `resolve_type` đếm; `resolve_ann` bỏ cấm `T??`; `coerce` tự nâng nhiều lớp — bóc `Opt` hai vế, `f ≤ t`); value in `Some(...)`/`None`; migrate 25 `.out`/fixture + host suite + crates tests; **4 ca đỏ-trước** (`optional/int-qhap-gan` · `optional/map-get-lot` · `optional/t-qhap-generic` · `loi/none-bo` VS0203); harness **465→469** (`sem.rs`, `vis_test.rs`, `diag_cmp.rs` 383/51/35, `vs_interp.rs` 418/51/39, `roundtrip.rs`, `tin_tyck.rs`, `fuzz_mutate.rs` 383); thiết kế doc §2/§7.3/grammar + doc-examples regen (24 fixture); workspace **1217/0**; `vis test` **469/469** ×2; `gate --fast` **PASS** |
> DONE 2026-11-18 · **Chưa làm** (ngoài phạm vi phiên này, cần A0): (a) `Option`/`Result` vẫn là **kiểu dựng sẵn**, chưa enum-hoá thật trong prelude (spec R2 “enum std”); (b) phát hiện lệch interp/VM pre-existing: `print(x)` khi `x` là giá trị coerced **chưa tag** — interp in `5`, VM in `Some(5)` (VM chèn wrap, interp elide); không ca sem nào chạm (mọi ca dùng `??`/match/tagged); ghi nợ chờ A0; (c) từ khoá `none` **vẫn lex** (như `alias`/`newtype`) để `vis fmt` sửa được — xoá khỏi lexer ở VS3 nếu A0 muốn; (d) ca `first[T]` với `T = int?` được `t-qhap-generic` phủ (không phủ `map.get` với `T = int?` — builtin chưa nhận chữ ký generic lồng) |

| ST-204 | **(Agent 2) Báo cáo `R30`** (chốt đợt tách repo + pha 1 công thái học) | ST-171, ST-178, ST-179, ST-180, ST-181, ST-182, ST-183, ST-184, ST-185, ST-186, ST-187, ST-188, ST-189, ST-190, ST-191, ST-192, ST-193, ST-194, ST-195, ST-196, ST-197, ST-198, ST-199, ST-200, ST-201, ST-202, ST-203 | 1 h | **DONE** | `reports/R30-viscript.{md,json,json.license}`; `head` = `0d5e0e5`; `check_report --all` PASS (30 báo cáo, 223 hàng); liệt kê task, số đo, blocked[] (0), nợ; lật ở chính commit chứa báo cáo |
> DONE 2026-11-18 · gate `--fast` **PASS 10/10** tại `0d5e0e5` (máy `ps`, Linux); workspace **1217/0**; `vis test` **469/469** ×2; macOS/Windows không đo lại (mac chuẩn = R29/ST-171); log `JOURNAL/st204-r30.md`; số macOS trong đợt: không (chỉ Linux) |

GHI CHÚ 2026-11-17 (VD-178) · **ST-179 trở đi** (Agent 2): pha 1 công thái học theo **C3** — P0 → R9(+C1) → R6 → R17 → R1+R2 → R3 → R8 → R4 → R5 → R11 → R12 → R16 → R7 → R19b → R19c; mỗi task ≤3 h, đỏ-trước, gate PASS; **xen kẽ được VS3-5a/5d**; sau pha 1: VS3-QĐ-1 → (QĐ-2,3,4) → QĐ-6, QĐ-5 phần còn lại sau R6
GHI CHÚ 2026-11-17 (VD-180) · **thay QĐ-6 mục 1, 2 của VD-178**: mô hình gói **phi tập trung kiểu Go** (tên = đường dẫn, tag semver, **MVS**, `vifast.org/x/…`, **sum.vifast.org** + `vis.lock` sha256, `VS0707`, `VIS_SUMDB=off`/`VIS_PRIVATE`, `VIS_PROXY=direct`); proxy **chỉ thiết kế**, chưa vận hành; **ST-178 sửa `vs3-qd-6.md`** + 4 ca đỏ-trước bắt buộc (tag đổi nội dung, MVS v1.2/v1.4, vanity → GitHub, offline + cache)
GHI CHÚ 2026-11-17 (VD-181) · tên gói chốt **`vifast.org/vis/<tên>`** (path, không `vis.`); **`sum.`/`proxy.vifast.org`** ngang hàng, phục vụ mọi sản phẩm; `vis.vifast.org` chỉ website; `viscript.org` chỉ thương hiệu; **Traefik không phải hợp đồng (meta tag là)**; tên **bất biến** sau gói đầu tiên
| ST-172 | **(Agent 1) Tài liệu công thái học** `JOURNAL/vs-ergo.md` (7 mục a–g + VS0412 + cheat sheet; **chỉ đọc, 0 dòng mã**) | ST-169 | 2 h | **DONE** | mỗi mục: hiện trạng `file:dòng`, đề xuất, ca đỏ-trước, ảnh hưởng mã/test; VS0412: kết luận có/không 2 lỗi + đề xuất tách mã; cheat sheet đổi nhãn "chưa có tại <commit>"; 0 dòng mã sửa |

> CHỜ DUYỆT Agent 0 · `JOURNAL/vs-ergo.md` (7 mục a–g + §8 VS0412: **có** 4 nhóm lỗi chung mã, đề xuất VS0413/0414/0415/0416; §9 cheat sheet đã đổi nhãn); **0 dòng mã**
GHI CHÚ 2026-11-17 (VD-168) · phạm vi mở rộng 12 mục a–l (thay mục 2,4 của VD-167) — đã bổ sung trong `vs-ergo.md`; ưu tiên P0=(c) AnyError; duyệt cùng 6 tài liệu VS3; mục được duyệt thực thi TRƯỚC VS3
GHI CHÚ 2026-11-17 (VD-169) · hàng này **tách** thành **ST-172a** (dữ kiện) + **ST-172b** (đề xuất); nội dung `vs-ergo.md` hiện có là **bản nháp**, chưa được duyệt
| ST-172a | **(Agent 1) Dữ kiện công thái học** `JOURNAL/vs-ergo-facts.md` — **chỉ dữ kiện, không đề xuất**; 8 nhóm, trạng thái [ĐÃ CÓ]/[THIẾT KẾ CÓ, CHƯA LÀM]/[CHƯA CÓ] | ST-169 | 2,5 h | **DONE** | mục lục + `## 1..8`; mỗi mục có `file:dòng` + commit; `all_codes` đầy đủ + đánh dấu mã đa nghĩa; 3 script 50–200 dòng + bản Python; **0 dòng mã** sửa |
> CHỜ DUYỆT Agent 0 · `JOURNAL/vs-ergo-facts.md` **420 dòng**, 8 nhóm, mọi mục có nhãn trạng thái + `file:dòng`; 3 script + Python chạy thật; bảng 33 mã lỗi + 10 văn bản lỗi thật; **0 dòng mã**
APPROVED 2026-11-17 Agent 0 · ST-172a đạt: 420 dòng, 8 nhóm, nhãn trạng thái + file:dòng, 33 mã + 10 văn bản lỗi thật, 3 script chạy thật; dùng làm căn cứ VD-170
| ST-172b | **(Agent 1) `JOURNAL/vs-ergo.md`** theo VD-168 (12 mục a–l, mức ưu tiên) | ST-172a | 2 h | **DONE** | **Chờ Agent 0 gửi phân tích dựa trên `vs-ergo-facts.md`** (VD-169) rồi mới bắt đầu; 0 dòng mã |

GHI CHÚ 2026-11-17 (VD-170) · Agent 0 đã phân tích `vs-ergo-facts.md`: phạm vi mới 13 mục (P0: newtype báo lỗi, bỏ “VS0”, cheat sheet doc-test; P1: p-string, gợi ý, tách mã; P2: viết hoa, AnyError, method, str+str, print/format/record/if-let; P3: const/duration/--help, Q2); **rút** xs[i]→T? và tự gói T→T?; probe lại ở HEAD mới nhất
> CHỜ DUYỆT Agent 0 · `JOURNAL/vs-ergo.md` **13 mục P0–P3** theo VD-170: hiện trạng `file:dòng` + probe HEAD `efa78295`, đặc tả, ca đỏ-trước, ảnh hưởng, task ≤3 h, phụ thuộc; §14 3 script theo thành ngữ + so dòng ViScript/Python (12/6, 12/8, 25/8) kèm lý do; **0 dòng mã**
GHI CHÚ 2026-11-17 (VD-171) · phạm vi đổi thành **R1–R10** (thay mục 7,8,10,11,13 của VD-170; sửa VD-19/Q2/§1.6): viết hoa enum, Option/Result std + `T??`, `?`/`??`/`if let`/`let else`, `Error` record + `Json` enum, gọi hàm tên + record theo tên + method toàn std, chuỗi (`p""` thô, `str+str`, format, print nhiều đối số), số (`/`→float, `//`, `%`, const), chỉ số (xác minh), mã lỗi một nghĩa + gợi ý + bỏ “VS0”, đơn vị chuẩn hoá; **thứ tự**: P0 → R9 → R6 → R1+R2 → R3+R4 → R5 → R7 → R10
GHI CHÚ 2026-11-17 (VD-172) · bổ sung **R11–R21** (hàm dựng, type gộp alias/newtype, biến thể enum không tiền tố, impl self/var self, nối dòng, let che tên, điều kiện bool, pattern + while let, 0o755 + panic tràn + bit method, assert/use-as/use-{}/list+range, pipeline chỉ shell/cmd); **thứ tự**: … R5 → R11…R21; cheat sheet theo R1–R21 sau khi thực thi
GHI CHÚ 2026-11-17 (VD-173) · chia **2 pha**: Pha 1 (trước VS3, phá mã cũ) = P0 VD-170 + R9 + R17(gợi ý) + R6 + R1+R2 + R3+R4 + R5+R11 + R12 + R16 + R7 + R19c; Pha 2 = R14 (đầu pha) + R18 + R19(0o755,bit) + R20 + R10 + R15(./??/and/or) + R21; thêm **bảng tổng** đầu tệp; **không** sửa mã R trước khi A0 duyệt
GHI CHÚ 2026-11-17 (VD-174) · nguyên tắc mô hình: không class/kế thừa, không lifetime/con trỏ/unsafe/macro; không chia sẻ sửa được (map+ID / `Chan`); **mục 4–6** = trait-as-type (`list[Shape]`, bảng ảo), sửa lồng sâu (`u.addr.city = x`), iterator lười + `.to_list()` — đứng **cùng R14 đầu pha 2**, đặc tả trước R20
APPROVED 2026-11-17 Agent 0 · `vs-ergo.md` R1–R24 **duyệt CÓ SỬA** (VD-178); [SỬA] ở ST-178; pha 1 theo thứ tự C3; R24 đảo thành `.lazy()` (giữ filter/map trả collection)
| ST-31 | báo cáo **R4** | ST-25..ST-30 | 1,5 h | **DONE** | MD+JSON, 36 hàng, số liệu tái lập |
| ST-24 | báo cáo **R3** | ST-18..ST-23 | 1 h | **DONE** | MD+JSON, 27 hàng, số liệu tái lập |
| ST-17 | báo cáo **R2** (`reports/R2-viscript.*`) | ST-11..ST-16 | 1 h | **DONE** | MD+JSON, 20 hàng QUEUE, số liệu tái lập được |
| ST-04 | lexer theo SPEC-lex (ST-02a đã DONE) | ST-02a | 3 h | **DONE** | **60 golden token đỏ trước**; fuzz 10 phút + NFC/NFD |
| ST-05 | parser + CST lossless | ST-04 | 3 h | **DONE** | **30 golden biểu thức đỏ trước**; 100 % fixture tài liệu parse; mỗi tệp lỗi đúng 1 lỗi đầu |
| ST-06 | CmdCST + golden từng hàng bảng 9.4 + 10 ca mơ hồ | ST-05 | 3 h | **DONE** | golden AST/CST |
| ST-07 | `vis-fmt` lossless, idempotent, sửa tab | ST-05 | 3 h | **DONE** | vòng tròn parse→fmt→parse |
| ST-08 | runner `--check` cú pháp (chưa interpreter) | ST-05 | 2 h | **DONE** | TAP; chạy fixture |
| ST-09 | fixture theo QD §7.4 (1/khối mã + 1/quyết định; cấm `...`) | ST-06,07,08 | 2 h | **DONE** | xanh |
| ST-10 | báo cáo `R1-viscript` | các task | 1 h | **DONE** | MD + JSON có `project` |

APPROVED 2026-11-12 Agent 0

> ST-02a DONE; VD-27 chốt mã lỗi 01xx..05xx. ST-02b/ST-10 DONE; ST-03 READY. Commit `3d5539a` do người điều phối, không phải Agent 2.

APPROVED 2026-11-12 Agent 0

> **ST-04 DONE** — commit `663eacb` (đã push `origin/main`): lexer `crates/vis-syntax` theo `SPEC-lex`; **60/60
> golden token đỏ trước** (`JOURNAL/st04-red-golden.log`), fuzz **10 phút** (18.661.376 ca / 336.316.250 token,
> 0 vi phạm, 25/25 kind, đủ 10 mã — `JOURNAL/st04-fuzz-10m.log`), NFC/NFD, `./vis-dev gate --fast` PASS, 40 test.
> **ST-05 mở ngay** (`READY`). Quyết định kèm theo: **VD-28..VD-34** (Q1..Q5 của SPEC-lex, phụ thuộc Unicode, và
> bộ **30 golden biểu thức**).

APPROVED 2026-11-12 Agent 0

> **ST-05 DONE** — commit `49f2539` (đã push): parser + CST lossless `crates/vis-syntax`; **30/30 golden biểu
> thức** đỏ trước (`JOURNAL/st05-red-expr.log`, kỳ vọng Agent 0 ở `viscript/test/expr/golden.expr`, VD-34);
> **15/15 khối tài liệu đủ thân** parse 0 lỗi + lossless, 7 khối loại trừ tường minh (VD-36); **1 lỗi
> đầu/tệp** (13 nguồn khoá); sổ mã cú pháp `VS0201/VS0202` **tách** khỏi `CODES` (VD-35); 5 chỗ Phụ lục A
> không tự chứa được khoá bằng test (VD-37). `gate --fast` PASS, **55** test vis-syntax. Review độc lập **3
> vòng** (2 lần DRIFT: thiếu `self`/NEWLINE, lý do loại khối 1412 sai, `require_line_end` báo oan tệp không
> có newline cuối) — đã sửa hết trước khi commit.
> **ST-06, ST-07, ST-08 mở** (đều phụ thuộc ST-05). Thứ tự đề nghị: **ST-08 (2 h) → ST-07 (3 h) → ST-06 (3 h)**
> — ST-08 nhỏ và cho người dùng thấy kết quả sớm; ST-06 để sau vì nặng nhất. Lỗ hổng đã biết (hoãn, ghi
> `JOURNAL` + vào R2): pattern `dotted_path`, lvalue postfix cho `+=`, `weak`/`cap` chưa là từ khoá,
> `Entry(trường: …)`/`assert`/`...` trong tài liệu, variadic chưa có token.

APPROVED 2026-11-12 Agent 0

> **ST-08 DONE** — commit `2971a72` (đã push): `vis check <tệp…>` in **TAP** (`TAP version 13`, `1..N`,
> `ok`/`not ok`), mỗi tệp `not ok` đúng **1** dòng `# VSxxxx dòng:cột`; mã thoát 0/1/2; `.vish` ⇒ `Bail out!`
> + exit 2 (ST-06); tệp không đọc được ⇒ `not ok` (ngoại lệ có chủ ý, không cấp mã mới); **lọc tầng**: có
> `01xx` thì in `01xx` trước (VD-38 mục 8) để lỗi từ vựng không bị `VS0201` che. **12 ca đỏ trước**
> (`JOURNAL/st08-red-check.log`, sha256 harness khớp bản commit — đã chụp lại một lần vì lệch hash), **78**
> test, `gate --fast` PASS. 15 khối tài liệu đủ thân ⇒ 15 `ok`; 7 khối loại trừ ⇒ 7 `not ok`.
> **ST-07 mở** (kế tiếp, 3 h); **ST-06** để sau (nặng nhất). Quyết định kèm theo: **VD-39** (phạm vi `vis fmt`
> vòng 1) + VD-38 (sổ lỗ hổng đã biết).

APPROVED 2026-11-12 Agent 0

> **ST-07 DONE** — commit `b149fe9` (đã push): crate mới `crates/vis-fmt` + `vis fmt <tệp>|--write|--check`;
> phạm vi **VD-39** (chỉ thụt lề/tab/khoảng trắng cuối/một newline cuối; giữ nguyên văn trong dòng; ra NFC).
> Bốn tiêu chí §18.4: 15/15 khối tài liệu + `chuan.vis` **bất động byte-for-byte**; idempotent 48 nguồn; tab ⇒
> 0 tab + thụt bội 4; tệp lỗi ⇒ exit 2, `--write` không ghi. **105** test, `gate --fast` PASS. Review độc lập 2
> vòng: DRIFT (dòng nối sau chú thích mất thụt lề gốc, tab trong gap cùng dòng + trong thân chú thích bị đổi)
> ⇒ đã sửa, 3 ca tái hiện thành điểm bất động.
> **Đính chính số**: tiêu đề commit ghi "(17 ca đỏ trước)" là **sai do Agent 0 soạn thông điệp**; log đỏ thật
> có **27 ca** (5 unit + 13 integration + 9 CLI). Không amend (đã push, tránh force-push) — sổ này và báo cáo
> R2 ghi đúng **27**.
> **ST-06 mở** (kế tiếp, 3 h, chia 2 pha): pha A = bộ **22 golden chế độ lệnh** (12 hàng §9.4 + 10 ca mơ hồ) do
> Agent 2 soạn nháp + Agent 0 **duyệt trước** khi hiện thực; pha B = `parse(Mode::Command)` → CmdCST + nối
> `.vish` vào `vis check`.

APPROVED 2026-11-12 Agent 0

> **ST-06 DONE** — commit `590e9f7` (đã push): `parse(src, Mode::Command)` → **CmdCST** chưa định kiểu
> (`cmd_lower` vẫn vòng 2, VD-22); **33 golden** chế độ lệnh (12 hàng §9.4 + 10 ca mơ hồ + `.vish` nhiều dòng
> + 10 ca từ review, 7 ca `!ERR`) đỏ trước → xanh; `vis check` nay kiểm **`.vish`** (bỏ `Bail out!`), 2 khối
> ```vish-session của tài liệu ⇒ 2 `ok`; `vis fmt` vẫn từ chối `.vish`. **142** test, `gate --fast` PASS, lexer
> không đụng. Review độc lập 2 vòng: DRIFT (`let` có kiểu hỏng, closure cắt nhầm tại `|>`, `&` trước `;` được
> nhận, `foo[ab]` tách thành 2 từ) ⇒ đã sửa hết + MINOR; vòng 2 OK. Quyết định: **VD-40/VD-41** (xem
> `DECISIONS.md`).
> **ST-09 mở** — bước cuối vòng 1: cổng **doc-examples parse** (QD §7.4 + VT09): bước mới trong `vis-dev gate`,
> allowlist `docs/viscript/doc-examples-allow.txt` **7 mục được kiểm chứng** (mỗi mục phải thật đỏ; khối đỏ
> ngoài danh sách ⇒ cổng đỏ), 17 fixture tự sinh + ≥10 fixture cho quyết định §7.1–7.3, ≥1 case âm.

APPROVED 2026-11-12 Agent 0

> **ST-09 DONE** — commit `ba777e9` (đã push): bước `doc-examples` trong `vis-dev check`/`gate --fast`
> (**sau** `--- build: OK`, kiểm độ mới của binary — không xanh trên binary cũ); `docs/viscript/doc-examples-allow.txt`
> **đúng 7 mục** (6 khối `...` + khối 1412), mỗi mục phải **thật đỏ**, khối đỏ không khai ⇒ cổng đỏ; **24** fixture
> `doc-*` tự sinh (22 `vis` + 2 `vish-session`, so byte với tài liệu) + **12** `qd-*` cho QD §7.1..§7.3 (so **đúng
> mã**); **12 case âm** (khối lạ, ngoại lệ cũ đã xanh, sai số 7, lệch byte, qd=10/13, binary cũ…). `--- doc-examples:
> OK (24 khối: 17 xanh + 7 ngoại lệ; 12 fixture QD)`. Review 2 vòng: DRIFT (nới ngưỡng `>=10`; cổng chạy trước
> `cargo build`) ⇒ đã sửa; vòng 2 Agent 0 xác nhận trực tiếp.
> **ST-03 là task cuối còn lại của vòng 1** (vis-diag: cột theo scalar, render, vi/en/ja, snapshot 20 ca; cap 2 h).

APPROVED 2026-11-12 Agent 0

> **ST-03 DONE — VÒNG 1 XONG (ST-01..ST-10, 11/11 dòng `DONE`)** — commit `ada1ebc` (đã push): `vis-diag` có
> cột theo **Unicode scalar** (khoá 5 ca: Việt NFC/NFD, `🚀`, `👨‍👩‍👧` = 5 scalar, tab = 4 — lexer ST-04 vốn đã
> đúng, chỉ khoá lại), `render(&DiagLine, Lang)` in `<tệp>:<dòng>:<cột>: <mức> <mã>: <thông điệp>` + dòng nguồn
> + con trỏ canh theo scalar, bảng **vi/en/ja** đủ 12 mã (`message_vi` không đổi, SPEC-lex §7 vẫn là chuẩn),
> `vis explain --lang vi|en|ja` + `--list --lang`, **20 snapshot `render`** (NFC/NFD/emoji/ZWJ/tab giữa dòng/dòng
> dài/ngoài biên/CJK/3 ngôn ngữ) + 20 ca message mẫu. **157** test, `gate --fast` PASS. Review 2 vòng: DRIFT
> ("snapshot 20 ca" mới là message, thiếu emoji) ⇒ đã thêm bộ 20 snapshot render; vòng 2 Agent 0 xác nhận.

APPROVED 2026-11-12 Agent 0

> **ST-11 DONE** — commit `94a5342` (đã push): crate mới `crates/vis-types` (`Type` theo VD-43 + `Float`/`TypeRef`/
> `Error`), HIR chế độ script giữ `Span`, `lower()` (hạ CST→HIR + giải tên), `Symbols`/`TypeTable`. Hạ **permissive**
> (VD-44) — biểu diễn mọi thứ Phụ lục A ở mức cú pháp; `Mode::Command` ⇒ lỗi API, không panic. Giải tên (VD-45):
> mặc định **Permissive** (`VS0301` chỉ khi gán tên chưa khai, ở gốc lvalue; `VS0302` khai lại cùng khung),
> `Strict` có test riêng. Sổ thứ ba `NAME_CODES` (`VS0301/VS0302` vi/en/ja), `next_block()` ⇒ **601** suy ra.
> **194** test; 15/15 khối tài liệu hạ 0 chẩn đoán; 27 golden ⇒ sexp HIR; 3 ca `!ERR` dừng ở parse; `gate --fast` PASS.
> Review: DRIFT (`lower_let` khai tên trước khi hạ `init`; `Strict` báo `VS0301` hai lần) ⇒ đã sửa + khoá test.
> **ST-12 mở**.

APPROVED 2026-11-12 Agent 0

> **ST-12 DONE** — commit `273a921` (đã push): kiểm kiểu VS0 (`tyck/{mod,expr,pat,num}.rs`) + **`match` xét hết**,
> sổ thứ tư `TYPE_CODES` (`VS0401/0402/0403/0412`, vi/en/ja), `all_codes()` = 18, `explain` phủ 4 sổ. **116 ca đỏ
> trước** (88 tyck + 28 `match`) có **chống xanh-giả**; **205** test; `gate --fast` PASS. Quyết định **VD-46**:
> gán cho `let`/tham số ⇒ `VS0412` + thông điệp "chỉ `var` mới gán"; lambda param `{error}` (hoãn suy kiểu hai
> chiều, parser đúng §A.3); pattern bản ghi trần thiếu `sym` ⇒ hoãn ST-14; `line/col = 0` ở tầng tyck chấp nhận —
> **ST-14 bắt buộc in lại thật từ `span` + văn bản**.
> **ST-13 mở** (`vis-interp` + builtins lõi, `VS05xx` runtime).

APPROVED 2026-11-12 Agent 0

> **ST-13 DONE** — commit `5e9a962` (đã push): crate mới `crates/vis-interp` (`Value` 16 biến thể + `Closure`;
> `interp.rs` môi trường/câu lệnh/biểu thức/pattern, gọi hàm tham số tên + mặc định + đệ quy, `?`/`return` trả sớm,
> `match` đầy đủ, `with`, chỉ số, `|>`, `??`; `builtin.rs` toán tử §3.6 + 8 tên dựng sẵn + bộ phương thức lõi),
> sổ thứ năm `RUNTIME_CODES` (`VS0501/0502/0503`), `all_codes()` = 21, `next_block()` vẫn 601. **79 ca đỏ trước**,
> **227** test, `gate --fast` PASS, chẩn đoán runtime in `dòng:cột` **thật** (VD-46). Quyết định **VD-47** (giá trị
> câu lệnh cuối; `Show` size/duration/percent; `percent * size|int` cắt về 0; bảng phương thức khoá test). Hoãn có
> ghi: nội suy `{expr}` ⇒ ST-14; `ref/await/cmd`/bit ⇒ `VS0501`; gán trường/phần tử ⇒ `VS0501` + gợi ý `with`.
> **ST-12b mở** (sửa 7 lỗi kiểm kiểu do review độc lập bắt; xong mới coi ST-12 DONE thật).

APPROVED 2026-11-12 Agent 0

> **ST-12b DONE** — commit `dec75e3` (đã push): sửa **đúng 7 lỗi** kiểm kiểu do review độc lập bắt (`?` theo §4.4,
> `int→float` chỉ literal, builtin được kiểm, gán `Field`/`Index`, `--` chỉ cho tham số khai `--`, `percent` vs
> `float` theo tỉ lệ, `match` `unit`/tuple hữu hạn) + **VD-48** (11 kiểu ngoài VS0 ⇒ `VS0301` kèm thông điệp rõ;
> `percent / percent` ⇒ float…). **24 chỗ đỏ trước** trên code cũ (stash bản vá), **237** test, `gate --fast` PASS.
> **ST-12 nay DONE thật** (đã qua review + vá). **ST-14 mở** (`vis run` + TAP + 100 ca + nội suy `{expr}`).

APPROVED 2026-11-12 Agent 0

> **ST-13b DONE** — commit `f1ac635` (đã push): sửa **10 lỗi** review ST-13 bắt — `Type::Never` cho `return`/
> nhánh phân kỳ (hết `VS0412` oan cho `fn f()->int: return 1`), **panic tràn range** `0..=i64::MAX` (checked_add,
> dừng dải), vòng lặp bỏ giá trị cuối thân, `for c in "ab"` ⇒ `char`, range VS0 chỉ `int`, số học đối xứng
> (`size*percent`, `percent/percent`, `%` cùng đơn vị), bảng phương thức kiểm kiểu + keyword sau `.` (`m.has`),
> `pos` tab = **4 cột** + span thật, `break/continue` ngoài vòng/`?` ngoài hàm ⇒ `VS0501`, `Show` thêm `w` +
> escape `char`, decimal trước binary. **246** test, `gate --fast` PASS.
> **ST-14 DONE** — commit `9aafdb8` + `4d52164` (Cargo.lock, không amend commit đã push): **nội suy `{expr}`**
> (`HExpr::Interp`, lower trong cùng ngữ cảnh giải tên), **`vis run`** (exit 0/1/2, `--tap`, `.vish` bail),
> **harness ngữ nghĩa TAP** chạy qua binary, **100 ca/12 nhóm** xanh. **247** test, `gate --fast` PASS.
> **ST-15 mở** (≥ 300 ca ngữ nghĩa).

APPROVED 2026-11-12 Agent 0

> **ST-15 DONE** — `27aa6aa`: **302 ca/16 nhóm** (khoá cứng + chống bỏ ca), cap độ sâu gọi **2.000** (đo 10^4 tràn
> stack), `vis run` chạy interp trong luồng 256 MiB. **ST-15b DONE** — `806e29a`: sửa **4 lệch thiết kế**
> (`push` ghi lại lvalue theo CoW §7.1; closure chụp **snapshot** env; `{x, ..}` hợp lệ §4.3; pattern trường trần
> có `sym`), tách `pat.rs` khỏi `interp.rs` (1.511→1.336), **309 ca**. **ST-16 DONE** — `a89f036`: `vis-parse-fuzz`
> **1 giờ thật: 182.328.926 ca / 4.145.431.263 token / 0 vi phạm**, `vis-dev nightly [--fuzz-minutes N]` chạy được
> (gate + fuzz + vòng tròn fmt 309 tệp) ⇒ `nightly: PASS`, hết exit 2.
> **ST-17 mở** (báo cáo **R2** — chốt VS0).

APPROVED 2026-11-13 Agent 0

> **ST-17 DONE — VÒNG 2 (VS0) CHỐT.** `reports/R2-viscript.{md,json}` (`8059351`, cập nhật phủ đủ 20 hàng `3cc8139`)
> ghi 20 hàng QUEUE, số liệu tái lập được (247 test; 309 ca ngữ nghĩa/16 nhóm; 5 sổ mã `01xx..05xx` = 21 mã;
> fuzz parser 1 giờ 182.328.926 ca / 4.145.431.263 token / **0 vi phạm**; 24 khối tài liệu = 17 xanh + 7 ngoại lệ;
> `nightly --fuzz-minutes 1` PASS), **VD-42..VD-51**, **14 lỗ hổng đã biết**, và **bỏ không đoán** những bộ đếm
> không tái lập được bằng lệnh độc lập.
> **Đối chiếu §19 (VS0)**: ≥ 300 test `.vis` ✓ 309 · fuzz parser 1 giờ không panic ✓ 0 vi phạm · vòng tròn fmt ✓
> trong `nightly` · 100 % fixture tài liệu parse ✓ 24/24. **Mốc VS0 ĐẠT.** Dòng ViScript **hết task trong QUEUE**;
> vòng 3 (khi mở): `cmd_lower` + chữ ký lệnh (VD-22), `vis check --lang`, `vis fmt` cho `.vish`, closure theo tham
> chiếu, nội suy lồng, `match`/`if` làm đối số.

APPROVED 2026-11-13 Agent 0

> **ST-18 DONE** — commit `57d9e1a` (đã push): **`cmd_lower`** (`vis-types/src/lower/cmd.rs`) hạ chế độ lệnh ⇒ HIR,
> `cmd` decl = `HItem::Fn` + `FnKind::Cmd`; **khe ống** (tham số vị trí đầu tiên khớp kiểu ra, giữ chỗ trước) ⇒
> `echo "a\nb" | lines | take 1` = `take(lines(echo("a\nb")), 1)`, không có khe ⇒ `VS0603`; chữ ký đọc từ CST
> (`5`→int, `5mb`→size, `-5` là số, `--flag` + default), thiếu/thừa/sai kiểu ⇒ `VS0602`, lệnh lạ ⇒ `VS0601`,
> `&`/`$x`/`^prog` ⇒ `VS0604`; fail-fast giữa pipeline; 3 builtin `echo`/`lines`/`take`; sổ thứ sáu `COMMAND_CODES`
> (`06xx`, 4 mã) ⇒ `all_codes()` 25, `next_block()` **701**; `vis check/run *.vish` thật (hết `Bail out!`).
> **265 test**, **323 ca sem/17 nhóm** (309 `.vis` + 14 `.vish`), `gate --fast` PASS; 7 kỳ vọng cũ sửa **có lý do**
> trong từng tệp (VD-42 cấp `06xx`, tên dựng sẵn 11, `CommandMode` hạ thật, allowlist 9).
> **ST-19 mở** (`vis-bytecode`).

APPROVED 2026-11-13 Agent 0

> **ST-19 DONE** — commit `750815b` (đã push): crate `crates/vis-bytecode` (không phụ thuộc OS, không `unsafe`) —
> `VBC1` + u16 + 32 byte hash nguồn, const pool nguyên văn, bảng hàm (`arity/nparams/nlocals/max_stack`) + code +
> **bảng span song song**; **45 lệnh**; `Bin`/`Un` mang chính `vis_types::{BinOp,UnOp}`; **`match` = chuỗi test +
> nhảy** (VM không biết pattern); closure `MakeClosure`+`LoadCaptured` (snapshot, VD-51); `DecodeError` 9 biến thể,
> không panic với dữ liệu hỏng. Sinh mã **272/323** ca sem (51 ca lỗi tĩnh), **0 Unsupported**, 3.827 lệnh, 36/45 tag;
> round-trip xanh trên 272 ca + 27 golden. **284 test**, `gate --fast` PASS.
> **ST-20 mở** (`vis-vm`).

APPROVED 2026-11-13 Agent 0

> **ST-20 DONE** — `65cb63b`: `crates/vis-vm` (fetch–decode–execute, ngăn xếp `Vec<Value>` động — **không** tin
> `max_stack`, cap khung 2.000 + cap 10^7 lệnh, bytecode hỏng ⇒ `VS0501` không panic) + **refactor `vis-interp`**
> để mọi ngữ nghĩa (toán tử/builtin/`Show`/`eq`/`iter`) **dùng chung** (284 test + 323 ca xanh y nguyên trước khi
> thêm VM); codegen sửa theo differential (bind pattern, thứ tự bind→guard, `MakeEnum`, closure, `CallValue`);
> **`vis run --vm`** + `--dump-bytecode`. 289 test; differential 253/272 (19 ca lệch khoá trong `KNOWN_LECH`).
> **ST-20b DONE** — `516f668`: sửa **đúng 19 ca** (pattern lồng nhiều tầng, `some/none/ok/err`, `()` vs tuple rỗng,
> `?.` phẳng, `JumpIfNone` luôn pop, `LoadCaptured` trong closure, named-arg theo thứ tự tham số, `T?` tự nâng,
> một quy ước đích nhảy duy nhất) ⇒ **272/272 khớp 0 lệch**, `KNOWN_LECH` **rỗng** + test khoá **hai chiều**.
> **ST-21 mở** (verifier).

APPROVED 2026-11-13 Agent 0

> **ST-21 DONE** — `70a5d38`: `verify(&Module)` + `need_stack` mô phỏng ngăn xếp trên **worklist CFG** (độc lập
> metadata; codegen dùng **chính** hàm này nên `declared == need`), **13 `VerifyError`**, `load_verified`,
> `Vm::run` verify trước; verifier chặn **4 bug codegen**, fuzz đột biến **10 phút 2.289.260 vòng / 0 vi phạm**
> (lần đầu vi phạm ⇒ **sửa VM: mỗi khung một ngăn xếp giá trị**). **316** test.
> **ST-22 DONE** — `f071166`: **100.000 chương trình sinh tự động, 0 lệch** (`DIFF tong=100000 loi_tinh=0
> chay=100000 lech=0 giay=189.2 seed=3523158054`), cổng chạy 1.000 (~1,3 s) + `bin/vis-diff` cho 100k; bắt 1 bug
> span `IterInit`. **ST-23 DONE** — `213e691`: cache `$XDG_CACHE_HOME/vifast/vis/<hash>.vbc` (FNV-1a×4 trên
> compiler+VBC1+mode+toàn văn), nạp **luôn** `load_verified`, ghi tệp tạm + rename, `--no-cache`/`vis cache`;
> **khởi động ấm: 8,96–10,57 ms trung vị, p95 ≤ 11,2 ms ⇒ ĐẠT §19 < 20 ms** (lợi ích cache 7–8 % vì chi phối là
> khởi động tiến trình ~8 ms; mặc định vẫn **interp** — xem VD-56).
> **ST-24 mở** (R3).

APPROVED 2026-11-13 Agent 0

> **ST-24 DONE — VÒNG 3 (KHUNG VS1) CHỐT.** `reports/R3-viscript.{md,json}` (`c194ae7`) — 27 hàng QUEUE, số liệu
> tái lập được: **318 test** (20 diag + 96 syntax + 18 fmt + 65 types + 24 interp + 45 bytecode + 8 vm + 42 cli);
> **323 ca ngữ nghĩa/17 nhóm**; differential gốc **272/272**; **100.000 chương trình sinh tự động, 0 lệch**
> (192,6 s, 519/s, seed 3523158054); **VBC1 53 tag**; verifier **13 biến thể / 26 ca**; fuzz đột biến **10 phút
> 2.311.244 vòng / 0 vi phạm**; **khởi động ấm 8,51 / 9,92 / 8,64 ms trung vị** (p95 ≤ 10,33) ⇒ **đạt §19 VS1 < 20 ms**;
> `gate --fast` PASS; `nightly --fuzz-minutes 1` PASS.
> **Đối chiếu §19 VS1**: **đạt** differential 100k 0 lệch + khởi động ấm < 20 ms; **chưa** generic/trait/module/
> `vis-std`/`vis-host-posix`/`vis run|check|fmt|test` đầy đủ ⇒ **VS1b** (15 mục liệt kê ở §5 báo cáo).
> Dòng ViScript **hết task trong QUEUE**; vòng 4 (đề xuất): VS1b (generic/trait/module, `vis-std`, host posix,
> `cmd` variadic/tham chiếu theo tên, SHA-256 cho consent, đổi mặc định sang VM có đo lại).

APPROVED 2026-11-13 Agent 0

> **ST-25 DONE** — commit `4a03c6d` (đã push): `vis-cli/src/modules.rs` (giải `use ./x`, `./x` ≡ `./x.vis`, `..` +
> `canonicalize`, `std.*` ⇒ `VS0701` "ST-26", `pkg.*` ⇒ `VS0701` "VS2", DFS hậu duyệt, **cấm vòng**, `--dump-deps`);
> `lower_into` **append** vào `Symbols`/`TypeTable` chung ⇒ id **ổn định**; **tầm nhìn `pub` kiểm ở `lower::resolve`**
> (không kéo `tyck`): tên từ module khác cần `pub` **và** `use`, sai ⇒ `VS0301`; parser nhận `use ./x`; `khoa_multi`
> FNV-1a×4 trên compiler+VBC1+mode+Σ(rel+text) ⇒ sửa phụ thuộc **đổi khoá cache**; sổ thứ bảy `MODULE_CODES` `07xx`
> ⇒ `BOOKS` 7, `all_codes()` 28, `next_block()` **801**. **335** test; 323 ca + 272/272 differential xanh; gate PASS.
> Hoãn có ghi: `pub` cho `let`/`const`; `dòng:cột` runtime của hàm trong tệp phụ vẫn theo văn bản tệp chính (HIR chưa
> mang `file`); `use` không re-export; biên `..`.
> **ST-26 mở** (`vis-std` lõi).

APPROVED 2026-11-13 Agent 0

> **ST-26 DONE** — commit `3a54757` (đã push): `vis-types/src/stdlib.rs` (bảng tên + chữ ký + `HA_CHUA_HO_TRO`) +
> `vis-interp/src/stdlib.rs` (một hiện thực, `builtin::call` chuyển tiếp `std.*`, **VM dùng chung** ⇒ **differential
> 70/70**); tên **đầy đủ** `str.split(…)`/`path.join(…)`/`time.now_ms()`/`json.encode(…)`; `std.list.*`/`std.map.*`
> dùng **`Any`** (tyck kiểm số đối số; sai kiểu ⇒ `VS0501` nêu hàm + kiểu nhận) — nợ **VS1c** `list[T]`/`map[K,V]`;
> `sort_by` = closure khoá, tăng dần, **ổn định** (có ca chứng minh); `json.encode` + `encode_pretty`, `json.decode`
> **không** export ⇒ `VS0301` "chưa hỗ trợ (VS1b)"; `use std.{str,list,map,path,time,json}`, `std.fs` ⇒ `VS0701`
> "ST-27". **344** test, **70** fixture std (18/20/9/12/2/10) + 6 ca lỗi; 323 ca + 272/272 differential gốc xanh;
> gate PASS. Hai lỗi của bộ sinh/kỳ vọng được khai trong log, sửa bộ sinh/kỳ vọng **không nới test**.
> **ST-27 mở** (`vis-host-posix`).

APPROVED 2026-11-13 Agent 0

> **ST-27 DONE** — commit `a98d0e8` (đã push): **chương trình ngoài** (§9.3) chạy thật qua `std::process::Command`
> (không shell, đối số mảng, env kế thừa); stdout ⇒ `list[str]` hoặc `--raw` ⇒ `str`; stderr ⇒ **chẩn đoán**;
> `exit != 0` ⇒ `VS0501` (§9.4); **ranh giới OS giữ đúng**: `vis-interp` chỉ biết `trait Ngoai`, CLI cài runner,
> **không host** ⇒ `VS0604` (differential 4 ca `.vish` khớp hai đường); `--no-extern` ⇒ `VS0601`; **variadic
> `...rest`** + **`--x=v`**; timeout 10 s, chạy tuần tự, `--explain-perms`. **357** test; **328 ca sem/17 nhóm**;
> differential **276/276**; gate PASS. **Allowlist doc-examples 9 → 8** (`APPROVED e009411`) vì khối 662 dạng
> `extern cmd … ...rest` nay **hợp lệ** — chặt hơn, không nới. Hoãn: `| complete`, pipe→stdin, consent thật/cap[Proc].
> **ST-28 mở** (generic).

APPROVED 2026-11-13 Agent 0

> **ST-28 DONE** — commit `0423e67` (đã push): **generic** hàm + kiểu `[T]` theo hướng **xoá kiểu + hợp nhất ở
> call-site** (không đơn hình hoá ⇒ **một** hiện thực, differential 0 lệch **theo cấu trúc**): `FnSig{generics,
> raw_params, raw_ret}`, thân hàm dùng `erase_gen` (không `VS0301` oan), call-site `hop_nhat` đệ quy ⇒ mâu thuẫn
> `VS0412` nêu `T` + hai kiểu; record/enum generic payload xoá kiểu, `match` chạy đúng cả 2 kiểu. **Hai bug thật
> được phát hiện & sửa**: (i) `vis check` lọc theo lô khác `vis run` ⇒ `check` nay báo **đúng chẩn đoán đầu**,
> lọc chỉ ở `--syntax` (cổng tài liệu giữ 24 khối = 16 xanh + 8 ngoại lệ); (ii) **VM bỏ qua `fn main`** (entry vào
> thân cấp tệp) — differential trước đó **chưa phủ** chương trình có `main`, nay entry phát `Call(main, 0)`.
> **392** test; **338 ca sem/18 nhóm**; differential **286/286**; gate PASS.
> Hoãn (VS1b): instantiation thật (`Hop[int] ≠ Hop[str]`), phép toán trên `T` kiểm động, constraint/trait ⇒ ST-29.
> **ST-29 mở** (trait + `derives`).

APPROVED 2026-11-13 Agent 0

> **ST-29 DONE** — commit `7633a56` (đã push): **dispatch TĨNH ở tầng hạ** (VD-62) — `x.m(…)` với `x` kiểu tĩnh
> `Record`/`Enum` hạ thẳng `Call(impl_fn)` + `self` theo giá trị ⇒ **cùng HIR** cho interp **và** VM, **không** bảng
> `impls`, **không** bump VBC1 (ca dispatch in `hop`/`7` khớp hai đường); 3 tầng phân giải (nội tại → impl tĩnh →
> `VS0403`); impl↔trait (thiếu/thừa/mơ hồ/mồ côi/`derives` lạ/`Json` trên closure) ⇒ `VS0403` **mở rộng nghĩa**
> (vi/en/ja + snapshot, `TYPE_CODES` vẫn 4, `all_codes()` 28); **`derives` chạy thật** (Eq/Show/Json dùng ngữ nghĩa
> sẵn có); đăng ký hàm `impl` ở interp + codegen; tách `lower/trait_impl.rs` khi R1 đỏ. **421** test; sem 338 +
> differential 286 **giữ nguyên**; gate PASS.
> **Sự cố của tôi (Agent 0)**: commit `520c2fe` (thông điệp `[ST-29]`) đã **cuốn theo 3 tệp đầu của ST-30** (`sha2`
> + hash SHA-256) — cây vẫn xanh, không mất gì, không rewrite lịch sử; đã yêu cầu Agent 2 ghi rõ trong `[ST-30]`.
> **ST-30 mở** (`sha2` + `vis test`).

APPROVED 2026-11-13 Agent 0

> **ST-30 DONE** — `02b97ad`: **`vis test [--vm] [đường dẫn]`** (TAP `1..338`, exit 0/1/2, tự tìm `test/sem`,
> quy ước `.out`/`.err`) + **SHA-256 thật** (`sha2 0.10` — duyệt allowlist, **gỡ nợ VD-54 (1)**: hash nguồn VBC1 **và**
> `cache::khoa_multi`, cache cũ tự miss, nạp vẫn qua `load_verified`; vector FIPS khoá trong test). **435** test.
> **ST-30b DONE** — `e11efae`: **đồng bộ chẩn đoán hai đường** (VM `index` ⇒ `VS0503` + **đúng câu** interp, thêm nhánh
> `Bytes`, thông điệp đệ quy sâu; interp "tên …" thay "hàm …"); **differential so ĐỦ** `(mã,dòng,cột,thông điệp)` trên
> 338 ca ⇒ **312/312 khớp, 0 lệch** (26 ca bỏ qua là **lỗi biên dịch** — nợ); `vis test --vm` **so đúng mã** (không nới),
> **6 SKIP** đúng TAP cho ca cần host. **436** test. Sự cố `520c2fe` (3 tệp ST-30 lỡ nằm trong commit ST-29) do **tôi**
> gây ra, đã ghi trong commit/JOURNAL — không rewrite lịch sử.
> **ST-31 mở** (R4 — chốt VS1b).

APPROVED 2026-11-13 Agent 0

> **ST-31 DONE — VÒNG 4 (VS1b) CHỐT.** `reports/R4-viscript.{md,json}` (`4748ec4`, `head` d92e947) — **436 test**
> (21 diag + 96 syntax + 18 fmt + 118 types + 24 interp + 50 bytecode + 9 vm + 100 cli), **338 ca sem/18 nhóm**,
> `vis test` **338 ok** / `--vm` **338 ok + 6 SKIP**, **parity chẩn đoán 312/312 khớp 0 lệch**, differential
> **100.000 chương trình 0 lệch** (203,5 s, 491/s, so **đủ** cả thông điệp), **SHA-256** thay FNV, `VBC1` 53 tag,
> **7 sổ mã / 28 mã**, `gate --fast` PASS + `nightly --fuzz-minutes 1` PASS, khởi động ấm 10,3–11,1 ms (debug,
> đường `--vm`). **VD-57..VD-63**; **13 lỗ hổng** đã liệt kê; §19 VS1 **đạt phần khung**, phần còn lại = **VS2**.
> Dòng ViScript **hết task trong QUEUE** (36/36 DONE). Vòng 5 (đề xuất, ưu tiên): 26 ca lỗi biên dịch chưa so
> chẩn đoán hai đường · `&self` · `json.decode` · `std.fs` · `| complete`/pipe→stdin · consent/`cap[Proc]` ·
> `Hop[int] ≠ Hop[str]` (generic thật) · `pkg.*`/registry ⇒ VS2.

APPROVED 2026-11-13 Agent 0

> **ST-32 DONE** — commit `774369b` (**Agent 2 tự commit**): harness 3 đường so **cả 338 ca** (286 xanh / 26 lỗi
> biên dịch / 26 lỗi lúc chạy, **0 ca bỏ qua**), **0 lệch**; ca biên dịch so `check` ↔ `run --vm --no-cache` cùng
> `(mã,dòng,cột,thông điệp)` + **ánh xạ exit 1↔2** (không đòi bằng); `chuan_hoa` dùng chung 3 đường + unit-test;
> **`vis_bytecode::compile()` gọi tới cả 26 ca** (không panic); **phát hiện**: **21/26 ca codegen vẫn sinh module và
> VM chạy sạch** dù HIR sai kiểu ⇒ codegen **tin tyck**, khoá hằng số `CODEGEN_TIN_TYCK = 21` (nợ có ý thức).
> **443** test; parity 312/312; differential 100k 0 lệch; gate PASS.
> **ST-33 mở** (`json.decode` + `std.fs` qua `Host`).

APPROVED 2026-11-13 Agent 0

> **ST-33 DONE** — commit `f9e6db5` (**Agent 2 tự commit**): **`json.decode`** (`Result[Any,str]`, object giữ thứ tự,
> number `.`/`e` ⇒ float, tràn i64 ⇒ `Err` ở **byte** đầu token, escape đủ + **surrogate pair** ⇒ `😀`, nửa cặp ⇒ `Err`,
> `null` ⇒ `none`, lỗi nêu **byte offset** đo trên chuỗi Việt+emoji, depth cap 128) + **`std.fs`** (`read/write/exists/list`
> qua **`FsHost`** trên `Host`, mặc định `None ⇒ VS0604` **cả hai đường**; `vis-cli` impl bằng `std::fs`, `--explain-perms`
> in `FS …`; cho phép đường dẫn tuyệt đối/`..`, consent thật ⇒ VS2). **466** test (+23); ~20 ca round-trip JSON, 6 ca FS;
> **338 ca sem giữ nguyên**, parity 312/312, differential 100k **0 lệch** (201,9 s). 3 test cũ sửa **kèm lý do**
> (`std.fs`/`json.decode` nay hợp lệ). **ST-34 mở** (`| complete` + pipe→stdin).

APPROVED 2026-11-13 Agent 0

> **ST-34 DONE** — commit `4ababc6` (**Agent 2 tự commit**): **pipe→stdin** (`a | b` với `b` ngoài ⇒ ghi stdout của `a`
> vào stdin `b`; str nguyên văn, `list[str]` nối `\n`; cap `MAX_STDIN` **8 MiB** ⇒ `VS0501` nêu số byte **trước khi
> spawn**; thread ghi + `drop` + bỏ qua `EPIPE` sau timeout) + **`| complete`** (`{stdout, stderr, code}`, `grep` không
> khớp **hết là lỗi**; chỉ cho stage ngoài, Vis ⇒ `VS0602`); timeout 10 s/**stage** nêu **stage nào**, không cap tổng.
> `Ty::Any` cho `complete` (nợ kiểm tĩnh). **482** test (+16 `pipeline.rs`) + 4 fixture `.vish`; **338 ca sem giữ
> nguyên**; parity 312/312; differential 100k **0 lệch** (206,2 s). **ST-35 mở** (generic thật).

APPROVED 2026-11-13 Agent 0

> **ST-35 DONE** — commit `a0b5525` (**Agent 2 tự commit**): **generic thật** — `RecId`/`EnumId` **interned theo tổ hợp**
> (`Hop[int]` ≠ `Hop[str]`), `inst_args` + `rec_pos`/`enum_pos`, `Hop[T]` chưa ràng buộc **giữ** hành vi xoá kiểu;
> `a.gia_tri` ⇒ **`int`**; hàm liên kết `f[T](x: T, h: Hop[T])`; `Or[int,str]` (`match` thiếu nhánh ⇒ `VS0402`); gán
> chéo hai instantiation ⇒ `VS0412`, cùng instantiation ⇒ sạch; `Hop[Hop[int]]` chạy được (nợ: chưa tạo id cho lớp
> trong). **Không** đụng `codegen`/`VBC1` ⇒ differential **100.000 chương trình 0 lệch** (228,6 s). **507** test;
> **sem 338 → 346** (+8) với bảng cũ→mới ở **đủ 6 chỗ khoá số**; parity **320/320**; `vis test` **346 ok**.
> **ST-36 mở** (R5 — chốt VS1c).

APPROVED 2026-11-13 Agent 0

> **ST-36 DONE — VÒNG 5 (VS1c) CHỐT.** `reports/R5-viscript.{md,json,license}` (`97d3dd4`, `head` 58080a9) —
> `report-check` **PASS (5 báo cáo, 49 hàng QUEUE phủ đủ)**. Số đo **chạy lại**: **507 test** (21 diag · 96 syntax ·
> 18 fmt · **143** types · 24 interp · 50 bytecode · 9 vm · **146** cli) · **346 ca sem/18 nhóm** (327 `.vis` + 19 `.vish`,
> 52 `.err`) · `vis test` **346 ok** / `--vm` **346 ok + 6 SKIP** · **parity 320/320** · **differential 100.000 chương trình
> 0 lệch** (200,4 s, 499/s) · fuzz parser 60 s **3.035.575 ca/0 vi phạm** · `VBC1` 53 tag · **7 sổ/28 mã** · khởi động ấm
> 11,2–11,7 ms (debug) · `gate --fast` + `nightly --fuzz-minutes 1` PASS. **5 lỗ hổng đóng/nới** trong vòng (26 ca biên
> dịch đã so; `grep exit 1` hết là lỗi; `json.decode`; `std.fs`; `Hop[int] ≠ Hop[str]`), **15 nợ còn lại** ghi rõ.
> **VS2** (async/kênh/net/Landlock/LSP/gói/4 nền tảng) là vòng sau. QUEUE: **41/41 hàng DONE**.

APPROVED 2026-11-13 Agent 0

> **VÒNG 6 mở (VSH1a — chế độ shell dùng được, §9.5–§9.7)**: 8 hàng `ST-37..ST-43` (ST-40 tách a/b) ≈ 21,5 h ≈ 3 vòng.
> Căn cứ lộ trình §19 (bước "shell dùng được") + pipeline mẫu §9.7 (khối tài liệu dòng 672 **còn** trong allowlist 8 mục).
> **ST-37 mở**; thứ tự bắt buộc ST-37 → ST-38 → … → ST-42 → ST-43; ST-37..ST-42 đụng `lower/cmd.rs` + `tyck` nên
> **không** chạy song song. Allowlist `doc-examples` **chỉ** được giảm **8 → 7** khi ST-42 **thật xanh** (Agent 0 sửa).

APPROVED 2026-11-13 Agent 0

> **ST-37 DONE** — commit `02daf9f` (**Agent 2 tự commit**): **khe ống hợp nhất** (chọn vị trí theo **khả năng nhận**
> bằng hợp nhất một chiều; `up_ty = {error}` ⇒ khe **vị trí đầu**), `CmdSig`/`SigSource` ở `vis-types` (`cmdsig.rs`),
> `kieu_stage` cho `CmdGroup` suy kiểu literal ⇒ `( [1,2] ) | take 1` ⇒ **`[1]`**; `VS0603`/`VS0602` giữ nghĩa cũ.
> **521** test (+14 `khe_ong`); sem **346 → 349**, `vis test` **349 ok**, parity **323/323**, `fuzz_mutate` 297;
> differential 100k **0 lệch** (197,8 s). **Allowlst doc-examples 8 → 6** (VD-70): khối **1451** và **672** (§9.7) nay
> **hợp lệ** — chặt hơn, không nới (APPROVED `59a407c` + `260fe8a`). Nợ **VSH1b**: `[1,2] | take 1` không ngoặc ⇒
> `VS0201`; `( … )` làm đối số ⇒ `VS0603`. **ST-38 mở** (closure-arg + `where`/`each`/`first`/`count`).

APPROVED 2026-11-13 Agent 0

> **ST-38 DONE** — commit `86f7804` (**Agent 2 tự commit**): **closure-arg** hạ thành `Lambda` với span **tuyệt đối**
> bằng đệm **byte** (có ca chứng minh bẫy nhiều byte: dòng tiếng Việt + emoji trước closure ⇒ cột **không đổi**);
> **4 lệnh mới** `where`/`each`/`first`/`count` (hai chỗ `cmdsig` + `builtin`, **một** hiện thực qua `Host::call`);
> `each` trả `()` và là **stage cuối** (nối tiếp ⇒ `VS0603`); `first` rỗng ⇒ `none`; **sugar `it`** (`where size > 5mb`
> ⇒ `it.size > 5mb`, `where 1` ⇒ `VS0412`); `BUILTIN_VALUE_NAMES` **giữ 11**. **538** test (+17); sem **355**
> (336 `.vis` + 28 `.vish`), `vis test` **355 ok**, parity **329/329**; differential 100k **0 lệch** (229,5 s).
> **Allowlst giữ 6** (khối 672/1451 vẫn xanh). Hai bug tự phát hiện & sửa: `lower_vach` hạ vào **bảng chung**
> (`RecId` ẩn danh); **không** thêm 4 lệnh vào `BUILTIN_VALUE_NAMES` (làm hỏng `fn first` của người dùng ⇒ `VS0302`).
> Nợ: `where size` (kiểu không `bool`) hiện `VS0501` lúc chạy — "kiểm tĩnh khi pipe chưa rõ kiểu".
> **ST-39 mở** (`sort_by --desc`).

APPROVED 2026-11-13 Agent 0

> **ST-39 DONE** — commit `1c3f1d3` (**Agent 2 tự commit**): **`sort_by` shell** = lệnh builtin **mới** (một hiện thực
> qua `Host::call`), `--desc` là **cờ** (không bị khe ống chiếm), khoá `int`/`size`=**byte**/`duration`=**ns**/`str`=
> byte UTF-8, **ổn định cả hai chiều** (đảo comparator, có ca `--desc` + khoá `size` bằng nhau); khoá `bool` ⇒
> **`VS0412`** (kiểm ở lowering cho `.vish` + `kiem_builtin` cho `.vis`, runtime chốt `VS0501`). Phát hiện & sửa:
> parser **nuốt `--desc` vào thân closure**. **550** test (+12); sem **359** (336 `.vis` + 32 `.vish`), `vis test`
> **359 ok**, parity **333/333**; differential 100k **0 lệch** (202,8 s); allowlist **vẫn 6**. Nợ: **hai** `sort_by`
> (shell vs `std.list`) ⇒ hợp nhất ở VS2; chạy end-to-end khối 672 ⇒ ST-42.
> **ST-40a mở** (prelude bản ghi + `it` + viết tắt trường + `VS0605`).

APPROVED 2026-11-13 Agent 0

> **ST-40a DONE** — commit `9b97d75` (**Agent 2 tự commit**): **prelude bản ghi** `FileEntry`/`ProcEntry` khai trong
> `vis-types/src/prelude.rs`, đăng ký **chỉ cho `.vish`** (`.vis` không thấy ⇒ không `VS0302` oan); `it` nay suy **kiểu
> thật** từ kiểu phần tử (`kieu_text`/`kieu_stage`, **không** đụng `CmdSig`), không suy được ⇒ **kiểm động** như cũ;
> **`VS0605`** "trường không có trên `it`" vào `COMMAND_CODES` (4 → 5) ⇒ `all_codes()` **29**, `BOOKS` 7,
> `next_block()` 801. **563** test (+13); sem **363** (336 `.vis` + 36 `.vish`), `vis test` **363 ok**, parity **337/337**;
> differential 100k **0 lệch** (204,2 s); allowlist **vẫn 6**; khởi động **+0,08 ms** (10,06 → 10,14). Phát hiện: `.vish`
> **không có** cú pháp khai `cmd`/`type`/`let` có kiểu ⇒ ca "chạy thật" dùng **literal bản ghi khớp prelude** (đã khoá
> interp ↔ `--vm`); nợ: khi `.vish` có cú pháp khai `type` thì phải `VS0302` với prelude.
> **ST-40b mở** (`get`/`select`).

APPROVED 2026-11-13 Agent 0

> **ST-40b DONE** — commit `942bcff` (**Agent 2 tự commit**): **`get`** (`get name` ⇒ sugar `it => it.name`, `list[K]` thật)
> + **`select`** (`select pid, name` ⇒ hạ lambda trả **bản ghi ẩn danh** `list[record#N]` với trường lấy từ `it`, giữ **đúng
> thứ tự đối số**) — **không** thêm `Type::Any`; trường lạ ⇒ **`VS0605`**; `it` kiểu `{error}` ⇒ **kiểm động** (không oan);
> `get` trên list rỗng ⇒ `[]`. **576** test (+13); sem **367** (336 `.vis` + 40 `.vish`), `vis test` **367 ok**, parity
> **341/341**; differential 100k **0 lệch** (225,9 s); allowlist **vẫn 6**; R1 đỏ ⇒ **tách** `lower/chieu.rs` (188 dòng,
> `cmd.rs` 1.390). Nợ: `select` bản ghi ẩn danh chưa `to json` được (cần `Type::Any` ⇒ VS2).
> **ST-41 mở** (`Render`).

APPROVED 2026-11-13 Agent 0

> **ST-41 DONE** — commit `f2c256f` (**Agent 2 tự commit**): **`Render`** (`vis-interp/src/render.rs` + builtin `render`,
> `boc_print` → `boc_render`): `list[bản ghi]` ⇒ **bảng** (căn trái `str`, phải số/`size`/`duration`/`percent`, tiêu đề =
> tên trường), bản ghi đơn ⇒ **khối khoá–giá trị**, `list[bản ghi]` rỗng ⇒ **`(rỗng)`** — **`list[str]`/`list[int]` rỗng
> giữ `[]`**; độ rộng = **số `char` sau NFC** (`unicode-normalization` có sẵn) ⇒ NFD/NFC, emoji, CJK đều đúng; **`print`
> giữ nguyên từng byte**; interp ↔ `--vm` **cùng byte**. **592** test (+16 snapshot); sem **370** (336 `.vis` + 43 `.vish`),
> `vis test` **370 ok**, parity **344/344**; differential 100k **0 lệch** (206,4 s); allowlist **vẫn 6**;
> **7 `.out` đổi cùng lý do `Render`** (liệt kê trong commit) + ~20 assertion Rust. Phát hiện: **cache cũ** trả `Show` ⇒
> nhãn `cache::PHIEN_BAN` `"+render-st41"`. **ST-42 mở** (`ls`/`open` qua `FsHost` + `--fs-mock`).

APPROVED 2026-11-13 Agent 0

> **ST-42 DONE** — commit `331dc58` (**Agent 2 tự commit**): **`ls`** ⇒ `list[FileEntry]` (**kiểu thật**, sắp xếp theo
> tên byte, `--all`, `--long` no-op + nợ, không đối số ⇒ `"."`) + **`open`** ⇒ `str` / `VS0501` nêu đường dẫn;
> **`FsHost` mở rộng** `MucFs` + `list_entries` (**default `Err`** ⇒ impl cũ vẫn biên dịch), `Runner` thật dùng
> `std::fs::metadata`; **`--fs-mock <json>`** (`dirs`/`files`, `size` theo **bảng lexer**: `2mb` = 2 000 000 và `2mib` =
> 2 097 152 — khoá bằng ca), `modified` = duration **từ epoch** ⇒ tất định; **kiểm `doc-examples`: allowlist vẫn 6**
> (không khối đang đỏ nào chuyển xanh). **606** test (+14 `ls_open`); sem **370 không đổi** (4 fixture `test/cmd/fs/**`
> chạy bằng test riêng vì `vis test` không có `--fs-mock`) ⇒ **6 chỗ khoá số không đổi**, ghi rõ trong commit thay vì
> tăng số ảo; differential 100k **0 lệch** (207,2 s). Sửa: cờ bị bỏ khi ô sau cờ để trống (phải điền default **mọi**
> ô trống); R3 cần sidecar `.license` cho `mock.json`.
> **ST-43 mở** (R6 — chốt VSH1a).

APPROVED 2026-11-13 Agent 0

> **ST-43 DONE — VÒNG 6 (VSH1a) CHỐT.** `reports/R6-viscript.{md,json,license}` (`a63914a`, nền 05f3311) —
> `report-check` **PASS (6 báo cáo, 49 hàng QUEUE phủ đủ, `blocked[]` rỗng)**. Số đo **chạy lại**: **606 test**
> (21 diag · 96 syntax · 18 fmt · 143 types · 24 interp · 50 bytecode · 9 vm · **245** cli) · **370 ca sem/18 nhóm**
> (327 `.vis` + 43 `.vish`, 52 `.err`) · `vis test` **370 ok** / `--vm` **370 ok + 6 SKIP** · **parity 344/344** ·
> **differential 100.000 chương trình 0 lệch** (208,0 s, 481/s) · fuzz parser 60 s **2.853.740 ca/0 vi phạm** ·
> `VBC1` 53 tag · **7 sổ/29 mã** (`VS0605`) · khởi động ấm 11,4–11,6 ms (debug) · prelude **+0,08 ms** ·
> `gate --fast` + `nightly` PASS · **allowlist 8 → 6** (khối 1451 + 672 xanh thật; 672 **chạy** từ ST-42).
> **17 lỗ hổng còn mở** ghi trong `R6` §5. Vòng sau (**VSH1b**): cú pháp khai `cmd`/`type`/`let` có kiểu trong `.vish`;
> `where` kiểm tĩnh đầy đủ; `--long` thật; `( … )` làm đối số + chẩn đoán tuyệt đối trong ngoặc.

APPROVED 2026-11-13 Agent 0

> **ST-44 DONE** — commit `32c5af5` (**Agent 2 tự commit**): **`.vish` có khai báo kiểu** bằng cách **tách vùng** ở cột 1
> (từ khoá item + `let`): tiền tố parse lại **Mode::Script** với span **tuyệt đối**, hạ vào **cùng** `Symbols`/`TypeTable`;
> `type`/`enum`/`let x: T`/`cmd f(...) -> T` dùng **nguyên** `item()`/`suite()` của `.vis`; khai báo **sau** stage ⇒
> **`VS0201`** + gợi ý; trùng tên (kể cả prelude) ⇒ **`VS0302`**; `.vish` không khai báo ⇒ hành vi **y hệt vòng 6**
> (4 fixture ST-42 xanh + ca riêng); `.vis` **không** đổi. **622** test (+16 `vish_decl`); sem **370 không đổi**;
> `vis test` **370 ok**; parity 344/344; differential 100k **0 lệch** (203,1 s); allowlist **6**. Phát hiện & sửa:
> `f()` phải là **lời gọi 0 đối số** (không thì bị coi là lệnh `(`); `let x: int = ls | grep foo` vẫn là **shell-let**
> (phân biệt theo RHS) ⇒ không phá golden `test/cmd/golden.cmd` ca 24.
> Nợ mới: **thân `cmd` bằng pipeline lệnh** (thụt) chưa hỗ trợ ⇒ **VSH1b-2** (task riêng).
> **ST-45 mở** (`where` kiểm tĩnh đầy đủ).

APPROVED 2026-11-13 Agent 0

> **ST-45 DONE** — commit `ea42dd1` (**Agent 2 tự commit**): đo trước ⇒ **`where` đã kiểm tĩnh từ ST-38** (`VS0412` nêu kiểu
> thật); việc thật là **nới `each`** (kiểu trả lambda = `{error}` ⇒ không `coerce`, giống `for` trong Rust) + **bộ
> regression 14 ca** giữ `where` không bị nới ngược + ca **kiểu không biết ⇒ kiểm động** (`( [] ) | where size` ⇒ `[]`).
> Ca bắt buộc: `.vish` **khai báo** + `cmd loc() -> list[FileEntry]` + `where size` ⇒ `VS0412`; bản `> 5mb` ⇒ bảng,
> interp ↔ `--vm` **cùng byte**. **636** test (+14); sem 370; parity 344/344; differential 100k **0 lệch** (202,7 s);
> allowlist 6. Khác brief (đo được): `each x => x.khong_co` (closure tường minh) ⇒ **`VS0403`**, còn `where khong_co`
> (viết tắt `it`) ⇒ **`VS0605`** — **cả hai đều đỏ**, 2 ca khoá cả hai mã; thống nhất mã ⇒ task riêng.
> R1 đỏ khi thêm hook ⇒ **tách** `lower/sugar.rs` (271 dòng), `cmd.rs` còn 1.252.
> **ST-46 mở** (`ls --long` thật + `( … )` làm đối số + chẩn đoán tuyệt đối trong ngoặc).

APPROVED 2026-11-13 Agent 0

> **ST-46 DONE** — commit `2207ea4` (**Agent 2 tự commit**): **`ls --long` thật** (`MucFs.mode: u32`, **`FileEntryLong`**
> bản ghi **riêng** 6 trường ⇒ kiểm tĩnh được `select mode`; `mode` = `rw-r--r--`, `modified_time` = **UTC**
> `YYYY-MM-DD HH:MM`, tự viết civil-from-days, **không** thêm crate; không `--long` ⇒ 4 cột **không đổi**) + **nhóm là
> đối số** (`take ( 1 + 1 )`; bỏ `VS0603`, sai kiểu ⇒ `VS0412`; nhóm rỗng vẫn là gọi 0 đối số) + **chẩn đoán tuyệt đối
> trong `( … )`** (đệm **dấu cách**, không `
` ⇒ không `Indent` giả; `( 1 / 0 )` ⇒ **`3:3`**). **651** test (+15);
> sem 370; parity 344/344; differential 100k **0 lệch** (224,8 s); allowlist 6. Kỳ vọng cũ đổi **kèm lý do**:
> `ls_open` (`--long` hết no-op ⇒ 6 cột), `khe_ong` (nhóm-đối-số `VS0603` → `VS0412`).
> Nợ còn: `( ls … --long )` (nhóm-**lệnh** có cờ), nhóm **nhiều dòng** ⇒ `VS0201`, `where ( x => x )` kiểm **động**.
> **ST-47 mở** (R7 — chốt VSH1b).

APPROVED 2026-11-13 Agent 0

> **ST-47 DONE — VÒNG 7 (VSH1b) CHỐT.** `reports/R7-viscript.{md,json,license}` (`a06434b`, nền 426eb91) —
> `report-check` **PASS (7 báo cáo, 53 hàng QUEUE phủ đủ, `blocked[]` rỗng)**. Số đo **chạy lại**: **651 test**
> (21 diag · 96 syntax · 18 fmt · 143 types · 24 interp · 50 bytecode · 9 vm · **290** cli) · **370 ca sem/18 nhóm**
> (327 `.vis` + 43 `.vish`) · `vis test` **370 ok** / `--vm` **370 ok + 6 SKIP** · **parity 344/344** · **differential
> 100.000 chương trình 0 lệch** (212,2 s, 471/s) · fuzz parser 60 s **3.003.112 ca/0 vi phạm** · `VBC1` 53 tag ·
> **7 sổ/29 mã** · khởi động ấm 11,1–11,3 ms · **allowlist 6** · `gate --fast` + `nightly` PASS.
> **§19 VSH1b: 8/8 hạng mục đạt.** **14 nợ còn mở** + **VSH1b-2** + **VS2** (async/kênh/net/Landlock thật/LSP/gói/
> registry/4 nền tảng) ghi trong `R7` §5 — **ViScript chưa hoàn thiện toàn bộ**, khoảng cách đã liệt kê chính xác.

APPROVED 2026-11-13 Agent 0

> **ST-48 DONE** — commit `739fc3e` (**Agent 2 tự commit**): **thân `cmd` bằng pipeline lệnh** (VSH1b-2) theo chiến lược
> **thử-script-trước, lỗi-thì-lệnh** trong parser; **filler `()` giữ nguyên byte** cho tiền tố ⇒ **offset tuyệt đối không
> đổi** (ca: `type` **sau** `cmd` thân-lệnh báo lỗi cố ý ⇒ **`5:5`** đúng); hạ bằng `ha_pipeline` ⇒ `HItem::Fn.body =
> [giá trị cuối pipeline]`, **không** tạo `HExpr::Pipeline`; thân = **đúng một pipeline** (rỗng/>1/`let`+lệnh ⇒ `VS0201`).
> **666** test (+15); sem 370; `vis test` **370 ok** / `--vm` 370 ok+6 SKIP; parity 344/344; differential 100k **0 lệch**
> (174,0 s, 575/s); allowlist **6**. Chống hồi quy đủ 4 điểm (sem, `cmd` script cũ, `--fs-mock` interp↔`--vm` cùng byte,
> item sau thân đúng dòng:cột). Tách `lower/than.rs` khi R1 đỏ.
> Nợ: thân lệnh **chưa dùng tham số `cmd`**; body "vừa script vừa lệnh" ⇒ `VS0201` (bảo thủ).
> **ST-49 mở** (nhóm-lệnh có cờ + nhóm nhiều dòng).

APPROVED 2026-11-13 Agent 0

> **ST-49 DONE** — commit `e0c084a` (**Agent 2 tự commit**): **xoá** `lower_nhom_lenh` (nay **một** đường hạ nhóm:
> newline→dấu cách 1 byte→1 byte, `parse_voi` + span **tuyệt đối**, Lowerer con chung bảng) ⇒ **nhóm-lệnh có cờ** chạy
> (`( ls --long )` ⇒ **6 cột**, `kieu_stage` trả `list[FileEntryLong]` ⇒ `… | select mode` kiểm tĩnh) + **nhóm nhiều dòng**
> (`( 1 +
 1 )` ⇒ 2; `( 1 +
 0 / 0 )` ⇒ **3:3**; nhóm chứa pipeline). **680** test (+14); sem 370; `vis test` **370 ok** /
> `--vm` 370 ok+6 SKIP; parity 344/344; differential 100k **0 lệch** (210,4 s); allowlist **6**.
> **APPROVED** đổi golden `ls | (a | b)` từ `!ERR` → hợp lệ (có dòng lý do), ca 24 không đụng.
> Nợ: trong nhóm chỉ hạ **một** pipeline; kiểu tĩnh lấy **stage đầu**.
> **ST-50 mở** (thống nhất mã field-access + `where ( x => x )` kiểm tĩnh).

APPROVED 2026-11-13 Agent 0

> **ST-50 DONE** — commit `da50f28` (**Agent 2 tự commit**): **`VS0605` → `VS0405`** (chuyển từ sổ **lệnh** `06xx` sang sổ
> **kiểu** `04xx` — trường là **lỗi kiểu**, VD-27; `COMMAND_CODES` 5→**4**, `TYPE_CODES` 4→**5**, `all_codes()` **29**,
> `BOOKS` 7, `next_block()` 801 **không đổi**); `kiem_field` ⇒ `VS0405`, **`VS0403` giữ 6 chỗ khai báo**; hạ dùng `ma_truong()`
> tra theo **id** (không theo chỉ số mảng). **`where ( x => … )` kiểm tĩnh**: nhánh `"where"` trong `kiem_builtin` (thân
> phải `bool` ⇒ `VS0412`; `{error}` ⇒ kiểm động) + hạ gán kiểu tham số lambda = phần tử ⇒ `where ( x => x.khong_co )`
> ra `VS0405` **tĩnh**. **692** test (+12); sem 370; `vis test` **370 ok** / `--vm` 370 ok+6 SKIP; parity 344/344;
> differential 100k **0 lệch**; allowlist **6**. Tách `tyck/truong.rs` (R1). Nợ: `VS0403` còn nghĩa rộng; `where` chưa kiểm
> kiểu tham số khi lambda không chú thích.
> **ST-51 mở** (stage đầu là biểu thức).

APPROVED 2026-11-13 Agent 0

> **ST-51 DONE** — commit `e721858` (**Agent 2 tự commit**): **stage đầu là biểu thức** (không cần ngoặc) — nhận **hẹp
> nhất**: chỉ `[`, `{`, string literal ở **đầu dòng**, có `|` độ sâu 0, tiền tố parse Script **0 chẩn đoán** ⇒
> `[1,2] | take 1` ⇒ **`[1]`**, `{size: 6mb, …} | where size > 1mb` **chạy thật** (bọc `HExpr::List` một phần tử ⇒
> mọi stage hạ nguồn hiểu như một dòng); **không** nhận số/biến/tên lệnh (giữ §9.5). **704** test (+12); sem 370;
> `vis test` **370 ok** / `--vm` 370 ok+6 SKIP; parity 344/344; differential 100k **0 lệch** (212,9 s); allowlist **6**;
> golden ca 24 + ca `ls | (a | b)` xác nhận xanh. Sửa kèm lý do: `khe_ong::khong_ngoac_van_la_loi_parse` →
> `bieu_thuc_dau_khong_ngoac_chay`. Nợ: biến không nhận; bọc-list chỉ cho stage **đầu**; `[1,2]` trần vẫn `VS0201`.
> **ST-52 mở** (R8 — chốt VSH1c).

APPROVED 2026-11-13 Agent 0

> **ST-52 DONE — VÒNG 8 (VSH1c) CHỐT.** `reports/R8-viscript.{md,json,license}` (`a0cbe79`, head a44c357) —
> `report-check` **PASS (8 báo cáo, 58 hàng QUEUE phủ đủ)**. Số đo **chạy lại**: **704 test** (343 cli) · **370 ca
> sem/18 nhóm** · `vis test` **370 ok** / `--vm` **370 ok + 6 SKIP** · **parity 344/344** · **differential 100.000
> chương trình 0 lệch** (211,5 s, 473/s) · fuzz parser 60 s **3.023.954 ca/0 vi phạm** · `VBC1` 53 tag · **7 sổ/29 mã**
> (`VS0405` ở **sổ kiểu**) · khởi động ấm 10,7–11,0 ms · **allowlist 6** · `gate --fast` + `nightly` PASS.
> **Đóng 5 nợ**: thân `cmd` pipeline · nhóm-lệnh có cờ + nhiều dòng + **một** đường hạ nhóm · một mã field-access đúng
> sổ · `where ( x => x )` tĩnh · stage đầu là biểu thức. **Còn 9 nợ lõi + VS2**, ghi trong `R8` §5.
> **Sửa sổ**: hàng ST-51 trước còn `BLOCKED` (lỗi mẫu regex của tôi khi lật) ⇒ nay `**DONE**` cùng ST-52; `report-check`
> vẫn PASS cả trước và sau (máy kiểm đọc `blocked[]` theo `queue`).
> **Vòng 9 (mở sau)**: ST-53.. (nợ lõi) + khung VS2 có tiêu chí đo.

APPROVED 2026-11-14 Agent 0

> **ST-53 DONE** — commit `d68eb16` (**Agent 2 tự commit**): **`to json`** = stage lệnh **còn thiếu** (trước đó chạy `to`
> như chương trình ngoài ⇒ `VS0501`), nay hạ `Call(std.json.encode, [prev])` (+ `--pretty` ⇒ `encode_pretty`), **không**
> cần `use std.json`; bản ghi **ẩn danh** đi JSON đúng **khoá trường + thứ tự** (serializer bỏ tên) ⇒ **không** cần
> `Type::Any`. **Hợp nhất comparator**: một lõi `sap_xep_theo_khoa` (khoá tính 1 lần, **sort ổn định**, `desc` đảo
> comparator) cho **cả** shell `sort_by` (`KieuKhoa::Moi`) và `std.list.sort_by` (`KieuKhoa::ChiInt`, **giữ** hợp đồng
> `FnIntInt`). **718** test (+14); sem 370; `vis test` **370 ok**; parity 344/344; differential 100k **0 lệch** (204,8 s);
> allowlist **6**. Ca bắt buộc: `( ls ) | where size > 5mb | select name, size | to json` với `--fs-mock` ⇒ JSON đúng,
> interp ↔ `--vm` **cùng byte**. Tách `lower/sugar.rs` (`ha_to_json`) giữ R1.
> Nợ: khoá `str` cho `std.list.sort_by` (cần task riêng).
> **ST-54 mở** (`CODEGEN_TIN_TYCK`).

APPROVED 2026-11-14 Agent 0

> **ST-54 DONE** — commit `4627fbb` (**Agent 2 tự commit**): **`CODEGEN_TIN_TYCK` 21 → 0**. Dữ kiện quyết định: walker trên
> 370 ca cho **0 nút `HExpr::Error` / 0 kiểu `{error}`** kể cả ở ca lỗi tyck ⇒ **không thể** chặn bằng soi kiểu; cửa đúng
> là **truyền chẩn đoán** vào codegen: `compile_kiem(hir, symbols, text, chan_doan)` (giữ `compile()` = `&[]`),
> `Err(CodegenError::ChanDoan{short,dong,cot,msg})` mang **đủ mã + dòng:cột + thông điệp**; CLI `run --vm` truyền
> `loaded.diags`; phòng thủ `HExpr::Error` ⇒ `Err(Unsupported("error-expr"))`. `diag_cmp` dùng `compile_kiem` với **đủ 3
> tầng** chẩn đoán ⇒ **26/26 bị từ chối**, so **đủ** mã/dòng/cột/message; **26/26 không còn ca nào ra module chạy sạch**;
> cửa **chỉ chặn `error`** (cảnh báo vẫn compile — có ca). **730** test (+12); sem 370; `vis test` **370 ok**; parity 344/344;
> differential 100k **0 lệch** (203,0 s); allowlist **6**.
> **ST-55 mở** (R9 — chốt VSH2).

APPROVED 2026-11-14 Agent 0

> **ST-55 DONE — VÒNG 9 (VSH2) CHỐT.** `reports/R9-viscript.{md,json,license}` (`af00658`, head 8c8c36a) —
> `report-check` **PASS (9 báo cáo, 61 hàng QUEUE phủ đủ)**. Số đo **chạy lại**: **730 test** (357 cli · 62 bytecode) ·
> **370 ca sem/18 nhóm** · `vis test` **370 ok** / `--vm` **370 ok + 6 SKIP** · **parity 344/344** · **differential 100.000
> chương trình 0 lệch** (213,2 s) · fuzz parser 60 s **2.996.353 ca/0 vi phạm** · `VBC1` 53 tag · **7 sổ/29 mã** ·
> **`CODEGEN_TIN_TYCK = 0`** · khởi động ấm 11,1–11,3 ms · **allowlist 6** · `gate --fast` + `nightly` PASS.
> **Đóng 3 nợ lõi**: `to json` (ST-53), hợp nhất comparator hai `sort_by` (ST-53), **codegen từ chối HIR có chẩn đoán**
> + `CODEGEN_TIN_TYCK` **21 → 0** (ST-54). **Còn 10 nợ có chủ đích** + **VS2** (bảng 8 hạng mục có **tiêu chí đo** và
> `[ƯL]` trong `R9` §4). **ViScript chưa hoàn thiện toàn bộ.**
> **Vòng 10 (mở sau)**: ST-56 (nợ lõi còn lại) · ST-57 (miếng VS2 đầu tiên: async + kênh) · ST-58 (R10).

## Ghi chú vận hành

- **ST-09 = READY** (2026-11-12, sau ST-06/07/08): bước `doc-examples` trong `./vis-dev gate --fast`; tệp
  **mới** `docs/viscript/doc-examples-allow.txt` (Agent 0 cho phép tạo đúng tệp này, QD §7.4 chỉ định) gồm **7**
  mục: 6 khối có `...` (dòng 305/432/445/610/662/737) + khối 1412 (`assert`/`Entry(trường: …)`); script phải
  kiểm chứng (đúng 7 mục, mỗi mục **thật sự đỏ**, khối đỏ ngoài danh sách ⇒ cổng **đỏ**). Fixture VT09: 17 tệp
  tự sinh (15 `vis` + 2 `vish-session`) + ≥10 tệp viết tay cho quyết định QD §7.1–§7.3; ≥1 case âm chứng minh
  cổng bắt được khối `...` mới.
- **ST-06 = DONE** (2026-11-12, sau ST-05): **pha A** — `viscript/test/cmd/golden.cmd` gồm **22 ca** (đúng
  **12 ca = 12 hàng bảng §9.4** theo thứ tự bảng + **10 ca mơ hồ**), dạng `<nguồn> => <sexp CmdCST>`/`!ERR`,
  quy ước sexp ghi ở đầu tệp, harness `crates/vis-syntax/tests/cmd.rs` khoá đúng 22 ca + lossless;
  **Agent 2 soạn nháp ⇒ Agent 0 duyệt ⇒ mới hiện thực**. **Pha B** — `parse(Mode::Command)` ra CmdCST (chưa
  định kiểu, `cmd_lower` là vòng 2 theo VD-22) + nối `.vish` vào `vis check` (bỏ Bail out cũ, cập nhật test
  ST-08 tương ứng); `vis fmt` **vẫn** từ chối `.vish` (định dạng chế độ lệnh chưa có đặc tả).
- **ST-07 = DONE** (2026-11-12, sau ST-05): `vis fmt <tệp…>` — phạm vi vòng 1 theo **VD-39** (chỉ chuẩn hoá
  **thụt lề** 4 dấu cách/cấp + tab→4 dấu cách + bỏ khoảng trắng cuối dòng + đúng **một** newline cuối; **giữ
  nguyên văn trong dòng**, giữ chú thích, giữ thứ tự token; ghi ra **NFC**). 4 tiêu chí §18.4: (a) lossless
  trên tệp đã đúng chuẩn (điểm bất động), (b) idempotent, (c) sửa tab, (d) tệp lỗi cú pháp ⇒ **không sửa**,
  báo lỗi, exit **2**. Test bắt buộc: quét 15 khối tài liệu + 30 golden + tệp tab + tệp lỗi; kiểm thêm "token
  không đổi" (re-lex sau fmt ⇒ cùng kind/text ngoài trivia).
- **ST-08 = DONE** (2026-11-12, sau ST-05): `vis check <tệp…>` — TAP (`1..N`, `ok`/`not ok`, chẩn đoán
  dạng `#`), `.vis` chạy chế độ script; **`.vish` báo rõ "chế độ lệnh thuộc ST-06" + exit 2**, không im lặng
  bỏ qua; mỗi tệp in **1** chẩn đoán (lọc tầng: tệp có lỗi từ vựng vẫn chỉ 1 dòng). Test bằng chính các khối
  tài liệu (15 khối sạch ⇒ `ok`, 7 khối loại trừ ⇒ `not ok`) + nguồn hỏng tự thêm. Không cần interpreter.
- **ST-05 = DONE** (2026-11-12, sau ST-04): kỳ vọng **30 golden biểu thức** do **Agent 0** soạn ở
  `viscript/test/expr/golden.expr` (định dạng + quy ước sexp ở đầu tệp, VD-34) — Agent 2 **không sửa** tệp này,
  thấy sai thì báo. **CST lossless**: ghép CST theo thứ tự phải ra nguyên văn nguồn (kể cả khoảng trắng, chú
  thích, độ rộng thụt lề). "100 % fixture tài liệu" = **mọi khối ```vis trong `docs/viscript/*.md`** (22 khối,
  chế độ script); khối ```vish-session là pipeline chế độ lệnh ⇒ thuộc **ST-06 (CmdCST)**, không tính ở ST-05.
  Ngoại lệ nào cũng phải khai trong `JOURNAL` + báo cáo, **không** âm thầm bỏ khối.
- **ST-02a = DONE**: `APPROVED 2026-11-12 Agent 0` (SPEC-lex + Phụ lục A sau errata). **ST-04 mở ngay**; golden token là test đỏ đầu tiên của ST-04, golden biểu thức của ST-05. **Ưu tiên ST-04 trước ST-03** (đường tới hạn lexer); `agentctl next` có thể trả ST-03 vì thứ tự bảng — làm ST-04 trước.
- `vifast-agentctl --project viscript next` **bỏ qua** task `REVIEW`; `next` chỉ trả task `READY` có phụ thuộc `DONE`.
- **Mỗi task ≤ 3 h; task lớn tách.** ST-02a tách khỏi ST-02b để phần sổ mã lỗi không phải chờ duyệt.
- Parse chế độ lệnh **hai pha** (QD §7.2 + D §6.D): `vis-syntax` sinh `CmdCST` → `cmd_lower` (vis-types, dùng bảng chữ ký) → **HIR**. "Hai chế độ ra cùng AST" đọc là **cùng HIR**; `cmd_lower` là task vòng 2. Vòng 1 dừng ở lexer → parser/CST → `CmdCST` → `vis-fmt` → runner `--check` cú pháp (chưa interpreter, chưa kiểm kiểu).
- `ST-09` cấm `...` trong fixture: mọi khối mã trong tài liệu phải parse được, ngoại lệ phải khai báo trong SPEC.
- Ưu tiên hai dòng: **OS trước**; dòng này chỉ lấy task khi `v2/agent/QUEUE.md` không còn task `READY` (D2-20). Ngoại lệ đã đóng: ST-02a DONE, ST-04 mở ngay.
- Số nào chưa có nguồn trong QD/tài liệu ViScript thì ghi `[TK]`/`[ƯL]`, không tự đặt.

---

## Vòng 6 — VSH1a (từ issue #21, "bộ lệnh shell có cấu trúc")

APPROVED 2026-11-13 Agent 0

> **Mở vòng 6 — VSH1a.** Issue #21 (`khoa-pham-aepsy`, <https://github.com/vifast-oss/ViFastOs/issues/21>) — tác giả
> **đã đóng #21** và đăng lại y nguyên ở **#22** (`khoajunior`, cùng người, tài khoản cá nhân); **luồng sống là #22**
> (<https://github.com/vifast-oss/ViFastOs/issues/22>) — xin
> `std.shell` (`where/sort_by/ls/ps`) theo §9/§13. **Nhận hướng, không nhận cách chia P1..P4**: chốt lại thành
> **ST-37..ST-43** (8 hàng, ST-40 tách a/b) ≈ 21,5 h ≈ 3 vòng, **sau ST-36 (R5)**. Căn cứ lộ trình §19:
> `VSH1 ← VS1`, `VS2 ← VS1` ⇒ VSH1 là anh em của VS2; issue xin đúng **nửa đầu VSH1**. Gói quyết định + bằng
> chứng đo (3 lệnh `vis check/run` chạy thật ở HEAD `0e65446`):
> [`review/agent0/GOI-ISSUE-21-VSH1a.md`](../../review/agent0/GOI-ISSUE-21-VSH1a.md).
>
> **Quyết định kèm theo: VD-68..VD-75.** Ba điều kiện bắt buộc của vòng:
> (1) **typed** — verb có chữ ký, **không** đi đường `Ty::Any` của `std.list.*` (nợ VD-59 (3));
> (2) **ranh giới tách repo** — mọi thứ thuộc ngôn ngữ nằm trong `viscript/` (`Type`, prelude bản ghi, `Render`,
> `Host`, `HOST_API`); `ps`/`kill` chạy thật là `vis-host-vifast` (ngoài `viscript/`) + `procsrv`, sau VS2;
> (3) **không nới ngưỡng**; allowlist `doc-examples` chỉ được giảm **8 → 7** khi ST-42 **thật xanh**.
>
> **Không làm ở vòng 6** (ghi để khỏi trôi): `stream[T]` lười/SIGPIPE/huỷ pipeline (VSH1b); `walk`/`save`/
> `to json`/`from json[T]`/`http` (VSH1b, mục 1451 của allowlist); `std.shell` **viết bằng ViScript** (seam
> `SigSource` giữ đường, làm sau khi tách repo); cưỡng chế `cap[Dir]`/`cap[Proc]` (VS2); `ps`/`kill` chạy thật.
>
> **Thứ tự**: ST-37 chỉ mở khi **ST-36 DONE**; ST-37..ST-42 đụng `lower/cmd.rs` + `tyck` nên **không** chạy
> song song với nhau. ST-42 là task chạm `docs/viscript/doc-examples-allow.txt` ⇒ **Agent 0** sửa tệp đó, Agent 2
> chỉ báo kết quả cổng.

APPROVED 2026-11-14 Agent 0

> **Mở vòng 10 — nợ lõi + miếng VS2 đầu.** Sáu task **ST-56..ST-61** + **ST-62 (R10)**, làm **tuần tự**; mỗi task
> red-first → xanh → `[ST-##]` commit riêng → push. **Quyết định kèm theo: VD-89..VD-94.**
> **Điều kiện bắt buộc**: (1) **không** nới ngưỡng, **không** tăng allowlist; (2) mọi nợ đóng phải có **≥2 ca**
> (một ca xanh, một ca âm) và **interp ↔ `--vm` cùng byte** khi tính năng chạy ở hai đường; (3) **ST-56 đóng nốt
> `Hop[Hop[int]]`** — nếu probe cho thấy đã chạy, ghi rõ "đã xanh từ trước" + khoá bằng test, **không** giả vờ sửa;
> (4) **ST-60/ST-61**: Agent 2 **báo cáo thiết kế trước khi code** (bộ lập lịch, biểu diễn `Task`/`Chan`, chỗ sửa trong
> `vis-interp` và `vis-vm`, cách giữ cùng byte) — Agent 0 duyệt rồi mới code; (5) async/kênh **không** được hứa hơn
> thực tế: mô hình "chạy tới hoàn tất" là **nợ có chủ đích**, ghi vào `R10` §5.
>
> **Không làm ở vòng 10**: song song thật/đa luồng; đình chỉ giữa thân tác vụ; net/http; Landlock; LSP; gói/registry;
> host windows/wasm; mượn thật `&T`. Các mục này ở vòng 11+.
>
> **Thứ tự**: ST-56 → ST-57 → ST-58 → ST-59 → ST-60 → ST-61 → ST-62. ST-58 chạm `stdlib.rs` + `tyck` nên ST-59 chỉ mở
> sau khi ST-58 xanh. ST-60/ST-61 chạm `vis-types`/`vis-interp`/`vis-bytecode`/`vis-vm` ⇒ **cấm** chạy song song với nhau
> hoặc với task khác đang sửa cùng crate.

APPROVED 2026-11-14 Agent 0

> **ST-56 DONE** — commit `767b195` (**Agent 2 tự commit, đã push**): khoá `str` cho `std.list.sort_by`
> (`KieuKhoa::ChiIntHoacStr`, sort ổn định, trộn kiểu ⇒ `VS0501`), `ref(x)` = **identity** ở cả hai đường (hết
> `Unsupported("ref")`), `&self` parse + hạ như `self`, `Hop[Hop[int]]` **đã xanh từ trước** (khoá bằng 5 biến thể).
> Đo: **747 test** · sem **376/18** · `vis test` **376 ok** · parity **348/348** · differential **100k/0 lệch** (202,7 s)
> · allowlist **6**. Kèm **arity guard** cho `builtin::call` (bytecode đột biến gọi `render` rỗng từng panic) — sửa
> đúng, không đổi chương trình hợp lệ.
> **Phát hiện phải đóng ngay (ST-56b)**: `self.x = v` ⇒ interp `VS0501` ↔ VM **chạy** (in 2); `var xs` gán phần tử ⇒
> interp `VS0501` ↔ VM **chạy**; `--vm` lỗi codegen ⇒ **exit 0**. Tyck **đã** chặn `let`/tham số (`VS0412`) — lỗ nằm ở
> `self` và ở interp. **VD-97**.

APPROVED 2026-11-14 Agent 0

> **ST-56b DONE** — commit `dd04afb` (**Agent 2 tự commit, đã push**). Dữ kiện quyết định: **thân `impl` chưa hề được tyck**
> (`HItem::Impl(_) => {}`) nên `self.x = v` không có chẩn đoán nào; đã thêm kiểm **đích gán** trong thân `impl` (`self`
> không `var` ⇒ `VS0412`; `var self` được; `+=` trên trường/phần tử ⇒ `VS0412`). **Codegen sai thật**: `x[i] = v` phát
> `Set` + **`Pop`** ⇒ VM **im lặng bỏ kết quả** (in `[1, 2]` thay vì `[5, 2]`) — không phải "interp thiếu" như tôi đoán
> ban đầu; nay `Set` + `Assign(slot)`. Interp hạ **đúng** `With`/`Set`, cùng thứ tự đánh giá ⇒ cùng byte. `run --vm` lỗi
> codegen nay **exit 1** (+ `not ok` ở `--tap`). Đo: **756 test** · sem **382/18** · `vis test` **382 ok** · parity
> **352/352** · differential **100k/0 lệch** (194,7 s) · allowlist **6**. Nợ ghi `R10` §5: **tyck toàn bộ thân phương
> thức** (mới chỉ kiểm đích gán), `p.x += 1` là lỗi cú pháp (giữ), base `{error}` chỉ chẩn đoán lúc chạy.

APPROVED 2026-11-14 Agent 0

> **ST-57 DONE** — commit `57ea7212` (**Agent 2 tự commit, đã push**). Ba nợ đóng: **mồ côi đa tệp** (`TraitSig` +
> `LowerOptions.trait_ngoai`, `modules.rs` xuất `pub_traits`; mồ côi thật = **cả** trait lẫn kiểu ngoài chương trình ⇒
> `VS0403`), **siêu trait** (`trait B: A:` kiểm **một tầng** sau 1b, thứ tự `impl` tự do, `VS0403`), **`derives` sinh
> mã** ở lượt **1c** (`show` = `HExpr::Interp`, `eq` = `==`; tên nội bộ có tiền tố; người dùng thắng; generic/`float`/
> `Fn` ⇒ `VS0403` tĩnh). Đo: **765 test** · sem **388/19 nhóm** · `vis test` **388 ok** · parity **356/356** ·
> differential **100k/0 lệch** (198,5 s) · allowlist **6**. Tách `lower/options.rs` để giữ R1 (`lower.rs` 1467).
> **Ba chỗ lệch gợi ý, đã xác minh**: (1) `print(p)` in **dạng ẩn danh** `{x: 1, ten: a}` (ghi chú của tôi "P(x: 1, …)"
> là **sai**) — `show` sinh khớp `Value::show`; (2) cú pháp siêu trait là `trait B: A:` (không phải `trait B(A):`);
> (3) `hir.rs` doc-block `(4,0,4) → (5,0,9)` có lý do.
> Nợ ghi `R10` §5: `enum show`, `derives Json` sinh mã, siêu trait **bắc cầu**, dispatch `impl` **xuyên tệp**, tyck kiểu
> trả phương thức `impl`, `trait_ngoai` chỉ thấy module `use` trực tiếp.
> **ST-58 mở** (`Ty::Any` → biến kiểu; VD-99).

APPROVED 2026-11-14 Agent 0

> **ST-58 DONE** — commit `abdc253` (**Agent 2 tự commit, đã push**): **`Ty::Any` xoá hẳn**, thay bằng `Var(u8)` +
> `List(&'static Ty)` + `Map(…)` (const-friendly, promotion trong `const FUNCS`); `sang_type()` **tổng** (`Var` →
> `TypeRef T/K/V`); `kiem_std` **hợp nhất** bằng `hop_nhat`+`thay_gen` (biến lặp phải `same`, `{error}` không gán, sai
> hình dạng ⇒ `VS0412`, số đối số ⇒ `VS0401`). **16/38** chữ ký đổi (list 8, map 5, json 3); ngoại lệ **có ghi duy nhất**
> `list.sort_by[1] = Ty::FnKhoa`. Đo: **770 test** · sem **388/19** (không đổi) · `test/std` **73** · `vis test` **388 ok**
> · parity **356/356** · differential **100k/0 lệch** (193,2 s) · **7 sổ/29 mã**. Ba ca cũ dựa `Any` đã lật thành lỗi
> **tĩnh** (mạnh hơn), ghi "SỬA KÈM LÝ DO"; ca **TRON** của `sort_by` vẫn runtime `VS0501` nhờ `json.decode` không kiểu.
> Nợ: `Ty::Fn` thật cho `sort_by`; map khoá không-`str`; thứ tự khoá map.
> **ST-59 mở** (nợ shell — biến làm stage đầu, `let` không chú thích, thân `cmd` vừa script vừa lệnh).

APPROVED 2026-11-14 Agent 0

> **ST-59 DONE** — commit `3e2a3cd` (**Agent 2 tự commit, đã push**): **một nguồn luật `let`** (`vis_syntax::let_la_khai`,
> trước có **hai bản sao lệch** ở parser và tầng hạ) ⇒ `let x = 1` / `let xs = [1,2,3]` **bind** ở vùng item; **biến làm
> stage đầu** (`Var/Param/PatBind` đã bind ⇒ stage biểu thức + **kiểu tĩnh** cho `where/get/select`; chưa bind ⇒ giữ đường
> lệnh); **thân `cmd` khai báo + một pipeline** (`KieuThan::KhaiLenh`, một nguồn ở `vis-syntax`, hai Lowerer con lồng khung);
> khai báo sau pipeline / nhiều pipeline ⇒ **`VS0201`**. Đo: **771 test** · sem **394/19** (49 `.vish`, 62 `.err`) ·
> `vis test` **394 ok** · `--vm` **394 ok + 7 SKIP** · parity **360/360** · differential **100k/0 lệch** (202,8 s) ·
> **7 sổ/29 mã**. Hai phát hiện: **luật `let` hai bản sao** (nếu sửa một chỗ thì `check` ↔ `run` lệch) và **parity suýt
> lệch** ở ca nhiều pipeline (đã cho body `{error}`). Nợ: shell-let thật (`let out = ls | wc -l`), nhóm nhiều pipeline +
> kiểu tĩnh lấy stage đầu, lambda `where` không chú thích, `lower/cmd.rs` **1491** dòng (sát R1).
> **ST-60 mở** (VS2 #1 — async; **VD-90/VD-95**).

APPROVED 2026-11-14 Agent 0

> **ST-60 DONE — VS2 #1 (async) CÓ MẶT.** Commit `2542421` (**Agent 2 tự commit, đã push**). **Không thêm tag `VBC1`**
> (vẫn 53): hạ `async fn`/`spawn`/`await` thành builtin nội bộ `$tac_vu`/`$spawn`/`$await` + **một** runtime
> `vis-interp/src/rt.rs` dùng chung hai đường; `Task[T]` = `TypeRef`; lập lịch **FIFO, một luồng, chạy tới hoàn tất**;
> join cuối theo **thứ tự tạo**; lỗi thân ⇒ **`VS0506`** (mã mới, sổ **30**). Tách `host.rs`/`gan.rs` để giữ R1
> (`interp.rs` 1452). Đo: **774 test** · sem **416/20 nhóm** · `vis test` **416 ok** · parity **377/377** · differential
> **100k/0 lệch** (208,7 s) · **10.000 task: interp 309 ms, VM 166 ms, cùng byte, 10.001 dòng stdout, không tràn**.
> **Lệch có ghi**: parser chưa nhận `async` sau `impl`/`cmd` ⇒ hiện ra **`VS0201`** (luật `VS0412` trong tyck giữ làm
> phòng thủ); `async fn` không nhận `--tên`; **chưa** song song thật/không GC (nợ `R10` §5).
> **ST-60b mở** (hotfix review độc lập — xem dưới) và **ST-60c** (derives/siêu trait + SPDX).

APPROVED 2026-11-14 Agent 0

> **ST-60b DONE** — commit `f158085` (**Agent 2 tự commit, đã push**). **Hồi quy parity thật đã đóng**: closure bắt theo
> **giá trị** nay **cấp ô cục bộ riêng** + prologue `LoadCaptured`→`Assign` ⇒ đọc/ghi trong closure không đụng biến ngoài,
> hai đường **cùng byte** (C1 `[1, 2]`, C2 `()\n0`). Phát hiện **cùng họ**: `()` từng bị mã hoá `Const("")` ⇒ VM in chuỗi
> rỗng thay `()` — nay **`PushUnit`** thật. Tyck **lần xuống gốc** cho `self.xs[0]`/`p.xs[0]`; thông điệp **đích lồng** nêu
> đúng gốc; `.vish` shell-let `let out = ls -l` phân loại lại đúng; `std.*` **từ chối** `--tên` (`VS0401`). Đo: **782 test**
> · sem **419/20** · parity **378/378** · differential **100k/0 lệch** (223,6 s) · **VBC1 53** · **7 sổ/30 mã**.
> Tách `tyck/sig.rs` + `bytecode/cerr.rs` để giữ R1. Nợ: shell-let **chưa hạ**; `match` không khớp ở ngữ cảnh giá trị còn
> đẩy `false` (nghi lệch interp↔VM); gán lồng sâu hơn một tầng trường là `VS0412` có chủ đích.
> **ST-60c mở** (derives/siêu trait + số liệu).

APPROVED 2026-11-14 Agent 0

> **ST-60c DONE** — commit `b254f03` (**Agent 2 tự commit, đã push**). **B3**: `sinh_derives` chạy **trước** kiểm siêu trait và
> ghi `(kiểu, Show/Eq)` vào `impl_pairs` — **chỉ khi thật sự sinh được** ⇒ `impl Hien for P` hết `VS0403` oan, thiếu thật vẫn
> đỏ. **B4**: chọn **sinh mã `Show` cho enum** (`InterpPart::Expr(self)` ⇒ `Value::show`) thay vì lỗi tĩnh — rẻ hơn và giữ
> khối tài liệu 214 xanh; lý do ghi STATE/JOURNAL. **B5**: `Eq` chặn **sâu** đệ quy `List/Map/Opt/Result/TypeRef` ⇒
> `list[float]`/`map[str,float]`/`Opt[list[float]]` là `VS0403`. Đo: **782 test** · sem **426/20** · `vis test` **426 ok** ·
> parity **381/381** · differential **100k/0 lệch** (248,0 s) · **7 sổ/30 mã**. Nợ: siêu trait **bắc cầu**, `impl_pairs`
> per-tệp, `co_float_sau` chưa soi bản ghi lồng, `derives Json` chưa sinh mã.
> **ST-61 mở** (VS2 #2 — kênh; **VD-91/VD-96**).

APPROVED 2026-11-14 Agent 0

> **ST-61 DONE — VS2 #2 (kênh) CÓ MẶT.** Commit `782f1b9` (**Agent 2 tự commit, đã push**; agent2c code, agent2d kiểm chứng + đóng).
> `chan[T](n)` + `send`/`recv`/`close`; **một** runtime `rt.rs` cho hai đường (`$kenh` builtin, không thêm tag `VBC1` — vẫn 53);
> chặn ⇒ **drain** hàng đợi FIFO rồi thử lại; hết queue ⇒ **`VS0505` deadlock** (đo **0,085–0,104 s**, không treo); gửi sau
> `close` ⇒ **`VS0504`**; `close` **idempotent**; `recv` sau close trả nốt hàng rồi `none`; `send` sai kiểu ⇒ **`VS0412` tĩnh**.
> Đo: **787 test** · sem **441/21 nhóm** · `vis test` **441 ok** · `--vm` **441 ok + 7 SKIP** · parity **396/396** ·
> differential **100k/0 lệch** (213,4 s, seed 3523158054) · **7 sổ/32 mã** (`all_codes()` 29→32 qua ST-60/61) · gate PASS.
> Log đỏ trung thực: `st61-red-sem.log` (14/15 đỏ đúng lý do; 1 ca **đã xanh sẵn** — `chan` là soft keyword) + `st61-red-rust.log`
> (0/5). Tách `tyck/expr/{kenh,phu}.rs` để giữ R1 (`expr.rs` 1.464).
> **Nợ (R10 §5)**: drain chỉ chạy task `ready` (`spawn`) — task `Moi` do gọi `async fn` chưa được chạy ⇒ `recv` có thể `VS0505`
> dù còn Task; `close` không đánh thức ai; **không** `select`/timeout; `Chan` không qua biên host; `cap=0` không dùng được;
> dung lượng âm là lỗi **lúc chạy**; `Chan` in `<kênh>`.
> **ST-62 mở** (R10 — chốt vòng 10).

APPROVED 2026-11-14 Agent 0

> **ST-62 DONE — VÒNG 10 CHỐT.** `reports/R10-viscript.{md,json,license}` (`2b28f10`, head `6192bbd`) — `report-check`
> **PASS (10 báo cáo, 85 hàng QUEUE phủ đủ)**. Số liệu **chạy lại**: **787 test** · sem **441/21 nhóm** · `vis test` **441 ok**
> · `--vm` **441 ok + 7 SKIP** · parity **395/395** · differential **100k/0 lệch** (208,5 s) · fuzz parser 60 s
> **2.661.866 ca / 0 vi phạm** · **7 sổ/32 mã** · `VBC1` **53** · allowlist **6** · 10k task async **319/168 ms**.
> Vòng 10 đóng **5 nợ lõi** (khoá `str` sort_by, `&self`/`ref`, `Hop[Hop]`, mồ côi đa tệp, siêu trait, derives sinh mã,
> `Ty::Any`) **và mở VS2**: **#1 async** + **#2 kênh** chạy thật hai đường. **Còn**: net/http, Landlock, LSP, gói/registry,
> windows/wasm, hợp chuẩn 4 nền — đã có hàng **ST-63..ST-76** (vòng 11).
> **ST-63 mở** (VS2 #3a — `NetHost` + TCP client; **VD-105**: **R1 trước, code sau**).

APPROVED 2026-11-14 Agent 0

> **ST-63 DONE — VS2 #3a (NetHost + TCP client) CÓ MẶT.** Commit `29f236aa` (**Agent 2 tự commit, đã push**). Bước 0 đúng
> luật: `interp.rs` **1.486 → 1.217** dòng (tách `interp/goi.rs` 287 dòng, **nguyên văn**, chỉ đổi `fn`→`pub(crate) fn`),
> gate xanh **trước** khi thêm net. `NetHost` (7 method, handle `str` mờ, `read → Vec<u8>`), `Host::net()` default `None`
> ⇒ `VS0604`; `MODULES` 7→8, `FUNCS` 38→43 (**+5** `net.*`), **không** variant `Ty` mới, **không** sửa codegen. CLI
> `net.rs` TCP thật, **chỉ loopback**, coi **cả** `WouldBlock` (macOS 35) và `TimedOut` là hết giờ; nối **đủ 4** điểm
> truyền host. **6 ca** Rust loopback thật, so `run` ↔ `--vm --no-cache` (+1 ca cache hit). Đo: **793 test** · sem
> **441/21** giữ nguyên · parity **395/395** · differential **100k/0 lệch** (215,3 s) · **7 sổ/32 mã** · `VBC1` 53.
> **Lệch có ghi**: `net.read` **3 đối số** `(tay, tối_đa, timeout_ms)`; `std.net.connect` dạng **đầy đủ** chưa hạ (dùng
> dạng ngắn sau `use`, như `fs.*`/`json.*`); bỏ `is_open`; `read` trả `str` **lossy**; mặc định connect 1 s.
> **ST-64 mở** (VS2 #3b — `http.get`).

APPROVED 2026-11-14 Agent 0

> **ST-64 DONE — VS2 #3b (`http.get`) CÓ MẶT.** Commit `609d313d` (**Agent 2 tự commit, đã push**). Giao thức HTTP nằm
> trong `vis-interp/src/net.rs` (211 dòng) ⇒ interp ↔ VM **cùng byte**, **không** sửa codegen; URL chỉ `http://` + loopback,
> `Connection: close`, đọc tới EOF/`Content-Length`, **cap body 8 MiB** + header 64 KiB, non-2xx ⇒ `Err("HTTP <mã>")`,
> thiếu host ⇒ **`VS0604`**. `MODULES` 8→9, `FUNCS` 43→**45** (`http.get`, `http.get_timeout`; **bỏ** `body_len` — đúng
> cho phép). **7 ca** Rust loopback thật (200 với body nhiều byte, 404, timeout ≤400 ms, refused, cap 8 MiB, CL>
> body, sai kiểu tĩnh) + `--explain-perms` `NET http GET …` qua hook `nhat_ky` mới. Đo: **802 test** · sem **441/21** ·
> parity **395/395** · differential **100k/0 lệch** (233,0 s) · R1 max 1.491 · `VBC1` 53 · 7 sổ/32 mã.
> **Lệch có ghi (đo được)**: `vis check` lỗi chẩn đoán = **exit 1** (exit 2 = dùng sai CLI); thiếu `use std.http` ⇒
> **`VS0501` lúc chạy** (không phải `VS0301`) — `vis check` permissive với tên cấp tệp là **nợ có sẵn**; `"chào 😀"` =
> **10 byte** (không phải 9 — ghi chú của tôi sai).
> **ST-65 mở** (VS2 #3c — server `listen/accept` + e2e 2 tiến trình).

APPROVED 2026-11-14 Agent 0

> **ST-65 DONE — VS2 #3c (server TCP) CÓ MẶT.** Commit `6514fce1` (**Agent 2 tự commit, đã push**). `net.listen/local_addr/
> accept/accept_timeout/close_listen` (`FUNCS` 45→**50**, `MODULES` 9, handle `"L<n>"`), `accept` timeout 0 = **chặn**,
> `>0` = poll 5 ms, coi **cả** `WouldBlock` và `TimedOut` là hết giờ; đóng chung một đường cho `C…`/`L…`. **5 ca** Rust
> loopback thật, gồm **e2e 2 tiến trình `vis`** bàn giao cổng qua **tệp** (`std.fs.write`, deadline 5 s) chạy **3 vòng**.
> Đo: **807 test** · sem **441/21** · parity **395/395** · differential **100k/0 lệch** (257,7 s) · R1 max 1.491 ·
> CLI `net.rs` 235 dòng. Thời gian e2e: một tiến trình **103/320 ms**, hai tiến trình 3 vòng **93/72/357 ms**, accept timeout
> **320/201 ms** — **giữ nguyên ngưỡng 400 ms/3 s** (máy bận, số cao hơn lý thuyết; ghi rõ, không nới).
> **Minh bạch**: sau push có **1** lần chạy lại gặp 4/5 (một đường chạm trần 400 ms do máy bận) — **không** dùng để lấy xanh;
> 10/10 lần chạy tiếp xanh. Nợ: **một luồng**, không `select`/epoll, phục vụ **tuần tự** một client, chỉ loopback, không TLS.
> **ST-66 mở** (VS2 #3d — ca biên net/http + `--explain-perms` + `--vm` cache/no-cache).

APPROVED 2026-11-14 Agent 0

> **ST-66 DONE — VS2 #3 (net/HTTP) ĐỦ TIÊU CHÍ.** Commit `f1654ef5` (**Agent 2 tự commit, đã push**). **24 ca** net/http/server
> (yêu cầu ≥10, có 4 ca lỗi + timeout/refused), không có mã sản phẩm mới: khoá thêm 6 ca biên (read/write sau `close`,
> `read` cap 64, `accept` 2 lần cùng listener, path+query, body rỗng, `--explain-perms` ≥4 dòng) + `server_mot_lan` chạy
> **4 lượt** (interp, `--vm --no-cache`, cache lần 1, **cache hit**) cùng byte. Đo: **813 test** · sem **441/21** · parity
> **395/395** · differential **100k/0 lệch** (217,0 s) · e2e 2 tiến trình **93–163 ms**/vòng. **Trung thực**: task này
> **không có log đỏ thật** (không đổi mã sản phẩm) — ghi rõ trong JOURNAL thay vì dựng đỏ giả.
> Nợ giữ nguyên (một luồng, không TLS/redirect/keep-alive/chunked, chỉ loopback, handle per-process, cap 8 MiB).
> **ST-67 mở** (VS2 #5a — LSP khung JSON-RPC; **VD-102**).

APPROVED 2026-11-14 Agent 0

> **ST-67 DONE — VS2 #5a (khung LSP) CÓ MẶT.** Commit `c35a0c00` (**Agent 2 tự commit, đã push**). `vis lsp --stdio`:
> framing `Content-Length` (bắt buộc header, `read_exact` N **byte**, cap body 16 MiB, flush từng frame), JSON-RPC 2.0 dựng
> trên `Value::Map` với **JSON của `vis_interp`** (không crate mới); `initialize` (capabilities + `positionEncoding:
> "utf-16"`) / `initialized` (không frame) / `shutdown` / `exit` (0 sau shutdown, 1 trước) / `-32601` / `-32700` (id null) /
> `-32002` / `-32600`; **stdout tuyệt đối sạch**, thoát khi `exit` **hoặc EOF**. **8 ca** snapshot nguyên văn, deadline 5 s
> chống treo. Đo: **821 test** · sem **441/21** · parity **395/395** · differential **100k/0 lệch** (282,6 s) · tệp LSP
> 181/72/66 dòng. Nợ: **chưa** diagnostics/completion/hover thật (ST-68/69), chưa đọc `params`, EOF ⇒ exit 0 (khác spec
> LSP — cần cho test/CI), một luồng.
> **ST-68 mở** (VS2 #5b — diagnostics + vị trí **UTF-16**).

APPROVED 2026-11-14 Agent 0

> **ST-68 DONE — VS2 #5b (LSP diagnostics + UTF-16) CÓ MẶT.** Commit `ab1ed6c8` (**Agent 2 tự commit, đã push**).
> `didOpen`/`didChange`(full sync)/`didClose` ⇒ `publishDiagnostics`, đường phân tích **đúng `vis check`** (lex+parse+lower+tyck,
> gộp 4 nguồn, sắp `(span.start, mã)`, severity 1/2, message làm phẳng); vị trí **0-based, UTF-16, tab = 1** qua hai hàm mới
> (không dùng `modules::vi_tri` 1-based/scalar/tab=4), tính trên `lexed.text` đã NFC/CRLF→LF. **7 snapshot** + **6 unit** vị trí
> — trong đó ca emoji ra **`0:13–0:14`** (scalar sẽ sai `0:12`) và ca tab ra `1:0–1:1` + `1:14–1:17`. Đo: **834 test** · sem
> **441/21** · parity **395/395** · differential **100k/0 lệch** (313,5 s). Clippy đỏ lần 1 (`if_same_then_else`, 2
> `needless_lifetimes`) ⇒ sửa **mã**, không nới lint. Nợ: chưa module-aware (`Use`/`std.*` có thể khác `vis check`), lower/tyck
> vẫn chạy khi lex/parse lỗi (có thể thêm nhiễu), chỉ full sync, chưa completion/hover.
> **ST-69 mở** (VS2 #5c — completion/hover; đích **≥13 phản hồi**).

APPROVED 2026-11-14 Agent 0

> **ST-69 DONE — VS2 #5 (LSP) ĐỦ TIÊU CHÍ.** Commit `40f32a40` (**Agent 2 tự commit, đã push**). **Tổng 22 phản hồi** LSP
> (8 khung + 7 diagnostics + 7 completion/hover) ≥ yêu cầu 10. Completion: item cấp tệp (fn/record/enum/alias/const/trait/
> extern) → biến & tham số (**kind 6**, đọc **HIR** vì `Symbols` không mang kiểu — phát hiện đúng) → builtin (11) → từ khoá
> (40); khử trùng, sắp byte, lọc tiền tố, cap 200. Hover: markdown chữ ký/biến/builtin/từ khoá hoặc `null`, `range` **UTF-16**.
> Capabilities **không đổi** (snapshot ST-67 vẫn xanh). Tách `vi_tri.rs`/`chan_doan.rs`/`goi_y.rs` ⇒ mọi tệp lsp **≤300** dòng.
> Đo: **841 test** · sem **441/21** · parity **395/395** · differential **100k/0 lệch** (214,1 s). Nợ: **chưa module-aware**,
> completion chưa theo ngữ cảnh sau `.`/kiểu, hover chưa field, full sync, biến gộp mọi phạm vi.
> **ST-70 mở** (VS2 #6a — `vis pkg`; **VD-101**).

APPROVED 2026-11-14 Agent 0

> **ST-70 DONE — VS2 #6a (`vis pkg`) CÓ MẶT.** Commit `7743e0e2` (**Agent 2 tự commit, đã push**). `vis pkg add/install/list`,
> registry **local** (`./registry` / `--registry` / `VIS_REGISTRY`), manifest `files[]` **tường minh** (không glob), chuẩn hoá
> BOM/CRLF/một LF cuối, `tree_sha = SHA256("vispkg1\n" + Σ sorted(path+file_sha))`, lock **tất định** (sắp byte, không
> timestamp/đường tuyệt đối, ghi tmp+rename), nhiều version cùng tên theo dải `1.0.0 / 1.9.0 / 1.10.0` với so sánh **theo số**.
> `pkg.rs` 739 dòng; `.gitignore` `.vis/`. **8 ca** (6 CLI + 2 unit), gồm **e2e cài 2 lần cùng byte** và `lock_hong` không ghi
> nửa vời. Đo: **849 test** · sem **441/21** · parity **395/395** · differential **100k/0 lệch** (274,1 s).
> **Mở rộng có ghi**: để `install` **không** cần registry, thêm **store content-addressed `.vis/store/<file_sha>`** (nội bộ,
> **không** nằm trong lock) — đúng ý VD-101(4) và khoá bằng ca 6/7.
> Nợ: **chưa** `use pkg.*` (ST-71), deps ghim đúng version (chưa resolver dải), chưa registry HTTP/ký số, `.vis/store` không GC,
> chưa `pkg remove/update`.
> **ST-71 mở** (VS2 #6b — `use pkg.*` + kiểm sha lúc chạy).

APPROVED 2026-11-14 Agent 0

> **ST-71 DONE — VS2 #6 (gói/registry) ĐỦ TIÊU CHÍ.** Commit `d85e9665` (**Agent 2 tự commit, đã push**). `use pkg.<tên>[.<mod>]`
> đọc `vis.lock`, chỉ nạp tệp **có trong lock**, **chuẩn hoá + so `file_sha`** ⇒ lệch = **`VS0701` "gói bị sửa"** (exit 2, không
> chạy mã sai); `rel` tổng hợp **`pkg/<tên>/<ver>/<path>`** (hai gói cùng `src/main.vis` **không** đụng khoá cache/chẩn đoán);
> vòng qua gói ⇒ **`VS0702`**, không treo; `--dump-deps` in rel gói; chữ ký `load_full` **không đổi**. **5 ca** mới + **1 ca cũ
> đổi kỳ vọng có lý do** (`pkg_bao_vs2` → `pkg_chua_cai_bao_vs0701`). Đo: **854 test** · sem **441/21** · parity **395/395** ·
> differential **100k/0 lệch** (246,6 s) · R1 max 1.491 (`modules.rs` 726, `pkg.rs` 753).
> Nợ: lock **chưa lưu `entry`** (dùng quy ước), chưa resolver dải, chưa ký số/registry HTTP, chưa `pkg remove/update`, lock chỉ tìm
> ở `main_dir`, `.vis/store` không GC.
> **ST-72 mở** (VS2 #4a — quyền `cap[Dir]`/`cap[Net]` cưỡng chế ở host; **VD-103**).

APPROVED 2026-11-14 Agent 0

> **ST-72 DONE — VS2 #4a (`cap[...]` là kiểu thật) CÓ MẶT.** Commit `4094a8d0` (**Agent 2 tự commit, đã push**). `cap[Dir|File|
> Net|Proc|Env|Term]` = `TypeRef{"cap",[…]}` (không thêm biến thể `Type`), `resolve_named` xử `cap` **trước** đối số ⇒ `Dir`
> ngoài `cap` vẫn `VS0301`; `@ p"…"`/`@ "host"` + `(write)` kiểm ở `lower/quyen.rs` (83 dòng) ⇒ sai = **`VS0412`**; bất biến
> "cap **không** so sánh/không in/không literal" khoá bằng ca; `vis run --dry` in quyền từ **CST** của `main` ra **stderr**
> (stdout rỗng ⇒ chứng minh **không chạy**). **6 ca sem** nhóm mới `cap/` ⇒ **447 ca / 22 nhóm** + **4 ca Rust**. Đo: **858 test**
> · `vis test` **447 ok** · parity **398/398** · differential **100k/0 lệch** (286,7 s) · R1 max 1.491.
> **Nợ rõ**: **chưa** cưỡng chế (ST-72b), **chưa** có giá trị runtime cho `cap` (ca sem mới kiểm tyck), chưa Landlock.
> **ST-72b mở** (cưỡng chế ở host + `--grant`).

APPROVED 2026-11-14 Agent 0

> **ST-72b DONE — VS2 #4b (cưỡng chế quyền ở biên host) CÓ MẶT.** Commit `0d7e9dba` (**Agent 2 tự commit, đã push**).
> `Value::Cap{kind,root,write}` (show `<cap[Dir] ./x>`, `eq` false), **bind tham số `cap` của `main` từ CST** (đúng chỉ số tham
> số, cả interp lẫn VM), `FsGuard` canonicalize đích (hoặc thư mục cha nếu chưa tồn tại) rồi `starts_with(root)`; `write` đòi
> `(write)`; ngoài root ⇒ **trap `VS0604`** nêu **quyền thiếu** + path, exit **2** (khác `VS0604` thiếu host giữ exit 1 — có ca
> cũ khoá); `NetGuard` đòi `cap[Net]` + loopback/host khai; **`--grant tên=path`** ghi đè root (tên lạ ⇒ exit 2). **8 ca** chạy
> **cả** interp và `--vm --no-cache`. Đo: **866 test** · sem **447/22** · parity **398/398** · differential **100k/0 lệch** (196,0 s).
> **Nợ có ghi**: **siết mặc định** (không khai `cap` ⇒ không hạn chế) chưa làm; `cap[Net]` vẫn bị luật loopback ST-63;
> `fs.exists` bị chặn trả `false` (không trap); ca sem `cap/` mới kiểm **tyck**.
> **ST-73 mở** — phạm vi **đổi** theo **VD-107**: `--sandbox-check` + khung `vis-host-linux` **không** `unsafe`/không crate mới,
> **không** được tuyên bố "đã cưỡng chế bằng Landlock".

APPROVED 2026-11-14 Agent 0

> **ST-73 DONE — VS2 #4c (probe + khung Linux) ĐÚNG MỨC BẰNG CHỨNG.** Commit `f9c201ba` (**Agent 2 tự commit, đã push**).
> `vis run --sandbox-check` (**không** chạy chương trình) in `landlock:` + `cưỡng chế runtime:` + `cap điều kiện:`, exit 0/3/4 theo
> mức probe; trên macOS đo được `landlock: không hỗ trợ trên macos` (exit 3) — **không** giả vờ. Khung `vis-host-linux/` ở **gốc
> repo** (workspace riêng, **ngoài** `viscript/`) `#![cfg(target_os = "linux")]`, chỉ **đọc** `/sys/kernel/security/lsm`, không
> `unsafe`/dependency, bằng chứng `cargo check --target x86_64-unknown-linux-gnu` **xanh**. Tài liệu `agent/HOST-BAO-DAM.md`
> (38 dòng) chốt **cấm ngôn từ** "đã cưỡng chế bằng Landlock". **6 ca** + 1 ca `cfg(linux)`. Đo: **872 test** · sem **447/22** ·
> parity **398/398** · differential **100k/0 lệch** (101,5 s). Nợ: **chưa** cưỡng chế bằng kernel ở **bất kỳ** nền nào; Linux
> **chưa chạy thật**; `vis-cli` **chưa** dùng crate khung (logic probe lặp).
> **ST-74 mở** (VS2 #7 — đa nền tảng: `cargo check --target` windows + wasm; **VD-104**).

APPROVED 2026-11-14 Agent 0

> **ST-74 DONE — VS2 #7 (đa nền tảng) ĐÚNG MỨC.** Commit `0af15902` (**Agent 2 tự commit, đã push**). Ma trận `cargo check
> --target` **thật**: **18/18 ô PASS** — 7 crate lõi xanh trên **cả** `x86_64-pc-windows-msvc` và `wasm32-unknown-unknown`,
> `vis-host-wasm` (khung ở gốc repo) check xanh cho wasm, `vis-host-linux` cho linux-gnu. **Kết quả đo ngược kỳ vọng của tôi**:
> `vis-cli` × wasm **check xanh** (std chỉ có stub) ⇒ **không** có ô đỏ để khoá; agent **không** tạo đỏ giả và ghi rõ "check xanh
> **không phải** bằng chứng chạy được". Thêm `bin/vis-targets` (49 dòng, exit 1 nếu ô kỳ vọng xanh mà đỏ), `HOST-BAO-DAM.md` 54
> dòng (bảng nhóm crate × target × **mức bằng chứng**), 1 smoke test **0,4 s**. Đo: **873 test** · sem **447/22** · parity
> **398/398** · differential **100k/0 lệch** (194,7 s).
> Nợ: **chưa có runner** nền nào ngoài macOS ⇒ mọi ô chỉ là **check**; `vis-cli` trên wasm **không chạy được**; chưa `cargo build`
> Windows (cần linker).
> **ST-75 mở** (VS2 #8 — bộ hợp chuẩn host ≥50 ca chạy **thật** trên macOS).

APPROVED 2026-11-14 Agent 0

> **ST-75 DONE — VS2 #8 (bộ hợp chuẩn host) ĐỦ TIÊU CHÍ TRÊN macOS.** Commit `2602333d` (**Agent 2 tự commit, đã push**).
> `test/host/suite.txt` (TSV có comment) + harness `host_conformance.rs` (194 dòng) chạy **cả** interp và `--vm --no-cache`, so
> **byte** stdout + exit + stderr: **54 ca / 54 xanh**, **120 lần chạy, 0 lệch**, harness **3,9 s** — phân bổ **FsHost 22 ·
> Ngoai 13 · shell/ls/open 11 · quyền 5 · determinism 3** (`repeat=3`). Mỗi ca **có kiểm thật** (stdout ≠ rỗng hoặc exit cụ thể).
> `bin/vis-host-suite` (40 dòng) in bảng + exit 1 nếu thiếu/đỏ; `HOST-BAO-DAM.md` 71 dòng ghi rõ **Windows/Linux/wasm chưa đo**.
> Đo: **874 test** · sem **447/22** · parity **398/398** · differential **100k/0 lệch** (196,3 s). Đỏ R3 lần 1 (thiếu sidecar `.license`)
> ⇒ thêm **3 sidecar**, không nới luật.
> Nợ: net **không** trong suite (đã có 24 ca riêng, thay bằng 3 ca determinism — ghi rõ); **13 ca Ngoai** khai kỳ vọng `vm:VS0604`
> (VM **không** có host `Ngoai` — đúng thiết kế ST-27); suite phụ thuộc lệnh macOS.
> **ST-76 mở** (báo cáo R11).

APPROVED 2026-11-14 Agent 0

> **ST-76 DONE — VÒNG 11 CHỐT.** `reports/R11-viscript.{md,json,license}` (`c2c31e45`, head `3b96011e`) — `report-check`
> **PASS (11 báo cáo, 86 hàng QUEUE phủ đủ)**. Số liệu **chạy lại**: **874 test** (488 cli) · sem **447/22 nhóm** · `vis test`
> **447 ok** · `--vm` **447 ok + 7 SKIP** · parity **398/398** (+49 lỗi biên dịch) · differential **100k/0 lệch** (352,5 s) ·
> fuzz parser 60 s **4.776.316 ca / 0 vi phạm** · `VBC1` **53** · **7 sổ/32 mã** · allowlist **6** · async 10k **328/182 ms** ·
> host suite **54 ca / 120 lần chạy / 0 lệch** · ma trận target **18/18** · `--sandbox-check` macOS exit 3 (**không hỗ trợ**).
> **Vòng 11 đóng trọn 6 hạng mục VS2 ở mức đo được**: async, kênh, net/HTTP (24 ca), quyền runtime (8 ca), LSP (22 phản hồi),
> gói/registry (13 ca) — cộng 2 hạng mục **đúng mức bằng chứng** (Landlock: **probe** + check chéo; đa nền tảng: **check**),
> và **bộ hợp chuẩn 54 ca trên macOS**. **Còn**: Landlock **thật**, runner **Windows/Linux/wasm**, và **37 mục nợ** (R11 §5).
> **ViScript vẫn chưa hoàn thiện toàn bộ** — nhưng mọi hạng mục VS2 giờ đều có **bằng chứng đo** hoặc **lý do chặn** rõ ràng.

APPROVED 2026-11-14 Agent 0

> **ST-77 DONE — nhất quán #1 (`vis fmt`).** Commit `920cb957` (**Agent 2 tự commit, đã push**). Đỏ trước **thật**: `--check` thư mục
> không nhận thư mục (exit 2), `.vish` bail-out "chưa có đặc tả", render script lên `.vish` **mất thụt lề khối `cmd`**; **phát hiện
> thêm** fmt chỉ chặn `01xx/02xx` ⇒ `.vish` sai **lệnh** (`VS0602`) cho ra **chuỗi rỗng** (mất token) ⇒ nay chặn **mọi** chẩn đoán
> khác tab. Sửa: `format_mode` công khai, `.vish` = `Mode::Command` + **giữ thụt lề gốc**, CLI nhận **thư mục** (đệ quy, sắp byte),
> `--check/--write` đúng mode. **9 snapshot + lossless trên 35 fixture + idempotent**, 3 ca CLI. `fmt --check test/` **xanh 497 tệp**
> (bỏ **136** input cố ý: `test/fmt/**`, `test/check/**`, `test/doc-examples/**`, tệp có sibling `.err` — **có lý do trong mã**).
> Đo: **886 test** (fmt 18→**27**) · sem **447/22** · parity **398/398** · differential **100k/0 lệch** (219,3 s).
> Nợ: `.vish` **không** chuẩn hoá độ sâu khối (giữ thụt lề gốc), fmt **chỉ tầng cú pháp**, chưa reflow dòng dài.
> **ST-78 mở** (`explain` + sổ mã 3 ngôn ngữ).

APPROVED 2026-11-14 Agent 0

> **ST-78 DONE — nhất quán #2 (`explain` + sổ mã).** Commit `3953f6bd` (**Agent 2 tự commit, đã push**). Đo trước cho thấy
> `VS0504/0505/0506` **đã đủ** 3 ngôn ngữ và `--list` **đã 32 dòng** ⇒ **không** giả vờ có lỗi; lỗ hổng thật là `vis explain`
> không tham số rơi vào usage chung (**không nêu số mã**) ⇒ thêm `usage_explain()` đếm **động** (`BLOCKS_RESERVED` + `all_codes()`);
> `VS0506` viết lại **rõ nghĩa** ở cả 3 ngôn ngữ (bỏ meta "message kèm mã gốc"), id/thứ tự **không đổi**. **6 ca CLI** (9 lần gọi
> so **nguyên văn**, `--list` 32 dòng đúng thứ tự `all_codes()`, VS9999/`--lang` sai exit 2, quét **toàn bộ 32 mã** non-empty +
> **không trùng**) + snapshot `messages.rs` 20→**29 ca**. Đo: **892 test** · sem **447/22** · parity **398/398** · differential
> **100k/0 lệch** (253,6 s). Nợ: `usage_explain` chỉ tiếng Việt; `--list` có cột severity.
> **ST-79 mở** (`vis test` với async/kênh).

APPROVED 2026-11-14 Agent 0

> **ST-79 DONE — nhất quán #3 (`vis test` với async/kênh).** Commit `2a62cace` (**Agent 2 tự commit, đã push**). Đo trước **đúng
> như nghi ngờ**: `vis test` trước đây **không chạy** `test fn` (chỉ là fixture `.out/.err`), `async fn` trong thân `test` ⇒
> `VS0201`, `assert` **chưa tồn tại**. Đã làm: runner `test fn` **thật** (CST → mỗi `TestDecl` một ca TAP; chạy bản sao tạm
> `fn main`, giữ `async fn` cấp tệp) qua `vis run`/`--vm`; **join cuối** (task không await vẫn chạy, lỗi ⇒ `VS0506`); **`assert(bool)`**
> dùng chung interp/VM (`SIGS` 21→22, false ⇒ `VS0501` "assert thất bại", **không** mã mới); deadlock kênh ⇒ `VS0505`, **không treo**.
> **8 ca** chạy cả hai đường, so **byte** TAP. Đo: **900 test** · sem **447/22** · parity **398/398** · differential **100k/0 lệch**
> (442,6 s — máy bận). Đỏ R1 lần 1 (`expr.rs` 1503) ⇒ **nén xuống 1.498**, không nới ngưỡng.
> Nợ: `async fn` **trong** `test` chưa hỗ trợ; `assert` chỉ dạng **gọi hàm**; runner biến đổi **văn bản** (câu lệnh cấp tệp chạy lại
> mỗi test); constructor là `Some` (không `some`).
> **ST-80 mở** (LSP module-aware).

APPROVED 2026-11-14 Agent 0

> **ST-80 DONE — nhất quán #4 (LSP module-aware).** Commit `74a48476` (**Agent 2 tự commit, đã push**). Đo trước **ngược dự đoán
> của tôi**: LSP **không** bịa `VS0701`; nó **bỏ sót** chẩn đoán thật (`str.khong_co`, `use ./util` sai, thiếu module) vì chạy
> `LowerOptions::default()`. Sửa: `modules::load_full_voi_text(path, text, …)` (tệp chính = **văn bản buffer**, tệp phụ đọc đĩa;
> chữ ký cũ **không đổi**), `lsp::uri_thanh_duong` (bỏ `file://`, **percent-decode** UTF-8), chỉ publish chẩn đoán của **tệp
> chính**, tệp không có trên đĩa ⇒ đường một tệp như trước (**không** bịa). Ca mạnh nhất: bộ `(mã, dòng, cột-1)` của LSP **bằng**
> `vis check` trên 3 tệp mẫu. **7 ca** (6 + hiệu năng): **5 didOpen 8,01 ms**, 5 open+change **8,82 ms**. Đo: **907 test** · sem
> **447/22** · parity **398/398** · differential **100k/0 lệch** (96,3 s). Nợ: chẩn đoán tệp **phụ** không publish (chủ ý);
> completion/hover **chưa** module-aware; buffer chưa lưu chưa có chẩn đoán module; không cache giữa các `didChange`.
> **ST-81 mở** (`--strict-caps`).

APPROVED 2026-11-14 Agent 0

> **ST-81 DONE — nhất quán #5 (`--strict-caps`).** Commit `8aca7f7b` (**Agent 2 tự commit, đã push**). Cờ mới cho `run` **và** `test`
> (`Quyen.siet`; `bat = siet || !ds.is_empty()`): chạm `std.fs`/`net` mà **không** khai `cap` ⇒ **`VS0604`** nêu "cần khai
> `cap[Dir]`/`cap[Net]`" + **đường dẫn**, exit 2; có `cap` thì luật ST-72b giữ nguyên; `--fs-mock` **vẫn** kiểm quyền (khoá lựa
> chọn). **Mặc định không đổi** (khoá bằng ca (a) + 8 ca `quyen.rs` cũ vẫn xanh). **7 ca** chạy **cả** interp và `--vm --no-cache`.
> Đo: **914 test** · sem **447/22** · parity **398/398** · differential **100k/0 lệch** (217,6 s). Đỏ trước **thật**: cờ chưa tồn tại
> ⇒ "không hỗ trợ cờ" exit 2; 6/7 ca đỏ.
> Nợ: **siết mặc định** vẫn là nợ lớn (phải chuyển toàn fixture), `fs.exists` ngoài quyền vẫn trả `false` im lặng, cờ chỉ áp cho
> `run`/`test`.
> **ST-82 mở** (R12 — chốt vòng 12).

APPROVED 2026-11-14 Agent 0

> **ST-83 DONE — nợ nhỏ #1.** Commit `7bac3c70` (**Agent 2 tự commit, đã push**). Ba nợ đóng: **`fs.exists` ngoài quyền ⇒ trap
> `VS0604`** (cùng câu với `read`, exit 2 — hết im lặng `false`), **`assert <đk>` dạng câu lệnh** (`assert` = **soft keyword**,
> hạ `Call(assert,[expr])`, giữ `assert(x)`, trong `test fn` ⇒ TAP `not ok`), **help `explain` theo `--lang`** đủ **vi/en/ja**
> (số khối/mã vẫn **đếm động**). **6 ca** chạy cả hai đường. Đo: **920 test** · sem **447/22** · parity **398/398** · differential
> **100k/0 lệch** (231,4 s). Hai test cũ **SỬA KÈM LÝ DO** (ca âm `assert xs == [...]` nay **hợp lệ** ⇒ đổi sang `assert 1 2`;
> đo hiệu năng LSP đổi sang **phần thêm** so với nền, **ngưỡng 100 ms giữ nguyên**).
> Nợ: `assert` câu lệnh chỉ một biểu thức (không message); biến tên `assert` ở đầu câu lệnh bị hiểu thành assert; help dịch tay.
> **ST-84 mở** (LSP completion/hover module-aware).

APPROVED 2026-11-14 Agent 0

> **ST-84 DONE — LSP completion/hover module-aware.** Commit `2d3032b7` (**Agent 2 tự commit, đã push**). `Loaded.item_rel` (biết item
> thuộc tệp nào) + `lsp/module.rs` (157 dòng): item **`pub`** của `use ./x` và hàm `std.<m>` đã `use` vào completion (kind + `detail`
> **nêu nguồn**) và hover markdown; khử trùng **tệp chính > module > builtin > từ khoá**, sắp byte, cap 200; tệp **không** có trên
> đĩa ⇒ **không** bịa. **5 ca** + **unit** hiệu năng nạp 5 module (<1 ms; test tích hợp đổi sang đếm frame, **không** nới ngưỡng
> 100 ms). Đo: **926 test** · sem **447/22** · parity **398/398** · differential **100k/0 lệch** (219,9 s); 34/34 ca LSP xanh.
> Nợ: `is_pub` mới cho `fn` (type/enum/const chưa); `detail` `std.*` dùng tên tham số giả; completion chưa theo ngữ cảnh sau `.`;
> hover chưa field; nạp lại module mỗi request.
> **ST-85 mở** (`vis fmt` chuẩn hoá thụt lề `.vish` — **có rào**: nếu thụt lề là ngữ nghĩa, **không** đổi, ghi lý do đo được).

APPROVED 2026-11-14 Agent 0

> **ST-85 DONE — nhánh B, đúng luật (đo trước, không đổi khi ngữ nghĩa).** Commit `12c0231c` (**Agent 2 tự commit, đã push**).
> 6 phép đo: thân `cmd` thụt lề **không** đổi nghĩa (2/4/tab — tab bị `VS0107`), **nhưng cấp tệp thì CÓ** (tệp thụt hết ⇒ **không in
> gì**, exit 0; `topmix` bỏ dòng đầu). ⇒ **không** chuẩn hoá; **0 fixture đổi**; thêm `--indent-report` (đếm dòng lệch chuẩn 4×độ sâu;
> `test/` = **8 tệp/10 dòng**) và `--check` **cảnh báo** stderr, **không đỏ**; lý do ghi `HOST-BAO-DAM.md`. **6 ca**.
> Đo: **932 test** · sem **447/22** · parity **398/398** · differential **100k/0 lệch** (212,2 s).
> **Phát hiện lỗi thật (mở ST-85b ngay)**: dòng **cấp tệp thụt lề** bị **bỏ im lặng** — không chẩn đoán, không in. Đây là lỗi đúng
> nghĩa "im lặng sai", phải đóng trước khi coi `.vish` là ổn định.

APPROVED 2026-11-14 Agent 0

> **ST-85b DONE — lỗi "im lặng bỏ dòng" ĐÃ ĐÓNG.** Commit `9dacea49` (**Agent 2 tự commit, đã push**). Chuỗi im lặng được chỉ đúng
> **file:line**: `lower/khai.rs:28-30` (`la_khai_bao_voi` coi **mọi** dòng thụt là "vùng khai báo") → `khai.rs:79-104` (`tach` nuốt
> tiền tố, `moc` = EOF) → `lower.rs:390-412` → `lower/cmd.rs:426-430` (`run_shell_tu` **bỏ** mọi node `span.start < tu`) ⇒ **không**
> stage nào được hạ ⇒ rỗng, exit 0. Chọn **nhánh (A)** có căn cứ **§9.5** (`.vish` **không** có block cấp tệp): `khai::dong_thut_sai`
> (heuristic văn bản, bỏ qua thân khối mở bằng dòng cột 1 + ngoặc chưa đóng) ⇒ **`VS0201`** "câu lệnh cấp tệp không được thụt lề
> (`.vish` không có block cấp tệp)" tại `dòng:cột`, exit 2; `.vis` script **không** đi nhánh này. **7 ca** (3 dạng lỗi + `.vis` +
> `cmd` 2 dấu cách + nhóm nhiều dòng + `fn` khai báo), **0 golden phải sửa**. Đo: **939 test** · sem **447/22** · host **54/54** ·
> parity **398/398** · differential **100k/0 lệch** (218,7 s).
> Nợ: heuristic **văn bản** (dấu `:` trong chuỗi/comment có thể bị coi mở khối; khối một dòng `cmd f(): x`; nối dòng `\`); gốc
> `khai::tach` **chưa dọn**; `topmix2` **đổi hành vi** (trước chạy dòng thụt như lệnh ngoài — sai thiết kế, nay báo lỗi).
> **ST-86 mở** (fuzz differential cho cú pháp mới).

APPROVED 2026-11-14 Agent 0

> **ST-86 DONE — fuzz differential phủ cú pháp mới.** Commit `1d690869` (**Agent 2 tự commit, đã push**). Bộ sinh phủ **8 nhóm**
> (async 100 · chan 100 · cap 100 · derives 100 · trait 100 · assert 100 · lỗi chạy 100 · lỗi tĩnh 100), hỗn hợp 50% cũ/mới; trên
> 2.000 seed **chạm đủ 8 nhóm** `[252, 222, 172, 124, 124, 163, 33, 36]`; mọi chương trình **kết thúc** (deadlock có `VS0505`).
> **20.000 chương trình seed `20261115`: `lech=0`** (31,8 s, 628/s) và **100.000: `lech=0`** (158,5 s, 631/s) — so **byte**
> stdout/exit + đủ `(mã, dòng, cột, message)` hai đường. **5 ca Rust** (đếm mẫu nhóm, đủ 8 nhóm trên 2.000 seed, lỗi tĩnh, lỗi chạy
> hai đường) + **7 ca CLI lỗi** (send sai kiểu, await không Task, thiếu cap, thiếu gói `VS0701`, deadlock, assert, send-sau-close).
> Cổng giữ **1.000** mặc định; test chạy **2.000 trong 3,2 s**. Đo: **951 test** · sem **447/22** · host **54/54** · parity **398/398**.
> Nợ: mẫu **cố định** (chưa ghép tổ hợp sâu); kênh chỉ cặp cân bằng; chưa sinh `use pkg.*` **thành công**; `@`/`(write)` mới ở mức
> **cú pháp**.
> **ST-87 mở** (cache không che lỗi — 3 chế độ).

APPROVED 2026-11-14 Agent 0

> **ST-87 DONE — cache không che lỗi (đo được).** Commit `0c23dad3` (**Agent 2 tự commit, đã push**). Harness `cache_khong_che.rs`:
> **520 tệp** (`test/sem` 447 + `test/std` 73, gồm 83 ca `.err`) × **4 lần chạy** (interp · `--vm` lần 1 · `--vm` cache hit ·
> `--vm --no-cache`) so **byte** `(exit, stdout, stderr)` ⇒ **513 ca 0 lệch**; **7 ca `.vish` dùng chương trình ngoài** bị loại khỏi so
> interp vì VM **chưa có host `Ngoai`** (**cùng** `VS0604`) — với chúng vẫn kiểm `B1 = B2 = C`. 5 ca: cache hit **141 → 12 ms**;
> `.vis`/`.vish` **khác khoá** (2 tệp `.vbc`); module phụ đổi ⇒ **không** dùng cache cũ; `--strict-caps` **hai lần** đều `VS0604`
> (**cache không che quyền**). Harness **24,9 s**. Đo: **956 test** · sem **447/22** · host **54/54** · parity **398/398** ·
> differential **100k/0 lệch** (158,1 s).
> **Phát hiện lỗi thật (mở ST-87b)**: **fast-path cache là dead code** — `main.rs` đọc **khoá v1** (`khoa`) trong khi mọi chỗ ghi dùng
> **khoá v2** (`khoa_multi`) ⇒ tệp một-module **không bao giờ** cache hit (chỉ đa tệp mới hit).

APPROVED 2026-11-14 Agent 0

> **ST-87b DONE — nhánh (B): xoá fast-path chết.** Commit `32e9bebd` (**Agent 2 tự commit, đã push**). Xác nhận: tệp một module
> **không bao giờ** hit (đọc khoá v1 `cache::khoa` ↔ ghi khoá v2 `khoa_multi`) ⇒ **xoá** fast-path + `cache::khoa`, đường duy nhất
> `khoa_multi` (`main.rs` 1.083 → **1.024** dòng). Sau sửa: **13,4 / 12,1 / 11,7 ms**, byte y hệt ⇒ **không hồi quy**; `use ./x`
> vẫn hit (**mtime `.vbc` không đổi**); sửa tệp chính/phụ đều **miss** (`.vbc` 1 → 2). **5 ca**, gồm ca chống nhánh chết (đỏ trước).
> Đo: **961 test** · sem **447/22** · host **54/54** · parity **398/398** · differential **100k/0 lệch** (153,9 s).
> Nợ: frontend vẫn chạy mỗi lần (~1 ms) vì không còn fast-path bỏ frontend; ca chống nhánh chết **đọc mã nguồn** (brittle).
> **ST-88 mở** (R13 — chốt vòng 13).

APPROVED 2026-11-14 Agent 0

> **ST-89 DONE — khoảng lệch interp ↔ VM CUỐI CÙNG đã đóng.** Commit `865342a8` (**Agent 2 tự commit, đã push**). VM nhận
> `Option<&mut dyn vis_interp::builtin::Ngoai>` (**cùng trait** với interp; VM **không** gọi `std::process`), CLI truyền **cùng**
> `ngoai::Runner` ⇒ `vis test --vm` **447 ok / 0 SKIP** (từ **7 SKIP**), host suite **54/54 × 2 đường** (120 lần chạy), 13 token
> `vm:vs0604` trong `suite.txt` **bỏ có lý do**. 5 ca mới so **byte** hai đường. Đo: **966 test** · sem **447/22** · `diag_cmp`
> **364/49/34** · differential **100k/0 lệch** (148,4 s) · cache **520×4 0 lệch**.
> **ĐÍNH CHÍNH CHỈ ĐẠO CỦA TÔI**: tôi yêu cầu "parity 398 → **405**" — **sai**. 7 ca đó **đã nằm trong 398** (trước khớp bằng `VS0604`);
> nay khớp **thật**. Phân hoạch đúng: **447 = 398 chạy được + 49 lỗi biên dịch**; `398 + 7` là **đếm trùng**. Agent **giữ số đo thật**
> và ghi chú — đúng luật "số đo thắng chỉ đạo". Từ nay **không** dùng 405.
> Nợ: `--vm` vẫn **một luồng**; lệnh ngoài **chưa qua `cap`** (cả hai đường); `vis test` vẫn `--no-cache` cho tệp tạm.
> **ST-90 mở** (pkg `entry` + `pkg remove`).

APPROVED 2026-11-14 Agent 0

> **ST-90 DONE — pkg `entry` + `pkg remove`.** Commit `bdfd122b` (**Agent 2 tự commit, đã push**). Lock ghi **`entry`** khi cài
> (`--entry` để override khi manifest thiếu — manifest **đã có** `entry` từ ST-70, *đính chính đề bài của tôi*); **lock cũ** thiếu
> `entry` ⇒ **fallback quy ước, không cảnh báo** (tương thích); `entry` sai ⇒ **`VS0701` nêu đúng tệp**. `vis pkg remove` xoá
> mục lock + link `.vis/pkg` + blob **chỉ khi không còn gói nào dùng** (so sha — store content-addressed), **không** sửa `vis.toml`
> (khoá bằng ca), tên lạ ⇒ exit **2**, lock **byte-identical**. **7 ca** (gồm ca hai gói cùng nội dung). Đo: **973 test** · sem
> **447/22** · `--vm` **447 ok/0 SKIP** · host **54/54** · differential **100k/0 lệch** (155,3 s).
> Nợ: lock cũ **không tự nâng cấp**; `entry` chưa kiểm đuôi; `remove` **không** dọn dep mồ côi (đúng "chỉ xoá tên được gọi");
> store không GC; chưa `pkg update`; `--entry` chỉ áp gói đích.
> **ST-91 mở** (LSP completion sau `.`).

APPROVED 2026-11-14 Agent 0

> **ST-91 DONE — LSP completion sau `.` theo kiểu (đo được: 50 → 2).** Commit `a2eb53e5` (**Agent 2 tự commit, đã push**). Trước:
> `p.` trả **50 mục fallback** (không có `x`/`ten`). Sau: **2 mục** `ten: str`, `x: int` (kind 5); có `impl Show for P` ⇒ thêm `show`
> (kind 2, chữ ký); `s: str` (kiểu biết, chưa có bảng thành viên) ⇒ **rỗng**; **không rõ kiểu** ⇒ fallback 50 như cũ. Tách
> `lsp/thanh_vien.rs` (219 dòng) + `goi_y.rs` (239); `p.` dở ⇒ sentinel `__vis` parse **riêng** (không đổi chẩn đoán gốc).
> **5 ca**, 39/39 LSP xanh, **5 lần cùng byte**. *(Đính chính cú pháp: `type P: x: int, ten: str` một dòng + `P(x:…)` **không** hợp lệ —
> agent dùng `type P:` nhiều dòng + `let p: P = {…}`, đúng.)* Đo: **978 test** · sem **447/22** · `--vm` **447 ok/0 SKIP** · host **54/54** ·
> differential **100k/0 lệch** (157,6 s).
> Nợ: chỉ receiver **đơn giản** (`tên.`), chưa `a.b.`/`f().`; chưa generic/`list[P]`; chưa bảng `std.str`; `impl` cho alias/enum chưa gom;
> parse sentinel chưa cache.
> **ST-92 mở** (R14).

APPROVED 2026-11-14 Agent 0

> **ST-93 DONE — `cap[Proc]` (cưỡng chế dưới `--strict-caps`).** Commit `3e1ea655` (**Agent 2 tự commit, đã push**). Trước: strict không
> cap + `/bin/echo` **vẫn chạy** (lỗ hổng). Sau: **`VS0604: [quyền] cần khai `cap[Proc]` cho /bin/echo`**, exit 2; có `cap[Proc]` ⇒
> chạy; **`cap[Dir]` không** thoả `cap[Proc]` (**nêu đúng Proc**); không strict ⇒ **tương thích** (host 54 ca xanh). `--dry` in
> `p: cap[Proc]`. Runner kiểm **trước spawn**, **cùng host** cho interp + VM. **6 ca** (5/6 đỏ trước). Đo: **984 test** · sem **447/22**
> · `--vm` **447 ok/0 SKIP** · host **54/54** · parity **398/398+49** · differential **100k/0 lệch** (154,7 s).
> **Phát hiện**: `.vish` có `fn main` từng **nuốt** pipeline cấp tệp; nay pipeline là **entry** (thay đổi **có chủ đích**, đã ghi nợ).
> **Nhưng** thân `main` trong `.vish` **không chạy** mà **không** có chẩn đoán ⇒ im lặng ⇒ **mở ST-93b** để báo lỗi rõ.

APPROVED 2026-11-14 Agent 0

> **ST-93b DONE — im lặng mới đã đóng.** Commit `f79589ee` + `fc8717b4` (**Agent 2 tự commit, đã push**). `.vish` `fn main` có **câu
> lệnh thật** ⇒ **`VS0201` tại câu lệnh đầu** ("`fn main` chỉ dùng để khai `cap`…"), exit 2, **không chạy gì**; thân `pass`/rỗng ⇒
> **hợp lệ** (pipeline vẫn là entry). Chặn **cả** `vis check` và `vis run` (cùng HIR) ⇒ **hai đường** không lệch. **5 ca**; 4 chỗ
> trong `cap_proc.rs` đổi `let` → `pass` **kèm lý do**. Đo: **989 test** · sem **447/22** · `--vm` **447 ok/0 SKIP** · host **54/54** ·
> differential **100k/0 lệch** (154,3 s).
> Nợ: `pass` nhận diện **theo tên**; `.vish` `main` + pipeline vẫn **bỏ main** khi thân rỗng (ST-93, nay chỉ khi không có câu lệnh thật).
> **ST-94 mở** (kênh: capacity 0 + `close`/drain).

APPROVED 2026-11-14 Agent 0

> **ST-94 DONE — nhánh (A): `chan(0)` bị **từ chối rõ ràng**, không giả vờ.** Commit `fe6d459b` (**Agent 2 tự commit, đã push**).
> Lý do đúng: scheduler **một luồng run-to-completion** (VD-90) **không thể** rendezvous ⇒ chặn **ngay khi tạo** (literal, biến `0`,
> trong hàm), thông điệp nêu "cần scheduler thật — chưa hỗ trợ; dùng capacity ≥ 1", **0,01 s** (trước deadlock 0,30 s), **không treo**.
> `recv` sau `close` ⇒ `none`; `send` sau `close` ⇒ `VS0504`; `recv` rỗng không task ⇒ `VS0505`; **drain**: 100 `spawn` không `await`
> ⇒ đủ 100 dòng **đúng thứ tự spawn**; task lỗi không await ⇒ `VS0506`. **10 ca** × 2 đường so **byte**. **ĐÍNH CHÍNH ĐỀ**: tôi đòi
> exit **2** (static `VS0412`) — đo được rằng nó đẩy 1 ca sang **lỗi biên dịch** ⇒ parity **397+50**, phá ràng buộc; agent chọn
> **exit 1 runtime** để giữ **398+49** và 447 sem. **Chấp nhận**, ghi nợ. 1 fixture `close-roi-recv-trong.vis` đổi `chan(1)` **kèm lý do**
> (output không đổi). Đo: **999 test** · sem **447/22** · `--vm` **447 ok/0 SKIP** · host **54/54** · differential **100k/0 lệch** (155,3 s).
> Nợ: **không** rendezvous thật; exit 1 (không 2) cho `chan(0)`; drain không có wait-set; `chan(0)` bị chặn cả khi chỉ `close`.
> **ST-95 mở** (R15).

APPROVED 2026-11-14 Agent 0

> **ST-96 DONE — bất biến sổ mã đã khoá (đo được: A=33, B=32, `A\B={VS0001}`, `B\A=∅`).** Commit `d442721b` (**Agent 2 tự commit,
> đã push**). `A\B` chỉ là **literal ví dụ định dạng** trong unit test `vis-diag` (không phải mã chẩn đoán) ⇒ đổi sang
> `format!("VS{:04}", 1)` **kèm lý do**; **không** thêm/đổi mã (`all_codes()` vẫn **32**, `--list` 32). `DU_PHONG = &[]` (rỗng) có
> tài liệu lý do; **4 ca** gồm (b) khẳng định `B\A == DU_PHONG` ⇒ thêm mã vào sổ mà **không dùng** sẽ **làm đỏ**. Đo: **1003 test** ·
> sem **447/22** · `--vm` **447 ok/0 SKIP** · host **54/54** · differential **100k/0 lệch** (153,2 s).
> Nợ: phép quét **cố ý** chỉ phủ `crates/*/src/**/*.rs`; `DU_PHONG` rỗng; chưa đối chiếu tài liệu/spec (ngoài quyền).
> **ST-97 mở** (`--grant` cho `cap[Proc]`).

APPROVED 2026-11-14 Agent 0

> **ST-97 DONE — `cap[Proc]` có allowlist qua `--grant`.** Commit `0397c5c6` (**Agent 2 tự commit, đã push**). Dạng: **lặp**
> `--grant p=<lệnh>` (không dùng `:` vì lệnh có thể chứa `:`); nhiều grant cùng tên ⇒ **allowlist**; không grant ⇒ **không** giới hạn
> (tương thích 6 ca `cap_proc` + host 54). Khớp khi **basename bằng** **hoặc** **canonicalize bằng**; không khớp ⇒
> **`VS0604: [quyền] lệnh \`X\` không nằm trong allowlist của \`cap[Proc]\`** exit 2 (in cả danh sách đã cấp); cưỡng chế ở `Runner`
> **dùng chung** interp + VM; grant lệnh cho `cap[Dir]` ⇒ **lỗi kiểu** exit 2. **6 ca** × 2 đường so byte. Đo: **1009 test** · sem
> **447/22** · `--vm` **447 ok/0 SKIP** · host **54/54** · parity **398+49** · differential **100k/0 lệch** (153,3 s).
> Nợ: basename khớp binary **cùng tên ở path khác** (rộng có chủ đích); **không** giới hạn **đối số**; grant áp **cả khi không**
> `--strict-caps` (theo ý người dùng); `cap[Proc]` không grant vẫn unrestricted.
> **ST-98 mở** (soak 30 phút, tuần tự).

APPROVED 2026-11-14 Agent 0

> **ST-98 DONE — soak 8 vòng tuần tự, 8/8 OK.** Commit `3840d463` (**Agent 2 tự commit, đã push**). Bảng: **gate 224/211 s**,
> **diff 160/158 s** (100k, 0 lệch), **host 9/7 s**, **cache 24/25 s**, tổng **818 s**, load trước 7,48 → sau 4,31 (máy bận, kết quả
> không đổi). **0 flaky sản phẩm**. Một lần **`T-INFRA`** được ghi **trung thực** (`cargo: command not found` do PATH của background
> job ⇒ exit 127 **ở `cache-1`**), log giữ riêng `st98-soak-attempt1-infra.log`, sửa script thêm PATH rồi **chạy lại toàn bộ 8 vòng**
> — **không** retry để che đỏ. `bin/vis-soak` 61 dòng (SPDX, `set -u`, tuần tự, dừng khi đỏ, `--dry-run` 8 tên) + 1 test tĩnh.
> Đo: **1010 test** · sem **447/22** · `--vm` **447 ok/0 SKIP** · host **54/54 ×2**.
> Nợ: soak **818 s < 30 phút** (đúng 8 vòng theo đề — muốn 30 phút phải thêm vòng/seed, không tự nới); script giả định cargo ở
> `~/.cargo/bin`; test chỉ kiểm tĩnh; chưa soak nền khác.
> **ST-99 mở** (R16 + ROADMAP bàn giao).

APPROVED 2026-11-14 Agent 0

> **ST-100 DONE — LSP hoàn thiện nhỏ (đo được).** Commit `45334bfc` (**Agent 2 tự commit, đã push**). `hover` trường: `p.x` ⇒ `**x**: int`
> + "trường của `P`" (dùng **cùng** bảng `thanh_vien::gom`); `didClose` xoá cache **cả hai** (item module + văn bản tệp phụ), URI khác
> vẫn đúng; cache **item module phụ** theo `use` + `mtime/size`, **không** cache tệp chính ⇒ lần 2 **0,466 ms** (từ **4,862 ms**,
> đo in-process min 3 vòng). **5 ca** (4 tích hợp + 1 unit perf), 43/43 LSP xanh, `src/lsp/*` ≤300. Đo: **1015 test** · sem **447/22**
> · `--vm` **447 ok/0 SKIP** · host **54/54** · differential **100k/0 lệch** (152,2 s).
> Nợ: cache **global/thread-local**; key `use` dùng **văn bản** (tệp dùng `pkg` **luôn** nạp lại); `didClose` xoá **toàn bộ** cache
> (an toàn, hơi rộng); hover khai báo chỉ khi tên **duy nhất**; chưa `f().x`; chưa `didSave`.
> **ST-101 mở** (`--json` cho `check`/`test`).

APPROVED 2026-11-14 Agent 0

> **ST-101 DONE — đầu ra máy đọc (`--json`).** Commit `e2f25945` (**Agent 2 tự commit, đã push**). `check --json`: `{ok, files[{file,
> diagnostics[{code,line,col,message}]}]}`, `line/col` **1-based**, thứ tự tệp = thứ tự đối số, stderr rỗng, exit **giữ nguyên**
> 0/1/2; `test --json`: `{total,ok,failed,cases[{name,ok,message}]}`, message **một dòng ≤200**, đúng thứ tự; **không cờ** ⇒ TAP
> **byte y hệt**. **6 ca** (5/6 đỏ trước vì `--json` là cờ lạ). Không serde (JSON viết tay). Đo: **1021 test** · sem **447/22** ·
> `--vm` **447 ok/0 SKIP** · host **54/54** · differential **100k/0 lệch** (152,7 s).
> **Nợ nghiêm trọng đã tự khai**: **chưa escape** chuỗi JSON ⇒ **mở ST-101b** để đóng (JSON phải **parse được** với message có `"`,
> `\`, newline). Các nợ khác: chẩn đoán tệp phụ nằm dưới `file` tệp chính; cắt theo **ký tự**; thiếu `severity`.

APPROVED 2026-11-14 Agent 0

> **ST-101b DONE — lỗ hổng JSON đã đóng (đo được trước/sau).** Commit `494788bc` (**Agent 2 tự commit, đã push**). **Trước**: hàm escape
> của `test --json` **không** escape newline/tab/control ⇒ unit test **panic** `JSON không hợp lệ ở byte 13: ký tự điều khiển thô trong
> string`. **Sau**: một nguồn [`json_esc.rs`](viscript/crates/vis-cli/src/json_esc.rs) (66 dòng) escape `"` `\` `\b` `\f` `\n` `\r` `\t`
> + `<0x20` ⇒ `\u00XX`, **giữ UTF-8 thô**; áp cho **mọi** chuỗi của **cả hai** hợp đồng; **không** đổi khoá/thứ tự/TAP. Roundtrip qua
> **`json.decode` của ViScript** (Việt + emoji **khôi phục đúng**, không thành `\u`) + integration `python3 json.load`. **6 test**.
> Đo: **1027 test** · sem **447/22** · `--vm` **447 ok/0 SKIP** · host **54/54** · differential **100k/0 lệch** (158,9 s).
> Nợ: integration dùng **python3**; emoji không làm tên ca CLI (lexer); chưa fuzz chuỗi JSON; chẩn đoán tệp phụ vẫn dưới `file` tệp chính.
> **ST-102 mở** (`pkg update` + `list --json`).

APPROVED 2026-11-14 Agent 0

> **ST-102 DONE — `pkg update` + `list --json`.** Commit `c20c90cf` (**Agent 2 tự commit, đã push**). `pkg update [<tên>] [--registry <dir>]`
> **không mạng, không resolver**: nguồn cùng version **không đổi** ⇒ in `không đổi` + lock **byte-identical**; nguồn **đổi** ⇒ ghi blob mới,
> dựng lại `.vis/pkg`, cập nhật `entry`/`sha256`/`files`, in `đã cập nhật`, lần 2 **idempotent**; `update <tên>` chỉ đụng gói đó;
> tên lạ ⇒ exit **2**; **không** sửa `vis.toml`, **không** xoá blob còn dùng. `pkg list --json` = `{packages[{name,version,entry,sha256}]}`
> sắp **tên byte-order**, dùng `json_esc`; **không cờ** giữ nguyên. **6 ca** (5/6 đỏ trước). Đo: **1033 test** · sem **447/22** ·
> `--vm` **447 ok/0 SKIP** · host **54/54** · differential **100k/0 lệch** (158,8 s).
> Nợ: **không** đổi version (đúng yêu cầu), `update` **không** cập nhật phụ thuộc; blob cũ **không GC**; lock cũ thiếu `entry` ⇒ `""`;
> registry cục bộ, chưa mạng/ký số.
> **ST-103 mở** (R17).

APPROVED 2026-11-14 Agent 0

> **ST-104 DONE — HTTP client đúng 3 thứ cần thiết.** Commit `2b55b102` (**Agent 2 tự commit, đã push**). **Chunked**: 3 chunk ⇒ `abcdef`,
> chunk mở rộng `;ext` **bỏ** ext, dừng ở chunk `0`, **không** đọc `Content-Length` (trước: body **rỗng**). **Redirect**: theo tối đa
> **5**, tuyệt đối **và** tương đối, giữ GET, thiếu `Location` ⇒ mã gốc, lần 6 ⇒ lỗi rõ "quá 5 lần chuyển hướng", **vẫn** loopback.
> **CL/EOF/timeout**: CL đọc đúng N rồi trả (**không** chờ đóng); mỗi lần đọc cap **2000 ms** ⇒ server im lặng lỗi **2,01 s** (trước chờ
> tới khi đóng **3,3 s**). **7 ca** (6/7 đỏ trước) × 2 đường so byte; 16 ca http tổng. Đo: **1040 test** · sem **447/22** · `--vm`
> **447 ok/0 SKIP** · host **54/54** · differential **100k/0 lệch** (157,5 s). Gate lần 1 **FAIL `fmt`** ⇒ `cargo fmt --all` rồi **PASS**
> (không phải retry để che lỗi).
> Nợ: chưa TLS/HTTP2/keep-alive; redirect **chỉ GET**, không cookie/auth; timeout **mỗi lần đọc** (tổng có thể lâu hơn); chunk **trailer**
> bỏ qua; `Location` chưa resolve `..`/query-only; body cap 8 MiB.
> **ST-105 mở** (`vis test --timeout`).

APPROVED 2026-11-14 Agent 0

> **ST-105 DONE — `vis test --timeout` (mặc định không đổi).** Commit `c3afa7b1` (**Agent 2 tự commit, đã push**). Trước: ca treo
> **treo vô hạn**; cờ **chưa có** ⇒ exit 2. Sau: `--timeout 300` ⇒ ca treo bị **giết sau ~300 ms**, `not ok N - <tên>` +
> `# <tên>: quá hạn 300 ms`, **exit 1**, **tiếp tục** ca kế (ca nhanh `ok 2` vẫn chạy); tổng 2 ca **0,89 s**; **không rò** tiến trình
> (kiểm `ps -axo pid=,ppid=` **rỗng** sau khi kết thúc); `--json` có `message` đúng; `0`/`abc` ⇒ exit 2. Cơ chế: **poll `try_wait`
> 10 ms** + `kill` + `wait_with_output` (reap), **không** crate mới, **không** `unsafe`. Chỉ áp `test fn` (fixture `.out/.err` giữ nguyên).
> **5 ca** (4/5 đỏ trước). Đo: **1045 test** · sem **447/22** · `--vm` **447 ok/0 SKIP** · host **54/54** · differential **100k/0 lệch** (154,0 s).
> Nợ: chỉ `test fn`; kill trễ ~10–20 ms; **không** kill cây tiến trình cháu; pipe có thể nghẽn nếu ca in quá buffer (chưa drain song song);
> `--timeout` **tính cả** thời gian biên dịch của `vis run`.
> **ST-106 mở** (tách `lower/cmd.rs`, hạ R1 max).

APPROVED 2026-11-14 Agent 0

> **ST-106 DONE — refactor thuần, R1 max giảm.** Commit `5d7b1353` (**Agent 2 tự commit, đã push**). `lower/cmd.rs` **1491 → 1060**,
> tách `cmd/doc.rs` (**235**) + `cmd/hop.rs` (**213**); chuyển `ha_closure` (92), `run_shell_tu` (66), `kieu_stage` (62), `ngoai_call` (49),
> `boc_render` (24) + `ha_dong`/`ha_pipeline`/`ten_lenh`; chỉ nới `pub(super)`, **không** đổi logic/api. **R1 max 1491 → 1482**
> (`lower.rs`, tệp khác) — `cmd.rs` giảm **431**. Bằng chứng **trùng khớp**: checksum 1045 dòng test **trước = sau** (`5c194bb8…`),
> differential 100k sha256 **trước = sau** (`62bdd699…`), sem **447/22**, parity **398+49**, host **54/54**, `VBC1` **53**, **32 mã**.
> clippy/fmt sạch, **không** `#[allow]`. Nợ: `lower.rs` **1482** vẫn là max ⇒ tách ở task sau; `ha_stage` 782 còn trong `cmd.rs`.
> **ST-107 mở** (R18).

APPROVED 2026-11-14 Agent 0

> **ST-108 DONE — R1 max đạt ≤1.200.** Commit `0cd6d471` (**Agent 2 tự commit, đã push**). `lower.rs` **1482 → 993**, tách
> [`lower/khai_bao.rs`](viscript/crates/vis-types/src/lower/khai_bao.rs) (**497**) chứa 6 bộ phận: `predeclare_item` (74), `lower_enum` (67),
> `lower_params` (65), `lower_fn` (57), `lower_rec` (45), `lower_impl`/`lower_trait`/`lower_alias`/`lower_const`/`build_item`/`lower_extern`.
> **R1 max 1482 → 1060** (`cmd.rs`) — **≤1.200**. Bằng chứng **trùng khớp**: checksum 1045 test `5c194bb8…`, sha256 diff `c40b9a84…`
> **trước = sau**; sem/parity/diag/host không đổi; clippy+fmt sạch, **không** `#[allow]`.
> Nợ: `cmd.rs` **1060** vẫn max; `ha_stage` **782** chưa tách; `lower.rs` còn 993; kiểm chứng bằng checksum (không snapshot AST).
> **ST-109 mở** (HTTP Location/HEAD).

APPROVED 2026-11-14 Agent 0

> **ST-109 DONE — `Location` đầy đủ + `http.head`.** Commit `28c441cb` (**Agent 2 tự commit, đã push**). `Location`: `/a/b/../c?x=1`
> ⇒ **`/a/c?x=1`**; `?y=2` ⇒ **giữ path cũ** + query mới; `#f` ⇒ **dừng** (cùng URL, tránh vòng); rỗng ⇒ **mã gốc**. `http.head`:
> chỉ đọc header, server `Content-Length: 999999` + 1.000 byte thân giả + giữ kết nối ⇒ **2 lần HEAD < 1,2 s**, không treo; HEAD + 302
> ⇒ **không** theo redirect (đã chọn + ghi tài liệu). `FUNCS` **50 → 52** (test đếm sửa **kèm lý do**); `HOST-BAO-DAM.md` thêm mục
> HTTP (chunk trailer bỏ, redirect chỉ GET, không TLS/keep-alive, timeout đọc 2 s). **5 ca** (5/5 đỏ trước) + 1 unit; 21 ca http.
> Đo: **1051 test** (interp **27**) · sem **447/22** · `--vm` **447 ok/0 SKIP** · host **54/54** · differential **100k/0 lệch** (156,9 s).
> Nợ: HEAD không redirect; một kết nối/request; chưa TLS/HTTP2/keep-alive; `Location` chưa `//host`/userinfo; `FUNCS` 52 cần review khi chốt API.
> **ST-110 mở** (`check --json` severity + tệp phụ).

APPROVED 2026-11-14 Agent 0

> **ST-110 DONE — `check --json` đúng tệp + severity.** Commit `5ae46d04` (**Agent 2 tự commit, đã push**). Trước: mọi chẩn đoán dồn dưới
> **tệp đối số** (sai). Sau: `files` gồm **mọi tệp đã nạp** theo **thứ tự DFS `use`** (chính trước), mỗi chẩn đoán có
> **`severity`** (`vis-diag` đã có `Code.severity`; chỉ thêm hàm ánh xạ — **không** thêm mã, `all_codes()` vẫn **32**); `VS0107` (tab
> trong `.vish`) = **warning**, còn lại **error**; TAP **không đổi** byte. **5 ca** (4/5 đỏ trước). Đo: **1056 test** · sem **447/22**
> · `--vm` **447 ok/0 SKIP** · host **54/54** · differential **100k/0 lệch** (166,3 s).
> **Nợ QUAN TRỌNG (ghi rõ, chưa sửa)**: câu lệnh **cấp tệp** ở tệp chính vẫn có thể bị gắn sang **module** vì span HIR của mỗi tệp đều
> **bắt đầu từ 0** ⇒ cần **offset/file-id cho `Span`** ở task sau. Đây là **hạn chế thật của chẩn đoán**, không chỉ của JSON.
> Nợ khác: `check --json` nhiều đối số có thể lặp module; `files` chưa có metadata `main`/`phụ`; TAP chỉ báo chẩn đoán **đầu**.
> **ST-111 mở** (R19).

APPROVED 2026-11-14 Agent 0

> **ST-112 DONE — chẩn đoán ĐÚNG (tái hiện + chỉ đúng chỗ), quyết định **KHÔNG sửa** là **đúng luật**.** Commit `7c3f5a49`
> (**Agent 2 tự commit, đã push**). Tái hiện: `a.vis` có `use ./b` + `let x: int = "s"` cấp tệp ⇒ JSON gắn chẩn đoán vào
> **`b.vis`** (`2:3`) và TAP in `not ok 1 - a.vis (b.vis:2:3)` — **sai tệp**. Chỉ đúng dòng: `modules.rs:795-820` map `span → rel`
> **chỉ duyệt `hir.items`** (không có file-id) ⇒ item `b` (span 0..34) **đứng trước** thắng item `a`; `dòng:cột` tính lại trên
> **văn bản b**. Gốc: **`Span` không mang tệp**, mỗi tệp bắt đầu `start = 0` ⇒ span **chồng lấn**. Ước lượng **>150 dòng**
> (`Span` + **90** `Span::new` ở 5 crate + **13** chỗ cắt `&text[span]` + `vi_tri`/`interp::pos`/codegen/LSP) ⇒ dừng, viết
> `st112-ke-hoach.md` (**30 dòng**: `Span.file: u16`, thứ tự, rủi ro, ca) + `ROADMAP.md` §C. **0 dòng code**; **không** tuyên bố
> chẩn đoán đa tệp đúng hoàn toàn. Đo: **1056 test**, sem **447/22**, parity **398+49**, host **54/54**, R1 max **1060**.
> **ST-113 mở** (soak cuối ≥10 vòng + R20).

APPROVED 2026-11-14 Agent 0

> **ST-114a DONE — `Span.file` đã có, hành vi **y hệt** (khoá bằng checksum).** Commit `bf11a21f` (**Agent 2 tự commit, đã push**).
> `Span` thêm `pub file: u16`, `new` **giữ file 0**, thêm `new_file`/`with_file`; `lex_file`/`parse_file`/`parse_voi_file`/`dat_file_cst`
> + `LowerOptions.file`/`Lowerer.file`; `modules.rs` cấp **chỉ số tệp theo thứ tự nạp** (chính **0**, module theo `use`), `item_rel`
> giữ nguyên. **Số lời gọi `Span::new` phải đổi: 0** (chỉ thêm 4 chỗ `Span::new_file` nội bộ). Debug xác nhận: top-level `a.vis`
> ⇒ `file:0`, `return` trong `b.vis` ⇒ `file:1`. **Checksum 1056 test `6c1ea8bf…` trước = sau**; **sha256 diff `62bdd699…` trùng**;
> sem **447/22**, parity **398+49**, host **54/54**, R1 max **1060**. Map chẩn đoán **chưa đổi** (đúng hợp đồng — ST-114b).
> **ST-114b mở** (dùng `file` để map chẩn đoán + `vi_tri` theo đúng văn bản).

APPROVED 2026-11-14 Agent 0

> **ST-114b DONE — NỢ CORRECTNESS ST-112 ĐÃ ĐÓNG.** Commit `614d55e0` (**Agent 2 tự commit, đã push**). `Nap.file_rel` cấp **cùng**
> `file_id` (chính 0, module theo `use`); tyck `rel = file_rel[d.span.file]`; **fallback heuristic span cũ chỉ khi** file-id ngoài bảng
> (**có `eprintln` cảnh báo**); `vi_tri` dùng **`f.text` của rel đúng** ⇒ **TAP và `--json`** đều đúng dòng:cột. **4 kịch bản**:
> (a) a cấp tệp `b.vis 2:3` ❌ → **`a.vis 2:14`** ✅; (d) cả hai dồn `b.vis` ❌ → **`a.vis 2:14` + `b.vis 5:5`** ✅; (b)(c) vẫn đúng;
> TAP không còn `(b.vis:` sai. **6 ca** (5/6 **đỏ** trước) gồm ca `b.vis` **dài ≥100 dòng**. Đo: **1062 test** · sem **447/22** ·
> `--vm` **447 ok/0 SKIP** · host **54/54** · parity **398+49** · diff **100k/0 lệch** (163,5 s) · `VBC1` **53** **không đổi**.
> Hạn chế còn lại (ghi rõ): span **tổng hợp** do lower sinh vẫn `file=0` (chưa thấy ca thực tế); **LSP chưa** dùng file-id để publish
> chẩn đoán tệp phụ; **không** tuyên bố "mọi chẩn đoán đa tệp đúng 100%" cho node tổng hợp.
> **ST-115 mở** (R21).

APPROVED 2026-11-14 Agent 0

> **ST-116 DONE — tách `parse/mod.rs`.** Commit `167d7cc4` (**Agent 2 tự commit, đã push**). `parse/mod.rs` **1452 → 1069**, thêm
> `parse/khai_bao.rs` (**391**) chứa khai báo A.1 (fn/cmd/test/extern · type/enum/alias/newtype + field/variant/derives · trait/impl ·
> const · params/generics · grant_list/record_type); 19 method `pub(super)`, **API công khai không đổi**. **Checksum 1062 test
> `c996f294…` trước = sau**; sha256 diff **`62bdd699…` trùng**; sem/parity/host không đổi. **R1 max 1452 → 1254** (`main.rs`).
> **Chưa đạt ≤1.200** như VD-118 đặt ⇒ mở **ST-116b** (tách `main.rs` 1254 → ≤1.000) để đóng nốt; đây là **chênh lệch có ghi nhận**,
> không tuyên bố đạt.
> **ST-117 mở** (LSP chẩn đoán đa tệp).

APPROVED 2026-11-14 Agent 0

> **ST-117 DONE — LSP đa tệp ĐÚNG, không phải sửa.** Commit `5183b81a` (**Agent 2 tự commit, đã push**). Đo 4 kịch bản
> `publishDiagnostics`: (a) mở `a.vis` cả hai lỗi ⇒ **1** chẩn đoán **của a** (`1:13`), không lẫn b, **không rỗng**; (b) mở `b.vis`
> (module) ⇒ **1** chẩn đoán **của b** (`3:4`); (c) a sạch ⇒ `[]`, mở b ⇒ có lỗi; (d) `didChange` a (b còn lỗi) ⇒ lần 2 **1** lỗi
> **của a**. ⇒ hành vi **đã đúng từ ST-114b** ⇒ **không** sửa mã LSP, chỉ **4 ca** mới. Đo: **1066 test** · sem **447/22** · `--vm`
> **447 ok/0 SKIP** · host **54/54** · parity **398+49** · diff **100k/0 lệch** (161,3 s); `src/lsp/*` max **293**.
> Nợ giữ nguyên: LSP **không** publish chẩn đoán **tệp phụ** cho tài liệu chính (đúng thiết kế một-tài-liệu, nhưng là hạn chế);
> span tổng hợp `file=0`; bytecode `Span`.
> **ST-116b mở** (tách `main.rs` 1254 → ≤1.000, đóng mục tiêu R1 của vòng 22).

APPROVED 2026-11-14 Agent 0

> **ST-116b DONE — `main.rs` 1254 → 591; R1 max toàn cục 1060 ⇒ **đạt mục tiêu ≤1.200 của VD-118**.** Commit `e34a8984` +
> `63d27ba4` (**Agent 2 tự commit, đã push**). Tách `cli/check.rs` (**335**), `cli/fmt.rs` (**252**), `cli/run.rs` (**94**), `cli/mod.rs` (6);
> `run_run` **ở lại** `main.rs` để giữ ca `cache_fastpath` (đọc `cache::khoa_multi` trong `main.rs`) — **không** đổi kỳ vọng ca.
> Bằng chứng **trùng**: checksum **1066 test** `b88d160f…`, sha256 diff **`62bdd699…`**, **chuỗi usage/exit y hệt** (`vis` không tham số
> exit 2; `run --la` exit 2; `check/fmt --help` exit 2). clippy/fmt sạch, **không** `#[allow]`. 측: sem **447/22**, `--vm` **447 ok/0 SKIP**,
> parity **398+49**, host **54/54**.
> **Ghi nhận thẳng**: mục tiêu **≤1.000 toàn cục** tôi nêu thêm **chưa** đạt (`lower/cmd.rs` **1060**, `parse/cmd.rs` **1007**) — **không**
> tuyên bố đạt; muốn đạt phải có task riêng (ghi nợ). Mục tiêu **của VD-118 (≤1.200)** thì **đã đạt**.
> **ST-118 mở** (soak cuối 10 vòng + R22 + ROADMAP chốt — task cuối của vòng 22).

APPROVED 2026-11-14 Agent 0

> **ST-118 DONE — VÒNG 22 CHỐT (vòng kết thúc đợt chạy liên tục).** Commit `326cdfa0` (**Agent 2 tự commit, đã push**).
> **2026-11-16 · mở lại đợt 23**: hàng ST-119..ST-133 ở trên thay cho khối này. Căn cứ: `viscript/agent/archive/journal-pre-v040/quyet-dinh-a0-2026-11-16.md` (VD-119..VD-131). Khối dưới là **lịch sử đợt 1..22**, giữ nguyên.
>
> **Soak cuối 10/10 OK = 1297 s** (gate 256/253/258, diff 155/155/156 — **0 lệch** cả 3 lần, host 8/8, cache 25/23), load 2,04 → 1,83;
> chạy **một lần**, **không** retry. `report-check` **PASS (22 báo cáo, 134 hàng QUEUE phủ đủ)**. `ROADMAP.md` chốt **64 dòng** với
> **bảng 8 hạng mục × mức thật × điều kiện đóng 100%** + nơi đọc + lệnh chạy.
> **Số cuối**: **1066 test** · sem **447/22 nhóm** · `vis test` **447 ok** · `--vm` **447 ok + 0 SKIP** · parity **398/398 + 49 lỗi biên dịch** ·
> host suite **54 ca ×2 đường / 0 lệch** · differential **100k / 0 lệch** (155 s, seed `20261115`) · cache **520 tệp × 4 lần / 0 lệch** ·
> fuzz parser **2.506.504 ca / 0 vi phạm** · **R1 max 1060** (`lower/cmd.rs`) · `FUNCS` **52** · `all_codes` **32** · `VBC1` **53** ·
> allowlist **6** · targets **18/18 `check`** · fmt `--check` **497 tệp**.
> **Nợ còn lại: 104 mục** (R22 §5). **VS2 CHƯA ĐÓNG** — đúng mức đã đo: Landlock **chỉ probe** (chưa cưỡng chế bằng kernel), 4 nền
> **chỉ `check`** (chưa có runner), net **loopback/không TLS**, async **một luồng** (không song song), kênh **không rendezvous**,
> quyền **chưa siết mặc định**, span **tổng hợp** `file=0`, LSP **chưa** publish chẩn đoán tệp phụ.
> **ĐỢT CHẠY LIÊN TỤC KẾT THÚC Ở ĐÂY** (22 vòng, 134 hàng QUEUE, 22 báo cáo). Việc tiếp theo **cần quyết định của người** (duyệt crate
> `landlock` hoặc `unsafe` đóng + runner Linux; duyệt thay đổi scheduler cho async song song; duyệt migration "siết mặc định caps").


> **[IMPLEMENTING — Agent 1/Agent 2 mở VD] VD "Windows test-suite portability"** — **ĐÃ MỞ** 2026-11-19 theo bệnh uỷ thác Agent 0 (người dùng dispatch): kế hoạch trong `agent/PLAN-WINDOWS-L0-PORTABILITY-2026-11-19.md`.
> Trạng thái hiện hành: `test-child` binary emulating `/bin/{echo,cat,grep,printf,true,false,sleep,sh}` đăng ký tại `crates/vis-cli/src/bin/test_child.rs` (cần `cargo build --bin test-child` cho tests).
> Phạm vi đợt này: (A) **POSIX binaries** — thay `/bin/*` bằng `CARGO_BIN_EXE_test-child`; (B) **fixture path/CRLF/JSON** — normalize trong test harness; (D) **WinSock** — assert hai giá trị; (E) **mode file** — expect `-` hoặc `100600`; (C) **WSL stub** — direct-call thay `bash -lc`.
> Base khởi điểm: `349af67`. Mục tiêu: phân loại từng test theo hợp đồng thật; port fixture/assertion khi ngữ nghĩa chung; tương đương native cho API đa nền tảng; chỉ `#[cfg(unix)]` cho Unix-integration thật.
> Kết quả phải đạt: mọi API process đa nền tảng có coverage Windows (stdout/stderr/exit code≠0; capability grant/deny); **không giảm tổng số test process chỉ để xanh**; không "full Windows gate PASS" cho tới khi gate xanh thật (Tier 1 sau VD + required CI/Windows nightly).
> Chính sách A0 đầy đủ: `agent/DECISIONS.md` block "Quyết định Agent 0 · 2026-11-19 (Windows @ 349af67)" · baseline: `agent/WINDOWS-BASELINE-2026-11-19.md` (25 suite đỏ / 101 ca / 90 tên duy nhất). **Đã merge `8355022`**; xem `agent/SNAPSHOT-2026-11-20.md`.
> **Trạng thái** _(2026-11-20, cập nhật bởi Agent 1 @ `c181266`)_: **verify XANH ĐẦY ĐỦ** — nhóm A **đã migrate** (ngoai_vm, pipeline, vish_main, extern, grant_proc_canon, cap_proc, cap_proc_grant; `test-child` đăng ký + POSIX-emulate); nhóm B/C/D/E **đã fix** (`6ccf367` json_ra + fmt/clippy; `f9430b9` CRLF/path/JSON + PATHEXT + Git Bash; `c7cef71` net_server forward-slash). Linux workspace **150 khối `test result: ok`**; gate `--fast` **PASS** ×3; nightly **PASS** @ `4cae3e3` (fuzz 19,073,168 ca; fmt 475; L0 OK); report-check PASS; fresh-clone gate **PASS** @ `5f4edd7`. **Windows verify**: @ `c7cef71`/`2d93101`/`5f4edd7` đều **exit 0** (150 khối ok, 0 đỏ) — log `/c/Users/ps/vislogs/2d93101-ws.out`. **pha1-rc1 nguyên vẹn; Windows Tier 2 giữ nguyên; không tuyên bố Tier 1; pha 2 chưa mở**. VD-winport ở trạng thái **DONE kỹ thuật; đóng/mở lại do A0 quyết**. Không đổi sản phẩm/API/opcode/VBC1; `pha1-rc1` nguyên vẹn.
> MVP scope: port tests spawn-expensive nhóm A-E; không đổi sản phẩm/API/opcode/VBC1; `pha1-rc1` nguyên vẹn. Không được đổi các test ngụ ý check chính xác semantics POSIX (vd `chunk-final` có thể fail khác); các ca WINSock acceptance tuỳ thuộc OS semantics.
> **[HOÀN TẤT 2026-11-19] SQLite Gate 2** — merged `main` `8355022` (nhánh `rev/vd-sqlite-mvp-2026-11-19` ff-merge); A0 narrow freeze `+9 std.sqlite.*` trong `agent/API-STD.txt` (CODES/VBC1 không đổi); review chéo cuối SHIP; gate PASS @ HEAD `c333e8f` (workspace 149/149; sem 525/525 ×2; smoke **22/22** ×2; sqlite 15/15). Gate 2 KHÔNG mở rộng phạm vi sang Windows/macOS; `pha1-rc1` không di chuyển. Xem `agent/SNAPSHOT-2026-11-20.md` để đọc đầy đủ trạng thái.

## Đợt ổn định sau pha 1 — VD-242 (A0, 2026-11-20)

| ST | Việc | Dep | Ước | Trạng thái | Ghi chú |
|---|---|---|---|---|---|
| ST-236 | **(Agent 1) Ghi VD-242 + R31 + phân quyền bốn luồng** (VD-242) | — | 0,5 h | **DONE** | VD-242 ghi vào `DECISIONS.md`; các hàng ST-237..ST-244 thêm mới; `reports/R31-viscript.{md,json}` lấy baseline `31b51d2` (không tự tham chiếu hash); STATE/SNAPSHOT/ROADMAP trỏ VD-242; commit ghi sổ là commit chứa hàng này (xem lịch sử); không tag `0.x` lúc này. |
| ST-237 | **(Agent 2, luồng D) Ma trận 28 phát hiện** (VD-242 §4) | ST-236 | 1 h | **DONE** | **28/28 reproduced** bằng binary baseline `31b51d2`; báo cáo `JOURNAL/st237-ma-tran-28.md` (commit worktree `e21fc0d`, cherry-pick `7c4debb`). **#176 bác bỏ có bằng chứng** (`SPEC-lex.md:44`) ⇒ SO-NO `không làm có lý do`; **#165** bác hướng “sửa VM trả 2” theo VD-242 §3.1; **#161 VM đúng, interp sai**; #162/#165/#170/#176/#181/#182/#183 là chương trình/file **không hợp lệ** ⇒ kết quả đúng là từ chối. **#172 chờ A0** — `BLOCKED.md` §12 (bão hoà hay `VS0507`). Không sửa mã sản phẩm. |
| ST-238 | **(Agent 2, luồng A) Đợt A — treo, abort, nuốt lỗi** (VD-242 §4) | ST-237 | 2–3 h | **DONE** | Worktree `vd242-st238-fixA`, commit Agent 2 `93368c3`; **review từng finding trước merge** `JOURNAL/st238-review.md` (9/9 SHIP, commit `744f6e3`); cherry-pick vào `main` @ `0d97758`. Đã sửa parity ca 1,2,3,4,7,8 + #167 `net.read` (cap 8 MiB, kiểm âm/overflow/chuyển kiểu, `Err` thay abort) + #169 `json.encode` non-finite ⇒ lỗi + #181 `--dry`/`--sandbox-check` ⇒ `VS0201` exit 2. Không đổi API/freeze/opcode/`VERSION`/VBC1. **Ghi đúng commit cho từng phép đo (A0 bổ sung)**: tại **`0d97758`** (code tích hợp) — `./vis-dev check` PASS · `./vis-dev gate --fast` PASS · `PATH="$HOME/.cargo/bin:$PATH" ./vis test test/sem` **525/525** · `PATH="$HOME/.cargo/bin:$PATH" ./vis test --vm test/sem` **525/525** · `cargo test` 4 crate xanh · `st238_parity.rs` 6/6 + unit stdlib 2/2; tại **`ef2c04c`** (chỉ cập nhật sổ) — `./vis-dev check` PASS. Không chạy lại chỉ để sửa trình bày. `GLOBAL_SLOT_BASE = 0x8000` là ánh xạ nội bộ VM (không opcode); nếu muốn vào hợp đồng VBC1 cần VD riêng — ghi nhận, không mở. Nợ cũ ST-28 (chương trình không hợp lệ có cả `let` cấp tệp + `fn main` vẫn lệch thông điệp) **không phải hồi quy**, báo riêng. |
| ST-239 | **(Agent 2, luồng B) Đợt B parity + hợp đồng Option/map** (VD-242 §3.2–3.4) | ST-237 | 2–3 h | **DONE** | Worktree `vd242-st239-option`, commit Agent 2 `72f5ea6`; review `JOURNAL/st239-review.md` (5/5 SHIP, commit `43eb181`), tích hợp @ `4631906`. Đã sửa **#161** (interp `?.` base optional gọi `$lift-opt`, không đụng VM), **#162** (base thường ⇒ `VS0412`), **#165** (or-pattern bind không đầy đủ ⇒ từ chối, **không** trả `2`), **#166** (VM `MakeSet` dedup), **#170** (literal trùng tĩnh lỗi / runtime last-wins / JSON last-wins = lựa chọn ViScript / equality so ánh xạ, checker không gọi mã người dùng). `st239_parity.rs` 8/8. Kiểm trên `4631906`: `vis-dev check` PASS · `gate --fast` PASS · `vis test` **525/525** · `--vm` **525/525** (PATH 1.99.1). **Còn thiểu**: field/index assignment lift (VD-242 §3.3) → **ST-248**; fixture `test/sem` không thêm (số cứng `cli/**` ngoài quyền), regression Rust đã phủ; docs `T??` đã sẵn sàng, không cần đổi. |
| ST-240 | **(Agent 2, luồng C) std/CLI hardening đợt 2** (VD-242 §3.5, §4; **VD-243** cho #172) | ST-237 | 2–3 h | **DONE** | Worktree `vd242-st240-stdcli`, commit Agent 2 `b2666d8`; review `JOURNAL/st240-review.md` (`19c8a5a`), tích hợp @ `7fa68ba`. Sửa **15 ca**: #168 sort lỗi không sắp một phần; #173 float giữ dấu; #175 `01` bị từ chối; #174 path gốc; #171 mtime `–`; #177/#178/#179/#180/#182/#183/#184 vị trí/CRLF/warning/fmt/LSP; **#172** `percent` vượt miền ⇒ `VS0507` exit ≠ 0, non-finite chặn tại nơi tạo giá trị, biên hợp lệ giữ nguyên. #169/#170/Option/parity **chỉ kiểm tích hợp**; #176 không sửa. `st240_std_cli.rs` 11/11, LSP framing 4/4. Kiểm trên `7fa68ba`: `check` PASS · `gate --fast` PASS · `vis test` 525/525 · `--vm` 525/525. **Ghi chú A0**: (1) miền `percent` SPEC “0..1” chưa rõ — Agent 2 dùng ranh giới biểu diễn hiện hành, không chốt miền mới; (2) literal `percent` từ chối tại `lit_value`, hai engine cùng **loại** lỗi nhưng mã khác nhau (VD-243 §1 cho phép); (3) #177 VM hạn chế do `Span` bytecode không có file id, CLI chỉ khôi phục khi duy nhất — sửa triệt để cần đổi format (ngoài phạm vi); (4) #171 in `–` theo VD-242, muốn năm 2446 cần sửa `ngoai.rs`. `BLOCKED.md` §12 đóng. |
| ST-241 | **(Agent 2, luồng C/D) SQLite feature tùy chọn + target matrix** (VD-242 §5) | ST-238 | 1 h | **READY** | Feature `sqlite` optional; ma trận core-không-SQLite và host-có-SQLite; giữ `16/18` cấu hình cũ có tên riêng; cap **16 MiB tổng payload tham số**; `query` chỉ đọc, không dùng danh sách từ khoá làm hàng rào duy nhất; build không SQLite ⇒ từ chối rõ, không panic. |
| ST-242 | **(Agent 2, luồng D) Host identity + đính chính macOS** (VD-242 §5) | ST-236 | 0,5 h | **DONE** | Worktree `vd242-st242-host`, commit `0f5c15b` (tích hợp vào `main` ở commit ghi sổ này). `bin/vis-remote`: `require_darwin()` chặn `mac|macos` nếu `uname -s != Darwin` (exit **3**, thông báo nêu host thật) **trước** mò/chạy; tách `local` chạy tại chỗ không kiểm OS; `ps`/`psvn` giữ nguyên; log thêm `baseline_commit_do`/`kernel`/`arch`/`os_arch_thuc`/toolchain/cấu hình/lệnh/`host_check`; `--self-test` 5/5. Kiểm thật: `mac --check` và `macos --check` trên Linux ⇒ **exit 3**; `local --log … -- true` ⇒ exit 0. Thêm addendum macOS vào `SNAPSHOT`/`STATE` (giữ lịch sử, tag `pha1-rc1` nguyên). SO-NO: #147/#152 gắn **blocked do phần cứng nhưng vẫn mở/vẫn đếm**; #176 đã bác ở ST-237 ⇒ sổ **185 = 121 mở / 38 đóng / 23 VS3 / 2 không làm / 1 điều hướng**. `./vis-dev check` **PASS**. Không đụng `crates/`. |
| ST-243 | **(Agent 2, luồng D) Verify Windows commit tích hợp cuối** (VD-242 §5, §8) | ST-238, ST-239, ST-240, ST-241 | 0,5 h | **READY** | Re-verify `cargo test --workspace` trên psvn tại commit cuối; CI informational nếu có runner/quyền, không gọi required; báo cáo giới hạn Tier 2; không đổi expected để hợp thức hoá lỗi. |
| ST-244 | **(Agent 1) Ratify proxy có chọn lọc** (VD-242 §6) | ST-236 | 0,5 h | **DONE** | Báo cáo `agent/BAO-CAO-RATIFICATION-2026-11-20.md`. Kết luận: **không còn ngữ nghĩa chưa có quyết định trong phạm vi phát hành** — VD-241 đã ratify toàn bộ VD-215..240 (gồm proxy VD-221..229; VD-222 theo VD-231; VD-229 bỏ closure-params); VD-223(i) huỷ bởi VD-224; closure-params → #155/VS3; VD-242..246 là quyết định A0 trực tiếp. Không backfill số VD, không xin duyệt lại phần đã chốt. |
| ST-245 | **(Agent 2, luồng D) Ranh giới global-slot/verifier + test hồi quy async** (A0 bổ sung sau ST-238) | ST-238 | 1 h | **DONE** | Worktree `vd242-st245-boundary`, commit Agent 2 `3ceb041`; review `JOURNAL/st245-review.md` (commit `bfa2138`), tích hợp @ `fcc7f29`. **Phát hiện lỗ hổng verifier cũ**: miễn trừ mọi ô `>= 0x8000`; repro `LoadLocal(0xFFFF)` ⇒ trước `Ok(())`/VM đọc `Unit`, sau `Err(GlobalOverflow {slot:0x7FFF, nglobals:0})` — **đã đóng trong task**, báo A0 ngay theo lệnh. Bốn xác nhận đạt: local không va miền global (`nlocals > BASE ⇒ BadArity`); global kiểm bằng số global suy ra từ lệnh ghi; bytecode sai chặn ở cả `verify()`/`load_verified()`, không panic; **ảnh hưởng tương thích ghi rõ** trong `format.rs` (toán hạng `>= 0x8000` có ngữ nghĩa mới; VM cũ trả kết quả sai, không crash; **không** nói “không đổi hợp đồng”). `VerifyError` 14 biến thể (STATE giữ số 13 lịch sử + addendum). Test: `st245_boundary.rs` 9/9, `st245_async_regression.rs` 4/4 (đỏ trên bản cũ bằng cách đảo `slot_cua → f.slot`); phép đo 10k nguyên ở `async_10k.rs`. Kiểm trên `fcc7f29`: `./vis-dev check` PASS · `gate --fast` PASS · `PATH=… cargo test -p vis-bytecode -p vis-vm` xanh (320 ca parity + 2.000 chương trình sinh vẫn qua). Không đụng `vm.rs`/codegen. **A0 nhận có phạm vi (VD-244)**: chỉ bản vá verifier đã chứng minh; **KHÔNG tuyên bố global-slot đã khép**. Blocker tương thích bytecode/cache → **ST-246** (`BLOCKED.md` §13); miền global suy từ lệnh ghi → **ST-247** (§14). |

## VD-244 — Bytecode/cache tương thích + miền global (A0 bổ sung 2026-11-20)

| ST | Việc | Dep | Ước | Trạng thái | Ghi chú |
|---|---|---|---|---|---|
| ST-246 | **(Agent 2) Blocker tương thích bytecode/cache** (VD-244 §2) | ST-245 | 1 h | **DONE** | Worktree `vd244-st246-compat`, commit Agent 2 `8bf73b0`; review `JOURNAL/st246-review.md` (`116d884`), tích hợp @ `9824e66`. `VERSION 1 → 2`, MAGIC/bố cục/opcode/`API-VBC1.txt` giữ nguyên; re-pin khoá cache `61490404… → 66c332a9…` kèm lý do; sentinel `2 → 3` trong 2 test cũ (bắt buộc, không đụng nguồn `verify.rs`). **4 bằng chứng**: runtime mới từ chối artifact v1 (`BadVersion(1)`, patch đúng 2 byte ⇒ decode OK); runtime cũ từ chối artifact v2 **bằng mô phỏng gate** (caveat: không build crate v1 thật); cache v1 ≠ v2 — artifact v1 in sai `999` trên path cache vẫn bị bỏ, VM in `42`, source recompile đúng interp+VM; bytecode v1 không source ⇒ `LoadError::Decode(BadVersion(1))`. Kiểm trên `9824e66`: `check` PASS · `gate --fast` PASS · `cargo test -p vis-bytecode -p vis-cli` xanh · `vis test` 525/525 · `--vm` 525/525. `BLOCKED.md` §13 đóng **có caveat**; §14 (miền global) vẫn mở — không tuyên bố global-slot khép. |
| ST-247 | **(Agent 2) Làm rõ miền global suy từ lệnh ghi** (VD-244 §3) | ST-245 | 1 h | **DONE** | Worktree `vd244-st247-global-domain`, commit `1309610`; review `JOURNAL/st247-review.md` (`49007ee`), tích hợp @ `e6f9809`. 5 câu: (1) ghi chỉ số cao **không** nới miền (`nglobals` đếm số ô có lệnh ghi); (2) `u16` là giới hạn mã hóa, verifier chặn biên; (3) lỗ hổng/chưa khởi tạo bị từ chối; (4) **blocker** — ghi trong nhánh không chạy/hàm chưa gọi ⇒ verifier chấp nhận, runtime trả `Unit` **thành công** (đã so `show == "()"`); (5) khớp runtime chỉ với codegen thật. §14 cập nhật, #188 mở, **VD-245** (runtime trap) duyệt ST-250. 10 test ST-247, `cargo test -p vis-bytecode -p vis-vm` 124/0, `vis test` 525/525 ×2. Không sửa `verify.rs`/`vm.rs`. |

| ST-248 | **(Agent 2) Option coercion — field/index assignment lift** (VD-242 §3.3; khoảng trống từ ST-239) | ST-239 | 0,5–1 h | **READY** | Checker đã chấp nhận `u.f = 5` (`f: int?`) và `m["a"] = 7` (`map[str,int?]`), nhưng **cả hai engine lưu giá trị thô** (`5`/`7`) thay vì `Some(5)`/`Some(7)`. Chèn `$lift-opt` ở `crates/vis-bytecode/src/codegen.rs` / `codegen/than.rs` (`lift_assign_set` hiện chỉ xử lý `HExpr::Var`) + mirror ở `crates/vis-interp/src/interp.rs`. Không đổi kiểu biến theo lần gán, không nâng nhiều lớp, không ratify `Unwrap → Unit`. Test repro trước/sau + cả hai engine. **Không phải ngữ nghĩa mới** — luật đã có trong VD-242 §3.3. Quyền tệp: `codegen.rs`, `codegen/than.rs`, `interp.rs`, test mới; **KHÔNG đụng** `format.rs`/`cache.rs` (ST-246), `verify.rs` (ST-247), `stdlib.rs`/`builtin.rs` (ST-240). |

| ST-249 | **(Agent 1/Agent 2) Bump `cache::PHIEN_BAN` sau ST-248** (nợ **#187**, `BLOCKED.md` §16) | ST-248 | 0,25 h | **DONE** | `cache.rs`: `PHIEN_BAN` += `+st248-field-index-lift-2026-11-20` kèm lý do **ST-249/VD-129**; khoá ghim re-pin `66c332a94f… → ad63ae9807…`; test trong `khoa_cache_ghim` khẳng định khoá cũ **không còn trùng** và nhãn logic hiện diện. Không đổi `VERSION` (ST-246 đã đổi cho ngữ nghĩa bytecode), không đổi opcode/API. Kiểm: `vis-dev check` PASS · `gate --fast` PASS · `cargo test -p vis-cli cache::tests` PASS · `vis test` 525/525 · `--vm` 525/525. `BLOCKED.md` §16 và nợ #187 đóng. Chặn fresh-clone/release verification và tag mới đã gỡ; §14 vẫn mở. |

| ST-250 | **(Agent 2) Global chưa khởi tạo ⇒ runtime trap** (VD-245, nợ **#188**, `BLOCKED.md` §14) | ST-247 | 1–2 h | **DONE** | Worktree `vd245-st250-init-trap`, commit Agent 2 `4285313`; review `JOURNAL/st250-review.md` (`3d734ca`), tích hợp @ `2551a6c`. `globals: Vec<Option<Value>>`, `None` = chưa khởi tạo (sentinel nội bộ, không lọt `Value`/`show`/toán tử/serialization); `doc_global(k, span)` là **một** kiểm tra cho mọi đường đọc; chưa khởi tạo ⇒ trap `VS0501` (mã hiện hành) exit 1; `init_globals()` reset + cấp phát từ lệnh ghi module đã verify, không resize theo chỉ số lúc chạy. 8/8 test VD-245 §4 (ghi nhánh không chạy/hàm chưa gọi, gọi hàm ghi, `Unit` hợp lệ, nhiều global, ngoài miền, `encode→decode→load_verified→execute`, 2 lần chạy). **Lệch phạm vi được chấp nhận**: sửa 2 assertion trong `st247_global_domain.rs` vì tệp cũ assert chính hành vi `Unit` mà VD-245 cấm. **Finding riêng #189** (capture/local `unwrap_or(Unit)`, mức Trung bình/Thấp) → `BLOCKED.md` §17, không tự sửa. Kiểm trên `2551a6c`: `check` PASS · `gate --fast` PASS · `cargo test -p vis-vm` xanh · `vis test` 525/525 ×2. #188/§14 đóng. Còn bump `PHIEN_BAN` lần cuối theo VD-245 §5. |

| ST-251 | **(Agent 1) Bump `PHIEN_BAN` lần cuối theo VD-245 §5** | ST-250 | 0,25 h | **DONE** | `cache.rs`: `PHIEN_BAN` += `+st250-global-init-trap-2026-11-20`; re-pin khoá ghim `ad63ae98… → c4e9236c…`; test `khoa_cache_ghim` khẳng định cả khoá trước ST-248 (`66c332a9…`) lẫn trước ST-250 (`ad63ae98…`) đều **không còn trùng**. Bản phát hành vì thế gồm đủ ngữ nghĩa global mới (VERSION=2 + trap). Kiểm: `check`/`gate --fast`/`cargo test -p vis-cli cache::tests`/`vis test` 525/525 ×2 xanh. Không đổi VERSION/opcode/API. |

| ST-252 | **(Agent 2) Sửa #189 — capture/local ngoài miền trap** (VD-246, nợ **#189**, `BLOCKED.md` §17) | ST-250 | 1–2 h | **DONE** | Worktree `vd246-st252-capture-local`, commit Agent 2 `7a62861`; review `JOURNAL/st252-review.md` (`7fa7569`), tích hợp @ `c4cee9f`. Verifier giữ `ClosureFree` sẵn có (quan hệ hàm đích/số capture/`MakeClosure`), thay khối no-op bằng ghi chú giới hạn; runtime thêm `doc_local`/`doc_captured`/`ghi_local` trap `VS0501` exit 1, bỏ `resize` âm thầm; giữ `Unit` hợp lệ + local hiện hành. 8/8 ca VD-246, mỗi ca ghi rõ verifier từ chối hay runtime trap; `encode→decode→load_verified→execute` ⇒ runtime trap; chứng minh hết thành công giả. R1 `vm.rs` 1491. Finding riêng `pop_frame` → **#190**/`BLOCKED.md` §18. **Quyết định cache**: KHÔNG bump `PHIEN_BAN` — không đổi format/VERSION/codegen, không có chương trình hợp lệ nào đổi kết quả, artifact trong cache byte-identical; căn cứ ghi trong `JOURNAL/st252-review.md` §3 (không mặc định hậu tố ST-250 bao phủ). Kiểm trên `c4cee9f`: `check` PASS · `gate --fast` PASS · `cargo test -p vis-vm -p vis-bytecode` xanh · `vis test` 525/525 ×2. #189/§17 đóng. |

| ST-253 | **(Agent 1) Toolchain chuẩn tái lập (#185)** | — | 0,25 h | **DONE** | `bin/_vis-toolchain.sh` dùng chung: tôn trọng cargo đạt `rust-version` trên PATH, fallback rustup default, không đạt ⇒ exit 78 + thông báo rõ (có “không thấy cargo” để giữ hợp đồng self-test). `bin/vis` và `bin/vis-dev` cùng source helper; `bin/vis` **không sửa PATH ngầm**. Test `test/toolchain/run.sh` 6/6: PATH cũ+không fallback ⇒ 78, PATH cũ+fallback ⇒ đúng cargo, PATH đạt ⇒ tôn trọng. `./vis-dev check` PASS (self-test “thiếu cargo báo rõ (78)”). #185 đóng. |

| ST-254 | **(Agent 2) SQLite feature tùy chọn + ma trận core/SQLite** (nợ **#156**, VD-242 Q6-A) | — | 1 h | **DONE** | Commit sản phẩm `7427de0`, review `605631f`, sổ `d0816d0`. **Lưu ý đính chính**: các commit/worktree ghi nhãn `ST-156` là nhãn nhầm — `ST-156` trong QUEUE là task R27 lịch sử đã DONE; task đúng của nợ #156 là **ST-254**. `default=["sqlite"]`, `sqlite=["dep:rusqlite"]`; core `--no-default-features` **18/18**, sqlite default **16 PASS + 2 EXPECTED_RED** (`libsqlite3-sys` cần C toolchain), 0 FAIL; `agent/TARGET-MATRIX-2026-11-20.md`. Core build từ chối `std.sqlite` rõ (`Result.Err(Error)`, không panic). Kiểm: `check`/`gate --fast`/`cargo check --workspace --no-default-features`/`vis-targets`/`vis test` 525/525 ×2 xanh. #156 đóng. |

| ST-255 | **(Agent 1) Kỳ vọng path trong test #177 theo nền (Windows final)** | ST-248 | 0,25 h | **DONE** | Windows final @ `8fd59d5` đỏ 1/11 ca `st240_std_cli`: chẩn đoán module dùng `rt\b.vis:3:11` (dấu `\` của Windows) trong khi test so literal `rt/b.vis:3:`. Theo VD-186/path contract, chỉ đổi **dấu phân cách trong kỳ vọng** theo `cfg!(windows)`; giữ nguyên tiêu chí “trỏ đúng module + dòng, không gán sang tệp chính”. Không đổi sản phẩm. Local `check`/`gate`/`st240_std_cli` xanh; sẽ đo lại fresh-clone + Windows trên commit mới. |

| ST-257 | **(Agent 1) Tối ưu công cụ cổng: gom spawn vòng tròn fmt + cờ `--skip-gate`** | ST-253 | 0,5 h | **DONE** | Được A0/Agent chủ trì quyết định **đưa vào & đo lại** trước khi chốt nghiệm thu. `do_vong_tron_fmt`: thay ~950 lần spawn `vis fmt` (475 tệp × 2) bằng đúng 2 lệnh `fmt --write`/`fmt --check` trên thư mục tạm (dàn phẳng tên, 0 va chạm); giữ nguyên phép kiểm/ngưỡng/số tệp. `nightly --skip-gate` chỉ bỏ lần chạy lại `gate --fast`, không đổi hành vi mặc định; fuzz + vòng tròn fmt vẫn chạy. Validate: `check`/`gate --fast` PASS; `nightly --skip-gate --fuzz-minutes 0` PASS (vòng tròn fmt 475 tệp, bỏ qua 78 ca lỗi cú pháp); negative test `fmt --check` trả 1 trước / 0 sau `--write` (kiểm không vô hiệu). Không đổi mã sản phẩm/ngữ nghĩa. Sẽ đo lại fresh-clone + Windows trên commit mới. |

| ST-258 | **(Agent 1, session song song) Tối ưu L0 test bằng `cargo-nextest`** | ST-257 | 0,25 h | **DONE** | `run_l0_tests()` dùng `cargo nextest run --workspace` (song song, 1430 ca) + `cargo test --doc` (5 ca) khi có `cargo-nextest`; không có thì fallback y hệt `cargo test --workspace`; `VIS_NO_NEXTEST=1` ép fallback để đối chiếu. Không đổi kỳ vọng/ca/mã thoát. Validate: fallback PASS; nhánh nextest PASS (1430/1430 + 5 doc = 1435, khớp đường cũ); gate **11,61 s** so với ~40 s (~3,4×) sau khi cài `cargo-nextest 0.9.152`. Kèm sửa R3: thêm sidecar `.license` cho `nghiem-thu-2026-11-20-targets.txt` (commit báo cáo trước đó làm gate đỏ). Sẽ đo lại fresh-clone + Windows trên commit mới. |

| ST-259 | **(Agent 1) Khóa hồ sơ nghiệm thu theo VD-247/A0** | ST-258 / VD-247 | 0,5 h | **DONE** | Chỉ đổi tài liệu, **không đo lại, không đổi baseline `dbae70a`**. Sửa 3 cách diễn đạt: (A) Windows nextest — ghi nhận `1416/1417`, ca `tcp_explain_tong` `WSAECONNABORTED` 1/3, **chưa phân lập nguyên nhân**, không gọi là race socket/không phải sản phẩm; mở finding **#191** / `BLOCKED.md` §19; đường gate Windows chuẩn = `cargo test --workspace --no-fail-fast`. (B) `nightly --skip-gate --fuzz-minutes 0` chỉ chứng minh kiểm được chạy, **không phải bằng chứng fuzz mới** — giữ phép đo, sửa nhãn. (C) 525 TAP = source semantics; 8+8 ca crafted-bytecode nằm ở `crates/vis-vm/tests/st{250,252}_*.rs` do gate `cargo test` chạy. Ghi **VD-247**, giữ bảng ratification truy nguyên từng mục; sổ nợ **192 = 96 mở / 70 đóng**. Xuất phiếu đề xuất tag `agent/DE-XUAT-TAG-0x-2026-11-20.md` (chưa tạo/push tag). |

| ST-260 | **(Agent 1) Đánh giá mức sẵn sàng sử dụng thực tế — ưu tiên async/await (VD-248)** | ST-259 / VD-248 | 2 h | **DONE** | A0 thu hồi quyền tag/release (VD-248): giữ `dbae70a` làm checkpoint, **không** tạo/push tag (tag duy nhất `pha1-rc1`), không mở VS3. Không audit toàn repo. Lập `agent/BAO-CAO-SAN-SANG-THUC-TE-2026-11-20.md` với 5 phần: (1) kiến trúc task — luồng/await yield/I/O chặn hay không; (2) vòng đời — fairness/timeout/cancellation/lỗi task/đóng tài nguyên/limit+backpressure; (3) phép đo đại diện — scheduler thuần, nhiều I/O chờ, trộn CPU+I/O, tạo/hủy lặp (thời gian/bộ nhớ/độ trễ/đúng đắn, interp/VM); (4) phân loại khoảng trống (lỗi đúng đắn / thiếu cơ chế / hiệu năng / thiếu bằng chứng); (5) đề xuất thứ tự phát triển. **Không tự chọn GC/worker model/API mới.** Kết quả: `agent/BAO-CAO-SAN-SANG-THUC-TE-2026-11-20.md`. Đo thật (server loopback trễ 200 ms, cả hai engine): I/O 1/5/20 task ⇒ 0,27/1,09/4,11 s (tuyến tính, chặn toàn runtime); CPU+I/O ⇒ `CPU_DONE` và `IO_START` cùng ms; scheduler/repeat 50k task ~0,28 s, RSS 24–29 MB. **Phát hiện mới**: interp đếm vòng lặp, VM đếm từng lệnh, cùng ngưỡng 10^7 ⇒ 2M vòng interp exit 0 / VM trap VS0506→VS0501 — lỗi đúng đắn cần A0 chốt ngữ nghĩa. Giữ tạm dừng tag/release, không mở VS3. |

| ST-261 | **(Agent 1) Hợp đồng tính phí ngân sách chung + phân lập exit 101 (VD-249)** | ST-260 / VD-249 | 1 h | **DONE (design-only)** | Design-only trước, **chưa sửa mã/chưa đổi VERSION/opcode**. Hợp đồng: điểm tính phí ngữ nghĩa + trọng số; ánh xạ interp/bytecode; một ngân sách cho cả lần chạy gồm task, không reset khi spawn/await; kiểm tra trước tác dụng phụ; giới hạn bộ nhớ/độ sâu/I/O là riêng. Phân lập exit 101 (panic hay exit chủ ý?). Test dự kiến: vòng lặp, đệ quy, nhiều task, tác dụng phụ sát biên. Chờ A0 chốt hợp đồng mới hiện thực. Kết quả: `JOURNAL/vd249-budget-draft.md` (7 sự kiện + trọng số, một pool dùng chung, kiểm trước tác dụng phụ, bảng phụ bytecode ⇒ VERSION 3, 8 câu hỏi A0) + `JOURNAL/vd249-exit101-triage.md`: **exit 101 là mã CLI chủ ý** (`cli/run.rs:13-21`, VS05xx ⇒ 101), không phải panic; lỗi là vị trí guard (interp đếm vòng `interp.rs:541`, VM đếm lệnh `vm.rs:97,737`). |
| ST-262 | **(Agent 1) Thiết kế async thực dụng một luồng — design-only (VD-249)** | ST-260 / VD-249 | 2 h | **DONE (design-only)** | Ngoại lệ có phạm vi, **không triển khai VS3**. Trả lời 8 câu: suspend/resume, scheduler, I/O, timer, cancellation, vòng đời, giới hạn, host. So sánh phương án + dependency (không mặc định Future/Poll/Waker, không mặc định tự viết). Giữ ràng buộc: chưa worker/GC/API mới, chưa đổi ngữ nghĩa task, đánh giá stack 256 MB, công bố phần còn chặn. Phiếu trình A0 gồm phương án/thay thế, bất biến, ảnh hưởng ngôn ngữ/API/bytecode, lát cắt chạy được, tiêu chí từng lát, rủi ro — không dùng ước cũ làm cam kết. Kết quả: `JOURNAL/vd249-async-design-draft.md` (8 câu + 3 phương án A/B/C, **A đề xuất**, 6 lát 0–5, 10 quyết định A0); rủi ro số 1 = parity frame interp (Rust stack) vs VM (heap frame có PC). Tổng hợp tại `agent/DE-XUAT-KIEN-TRUC-ASYNC-2026-11-20.md`. |
| ST-263 | **(Agent 1) #148 HTTP deadline tổng + đo vòng đời dài (VD-249)** | ST-260 / VD-249 | 1,5 h | **DONE (chờ A0 chốt hợp đồng)** | Sửa #148: một deadline monotonic cho toàn request; mỗi bước chờ dùng thời gian còn lại; slow-drip không gia hạn; hết hạn đóng/thu hồi tài nguyên theo hợp đồng hiện có. Không TLS/HTTP2/API timeout mới; nếu SPEC chốt nghĩa khác thì trình đổi hợp đồng trước. Đo bổ sung: tải dài, số task/tài nguyên còn sống bên cạnh RSS, monotonic latency, lặp + phân bố, giữ baseline. Không xung đột tệp với ST-261/262. Kết quả: `JOURNAL/vd249-lifecycle-measure.md`. **Không đổi mã**: `timeout` đã được **VD-183 Q2(1)** chốt là "giữa hai lần đọc" ⇒ trình A0 chọn A/B/C (đề xuất A: re-chốt thành deadline toàn lời gọi). Đính chính: deadline đã tính một lần/response, slow-drip một response đã bị chặn; hở là chuỗi redirect (≤~6× timeout). Đo: 50k task ~223 ms; 20×200 ms ~4,06 s; 60 060 task ⇒ RSS 10,9→28,7 MB (~320 B/task, không bằng phẳng), control không spawn bằng phẳng 9,5 MB — reachable, không phải ròi unreachable. Baseline 987 test xanh, `check` PASS. |

| ST-264 | **(Agent 1) Lát 0 — prototype suspend/resume frame heap (VD-250)** | ST-262 / VD-250 | — | **DONE (prototype, chưa nối runtime)** | Chỉ prototype nội bộ, **không đổi runtime mặc định/bytecode phát hành**, không dùng stack lớn hay đệ quy scheduler. Cả 2 engine: A chạy→suspend→B chạy→A resume đúng vị trí; local/capture giữ giá trị; không lặp tác dụng phụ; kết quả + lỗi qua continuation; 1 ca await biểu thức lồng chỉ ra bảo toàn thứ tự đánh giá. Sau Lát 0 **dừng trình A0**, không tự mở Lát 1. Kết quả: `crates/vis-interp/tests/lat0_suspend.rs` 10/10, đủ 6 tiêu chí trên cả 2 mô hình frame; **model spike**, chưa nối `goi_than` thật; runtime/bytecode/VERSION không đổi. `check` PASS. Checkpoint `agent/BAO-CAO-CHECKPOINT-LAT0-2026-11-20.md` — dừng, chờ A0 duyệt Lát 1. |
| ST-265 | **(Agent 1) #148 chọn A — deadline tổng toàn lời gọi (VD-250)** | ST-263 / VD-250 | — | **DONE** | Supersede phần timeout liên quan của VD-183: một deadline monotonic từ đầu lời gồm connect/redirect/đọc; mỗi chờ dùng thời gian còn lại; không reset theo hop; giữ giới hạn riêng đã có; không tự thêm clamp 2 s; đóng tài nguyên trên success/error/timeout. Test đỏ-trước: redirect + response chậm. Không TLS/HTTP2/API mới. Kết quả: RED 1,01 s → GREEN ~300 ms (3 redirect + response chậm); 22 test HTTP xanh; `net.rs` một deadline cho toàn lời gọi, cùng `han` mọi hop, clamp 2 s/đọc giữ nguyên. Supersede VD-183.timeout. `JOURNAL/vd250-http148.md`. |
| ST-266 | **(Agent 1) Đặc tả còn thiếu cho Lát 1 — tất định/sự kiện/ownership/dependency (VD-250)** | ST-262 / VD-250 | — | **DONE (draft, chờ A0 chốt)** | Soạn: hợp đồng tất định theo event trace (reactor giả), định nghĩa chính xác 5 câu hỏi tính phí (E_CALL/E_EXPR, builtin, short-circuit, loop, resume), so sánh bảng phụ vs opcode `Charge` kèm kiểm crafted-bytecode (không tin artifact decode được), ownership task thu hồi được + handle thoát task, chính sách dependency ViScript (không áp tự động allowlist ViFastOs). Chưa ấn VERSION, chưa mở Lát 1. Kết quả: 5 file `JOURNAL/vd250-{determinism-contract,charging-spec,fee-repr,ownership,dependency-policy}.md`; 5 nhóm câu hỏi A0; chưa chọn crate/VERSION. |

| ST-267 | **(Agent 1) Lát 1 — nối scheduler/frame vào interp & VM thật (VD-251)** | ST-264 / VD-251 | — | **DONE** | Chỉ đường test nội bộ, **không đổi runtime mặc định/VERSION/cache/API**, không reactor/socket nonblocking, không giảm vị trí await checker. FIFO, wake không trùng, bỏ event đăng ký cũ, tie-break theo khóa trace, ownership thu hồi 2 điều kiện, `diag_snapshot()` test/debug. Chạy cả 2 engine qua source→tyck→đường thật; B1/B2 chỉ baseline. Dừng trước Lát 2. Kết quả: `crates/vis-cli/tests/lat1_frame.rs` 8/8 (cả 2 engine byte-identical stdout/chẩn đoán/trace/snapshot) + interp smoke 1/1; `cargo test -p vis-interp -p vis-cli` 1007/0; `./vis-dev gate --fast` **1450/1450 PASS** (sau `cargo clean` sửa artifact stale do drift toolchain stable 1.99.1→1.100.0). 8 tiêu chí A0 đạt; B1 4,04 s / B2 cùng-ms giữ baseline. Step-cap parity vẫn mở. `agent/BAO-CAO-CHECKPOINT-LAT1-2026-11-20.md` — dừng, chờ A0 duyệt Lát 2. |

| ST-268 | **(Agent 1) Lát 2 — reactor read/accept + timer tối thiểu trên đường frame (VD-252)** | ST-267 / VD-252 | — | **DONE** | Cả 2 engine, chỉ đường test; bỏ sleep(5ms), monotonic timer, FIFO/wake gen, WouldBlock đăng ký đúng, cleanup đúng 1 lần; không runtime mặc định, không VERSION/cache/API, Linux-only rõ, Windows không hồi quy. Nghiệm thu: trace giả parity, mạng thật, cleanup, B1 chồng lấp (~0,3s mục tiêu đo), B2 vẫn mở. Dừng trước Lát 3. Kết quả: `crates/vis-cli/tests/lat2_reactor.rs` 13/13, byte-identical 2 engine cùng trace; B1 đường frame 201,2–201,5 ms (mặc định 4,03–4,04 s); B2/step-cap vẫn mở; explicit-rejection 3 chỗ có test âm. Reactor **giả** — real readiness cần dependency. `cargo test -p vis-interp -p vis-cli` 1020/0; gate 1463/1463. `agent/BAO-CAO-CHECKPOINT-LAT2-2026-11-20.md` — dừng trước Lát 3. |
| ST-269 | **(Agent 1) Ma trận coverage + đề xuất dependency selector (VD-252)** | ST-267 / VD-252 | — | **DONE** | Ma trận cấu trúc HIR được/phủ/không phủ trong đường frame, cấu trúc unsupported **từ chối rõ** (không fallback đồng bộ). Đề xuất 1 selector ưu tiên + thay thế ngắn: license/platform/feature/transitive/unsafe — **chưa merge**, chờ A0 duyệt. Ghim toolchain 1.100.0 đã làm; MSRV 1.99.1 giữ nguyên. Kết quả: `JOURNAL/vd252-{coverage-matrix,selector-proposal,charging-final}.md`. Số liệu selector đã kiểm chứng độc lập: `mio = "=1.2.5"` (MIT, transitive chỉ libc, cfg Linux-only) ưu tiên; `polling = "=3.11.0"` thay thế; **chưa merge**, chờ A0 duyệt. 5 quyết định tính phí đã chốt. |

| ST-270 | **(Agent 1) Nâng MSRV 1.99.1 → 1.100.0 (VD-253, override VD-252 §6)** | ST-269 / VD-253 | 0,25 h | **DONE** | Chủ trì trả lời "nâng hết lên 1.100.0" gồm cả MSRV. Đổi `rust-version` tại `Cargo.toml`, `vis-host-wasm/Cargo.toml`, `vis-host-linux/Cargo.toml`; crate khác dùng `rust-version.workspace = true`. `rust-toolchain.toml` giữ ghim `1.100.0`. Không sửa báo cáo lịch sử ghi 1.99.1. Luật R1..R6 OK. Đổi rust-version 1.99.1→1.100.0 tại 3 manifest; toolchain ghim 1.100.0; luật OK. |

| ST-271 | **(Agent 1) Tích hợp mio 1.2.5 + Lát 3 readiness thật Linux (VD-254)** | ST-268 / VD-254 | — | **DONE** | Duyệt `mio = "=1.2.5"`, `default-features=false`, `features=["os-poll","net"]`, chỉ crate sở hữu adapter + cfg Linux, chỉ đường test nội bộ. Adapter read/accept đăng ký/đăng ký lại/hủy, token có generation, WouldBlock không hot loop, nhiều waiter, lỗi selector rõ, deadline monotonic. Test mạng thật interp+VM, B1-real-frame 1/5/20, cleanup. Không runtime mặc định/VERSION/cache/API; Windows/core/wasm không hồi quy. Dừng trước bước đổi runtime mặc định. Kết quả: `agent/archive/journal-pre-v040/vd254-lat3-report.md`. mio 1.2.5 dev-dependency cfg Linux trong `vis-cli` (checksum/lockfile đúng, không polling/rustix/log, Windows/wasm không kéo mio). Reactor thật epoll cho read/accept trên đường frame cả 2 engine; 12/12 test mạng thật; B1-real-frame N=1/5/20 ≈ 201/202/202–203 ms (startup 0,8–10,7 ms) so B1-real-default 4,03–4,04 s. `cargo test -p vis-interp -p vis-cli` 1032/0. B2/step-cap/coverage/closure-writeback vẫn mở. `agent/BAN-GIAO-A0-LAT3-2026-11-20.md` — dừng, chờ A0. |
| ST-272 | **(Agent 1) Giấy phép & thông báo bên thứ ba cho mio (VD-254)** | ST-271 / VD-254 | — | **DONE** | Ghi license MIT của mio 1.2.5, transitive libc/windows-sys theo chính sách ViScript; review diff Cargo.lock đúng version/registry/không dependency lạ. Chỉ tài liệu, chờ commit tích hợp của ST-271 để đối chiếu lockfile. Kết quả: `agent/archive/journal-pre-v040/vd254-third-party-notice.md` — MIT, checksum đầy đủ, 11 ô checklist lockfile; rà soát xác nhận lockfile đúng version/registry/không dependency lạ; thông báo bên thứ ba tại chỗ (chưa có THIRD-PARTY.md/deny.toml). |

| ST-273 | **(Agent 1, luồng tooling/README song song) Đăng ký GitHub Linguist + kiểm ví dụ README** | ST-272 | — | **DONE** | README dùng fence ` ```python title="ViScript" ` để tô màu tạm (GitHub chưa nhận `vis`); thêm `test/readme-examples.py` vào `bin/vis-dev check` (20/20 khối xanh, tách khỏi cổng doc-examples của Agent 0); README cập nhật yêu cầu Rust ≥ 1.100.0 (VD-253). Khung `infra/linguist/` gồm tree-sitter grammar scaffold, patch `languages.yml`/`grammars.yml`/sample + `PR-BODY.md` — **chưa gửi PR**, grammar phải sinh/test riêng. Thêm `vi-du/` ví dụ. Validate: `vis-dev check` PASS, `gate --fast` 1475/1475. |

| ST-274 | **(Agent 1) MỞ RỘNG coverage frame & closure/writeback (VD-255/256 — KHÔNG phải hoàn tất)** | ST-271 / VD-255 | — | **DONE** | Mã task riêng, **không gọi Lát 4** (số đó dành cho hạn ngạch CPU). 3 nhóm nhỏ: A điều khiển luồng (if/match, loop, break/continue, short-circuit); B biểu thức/gọi (field/index, record/tuple, đối số tên, toán tử còn thiếu, hằng cấp tệp); C closure (callee, capture dùng chung, writeback, nhiều frame qua suspend). Test source thật cả 2 engine, suspend ở vị trí dễ sai, không eval lại operand, golden chỉ đổi có căn cứ, **không fallback đồng bộ**. Chưa đổi runtime mặc định. Kết quả: `agent/archive/journal-pre-v040/vd255-coverage-report.md`. `frame.rs` tách thành `frame/{proc,expr}.rs`; A: if/match/loop/break/continue/short-circuit/`??`/`|>`; B: field/index/`?.`/record/tuple/named args/hằng cấp tệp; C: closure callee + capture dùng chung + writeback qua suspend. Test mới `lat255_coverage.rs` 9/9 (source thật, cả 2 engine), lat1 8/8 + lat2 13/13 + lat3 12/12; `cargo test -p vis-interp -p vis-cli` 1041/0; gate 1484/1484. Fixture async thực tế **26 `.vis`** (52 tệp gồm .out/.err), đã phân loại, **không đổi golden**. Còn mở: `?`/`chan`/`CmdBlock` từ chối rõ, lỗi final-join chưa surfaced, VM codegen `await` trong thân closure, chưa tính phí; **không đổi runtime mặc định**. |
| ST-275 | **(Agent 1) Phiếu quyết định ngân sách ngắn (VD-255)** | ST-266 / VD-255 | — | **DONE** | Chuẩn bị song song: danh mục sự kiện + trọng số; bảng phụ vs `Instr::Charge`; kiểm crafted-bytecode không né phí; quan hệ ngân sách tổng/hạn ngạch lượt. **Chưa chọn biểu diễn, chưa đổi VERSION/cache/API.** Không để finding step-cap biến mất. Kết quả: `agent/archive/journal-pre-v040/vd255-budget-decision-sheet.md` — 8 sự kiện + trọng số đề xuất (D1–D9 chờ A0), so sánh bảng phụ/opcode, kiểm crafted-bytecode né phí, ngân sách tổng vs hạn ngạch lượt. **Chưa chọn biểu diễn, chưa đổi VERSION/cache/API**; step-cap parity vẫn mở, không tăng ngưỡng. |

| ST-276 | **(Agent 1) Instrumentation & biểu diễn S cho ngân sách (VD-256)** | ST-275 / VD-256 | — | **DONE (subset, chưa bật default)** | D1–D9: catalog theo chương trình (loại writeback nội bộ, tách await/final join), bảng phụ S theo PC (kind/site, không serialize weight), nhiều sự kiện/có thứ tự tại 1 PC, verifier cấu trúc+CFG, fuel VM riêng, test-hook budget nhỏ + trace, VERSION chưa dùng + cache. Test so **dãy sự kiện có thứ tự**, không chỉ tổng. Chưa bật ngân sách mặc định; chưa chốt trọng số/BUDGET; step-cap vẫn mở. Kết quả: `agent/archive/journal-pre-v040/vd256-budget-report.md`. `fee_shared.rs` định nghĩa chung 8 sự kiện; bảng phụ S theo PC (`pc/kind/site`, nhiều sự kiện có thứ tự, **không serialize weight**); verifier cấu trúc+CFG, bảng rỗng bị từ chối; fuel VM riêng `VS0501-FUEL`; test-hook budget nhỏ; `fee::VERSION=4` (3 là sentinel lạ) + cache key/hash cập nhật; core VBC1 vẫn 2. Test so **dãy sự kiện có thứ tự** byte-identical; `cargo test -p vis-interp -p vis-cli -p vis-bytecode` 1151/0/1 ignored; gate cây chung 1512/1512. **Chưa bật ngân sách mặc định**; trọng số/BUDGET 20 triệu + fuel 10^7 là đề xuất chờ A0; step-cap vẫn mở.
| ST-277 | **(Agent 1) Lát connect/write Pending + partial write (VD-256)** | ST-274 / VD-256 | — | **DONE** | Tách quyền tệp với ST-276. connect Pending→readiness→kiểm kết quả thật; write partial+offset, resume không gửi lại; deadline không reset; peer không đọc không giữ scheduler vô hạn; cleanup đúng. Test: nối thất bại, peer chậm, write lớn backpressure, đóng giữa chừng, task khác tiến triển. Giữ API/runtime mặc định; đổi hợp đồng net.write phải trình riêng. Kết quả: `agent/archive/journal-pre-v040/vd256-connect-write-report.md`. connect Pending→readiness→kiểm kết quả thật (SO_ERROR/peer_addr); write partial + offset, resume không gửi lại; deadline không reset; token generation, cleanup đúng 1 lần. `lat256_net.rs` 9/9 (lặp 3 lần 8,15–8,20 s); tổng 1058/0; chỉ sửa `stdlib.rs`, không đụa `net.rs`/`rt.rs`. Chỉ đường test; API/runtime mặc định/VERSION không đổi.
| ST-278 | **(Agent 1) Sửa lỗi final-join bị nuốt (VD-256)** | ST-274 / VD-256 | — | **DONE** | Lỗi task ở final join phải surfaced theo hợp đồng VS0506 hiện hành; không tự đổi chính sách hủy các task còn lại trong cùng bản sửa. Có test âm/dương. Kết quả: nằm trong `vd256-budget-report.md` — `frame_final_join` giờ **surfaced** `VS0506` + mã/span gốc trên cả 2 engine, chương trình không exit thành công; **không đổi** chính sách hủy task còn lại. 3 dòng wiring trong `frame.rs` đã ghi rõ trong report.

| ST-279 | **(Agent 2, worktree A) Coverage frame còn lại — VD-257** | ST-276 / VD-257 | — | **DONE** | `?`, channel (đầy/rỗng/đóng), `CmdBlock`, await trong closure trên VM, field/index assignment + phân biệt method user/host, resume không double-charge. Source thật 2 engine, không fallback đồng bộ, không đổi checker. Worktree `vd257-a-coverage`, sở hữu `frame/**`, `vm_frame.rs`, test `lat257_a_coverage.rs`. Merge `eee4fb0`→`e31ce3f` (VD-257/A): `?` (Option/Result/index), `chan` đầy/rỗng/đóng, `cmd` từ chối rõ, field/index assign, host-vs-value method, resume không double-charge. `lat257_a_coverage` 13/13; đã patch 2 assertion VD-255 lỗi thời + 1 assertion closure-await sau khi B sửa VM. VM closure-await giờ **chạy được** (B sửa `goi_ham_bytecode`).
| ST-280 | **(Agent 2, worktree B) Tương thích bytecode/cache + bằng chứng ngân sách — VD-257** | ST-276 / VD-257 | — | **DONE** | 4 trục phiên bản (module/event-table/weight-table/cache); runtime cũ từ chối artifact mới, runtime mới xử lý artifact cũ rõ ràng; cache + đường nạp trực tiếp được bảo vệ. Được tăng core VERSION sang số chưa dùng (kiểm lịch sử, không chọn để né test). Đo số lệnh VM, miền hỗ trợ, quan hệ fuel↔ngân sách; crafted/fee sai/loop né phí. Chưa bật ngân sách mặc định. Worktree `vd257-b-budget`. Merge `0bad843`→`a9faed7` (VD-257/B): 4 trục VERSION (module 2/3 instrumented, event-table 4, weight-table 1, cache), runtime cũ từ chối artifact v3 cả đường nạp trực tiếp, runtime mới từ chối artifact cũ thiếu VFEE; cache pin `98a0b983…`. Đo số lệnh VM: root cause step-cap = fuel 10⁷ chặn trước ngân sách ~18M; đề xuất (chưa duyệt) BUDGET 40M / FUEL 50M. 9 compat + budget + closure-await tests xanh.
| ST-281 | **(Agent 2, worktree C) Cancellation & vòng đời — VD-257** | ST-274 / VD-257 | — | **DONE** | Hủy nội bộ hợp tác tại điểm an toàn; gỡ I/O/timer/waiter đúng 1 lần; không đóng handle thoát ra/chia sẻ; handle sống giữ kết quả/lỗi; hết handle + xong join ⇒ thu hồi; giữ final join, không tự hủy lúc main exit. Mã lỗi/chính sách lỗi mới ⇒ blocker riêng. Worktree `vd257-c-lifecycle`. Merge `a36d8d5`→`b91b979` (VD-257/C): cancellation nội bộ tại safe point, gỡ I/O/timer/waiter đúng 1 lần, token gen, handle sống giữ kết quả, hết handle+join thu hồi; final join giữ nguyên. `lat257_c_lifecycle` 11/11. Blocker A0: mã lỗi cancellation riêng & chính sách lỗi task đầu/chính sách khác — ghi riêng, không tự đổi.
| ST-282 | **(Agent 2, worktree D) Phân tích fairness & quét Q — VD-257** | ST-274 / VD-257 | — | **DONE** | Phase 1 chỉ phân tích/đo (không sửa source để tránh va A/C): thiết kế quota riêng, reset/turn, hết hạn đưa task về cuối ready (không trap), yield tại continuation đã lưu; kế hoạch quét Q + đo task/tài nguyên sống qua sóng; liệt kê file cần sửa ở Phase 2 để leader xếp thứ tự bàn giao. Worktree `vd257-d-fairness`. Phase 1 xong (`2de9c51`): thiết kế quota riêng/reset-turn/tail-không-trap, kế hoạch quét Q + đo vòng đời, liệt kê file phase 2. Phase 2 đã dispatch sau khi A/B/C merge + rebase lên `fd4ba32`: nối `charge` vào frame path, quota thử nghiệm, quét Q, B2, đo task/tài nguyên sống. Không chọn default Q. **Phase 2 xong**: `fef5c45` → merge `3a8e1e7`; báo cáo `JOURNAL/vd257-d-fairness-result.md`; vẫn **không** chọn default Q.

| ST-283 | **(Agent 1, tự chủ VD-257 §6) Regression/fuzz/đo dài có mục tiêu** | ST-282 / VD-257 | — | **DONE** | Không mở tính năng mới. Chạy kiểm hồi quy đa target (Windows-msvc/core/wasm32), fuzz parser có seed hữu hạn + lưu seed lỗi, đo release có giới hạn, soak vòng đời hữu hạn. Mọi kết quả ghi `agent/archive/journal-pre-v040/vd257-regression.md`; không chạy lại full gate trên cùng commit chỉ để tăng số PASS. Kết quả: `agent/archive/journal-pre-v040/vd257-regression.md`. 3 target check PASS (core/Windows-msvc/wasm32 — chỉ biên dịch); fuzz 3 seed × 60s (~28,4 triệu ca) 0 vi phạm; đo release: loop-2M 374,68 ms (debug 3,43 s), Q sweep cost-unit không đổi, snapshot 0. Không đổi đề xuất số, không chạy lại full gate. |

## Đợt tối ưu liên tục VD-258..VD-270 (A0, 2026-11-20 → 2026-11-21)

> **Không cấp ST-## cho từng mục** trong đợt này: A0 điều hành trực tiếp theo `VD-258`..`VD-270`, mỗi mục
> là một commit/merge có mã tính năng. Nhật ký đầy đủ (từng commit, số đo, test) ở
> [`JOURNAL/vd260-report.md`](JOURNAL/vd260-report.md); kế hoạch 4 luồng ở [`JOURNAL/vd258-plan.md`](JOURNAL/vd258-plan.md);
> tiến độ tích hợp ở [`JOURNAL/vd258-progress.md`](JOURNAL/vd258-progress.md); quyết định ở `DECISIONS.md`
> VD-258..VD-270. Khối này **định vị** đợt trong QUEUE, không thay thế nhật ký.
>
> **Trạng thái**: đợt **chưa đóng** (uỷ quyền đến 09:00 23/11). Không mở pha 2/VS3, không đổi runtime
> mặc định, không tag mới (`pha1-rc1` = `05e42d0` giữ nguyên).

| Chặng | Nội dung chính | Bằng chứng | Trạng thái |
|---|---|---|---|
| VD-258 — 4 luồng A/B/C/D | checker/soundness; frame/async; fee/tooling; SQLite/stdlib | `JOURNAL/vd258-plan.md`, `vd258-progress.md`; gate 1633 | DONE |
| Wave 1–6 | `let … else`, break/continue biểu thức, `with`, generic, range lười, infer; stdlib `zip/min/max/lines/split_whitespace` | `JOURNAL/vd260-report.md` §1; gate 1822 | DONE |
| Wave 7–10 | LSP definition/documentSymbol/references/rename/signatureHelp/folding; CLI `run/check --stdin`, `fmt --stdin`, `fmt --check --json/list/diff/stat`; `str.find/rfind/count`; `list.index_of`; `panic`; `i64::MIN` | `JOURNAL/vd260-report.md`; gate 2170 | DONE |
| Wave 11–14 | `fs.read_lines/append/mkdir/remove/walk/glob/copy/rename/size/is_file/is_dir`; `sqlite.query_one/last_insert_rowid/changes`; `sqlite.execute_batch`; LSP hover/implementation; flatten; db explain | `JOURNAL/vd260-report.md`; gate 2293 | DONE |
| Wave 15–19 | `path.normalize/relative/join_all`; `str.strip_prefix/suffix`, `pad_*`, `splitn`, `parse_int`, `parse_float`; `list.binary_search`; `test --repeat/--shuffle`; `fmt --stat`/`--diff` | `JOURNAL/vd260-report.md`; gate 2505 | DONE |
| Wave 20–22 | `list.median/sum_float/sum/product/contains/windows/is_empty`; `path.is_relative/is_absolute/with_extension`; `str.is_ascii/replace_first/last/n`; `sqlite.tables/columns/indexes/index_columns/primary_keys/index_exists/table_exists/schema_sql/explain` | `JOURNAL/vd260-report.md`; gate 2821 | DONE |
| Wave 23 | `map.contains_key` (đã merge), `str.char_at`, `path.is_absolute`, `str.is_blank` (theo `STATE.md` lúc ghi) | `CHANGELOG.md`, `STATE.md` (21/11) | IN PROGRESS |

**Mốc đo cuối khối này** (chi tiết + log trong `JOURNAL/vd260-report.md`):

- `gate --fast` **2840 passed / 0 skipped** @ `745851a`; `vis test test/sem` **525/525** interp và VM.
- `core-test` / `release` / `toolchain` / `host 54/54` **PASS** @ `a71c85b`.
- `API-STD.txt` **136** mục; nightly gần nhất PASS (fuzz 10.880.217 ca / 0 vi phạm @ `da6b12c`).
- Windows Tier 2: workspace xanh @ `c7cef71` (20/11), **chưa verify lại ở HEAD**; macOS **chưa verify**.
- Còn mở: step-cap parity; P3 enum alloc (#152); #190; #191; #147/#152 blocked phần cứng; TLS/HTTP2;
  runner wasm/macOS; budget/fuel/Q chưa chốt default.

## Chương trình Dài hạn hậu `v0.3.5` — `VD-272`..`VD-280` (`v0.3.6` → `v1.0.0-rc1`)

> **Nguồn quyết định**: `agent/DECISIONS.md` `VD-272`..`VD-280` (2026-11-22), `agent/archive/journal-pre-v040/quyet-dinh-a0-2026-11-22.md`, `agent/archive/journal-pre-v040/quyet-dinh-a0-2026-11-22-vd280.md`.
> **Luật thực thi (`VD-279`/`VD-280`)**: 4 luồng worktree riêng (`W1` Runtime/Worker, `W2` Net/Sec, `W3` Budget/Select/Platform, `W4` QA/Windows/Docs); batch test 4 giờ; **không** GitHub Actions; macOS ngoài phạm vi. Giai đoạn sau chỉ chuyển `BLOCKED → READY` khi toàn bộ task giai đoạn trước ở trạng thái `DONE`.

### Giai đoạn 0 — Khử nghẽn R1, Hook huỷ HTTP, Khép G3 Gói & Đồng bộ Sự thật (`v0.3.6` — ĐÃ TAG @ `8fa2c09`)

| ID | Mục tiêu | Phụ thuộc | Ước lượng | Trạng thái | Ghi chú |
|---|---|---|---|---|---|
| ST-284 | **(W1) R0a Refactor `vm.rs` & `frame.rs` (0 đổi hành vi)** | VD-272 | 1 ngày | DONE | APPROVED 2026-11-22 Agent 0 (`VD-280`). Merge `e363a15` (`83a9a10`): `vm.rs` 1.240, `frame.rs` 1.042 (tách `vm/host.rs`, `frame/worker.rs`); 0 đổi hành vi, gate PASS, diff 100k = 0 (`JOURNAL/st284-refactor.md`). |
| ST-285 | **(W1) R0b Refactor `value.rs`, `codegen.rs`, `tyck/expr.rs`, `interp/goi.rs` (0 đổi hành vi)** | VD-272 | 1 ngày | DONE | APPROVED 2026-11-22 Agent 0 (`VD-280`). Merge `246b6c5` (`ab5d292`): `value.rs` 1.186, `codegen.rs` 1.045, `tyck/expr.rs` 1.064, `interp/goi.rs` 1.155; 0 đổi golden/API/cache, diff 100k = 0 (`JOURNAL/st285-refactor.md`). |
| ST-286 | **(W2) R0c Refactor `async_net.rs`, `quyen.rs`, `db/sqlite/mod.rs` (0 đổi hành vi)** | VD-272 | 1 ngày | DONE | APPROVED 2026-11-22 Agent 0 (`VD-280`). Merge `cfba662` (`f193c57`): `async_net.rs` 606, `quyen.rs` 1.190, `db/sqlite/mod.rs` 1.245; diff 100k = 0 (`JOURNAL/st286-tach-module.md`). |
| ST-287 | **(W2) Hook huỷ giữa HTTP request cho cả HTTP/1.1 và HTTP/2 (`VS3-1b`)** | ST-286 / VD-273 | 1 ngày | DONE | APPROVED 2026-11-22 Agent 0 (`VD-280`). Merge `f8f1d4b` (`03ed84c`) + Windows fix `0645700` (`6b2a216`): `cancel_task_handles` đóng socket h1 / `RST_STREAM` h2 giữ session; Linux 5/5 + 8/8, Windows 40/40 + tag 10/10 (`JOURNAL/st287-http-cancel.md`, `st287-win-cancel-flake.md`). |
| ST-288 | **(W3) Khép G3 Gói: cấp `VS0708`, chốt `pkg gc --root` & vá `#142`** | VD-273 | 1 ngày | DONE | APPROVED 2026-11-22 Agent 0 (`VD-280`; Gatekeeper độc lập `ae5fc9c`). Merge `545a38d` (`c513e7a`): `VS0708` vi/en/ja (`API-CODES.txt` = 54), soft pin `pkg update` (đóng `#142`), `pkg gc --root` dedupe (`JOURNAL/st288-goi.md`). |
| ST-289 | **(W4) Đồng bộ Version, `SO-NO.md`, Docs hiện thực, `#151` & Báo cáo `R32` (`v0.3.6`)** | ST-284, ST-285, ST-286, ST-287, ST-288 / VD-272 | 1,5 ngày | DONE | APPROVED 2026-11-22 Agent 0 (`VD-280`). Tag **`v0.3.6`** @ `8fa2c09` + Windows tag verify `be51ae09`: 8 crate `version = "0.3.6"`, đóng `#111`/`#151`, `R32-viscript.{md,json}`; Linux gate 3.581/3 + release 3.586/0/3 + soak 10/10 + diff 0; Windows 3.538/0/3 + cancel 10/10 + host 54/54 (`JOURNAL/tag-v036-win.md`). |

### Giai đoạn 1 — G4a Core: Ngân sách, Worker `P2b`/`P4`, `select`, `async-h2` & TOCTOU Ghi (`v0.4.0`)

| ID | Mục tiêu | Phụ thuộc | Ước lượng | Trạng thái | Ghi chú |
|---|---|---|---|---|---|
| ST-290 | **(W3) Bật Ngân sách mặc định (`40M`) + cấp `VS0509`/`VS0510` + bump cache (`VS3-1a`)** | ST-284, ST-289 / VD-273, VD-280 | 2 ngày | DONE | APPROVED 2026-11-22 Agent 0 (`VD-281`). Merge `a48d5b87` (`3627f3bc`): ngân sách default `40M`, `VS0509`/`VS0510` (`API-CODES.txt` = 56), `--max-budget`/`--unlimited-budget`, cache `VFEE` + fix `verify_fee`; đỉnh `test/sem` = 18.019 khớp 2 engine, workspace 3.594/0/3, diff 0 (`JOURNAL/st290-budget-default.md`). **Kết quả:** DONE — merge `a48d5b87`; gate **3594/0/3**. |
| ST-291 | **(W1) Worker `P2b-1 + P2b-2`: lazy/batched job submit & compact `Vec` proxy** | ST-284, ST-285, ST-289 / VD-275, VD-280 | 2 ngày | DONE | APPROVED 2026-11-23 Agent 0 (`VD-283`). Merge `8cc3588` (`269ff71`): batch 10k 28,6–29,7 ms (đạt ≤30 ms), gate **3.656/4**; phê chuẩn **`P2b-3/4 NO-GO`** (`16fba3a`, envelope 15,5 ms/10k mang tính cấu trúc). |
| ST-292 | **(W1) Worker Frame `P4a..P4c` + `VS0509` + `--workers <N>` + nối `SharedBudget` (`VS3-3b`)** | ST-290, ST-291 / VD-275, VD-280, VD-281 | 4–5 ngày | DONE | APPROVED 2026-11-23 Agent 0 (`VD-283`). Merge `e47b3ca` (`eca4e82`): `P4a/P4b + SharedBudget + --workers <N> + VS0509`, worker-frame 12/12 + VM 6/6, soak 10m RSS phẳng, gate **3.656/4**, Windows 3.607/0/3. Phần tồn đọng `P4c` (`VM --workers` reactor + test cancel frame) tách thành nợ **`#192`** xử ngay đầu `ST-301`. |
| ST-293 | **(W3) Kênh `select` (`VS3-2c`): `select_recv*` / `select_send*`** | ST-285, ST-289 / VD-274, VD-280 | 3 ngày | DONE | APPROVED 2026-11-22 Agent 0 (`VD-282`). Merge `6d6bfe9c` (code `82064e96`): 4 builtin + pseudo-record `SelectRecv[T]` cô lập tại `rt/select.rs` + `tyck/expr/kenh.rs` (0 chạm `interp.rs`/`stdlib.rs`); **25/25** ca byte-identical interp↔VM cả Linux & Windows release; gate **3.632/4**, diff 100k seed `202611222100` = 0 (`JOURNAL/st293-select.md`). |
| ST-294 | **(W2) Port `async-h2` sang Windows, dọn warning non-TLS & bật mặc định non-wasm** | ST-287, ST-289 / VD-277, VD-280 | 3 ngày | DONE | APPROVED 2026-11-22 Agent 0 (`VD-281`). Merge `cce2d0fb` (`39cc81cc`) + fix soak snapshot `9c0ffd12`: gỡ `#![cfg(unix)]`/`RawFd`/`SourceFd`, `IoH2` dùng `SockChiaSe(Rc<RefCell<mio::net::TcpStream>>)` + `rustls::StreamOwned`, bật `async-h2` default-on non-wasm, 0 warning `--no-default-features`; Linux gate 3.607/4, Windows `psvn` 3.563/0/3, `vs3_http2` 33/33 ×2, `vs3_http_cancel` 8/8 (`JOURNAL/st294-h2-windows.md`). |
| ST-295 | **(W2) Đóng khe TOCTOU đường ghi Unix (`#109b`/`#140`) & Landlock `#125`** | ST-286, ST-289 / VD-276, VD-280 | 3 ngày | DONE | APPROVED 2026-11-22 Agent 0 (`VD-282`). Merge `cd76da56` (code `ba4023fb`): `rustix =1.1.6` (`fs`+`std`, Unix-only; 3 gói mới đều MIT/Apache-2.0, `st295-license.log`), `openat2`/`O_NOFOLLOW` chung đọc+ghi trong `quyen_mo.rs`, race ghi tất định **100/100 → 0/100**, Landlock `Execute` **18/18** + `st295_sandbox_proc` 3/3, đã xoá cảnh báo đường ghi ở `HOST-BAO-DAM.md`, gate **3.613/4**, diff 0; đóng `#109b`, `#125`, `#140` (`JOURNAL/st295-baove.md`). |
| ST-296 | **(W4) Tổng QA, Nightly, Fix `vd270_test_slowest`, Đối chiếu `SO-NO.md`, `R33` & Release `v0.4.0`** | ST-290, ST-291, ST-292, ST-293, ST-294, ST-295 / VD-272, VD-280, VD-281, VD-282 | 2 ngày | DONE | APPROVED 2026-11-23 Agent 0 (`VD-283`). Tag **`v0.4.0` @ `c2accdf`**: Linux gate **3.656/4**, release **3.661/0/4**, 2× soak **10/10**, diff 0; Windows full **3.607/0/3**, release **3.606/0/3** (`vis 0.4.0`). |

### Giai đoạn 1.5 — Tối ưu Cấu trúc Repository & Quy trình DevEx (`v0.4.1` — ĐÃ TAG @ `90844bc`, APPROVED `VD-283`)

| ID | Mục tiêu | Phụ thuộc | Ước lượng | Trạng thái | Ghi chú |
|---|---|---|---|---|---|
| ST-296d-tools | **(W4) Vá harness soak/timing, Luật cổng R7 & Bộ công cụ vận hành (`vis-{gc-targets,jobs,status,gate-windows}`)** | VD-282 | 0,5 ngày | DONE | APPROVED 2026-11-23 Agent 0 (`VD-283`). Merge `f240c95` + `54e82fc` (`b36f8a5`/`4b2484f`): fix `bin/vis-soak` TMPDIR & LSP timing, Luật **R7** (173 legacy log đóng băng, self-test 15/15), `.config/nextest.toml` (`retries=0`), 4 script `bin/vis-*`, auto `CARGO_BUILD_JOBS`/`sccache`. |
| ST-296a | **(W4) Gom 280 test binary `crates/vis-cli/tests/` → 13–22 binary theo miền + `tests/common/mod.rs`** | ST-296d-tools / VD-281, VD-282 | 1 ngày | DONE | APPROVED 2026-11-23 Agent 0 (`VD-283`). Merge `53990c0`: `vis-cli` **280 → 22** binary, toàn workspace **352 → 92** binary, `gate --fast` giảm **304 s → 108–210 s**, giữ 100% **3.656/4** test pass. |
| ST-296b | **(W3) Tách `crates/vis-loader` & `crates/vis-lsp` + `[workspace.dependencies]`** | VD-281, VD-282 | 1,5 ngày | DONE | APPROVED 2026-11-23 Agent 0 (`VD-283`). Merge `067e1a5`/`1b27cb8`: tách `crates/vis-loader` (3.332 LOC, xanh `wasm32`) + `crates/vis-lsp` (6.771 LOC) + `[workspace.dependencies]`; gate **3.656/4**, Windows **3.607/0/3**, targets core 18/18, diff 0. |
| ST-296c | **(W1) Gom 68 tệp `vis-interp/src/stdlib/`, tách `vis-types/src/stdlib/` & khử 3 tệp cận trần R1** | ST-296 / VD-281, VD-282 | 1 ngày | DONE | APPROVED 2026-11-23 Agent 0 (`VD-283`). Merge `ab8d393` + flake fix `044b289`: `vis-types/stdlib.rs` 1.456→23+11, `sqlite/tests.rs` 1.495→68+3, `lower/expr.rs` 1.456→702+2, `lower.rs` 1.446→420+2, interp stdlib 68→17; gate **3.656/4**, diff 0. |
| ST-296d | **(W4) Quy hoạch `agent/archive/`, `DECISIONS-INDEX.md` & Tag `v0.4.1`** | ST-296a, ST-296b, ST-296c, ST-296d-tools / VD-281, VD-282 | 0,5 ngày | DONE | APPROVED 2026-11-23 Agent 0 (`VD-283`). Merge `a1730c5` + `90844bc`: 63 tệp archive, `DECISIONS-INDEX` 271 VD, `vis-host-wasm/Cargo.lock` 0.4.1, Windows full 3.607/0/3 & Linux release 3.661/0/4, tag **`v0.4.1` @ `90844bc`**, `R34-viscript.md` chốt. |

### Giai đoạn 2 — G4b Platforms: Windows Tier 1 & WASI/Wasmtime Runtime (`v0.4.5`)

| ID | Mục tiêu | Phụ thuộc | Ước lượng | Trạng thái | Ghi chú |
|---|---|---|---|---|---|
| ST-297 | **(W3) Đóng toàn bộ nợ `#149` (Path-split audit đa nền)** | ST-296d / VD-277, VD-281 | 2 ngày | DONE | APPROVED 2026-11-23 Agent 0 (`VD-283`). Merge `f4cb136` + `8a9f93e`: LSP URI UNC/Windows + `--grant` path classification; gate **3.660/4**, Windows **3× full 3.611/0/3**, Linux **10×** `vis-lsp` 29/29 & `path_impl` 2/2; **`#149` đóng**. |
| ST-298 | **(W4) Hoàn tất `bin/vis-gate-windows` xác nhận 3× + soak 4h & nâng Windows Tier 1** | ST-294, ST-297 / VD-277, VD-281, VD-282, VD-283 | 2 ngày | DONE | APPROVED 2026-11-23 Agent 0 (`VD-284`). Merge `3daff62b` + `30890bab`: 3× `vis-gate-windows --full` **3.611/0/3** (host 54/54, targets 18/18+17/1) + **soak 4h Windows 985 vòng / 0 fail** (RAM phẳng 8.204–8.766 MB, proc/threads 0/0, dirty 0); **Windows Tier 1** công bố trong `ROADMAP.md` & `HOST-BAO-DAM.md` (caveat `FsGuard`). |
| ST-299a | **(W2) Triển khai `WasiHost` lõi trong workspace độc lập `vis-host-wasm/` (`wasm32-wasip1`)** | VD-277, VD-282 | 2 ngày | DONE | APPROVED 2026-11-23 Agent 0 (`VD-283`). Merge `060a744` (`5cc1903`): `WasiHost` `wasm32-wasip1` + `vis-wasm-run` (0 `unsafe`), wasm **26/26** (13×2) trên WASI Preview 1. |
| ST-299b | **(W2) Nối `vis-loader` vào `vis-host-wasm` cho chương trình đa tệp** | ST-299a, ST-296b, ST-296d / VD-277, VD-282 | 1 ngày | DONE | APPROVED 2026-11-23 Agent 0 (`VD-283`). Merge `3b8049c` + `aab5ffa`: `vis-loader` đa tệp + `use` trong `vis-wasm-run`; native **29/29**, wasm **28/28**, gate **3.660/4**. |
| ST-300 | **(W3/W4) Runner `bin/vis-wasm-suite` / `vis-wasm-sem` (WASI), Fix `vis-dev` fuzz dir, `R35` & Release `v0.4.5`** | ST-298, ST-299b / VD-277, VD-282, VD-283 | 3 ngày | DONE | APPROVED 2026-11-23 Agent 0 (`VD-284`). Tag **`v0.4.5` @ `3daff62`**: `bin/vis-wasm-sem` **388/390 = 99,5%** (VM 100%, nợ `#193`), nightly fuzz 13,56M/0, final Linux soak 10/10, fix `CARGO_TARGET_DIR` trong `bin/vis-dev`, `R35-viscript.{md,json,json.license}` PASS `check_report`, `vis 0.4.5`. |

### Giai đoạn 3 — Kiến trúc Mỏng, macOS arm64 Tier 1, Pha 2 Ngôn ngữ, CLI Developer & ViShell MVP (`v0.5.0` — `VD-283`/`VD-284`)

| ID | Mục tiêu | Phụ thuộc | Ước lượng | Trạng thái | Ghi chú |
|---|---|---|---|---|---|
| ST-300b | **(W2) Tách `crates/vis-host-native` & `crates/vis-pkg`, làm mỏng `vis-cli` (`main.rs` < 900) & tách `lsp_it/lsp.rs` (0 đổi hành vi)** | ST-296b, ST-296d / VD-283 | 1,5 ngày | DONE | APPROVED 2026-11-23 Agent 0 (`VD-284`). Merge `770fdf7b`: tách `crates/vis-host-native` (13.107 LOC, 0 ref ngược, `#![forbid(unsafe_code)]`) + `crates/vis-pkg` (3.270 LOC), `vis-cli/src/` giảm 77,3% (21.147→4.809 LOC), `main.rs` 225 dòng, `lsp_it/lsp.rs` tách đôi < 760 dòng; 12 feature matrix xanh, targets core 18/18, diff 100k = 0 (`JOURNAL/st300b-host-pkg.md`). |
| ST-300m1 | **(W2) macOS Core Fixes (2 lỗi Darwin errno `ELOOP`/`EINPROGRESS`), kqueue tests, `bin/vis-gate-macos`, `vis-release` arm64 & đóng `#137`** | ST-300b / VD-284 | 1 ngày | DONE | APPROVED 2026-11-23 Agent 0 (`VD-285`). Merge `c3b72907`: vá Darwin `ELOOP`/`EXDEV`/`EINPROGRESS`, `VS0604` symlink APFS, `quyen_st174` + `quyen_st295` 0/100 trên APFS `mps`, đóng **`#137`**, `bin/vis-gate-macos`, `vis-release` & `vis-targets` arm64, gate Linux 3.672/4 (`JOURNAL/st300m1-mac-fixes.md`). |
| ST-300m2 | **(W2) Đo chuẩn độc quyền trên `mps` rảnh (`bin/vis-lock`): đóng nợ phần cứng `#147` & `#152`** | ST-300m1 / VD-284 | 0,5–1 ngày | DONE | APPROVED 2026-11-23 Agent 0 (`VD-285`). Merge `5741d234`: `mps` rảnh soak 10 vòng **379,7 s vs `ps` 517,1 s = 0,73×**, A/B 24 cặp `opt-bench` interp **1,0385** / VM **1,0626** — đóng **`#147`** & **`#152`** (`JOURNAL/st300m2-mac-bench.md`). |
| ST-300m3 | **(W4) Chứng nhận macOS arm64 Tier 1: 3× `vis-gate-macos --full` + Soak 4h trên `mps` + Cập nhật `HOST-BAO-DAM`/`ROADMAP`** | ST-300m1 / VD-284, VD-285 | 1 ngày | DONE | APPROVED 2026-11-23 Agent 0 (`VD-286`). Merge `f381c8f9` (`7930e906`): 3× `bin/vis-gate-macos --full` PASS (220/216/210 s; nextest 3.745/3/0, doc-test 5/0/1, host-wasm 29/29, WASI 28/28), soak 4h **1.878 vòng / 0 fail** (RSS 6.960–7.008 kB, fd 6/6, dirty 0), `vis-wasm-sem` **388/390**; công bố **macOS arm64 Tier 1** trên `HOST-BAO-DAM.md` & `ROADMAP.md` (`JOURNAL/st300m3-macos-tier1.md`). |
| ST-301 | **(W1) Đóng `#192` + Pha 2a: Xác nhận `T??` + Lambda param/return annotation + đóng `#155`** | ST-300 / VD-278, VD-283 | 3 ngày | DONE | APPROVED 2026-11-23 Agent 0 (`VD-284`). Merge `2bc6be74`: đóng `#192` (VM `--workers` reactor + ca cancel worker frame) & `#155` (lambda `(x: int?, y: str) -> int? => expr` + auto-lift interp↔VM + `T??` parity + 22 ca bảng); fuzz parser 1h **383.008.776 ca / 0 vi phạm**, diff 100k = 0 (`JOURNAL/st301-pha2a-lambda-annot.md`). |
| ST-302 | **(W1) Pha 2b: `async` method/`cmd` (`#13`) & `derives Json` codegen (`#8`)** | ST-301 / VD-278, VD-283, VD-284 | 4 ngày | DONE | APPROVED 2026-11-23 Agent 0 (`VD-285`). Merge `b79ae3ae` + `ST-302b` `6bc4ac35`: `async fn` trong `impl`, `async cmd`, `spawn obj.m()` parity interp↔VM, `derives Json` codegen thật (từ chối `cap`/`Chan`/`fn` bằng `VS0412`), đóng **`#8`** & **`#13`**, gate Linux 3.689/4. |
| ST-303 | **(W3) Công cụ CLI: `vis new`, `vis doc`, `vis repl`** | ST-300 / VD-278, VD-283 | 3–4 ngày | DONE | APPROVED 2026-11-23 Agent 0 (`VD-284`). Merge `a114e389`: `cli/{new,doc,repl}.rs`, 8/8 ca mới (`cli_it/st303_cli_dev.rs`), gate Linux **3.671/4**, 0 `unsafe` (`JOURNAL/st303-cli-dev.md`). |
| ST-304 | **(W3) ViShell MVP phần A (`crates/vish-edit`): Soạn dòng, CST coloring, Lịch sử & Ctrl-C (3-OS)** | ST-303 / VD-278, VD-281, VD-283, VD-284 | 3 ngày | DONE | APPROVED 2026-11-23 Agent 0 (`VD-285`). Merge `8e73e81f`: `crates/vish-edit` raw mode Unix `rustix`/Windows `crossterm`, multiline, CST coloring, fuzzy history, Ctrl-C 130, ấm 50 µs, 0 `unsafe` (đấu nối `vis repl` ở `ST-310b`). |
| ST-304b | **(W3) ViShell MVP phần B: Gợi ý tab theo kiểu, Job control POSIX (Linux + macOS) & Bộ test PTY giả** | ST-304 / VD-278, VD-281, VD-284 | 3 ngày | DONE | APPROVED 2026-11-23 Agent 0 (`VD-285`). Merge `1eb2768c`: completion theo hệ kiểu + job control POSIX (`&`/`jobs`/`fg`/`bg`/`kill`) + bộ test PTY giả (17/17 PASS, gate 3.688/4, 0 `unsafe`). |
| ST-305 | **(W2/W3/W4) Hoàn tất LSP (`vis-lsp`), Extension VS Code mỏng & Release `v0.5.0` (3-OS Tier 1)** | ST-300b, ST-300m3, ST-302, ST-304b / VD-278, VD-281, VD-283, VD-284, VD-285 | 3 ngày | DONE | APPROVED 2026-11-23 Agent 0 (`VD-286`). Tag **`v0.5.0` @ `c06d7c6f`**: LSP hover/completion, `editors/vscode/`, 3-OS Tier 1 (Linux 3.754/4 ×3, Windows 3.701/0/4, macOS arm64 Tier 1), WASI 28/28 + 388/390, `R36-viscript.{md,json,json.license}` PASS `check_report` (`JOURNAL/tag-v050.md`). |

### Giai đoạn 4 — G5 Hệ sinh thái: Plugin Rust/WASI ABI, ViTerm Protocol & `v1.0.0-rc1` (`v0.6.0` → `v1.0.0-rc1`)

| ID | Mục tiêu | Phụ thuộc | Ước lượng | Trạng thái | Ghi chú |
|---|---|---|---|---|---|
| ST-306 | **(W1/W2) Plugin tĩnh Native Rust: `vis-plugin-abi` (`VIS_PLUGIN_ABI_V1`) & `vis-plugin-macros`** | ST-305 / VD-278, VD-281 | 5 ngày | DONE | APPROVED 2026-11-23 Agent 0 (`VD-285`). Merge `6093aef0`: `crates/vis-plugin-abi` + `crates/vis-plugin-macros` (`VIS_PLUGIN_ABI_V1`, 0 `unsafe`, truyền `cap[...]` tường minh, parity interp↔VM; tách mỏng guest ABI vs host ở `ST-310c`). |
| ST-307 | **(W2) Plugin động WASI/WASM có kiểm soát quyền (`vs3_plugin.rs`)** | ST-306, ST-299 / VD-278, VD-281 | 5 ngày | DONE | APPROVED 2026-11-23 Agent 0 (`VD-285`). Merge `ab6beb77` + `b5aa40b1`: plugin động WASI/WASM 3 lớp ABI v1, sandbox theo `cap[...]`, tắt cứng khi `--strict-caps`/`--sandbox`, `cli/plugin.rs`, Windows quick 6/6 WASI xanh, 0 `unsafe`. |
| ST-308 | **(W3) `Terminal` trait & Giao thức hiển thị có cấu trúc ViTerm (`VS3-8`)** | ST-304b, ST-305 / VD-278 | 4 ngày | DONE | APPROVED 2026-11-23 Agent 0 (`VD-285`). Merge `a9eef01f`: `crates/vish-term` (9 tệp, 1.548 LOC, `Terminal` + `AnsiPosix`/`WinConsole`/`ViTermFramed`, 0 `unsafe`, giữ R4; đấu nối `vish-edit`/`vis-cli` ở `ST-310b`). |
| ST-309 | **(W1..W4) Tổng Nghiệm thu, `docs/SPEC.md`, Kiểm toán Bảo mật & Tag `v0.6.0` → `v1.0.0-rc1`** | ST-307, ST-308, ST-310f, ST-311a..f, ST-313d / VD-272, VD-281, VD-285, VD-286, VD-288, VD-289 | 5 ngày | BLOCKED | **TẠM DỪNG VÔ THỜI HẠN (`VD-289`, 2026-11-24)**: hủy kế hoạch phát hành cũ (`v0.6.0`/`v1.0.0-rc1`, soak 24h, fuzz 8h, 3× full 3-OS, `R37`) theo chỉ đạo người dùng để tập trung hoàn thiện kiến trúc đích 5 tầng DAG (`ST-312a..ST-313d`). Chỉ mở lại khi kiến trúc hoàn tất và người dùng quyết định phát hành. |

### Giai đoạn 4.5 — Refactor Kiến trúc 4 Tầng & Tối ưu Cấu trúc Kho (`v0.5.5`, `VD-285`)

| ID | Mục tiêu | Phụ thuộc | Ước lượng | Trạng thái | Ghi chú |
|---|---|---|---|---|---|
| ST-310a | **(W1) Trích `crates/vis-core` (gỡ coupling `vis-interp` cho 4 crate), đóng `#112`, chia nhỏ `interp.rs`/`rt.rs` (0 đổi hành vi)** | ST-305 (`v0.5.0`) / VD-285 | 1,5 ngày | DONE | APPROVED 2026-11-23 Agent 0 (`VD-286`). Merge `23b91df7` (tag `v0.5.5`): `crates/vis-core` 5.807 LOC/0 dep ngoài, 4 crate 0 dep `vis-interp`, đóng **`#112`**, `interp.rs` 1.460→418, `rt.rs` 1.338→151, diff 100k=0, `vis-host-wasm` xanh. |
| ST-310b | **(W3) Đấu nối ViShell: `vish-term` ← `vish-edit` ← `vis-cli` (`vis repl` TTY + `--format viterm`, bỏ 4× trùng lặp render)** | ST-310a / VD-285 | 1 ngày | DONE | APPROVED 2026-11-23 Agent 0 (`VD-286`). Merge `5219f9bf` (tag `v0.5.5`): `vish-term` là tầng Terminal/Render duy nhất, nối `vish-edit` + `vis repl` TTY/non-TTY + `--format viterm`, 0 crate mồ côi, 0 `unsafe`, xanh 3 OS. |
| ST-310c | **(W2) Tách `vis-plugin-abi` (0 dep nội bộ, `wasm32` < 1 s) vs `crates/vis-plugin-host` & thống nhất `openat2` (xoá `cap_fs.rs`)** | ST-310a / VD-285 | 1 ngày | DONE | APPROVED 2026-11-23 Agent 0 (`VD-286`). Merge `3c919f66` (tag `v0.5.5`): `vis-plugin-abi` 351 LOC/0 dep (`wasm32` 0,22 s), `crates/vis-plugin-host` (max 601 LOC), xoá `cap_fs.rs`, dùng duy nhất `vis_host_native::quyen`, 0 `unsafe`. |
| ST-310d | **(W2) Làm mỏng `crates/vis-cli/Cargo.toml`: gỡ 7 dep OS/TLS/SQLite trực tiếp thừa (0 đổi hành vi)** | ST-310a, ST-310c / VD-285 | 0,5 ngày | DONE | APPROVED 2026-11-23 Agent 0 (`VD-286`). Merge `f59ebd83` (tag `v0.5.5`): `vis-cli/Cargo.toml` `[dependencies]` = 0 dep OS/TLS/SQLite trực tiếp, chỉ feature-forwarding, 12 tổ hợp feature xanh. |
| ST-310e | **(W4) Tái phân bổ & gom `vis-cli/tests/` (27 → ≤ 12 binary) về đúng crate chủ + cổng chống phình trong `bin/vis-dev`** | ST-310a, ST-310c, ST-310d / VD-285 | 1 ngày | DONE | APPROVED 2026-11-23 Agent 0 (`VD-286`). Merge `c08cea53` (tag `v0.5.5`): `vis-cli/tests/` 27 → **8 binary**, chuyển suite về 5 crate chủ, thêm cổng **R8** & **R9** (`r9-openat2-ngoai-le.txt` rỗng), 0 test bị mất. |
| ST-310f | **(W3/W4) Tổ chức thư mục con `vis-lsp`/`vis-host-native`, `bin/vis-targets` 100% workspace, vệ sinh `agent/`/`CHANGELOG.md` & tag `v0.5.5`** | ST-310b, ST-310e / VD-285 | 1 ngày | DONE | APPROVED 2026-11-23 Agent 0 (`VD-286`). Tag **`v0.5.5` @ `3d85bd2c`**: tổ chức `vis-lsp`/`vis-host-native`, `vis-targets` 38/38 phủ 18/18 crate, `CHANGELOG.md` 28 KB, dọn 3 script `JOURNAL/`, full QA 3-OS xanh (Linux 3.773/4 + release 3.778/0/5 + diff 5×100k=0, Windows 3.720/0/4, macOS 3.764/0/3 + WASI 28/28 & 388/390). |

### Giai đoạn 4.6 — Củng cố Bảo mật (`#195`, `#194`, `#190`) & Tối ưu Kiến trúc Đợt 2 Tiền-`v0.6.0` (`VD-286` / `VD-288`)

| ID | Mục tiêu | Phụ thuộc | Ước lượng | Trạng thái | Ghi chú |
|---|---|---|---|---|---|
| ST-311a | **(W1) Đóng blocker `#195` (VM `loop_ends` O(1) + `MAX_ITER_DEPTH` + `deadline` + watchdog) & `#190` (`pop_frame` trap) + tách `vm.rs`** | ST-310f (`v0.5.5`) / VD-286 | 1 ngày | DONE | APPROVED 2026-11-23 (`VD-286`). Commit `525f5054`: `Frame::loop_ends` O(1) + `MAX_ITER_DEPTH = 1.024` + `Vm::deadline` (`VS0501-TIME`) + watchdog `fuzz_mutate.rs` (seed `0x5EED_2026_1121` release **61.111 ca / max 4,55 s / 0 vi phạm**, từ > 101 phút); đóng **`#190`** (`pop_frame` trap `VS0501`); tách `vm.rs` 1.244 → 634 + `vm/exec.rs` 714 LOC (`#195` chờ đủ watchdog 2h trên `H4` ở Phase C để đóng sổ theo R37 §4.2). |
| ST-311b | **(W2) Đóng blocker `#194`: truy vấn metadata FS theo `fd` (`statat`/`openat`/`readlinkat`) trên Unix + ca đua 100k + tách `quyen/mod.rs`** | ST-310f (`v0.5.5`) / VD-286 | 1 ngày | DONE | APPROVED 2026-11-24 (`VD-288`). Commit `61bf4e1` + `0e7532b` (merge `ad961bc`): toàn bộ metadata FS Unix đi qua `rustix::fs` theo `fd` (0 `unsafe`, giữ R9); ca đua 100.000 vòng `rename`/`symlink` **0 rò** Linux `ps` (`ok_trong=97736`) & macOS `mps` (`ok_trong=91631`); cập nhật `HOST-BAO-DAM.md` (giữ caveat Windows); đóng **`#194`**; tách `quyen/mod.rs` 1.197 → 675; gate Linux **3.782/4** (`JOURNAL/st311b-metadata.md`). |
| ST-311c | **(W3) Tối ưu kiến trúc test toàn workspace: 1 fixture `vs3_tls`, dọn `vis-cli` dev-deps, gom 56 binary rời → ≤ 35, thêm `vis-core/tests/` & R8b** | ST-310f (`v0.5.5`) / VD-286 | 1 ngày | DONE | APPROVED 2026-11-23 (`VD-286`). Commit `a320e027`: hợp nhất 3 bản sao TLS thành 1 nguồn `test/fixtures/vs3_tls/`; chuyển `vs3_http_cancel.rs` → `vis-host-native/tests/http2_it/` & gỡ 7 dev-dep thừa khỏi `vis-cli/Cargo.toml`; gom 56 binary rời giảm tổng workspace **78 → 37 binary** (−52,6%); thêm `crates/vis-core/tests/core_it.rs`; mở rộng `R8` (`bin/vis-dev` ≤ 40 binary toàn workspace); `fmt`/`clippy`/R1..R9/self-test 17/17 PASS. |
| ST-311d | **(W4) Chia nhỏ 10 tệp > 1.150 LOC, gom 6 `builtin_*.rs` vào `vis-interp/src/builtin/`, thử `-zstack-size` WASI (`#193`) & bump `0.6.0`** | ST-311a..c / VD-286 | 1 ngày | DONE | APPROVED 2026-11-24 (`VD-288`). Commit `7383e58`/`7b7144b` (merge `415d205`) + bump `0.6.0` `478aa9f`: gom 6 `builtin_*.rs` vào `builtin/`; chia 10 tệp > 1.150 LOC về ≤ 1.150; thử `-zstack-size` WASI (xác nhận cần `--stack-size` trên `node:wasi`, giữ `#193` mở non-blocking); vá 2 kỳ vọng `core_it.rs` & mở nợ **`#196`** (`parse_duration`); bump `0.6.0` + cache key mới (`JOURNAL/st311d-split.md`). |
| ST-311e | **(W4) Vá 3 lỗi clippy Windows-only (`JobControl.next` dead_code + 2 test Unix-only) chặn soak Windows** | ST-311d / VD-288 | 0,25 ngày | DONE | APPROVED 2026-11-24 (`VD-288`). Commit `d76e257`, `76d2121`: `#[cfg_attr(not(unix), allow(dead_code))]` trên `JobControl.next` (`vish-edit/src/job.rs`), `#[cfg(unix)]` cho `completion_job.rs` & `security_it/st309.rs`; clippy `--all-targets -D warnings` sạch trên Windows (`JOURNAL/st311e-win-clippy.md`). |
| ST-311f | **(W4) Vá race harness soak Windows/macOS ở `lat0_suspend` (dùng binary `vis` đã build thay shim `cargo run`)** | ST-311e / VD-288 | 0,25 ngày | DONE | APPROVED 2026-11-24 (`VD-288`). Commit `db67eab`: `lat0_suspend::baseline_matches_current_vis` gọi trực tiếp binary `vis[.exe]` đã build thay vì shim `cargo run` (`JOURNAL/st311f-soak-race.md`). |

### Chương trình Tái kiến trúc Tối ưu 5 Tầng DAG (`VD-289` — Chế độ Phát triển, Không phát hành)

| ID | Mục tiêu | Phụ thuộc | Ước lượng | Trạng thái | Ghi chú |
|---|---|---|---|---|---|
| ST-312a | **(W1 · Sóng 0) Dọn 9 shim re-export `vis_interp → vis_core` & chuẩn hóa kiểu lỗi/config (`Abort`/`Trap`/`R`/`RunOutput`/`FrameConfig`/`FrameOutput`) vào `vis-core`** | ST-311f / VD-289 | 0,5 ngày | REVIEW | **XONG 2026-11-24**: 9 shim công khai đã dọn (2 tệp `cookie.rs`/`json.rs` xóa; 7 chỗ còn lại `pub(crate)`, API ngoài dùng `vis_core`); `RunOutput`+`FrameConfig`+`FrameOutput`+`FakeEvent`/`ReactorEvent`/`ReactorOutcome`/`DiagSnapshot` chuyển `vis-core`; `vis-vm` thêm dep `vis-core`, 0 import lõi qua `vis_interp`. Cổng: `gate --fast` PASS · `vis-diff 100k lech=0` (58,4 s · 1711/s) · `cargo check --workspace --all-targets` sạch. |
| ST-312b | **(W4 · Sóng 0) Đưa `vis-host-linux` vào workspace chính (1 `Cargo.lock`), gộp `clippy -D warnings` vào `bin/vis-gate-windows --full` & vá `#196` (`parse_duration`)** | ST-311f / VD-289 | 0,5 ngày | REVIEW | **XONG 2026-11-24**: `vis-host-linux` thành viên workspace (19 crate, `version.workspace`, `landlock` ở `[workspace.dependencies]`, xóa lock riêng); `bin/vis-targets` phủ 19/19 + `vis-host-wasm` ở `EXTRA` (đã sửa import `vis_core`, wasm32 xanh); `bin/vis-gate-windows --full` thêm clippy; **đóng `#196`** (`parse_duration("sai")` ⇒ `None` + ca `""`/`"250"`). Dọn kho: 144 nhánh merge + 7 nhánh nội dung đã hấp thụ (lưu patch `/home/ps/vislogs/cleanup-289/`), 3 worktree còn lại xóa, origin còn `main` + `plan/st309-qa`; `vis-gc-targets` OK (23G trống). Cổng: `gate --fast` PASS · `vis-diff 100k lech=0` · `vis-targets` PASS=39/37. |
| ST-312c | **(W3 · Sóng 1) Tách `crates/vis-hir` (dữ liệu thuần HIR/Type/`TypeTable`/bảng chữ ký `stdlib`) khỏi `vis-types`** | ST-311f / VD-289 | 1–1,5 ngày | REVIEW | **XONG 2026-11-24**: `crates/vis-hir` (hir, ty, symbols, cmdsig, method_ret, prelude, stdlib) — 20 crate workspace; `vis-types` chỉ còn `lower` + `tyck` (+ re-export `HirOutput`/`LowerOptions`/`TraitSig`); `vis-core`/`vis-bytecode`/`vis-vm` **0 dep `vis-types`** (bytecode/vm chỉ dev-dep cho test hạ HIR). Test `MethodRet` dời `vis-types/tests/types_it/method_ret.rs` (tránh dev-cycle). Cổng: `gate --fast` PASS · `vis-diff 100k lech=0` · `vis-targets` PASS=41/39 FAIL=0. |
| ST-312d | **(W2 · Sóng 1) Tách `crates/vis-http` (sans-IO HTTP/1.1 + cookie + redirect + async state machine) & cắt `vis-pkg → vis-host-native`** | ST-311f / VD-289 | 1–1,5 ngày | READY | Di chuyển `vis-core/src/http_client/` + `vis-interp/src/net/async_http.rs` sang `crates/vis-http` (chỉ phụ thuộc `vis-core`/`NetHost`); thay `vis_host_native::net::Net` ở `vis-pkg/src/pkg/registry_http.rs:46` bằng tiêm `&mut dyn NetHost`; thêm unit test mock cho `vis-pkg`. |
| ST-313a | **(W1+W3 · Sóng 2) Tách `crates/vis-runtime` khỏi `vis-interp`, cắt `vis-vm → vis-interp` & chia `vm_frame.rs` < 1.000 LOC** | ST-312a, ST-312c, ST-312d / VD-289 | 2 ngày | REVIEW | **XONG 2026-11-24**: `crates/vis-runtime` (scheduler/chan/select/fee/reactor/frame-test/worker-pool + `builtin`/`stdlib`/`render`/`lit`/`lift`); `vis-interp` còn **8.820 LOC** (tree-walker + frame + orchestration), `vis-vm` **0 dep `vis-interp`** (chỉ dev-dep parity) và phụ thuộc `vis-runtime`+`vis-bytecode`+`vis-hir`; `vm_frame.rs` 1.164 → **952 LOC** (`vm_frame/budget.rs`). Cổng: `gate --fast` PASS · `vis-diff 100k lech=0` · `async_it` 339/339 · `net_pkg_it` 156/156 · `vis-targets` PASS=45/43 FAIL=0. | Chuyển `builtin/`, `stdlib/`, `rt/`, `render.rs`, `host.rs`, worker proxy sang `crates/vis-runtime`; tách worker tree-walker qua trait `WorkerRunner` về `vis-interp` (~8,6k LOC); `vis-vm` chỉ phụ thuộc `vis-runtime` + `vis-bytecode` + `vis-hir` (**0 dep `vis-interp`**); `gate --fast` + `vis-diff 100k = 0`. |
| ST-313b | **(W2+W4 · Sóng 2) Một nguồn sự thật stdlib (`StdId` O(1) + song ánh 1–1) & tổ chức `stdlib/` theo miền (0 tệp `_a.._e`)** | ST-313a / VD-289 | 1,5 ngày | REVIEW | **XONG 2026-11-24**: `vis_hir::stdlib::StdId` sinh từ **167** chữ ký (`COUNT/ALL/as_str/from_name/index`); `vis-runtime::stdlib::call` tra id một lần rồi `match StdId` vét cạn (`call_by_id`, 0 so chuỗi trong nhánh); test song ánh `FUNCS ↔ StdId` (2 ca). `stdlib/` chia miền: `str/{scan,parse,replace,split,trim}`, `sqlite/{pragma,introspect,index,schema,explain}`, `path/{join,normalize,ext,dispatch}`, `list/{math,stats}` — **0 tệp `_a.._e`**. Cổng: `gate --fast` PASS · `vis-diff 100k lech=0` · `vis-runtime` 283 test · `async_it` 339/339 · `net_pkg_it` 156/156. | Chia lại 20 tệp `str_a..e`/`sqlite_a..e`/`path_a..d`/`list_a..b`/`fs_net_d` thành thư mục theo miền ngữ nghĩa; định nghĩa `StdId` trong `vis-hir` + exhaustiveness `match` trong `vis-runtime` + test tự động khẳng định song ánh 1–1 `FUNCS` ↔ dispatch (`std.*` + `builtin`). |
| ST-313c | **(W1+W2 · Sóng 3) Dời test API thư viện khỏi `vis-cli/tests/` & dựng `crates/vis-conformance` (harness dữ liệu `interp ↔ VM` + WASI)** | ST-313a, ST-313b / VD-289 | 1,5 ngày | REVIEW | **XONG PHẦN HARNESS 2026-11-24**: `crates/vis-conformance` đọc `test/conformance/**` (`.vis` + `.out`/`.err`/`.exit`), chạy thẳng API thư viện **cả interp và VM** (không cần binary), hỗ trợ ca lỗi tĩnh (`.err` = mã parse/lower/tyck). Test `corpus` chạy **415/415** ca. Tệp gốc `vis-cli/tests` **giữ nguyên** (0 assertion mất) — việc xóa tệp gốc chờ A0 duyệt. | Chuyển toàn bộ test gọi API thư viện (`vis_interp`/`vis_vm`/`vis_syntax`/`vis_runtime`/`vis_http`) về crate chủ; tạo `crates/vis-conformance` đọc `test/conformance/**/*.vis` (+ `.out`/`.err`/exit) tự động chạy cả `interp` & `VM` (tái dùng cho WASI). |
| ST-313d | **(W3+W4 · Sóng 3) Chuyển suite hành vi `lang_it`/`stdlib_it` sang `test/conformance/`, rà R1..R9 & đánh giá đóng `#193`** | ST-313c / VD-289 | 2 ngày | REVIEW | **ĐANG LÀM 2026-11-24**: đã trích **297 ca hành vi** (`dung`) + **118 ca lỗi tĩnh** (`tu_choi`) từ `lang_it`/`stdlib_it` thành dữ liệu — tổng **415 ca** xanh trên cả 2 engine; R1..R9 PASS; `#193` **vẫn mở** (WASI sem 388/390, 2 ca đệ quy sâu). Còn: xóa/giảm tệp gốc sau khi A0 duyệt để tránh trùng lặp. | Di chuyển các ca hành vi thuần từ `vis-cli/tests/{lang_it,stdlib_it}` sang `test/conformance/` (**0 assertion mất**, tổng ca ≥ trước); cập nhật `bin/vis-dev` R8 nếu cần; kiểm 3-OS clippy + `vis-targets` 100% + đánh giá đóng `#193` (trampoline/stack WASI trên `vis-interp` mỏng). |