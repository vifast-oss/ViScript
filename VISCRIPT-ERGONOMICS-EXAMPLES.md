<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ViScript — 8 bài ví dụ hoàn chỉnh, chạy thật (2026-11-19)

> **Agent 0 · dữ liệu công thái học, 2026-11-19.** Thu thập theo yêu cầu A0 ("8 bài ngắn nhưng trọn vẹn").
> **KHÔNG** được dùng để sửa ngữ nghĩa; các **"VM hỏng"** dưới đây là **BLOCKED-CHO-A0** — xem `agent/DUNG-CHO-A0.md`.
>
> **HEAD**: `6fb72a7` (`main`) · binary: `/home/ps/ViScript/target/debug/vis` (debug) · máy: `ps` (Linux x86_64, 12 lõi) · tải lúc đo: ~0,3–3,6 (không phải lúc benchmark).
> Lệnh chạy: `vis run [--vm --no-cache] <tệp.vis> [-- args…]`. Mỗi bài dán **code nguyên văn** + stdout/exit code thật; phần **workaround** ghi rõ những chỗ phải viết lặp/API nội bộ vì ngôn ngữ chưa có.
> Scratch (input `data/`, tệp `.vis`, repro `t19/t23/t26/t29.vis`, bản `.vish` bài 6) giữ nguyên tại `/tmp/opencode/ergo-b/`, **không** commit. Code dưới đây là nguyên văn.
>
> **Tổng kết**: 6/8 xanh hai engine · bài 3 xanh interp / **VM hỏng pattern lồng + JSON do code dựng encode sai** · bài 6 **CHẶN hoàn toàn trong `.vis`** (chỉ `.vish` với giới hạn cứng).
Ngày 2026-11-19 · repo /home/ps/ViScript · HEAD `6fb72a7` (main) · binary `/home/ps/ViScript/target/debug/vis` (debug)
Scratch: `/tmp/opencode/ergo-b` (toàn bộ `.vis`/`.vish`/dữ liệu nhập ra) · **không** sửa repo, không commit, không sinh golden.
Cột lệnh: `vis` = `/home/ps/ViScript/target/debug/vis`. Interp = mặc định; VM = `run --vm --no-cache`. Trước mỗi bài: `vis check` xanh.
Output dán nguyên văn; chỉ cắt những chỗ "≤ 40 dòng" khi ghi chú.

---
## Bài 1 — Đọc file, lọc dòng, ghi kết quả — **XANH (workaround nhỏ)**

Đề: log ứng dụng, giữ lại các dòng chứa `ERROR`, ghi ra tệp mới.
Input `data/ung-dung.log` (7 dòng): 3 dòng `ERROR`, 4 dòng khác (INFO×3, DEBUG×1).

```vis
# SPDX-License-Identifier: MIT OR Apache-2.0
# Bài 1 — Đọc tệp log, lọc dòng chứa "ERROR", ghi ra tệp kết quả.
use std.fs
use std.str

fn loc_loi(duong_vao: str, duong_ra: str) -> int:
    match fs.read(duong_vao):
        Ok(noi_dung):
            let cac_dong = noi_dung.split("\n")
            var ket_qua: list[str] = []
            for d in cac_dong:
                if d.contains("ERROR"):
                    ket_qua.push(d)
            let van_ban = str.join(ket_qua, "\n")
            match fs.write(duong_ra, van_ban):
                Ok(_):
                    print("da loc {ket_qua.len()} dong loi -> {duong_ra}")
                    ket_qua.len()
                Err(e):
                    print("khong ghi duoc {duong_ra}: {e}")
                    0 - 1
        Err(e):
            print("khong doc duoc {duong_vao}: {e}")
            0 - 1

fn main():
    let n = loc_loi("data/ung-dung.log", "data/chi-co-loi.log")
    print("so dong loi: {n}")
```

Lệnh: `cd /tmp/opencode/ergo-b && vis run bai1.vis`  → exit `0`
```
da loc 3 dong loi -> data/chi-co-loi.log
so dong loi: 3
```
Tệp ra `data/chi-co-loi.log` (74 byte): 3 dòng ERROR, **không có newline cuối** (vì dùng `str.join`).
`vis run --vm --no-cache bai1.vis` → cùng output, exit `0`. `vis check bai1.vis` → `ok 1`.

Chú thích mô-đun `std.fs` (nguồn: `crates/vis-interp/src/stdlib.rs:403-436`, chữ ký `crates/vis-types/src/stdlib.rs`): `fs.read(str) -> Result[str,Error]`, `fs.write(str, str) -> Result[(),Error]`, `fs.exists(str) -> bool`, `fs.list(str) -> Result[list[str],Error]`. **Không có `fs.lines`/`fs.read_text`** ⇒ phải đọc toàn văn rồi `split("\n")`.

**strict-caps**: không khai `cap[Dir]` ⇒ `vis run --strict-caps bai1sc.vis` báo `VS0604: [quyền] cần khai cap[Dir] cho "data/..."`, exit `2`. Khai `fn main(d: cap[Dir] @ p"./data"):` ⇒ strict chạy OK, exit `0`. Không cần cap trong `.vis` mặc định.

**Dependency**: không (chỉ `use std.fs` / `use std.str`). **Workaround**: đọc cả file vì không có API đọc theo dòng; output thiếu newline cuối.

---
## Bài 2 — Ghép hai CSV theo khóa ID — **XANH (workaround: không có thư viện CSV, không có `int(str)`)**

Đề: ghép `san-pham.csv (id,ten,gia)` với `don-hang.csv (id,so_luong)` → `ket-hop.csv (id,ten,gia,so_luong,thanh_tien)`. Xử lý: dòng hỏng, khóa trùng (giữ dòng đầu + cảnh báo), đơn không có sản phẩm, sản phẩm không có đơn.

Input (`data/san-pham.csv` có bản ghi trùng khóa `2`; `data/don-hang.csv` có khóa `4` không tồn tại ở sản phẩm):
```
id,ten,gia            id,so_luong
1,Tao,15000           1,10
2,Chuoi,8000          2,5
3,Xoai,25000          4,3
2,Chuoi trung,9000    (header + 3 dòng)
```

```vis
# SPDX-License-Identifier: MIT OR Apache-2.0
# Bài 2 — Ghép hai CSV theo khóa `id` (tự viết, không có thư viện CSV).
use std.fs
use std.str
use std.list
use std.map

fn gia_tri_chu_so(c: str) -> int:
    let digits = "0123456789".chars()
    var i = 0
    while i < digits.len():
        if digits[i] == c:
            return i
        i += 1
    0 - 1

fn parse_so(s: str) -> int?:
    let cs = str.trim(s).chars()
    if cs.len() == 0:
        return None
    var i = 0
    var dau = 1
    if cs[0] == "-":
        dau = 0 - 1
        i = 1
    var kq = 0
    var co = false
    while i < cs.len():
        let d = gia_tri_chu_so(cs[i])
        if d < 0:
            return None
        kq = kq * 10 + d
        co = true
        i += 1
    if co:
        Some(kq * dau)
    else:
        None

fn main():
    var canh_bao: list[str] = []

    match fs.read("data/san-pham.csv"):
        Ok(nd):
            var ten_theo_id: map[str, str] = {}
            var gia_theo_id: map[str, int] = {}
            let dong = nd.split("\n")
            for i in 1..dong.len():
                let d = str.trim(dong[i])
                if d != "":
                    let p = d.split(",")
                    if p.len() != 3:
                        canh_bao.push("san-pham dong {i + 1}: sai so truong ({p.len()}), bo qua: {d}")
                        ()
                    else:
                        let id = str.trim(p[0])
                        let ten = str.trim(p[1])
                        match parse_so(p[2]):
                            Some(gia):
                                if ten_theo_id.has(id):
                                    canh_bao.push("san-pham id={id}: khoa trung, giu dong dau tien, bo qua: {d}")
                                    ()
                                else:
                                    ten_theo_id[id] = ten
                                    gia_theo_id[id] = gia
                                    ()
                            None:
                                canh_bao.push("san-pham dong {i + 1}: gia khong phai so, bo qua: {d}")
                                ()
                else:
                    ()

            match fs.read("data/don-hang.csv"):
                Ok(nd2):
                    var dong_ra: list[str] = ["id,ten,gia,so_luong,thanh_tien"]
                    var da_ban: map[str, bool] = {}
                    let don = nd2.split("\n")
                    for j in 1..don.len():
                        let d2 = str.trim(don[j])
                        if d2 != "":
                            let q = d2.split(",")
                            if q.len() != 2:
                                canh_bao.push("don-hang dong {j + 1}: sai so truong ({q.len()}), bo qua: {d2}")
                                ()
                            else:
                                let id = str.trim(q[0])
                                match parse_so(q[1]):
                                    Some(sl):
                                        if ten_theo_id.has(id):
                                            let gia = gia_theo_id[id]
                                            let ten = ten_theo_id[id]
                                            dong_ra.push("{id},{ten},{gia},{sl},{gia * sl}")
                                            da_ban[id] = true
                                            ()
                                        else:
                                            canh_bao.push("don-hang dong {j + 1}: khong co san pham id={id}, bo qua: {d2}")
                                            ()
                                    None:
                                        canh_bao.push("don-hang dong {j + 1}: so_luong khong phai so, bo qua: {d2}")
                                        ()
                        else:
                            ()
                    for id in list.sort(map.keys(ten_theo_id)):
                        if not da_ban.has(id):
                            canh_bao.push("san-pham id={id}: chua co don nao")
                    let van_ban = str.join(dong_ra, "\n")
                    match fs.write("data/ket-hop.csv", van_ban):
                        Ok(_):
                            print("da ghi data/ket-hop.csv ({dong_ra.len() - 1} dong du lieu)")
                        Err(e):
                            print("khong ghi duoc: {e}")
                    print("canh bao: {canh_bao.len()}")
                    for cb in canh_bao:
                        print("- {cb}")
                Err(e):
                    print("khong doc duoc data/don-hang.csv: {e}")
        Err(e):
            print("khong doc duoc data/san-pham.csv: {e}")
```

Lệnh: `vis run bai2.vis` → exit `0`
```
da ghi data/ket-hop.csv (2 dong du lieu)
canh bao: 3
- san-pham id=2: khoa trung, giu dong dau tien, bo qua: 2,Chuoi trung,9000
- don-hang dong 4: khong co san pham id=4, bo qua: 4,3
- san-pham id=3: chua co don nao
```
`data/ket-hop.csv` (không newline cuối):
```
id,ten,gia,so_luong,thanh_tien
1,Tao,15000,10,150000
2,Chuoi,8000,5,40000
```
`vis run --vm --no-cache bai2.vis` → cùng output, exit `0`. `vis check bai2.vis` → `ok 1`.

**Dependency**: `std.fs`, `std.str`, `std.list`, `std.map` (đều std, không package ngoài).
**Workaround/chỗ chật vật**:
- Không có `int(str)` trong ngôn ngữ ⇒ tự viết `parse_so` (so sánh từng ký tự `"0".."9"` — so sánh chuỗi theo mã, `"10" > "9"` = false nên phải so sánh từng ký tự đơn).
- `map.keys` trả theo thứ tự chèn ⇒ phải `list.sort` để in cảnh báo xác định.
- Không có kiểu tuple-db/named rows; hai `map[str,…]` riêng (`ten_theo_id`, `gia_theo_id`).
- CSV "tự viết" chỉ split `,` ⇒ không xử lý được field có dấu phẩy trong ngoặc kép ("a,b") — ghi rõ là giới hạn.
- Không có set ⇒ dùng `map[str, bool]` cho tập `da_ban`.
- `list.push` trả lại chính list (không phải unit) ⇒ mọi nhánh `match` phải đồng kiểu (xem mục "Phát hiện chung").

---
## Bài 3 — JSON có trường tùy chọn, biến đổi, xuất JSON — **XANH trên interp; VM hỏng (xem ghi chú)**

Đề: `nguoi-dung.json` — mỗi phần tử có `name` (bắt buộc), `email` (tùy chọn, có thể `null` hoặc vắng), `tuoi` (tùy chọn, có thể sai kiểu). Thêm trường `domain` (tách từ email), đổi sai kiểu/thiếu thành `null`, xuất lại JSON.

Input `data/nguoi-dung.json`:
```json
[
  {"name": "An", "email": "an@example.com", "tuoi": 30},
  {"name": "Binh", "email": null, "tuoi": 25},
  {"name": "Chi", "tuoi": 28},
  {"name": "Dung", "email": "khong-co-ac-tron", "tuoi": "hai muoi"}
]
```

```vis
# SPDX-License-Identifier: MIT OR Apache-2.0
# Bài 3 — Đọc JSON có trường tùy chọn (name bắt buộc; email, tuoi tùy chọn),
# biến đổi (thêm `domain` từ email), ghi JSON ra tệp.
use std.fs
use std.json
use std.str

fn null_json() -> Json:
    match json.decode("null"):
        Ok(v):
            v
        Err(e):
            Str("khong the xay ra: {e}")

fn domain_cua(email: str) -> str?:
    let p = email.split("@")
    if p.len() >= 2:
        Some(p[p.len() - 1])
    else:
        None

fn xu_ly(m: map[str, Json], chi_so: int) -> Json?:
    var name = "?"
    var ten_ok = false
    match m.get("name"):
        Some(Str(t)):
            name = t
            ten_ok = true
        Some(khac):
            print("dong {chi_so}: name khong phai chuoi, bo qua")
        None:
            print("dong {chi_so}: thieu name, bo qua")

    if not ten_ok:
        return None

    var email_j: Json = null_json()
    var domain_j: Json = null_json()
    match m.get("email"):
        Some(Str(e)):
            email_j = Str(e)
            match domain_cua(e):
                Some(d):
                    domain_j = Str(d)
                None:
                    print("dong {chi_so}: email '{e}' khong co @, domain = null")
        Some(Null):
            email_j = null_json()
        Some(khac):
            print("dong {chi_so}: email khong phai chuoi, dat null")
        None:
            domain_j = null_json()

    var tuoi_j: Json = null_json()
    match m.get("tuoi"):
        Some(Num(n)):
            tuoi_j = Num(n)
        Some(khac):
            print("dong {chi_so}: tuoi khong phai so, dat null")
        None:
            tuoi_j = null_json()

    var ra: map[str, Json] = {}
    ra["name"] = Str(name)
    ra["email"] = email_j
    ra["domain"] = domain_j
    ra["tuoi"] = tuoi_j
    Some(Obj(ra))

fn main():
    match fs.read("data/nguoi-dung.json"):
        Ok(s):
            match json.decode(s):
                Ok(v):
                    match v:
                        Arr(xs):
                            var ra: list[Json] = []
                            var i = 0
                            var dem = 0
                            for x in xs:
                                match x:
                                    Obj(m):
                                        match xu_ly(m, i + 1):
                                            Some(o):
                                                ra.push(o)
                                                dem = dem + 1
                                            None:
                                                dem = dem
                                    _:
                                        print("dong {i}: khong phai doi tuong, bo qua")
                                i += 1
                            let enc = json.encode_pretty(Arr(ra))
                            match fs.write("data/nguoi-dung-ra.json", enc):
                                Ok(_):
                                    print("da ghi data/nguoi-dung-ra.json")
                                Err(e):
                                    print("khong ghi duoc: {e}")
                            print(enc)
                            print("xu_ly {dem} doi tuong hop le")
                        _:
                            print("mong doi mot mang doi tuong")
                Err(e):
                    print("JSON khong hop le: {e}")
        Err(e):
            print("khong doc duoc tep: {e}")
```

Lệnh: `vis run bai3.vis` → exit `0`
```
dong 4: email 'khong-co-ac-tron' khong co @, domain = null
dong 4: tuoi khong phai so, dat null
da ghi data/nguoi-dung-ra.json
[
  { "name": "An", "email": "an@example.com", "domain": "example.com", "tuoi": 30 },
  { "name": "Binh", "email": null, "domain": null, "tuoi": 25 },
  { "name": "Chi", "email": null, "domain": null, "tuoi": 28 },
  { "name": "Dung", "email": "khong-co-ac-tron", "domain": null, "tuoi": null }
]
xu_ly 4 doi tuong hop le
```
(pretty JSON rút gọn một dòng khi dán; bản đầy đủ: `data/nguoi-dung-ra.json`)

JSON hỏng (`data/hong.json`): `vis run bai3.vis` in `JSON khong hop le: JSON không hợp lệ ở byte 17: khoá object phải là chuỗi`, exit `0`.

**VM — hỏng, xác định**: `vis run --vm --no-cache bai3.vis` in ra `dong N: name khong phai chuoi, bo qua` cho **mọi** dòng và ghi `data/nguoi-dung-ra.json` thành `{"variant":"Arr","value":[]}`. Tối thiểu hoá thành repro:
```
// t19.vis — match json.decode("{\"name\":\"An\"}"): Obj(m):
//   match m.get("name"): Some(Str(t)): "ten=..."   → interp: ten=An · VM: rơi vào Some(_)
// t22.vis — let g = m.get("name"); match g: Some(Str(t)): ...  → interp khớp, VM trượt khớp
// t23.vis — match Some(Str("An")): Some(Str(t)): ...            → interp khớp, VM trượt khớp
// t25/t26 — match Some(Some(5)): Some(Some(n)): ...             → interp ra,  VM: KHÔNG nhánh nào chạy, không in gì
// t27/t28 — enum người dùng enum A(int): Some(A(n))             → VM: trượt khớp (print "khac")
// t29.vis — json.encode(Arr([])) user-constructed               → interp: []       VM: {"variant":"Arr","value":[]}
```
Kết luận: **VM mis-compile pattern lồng nhau** và **enum `Json` dựng bởi code người dùng không giữ tag `Json`** (VM encode như enum chung `{"variant","value"}` — so `crates/vis-interp/src/stdlib.rs:713-760` có guard `ea == "Json"` trong `json_tai`; VM không đi qua hàm này hoặc không tag đúng). `vis check` xanh cả hai. File output bài 3 vì vậy phải tạo bằng interp (đã chạy lại interp để khôi phục `data/nguoi-dung-ra.json`).

**Dependency**: `std.fs`, `std.json`, `std.str`. **Workaround**:
- `Null` **không dựng được từ code** — `let j: Json = Null` ⇒ `VS0501: tên 'Null' chưa hỗ trợ ở VS0` (cả hai đường). `Some`/`Str`/`Num`/`Arr`/`Obj`/`Bool` dựng được; nullary biến thể enum người dùng (`Empty`) cũng bị VS0501 — phải viết `S.Empty`. Evidence: `crates/vis-interp/src/interp.rs:659-665` (chỉ `Some("None")` được đặc cách), `crates/vis-interp/src/builtin_value.rs`, `test/sem/enum/empty-branch.vis` (`let s: S = S.Empty`). Workaround: `null_json()` = `json.decode("null")`.
- Pattern lồng `Some(Str(t))` hợp lệ trên interp ⇒ giữ nguyên cho đường interp.
- `Num(n)` payload là `{error}` (VD-204) ⇒ không làm số học trên `n` được; chỉ chuyển tiếp `Json`.
- `print(e)`/`print(enc)`; không có eprint — cảnh báo ra stdout chung.

---
## Bài 4 — HTML → Markdown — **XANH (workaround lớn: không có HTML/regex lib, tự viết parser)**

Đề: chuyển `data/page.html` → markdown, giữ **text** và **URL** của liên kết (`<a href="URL">text</a>` → `[text](URL)`), không thư viện HTML/regex.
Input:
```html
<h1>Bao cao thang 11</h1>
<p>Xem <a href="https://viscript.org/docs">tai lieu chinh thuc</a> va <b>ghi chu</b>.</p>
<p>1 &lt; 2 &amp; 3 &gt; 0</p>
<ul>
<li>muc mot</li>
<li>muc hai voi <i>nhi danh</i></li>
</ul>
```

```vis
# SPDX-License-Identifier: MIT OR Apache-2.0
# Bài 4 — HTML -> Markdown (tự viết, KHÔNG có thư viện HTML/regex).
use std.fs
use std.str

fn xu_ly_thuc_the(the: str, canh_bao: list[str]) -> str:
    let t = str.trim(the).lower()
    let ten = str.split(t, " ")[0]
    if ten == "h1":
        "# "
    elif ten == "h2":
        "## "
    elif ten == "h3":
        "### "
    elif ten == "/h1" or ten == "/h2" or ten == "/h3":
        "\n"
    elif ten == "p":
        ""
    elif ten == "/p":
        "\n"
    elif ten == "ul" or ten == "ol" or ten == "/ul" or ten == "/ol":
        ""
    elif ten == "li":
        "- "
    elif ten == "/li":
        "\n"
    elif ten == "b" or ten == "strong":
        "**"
    elif ten == "/b" or ten == "/strong":
        "**"
    elif ten == "i" or ten == "em":
        "*"
    elif ten == "/i" or ten == "/em":
        "*"
    elif ten == "br" or ten == "br/" or ten == "/br":
        "\n"
    elif ten == "a":
        "["
    elif ten == "/a":
        "]"
    else:
        canh_bao.push("bo the khong ho tro: <{the}>")
        ""

fn href_cua(the: str) -> str?:
    let p = str.split(the, "href=\"")
    if p.len() >= 2:
        let t = str.split(p[1], "\"")
        Some(t[0])
    else:
        None

fn bo_entity(s: str) -> str:
    var x = str.replace(s, "&lt;", "<")
    x = str.replace(x, "&gt;", ">")
    x = str.replace(x, "&amp;", "&")
    x

fn chuyen(html: str, canh_bao: list[str]) -> str:
    let cs = html.chars()
    let n = cs.len()
    var ra: list[str] = []
    var i = 0
    var dang_link = false
    var url_hien_tai = ""
    while i < n:
        if cs[i] == "<":
            var j = i + 1
            var the = ""
            while j < n and cs[j] != ">":
                the = the + cs[j]
                j += 1
            if j >= n:
                ra.push("<")
                i = i + 1
            else:
                let t = str.trim(the).lower()
                let ten = str.split(t, " ")[0]
                if ten == "a":
                    match href_cua(the):
                        Some(u):
                            dang_link = true
                            url_hien_tai = u
                            ra.push("[")
                            ()
                        None:
                            canh_bao.push("the <a> khong co href, bo qua: <{the}>")
                            ()
                    i = j + 1
                elif ten == "/a":
                    if dang_link:
                        ra.push("]({url_hien_tai})")
                    else:
                        ra.push("]")
                    dang_link = false
                    url_hien_tai = ""
                    i = j + 1
                else:
                    ra.push(xu_ly_thuc_the(the, canh_bao))
                    i = j + 1
        else:
            var j = i
            var van_ban = ""
            while j < n and cs[j] != "<":
                van_ban = van_ban + cs[j]
                j += 1
            ra.push(bo_entity(van_ban))
            i = j
    str.join(ra, "")

fn main():
    match fs.read("data/page.html"):
        Ok(html):
            var canh_bao: list[str] = []
            let md = chuyen(html, canh_bao)
            match fs.write("data/page.md", md):
                Ok(_):
                    print("da ghi data/page.md")
                Err(e):
                    print("khong ghi duoc: {e}")
            print("==== noi dung page.md ====")
            print(md)
            print("==== canh bao ({canh_bao.len()})")
            for cb in canh_bao:
                print("- {cb}")
        Err(e):
            print("khong doc duoc data/page.html: {e}")
```

Lệnh: `vis run bai4.vis` → exit `0`
```
da ghi data/page.md
==== noi dung page.md ====
# Bao cao thang 11

Xem [tai lieu chinh thuc](https://viscript.org/docs) va **ghi chu**.

1 < 2 & 3 > 0


- muc mot

- muc hai voi *nhi danh*

==== canh bao (0)
```
`data/page.md`: y nguyên (giữ cả các dòng trống thừa — xem ghi chú). `--vm --no-cache`: output giống hệt interp (diff được kiểm, exit 0).

**Dependency**: `std.fs`, `std.str`. **Không có regex** trong ngôn ngữ (xác nhận: không module/chuỗi hàm regex trong `crates/vis-types/src/stdlib.rs`; `std.*` = 9 module, không `regex`). **Workaround/chật vật**:
- Parser tay 2 pha: tách theo `<...>` bằng `chars()` + `while` — **không có `str.find`/index/slice**, nên "tìm chuỗi con" phải split (`str.split(tag, "href=\"")`) hoặc tự duyệt ký tự.
- href trích bằng 2 lần `str.split` theo `"` — chỉ đúng với href nháy kép, không escape `"` trong URL.
- Không normalize whitespace ⇒ sinh nhiều dòng trống; entities chỉ `&lt; &gt; &amp;` (không có `&#xNN;`, không `&quot;`, không `&nbsp;`); thẻ lạ bị bỏ kèm cảnh báo trên stdout (không ghi vào `.md`).
- Không kiểm tra lồng hợp lệ (b dentro i thế nào cũng ra `*...*`). URL giữ nguyên dạng văn bản.
- Thẻ `<b>` chưa đóng → `**` treo cuối dòng, không cảnh báo (không stack-based checker).

---
## Bài 5 — CLI: tham số bắt buộc, mặc định, cờ boolean, báo lỗi hợp lệ — **XANH (workaround: không eprint/exit; validate tay)**

Đề: `vis run bai5.vis -- --ten TEN [--so-lan N] [--nga] [--giup]` — `--ten` bắt buộc; `--so-lan` mặc định 3, phải ≥ 1; `--nga` là cờ boolean; lỗi ⇒ thông điệp + thoát khác 0.

Cách lấy `args`: `fn main(args: list[str]):` với `vis run x.vis -- a b c` — argv sau `--` đưa nguyên vào tham số không-`cap` đầu tiên của `main` (interp **và** VM từ ST-157/VD-156; xem `crates/vis-cli/tests/vm_main_argv.rs`, `cap_khong_lo.rs`). Không có thư viện flag-parser ⇒ parse `list[str]` tay.

```vis
# SPDX-License-Identifier: MIT OR Apache-2.0
# Bài 5 — CLI: `--ten TEN` (bắt buộc), `--so-lan N` (mặc định 3, >= 1), `--nga` (cờ boolean), `--giup`.
use std.str

fn in_huong_dan():
    print("dung: vis run bai5.vis -- --ten TEN [--so-lan N] [--nga] [--giup]")
    print("  --ten TEN    ten can chao (bat buoc)")
    print("  --so-lan N   so lan lap lai (mac dinh 3, phai >= 1)")
    print("  --nga        in hoa toan bo ket qua")
    print("  --giup       in huong dan nay roi thoat")

fn gia_tri_chu_so(c: str) -> int:
    let digits = "0123456789".chars()
    var i = 0
    while i < digits.len():
        if digits[i] == c:
            return i
        i += 1
    0 - 1

fn parse_so(s: str) -> int?:
    let cs = s.trim().chars()
    if cs.len() == 0:
        return None
    var i = 0
    var dau = 1
    if cs[0] == "-":
        dau = 0 - 1
        i = 1
    var kq = 0
    var co = false
    while i < cs.len():
        let d = gia_tri_chu_so(cs[i])
        if d < 0:
            return None
        kq = kq * 10 + d
        co = true
        i += 1
    if co:
        Some(kq * dau)
    else:
        None

fn chet(msg: str):
    print("loi: {msg}")
    print("dung: vis run bai5.vis -- --ten TEN [--so-lan N] [--nga] [--giup]")
    assert false, msg

fn main(args: list[str]):
    var ten: str? = None
    var so_lan = 3
    var nga = false
    var giup = false
    var i = 0
    var loi = ""
    while i < args.len() and loi == "":
        let a = args[i]
        if a == "--giup":
            in_huong_dan()
            giup = true
            i = args.len()
        elif a == "--ten":
            if i + 1 < args.len():
                ten = Some(args[i + 1])
                i += 2
            else:
                loi = "thieu gia tri cho --ten"
                i += 1
        elif a == "--so-lan":
            if i + 1 < args.len():
                match parse_so(args[i + 1]):
                    Some(n):
                        if n >= 1:
                            so_lan = n
                        else:
                            loi = "--so-lan phai >= 1, nhan duoc {n}"
                        ()
                    None:
                        loi = "--so-lan khong phai so nguyen: '{args[i + 1]}'"
                        ()
                i += 2
            else:
                loi = "thieu gia tri cho --so-lan"
                i += 1
        elif a == "--nga":
            nga = true
            i += 1
        else:
            loi = "tham so la: '{a}'"
            i += 1

    if giup:
        so_lan = 0
    elif loi != "":
        chet(loi)
    else:
        match ten:
            Some(t):
                var k = 0
                while k < so_lan:
                    let dong = "Xin chao {t}!"
                    if nga:
                        print(dong.upper())
                    else:
                        print(dong)
                    k += 1
            None:
                chet("thieu tham so bat buoc --ten")
```

Lệnh thật + exit code:
```
$ vis run bai5.vis -- --ten An                    → 3 dòng "Xin chao An!"            exit 0
$ vis run bai5.vis -- --ten Binh --so-lan 2 --nga → XIN CHAO BINH! (x2)            exit 0
$ vis run bai5.vis -- --ten "An Ba"               → (tên có khoảng trắng) 3 dòng   exit 0
$ vis run bai5.vis                                → lỗi (dưới)                     exit 101
$ vis run bai5.vis -- --so-lan 0 --ten An         → --so-lan phai >= 1             exit 101
$ vis run bai5.vis -- --ten An --so-lan abc       → khong phai so nguyen           exit 101
$ vis run bai5.vis -- --khong-co                  → tham so la: '--khong-co'       exit 101
$ vis run bai5.vis -- --giup                      → in hướng dẫn                   exit 0
$ vis run --vm --no-cache bai5.vis -- --ten An --so-lan 2 → cùng output,           exit 0
```
Trường hợp thiếu `--ten` — **stdout** (nguyên văn):
```
loi: thieu tham so bat buoc --ten
dung: vis run bai5.vis -- --ten TEN [--so-lan N] [--nga] [--giup]
```
**stderr** (nguyên văn): `bai5.vis:48:5: error VS0501: assert thất bại: thieu tham so bat buoc --ten` → exit `101`.

**Dependency**: `std.str` + 22 builtin (`crates/vis-interp/src/builtin.rs:279-386`) — **không có `eprint`/`eprintln`/`exit`**; `print` là kênh duy nhất.
**Workaround / cách "báo lỗi hợp lệ"**: thông điệp lỗi in bằng `print` (stdout), sau đó `assert false, msg` ⇒ trap `VS0501`, **exit 101** (VD-178 C1/R9; hiện `VS0502/VS0503` vẫn thoát 1). Không thể trả mã 2 cho lỗi runtime; `vis` chỉ có exit 0/1/2/101. `main` trả `Err` **không** đổi mã thoát (VD-178 R4 — `[DỰ KIẾN]`). Check-time error ⇒ exit 2. Cách thoát "êm" duy nhất cho đường lỗi là `return` + thông điệp — nhưng muốn non-zero buộc phải assert.
Chú ý: `--giup` in ra rồi bỏ qua mọi kiểm tra còn lại (đặt cờ, kết thúc vòng lặp) thay vì `return` giữa `if` (`return` trong nhánh `if` ⇒ `never` ≠ `()` ⇒ VS0412; xem mục chung).

---
## Bài 6 — Gọi tiến trình ngoài (đối số có khoảng trắng, stdout/stderr/code) — **CHẶN trong `.vis`; một phần trong `.vish`**

**`.vis` — không có API**:
```
$ printf 'use std.proc\n' > bai6g.vis && vis run bai6g.vis
bai6g.vis:1:1: error VS0705: không có module `std.proc` (có: str/list/map/path/time/json/fs/net/http)   exit 2

$ printf 'extern cmd chao(loi: str)\nprint(chao("xin chao"))\n' > bai6f.vis && vis run bai6f.vis
bai6f.vis:2:7: error VS0501: tên `chao` chưa có giá trị lúc chạy                              exit 101

$ printf 'print(^/bin/echo hi)\n' > bai6h.vis && vis run bai6h.vis
bai6h.vis:1:7: error VS0201: mong biểu thức, thật `^` (PUNCT)                                  exit 2
```
Evidence: `extern cmd` chỉ parse được (test `vis-syntax/tests/doc_fixtures.rs`, doc-example `test/doc-examples/doc-ViScript-thiet-ke-690.vis`), không có hiện thực lúc chạy. `^` là cú pháp lỗi trong `.vis`. Không có module `proc`/`exec` trong `MODULES` (`crates/vis-types/src/stdlib.rs:144-146`), không chữ ký `proc.*`/`exec.*`, không builtin spawn ngoài `exec.*` nội bộ của chế độ lệnh. `cap[Proc]` **có** nhưng chỉ dùng để *gác* lệnh ngoài trong `.vish` (`crates/vis-cli/tests/cap_proc.rs`).

**`.vish` (chế độ lệnh của ViScript) — chạy được, lấy được code/stdout/stderr, nhưng chỉ in, không để lại giá trị** (`bai6.vish`):
```vish
# SPDX-License-Identifier: MIT OR Apache-2.0
# Bài 6 — gọi tiến trình ngoài, đối số có khoảng trắng, stdout/stderr/code.

# (1) đối số có khoảng trắng (printf nhận một chuỗi duy nhất)
/usr/bin/printf "mot doi so co khoang trang\n" | complete

# (2) stderr của tiến trình con được giữ riêng
/bin/sh -c "printf 'loi vao stderr\n' >&2; exit 0" | complete

# (3) exit code != 0 vẫn "thành công" nhờ complete
/bin/sh -c "exit 7" | complete
```
Lệnh: `vis run bai6.vish` → exit `0`
```
stdout  [mot doi so co khoang trang]
stderr  []
code    0
stdout  []
stderr  [loi vao stderr]
code    0
stdout  []
stderr  []
code    7
```
Không có `| complete` (`bai6b.vish`: `/bin/sh -c "exit 7"`):
```
bai6b.vish:1:1: error VS0501: chương trình ngoài `/bin/sh` thoát mã 7        exit 101
```
Lỗi tiến trình với complete (`bai6c.vish`: `/bin/cat /khong/co/that | complete`): `code 1`, `stderr [cat: /khong/co/that: No such file or directory]`, exit `0`.
strict-caps: `bai6d.vish` (`fn main(p: cap[Proc]):` + `pass` + pipeline) với `--strict-caps` ⇒ chạy, `code 0`, exit `0`; thiếu `cap[Proc]` (`bai6e.vish`) ⇒ `VS0604 [quyền] cần khai cap[Proc] cho /bin/sh`, exit `2`.
`--vm --no-cache bai6.vish` và VM == interp, exit `0`.

**Dependency**: không (chỉ chế độ lệnh tích hợp).
**Workaround/hạn chế cứng của `.vish`**:
- Kết quả `| complete` là **bản ghi** `{stdout: list[str], stderr: list[str], code: int}` — chỉ **in** tự động; **không gán được**: `let r = /bin/echo hi | complete` ⇒ `VS0501: tên 'r' chưa hỗ trợ ở VS0`, exit `101` (đã thử đủ `let r: …` / `let kq = …`).
- Không có hàm gọi lệnh ngoài trả `str`/`list[str]`/`Result` để xử lý trong code; pipeline không nằm được trong `fn`.
- Không thể ghi thẳng stdout của lệnh ngoài ra biến hay tệp (không có `>` redirection: `/bin/echo x > f` in nguyên văn `[x > f]`, exit 0).
⇒ Không có cách hợp pháp lấy `{stdout, stderr, code}` vào chương trình `.vis`. **Đây là điểm chặn của bài**; `.vish` chỉ đủ cho kịch bản xem-thông-tin-bằng-mắt.

---
## Bài 7 — Nhiều module: record + enum + trait — **XANH (workaround: dispatch phương thức không xuyên module)**

Đề: module `geometry` khai `pub type Point`, `pub enum Hinh`, `pub trait DoLuong`, `impl DoLuong for Hinh`; `main.vis` `use ./geometry` và in diện tích/chu vi các hình.

`bai7/geometry.vis`:
```vis
# SPDX-License-Identifier: MIT OR Apache-2.0
# Mô-đun hình học: bản ghi Point, enum Hinh, trait DoLuong + impl.
pub type Point:
    x: float
    y: float

pub enum Hinh:
    Tron(r: float)
    ChuNhat(rong: float, cao: float)
    TamGiac(a: Point, b: Point, c: Point)

pub trait DoLuong:
    fn dien_tich(self) -> float
    fn chu_vi(self) -> float

pub fn xa(p: Point, q: Point) -> float:
    let dx = p.x - q.x
    let dy = p.y - q.y
    (dx * dx + dy * dy) ** 0.5

impl DoLuong for Hinh:
    fn dien_tich(self) -> float:
        match self:
            Tron(r):
                3.14159 * r * r
            ChuNhat(rong, cao):
                rong * cao
            TamGiac(a, b, c):
                let t = a.x * (b.y - c.y) + b.x * (c.y - a.y) + c.x * (a.y - b.y)
                if t < 0.0:
                    0.0 - t / 2.0
                else:
                    t / 2.0

    fn chu_vi(self) -> float:
        match self:
            Tron(r):
                2.0 * 3.14159 * r
            ChuNhat(rong, cao):
                2.0 * (rong + cao)
            TamGiac(a, b, c):
                xa(a, b) + xa(b, c) + xa(c, a)

pub fn dien_tich_hinh(h: Hinh) -> float:
    h.dien_tich()

pub fn chu_vi_hinh(h: Hinh) -> float:
    h.chu_vi()
```
`bai7/main.vis`:
```vis
# SPDX-License-Identifier: MIT OR Apache-2.0
# Bài 7 — chương trình nhiều module: bản ghi + enum + trait, import qua `use ./module`.
use ./geometry

fn main():
    let p1: Point = {x: 0.0, y: 0.0}
    let p2: Point = {x: 3.0, y: 0.0}
    let p3: Point = {x: 0.0, y: 4.0}
    let cac_hinh: list[Hinh] = [Hinh.Tron(2.0), Hinh.ChuNhat(3.0, 4.0), Hinh.TamGiac(p1, p2, p3)]
    var tong_dien_tich = 0.0
    var tong_chu_vi = 0.0
    var i = 1
    for h in cac_hinh:
        let dt = dien_tich_hinh(h)
        let cv = chu_vi_hinh(h)
        print("hinh {i}: dien tich = {dt:.4}  chu vi = {cv:.4}")
        tong_dien_tich = tong_dien_tich + dt
        tong_chu_vi = tong_chu_vi + cv
        i += 1
    print("tong dien tich = {tong_dien_tich:.4}")
    print("tong chu vi    = {tong_chu_vi:.4}")
```

Lệnh: `vis run bai7/main.vis` → exit `0`
```
hinh 1: dien tich = 12.5664  chu vi = 12.5664
hinh 2: dien tich = 12.0000  chu vi = 14.0000
hinh 3: dien tich = 6.0000  chu vi = 12.0000
tong dien tich = 30.5664
tong chu vi    = 38.5664
```
(π≈3.14159; tam giác 3-4-5 có S=6.0, CV=12.0.) `--vm` → giống hệt, exit 0. `vis check bai7/main.vis` → `ok 1`.

**Dependency**: không (chỉ module cục bộ `./geometry`).
**Chỗ phải workaround / điểm chưa hỗ trợ**:
1. **`impl Trait for Type` không xuyên module**: để `h.dien_tich()` trong `main.vis` ⇒ `main.vis:14:18: error VS0405: enum không có biến thể 'dien_tich'`. Gán kiểu tĩnh cho biến vòng lặp (`let hh: Hinh = h`) ⇒ `VS0403: trait 'DoLuong' có phương thức 'dien_tich' nhưng 'Hinh' chưa cài (impl DoLuong for Hinh)`. Impl có trong `geometry.vis` nhưng **đăng ký impl là cục bộ từng module** (`impls` khởi tạo rỗng mỗi lần lowering — `crates/vis-types/src/lower.rs:272,346`; `dispatch_tinh` ở `crates/vis-types/src/lower/expr.rs:700-776` chỉ xét `self.impls` của module hiện tại).
2. **Quy tắc "mồ côi"**: đặt `impl DoLuong for Hinh` ở `main.vis` (khác cả tệp khai trait lẫn type) ⇒ `VS0403: mồ côi: trait 'DoLuong' và kiểu 'Hinh' đều ở module khác — 'impl' phải ở tệp khai trait hoặc kiểu (VD-98)`. ⇒ impl chỉ hợp lệ ở đúng file `geometry.vis`.
3. Hệ quả bắt buộc: dispatch phương thức **chỉ hoạt động trong cùng module với impl** (`h.dien_tich()` bên trong `dien_tich_hinh` ✔). Muốn gọi từ module khác phải bọc `pub fn` tự do (`dien_tich_hinh`, `chu_vi_hinh`) — đó là mẫu duy nhất chạy được hiện tại.
4. Biến vòng lặp `for h in cac_hinh:` không mang kiểu tĩnh ⇒ mọi `h.x()` cần bọc hàm hoặc dùng free fn (xem (1)).
5. **`use ./module`** (không có `use ... as alias` — grep repo không có `use ... as `): tên pub của module được gộp thẳng vào namespace
   (`pub fn`, `pub type`, `pub enum`, `pub trait` đều thấy được; `xa` nếu không `pub` ⇒ `main: ... error VS0301: không thấy tên 'xa'`).
6. Dựng variant enum xuyên module: `Hinh.Tron(2.0)` ✔; nullary (`Hinh.Trong` không có ở đây; xem bài 3 — trần `Trống` ⇒ VS0501, phải `S.Empty`).
7. Pattern `Tron(r)` bên trong module định nghĩa enum hoạt động bình thường (cùng-file).

---
## Bài 8 — Collection + closure + Option + Result trong một pipeline có nghĩa thật — **XANH**

Đề: từ `list[SinhVien]` (điểm `int?`, `None` = vắng), lọc sinh viên đạt, map sang mô tả, thống kê (si số, trung bình, min/max), lỗi validate được truyền ra bằng `?`.

```vis
# SPDX-License-Identifier: MIT OR Apache-2.0
# Bài 8 — filter + map + ?? + match + ? trên một pipeline có nghĩa thật:
use std.list
use std.str

type SinhVien:
    ten: str
    diem: int?

fn xep_loai(d: int) -> str:
    if d >= 9:
        "gioi"
    elif d >= 7:
        "kha"
    elif d >= 5:
        "trung binh"
    else:
        "yeu"

fn kiem_tra_diem(d: int, ten: str) -> Result[int, str]:
    if d < 0 or d > 10:
        Err("diem ngoai 0..10 cho {ten}: {d}")
    else:
        Ok(d)

fn dat(sv: SinhVien) -> bool:
    match sv.diem:
        Some(d):
            d >= 5
        None:
            false

fn mo_ta(sv: SinhVien) -> str:
    match sv.diem:
        Some(d):
            let hien = sv.diem ?? -1
            "{sv.ten}: {d} diem ({xep_loai(d)}) [?? cho thay gia tri: {hien}]"
        None:
            "{sv.ten}: vang mat"

fn bao_cao(ds: list[SinhVien]) -> Result[str, str]:
    if ds.len() == 0:
        return Err("danh sach trong")

    # validate mọi điểm trước, dùng `?` để thoát sớm khi sai
    for sv in ds:
        match sv.diem:
            Some(d):
                let ok_d = kiem_tra_diem(d, sv.ten)?
                ()
            None:
                ()

    var tong = 0
    var co_diem = 0
    var vang = 0
    var dat_ds = ds.filter(dat)
    let chi_tiet = ds.map(mo_ta)

    var cac_diem: list[int] = []
    for sv in ds:
        match sv.diem:
            Some(d):
                cac_diem.push(d)
                tong += d
                co_diem += 1
            None:
                vang += 1

    let tb = if co_diem == 0: 0 else: tong // co_diem
    let thap_nhat = if cac_diem.len() == 0: 0 - 1 else: list.sort(cac_diem)[0]
    let cao_nhat = if cac_diem.len() == 0: 0 - 1 else: list.sort(cac_diem)[cac_diem.len() - 1]

    var lines: list[str] = chi_tiet
    lines.push("----------------------------")
    lines.push("si so: {ds.len()}, co diem: {co_diem}, vang: {vang}")
    lines.push("dat (>=5): {dat_ds.len()}")
    lines.push("diem trung binh: {tb} | thap nhat: {thap_nhat} | cao nhat: {cao_nhat}")
    Ok(str.join(lines, "\n"))

fn main():
    let a: SinhVien = {ten: "An", diem: Some(9)}
    let b: SinhVien = {ten: "Binh", diem: None}
    let c: SinhVien = {ten: "Chi", diem: Some(6)}
    let d: SinhVien = {ten: "Dung", diem: Some(4)}
    let e: SinhVien = {ten: "Em", diem: Some(8)}
    let ds: list[SinhVien] = [a, b, c, d, e]

    match bao_cao(ds):
        Ok(r):
            print(r)
        Err(loi):
            print("loi bao cao: {loi}")
```

Lệnh: `vis run bai8.vis` → exit `0`
```
An: 9 diem (gioi) [?? cho thay gia tri: 9]
Binh: vang mat
Chi: 6 diem (trung binh) [?? cho thay gia tri: 6]
Dung: 4 diem (yeu) [?? cho thay gia tri: 4]
Em: 8 diem (kha) [?? cho thay gia tri: 8]
----------------------------
si so: 5, co diem: 4, vang: 1
dat (>=5): 3
diem trung binh: 6 | thap nhat: 4 | cao nhat: 9
```
Đường lỗi: đổi `Some(9)` → `Some(99)` (bản tạm) ⇒ `loi bao cao: diem ngoai 0..10 cho An: 99`, exit `0` (main nuốt `Err` in ra). `--vm` → cùng output interp, exit 0. `vis check bai8.vis` → `ok 1`.

**Dependency**: `std.list`, `std.str` + builtin (`print`). **Ghi chú/workaround**:
- Pipeline: `ds.filter(dat)` / `ds.map(mo_ta)` / co `??` (`sv.diem ?? -1`) / `match` trên `T?` / `?` (`kiem_tra_diem(d, sv.ten)?`) — đúng các nguyên liệu yêu cầu; `filter`/`map` chạy **ngay** (eager) trên list.
- `xs.join("-")`/`xs.map`/`xs.filter` là method trên `list` (không cần `use`), còn `list.sort` là hàm module (`use std.list`) — hai kiểu gọi khác nhau, dễ lẫn.
- **Không phải bug, nhưng lập trình theo nhánh**: mọi `match`/`if`/`elif` cạnh nhau phải **đồng kiểu**; `xs.push(v)` trả **list** (không phải unit), `()` trần là **tuple rỗng** (checker ra `tuple[]`, không `()`), assignment (`x = y`) ra `()` ⇒ nhánh trộn nhau ⇒ `VS0412 hai nhánh khác kiểu`. Cách ổn định: nhánh "không làm gì" kết thúc bằng một phép gán (hoặc `print`), không viết `()`.
- `tong // co_diem` (`//` = floor-div int, R7) — `tong / co_diem` ra float theo R7, dùng `//` để giữ int.
- `?` chỉ truyền được `Result[T, E]` với `E` khớp (ở đây `E = str` cả hai) — không có `anyhow`/box lỗi; `AnyError` là alias bản ghi `Error` (VD-231) dùng được nhưng cần kiểu tĩnh.
- `main` nhận `Err(…)` không đổi exit code (VD-178 R4 `[DỰ KIẾN]`) — phải `match` thủ công như trên.

---
# Tổng hợp

| # | Bài | Lệnh | Trạng thái |
|---|-----|------|------------|
| 1 | Đọc/lọc/ghi file | `vis run bai1.vis` | **XANH** (workaround: đọc cả tệp, không `fs.lines`) |
| 2 | Ghép CSV theo ID | `vis run bai2.vis` | **XANH** (workaround lớn: không CSV lib, không `int(str)`) |
| 3 | JSON optional → transform → JSON | `vis run bai3.vis` | **XANH trên interp**; **VM hỏng pattern lồng** (chi tiết bên dưới) |
| 4 | HTML → Markdown (giữ URL) | `vis run bai4.vis` | **XANH** (workaround lớn: không HTML/regex lib) |
| 5 | CLI: required + default + bool flag + lỗi | `vis run bai5.vis -- --ten An` | **XANH** (workaround: không eprint/exit → assert + VS0501, exit 101) |
| 6 | Gọi tiến trình ngoài, có khoảng trắng, stderr/code | `vis run bai6.vish` | **CHẶN trong `.vis`; `.vish` in được record nhưng không gán được biến** |
| 7 | Multi-module: record + enum + trait | `vis run bai7/main.vis` | **XANH** (workaround: impl không xuyên module → bọc `pub fn`) |
| 8 | Collection + closure + Option + Result | `vis run bai8.vis` | **XANH** |

`vis check` xanh cho mọi `.vis`; `--vm --no-cache` cho cùng byte output với interp ở bài **1, 2, 4, 5, 7, 8** (diff đã kiểm); bài 3 lệch (dưới).

# Những điểm bị chặn / chưa hỗ trợ (bằng chứng)

1. **`std.proc` / subprocess trong `.vis` — không có.** `use std.proc` ⇒ `VS0705: không có module std.proc (có: str/list/map/path/time/json/fs/net/http)`. `extern cmd` parse được nhưng "tên chưa có giá trị lúc chạy" (`VS0501`). `^` ⇒ `VS0201`. `.vish`: external command chạy được, lấy `code`/`stdout`/`stderr` qua `| complete` (in bản ghi), strict-caps gác bằng `cap[Proc]`; nhưng **kết quả không gán được vào biến** (`let r = … | complete` ⇒ `VS0501`). (Bài 6)
2. **Pattern enum lồng trên VM — không khớp.** `Some(Str(t))` trượt sang catch-all; `match Some(Some(5))` **không chạy nhánh nào**; `Some(A(n))` (enum người dùng) trượt; `json.encode` trên giá trị `Json` do code dựng ra cho `{"variant":…}` thay vì JSON thật. Interp đúng hết. Repro: `/tmp/opencode/ergo-b/t19.vis`, `t23.vis`, `t26.vis`, `t29.vis` (`vis run` vs `vis run --vm --no-cache`). (Bài 3)
3. **`Null` không dựng được từ code** (`VS0501`); enum nullary phải `S.Empty`; `Option::is` pattern `Some(…)`/`None` thì pattern dùng trần được. (Bài 3, mục 5 API)
4. **`impl Trait for Type` đăng ký cục bộ theo module**, dispatch phương thức không xuyên module ⇒ phải bọc `pub fn` khi gọi chéo module (gọi phương thức trong cùng module vẫn OK). Quy tắc "mồ côi" cấm đặt impl ở module thứ ba (VD-98). (Bài 7)
5. **Không có `int(str)`/`float(str)`** trong ngôn ngữ ⇒ tự viết parser số khi cần đọc số từ văn bản. (Bài 2, 5)
6. **Không có eprint/stderr/exit trong ViScript** (22 builtin: `print assert len str echo lines take ls open render get select sort_by where each first count Ok Err Some None ref`) ⇒ lỗi "hợp lệ" = thông điệp stdout + `assert false, "msg"` ⇒ `VS0501`, exit 101 (R9/C1); `main` trả `Err` không đổi exit code (VD-178 R4 `[DỰ KIẾN]`). (Bài 5)
7. **`()` trần ≠ `()`**: checker (`tyck/expr.rs`) gõ `()` là `tuple[]`; lowering lại gõ unit (`lower/expr.rs`); `unify_branch` chỉ chấp nhận trùng khớp/sai `Error` ⇒ nhánh `match` trộn `()` và gán ⇒ `VS0412`. Cách đúng: nhánh rỗng kết thúc bằng phép gán hoặc `print`. (Bài 2, 3, 4, 8)
8. **Thiếu API mong đợi**: `fs.lines`, `fs.read_text`, `str.find`/slice, `int(s)`, eprint/exit, `Json` ctor đầy đủ từ code (`Null`), set (phải dùng `map[str, bool]`), regex. (Bài 1, 2, 4, 5, 8)
9. **Cú pháp dễ vấp**: biến thể dựng phẳng (`Some(Some(x))` bị VM làm phẳng — liên quan bài 3); `for h in xs` biến vòng lặp không có kiểu tĩnh (dispatch phương thức cần chú thích `let hh: Hinh = h`); `match` arm phải xét hết; dùng `//` cho chia nguyên; `let` bất biến.
10. **strict-caps**: mọi truy cập `std.fs`/`net`/lệnh ngoài cần khai `cap[Dir] @ p"…"`/`cap[Proc]` trong chữ ký `fn main`; nếu không ⇒ `VS0604`, exit 2 — đã kiểm tra trên bài 1, 6.

# Ghi chú chung
- **interp ≠ VM** trên dải dựng `Json`/pattern lồng. Toàn bộ bài xanh ở trên đều xác nhận lại bằng `--vm --no-cache`.
- `vis check` = preflight (TAP), không sinh file; `vis run` thực thi; `vis test test/sem` là suite của repo (không chạy ở đây).
- Không có `--help`; dùng `vis help` (xem đầu task).
- Thư mục scratch `/tmp/opencode/ergo-b/` giữ nguyên code + dữ liệu + repro t19…t29 + `BAO-CAO.md` (tệp này); repo không bị đụng.
