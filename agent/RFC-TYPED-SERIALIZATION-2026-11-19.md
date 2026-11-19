# RFC — Typed serialization / deserialization cho ViScript (A0 2026-11-19)

> **Trình A0 · từ Agent 1 (ps) · 2026-11-19 · trạng thái: NHÁP — chưa duyệt.**
> Không dòng mã compiler nào bị thay đổi bởi tài liệu này (chỉ ghi nhận hiện trạng + đề xuất).
> Implementation **chưa được duyệt**; chờ A0 chốt các quyết định ở §11 trước khi mở VD triển khai.

## 0. Câu hỏi A0 đã chốt trước (nguyên văn từ A0)

- Typed serialization là **hướng cần thiết** để cải thiện công thái học: người dùng khai báo kiểu một lần, encode/decode trực tiếp, không phải tự `match Json` cho từng field.
- **MVP trước cho typed JSON**; YAML/TOML, typed CSV và `join` **ngoài MVP**; parse số, stderr, chuỗi cơ bản có thể là task riêng, **không chờ** framework serialization.
- **Trait/dispatch trước, derive sau** — derive là đường giảm code lặp, không được có ngữ nghĩa khác code người dùng viết tay.
- **Quyết định dữ liệu trước cú pháp**; không hứa round-trip Option lồng; không mở macros; không gộp YAML/TOML vào MVP; đối chiếu các task hiện có để tránh mở trùng và báo rõ phụ thuộc B-3 + fixes JSON/VM đã có.

## 1. Mục tiêu người dùng (đích UX, không phải cú pháp được duyệt)

```vis
type User derives Json:        # tên derive CHƯA chốt
    name: str
    email: str?
    tuoi: int?

let user = json.decode[User](text)?   # illustration only — CHƯA duyệt
let out = json.encode(user)?
```

- Một lần khai báo `type`, dùng được encode/decode.
- Không cần macro người dùng; compiler sinh implementation từ derive (giống ST-60c đã làm cho `Show`/`Eq`).

## 2. Hiện trạng thật (bằng chứng source/test tại `3b2c67e` = `main` hiện tại)

| Hạng mục | Hiện trạng (bằng chứng) | Hệ quả cho RFC |
|---|---|---|
| **Dispatch trait tĩnh** | `crates/vis-types/src/lower/trait_impl.rs` module-doc + `./vis` tests: target Record/Enum tĩnh → `Call(impl_fn)`, chèn `self` đầu; **không có** bảng impls chạy; **không bump VBC1** (ST-29) | Cùng đường sinh impl viết tay = impl sinh — cheap parity, không cần runtime table |
| **`derives`**: chỉ `Eq`, `Show`, `Json` | `trait_impl.rs:kiem_derives` (~L133): không có `derive` nào khác được cấp; `Json` chỉ kiểm trường/biến thể kiểu hàm → `VS0403`; **không sinh mã** | `derives Json` hiện là **marker**, không có `to_json`/`from_json` — verify bằng `probe`: `Hop` có/không `derives Json` đều `json.encode(h)` ⇒ `{"gia_tri":1}` |
| **Sinh mã derive (precedent)** | `sinh_show_enum` (~L514), `sinh_show` (~L540), `sinh_eq` (~L591) — sinh HFn thật, đăng ký `impl_pairs`; người dùng viết tay **thắng** nếu trùng tên (`sinh_pt` ~L460, VD-93(4)); generic `derives` ⇒ `VS0403` tĩnh | Derive codegen **đã có hạ tầng** (`sinh_pt`), không cần macros; derive mới = +`sinh_to_json`/+`sinh_from_json` cùng mẫu |
| **Static impl position** | `kiem_impl_khop`: inherent `impl Type:` — kiểu phải khai cùng tệp; `impl Trait for Type:` — trait phải định nghĩa trong tệp đang xem, `impl` ngoài tệp định nghĩa kiểu ⇒ **mồ côi** `VS0403`; trait ngoài tệp cũng mồ côi (module propagation chưa có) | Cross-module impl **chưa có** — dependency **B-3** (xem §10) |
| **Bound generic** | `fn showme[T: Show](x: T)` — **không enforcement** (`let a = (1,2); showme(a)` ⇒ chạy; không trait `Show` cho tuple) | Bounds là khoảng trống sẵn có — dependency **B-3** |
| **`std.json.encode`** | `crates/vis-interp/src/stdlib.rs` `"json.encode"`/`"json.encode_pretty"` (~L393) — runtime **walk Value**, không per-type; record → object (tên trường, thứ tự khai báo); enum `Json` → JSON đúng; `Option`: `None` ⇒ `null`, `Some(x)` ⇒ encode `x`; `Result` ⇒ `{"ok": v}`/`{"err": e}`; `Unit`→`?`; `Size/Duration`→int; `Bytes`? → **chưa đo** (cần khoá) | Encode hiện coi như "serialize Value" — typed encode có thể giữ nguyên đường này cho MVP |
| **`std.json.decode`** | ST-33 (`crates/interp/src/json.rs` parser); chữ ký tại `crates/vis-types/src/stdlib.rs` ⇒ **`Result[Json, Error]`** (VD-222 điểm 4, VD-187 dòng buộc); EnumId(0) built-in `Json` enum: `Null`/`Bool`/`Num`/`Str`/`Arr`/`Obj`; VD-66 "khớp exhaustive" — **bắt buộc `match`** để đọc ra field | Typed decode = thay `match` bằng đường tự động — điểm khác biệt chính |
| **JSON parity hai engine** | `vd_205::json_encode_owned_value_engine_parity` (VM) khoá bởi VD-241 §3/T3 (lift `$lift-opt`, `MakeSome` frozen) | Golden mới không bị lệch VM/interp — tiền đề bắt buộc cho MVP |
| **Lỗi** | Built-in record `Error` (RecId(0), `message/source/span`; in `print` chỉ `{message}`); `AnyError = alias Error` (VD-222); diagnostic path format **chưa** có (`users[3].age` chưa được khoá) | Decode error cần chốt format/path — §5-F |
| **Missing vs null vs Option lồng** | `Option[Option[int]]` = `Some(None)`/`Some(Some(42))`/`None` — 3 trạng thái phân biệt được; encode(`Some(None)` → `null`; `Some(Some(42))` → `42`) | **Round-trip Option lồng qua JSON `null` hiện tại KHÔNG giữ đủ 3 trạng thái** — tuyên bố up-front §3, §5-C |
| **SPEC/API freeze** | `api_dong_bang.rs` freeze; `docs/SPEC-lex.md` chỉ qua A0 | RFC không đụng từ vựng; mã lỗi mới (nếu có) là **thay đổi API** ⇒ cần ratify |
| **Phase order** | VD-241 §: tính đúng → kiểu & nhất quán → script/process → stdlib → tools | Typed JSON thuộc **kiểu/nhất quán** — sau `3b2c67e` |

> Ghi chú: mọi "probe" trong bảng đã chạy thật bởi Agent 1 (2026-11-19) với `./target/debug/vis` sau `3b2c67e`.

## 3. Phát biểu dữ liệu trước cú pháp (các câu hỏi phải chốt trước khi thêm từ khoá/tên)

A. **Missing vs null vs Some(None)** — ba trạng thái khác nhau, encode map:
    - MVP đề xuất: `Option` encode `null`; decode `null` ⇒ `None`; `Some(x)` ⇒ encode(x) giữ nguyên một lớp (hiện tại giữ nguyên, VD-204). **Không hứa round-trip Option lồng**: `Some(None)` và `None` đều → `null` ⇒ decode lại đều `None`. Nếu A0 muốn round-trip `int??` → cần field-omission/object wrapper hoặc từ chối cam kết — **ô quyết §11-D**.
    - Missing required record field khi decode ⇒ error + vị trí path (§F).
    - Field có default (`age: int = 30`) khi missing ⇒ điền default (đúng hiện trạng record default machinery).
B. **Enum representation** — đâu là default hợp lý? `{"type":"Variant","fields":{…}}` (internally tagged) / `{"Variant":{…}}` (externally tagged, serde default) / untagged. **Cần A0 chốt** — ảnh hưởng cả golden cả UX; **ô quyết §11-E**.
C. **Option lồng (khẳng định up-front trước)**: JSON không biểu diễn được 3 trạng thái qua `null`. Giải pháp duy nhất: chọn một trong (a) không round-trip Option lồng — `None` ngoài thành `null` làm mất `Some(None)`; (b) field omission cho `None` outer tại record field level (bỏ key) — decode khớp lại đúng variant theo cấu trúc, nhưng không dùng được cho value `let x: int?? = None` ngoài record; (c) custom marker. **RFC không tự chọn — ô quyết §11-D.**
D. **Field thừa ở input** — bỏ qua mặc định (serde default) vs lỗi (`VSxxxx`); cần gồm observable logic để user chọn chính sách cho record (ví dụ `#[deny_unknown_fields]`-style nếu ViScript mở attribute sau này — MVP nên chỉ có một mặc định, mở rộng sau). **Ô quyết §11-F.**
E. **Kiểu số** — `int` = i64; encode `int` > i64::MAX·2^63 không biểu diễn được (JSON 2^53 precision) — xử lý ngoại vi: encode báo `Result[str, Error]` lỗi range · hoặc truncate theo luật (không được im lặng). `float` NaN/±Inf — không có JSON representation; encode → lỗi · hoặc encode `null` (sai nghĩa) · hoặc "quoted" (sai kiểu). `-0.0`, subnormals. — **Ô quyết §11-G.**
F. **Đường dẫn lỗi (JSONPath-ish)** — `users[3].age` đồng nhất interp/VM; cần định dạng chuẩn diagnostic của ViScript (mã mới `VS05xx/VS07xx`? hay dùng `VS0403/VS0412` mở rộng thông điệp?); **ô quyết §11-H**.
G. **Duplicate keys** ở object đầu vào — giữ `last wins` (hiện parser parser nắm gì chưa đo) vs lỗi; **ô quyết §11-I.**
H. **Depth limit** — chống stack explosion khi decode; hiện parser có đệ quy hay là trampoline? (chưa đo — cần đo khi thiết kế); **ô quyết §11-J.**
I. **String encoding** — UTF-8; control chars escape theo JSON spec (raw `/` tuỳ ý); `\u0000`–\u001f" bắt buộc; **literal `\n` real newlines không phải JSON hợp lệ** — thống nhất encoder/decoder (interp/VM byte-identical theo T3); **ô quyết §11-K (chỉ đo, không quyết thêm).**
J. **`Unit` / `bytes` / `char` / `size` / `duration`** — cần khoá encode cho từng kiểu; **để A0 chốt ma trận 5 kiểu trên + `bytes` trong MVP** (hiện `bytes` encoding là base64-worthy? quyết §11-L).

## 4. Hai phương án kiến trúc (A0 yêu cầu so sánh, không chốt)

### PA-A — Trait chuyển đổi JSON-specific (kiểu "ToJson/FromJson")

- Shape: `trait JsonEncode`/`trait JsonDecode` (tên hẵng chưa chốt) hoặc hai trait riêng biệt (`ToJson` + `FromJson`).
- Ưu: nhỏ, đúng MVP; derive sinh trực tiếp cần match `Json` enum (có sẵn) hoặc serialize qua trait hàm; không khoá vào format-chung sai từ đầu; ít trait mới.
- Khuyến: enum mapping cần quyết định nay (§3-B); trait riêng cho JSON khác biệt với "multi-format" — nếu sau này YAML/TOML/Csv-typed đi vào, thêm `YamlEncode`/khách tương tự, có thể trùng cơ chế walker; `Result`/`Error` mapping dễ quyết hơn PA-B.
- Chi phí: nếu sau này thêm format thứ hai, phải nhân trait hoặc refactor lên PA-B — khi đó phải có VD migration.
- Bạn dạng nhẹ: **dùng lại đường runtime-value ở `stdlib.rs` cho encode** (không cần trait cho encode) + **chỉ thêm `decode[T]` kiểu tĩnh mới** — tối giản thêm bớt API.

### PA-B — Trait serialization/deserialization chung (serde-style `Serialize`/`Deserialize`)

- Shape: `trait Serialize`/`trait Deserialize` + driver per format (`JsonDriver`…); `fn serialize(self, fmt) -> Result[str, Error]`; `from_str[<T: Deserialize>](s) -> Result[T, Error]`.
- Ưu: một trait cho mọi format; codegen theo cùng mẫu; phù hợp đích dài hạn (YAML/TOML/CSV typed nếu A0 duyệt về sau).
- Khó: **lớn hơn cần thiết** cho MVP; buộc chốt `data model` generic (tag/content/seq/map visitor) — rất nhiều ẩn số chồng chất thêm (self-describing vs schema-driven); tăng nguy cơ "framework quá lai" trước khi có đủ format thật; nhiều khái niệm mới rơi vào user; `Deserialize` binding với `&str`/owned Strings/tokens — ViScript owned, lifetimes k có ⇒ dễ hơn.Platform, nhưng vẫn cần driver thiết kế.
- Chi phí thực của MVP v1 PA-B: driver JSON + 2 trait + sinh code + kiểm chứng — ước lượng gần gấp đôi scope PA-A; lợi ích chỉ hiện khi format thứ hai xuất hiện.

**Bảng tổng quát:**

| Tiêu chí | PA-A JSON-specific | PA-B serde-style |
|---|---|---|
| Scope MVP | **Nhỏ (khuyến nghị MVP)** — impl ~ trait + derive gen | Lớn — driver + 2 trait + visitor |
| Thêm format mới | Thêm trait mới × N (tái dùng codegen) | Thêm driver mới |
| Khoá semantics | Rủi ro khoá JSON sớm | Trừu tượng dài hạn nếu cần |
| Độ phức tạp cho user | Ít hơn | Nhiều khái niệm (visitor, seq, field) |
| Tương thích derives hiện có | Sinh từ ST-60c sinh `impl` cùng tệp | Cùng mẫu ST-60c |
| Rủi ro lớn nhất | Phải refactor nếu format 2 tới (flag nhánh) | Framework lớn trước khi cần |

**Không đặt`!` — A0 chốt** (§11-A/B chọn 1 trong 2 (hoặc "PA-A với đường upgrade PA-B")).

### Cây quyết định (gợi ý, chờ A0)

1. PA-A trait `ToJson`+`FromJson` → MVP typed JSON (decode `json.decode[User](text)`, encode `json.encode(user)` không đổi API) ← **phương án mỏng nhất**;
2. PA-B `Serialize`/`Deserialize` + JSON driver ← chỉ nếu A0 đã xác định YAML/TOML trong tương lai gần.

## 5. API — chỉ là minh hoạ một số biên; KHÔNG phải quyết định

Illustration usage (không dùng chữ ký này trừ khi A0 duyệt):

```vis
let user = json.decode[User](text)?     # kiểu tham số đặt ở call-site (Vec<T> style như decode[User])
let out  = json.encode(user)?           # hiện trạng encode trả Str
```

**Câu hỏi API ép A0 chốt (§11) — ghi rõ không mượn mượn:**

1. **`json.decode[User](text)` với chỉ một tham số vị trí** hay **`json.decode::<User>(text)`** — ViScript chưa có từ khoá `::` cho turbofish; `<User>` phải lex/parse theo grammar hiện có (`decode[User](...)` hợp lệ về mặt văn phạm?). (s: có — `list[int]` vs `decode[User]` — cần kiểm tra văn phạm hiện tại trước khi chốt — đã có ca `assoc` trong tier điển phạm? Chưa đo.)
2. **Suy luận kiểu trả `decode`** — cast `let user: User = json.decode(text)?` (không cần turbofish nếu có annot) vs turbofish bắt buộc; hay **pattern match `Json` vẫn hợp lệ** (VD-66) như hệ phản hồi lối cũ.
3. **Encode thành `str` hay `Result[str, Error]`** — encode bytes? `Unit`? ko-encode được? (A0 cảnh báo không mặc định encode luôn `str`).
4. **`json.encode(user)` với generic derive** — dispatch qua `to_json()` tự sinh — không đổi API; hay **`u.to_json()`** method style.
5. **Lỗi encode**: `Bytes` (encode base64? hay lỗi)? non-`str` map keys (hiện encode int keys ⇔ behaviour ko hợp đồng)?
6. **`Error` có `path` (§3-F)** — thêm field bắt buộc vào record `Error` (RecId(0) đã freeze `parent/subscope`? chưa) hay chỉ message chứa path (thỏa mãn §3-F không cần sửa layout `Error`).
7. **Số out-of-range / float NaN** — encode báo `Err` vs silent; decode báo `Err` vs truncate (JSON number parse hiện tại — chưa đo precision/range guard).
8. **Format đẹp** — `encode_pretty` là API phụ; cùng trait, chưa chốt MVP có cần `pretty` hay không.
9. **Thứ tự field khi encode** — khai báo thứ tự (hiện tại) hay nguồn — khuyến nghị giữ nguyên thứ tự khai báo cho ổn định golden; VM/interp byte-identical.
10. **Đo ngôn ngữ** — `json.decode_lines`? không vào MVP.
11. **`fs.lines`, `slice`, `join(on: "id")`, `csv.decode("data.csv")`** — **KHÔNG nằm trong phạm vi RFC này**; ghi nhận A0 đã xác nhận là ngoài MVP / cần chốt hợp đồng riêng; không use_any làm VP đi kèm.

## 6. Một implementation viết tay mẫu (user code) — không phụ thuộc derive

```vis
# type User derives ToJson, FromJson: (tên derive/trait chưa chốt)
type User:
    name: str
    email: str?
    tuoi: int?

# ≈ những gì derive sẽ sinh (codegen tái dùng sinh_pt ST-60c):
impl User:
    fn to_json(self) -> Json:
        # object { name, ... } với Option mapping §3-A
        ...

impl User:
    fn from_json(j: Json) -> Result[User, Error]:
        # match Obj(pairs):
        #   name: lấy Str(s) hoặc thiếu => Err
        #   email(b?: fehlt=> None / is Null? => None?) — chốt §3-A + §3-C
        ...

fn main() -> ():
    let t: str = "{\"name\":\"An\",\"email\":null,\"tuoi\":30}"
    let u: User = json.decode[User](t)?
    print(json.encode(u))
```

Hai viết đều tồn tại — nghề tay được hỗ trợ như đồng cấp (VD-93(4) "người dùng thắng"). Derive là **đường giảm code lặp**, không có ngữ nghĩa riêng.

## 7. Kế hoạch kiểm chứng (checker/interp/VM)

- Tầng corpus mới `test/sem/json-typed/`: encode→decode→re-encode giữ vàng hai engine byte-identical (áp dụng `json_encode_owned_value_engine_parity` pattern từ VD-241 T3).
- **90+ ca**: đầy đủ biên — record full/missing/extra, enum tags (theo §3-B), Option lồng (None/Some(None)/Some(Some(…))/hierarchy), Result mapping, float boundaries (NaN/±Inf per §11-G), int 2^53/2^63, string unicode, bytes (§11-L), depth attack, parse failures với path `users[1].age`, duplicate keys.
- **VM/interp parity byte-identical** bắt buộc mọi ca; two-engine independently verified (không lấy interp làm chuẩn).
- **Mutation testing** (VD-241 pattern): tắt `from_json` sinh ⇒ ca decode đỏ; tắt `to_json` ⇒ ca encode đỏ; đổi enum tag ⇒ golden đỏ.
- **Checker**: `json.decode[User](…)` không suy luận dưới annot ⇒ annot bắt buộc; annot khớp ⇒ verify trả về `User`; impl mồ côi ⇒ VS0403; generic bounds (`T: FromJson` chưa tồn tại — MVP không dùng bound generic) ⇒ không chặn MVP.
- **Gate**: `./vis-dev gate --fast` + `cargo test --workspace --no-fail-fast` + `vis test test/sem{, --vm}` + fresh-clone PASS; Windows/macOS re-verify sau khi merge (khuyến nghị mở lại review theo VD-241 Windows Tier 2).

## 8. Phạm vi MVP & hoãn

**MVP (v1, sau A0 duyệt)**:

1. Gen derive record/enum cho typed JSON (SN `sinh_pt` pattern), impl trong cùng tệp với kiểu (không phụ thuộc B-3).
2. `json.decode[T](s: str) -> Result[T, Error]` (đường call-site; không cần chung bound generic cho concrete type).
3. `json.encode` giữ nguyên API (trả `Str`); typed value lưu through `to_json()` trait call hoặc đường runtime Value — **không đổi** `json.encode` signature trừ khi A0 chốt §11-4 (encode ⇒ Result).
4. Bảng mapping primitive + Option lồng one-directional (§3-C, không round-trip hứa) + Result mapping (giữ VD-204 hiện trạng) + enum tags theo §3-B — **chỉ khi A0 chốt** Teams/Gates.
5. Error record extension với `path` (§3-F) — nếu A0 duyệt mở layout; hoặc message chèn path không đổi layout.
6. Corpus + parity gate theo §7.

**Hoãn (ngoài MVP)**:

- YAML/TOML/multi-format framework (PA-B) — cần format thứ hai thực sự.
- Typed CSV, `csv.decode(path)` (cần chốt nội dung vs đường dẫn), `join(on: …)` (design collection/query riêng).
- `fs.lines`, `slice`, `int(s)?` parse-edge semantics (parse trả Result hay trả int? khi overflow? khoảng signed/unsigned?) — task stdlib riêng, không chờ serialization.
- Generic bounds enforcement (`T: Show`) — **phụ thuộc B-3**, MVP không cần.
- Cross-module derive/impls — **phụ thuộc B-3**.
- Streaming/async decode, bytes base64 policy (§11-L), pretty gating (§11-4).
- Dynamic `map[str, T]` hetero decode (limit: JSON object với key không đồng nhất — MVP yêu cầu homogeneous shape).

## 9. Phụ thuộc & mở trùng task (không mở cấu trùng)

| Phụ thuộc | Trạng thái | Tương tác |
|---|---|---|
| **B-3** (impl xuyên module + bound generic) — **chưa bắt đầu**, chờ A0 ratify riêng | Blocker cho derive chạy cross-module / generic bounds | MVP không cần bound generic (chỉ concrete) — **không chặn MVP**; chặn "typed generic serde" |
| **JSON/VM fixes** — T1–T5, **đã merge** (3b2c67e + 349af67 Windows) | Done | Option lồng lặp runtime now correct for Sugar (`Some(None)` lift edges theo VD-241 §7 đúng theo lệnh A0); JSON VM encode now byte-identical |
| **Option lồng** (3b2c67e) | Done | `Option[Option[T]]` lồng thật; encoder giữ 3 trạng thái qua `null` **chưa giải quyết được** → §3-C up-front |
| **`#147`/`#152`** (mac) | Chờ máy rảnh | Không chặn RFC; re-verify nếu mở triển khai |
| **Windows Tier 2** | Đóng @ `349af67`; Windows chưa đo bản fix mới | §7 gate bắt buộc re-verify trước tuyên bố |
| **Phase order VD-241** | Tính đúng → kiểu & nhất quán → script/process → stdlib → tools | Typed JSON thuộc **kiểu/nhất quán** — đúng thứ tự sau `3b2c67e` |
| **`BAO-CAO A0 ~1521`/`OPTION-FIX`** | Lịch sử + báo cáo | Không mở lại |
| **`#151/#149` (pha 2)** | Giữ | Không liên quan RFC này |

## 10. Rủi ro & giảm thiểu

1. **Round-trip lỗ** (Option lồng, Unit, Numeric precision) → tuyên bố rõ trong docs; không hứa xa hơn mailman; golden hiển thị kèm ghi chú.
2. **Permission của `derive` sinh mã** → `sinh_pt` pattern "người dùng viết tay thắng" (VD-93(4)) — enforce ở mọi path; người dùng override được song song.
3. **API không ổn định** — freeze `api_dong_bang.rs` nếu áp dụng; nếu A0 không chốt API thì giữ **không công bố** thêm signature vào std.
4. **Phát sinh golden hàng loạt** → hạn chế ở §7 (chỉ corpus mới, không đụng golden cũ ngoài ca cùng nguyên nhân).
5. **B-3 trùng** — không mở thêm; nếu A0 ratify B-3 sau, RFC này được nốitiếp.
6. **Fill-change `Error`** — nếu không cho layook mở `Error` record layout, dùng message chèn path thay vì field (không phá freeze).
7. **Mutation under-covered** — mutation patterns định kỳ theo VD-241 đều được áp dụng.

## 11. Quyết định A0 cần (trước khi mở VD triển khai)

**A. Đường kiến trúc** — PA-A (JSON-specific, recommended MVP) hay PA-B (serde-style chung)?
**B. Tên trait/derive** — `ToJson`+`FromJson` / `JsonEncode`+`JsonDecode` / `Serialize`+`Deserialize` / khác; tên derive (giữ `Json` hay đổi); luật đụng với EnumId `Json`.
**C. Option lồng** — (a) không round-trip (`null ↔ None` mất `Some(None)`) · (b) field-omission cho None outer tại record field (decode khớp lại theo cấu trúc) · (c) custom marker · (d) phạm vi khác do A0.
**D. Enum tag representation** — externally tagged (`{"User": {…}}` serde default) · internally tagged (`{"type":"User",…}`) · untagged — hoặc cấu hình per-derive nếu A0 muốn.
**E. Field thừa** — bỏ qua mặc định · lỗi · bật tuỳ chọn record attribute.
**F. Encode failure semantics** — `json.encode` giữ `Str` infallible (dính bytes/non-str map keys) hay trả `Result[str, Error]` (align với `{"ok": v}` mapping)?
**G. Numeric boundaries** — int i64 → JSON 2^53/cap; `NaN`/`±Inf` encode thành `null` vs lỗi; decode sai type vs truncate; `-0.0`, subnormal. Trình bảng số cho A0.
**H. Error path format** — `users[3].age`-style trong `message` (gián tiếp) hay trường riêng `path: list[str|int]` trong record `Error`? Layout freeze hay mở layout + VD mới?
**I. Duplicate keys ở object input** — last wins · lỗi · first wins (default khuyến nghị: lỗi static? hay last?).
**J. Depth limit** — chọn số (128 mặc định?) hay tách riêng, bất lực test stack-depth.
**K. Decode API positioning** — `json.decode[User](text)` (turbofish-style tại call-site) hay `match json.decode(text): Ok(u: User) => …` (kiểu match như VD-66)? Cả hai đều dùng typeparam `decode[T]` hay khác nhau?
**L. MVP feature list** — (1) derive record/enum typed encode/decode; (2) decode[T]; (3) Option/Result mapping giữ nguyên; (4) Error path messages; (5) dump toche `encode_pretty`? Xác nhận/bỏ mục nào.
**M. Sequence & waivers** — thư viện thiết yếu (parse số, stderr, str) nên đi task riêng song song hay chờ RFC quyết xong?
**N. Windows/macOS Tier** — nếu A0 duyệt MVP, xác nhận re-verify bắt buộc trên cả hai máy trước đóng VD triển khai; hay được chấp nhận chỉ Linux khi chỉ là "typed derive dev-level"?
**O. SPEC/SPI** — cập nhật `docs/ViScript-thiet-ke.md` ("T?? là lỗi kiểu" §7.3 + tương đương ~4 chỗ) theo đúng luật A0 cho Option-lồng (VD-241 §7); RFC này chưa đụng — chờ A0.

## 12. Trạng thái tài liệu & theo dõi

- RFC này = bước trình chuẩn bị, không đổi mã; mọi sản phẩm code/gdir sẽ nằm trong VD riêng sau khi A0 duyệt §11.
- **Trạng thái**: chờ A0; các liên quan còn treo: B-3, typed-serde approval, lambda annotation (chờ A0), #147/#152 (mac idle), Windows Tier 2 bounds, `BAO-CAO-A0` Option fix §6.
- Người thực hiện thay đổi sau ratify: Agent 1 điều phối → Agent 2 implementer worktree.
