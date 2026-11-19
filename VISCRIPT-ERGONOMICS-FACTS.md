# ViScript — Dữ liệu công thái học (Agent 0) · FACTS

> **Agent 0 · dữ liệu thu thập, 2026-11-19.** Theo yêu cầu A0 "cảm nhận công thái học ViScript" — **KHÔNG phải thiết kế mới, KHÔNG đổi ngữ nghĩa/API/golden/SPEC.**
> Mọi nhận định về implementation kèm bằng chứng `file:symbol` hoặc `file:line` tại **HEAD `6fb72a7`** (`main`, 2026-11-19).
> Bốn trạng thái **không dùng nhãn chung**: **(1)** đã triển khai & kiểm chứng · **(2)** đã triển khai nhưng giới hạn · **(3)** đã quyết định chưa triển khai · **(4)** đang đề xuất chưa duyệt.
> Đo lại (repro) bởi Agent 1: `int??` tái lập độc lập (xem `agent/DUNG-CHO-A0.md` + `VISCRIPT-ERGONOMICS-DIAGNOSTICS.md`); các ca khác giữ nguyên bằng chứng của Agent 2.
> Các ca lệch hai engine được đánh dấu **[LỆCH ENGINE — BLOCKED-CHO-A0]** — không sửa, xem `agent/DUNG-CHO-A0.md` + câu hỏi cuối file.
>
> **CẬP NHẬT 2026-11-19 (sau khi A0 chốt ngữ nghĩa):** A0 đã ratify Option lồng — `T??` là `Option[Option[T]]` thật; hệ thống corpus đã được cập nhật thành **483 ca** (kéo thêm `long-*`); các lệch-byte (§6.1/6.2) đã được sửa theo đúng quyết định A0 — chi tiết xem `agent/QD-OPTION-LONG-2026-11-19.md` + `agent/DEM-2026-11-18.md` (~13:15). Các câu hỏi §9 của bản báo cáo gốc đã được A0 trả lời theo đúng thứ tự; các số cũ ở §3/§4/§5 dưới đây giữ nguyên lịch sử lúc 6fb72a7.

## 1. Mục tiêu & phạm vi

A0 cần phân biệt "có cú pháp gọn" với "thực sự dễ học, dễ viết và ít gây bất ngờ". Tài liệu này ghi lại **hiện trạng chạy thật**, không tô hồng:

- Người mới phải học và nhớ bao nhiêu quy tắc? (§3)
- Cùng một ý định có cách viết rõ ràng, nhất quán không? (§3, §6)
- Code ngắn có còn dễ đọc và dễ đoán hành vi không? (§3, §4)
- Khi kết hợp tính năng, có phát sinh ngoại lệ không? (§5 — chẩn đoán, §6 — ergonomics)
- Viết sai, người dùng có tự sửa được từ thông báo lỗi không? (§5, DIAGNOSTICS file)
- Phần nào nên giữ / đơn giản hóa / bổ sung / thiết kế lại? → **không tự trả lời ở đây** — chỉ ghi ứng viên, kèm chi phí để A0 quyết (§6).

**Không thuộc phạm vi**: sinh golden, đổi ngữ nghĩa, sửa checker/parser, redesign.

## 2. Ảnh chụp trạng thái hiện tại (2026-11-19)

### 2.1 Branch / commit / máy kiểm chứng

| Mục | Giá trị |
|---|---|
| Branch | `main` @ **`6fb72a7`** (== `origin/main`, cây git sạch lúc chụp) |
| HEAD pha 1 đóng chính thức | `pha1-rc1` = `05e42d01483c255987750656fa6cba6c434831c9` (tuyệt đối không di chuyển) |
| Máy đo | `ps` — Linux 7.0.0-43-generic x86_64 (Ubuntu), 12 lõi, `/home/ps/ViScript` |
| Trạng thái tải `ps` | dao động 0,07 → 3,6 (7 user chung máy); đo bench tại 0,07 |
| Engine được so từng ca | `vis run` (interp, mặc định) và `vis run --vm` (VM VBC1) |
| Binary kiểm chứng | `target/debug/vis` (mtime 2026-11-19 11:32) |
| Phiên bản `vis` | không báo version (chưa có `vis --version`; `vis help` ra usage) |

### 2.2 Lệnh build / check / run tối thiểu

```sh
# Người dùng cuối (README quickstart, dùng được nếu toolchain đúng):
git clone https://github.com/vifast-oss/ViScript.git && cd ViScript
./vis run vi-du/xin-chao.vis          # quickstart — cần toolchain đúng (xem 2.3)

# Với máy contributor, đúng convention repo:
export PATH="$HOME/.cargo/bin:$PATH"
./vis-dev check                        # fmt + clippy + luật + self-test + build + doc-examples
./vis run path/to/prog.vis             # interp
./vis run --vm --no-cache path/to/prog.vis
./vis check path/to/prog.vis           # TAP; --json cho máy đọc
./vis test test/sem                    # 473 ca, ~1,5 s
./vis test --vm test/sem               # 473/473 hai đường
./vis explain VS0412                   # dịch mã lỗi (vi mặc định; en/ja có)
```

- `./vis-dev gate --fast` mới là cổng merge (10/10). `vis fmt` không nằm trong `vis-dev check` (chỉ `nightly` vòng tròn fmt).
- `./vis` là script `cargo run -p vis-cli` — nghĩa là **mỗi lần chạy cần sẵn artifact build**.

### 2.3 Toolchain & tái lập (thực tế đo được)

- `rust-toolchain.toml`: `channel = "stable"` (float) + components `rustfmt`, `clippy`.
- `Cargo.toml` gốc **không** khai `rust-version`; `crates/vis-host-linux/Cargo.toml` (path-dep của `vis-cli`) khai **`rust-version = "1.99.1"`** ⇒ `vis-host-linux@0.1.0` trong đồ thị resolve.
- Máy `ps` có **hai** cargo: `/usr/bin/cargo` (rustc **1.93.1**, source tarball, không rustup) và `~/.cargo/bin/cargo` (rustup, rustc **1.99.1**; toolchains được cài: `1.96`, `1.98.1`, `1.99.1`, `stable-1.99.1`).
- **Với** `~/.cargo/bin` đầu PATH (convention mọi script trong repo + mọi lệnh đo trong sổ sách này): mọi build chạy được; `./vis-dev check` **PASS**.
- **Với** PATH mặc định (fresh-clone tay, `find_cargo` của `bin/vis-dev` ưu tiên `command -v cargo`): `vis-host-linux` yêu cầu ≥ 1.99.1 ⇒ lỗi resolve `rustc 1.93.1 is not supported` (exit 101) ngay lập tức; `./vis-dev check` **FAIL** ở 6 self-test + doc-examples; README quickstart **thất bại** (dòng `./vis run` gọi `cargo run`). Xem `VISCRIPT-ERGONOMICS-FACTS.md §7` và câu hỏi cho A0 ở §6.
- `Cargo.lock` commit ở gốc (+`vis-host-linux/Cargo.lock`, `vis-host-wasm/Cargo.lock`).
- **Không có CI** (không `.github/workflows`, không workflow ở `infra/` — `infra/linux/Dockerfile` pin đúng `rust:1.99.1-bookworm` nhưng tự khai "CHƯA DÙNG").

### 2.4 Nguồn chuẩn khi spec / implementation / quyết định khác nhau

Theo `AGENTS.md` §1 (repo ViScript): **code + test > `SPEC-lex.md` > DECISIONS.md > docs thiết kế**. Áp dụng trong tài liệu này ở mọi chỗ mâu thuẫn (ví dụ: VD-228 văn bản nói "đối số chưa nâng" nhưng code biên hàm có lift — **code thắng**; `chan(0)` VD-178 QĐ-2 "hợp lệ rendezvous" nhưng mã trả `VS0505` runtime — **code thắng**).

### 2.5 Bốn trạng thái — bảng tổng (chi tiết + bằng chứng từng mục ở §3)

| Trạng thái | Ví dụ tiêu biểu | Nhãn trong tài liệu |
|---|---|---|
| **(1) Đã triển khai & kiểm chứng** | `if let`/`while let` (R3a/R18b), `filter`/`map` trả collection, `match` non-void arms, auto-lift `T→T?` 1 tầng (VD-227/228), kênh cap ≥ 1 + FIFO | "kiểm chứng: <test>" |
| **(2) Đã triển khai nhưng giới hạn** | `async`/`spawn` một luồng run-to-completion (VD-240 grid), `chan(0)` bị chặn (`VS0505`), closure **không annot được** tham số, `vis check` không bắt lỗi runtime, thiếu `panic`, không stack-trace frame, không highlighting/LSP-rename | "giới hạn" |
| **(3) Đã quyết định nhưng chưa triển khai** | VS3-scope: async song song thật, rendezvous `chan(0)`, `select`/timeout, TLS/HTTP2, `T??` (hai nguồn spec mâu thuẫn — chưa chốt), JSON `VS0111` đã ngừng phát (VD-212) | "chưa triển khai" |
| **(4) Đang đề xuất chưa duyệt** | `#151` doc-examples slug, `#149` split-path toàn cục, `lambda annotation`, `#155` lift params closure, `T??` chốt nghĩa, async (a) runtime mới — `agent/ROADMAP-PHA2-NHAP.md` A0-1..A0-8 | "đề xuất" |

> Ghi chú quy ước: "golden 473 ca" thuộc trạng thái (1) — mọi ca `test/sem/**` đều chạy thật; bốn grid async VD-240 là ca mới nhất (2026-11-19).

---

## 3. Bản đồ cú pháp & ngữ nghĩa (tại HEAD `6fb72a7`; mỗi mục có bằng chứng)

> Nguồn đối chiếu: `docs/SPEC-lex.md` (từ vựng + EBNF), `docs/ViScript-thiet-ke.md`, `CHEAT-SHEET-ViScript.md` §13, mã nguồn `crates/vis-types/src/tyck/*` (kiểu), `crates/vis-syntax/` (parser), `crates/vis-interp/` (interp), `crates/vis-bytecode/` (VM VBC1), `test/sem/**` (473 ca, chạy hai engine ×2 — **trừ các ca `int??`, xem §6**).
> Cấu trúc mỗi nhóm: **cú pháp · ý nghĩa · ví dụ tối thiểu · giới hạn · bằng chứng file:line/test**.

### 3.0 Nhận xét tổng của người thu thập (để A0 đọc, không phải kết luận)

- Bề mặt cú pháp thật sự nhỏ: khoảng **13–16 quy tắc lõi** (let/var, annot, fn, match, Option/Result, list/map/record/enum, closure 1 tầng, trait dụng được, module, script/shell).
- Nhưng **độ tin cậy của các vùng kết hợp** không đồng đều: full corpus `test/sem` 473/473 ×2 engine xanh, **trong khi** hai engine lệch byte ở `int??` (§6.1) và VM mis-compile **pattern ctor lồng** (§6.2).
- **`vis check` không chặn nhiều lỗi chỉ nổ runtime** (§6.3) — người mới dễ hiểu rằng "check xanh = code đúng".
- Toolchain fresh-clone không tái lập trên máy `ps` nếu không theo convention PATH (§2.3, §7).

### 3.1 Biến & phạm vi

**Cú pháp · Ý nghĩa**: `let x: T? = …` · `var x = …` · `const N: int = 10`; block-scope; shadowing (`let` che tên trong cùng phạm vi — R16); `var x =` trùng ⇒ **`VS0302`**; gán `let` ⇒ **`VS0413`**; gán lặp trên `var` OK; `for` scope riêng.

**Ví dụ**: `let x = 1; let x = "a"` (che tên hợp lệ) — `test/sem/pham-vi/che-ten-let.vis`.

**Giới hạn**: `p.x += 1`, `xs[0] += 1` ⇒ `VS0201`; gán lồng `x.f[i][j]` ⇒ `VS0412`; closure **snapshot** biến lúc tạo (VD-51); `xs[0] = v` OK.

**Bằng chứng**: `test/sem/pham-vi/` (19 ca), `vis explain VS0413`.

### 3.2 Kiểu & suy luận

**Cú pháp · Ý nghĩa**: annot `let x: T = e`, `const T`, params/return; literal int⇒float **chỉ với annot float** (`is_int_lit`); `T?` = `Option[T]` enum std thật (VD-208); `Result[T,E]`; type alias `type Id = int` (R12a); newtype `type Email(str)` trong suốt; `AnyError` = alias `Error` (VD-231); **không union, không subtyping**; generic `[T]` xoá kiểu runtime.

**Ví dụ**: `let x: int = 1`

**Giới hạn**: `1 < x < 10` không hợp lệ (so sánh không kết hợp); annot `T??` hợp lệ nhưng **phẳng** (§6.4); không `float.inf`/`float.nan` (D-4); generic bound `[T: Sz]`, `[R: has {…}]` **parse nhưng không kiểm**; method call trên generic param ⇒ `VS0403`; literal `0.0/0.0` ra `NaN` print được.

**Bằng chứng**: `tyck/{mod,vd238}.rs`, `test/sem/generic/` (20 ca), `docs/SPEC-lex.md` §2.

### 3.3 Option / T? / Some / None / auto-lift

**Cú pháp · Ý nghĩa**: `T?`; `Some/None` (viết hoa cả dựng lẫn pattern; viết thường ⇒ **`VS0203`**); `x ?? d` (short-circuit); `u?.f`; `if let Some(v) = o:` / `while let …` (R18b); lift `T → T?` ở biên `let`/return/arg/closure-return (VD-227/228/229); pattern `Some` khớp giá trị chưa bọc.

**Ví dụ**: `let o: int? = Some(1)`; `print(o ?? 7)`

**Giới hạn**: lift **một lớp**; annot `{q =>}` không hợp lệ; `xs[i]?`/`m[k]?` trong fn ⇒ `T?` (R8), **ngoài fn ⇒ `VS0501` lúc chạy** (tyck không chặn); `map.get` ⇒ `T?`; **checker phẳng `int??`** (§6.4); annot lambda params không hỗ trợ (A0-3).

**Bằng chứng**: `test/sem/optional/` (21 ca); `pat.rs:54`; VD-227/228/229 (DECISIONS).

### 3.4 Lỗi — Result / truyền / bắt / tạo / panic

**Cú pháp · Ý nghĩa**: `Ok/Err`; `?` trong fn; `xs[i]?` (R8); `let v = r else e:` (R3b) vs `let v = r else:`; `if let/while let` cho Result; Error record `{message, source: Error?, span: str?}`; `AnyError` = alias `Error` (VD-231); trap ⇒ exit **101**: `VS0501` (runtime), `VS0502` (÷0), `VS0503` (index), `VS0505` (kênh), `VS0507` (tràn); `[quyền]` ⇒ exit **2**.

**Ví dụ**: `let v = f()?`; `match fs.read(p): Ok(s): … / Err(e): …`

**Giới hạn**: `f() else 0` inline **không có** (chỉ `let … else`); std `?` không kiểm biến thể E (trade-off VD-222); `panic` **không có**; `String/int` không ép; trap không kèm stack-trace frame (§7).

**Bằng chứng**: `test/sem/result/` (20 ca), `std/json` decode trả `Result[Json, Error]`, VD-222/225 (DECISIONS).

### 3.5 Điều khiển

**Cú pháp · Ý nghĩa**: `if/elif/else` (biểu thức khi có `else`); `match` (guard không tính coverage → `VS0402`; or-pattern `A(x) | B(x)`; exhaustive; thiếu nhánh ⇒ `VS0402` + nêu tên nhánh thiếu); `for x in range/list/str/map/(k,v)`; `while`, `while let …` (R18b); `loop`, `break`, `continue`, `return`.

**Ví dụ**: `if x > 0: print("a") elif … else:`

**Giới hạn**: range chỉ `int` (`size`/`duration` ⇒ **`VS0416`**); `1 < x < 10` ⇒ `VS0201`; `else` bắt buộc với `let…else`.

**Bằng chứng**: `test/sem/dieu-khien/` (19), `match-nang-cao/`; SPEC-lex §4.

### 3.6 Hàm, closure, lambda

**Cú pháp · Ý nghĩa**: vị trí/tên/mặc định (`fn f(a: int, b: int = 2)`, gọi `f(1, b: 5)`); **cờ boolean** `--x: bool` hợp lệ mọi fn (VD-224); fn làm giá trị; lambda `x => …`, `(a, b) => …`; multi-line `fn(x: int) -> int: suite`; generic `fn id[T]`; recursion ≤ 2000 khung.

**Ví dụ**: `fn add(a: int, b: int) -> int: a + b`

**Giới hạn**: **tham số lambda không annot được** (`(x: int?) =>` ⇒ `VS0201`; `x => x * 2` OK); closure params không nâng (#155 VS3); method gọi tĩnh chỉ record/enum (`impl Trait for int` không gọi được); lambda không `return` ưu tiên (multi-line thân chạy).

**Bằng chứng**: `test/sem/ham/` (25), `test/sem/generic/`; VD-224/232 (DECISIONS); `lower/trait_impl.rs`.

### 3.7 Dữ liệu — list / map / record / tuple / enum / destructuring

**Cú pháp · Ý nghĩa**: `[1, …]`; `{"a": 1}` map; `{a: 1}` record; `{}` map rỗng; `{(k): 1}` map computed (VD-215); `(1, "a")` tuple; `set{…}` (list khử-trùng literal — **không phải kiểu Set**, không `in`); `type U:` + `{f: T}` + `U(f: v)` (R11); `enum S: Circle(float) | Empty`; dựng `S.A(1)` / `A(1)` (namespace `S.` cần với nullary); update `u with f: v` (record update); destructure `let (a, b) = t`, `for (k, v) in m`; **không record pattern**.

**Ví dụ**: `let xs = [1, 2]`

**Giới hạn**: record literal **đủ trường & đúng thứ tự khai báo** (`same_record` zip — `{b:…, a:…}` ⇒ `VS0412`); default `age: int = 0` **không tự áp**; index OOB ⇒ **`VS0503`** exit 101; `xs[0] += 1` ⇒ `VS0201`; không slicing; tuple print như list (`(1,"a")` → `[1, a]`); enum match đầy đủ nhánh bắt buộc (`VS0402`).

**Bằng chứng**: `test/sem/{ban-ghi,enum,map,list,long-du-lieu}/`; `tyck/mod.rs:432-445`; VD-215 (DECISIONS).

### 3.8 Trait / impl / method resolution / generic constraints

**Cú pháp · Ý nghĩa**: `trait T: A:` siêu trait 1 tầng; `derives Eq/Ord/Hash/Show/Json/Default`; `impl T for Q:` static dispatch record/enum; builtin `Show`, `Error`; `dyn` không có.

**Ví dụ**: `type P derives Show: …`

**Giới hạn**: bound generic không kiểm (`[T: Sz]` parse, không enforce); method resolution tĩnh chỉ record/enum; supertrait 1 tầng; body impl chưa tyck đầy đủ; impl **cục bộ theo module** (xuyên module phải bọc `pub fn` — xem EXAMPLES bài 7); quy tắc mồ côi (`impl` phải ở tệp khai trait hoặc kiểu — VD-98); `derives Eq` trên float ⇒ `VS0403`; `impl Trait for int` đăng ký nhưng không gọi được.

**Bằng chứng**: `test/sem/trait/` (13); `lower/trait_impl.rs:33`; VD-98 (DECISIONS); EXAMPLES (giao diện impl xuyên module).

### 3.9 Module — import / export / alias / visibility

**Cú pháp · Ý nghĩa**: `use std.x`, `use ./x` (tương đối), `use pkg.x`; `pub`; không `as`; cấm vòng (`VS0702`); `use std.str` + `let str =` ⇒ cảnh báo **`VS0211`** (member position rơi về module std — §6.6).

**Ví dụ**: `use ./geometry`

**Giới hạn**: không alias; visibility per-tệp; missing `pub` ⇒ `VS0301 (gợi ý "gắn pub")`; registry cục bộ kiểu Go **chưa triển khai**; cross-file pub cần cùng dep-hash; `use pkg.*` kéo theo strict-caps (VD-124).

**Bằng chứng**: `test/modules/` (9); VD-57/58/124 (DECISIONS); `lower/expr.rs:251-263` (§6.6).

### 3.10 Script & shell (`.vish`) — điểm vào, args, exit, quoting

**Cú pháp · Ý nghĩa**: `.vis` + `fn main` ⇒ **top-level statements bị bỏ qua**, chỉ `main` chạy, argv vào tham số không-cap đầu; exit: `0` ok · `2` lỗi front-end/quyền · `101` trap (bảng `cli/run.rs::ma_thoat_khi_loi`); `fn main(): int:` **mã trả bị bỏ qua** (exit 0); `.vish` ⇒ pipeline cấp tệp chạy (thụt lề giữ nguyên), `fn main` chỉ khai cap (HIR bỏ main — #69); `cap[Dir] @ p"." (write)`; `--strict-caps [mặc định tắt]` — bật khi `--strict-caps` hoặc tự động khi `use pkg.*`; `--grant tên=path`; `^prog` ⇒ `VS0606`; `| complete` chỉ sau chương trình ngoài (print `{stdout, stderr, code}`, không bind biến); `&` ⇒ `VS0604`.

**Ví dụ**: `echo hi | take 1` (test/sem/vish/) · `fn main(args: list[str]): …` + `vis run x.vis -- a b`

**Giới hạn**: shell-let `out` không bind (#1); `.vish` fmt giữ thụt lề; **không glob**; `^/bin/echo` và `extern cmd` **không chạy được** trong `.vis` (`VS0201`/`VS0501`) — xem DIAGNOSTICS §tổng hợp; strict-caps mặc định tắt; **exit code của `main -> Err` không nối dây** (exit 0 dù Err — VD-178 R4 `[DỰ KIẾN]` chưa hiện thực).

**Bằng chứng**: `test/sem/vish/` (50 ca); `cli/run.rs`, `main.rs`; VD-178 R4; DIAGNOSTICS ca 6; EXAMPLES bài 5, 6.

### 3.11 Thư viện chuẩn

**API-STD.txt (đóng băng, 64 dòng)**: `std.fs` (exists/list/read/write), `std.list` (sort/sort_by/sort take/drop/chunk/concat/reverse/unique), `std.map` (keys/values/merge/remove/get_or), `std.str` (split/join/upper/lower/trim/replace/repeat/starts_with/ends_with/contains), `std.path` (join/basename/dirname/ext/stem/is_abs), `std.time` (now_ms/monotonic_ms), `std.json` (encode/encode_pretty/decode → `Result[Json, Error]`, VD-222/225), `std.net` (loopback only: listen/local_addr/accept(_timeout)/connect(_timeout)/read/write/close/close_listen), `std.http` (get/head +_timeout, HTTP/1.1 loopback, chunked, redirect GET ≤5, **không TLS/keep-alive**).

**Method trên giá trị (không cần `use`)**: list (`len is_empty get first last push map filter each sum join enumerate any all find`), str (`len is_empty upper lower split chars contains`), map (`len is_empty get has keys values`).

**Giới hạn**: **không có** regex · bytes · env · process · crypto · thread · Iterator · `int(str)`/`float(str)` (parse tay) · `fs.lines` · `str.find`/slice · Set · TLS.

**Bằng chứng**: `API-STD.txt`; `crates/vis-interp/src/stdlib.rs`; `crates/vis-types/src/stdlib.rs`; EXAMPLES bài 3 (JSON), bài 4 (str tay), bài 2 (parse số tay).

### 3.12 Runtime — mô hình thực thi & hai engine

**Kiến trúc**: hai engine dùng chung frontend/checker/value/builtin (`builtin::{bin,un,call,method}`; `value::{show,eq,cmp,iter_items}`) ⇒ byte-identical trên corpus `test/sem` (473/473 ×2 — nhưng **không** ở `int??`/nested pattern, §6.1–6.2 và `DUNG-CHO-A0`); VM: VBC1 đóng băng, không unsafe, cap 2.000 khung, `MAX_STEPS` 10⁷; interp: eval trực tiếp; **scheduler một luồng run-to-completion** (VD-240); kênh FIFO; `Chan` không qua biên host; `ref[T]` identity; **không GC**. Differential harness so stdout+exit+(mã, span), **không so `steps`** (đơn vị khác).

**Giới hạn**: `./vis run` exit 101 thay vì 1 khi trap; VM exit 101 codegen lỗi ⇒ exit 1; `vis fmt` vòng 1 only; panic chưa có; steps không so được ngữ nghĩa; steps không nguồn liên kết nếu lỗi frontend.

**Bằng chứng**: `vis-vm/`, `vis-bytecode/`, VD-52..56 (DECISIONS); nightly 2026-11-19 ~11:44: fuzz parser 120 s ⇒ **19.179.393 ca, 435.324.519 token, 0 vi phạm** (nằm trong sổ `agent/DEM-2026-11-18.md`).

### 3.13 Các câu hỏi đặc thù (truthiness… trọng lượng đánh giá)

| # | Câu hỏi | Trả lời (bằng chứng) |
|---|---|---|
| 1 | **Truthiness** | **Không có**. Chỉ `bool` ở `if/while/and/or/not`/guard/`map`. `if 1:` / `if "":` / `if []:` / `if None:` / `if Some(1):` ⇒ **`VS0412`** (list/map/str được gợi ý `.is_empty()` — `coerce_cond`, `tyck/mod.rs:499`). Runtime `truthy()` có chấp Option nhưng bị gate tĩnh chặn (`interp.rs:1371`). |
| 2 | **Equality** | `int == float` promote cả hai chiều; `percent == float` theo tỉ lệ; khác kiểu ⇒ **`VS0412`** (`"a" == 1`); record/list/map/enum so **giá trị**; enum so **cả tên enum** (`Option.Some(5) ≠ Foo.Some(5)` — VD-211); cap/fn không so sánh được; `derives Eq` trên float ⇒ `VS0403`. |
| 3 | **Thứ tự đánh giá & short-circuit** | Trái→phải; `and`/`or`/`??` short-circuit (đo thật: `false and side()` không gọi); `not` > `and` > `or` (SPEC-lex §5; thiết kế §376–391; `docs/ViScript-thiet-ke.md A.5`). |
| 4 | **Copy vs chia sẻ** | **Deep-copy mọi giá trị** (gán, truyền, trả); `var ys = xs` không alias; closure **snapshot** biến tại lúc tạo (VD-51); `ref(x)` giữ identity. |
| 5 | **Index/key không tồn tại** | `xs[i]`/`m[k]` OOB ⇒ **`VS0503`** exit 101; `xs[-1]` trap "chỉ số âm"; `xs[i]?`/`m[k]?` trong fn ⇒ `T?` (R8); ngoài fn ⇒ `VS0501`; `.get(…)` ⇒ `T?` không panic; `b[i]` (byte index) ⇒ `VS0503`. |
| 6 | **Unicode** | `s.len()` = **Unicode scalar count**; `s.chars()` ⇒ `list[str]`; `for c in s` ⇒ `char`; **không** `s[i]`/slice/grapheme; NFC chỉ ở lex. `"Việtb".len()` → **5**. |
| 7 | **Xuống dòng & thụt lề** | Đơn vị thụt = **cấp đầu tiên > 0**, sai ⇒ **`VS0109`** (2 cột ví dụ còn chạy); trộn 2/4 ⇒ VS0109; tab ⇒ **`VS0107`**; nối dòng chỉ trong `() [] { }` hoặc `|>` đầu dòng; `:` mở khối; `vis fmt` normalize 4 dấu cách nhưng **không tự vá tệp sai** (in lỗi, stdout rỗng). |
| 8 | **Precedence & associativity** | postfix > `**` (phải) > `-` (unary) > `* / %` > `+ -` > `.. ..=` (không kết hợp) > **`??` (phải) > so sánh (KHÔNG kết hợp — `x == 1 == y` ⇒ VS0201 + nhắc ngoặc) > `not` > `and` > `or` > `|>` (trái)**. `2**3**2=512`; `-2**2=-4`; `o ?? 10 + 5 = 15` (`??` lỏng hơn `+`); `o ?? 0 == 3` → `true` (`??` chặt hơn `==`). Nguồn: `docs/ViScript-thiet-ke.md:376-391`, `A.5 (1408-1423)`, SPEC-lex §5. |
| 9 | **Record vs map** | `{ten: 1}` record (khoá định danh trần) / `{"ten": 1}` map / `{(k): 1}` map computed / `{}` map rỗng / **`{[k]: 1}` ⇒ cảnh báo `VS0208`** (tắt `{([k]): 1}`) / trộn trần + chuỗi/`(expr)` bất kỳ thứ tự ⇒ **`VS0207`** (VD-215; đo 4/4). |
| 10 | **Trùng tên** | type re-khai / impl method / trait trùng cùng file ⇒ **`VS0302`**; biến/param trùng module std đã import ⇒ **cảnh báo `VS0211`** và member position **vẫn rơi về module std** (§6.6). |
| 11 | **Ngữ cảnh kiểu kỳ vọng** | Auto-lift `T → T?` ở biên annot (`let`/return/arg/closure-return — VD-227/228/229); literal int⇒float chỉ khi có annot float; `match Some(…)` khớp giá trị **chưa bọc**; `None` trần chỉ ở annot `T?` (VD-238). |
| 12 | **Some/None trong pattern** | Viết hoa (viết thường ⇒ **`VS0203`**, fmt không tự sửa); match phải đủ nhánh (**`VS0402`** kèm tên nhánh thiếu); `while let Some(v) = e:`; or-pattern `A(x) | B(x)`. |
| 13 | **`??` và `?`** | `l ?? r`: rhs chỉ eval khi lhs `None` (đo thật); lhs không `T?` ⇒ `VS0412`. `?` chỉ trong fn (ngoài fn ⇒ **`VS0501` lúc chạy — tyck không chặn**); bóc `Some`/`Ok`; `let v = r else e:` (R3b) là cách thay thế có khối else bắt buộc thoát. |

---

## 4. Các bài ví dụ hoàn chỉnh — trỏ tới `VISCRIPT-ERGONOMICS-EXAMPLES.md`

Tệp đi kèm: **[`VISCRIPT-ERGONOMICS-EXAMPLES.md`](VISCRIPT-ERGONOMICS-EXAMPLES.md)** — 8 bài, mỗi bài: đề bài + input xác định được kết quả + **toàn bộ code** + lệnh chạy + stdout/stderr/exit code thật + dependency + chỗ phải workaround/dùng API nội bộ/điểm bị chặn.

| # | Bài | Trạng thái | Chỗ đau nhất |
|---|-----|---|---|
| 1 | Đọc file → lọc dòng → ghi kết quả | **XANH** hai engine | không `fs.lines` — đọc cả tệp + `split("\n")` |
| 2 | Ghép hai CSV theo ID (thiếu khóa/khóa trùng) | **XANH** | không `int(str)` (parse tay ~30 dòng); không Set (dùng `map[str,bool]`); không CSV quote |
| 3 | JSON trường tùy chọn → transform → JSON | **XANH interp · VM HỎNG** [LỆCH ENGINE] | pattern ctor lồng trên VM trượt (§6.2); `Null` không dựng được từ code (VS0501); `Num(n)` payload `{error}` không làm số học được |
| 4 | HTML → Markdown (giữ text + URL liên kết) | **XANH** | không regex, không `str.find`/slice — parser tay 2 pha; entity chỉ 3 cái |
| 5 | CLI: bắt buộc + mặc định + cờ boolean + báo lỗi | **XANH** | **không eprint/exit**: lỗi hợp lệ = stdout + `assert false, msg` ⇒ VS0501, exit **101** |
| 6 | Gọi tiến trình ngoài, đối số có khoảng trắng, stdout/stderr/exit code | **CHẶN trong `.vis`** | `std.proc` **không tồn tại** (`VS0705`); `extern cmd` không chạy được (`VS0501`); `^prog` ⇒ `VS0201`. `.vish` lấy được `{stdout,stderr,code}` qua `| complete` (chỉ in, **không bind biến**) |
| 7 | Multi-module: record + enum + trait | **XANH** | `impl Trait for X` cục bộ theo module — **không dispatch xuyên module** ⇒ phải bọc `pub fn`; quy tắc mồ côi (VD-98) |
| 8 | Collection + closure + Option + Result | **XANH** | match/if phải đồng kiểu mọi nhánh; `xs.push` trả list vs `()` trần (`tuple[]`); `/` luôn float (chia nguyên `//`) |

`vis check` xanh cả 8; `--vm --no-cache` byte-identical với interp ở bài 1, 2, 4, 5, 7, 8; bài 3 **VM hỏng** (đã ghi ở §6.2 + `DUNG-CHO-A0`).

---

## 5. Ca dễ nhầm & thông báo lỗi — trỏ tới `VISCRIPT-ERGONOMICS-DIAGNOSTICS.md`

Tệp đi kèm: **[`VISCRIPT-ERGONOMICS-DIAGNOSTICS.md`](VISCRIPT-ERGONOMICS-DIAGNOSTICS.md)** — 20 ca, mỗi ca: **Ý định → code → hành vi thực tế → thông báo lỗi nguyên văn (vị trí + nội dung + dòng exit code) → cách sửa được khuyến nghị**.

Bảng tổng 20 ca (chi tiết trong tệp đi kèm):

| # | Ca | `check` | exit code run | Engine |
|---|---|---|---|---|
| 1 | `None` gán `int` | VS0412 (1) | 2 | giống |
| 2 | `int??` (optional lồng) | 0 | 0 — **VM byte-khác interp, tắt nhánh / im lặng / crash** | **LỆCH [BLOCKED-CHO-A0]** |
| 3 | record `{a = 1}` vs map `{"a": 1}` | VS0201 | 2 / 0 | giống |
| 4 | pattern binding che biến ngoài | 0 | 0 (che trong arm, đúng theo thiết kế) | giống |
| 5 | `match` thiếu nhánh | VS0402 (1) | 2 | giống |
| 6 | sai số/tên tham số | VS0401 | 2 | giống |
| 7 | closure không annot tham số | **0** | **101** (VS0501) | giống |
| 8 | shadow builtin/module | 0 (+VS0211) | 0 / VS0412 | giống |
| 9 | `?` sai chỗ | VS0412 | 2 | giống |
| 10 | field/key không tồn tại | 0 | **101** (VS0501/VS0503) | giống — check không chặn |
| 11 | thụt lề / xuống dòng | VS0109/VS0201 | 2 | giống |
| 12 | precedence & so sánh khác kiểu | VS0201/VS0412 | 2 | giống |
| 13 | `chan(0)` / recv rỗng / close-rồi-recv | 0 | VS0505 / None | giống |
| 14 | gọi tên không tồn tại | **0** | **101** (VS0501 "chưa hỗ trợ ở VS0") | giống |
| 15 | `return None` trong fn `-> int` | VS0412 (VD-238) | 2 | giống |
| 16 | `Some(x)` khi đã Option | 0/VS0412 | nested "thành thật"; match lệch VM | **LỆCH [BLOCKED-CHO-A0]** |
| 17 | truyền `42` cho `int?` | 0 | lift ngầm ở biên hàm | giống |
| 18 | match sai biến thể enum | VS0403+VS0402 (check chỉ in lỗi đầu) | 2 | giống |
| 19 | `let x = None` không annot | 0 | 0 (hợp lệ — đường duy nhất của `None` trần) | giống |
| 20 | destructure không phải tuple | **0** | **101** — **hai engine khác vị trí và nội dung message** | giống exit, **khác message** |

**Finding trọng yếu (chi tiết + repro trong tệp đi kèm)**

1. **Hai engine lệch byte tại `T??`/optional lồng** — VM không khớp arm, `--dump-bytecode` `JumpIfFalse` trỏ sai vào thân arm, VM có thể in **0 byte, exit 0** (kết quả sai **im lặng**) — §6.1.
2. **`vis check` PASS những lỗi chỉ nổ runtime**: tên không tồn tại (ca 14), closure unannotate kiểu sai (ca 7), record dựng thiếu field rồi đọc (ca 10), `m["k"]` thiếu khóa (ca 10), `chan(0)` (ca 13), destructure không phải tuple (ca 20).
3. **`vis explain` chỉ in một dòng**; thông báo runtime `VS0501 … chưa hỗ trợ ở VS0` làm người dùng hiểu nhầm "tính năng chưa có"; `<fn>` vs `<hàm>` khác nhau hai engine (ca 7).
4. **Tài liệu vs mã**: thiết kế §742 (chan trả tuple) sai thực tế (chan trả **một** giá trị `<kênh>`, `Chan[T]` không destructure được); VD-19/VD-228 văn bản "đối số chưa nâng" — **mã có nâng** (VD-228b); `chan(0)` "rendezvous hợp lệ" (VD-178 QĐ-2) — **mã từ chối** (`VS0505`).
5. **Không có diagnostics cho**: arm match chết hoàn toàn (ca 5b), `{bare-key: v}` là tên trường cố định, indent-2 cột chỉ `fmt` bắt (run/check xanh).

---

## 6. Những điểm chưa hài lòng & lựa chọn thiết kế (bằng chứng → chi phí đổi → để A0 quyết)

> Quy ước: **KHÔNG tự quyết.** Mỗi mục ghi (quan sát → vị trí → chi phí sơ bộ → hướng đề xuất **cho A0 cân nhắc**).
> Công thái học ở đây nhìn theo ba trục: (a) dễ học/ít bất ngờ, (b) khó đọc của code ngắn, (c) chỗ "hoạt động khác nhau tuỳ ngữ cảnh".
> Các mục thuộc nhóm "**engine lệch byte**" đã DỪNG theo VD-241 §6 — xem `agent/DUNG-CHO-A0.md`.

### 6.1 `T??` / optional lồng — docs tự mâu thuẫn, code phẳng 1 lớp, hai engine lệch byte [BLOCKED-CHO-A0]

- **Quan sát**: `docs/ViScript-thiet-ke.md` ghi 4 chỗ "`T??` là **lỗi kiểu**" (§4.4 dòng 220, 363, 498; EBNF dòng 1358) và §7.3 + VD-178 ghi phép tự nâng **nhiều lớp** `T → T? → T??` (dòng 1153, 1183). QUEUE ST-202b từng ghi `int?? = 5 ⇒ Some(Some(5))`.
- **HEAD đo thật**: annot `int??` hợp lệ, **giá trị phẳng** (`Some(5)`, không bọc 2 lớp); `match` `Some(Some(v))` ở interp khớp `Some` trên giá trị chưa bọc; **VM không khớp, không arm chạy, out rỗng, exit 0**; `let x: int?? = None` ⇒ VM crash `VS0501`; subset có arm `None` duy nhất ⇒ VM exit 0 im lặng.
- `vis check` **exit 0** mọi biến thể; corpus `test/sem` không phủ ⇒ 473/473 vẫn xanh.
- Byte-identical hợp đồng bị phá. **Không cos ca nào để phân định bên đúng.**
- **Chi phí**: (i) sửa checker chặn annot `T??` + sửa docs + thêm ca khoá + sửa lỗi VM codegen — ~0,5–1 ngày, 0 golden đổi (annot `int??` hiện không có ca trong corpus); (ii) hợp lệ thật `Opt(Opt(T))` phải sửa codegen VM + thêm differential — 1–2 ngày + ca khoá mới; va đập SPEC-lex.
- **Đề xuất cho A0**: chọn (i) (khớp văn bản §4.4, rẻ, không lồng hoá) — nhưng A0 quyết; không đụng trong VD-241.

### 6.2 VM mis-compile pattern ctor lồng [BLOCKED-CHO-A0]

- **Quan sát** (tái lập độc lập, `ergo-b/t19.vis`, `t23.vis`): `match json.decode(...): Ok(v): match v: Obj(m): match m.get("name"): Some(Str(t)): …` — **VM trượt arm `Some(Str(t))` vào `Some(_)`**; `Some(Some(5))` trên VM **không chạy nhánh nào**; `Some(EnumUser(...))` nullary cũng trượt.
  `Some(Str("An"))` (dựng tay): interp `nested ctor = An` vs VM `Some khac`.
- Giá trị `Json` **do code dựng** rồi `json.encode(...)`: VM ra `{"variant":"Arr","value":[]}` thay vì `[]`.
- `vis check` xanh mọi biến thể; **interp đúng tất cả** (theo quy tắc match khớp cấu trúc giá trị).
- **Vì sao nguy hiểm**: bài JSON thật (EXAMPLES bài 3) đẻ `Some(Str(field))` ở mọi `m.get(…)` — VM hỏng trong production; `test/sem` không có ca ctor lồng nên 473/473 không đỏ — **hợp đồng "byte-identical" đang được bảo vệ bằng lỗ corpus**.
- **Chi phí**: sửa `codegen` (VM) cho mọi pattern ctor lồng + ca differential byte-identical + đo edge (nullary enum user, Json encode) — ~1–2 ngày + thêm ca khoá (+n).
- **Đề xuất cho A0**: ratify bug-fix VM; **không** đổi SPEC; nếu A0 muốn "mẫu lồng không được bảo đảm" (phương án b) thì phải ghi rõ ở cheatsheet + CHEAT — nhưng cắt nghĩa `std.json` examples, không khuyến nghị.

### 6.3 `vis check` xanh cho các lỗi chỉ nổ runtime (chính sách cần chốt)

- **Quan sát** (20 ca ở DIAGNOSTICS): gọi/đọc tên không tồn tại (`VS0501 … chưa hỗ trợ ở VS0`), closure unannotate kiểu sai (ca 7), record **dựng thiếu field** rồi đọc (ca 10) — **checker hoàn toàn không phát hiện**, map key không tồn tại (`m["k"]`, ca 10), `chan(0)` (ca 13), destructure không phải tuple (ca 20) — tất cả **exit 0 ở check**.
- **Vì sao**: tyck hiện chỉ bắt được phần kiểu annot; giá trị runtime (tên không tồn tại đã qua namespace/ty) và destructure tuple (permissive — VD-44 #1) thoát khỏi kiểm.
- **Chi phí**: (a) mở rộng tyck phủ các ca trên ~2–4 ngày + rất nhiều ca corpus; đổi profile checker (rủi ro regression) — phải VD riêng; (b) **không đổi ngữ nghĩa**, chỉ **tài liệu rõ**: README "check xanh ≠ không có lỗi runtime" + thêm lối thoát (`err-of`, `unwrap_or`, strict-caps khuyến nghị) — ~0,5 ngày; (c) strict-caps mặc định? (hiện tắt — VD-124).
- **Đề xuất cho A0**: (b) ngay (tài liệu); (a) xếp pha 2; (c) giữ chế độ tắt theo VD-124.

### 6.4 Escape `\u{…}` silent miscompile ở HEAD (bug, chưa có VD cho phép tự sửa theo VD-241)

- **Quan sát** (fresh đo của sub-agent A, repro `/tmp/opencode/ergo-a/*.vis`): mọi chuỗi chứa `\u{…}` chạy qua interpolation splitter trước khi tới escape decoder (`giai_escape` chỉ biết 10 escape đơn) — `"\u{41}"` in `u41`, `"e\u{0301}"` in `eu301`; **không báo lỗi** — silent miscompile ở interp và VM.
- SPEC-lex hứa `\u{…}` đúng Unicode scalar (docs/SPEC-lex.md dòng ~57) — code hiện cạn.
- **Chi phí**: sửa `giai_escape`/`giai_interpolated` cho nhất quán + ca khoá — ~0,5 ngày + 1 ca sem; **thay đổi output compiler** (chuỗi đúng sau khi sửa).
- **Đề xuất cho A0**: ratify mini-VD "bug-fix `\u{…}` — repro trước/sau + 1–2 ca khoá" — hoặc đưa vào A0-3 đã dự trù lambda annotation? **Không gộp** — sửa riêng.

### 6.5 Fresh clone `./vis run` và `./vis-dev` có thể đỏ trên máy dev yếu toolchain — và `find_cargo` ưu tiên ngược

- **Quan sát**: `bin/vis-dev` `find_cargo()` (`bin/vis-dev:21-26`) ưu tiên `command -v cargo` hơn `$HOME/.cargo/bin/cargo`; máy `ps` có `/usr/bin/cargo` = **1.93.1** buộc đồ thị phải `vis-host-linux@0.1.0` (path-dep, `rust-version = "1.99.1"`) fail: `error: rustc 1.93.1 is not supported…` (cargo exit 101, 0.04 s). Hệ quả: fresh-clone quickstart (README) fail; `./vis-dev check` **FAIL** ở 6 self-test + doc-examples; chỉ khi export `PATH="$HOME/.cargo/bin:$PATH"` (convention mọi ritual của repo) thì **PASS**.
- `infra/linux/Dockerfile` pin đúng `rust:1.99.1-bookworm` nhưng tự khai "CHƯA DÙNG trong VS2".
- **Chi phí**: đổi 1–3 dòng `find_cargo` ("thử `~/.cargo/bin/cargo` trước `/usr/bin/cargo` khi phiên bản plain cargo < rust-version của workspace") hoặc tăng root `rust-version`; cập nhật README (ghi PATH convention); không tác động ngữ nghĩa — ~0,5 giờ + ghi DEM. **Nhưng** đây là sửa dev-tool **ở rìa của quyền** (interplay với CI drift) — tôi **cố ý chưa áp** vì A0 đang yêu cầu dữ liệu, không yêu cầu sửa tool; hỏi A0 để tránh đụng AGENTS.md luật vận hành.
- **Đề xuất cho A0**: ratify mini-VD "`find_cargo` thử `~/.cargo/bin` trước + README ghi PATH convention + root `rust-version = \"1.99\"`" — không đổi output.

### 6.6 Doc ↔ code lệch (cho A0 biết, không tự sửa trong đợt này)

| # | Doc nói | Code HEAD | Đề nghị |
|---|---|---|---|
| a | Cheatsheet §(scripts) "mã 1 `main` trả Err [DỰ KIẾN]" — §0 và §13 tự mâu thuẫn ([DỰ KIẾN] vs [ĐÃ CÓ] 101) | **main trả `Err` vẫn exit 0** (interp & VM); `RunOutput.exit` được tính nhưng CLI không đọc | nối dây R4 (VD-178) ⇒ exit 1 (golden? thêm 2–3 ca + sửa cheatsheet) |
| b | Cheatsheet §1 `let` reassign ⇒ `VS0412` | thực tế **`VS0413`** | sửa cheatsheet |
| c | Cheatsheet §13 "[DỰ KIẾN] `assert cond, \"msg\"`" | **đã làm** (R20a; chỉ cần `//` cho chia nguyên, `/` luôn float) | sửa cheatsheet §13 |
| d | Cheatsheet §5 record literal: "đủ trường" — nửa sự thật | **thứ tự khai báo phải khớp** (`same_record` zip, `tyck/mod.rs:432`); default `age: int = 0` không tự áp | sửa cheatsheet + doc-examples |
| e | Cheatsheet §12 #10: `vis fmt` sửa tab | fmt **không** tự vá file sai thụt — in lỗi, stdout rỗng | sửa cheatsheet |
| f | Cheatsheet §v "async một luồng … kiểu" | đã đúng sau VD-240 (đã sửa @ 6295c8e + review fixes d1b4ff2) | giữ |
| g | Thiết kế §742 `chan` trả `(Sender, Receiver)` tuple | chan trả **một** `<kênh>`; không destructure được | sửa §742 + ca khoá? (cần VD) |
| h | Thiết kế VD-228 văn bản "đối số chưa nâng" | **biên hàm có nâng** (VD-228b, `goi.rs` + test `vd_205.rs` (11) 2026-11-19) | sửa DECISIONS văn bản (Erratum; không đổi ngữ nghĩa) |
| i | `chan(0)` "rendezvous hợp lệ" (VD-178 QĐ-2) | **mã từ chối VS0505** | sửa §QĐ-2 hoặc chốt lại VD (A0) |
| j | VD-223(ii) "Use-site ưu tiên" | member call **rơi về module std** (`lower/expr.rs:251-263`; `vd_223.rs` pin) | sửa DECISIONS văn bản (Erratum) |
| k | SPEC-lex:41 `float.inf` / `float.nan` | **VS0501** | hoặc implement (đa năng), hoặc sửa SPEC-lex |
| l | SPEC-lex:57 `\u{…}` | **hỏng ở HEAD** (§6.4) | xem §6.4 |
| m | Thiết kế dòng 1590 "Linux/Windows biên dịch" | Linux chạy thật; **Windows chưa chạy thật**; wasm check only | sửa doc roadmap |
| n | Cheatsheet §1 exit code "1 [ĐÃ CÓ] " vs check=1/run=2/101 | **run: 0/2/101 · check: 0/1** | sửa cheatsheet |
| o | Cheatsheet §4"[ĐÃ CÓ] 101" vs dòng §0 "khác" | — | sửa cheatsheet |

(Cheatsheet các mục stale được ghi làm việc "sửa tài liệu cho khớp mã" thuộc thẩm quyền VD-241; **chưa áp** trong đợt ergonomics — hỏi A0 cùng gói §7.)

### 6.7 Checker-tĩnh thiếu nâng tại biến thể enum trong pattern (exit động khác nhau hai engine ở mức message)

- Quan sát (DIAGNOSTICS ca 20): `let (a, b) = 42` — interp trap tại **chỗ dùng** `a`; VM trap tại **chỗ destructure** — cùng exit 101, cùng VS0501, **message và vị trí khác nhau**; `vis check` exit 0. Cũng xảy ra với `let (tx, rx) = chan[int](1)`.
- Chi phí: doc-only (ghi rõ engine) hoặc VD nhỏ "destructure ≠ tuple ⇒ thông báo thống nhất hai engine" — 0,5 ngày + 1 ca khoá.

### 6.8 Cơ chế phụ không nhất quán nhỏ (ghi nhận, không chặn)

- `print(f)` closure: interp `<fn>` vs VM `<hàm>` (ca 7/Show).
- `vis fmt --check` rc=1 khi có tệp cần sửa, rc=2 khi file lỗi cú pháp/đọc được — hai ngữ nghĩa cho một cờ.
- `vis test`/`vis check` in TAP kể cả khi pipe (có `--json`).
- `--strict-caps` **tắt mặc định**, bật tự động khi `use pkg.*` (VD-124); `[quyền]`⇒`VS0604`, exit 2.
- `pkg` lock format `lock:2`; **install từ lock đơn độc thất bại** nếu thiếu `.vis/store/` (cần `add` lại) — "lock đi cùng kho líu cờ phí, không độc lập" (D §5).
- usage root **thiếu `vis test`/`vis cache`/`pkg update|remove`**; `vis help <cmd>` không có; `vis --version` không có.
- LSP: hover/completion/diagnostics OK; **không** def/refs/rename/codeAction/signatureHelp/documentSymbol (`grep -ri` 0 hit trong `crates/vis-cli/src/lsp/`).
- **Syntax highlighting: KHÔNG CÓ** mọi nơi (không `.vscode`, không grammar, không tree-sitter) — README "VS Code vơi LSP" chưa đủ.
- Stack trace: **không có frame ViScript** (thiết kế §526 hứa vết ngăn xếp — chưa làm); lỗi runtime chỉ 1 dòng `file:line:col error VSxxxx`.

### 6.9 Gõ bàn phím & độ ngắn gọn — chỗ bất tiện nhất đo được

| Ca | Mức bất tiện | Ghi chú |
|---|---|---|
| Không `int(str)`/`float(str)` | cao | mọi CSV/CLI phải viết parse_tay ~30 dòng (EXAMPLES bài 2, 5) |
| Không `str.find`/regex | cao | trích URL href bằng `str.split` (EXAMPLES bài 4) |
| Không eprint/exit | trung bình | lỗi hợp lệ = stdout + `assert false, msg` ⇒ 101 (EXAMPLES bài 5; VD deadline) |
| Không `use … as alias` | thấp | namespace toàn cục, tên pub gộp (EXAMPLES bài 7) |
| Type annotation bắt buộc cho closure params | trung bình | không annot được ⇒ không lift (#155); params lambda không parse annot (A0-3) |
| Every `match` nhánh đồng kiểu | trung bình | `xs.push` trả list vs `()` trần — mọi nhánh "không làm gì" phải kết thúc bằng phép gán (EXAMPLES bài 2, 3, 4, 8) |
| Không Set | thấp | `map[str, bool]` thay thế |

---

## 7. Trải nghiệm công cụ (đầu file đến cuối, đo được tại HEAD)

### 7.1 Formatter — `vis fmt` (**có**, vòng 1, không config)

- Chuẩn hoá **chỉ**: thụt lề (4 dấu cách/cấp suy từ CST), tab→4, trailing-space, đúng một newline cuối, NFC. Không gộp/ngắt dòng, không sắp xếp khoá map.
- `--check` in danh sách tệp, **rc=1**; file lỗi cú pháp rc=2; **bỏ im lặng** `test/fmt|check|doc-examples` + sibling `.err`.
- Idempotent (lặp fmt(fmt(x))==fmt(x) — gác trong `nightly` vòng tròn fmt); thụt lề `.vish` giữ nguyên; fmt **không tự vá** tệp sai.
- **Không nằm trong `vis-dev check`** (chỉ `nightly`).

### 7.2 Syntax highlighting — **KHÔNG CÓ**

0 grammar/plugin/tree-sitter/tmLanguage ở toàn repo (`grep -riE` không hit); không `.vscode/`, không `resources/` grammar, README "VS Code qua LSP" chỉ có LSP (không tô màu).

### 7.3 LSP — `vis lsp --stdio` (có khung; phạm vi hẹp)

- `textDocumentSync: 1` (full sync), `utf-16`; diagnostics **module-aware** (chỉ doc chính); hover kiểu (markdown fence vis, in `tên: kiểu`); completion (keyword/enum/std members/module, trigger `.`).
- **KHÔNG có**: go-to-definition, references, rename, codeAction, documentSymbol, signatureHelp, foldingRange (grep -ri 0 hit). Tests: `crates/vis-cli/tests/lsp.rs` (102 ca), `lsp_da_tep.rs` (20 ca).
- LSP trả **bytes protocol frame** sạch; EOF stdin ⇒ thoát sạch.

### 7.4 Stack trace / vị trí lỗi

- **KHÔNG có frame ViScript** — một dòng `file:line:col: error VSxxxx: msg` (interp lẫn VM `--vm`), rc=101.
- `panic` **chưa tồn tại** (`panic("x")` ⇒ VS0501 "chưa hỗ trợ ở VS0", exit 101).
- Thiết kế §526 hứa vết ngăn xếp có vị trí nguồn — **chưa hiện thực**.

### 7.5 Chạy / test / pkg / dự án

- `vis run`, `vis check` (TAP; `--json`), `vis test` (TAP v13, `--vm`, `--timeout`; 473 ca ~1,5 s); `vis fmt`; `vis lsp`; `vis pkg add|install|list|remove|update` (**registry local, không mạng**; lock `lock:2`; **install cần `.vis/store/`** — xoá `.vis/` rồi install ⇒ "thiếu blob", rc=2); `vis explain`; `vis cache` (không có trong usage root).
- **Exit codes**: `vis run` = 0/2/101 (front-end 2, trap 101, codegen VM 1); `vis check` = 0/1; `vis test` = 0/1; `vis fmt` = 0/1/2; `vis pkg` = **2 mọi lỗi** (kể cả usage); `vis help`/flags lạ = rc=2 + stderr.
- **Dự án**: **không có** `vis new`/`vis init`/template; chỉ `vi-du/` (5 tệp). `Cargo.lock` commit ở gốc; **không CI** (không `.github/workflows`); `infra/` Dockerfile tự khai "CHƯA DÙNG".
- **Cargo.lock + rust-toolchain float** (stable) — xem §2.3.

### 7.6 `vis explain` — điểm mạnh riêng

CÓ, tiếng Việt mặc định (`--lang en|ja`), **đủ 53 mã** theo sổ đăng ký (`short: "VS…"` trong `vis-diag`); `VS0203` (new từ VD-225) có gợi ý ("`none` đã bỏ — dùng `None`…"); VS0412 kèm gợi ý sửa khi rõ kiểu (VD-238). Một dòng, không ví dụ.

### 7.7 Hướng dẫn khởi đầu

- README "Chạy thử trong 2 phút": clone → `./vis run vi-du/xin-chao.vis` — **chỉ chạy được khi toolchain ≥ 1.99.1 available + PATH convention** (xem §2.3); CC BY-NC thoải mái: máy đúng chuẩn: ~5–15 phút tới chương trình đầu tiên; máy `ps` mặc định: **blocked**.
- CHEAT-SHEET `Mười cái bẫy cho người mới`; `test/sem/**` làm ví dụ chạy được thật; `docs/` không có tutorial riêng; `CONTRIBUTING.md` ghi gate ~4–5 phút (cần python3 + git).

### 7.8 Ghi chép thêm trên máy `ps`

Với `~/.cargo/bin` đầu PATH (convention repo): `./vis-dev check` = **PASS** (2026-11-19 12:30). Với PATH mặc định: **FAIL** (§2.3). Toàn bộ phép đo trong tài liệu này dùng PATH đúng convention.

---

## 8. Định dạng bàn giao & cách đọc

- Ba tệp, tất cả mới tạo 2026-11-19 (`6fb72a7` + commit docs này):
  1. **`VISCRIPT-ERGONOMICS-FACTS.md`** (bản này) — trạng thái, quy tắc, giới hạn, lựa chọn thiết kế, công cụ, câu hỏi cho A0.
  2. **`VISCRIPT-ERGONOMICS-EXAMPLES.md`** — 8 bài hoàn chỉnh chạy thật (code nguyên văn, input xác định, stdout/exit code).
  3. **`VISCRIPT-ERGONOMICS-DIAGNOSTICS.md`** — 20 ca dễ nhầm, thông báo lỗi nguyên văn.
- Quy ước bằng chứng: `file:line` hoặc `file` —symbol` trích từ lượt đọc thật 2026-11-19; đường dẫn tương đối từ gốc repo.
  Các ca "tái lập độc lập" ghi trong `/tmp/opencode/ergo-{a,b,c,d}/` (scratch, không commit) — nội dung đã gộp nguyên văn vào tệp đi kèm.
- Không đổi ngữ nghĩa / API / golden / SPEC. **Không tệp mã nào được sửa trong đợt này.** `pha1-rc1` còn nguyên tại `05e42d0`.
- Các mục đánh dấu [LỆCH ENGINE — BLOCKED-CHO-A0], §6.1/6.2, `agent/DUNG-CHO-A0.md` và hàng §6.4 (skip fix) thuộc nhánh **không làm tiếp** cho tới khi A0 quyết.
- Các hàng docs **chưa** phản ánh §6.6 (cheatsheet/DECISIONS errata) thuộc nhãn docs-sync chờ A0 (không đổi output, không đổi ngữ nghĩa — nhưng đụng tài liệu do A0 ratify).

## 9. Câu hỏi cho Agent 0

1. **`T??` / optional lồng** — chọn (i) annot `int??` = lỗi kiểu (sửa §4.4, docs 4 chỗ, codegen VM, ca khoá) hay (ii) `Opt(Opt(T))` thật (sửa codegen VM + hai engine differential + ca khoá)? (Đề xuất (i) — rẻ nhất, khớp §4.4.)
2. **VM mis-compile pattern ctor lồng** (`Some(Str(..))`, `Some(EnumUser(..))`, Json-encode do code dựng) — ratify mini-VD "sửa codegen VM, không đổi SPEC" với ca differential `test/sem` + roundtrip? (Đề xuất: **có**, xếp trước A0-3/4.)
3. **Escape `\u{…}` silent miscompile** — ratify mini-VD "bug-fix `\u{…}` + ca khoá" không đổi ngữ nghĩa? (Đề xuất: **có**.)
4. **`vis check` xanh cho lỗi chỉ nổ runtime** (§6.3) — chọn (b) docs rõ ngay / (a) mở rộng tyck với VD riêng / (c) strict-caps default-on?
5. **`main -> Err` ⇒ exit 0** (VD-178 R4 [DỰ KIẾN] chưa nối dây) — chọn exit 1 + ca khoá, hay ghi rõ "Err không làm đỏ process" ở README?
6. **`find_cargo` ưu tiên ngược / máy fresh-clone đỏ** (§6.5) — ratify sửa `bin/vis-dev` thử `~/.cargo/bin` trước + root `rust-version = \"1.99\"` + README PATH convention?
7. **Cheatsheet + DECISIONS errata §6.6** — cho phép gói "sync tài liệu khớp mã" (cheatsheet b,c,d,e,n,o + VD-223(ii) text + VD-228 text + bench note)? (Không đổi output/ngữ nghĩa; thuộc thẩm quyền VD-241; đang chờ A0 vì A0 yêu cầu đọc trước.)
8. **Phân kỳ `print(f)` show closure** (`<fn>` vs `<hàm>`) — chốt một bên? (Khuyến nghị: `<fn>` — match `print` tiếng Anh khác lệch.)
9. **`chan` trả 1 giá trị** (thiết kế §742 tuple, code thực tế) — sửa text §742 hay giữ tuple là "alias API" để tương thích tương lai?
10. **`.vish` pipeline + lambda annotation (A0-3) + #151/#149** — thứ tự ưu tiên cho pha 2 (tham chiếu `agent/ROADMAP-PHA2-NHAP.md`)?

## Phụ lục A — Bốn trạng thái (chi tiết, từ sub-agent A — đối chiếu tại HEAD)

### A.1 Đã triển khai & kiểm chứng (tiêu biểu)

- **let/var/const + shadowing** — `test/sem/pham-vi/` (19); `VS0413` = gán bất biến.
- **if/elif/else biểu thức; while/loop/break/continue; range `..`/`..=`** — `test/sem/dieu-khien/` (19).
- **match**: guard không tính coverage (→ `VS0402`), or-pattern, exhaustive.
- **if let / while let (R18b, ST-226)** — `test/sem/optional/` (21), `test/sem/result/if-let-pattern.vis`.
- **Option/T?/Some/None** — `T?` = enum `Option[T]` thật (VD-208).
- **Result/Ok/Err/?** — `?` bóc `Some/Ok`; `xs[i]?`/`m[k]?` ⇒ `T?` (R8).
- **`let x = f() else:` / `else e:` (R3b)**.
- **list/map/enum/tuple/record + deep-copy + CoW** — `test/sem/{ban-ghi,map,list,trait,generic,long-du-lieu}/`.
- **trait + derives Eq/Ord/Hash/Show/Json/Default**; siêu trait 1 tầng; impl dispatch tĩnh record/enum.
- **generic `[T]` đơn hình** — `test/sem/generic/` (20).
- **module**: `use ./x`/`std.x`/`pkg.x`, `pub`, cấm vòng `VS0702`; **không as**.
- **std đóng băng** (API-STD.txt).
- **JSON**: `json.decode(s) -> Result[Json, Error]`; enum `Json: Null|Bool|Num|Str|Arr|Obj`; `print(e)` chỉ `{message}` (VD-231).
- **`assert` cond / cond, msg** — đã làm (R20a), fail ⇒ VS0501 exit 101.
- **`{…}` rules (VD-215)**: record vs map; trộn ⇒ `VS0207`; `{[k]: 1}` ⇒ cảnh báo `VS0208`.
- **473 sem ca** — hai engine byte-identical stdout+exit **trên corpus**; **không phải trên** `int??` / nested pattern (§6.1, §6.2).

### A.2 Đã triển khai nhưng giới hạn

Async một luồng · `chan(0)` VS0505 · closure params không annot · closure snapshot · annot constraints không kiểm · impl tĩnh chỉ record/enum · `.vish` fmt giữ thụt lề · net/http chỉ loopback · print enum flat · `set{…}` không phải Set · record literal thiếu field im lặng (§6.3) · tuple print như list · steps hai engine không so được · steps cap 2.000 khung · NaN literal không có · `x in xs` không có · `p.x += 1` ⇒ VS0201 · `panic` không có.

### A.3 Đã quyết — CHƯA triển khai

- **R4**: `main` Err ⇒ exit 1 — chưa nối dây (exit 0).
- **R24**: `.lazy()` + `Iterator[T]` — chưa (`xs.lazy()` ⇒ VS0403).
- **R10** (nửa sau): in đơn vị chuẩn hoá — chưa (in `2h30m` ⇒ `150m`).
- **`T??` "tự gói nhiều lớp"** — code **phẳng một lớp** (§6.1).
- **assert label [DỰ KIẾN]** — thực tế đã làm.
- **goroutine/select-timeout/rendezvous/GC** — VS3 (SO-NO #12/#16/#17/#19/#71/#72).
- **escape `\u{…}`** — SPEC hứa, HEAD hỏng (§6.4).
- **`float.inf`/`float.nan`** — SPEC-lex hứa, HEAD VS0501 (§6.6-k).
- **`trait Error` thân mặc định** (`source`/`span`) — nợ #156.

### A.4 Đang đề xuất — CHƯA DUYỆT (ROADMAP pha 2 nháp, A0 chọn)

A0-1 (#151 doc-examples slug) · A0-2 (#149 split-path) · A0-3 (`lambda annotation`) · A0-4 (#155 lift params closure, sau A0-3) · **A0-5 (`T??` — chốt; vừa có thêm bằng chứng engine-diff)** · A0-6 (async (a) — **chưa xếp lịch**, rủi ro lớn nhất) · A0-7 (#152 — chỉ đo, không tối ưu, chờ mac) · A0-8 (#154 — hoãn vô hình hạn).

---

**Tài liệu tham chiếu** (đọc nếu cần bằng chứng sâu hơn): `docs/SPEC-lex.md` · `docs/ViScript-thiet-ke.md` §4.4, §7.3, A.5 · `CHEAT-SHEET-ViScript.md` §13 · `agent/DECISIONS.md` (VD-215…VD-241) · `agent/SO-NO.md` · `agent/ROADMAP-PHA2-NHAP.md` · `agent/DUNG-CHO-A0.md` (mới — BLOCKED-CHO-A0 chi tiết) · `agent/DEM-2026-11-18.md` (nhật ký vận hành các lượt đo) · `agent/BAO-CAO-A0-2026-11-19-0900.md` (pha 1 đóng + hàng việc VD-241).

**Kết thúc.** HEAD `6fb72a7`; không tệp mã nào bị sửa; ba file đi cùng commit này.
