<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-315d — Gatekeeper (Agent 1) kiểm toán độc lập `VS0304` + Tarjan SCC

> **Kết luận: `FAIL`** — chức năng đúng trên ca thường, registry/test xanh, nhưng **mã mới làm
> tiến trình `vis check` abort (SIGABRT) trên đầu vào hợp lệ** (F1) và **phình chẩn đoán theo
> O(N²) byte** với SCC lớn (F2). Cần vá F1/F2 trước khi `DONE`; F3–F7 là follow-up.
>
> **Snapshot**: `987b98b8` (nền `ST-315c`) + 4 commit `c2b051c9`, `ee0dcd8d`, `5f8f8012`,
> `0ba3e8f2` · **Nhánh**: `review/r315d` · **Ngày**: 2026-11-26 · **Không sửa mã**.
> **Môi trường**: Linux, `rustc 1.100.0`, `ulimit -s 8192`, profile **debug** (đúng đường `./vis`
> dùng `cargo run`).

## 0. Phạm vi đã đọc

- Toàn bộ `crates/vis-types/src/tyck/const_cycle.rs` (318 dòng, bản cuối), `tyck/mod.rs:18,55,162-167`,
  `lower/khai_bao.rs:581-600`, `lower/co_so.rs:719-761`, `crates/vis-syntax/src/parse/khai_bao.rs:294-306`,
  `crates/vis-diag/src/lib.rs:268-303`, `agent/API-CODES.txt`, 4 commit + toàn bộ test đã đổi.
- `VD-291` §3.4 (`agent/DECISIONS.md:4081-4095`) và bản đầy đủ
  `agent/JOURNAL/quyet-dinh-a0-2026-11-26-vd291.md:66-69`; `JOURNAL/st315d-vs0304.md`.
- Không tin test có sẵn: dựng **harness ngoài repo** (`/tmp/opencode/r315d/stress`) link trực tiếp
  `vis-hir`/`vis-types`/`vis-interp`/`vis-bytecode`/`vis-vm` từ worktree này để tách riêng tầng
  Tarjan khỏi parser/lowering.

## 1. Phát hiện

### F1 — `[CAO]` Tarjan **đệ quy** tràn stack, `vis check` abort trên chuỗi `const` sâu

**Vị trí**: `crates/vis-types/src/tyck/const_cycle.rs:97` — `tarjan(v, …)` gọi đệ quy; độ sâu DFS =
độ dài đường đi, không có ngưỡng/iterative.

**Repro thật (CLI)**:

```bash
cd /tmp/opencode/r315d
python3 - <<'EOF'
n = 25000
with open("chain_25000.vis", "w") as f:
    for i in range(n - 1):
        f.write(f"const A{i}: int = A{i+1}\n")
    f.write(f"const A{n-1}: int = 1\nfn main():\n    print(A0)\n")
EOF
/home/ps/ViScript/target/debug/vis check --json chain_25000.vis
# exit 134; "fatal runtime error: stack overflow, aborting"; 43,14 s; 0 byte JSON
```

**Tách riêng Tarjan (harness ngoài repo, cùng `vis-types` của worktree)**:

| N | hướng cạnh | kết quả |
|---:|---|---|
| 20.000 | `A0→A1→…` (sâu) | `xong: 0 chan doan` |
| 25.000 | `A0→A1→…` (sâu) | **abort SIGABRT — stack overflow** |
| 100.000 | mọi giá trị literal, không cạnh | `xong: 0 chan doan` |
| 100.000 | `A0←A1←…` (nông, cùng số node/checker) | `xong: 0 chan doan` |
| 100.000 | `A0→A1→…` (sâu) | **abort SIGABRT — stack overflow** |

Ba ca 100k có **cùng số `HConst`, cùng `Checker::run`**, chỉ khác hướng cạnh ⇒ nguyên nhân là
**DFS đệ quy của Tarjan**, không phải parser/lowering/tyck. Ngưỡng debug ~**20–25k**; release chưa
đo nhưng độ phức tạp stack vẫn **O(N)**, không có trần.

**Đề xuất**: thay Tarjan đệ quy bằng **iterative Tarjan** (stack tường minh `(u, next_edge)`),
hoặc Kosaraju lặp; thêm ca hồi quy dựng HIR chuỗi 30k **gọi thẳng `vis_types::check`** (không qua
parser) để khoá “không abort”. Nếu muốn chặn cả chi phí, đặt trần mềm số `const`/cạnh kèm chẩn
đoán riêng, **không** để abort.

### F2 — `[TRUNG BÌNH]` mỗi `const` lặp lại **toàn bộ** danh sách SCC ⇒ O(N²) byte, JSON 234 MB cho 5.000 hằng

**Vị trí**: `const_cycle.rs:63-74` — `ten.join(" → ")` tạo chuỗi một lần cho SCC, nhưng vòng
`for i in ds_scc` `format!` **sao chép nguyên chuỗi đó vào từng chẩn đoán**.

**Repro**:

```bash
python3 - <<'EOF'
n = 5000
with open("cycle_5000.vis", "w") as f:
    for i in range(n):
        f.write(f"const A{i}: int = A{(i+1) % n}\n")
    f.write("fn main():\n    print(A0)\n")
EOF
/usr/bin/time -f "%e s %M KB" vis check --json cycle_5000.vis > c5k.json
# 212,90 s, 736.748 KB RSS, c5k.json = 234 MB, 5.000 × VS0304
# mỗi message = 38.926 byte (cùng một chuỗi A0 → … → A4999)
/usr/bin/time -f "%e s %M KB" vis check cycle_5000.vis
# 133,22 s, 494.616 KB RSS (chỉ in 1 chẩn đoán TAP)
```

Harness chỉ tầng checker: **233 ms / 261.520 KB / 5.000 chẩn đoán** ⇒ riêng `const_cycle` đã giữ
~195 MB chuỗi; 133–213 s còn lại là ánh xạ `line/col` sẵn có của CLI (O(số chẩn đoán × kích thước
tệp), không do patch này tạo, nhưng bị khuếch đại). Với 10k hằng cùng SCC, bộ nhớ message ~4×
(≈ 780 MB) trước khi tính CLI.

**Đề xuất** (chọn 1):
1. **Một chẩn đoán cho mỗi SCC** (span = hằng đầu SCC), message liệt kê tối đa 8 tên + `… (N hằng)`;
   đổi test `types_it` đang đòi “mỗi const một VS0304” (yêu cầu này do thiết kế tự đặt, `VD-291`
   chỉ đòi bắt chu trình).
2. Giữ một chẩn đoán mỗi `const` nhưng message **ngắn, kích thước hằng**: `chu trình …: A (cùng N hằng)`;
   không nhân bản danh sách.
3. Tối thiểu: cắt `ten` còn 8 phần tử + `… (+N)` **trước** khi `join`, và sort chẩn đoán theo `span`.

Thêm ca SCC 2.000–5.000 khoá **kích thước output** (không chỉ số mã).

### F3 — `[THẤP]` Chuỗi `A → B → C` **không phải đường đi thật** khi SCC > 2

**Vị trí**: `const_cycle.rs:65-66` — tên được **sort theo chỉ số khai báo** rồi nối `→`, không
truy vết cạnh.

**Repro** `scc3.vis`:

```vis
const A: int = B + C
const B: int = A
const C: int = B
```

Message: `chu trình phụ thuộc hằng số (\`const\`): A → B → C`. Nhưng cạnh thật chỉ có
`A→B`, `A→C`, `B→A`, `C→B`; **không có** `B→C`/`C→A` như chuỗi gợi ý. Người dùng debug theo
“đường đi” sẽ sai.

**Đề xuất**: đổi câu thành `các hằng trong cùng chu trình: A, B, C` (đúng bản chất SCC), hoặc
truy vết một chu trình thật (A→B→A) và chỉ in chu trình đó; nếu in nhiều tên thì ghi rõ là “SCC”.

### F4 — `[THẤP]` Dương tính giả với nhánh chết `if false: B`

**Vị trí**: `const_cycle.rs:268-287` — visitor duyệt **mọi** nhánh `if`/`elif`/`else` bất kể điều
kiện.

**Repro**:

```vis
const A: int = if false: B else: 0
const B: int = A + 1
fn main():
    print(B)
```

`vis check` báo 2 × `VS0304`; nhưng chạy **bỏ qua checker** (harness `vis_types::lower` +
`vis_interp::run`) cho `stdout = "1\n"`, `codes = []`. Đây là quá xấp xỉ cố hữu của phân tích tĩnh;
chấp nhận được nếu **ghi rõ** trong tài liệu, nhưng `if false` literal có thể gấp hằng dễ dàng.
Không chặn nếu A0 chấp nhận over-approximation; nên có ca khoá hành vi + ghi chú.

### F5 — `[GHI NHẬN]` Âm tính giả qua thân hàm / IIFE — **hàng rào runtime vẫn đỡ**

- `const A: int = f()` + `fn f() -> int: return A` ⇒ `vis check` **sạch** (không đi vào thân hàm).
- `const A: int = (x => A)(1)` ⇒ **sạch** (IIFE: `HExpr::Lambda` bị bỏ ở `const_cycle.rs:311`, dù
  callee được gọi ngay).

Đã kiểm hàng rào sâu: interp trả `VS0501` “chu trình `const` tại `A`” (1:22 và 3:12); VM compile
được nhưng chạy trả `VS0501` “gọi hàm quá sâu (>2000 khung)”, **không abort**. Vậy **chấp nhận
được** cho phạm vi “phân tích cạnh `const` trực tiếp”, nhưng:
- câu trong module doc (`const_cycle.rs:4-7,310`) “thân lambda đánh giá sau” **sai với IIFE**;
- nên ghi “lỗ hổng đã biết: call graph qua hàm/IIFE — runtime trap” vào journal/QUEUE.

### F6 — `[THẤP]` Tài liệu còn ghi “DFS 3 màu”, mã đã là Tarjan

`JOURNAL/st315d-vs0304.md:15` và `agent/QUEUE.md:2231` vẫn ghi “DFS 3 màu”; commit `5f8f8012`
đã thay bằng **Tarjan SCC** (và chính commit message nói DFS 3 màu bỏ sót đỉnh trong SCC có
cross-edge). Cập nhật lại mô tả (giữ lịch sử: “đã thay DFS 3 màu bằng Tarjan ở 5f8f8012”).

### F7 — `[THẤP]` Gợi ý `VS0204` mới **không có test khoá**, `vis explain` vẫn nói câu cũ

- `khai_bao.rs:303` in đúng: `` `const` cần ghi kiểu tường minh — gợi ý: `const A: int = 1` ``;
  `const A = 1` ⇒ đúng câu; `const A: int = 1` ⇒ sạch.
- `API-CODES.txt` **không đổi** dòng `VS0204` (đúng chỉ đạo `VD-291` §3.4.2), nhưng
  `crates/vis-diag/src/lib.rs:207` / `vis explain VS0204` vẫn in “khi vế phải không phải literal”,
  nay **trái hành vi**. Đây là hệ quả của chuỗi đóng băng, không phải lỗi patch; cần một `VD` sau
  để đồng bộ `vis explain`, và thêm ca test khoá câu gợi ý mới (hiện **không** có test nào).

### F8 — `[GHI NHẬN]` Thứ tự chẩn đoán tất định nhưng **không theo thứ tự nguồn**

Với `A→C, B→D, C→A, D→B`, output là `A,C,B,D` (thứ tự hoàn tất SCC của Tarjan), hash JSON giống
nhau qua 3 lần chạy ⇒ **tất định**, nhưng không sort theo `span`. Nếu LSP/UI giả định thứ tự nguồn,
cần sort; không phải lỗi chức năng.

## 2. Kiểm chứng đạt (không finding)

### 2.1 Registry `VS0304`

| Hạng mục | Kết quả |
|---|---|
| `NAME_CODES` | `[Code; 4]`, id `301,302,303,304`; `CONST_CYCLE = &NAME_CODES[3]` |
| vi/en/ja | `chu trình phụ thuộc hằng số (\`const\`)` / `constant (\`const\`) dependency cycle` / `定数（\`const\`）の依存関係が循環しています` |
| `all_codes()` | **57** mã, 7 khối; `next_block() = 801` |
| `agent/API-CODES.txt` | 57 dòng, **khớp 100%** thứ tự `all_codes()`; diff chỉ **+1 dòng VS0304** (không đổi VS0204) |
| `vis explain --list` | 57 dòng, `VS0303, VS0304, VS0401` đúng chỗ |
| `vis explain` (không tham số) | “7 khối, **57** mã” |

### 2.2 Ngữ nghĩa chu trình (probe `vis check --json`)

| Ca | Kỳ vọng | Kết quả |
|---|---|---|
| 2 chiều `A=B; B=A` | 2 × VS0304 | đúng (dòng 1, 2) |
| tự tham chiếu `A=A` | 1 | đúng |
| qua `if true:` (then) / `else:` | 2 | đúng |
| qua `match` (arm) | 2 | đúng |
| qua `map` value | 2 (+ VS0412 kiểu) | đúng |
| qua đối số `call` | 2 | đúng |
| qua nội suy `"{B}"` | 2 | đúng |
| qua `spawn f(B)` | 2 (+ VS0412 Task) | đúng |
| `lambda` không gọi: `const A: fn(int)->int = (x) => B` | **không** báo | đúng |
| `let b` (lowercase) che `const b` | **không** báo | đúng |
| hình kim cương `A→B,C; B,C→D` | không báo | đúng |
| SCC 3 đỉnh `A→B,C; B→A; C→B` | 3 | đúng |
| 2 SCC rời | 4 | đúng |
| 2 self-loop rời | 2 | đúng |
| chuỗi hợp lệ `A=1; B=A+1` | 0 | đúng |
| module `a` dùng `pub const B` của `b` | 0 | đúng (HIR hợp nhất) |
| 2 module cùng `pub const X` | VS0302 | đúng (không thành cạnh giả) |

`line/col` trong `--json` là **thật** (1:1, 3:1, 6:1…), không phải 0:0. `vis check` TAP chỉ in
**một** chẩn đoán đầu (đúng `check.rs:138`); `--json`/`vis run` in đủ. `vis run` chặn tĩnh exit 2
trước runtime.

### 2.3 Gợi ý `VS0204`

```text
$ vis check --json no_type.vis
{"code":"VS0204","line":1,"col":9,...,"message":"`const` cần ghi kiểu tường minh — gợi ý: `const A: int = 1`"}
$ vis check --json lambda_skip2.vis     # const A: fn(int)->int = (x) => B
{"ok":true,...}
```

`const A: int = 1` hợp lệ; câu cũ “khi vế phải không phải literal / R7 cho suy kiểu từ literal”
không còn trong parser. **Thiếu test** khoá câu mới (F7).

### 2.4 Test/cổng đã chạy lại (2026-11-26)

| Lệnh | Kết quả |
|---|---|
| `cargo test -p vis-diag` | **PASS** |
| `cargo test -p vis-types --test types_it` | **PASS** — gồm `tyck::const_chu_trinh_bi_bat_tinh_voi_vs0304` |
| `cargo test -p vis-cli --test cli_it explain_moi` | **PASS** 6/6 (`list_du_57_dong_dung_thu_tu`, `tro_giup_neu_du_so_ma`, …) |
| `cargo test -p vis-cli --test cli_it vd270_explain_json` | **PASS** 9/9 |
| `cargo test -p vis-cli --test lang_it const_chu_trinh -- --test-threads=1` | **PASS** 1/1 |
| `cargo test -p vis-conformance --test lib_api_it api_dong_bang` | **PASS** 3/3 |
| `cargo test -p vis-vm --test vm_it const_chu_trinh` | **PASS** 1/1 |
| `./vis-dev gate --fast` | **PASS** — 3.825 test, 0 fail (nextest, log `/tmp/opencode/r315d/logs/all.log`) |
| `cargo clippy --workspace --all-targets -- -D warnings` | **PASS** |

**Cảnh báo hạ tầng**: `CARGO_TARGET_DIR=/home/ps/ViScript/target` dùng chung, có thể bị worktree
khác trộn artifact. Không có lỗi hạ tầng nào xảy ra trong lượt này; main sẽ rebuild sạch và nên
chạy lại 1 lần. Các kết luận F1/F2 được đo bằng **harness link trực tiếp source worktree này**
(``stress-const``/``phase`` trong `/tmp/opencode/r315d/stress`), không phụ thuộc binary `vis` dùng chung.
Ghi chú log: `all.log` có dòng `explain_moi: FAIL rc=1` do **wrapper truyền hai filter** cho
`cargo test` (lỗi cú pháp lệnh), không phải test đỏ; đã chạy lại tách hai lệnh — **6/6** và **9/9**
PASS (bảng trên).

## 3. Quan sát hiệu năng (không quy lỗi ST-315d)

- `vis check` chuỗi 1k: **0,05 s**; 10k: **2,2 s** (lượt đo đầu, máy rảnh); 25k: 43 s rồi abort;
  100k: **292 s** rồi abort.
- Harness tách tầng (10k, lúc máy đang chạy test song song): `lex+parse` 0,62 s; `lower` **5,51 s**;
  `check` **17 ms** ⇒ chi phí lớn nằm ở **lowering** (có dấu hiệu O(N²), có từ trước). Sau khi vá
  Tarjan, 100k vẫn không thực dụng nếu không xử lý lowering; nên ghi nợ riêng, không gộp vào ST-315d.

## 4. Đề nghị thứ tự vá

1. **F1** — iterative Tarjan + ca hồi quy 30k (gọi thẳng `vis_types::check`, không parser).
2. **F2** — một chẩn đoán/SCC hoặc message có trần; cập nhật test `types_it` + ca SCC lớn đo output.
3. **F3** — sửa câu message thành “các hằng trong cùng chu trình: …” (hoặc in chu trình thật).
4. **F6** — sửa journal/QUEUE “DFS 3 màu” → “Tarjan”; ghi lỗ hổng IIFE/call graph (F5).
5. **F7** — thêm test khoá câu `VS0204`; xin `VD` đồng bộ `vis explain VS0204` sau.
6. **F4/F8** — ghi chú over-approximation nhánh chết; (tuỳ UI) sort chẩn đoán theo `span`.

## 5. Phiếu trả lời

- **PASS/FAIL**: **FAIL** (không đạt cổng an toàn tài nguyên; chức năng/registry/test đạt).
- **Chặn `DONE`**: F1, F2. **Follow-up**: F3–F8.
- **Không sửa mã** trong lượt review này; chỉ tạo tệp này và commit `[ST-315d] review: …`.
