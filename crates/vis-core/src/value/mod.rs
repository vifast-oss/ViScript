// SPDX-License-Identifier: MIT OR Apache-2.0
//! **Giá trị lúc chạy** (`Value`) + `Show` (`str(x)` = `x.show()`, thiết kế §3.5) + đọc literal
//! `size`/`duration`/`percent` và escape chuỗi.
//!
//! # Đơn vị (chờ Agent 0 chốt — VD-53)
//!
//! - `size` lưu **byte** (`kb` = 1000, `kib` = 1024 — §3.1).
//! - `duration` lưu **nanosecond** (`ns us ms s m h d w`).
//! - `percent` lưu **tỉ lệ 0..1** (`50pct` = `0.5` — §3.6: "percent lưu tỉ lệ 0..1").
//! - `Show`: `size`/`duration` in theo **đơn vị lớn nhất chia hết** (`5000` ⇒ `5kb`,
//!   `250000000` ns ⇒ `250ms`), không chia hết thì về đơn vị gốc (`b`, `ns`); `percent` in
//!   `ratio*100` + `pct` (`0.5` ⇒ `50pct`).

use std::cell::{Cell, OnceCell, RefCell};
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::hash::{BuildHasherDefault, Hash, Hasher};
use std::ops::Deref;
use std::rc::Rc;

use vis_types::hir::{HBody, HPat};

mod runtime;
mod show_eq;

pub use runtime::{Chan, ChanSend, ChanSendState, Pending, Task, TrangThai, Wait};

thread_local! {
    /// **VD-259 (chỉ frame test)**: số lần phải dựng bản `'static` của thân closure.
    /// Một closure được gọi N lần chỉ được dựng **một** lần (xem `Closure::frame_body_static`).
    static FRAME_BODY_BUILDS: Cell<usize> = const { Cell::new(0) };
}

/// **VD-267** — FNV-1a cho bảng tên. Tên enum/biến thể rất ngắn; SipHash mặc định đắt gấp
/// nhiều lần trong vòng lặp nóng. Không cần kháng HashDoS: số tên bị chặn bởi kích thước mã
/// nguồn, không phải dữ liệu người dùng.
struct Fnv(u64);

impl Default for Fnv {
    fn default() -> Self {
        Fnv(0xcbf2_9ce4_8422_2325)
    }
}

impl Hasher for Fnv {
    fn write(&mut self, bytes: &[u8]) {
        let mut h = self.0;
        for &b in bytes {
            h ^= u64::from(b);
            h = h.wrapping_mul(0x0000_0100_0000_01b3);
        }
        self.0 = h;
    }

    fn finish(&self) -> u64 {
        self.0
    }
}

type NameSet = HashSet<Rc<str>, BuildHasherDefault<Fnv>>;

thread_local! {
    /// **VD-267** — bảng tên enum/biến thể **thread-local** (mỗi tên đúng **một** `Rc<str>`).
    ///
    /// Tên enum/biến thể đến từ mã nguồn nên số tên hữu hạn; bảng chỉ giữ một bản mỗi tên.
    /// `Value` vốn đã chứa `Rc` (không `Send`) nên thread-local là đủ, không cần đồng bộ.
    static ENUM_NAMES: RefCell<NameSet> = RefCell::new(HashSet::default());
}

/// **VD-267** — tên enum/biến thể **chia sẻ**.
///
/// Trước đây `Value::Enum(String, String, _)` cấp phát hai `String` cho **mỗi** giá trị enum:
/// vòng lặp dựng `Some(i)`/`E.A(i)` trả hai `malloc` + hai `free` mỗi vòng (chưa kể `Vec` đối
/// số). `Name` giữ `Rc<str>` lấy từ bảng tên thread-local: lần đầu cấp phát, các lần sau chỉ
/// tăng refcount (không `malloc`). So sánh vẫn **theo nội dung** như `String` cũ, nên `==`,
/// pattern và `show` không đổi ngữ nghĩa; tên nội bộ hoá còn cho phép `eq` đi tắt bằng
/// `Rc::ptr_eq`.
#[derive(Clone)]
pub struct Name(Rc<str>);

/// **VD-267** — cache trực tiếp (direct-mapped) cho [`Name::new`]: chỗ dựng tên từ **cùng một
/// chuỗi** (literal, HIR, hằng) chỉ cần so con trỏ + độ dài + nội dung, không tra bảng băm.
/// 64 ô; trượt chỉ tốn thêm một phép kiểm — nội dung **luôn** được xác minh nên địa chỉ bộ đệm
/// bị tái dùng không thể trả sai tên.
const NAME_CACHE_LEN: usize = 64;

#[derive(Clone)]
struct CachedName {
    ptr: usize,
    len: usize,
    name: Name,
}

thread_local! {
    static NAME_CACHE: RefCell<Vec<Option<CachedName>>> =
        RefCell::new(vec![None; NAME_CACHE_LEN]);
}

impl Name {
    /// Tên đã có trong bảng ⇒ chia sẻ bản cũ; chưa có ⇒ cấp phát **một lần**.
    pub fn new(s: &str) -> Self {
        let ptr = s.as_ptr() as usize;
        let len = s.len();
        let idx = (ptr.rotate_left(3) ^ len.wrapping_mul(0x9e37_79b9)) & (NAME_CACHE_LEN - 1);

        // Đường nhanh: cùng con trỏ + độ dài + **nội dung** (bộ đệm có thể bị tái dùng địa chỉ).
        let hit = NAME_CACHE.with(|c| {
            let c = c.borrow();
            c.get(idx).and_then(|slot| {
                slot.as_ref()
                    .filter(|e| e.ptr == ptr && e.len == len && e.name.as_str() == s)
                    .map(|e| e.name.clone())
            })
        });
        if let Some(n) = hit {
            return n;
        }

        let name = ENUM_NAMES.with(|names| {
            let mut names = names.borrow_mut();
            if let Some(n) = names.get(s) {
                return Name(Rc::clone(n));
            }
            let n: Rc<str> = Rc::from(s);
            names.insert(Rc::clone(&n));
            Name(n)
        });
        NAME_CACHE.with(|c| {
            if let Some(slot) = c.borrow_mut().get_mut(idx) {
                *slot = Some(CachedName {
                    ptr,
                    len,
                    name: name.clone(),
                });
            }
        });
        name
    }

    /// Tên dạng `&str` (tương thích chỗ cũ gọi `String::as_str`).
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Deref for Name {
    type Target = str;
    fn deref(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for Name {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Name {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// `Debug` giữ **đúng** dạng cũ của `String` (`"Some"`) — không lộ wrapper `Name`.
impl fmt::Debug for Name {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&*self.0, f)
    }
}

/// Mọi literal `.into()` cũ (`"Option".into()`) tự nội bộ hoá, không cần sửa từng chỗ lạnh.
impl From<&str> for Name {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl From<String> for Name {
    fn from(s: String) -> Self {
        Self::new(&s)
    }
}

impl From<&String> for Name {
    fn from(s: &String) -> Self {
        Self::new(s)
    }
}

impl PartialEq for Name {
    fn eq(&self, other: &Self) -> bool {
        // Tên nội bộ hoá ⇒ cùng nội dung luôn cùng `Rc`; `ptr_eq` là đường nhanh.
        Rc::ptr_eq(&self.0, &other.0) || self.0 == other.0
    }
}

impl Eq for Name {}

impl Hash for Name {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.as_ref().hash(state);
    }
}

impl PartialEq<str> for Name {
    fn eq(&self, other: &str) -> bool {
        &*self.0 == other
    }
}

impl PartialEq<&str> for Name {
    fn eq(&self, other: &&str) -> bool {
        &*self.0 == *other
    }
}

impl PartialEq<String> for Name {
    fn eq(&self, other: &String) -> bool {
        &*self.0 == other.as_str()
    }
}

impl PartialEq<&String> for Name {
    fn eq(&self, other: &&String) -> bool {
        &*self.0 == other.as_str()
    }
}

impl PartialEq<Name> for str {
    fn eq(&self, other: &Name) -> bool {
        self == &*other.0
    }
}

impl PartialEq<Name> for String {
    fn eq(&self, other: &Name) -> bool {
        self.as_str() == &*other.0
    }
}

thread_local! {
    /// Tên `Option`/`Result` dựng sẵn, giữ sẵn để `some`/`none`/`ok`/`err` không phải tra bảng.
    static TAGS: Tags = Tags {
        option: Name::new("Option"),
        some: Name::new("Some"),
        none: Name::new("None"),
        result: Name::new("Result"),
        ok: Name::new("Ok"),
        err: Name::new("Err"),
    };
}

struct Tags {
    option: Name,
    some: Name,
    none: Name,
    result: Name,
    ok: Name,
    err: Name,
}

/// Số bản `'static` của thân closure đã dựng trong **thread** hiện tại (test/đo rò rỉ).
#[doc(hidden)]
pub fn frame_body_builds_for_test() -> usize {
    FRAME_BODY_BUILDS.with(Cell::get)
}

/// Đặt lại bộ đếm [`frame_body_builds_for_test`].
#[doc(hidden)]
pub fn frame_body_builds_reset_for_test() {
    FRAME_BODY_BUILDS.with(|n| n.set(0));
}

/// Môi trường: **một** bảng `sym → giá trị` (ST-11 cấp `sym` duy nhất cho mọi khai báo nên
/// shadowing tự nhiên đúng); closure giữ `Rc` để bắt môi trường.
pub type Env = Rc<RefCell<HashMap<u32, Value>>>;

/// **VD-263 + VD-265** — giá trị binder `else e` của `let … else` khi pattern không khớp.
///
/// Cùng luật với checker/codegen (`HPat::else_binds_err`): tên trần/`Ok(...)` lấy payload `Err`
/// khi giá trị **thật** là `Err`; `Err(e)`/`Some(x)`/`None`/pattern khác bind `()`. Dùng chung
/// cho interp mặc định và frame test (VD-265) để hai đường không lệch giá trị binder.
pub fn let_else_binder(pat: &HPat, v: &Value) -> Value {
    if pat.else_binds_err() {
        if let Value::Enum(ea, va, args) = v {
            if ea == "Result" && va == "Err" && args.len() == 1 {
                return args[0].clone();
            }
        }
    }
    Value::Unit
}

/// Hàm/closure: thân HIR + môi trường bắt được.
#[derive(Debug, Clone)]
pub struct Closure {
    pub name: String,
    pub params: Vec<u32>,
    /// **VD-283/ST-301** — số lớp `Opt` của annot từng tham số (`T?` → 1, `T??` → 2);
    /// `0` = không nâng. Dùng ở biên gọi closure (mirror `lift_params_opt` của VM).
    #[doc(hidden)]
    pub param_depths: Vec<usize>,
    pub body: HBody,
    pub env: Env,
    /// **VD-241 §3 (A0, 2026-11-19)** — số lớp `Opt` của annot return (`T?` → 1, `T??` → 2).
    pub ret_depth: usize,
    /// **VD-259 (chỉ frame test)**: bản `'static` của `body`, dựng tối đa **một lần cho mỗi
    /// closure** thay vì `Box::leak` ở mỗi lần gọi. Runtime mặc định không đọc ô này.
    #[doc(hidden)]
    pub frame_body: OnceCell<&'static HBody>,
}

impl Closure {
    /// **VD-259**: thân dùng cho frame evaluator. `Work` cần tham chiếu sống qua suspend nên
    /// cần `'static`; cache theo closure ⇒ gọi N lần chỉ dựng một bản (không rò rỉ mỗi call).
    #[doc(hidden)]
    pub fn frame_body_static(&self) -> &'static HBody {
        self.frame_body.get_or_init(|| {
            FRAME_BODY_BUILDS.with(|n| n.set(n.get() + 1));
            Box::leak(Box::new(self.body.clone()))
        })
    }
}

/// Giá trị lúc chạy của VS0.
#[derive(Debug, Clone)]
pub enum Value {
    Unit,
    Int(i64),
    Float(f64),
    /// Byte.
    Size(i64),
    /// Nanosecond.
    Duration(i64),
    /// Tỉ lệ 0..1.
    Percent(f64),
    Str(String),
    Bool(bool),
    Bytes(Vec<u8>),
    Char(char),
    Path(String),
    List(Vec<Value>),
    /// Giữ **thứ tự chèn** để `keys()`/`Show` ổn định.
    ///
    /// **ST-239 (VD-242 §3.4, ca 170)**: map là ánh xạ **khoá → giá trị**, mỗi khoá **một**
    /// entry. Runtime (literal có khoá do biến tính, `m[k] = v`, `json.decode`) dùng **last-wins**
    /// (giữ giá trị cuối, vị trí chèn đầu); `eq_value` cũng so theo ánh xạ đã gộp. Riêng
    /// `json.decode` chọn last-wins là **lựa chọn của ViScript, KHÔNG phải luật JSON bắt buộc**
    /// (JSON chỉ khuyến nghị unique key; decoder của ngôn ngữ khác có thể lấy đầu hoặc báo lỗi).
    /// Literal có khoá trùng **xác định được tĩnh** bị checker từ chối (không phải sửa ở runtime).
    Map(Vec<(Value, Value)>),
    /// Tên kiểu (rỗng nếu ẩn danh) + trường theo thứ tự khai.
    Record(String, Vec<(String, Value)>),
    /// Tên enum + tên biến thể + đối số.
    ///
    /// **VD-204 (1)**: `Option`/`Result` **là enum std thật** — `Some(5)` của `Option[int]`
    /// và `Ok("x")` của `Result[int, str]` được **dựng và so khớp như mọi biến thể enum khác**
    /// (tên enum `Option`/`Result` được cất trong giá trị). Không còn biến thể riêng `Opt`/`Res`.
    ///
    /// **VD-267**: hai tên là [`Name`] (`Rc<str>` nội bộ hoá) — dựng giá trị chỉ tăng refcount,
    /// không cấp phát `String` mỗi lần.
    Enum(Name, Name, Vec<Value>),
    /// Nội bộ: `a..b` / `a..=b` (chỉ dùng cho `for`).
    Range(i64, i64, bool),
    /// Nội bộ: hàm dựng sẵn chưa gọi.
    Builtin(&'static str),
    Fn(Rc<Closure>),
    /// **Chỉ VM tạo** (ST-20): hàm bytecode + biến bắt (snapshot, VD-51). `interp` không bao giờ
    /// sinh biến thể này; `Show` in `<hàm>`.
    Vm(Rc<VmFunc>),
    /// **ST-60**: tác vụ `async` — chỉ `await`/join dùng; `Show` in `<tác vụ>` (không lộ handle).
    Task(Rc<crate::value::Task>),
    /// **ST-61 (VD-96)**: kênh — `send`/`recv`/`close`; `Show` in `<kênh>` (không lộ id/nội dung).
    Chan(Rc<std::cell::RefCell<crate::value::Chan>>),
    /// **ST-72b (VD-103/VD-106)**: quyền runtime — `main` nhận từ khai báo `cap[...]`.
    /// `Show` in `<cap[Dir] ./x>` (không lộ chi tiết host); `eq` luôn **false** (tyck đã cấm).
    Cap {
        kind: String,
        root: String,
        write: bool,
    },
}

/// Hàm bytecode (ST-20): chỉ số hàm trong `Module` + biến bắt **snapshot**.
#[derive(Debug, Clone)]
pub struct VmFunc {
    pub idx: u32,
    pub captured: Vec<Value>,
}

impl Value {
    /// `true` nếu là `()` — tiện cho `each`.
    pub fn is_unit(&self) -> bool {
        matches!(self, Value::Unit)
    }

    /// `Some(v)` — enum `Option` thật (VD-204 (1)).
    pub fn some(v: Value) -> Value {
        TAGS.with(|t| Value::Enum(t.option.clone(), t.some.clone(), vec![v]))
    }

    /// `None` — enum `Option` thật (VD-204 (1)).
    pub fn none() -> Value {
        TAGS.with(|t| Value::Enum(t.option.clone(), t.none.clone(), Vec::new()))
    }

    /// Dựng `Option` từ `Option<Value>` của Rust.
    pub fn opt(o: Option<Value>) -> Value {
        match o {
            Some(v) => Value::some(v),
            None => Value::none(),
        }
    }

    /// **VD-222 (điểm 2–3)** — dựng bản ghi `Error` dựng sẵn (`message`, `source = None`, `span = None`).
    pub fn ban_ghi_error(message: impl Into<String>, span: Option<String>) -> Value {
        Value::Record(
            "Error".into(),
            vec![
                ("message".into(), Value::Str(message.into())),
                ("source".into(), Value::none()),
                ("span".into(), Value::opt(span.map(Value::Str))),
            ],
        )
    }

    /// `Ok(v)` — enum `Result` thật (VD-204 (1)).
    pub fn ok(v: Value) -> Value {
        TAGS.with(|t| Value::Enum(t.result.clone(), t.ok.clone(), vec![v]))
    }

    /// `Err(e)` — enum `Result` thật (VD-204 (1)).
    pub fn err(e: Value) -> Value {
        TAGS.with(|t| Value::Enum(t.result.clone(), t.err.clone(), vec![e]))
    }

    /// Bóc `Some(v)` (theo **tên enum**, không nhầm với biến thể `Some` của enum khác).
    pub fn into_some(self) -> Option<Value> {
        match self {
            Value::Enum(ea, va, ag) if ea == "Option" && va == "Some" && ag.len() == 1 => {
                let mut ag = ag;
                ag.pop()
            }
            _ => None,
        }
    }

    /// `Some` payload by **reference** — tiện cho đường bóc `?.`/`x?.y` (không clone).
    pub fn opt_payload(&self) -> Option<&Value> {
        match self {
            Value::Enum(ea, va, ag) if ea == "Option" && va == "Some" && ag.len() == 1 => {
                ag.first()
            }
            _ => None,
        }
    }

    /// `true` nếu là `None` của enum `Option`.
    pub fn is_none(&self) -> bool {
        matches!(self, Value::Enum(ea, va, ag) if ea == "Option" && va == "None" && ag.is_empty())
    }

    /// `true` nếu là `Some(..)` của enum `Option`.
    pub fn is_some(&self) -> bool {
        matches!(self, Value::Enum(ea, va, ag) if ea == "Option" && va == "Some" && ag.len() == 1)
    }

    /// Bóc `Ok(v)` (theo **tên enum**).
    pub fn into_ok(self) -> Option<Value> {
        match self {
            Value::Enum(ea, va, ag) if ea == "Result" && va == "Ok" && ag.len() == 1 => {
                let mut ag = ag;
                ag.pop()
            }
            _ => None,
        }
    }

    /// Bóc `Err(e)` (theo **tên enum**).
    pub fn into_err(self) -> Option<Value> {
        match self {
            Value::Enum(ea, va, ag) if ea == "Result" && va == "Err" && ag.len() == 1 => {
                let mut ag = ag;
                ag.pop()
            }
            _ => None,
        }
    }
}

/// `char` phải **escape** ký tự điều khiển (`'\n'` in ra `'\\n'`, không phải xuống dòng thật).
fn show_char(c: char) -> String {
    let mut s = String::from("'");
    match c {
        '\n' => s.push_str("\\n"),
        '\t' => s.push_str("\\t"),
        '\r' => s.push_str("\\r"),
        '\0' => s.push_str("\\0"),
        '\\' => s.push_str("\\\\"),
        '\'' => s.push_str("\\'"),
        other => s.push(other),
    }
    s.push('\'');
    s
}

/// `float`: dùng `Display` (Rust in `1` cho `1.0`, `1.5` cho `1.5`).
fn show_float(f: f64) -> String {
    if f.is_nan() {
        "NaN".into()
    } else if f.is_infinite() {
        if f > 0.0 {
            "inf".into()
        } else {
            "-inf".into()
        }
    } else {
        format!("{f}")
    }
}

/// **R10 (VD-178/VD-267)** — `size` in theo **thang thập phân**, `b kb mb gb tb`.
///
/// Chọn **đơn vị lớn nhất có phần nguyên ≥ 1** (theo độ lớn), rồi in tối đa **2 chữ số lẻ**.
/// Phần lẻ **làm tròn nửa lên** (half-up trên độ lớn) bằng số nguyên để kết quả tất định, không
/// phụ thuộc biểu diễn `f64`; `999_995` ⇒ `1000kb` (làm tròn trước, không tự nhảy đơn vị).
/// Bỏ số 0 thừa ở cuối; giá trị âm giữ dấu; `0` ⇒ `0b`.
///
/// `kib`/`mib`/… chỉ còn là **thang của literal** (`1kib` = 1024 byte); hiển thị luôn thập phân
/// (`1024` ⇒ `1.02kb`).
const SIZE_UNITS: [(&str, i64); 5] = [
    ("tb", 1_000_000_000_000),
    ("gb", 1_000_000_000),
    ("mb", 1_000_000),
    ("kb", 1000),
    ("b", 1),
];

fn show_size(n: i64) -> String {
    if n == 0 {
        return "0b".into();
    }
    let mut out = String::new();
    if n < 0 {
        out.push('-');
    }
    let am = n.unsigned_abs();
    for (unit, factor) in SIZE_UNITS {
        let factor = factor as u64;
        if am >= factor {
            let mut nguyen = am / factor;
            let du = am % factor;
            // Hai chữ số lẻ, làm tròn nửa lên: `(du * 100 + factor/2) / factor` nằm trong 0..=100.
            // `du < 10^12` nên `du * 100 < 10^14`, cộng nửa đơn vị vẫn không tràn `u64`.
            let mut le = (du * 100 + factor / 2) / factor;
            if le == 100 {
                // 999.995… ⇒ 1000.00: nhớ sang phần nguyên (giữ nguyên đơn vị đã chọn).
                nguyen += 1;
                le = 0;
            }
            out.push_str(&nguyen.to_string());
            if le != 0 {
                let mut s = format!("{le:02}");
                while s.ends_with('0') {
                    s.pop();
                }
                out.push('.');
                out.push_str(&s);
            }
            out.push_str(unit);
            return out;
        }
    }
    unreachable!("bảng SIZE_UNITS phải có đơn vị `b` = 1")
}

/// **R10 (VD-178/VD-267)** — `duration` in **ghép** từ lớn đến nhỏ, bỏ thành phần 0, không cách:
/// `90000000000` ⇒ `1m30s`, `5400000000000` ⇒ `1h30m`, `150000000` ⇒ `150ms`.
///
/// Phân rã **chính xác bằng số nguyên** (không `f64`); giá trị âm giữ dấu; `0` ⇒ `0ns` (giữ quy
/// ước cũ, nhỏ nhất trong bảng đơn vị).
const DUR_UNITS: [(&str, i64); 8] = [
    // ST-13b (mục 9): `w` phải có — `parse_duration` nhận `w` nên `Show` phải in ra `w`.
    ("w", 604_800_000_000_000),
    ("d", 86_400_000_000_000),
    ("h", 3_600_000_000_000),
    ("m", 60_000_000_000),
    ("s", 1_000_000_000),
    ("ms", 1_000_000),
    ("us", 1000),
    ("ns", 1),
];

fn show_duration(n: i64) -> String {
    if n == 0 {
        return "0ns".into();
    }
    let mut out = String::new();
    if n < 0 {
        out.push('-');
    }
    let mut am = n.unsigned_abs();
    for (unit, factor) in DUR_UNITS {
        let factor = factor as u64;
        let phan = am / factor;
        if phan > 0 {
            out.push_str(&phan.to_string());
            out.push_str(unit);
            am %= factor;
        }
    }
    debug_assert_eq!(am, 0, "đơn vị `ns` phải hấp thụ phần dư");
    out
}

/// **ST-240 / VD-243 (#172)**: `2^63` — `i64::MAX as f64` (Rust làm tròn lên). `show_percent`
/// ép `p.round() as i64`; Rust **bão hoà** ở `|p| ≥ 2^63`, nên đây là ranh giới biểu diễn hiện hành.
/// Không đổi thang đo/quy tắc làm tròn — chỉ chặn giá trị vượt miền trước khi đi vào máy.
pub const PCT_GIOI_I64: f64 = 9_223_372_036_854_775_808.0;

/// **ST-240 / VD-243 (#172)**: `percent` hợp lệ = hữu finite và phần trăm (`ratio * 100`) vẫn nằm
/// trong miền `i64` hiện hành. Dùng ở mọi nơi **tạo/chuyển đổi** `Percent` (literal, toán tử) để
/// giá trị sai không bao giờ lọt vào tính toán hay `show`.
pub fn pct_hop_le(ratio: f64) -> bool {
    if !ratio.is_finite() {
        return false;
    }
    let pct = ratio * 100.0;
    (-PCT_GIOI_I64..=PCT_GIOI_I64).contains(&pct)
}

fn show_percent(r: f64) -> String {
    let p = r * 100.0;
    if (p - p.round()).abs() < 1e-9 && (-PCT_GIOI_I64..=PCT_GIOI_I64).contains(&p) {
        format!("{}pct", p.round() as i64)
    } else {
        // Phòng thủ: giá trị lọt qua (bytecode tay, test nội bộ) thì in dạng float thật,
        // **không** bão hoà thành `9223372036854775807pct`.
        format!("{p}pct")
    }
}

fn show_bytes(b: &[u8]) -> String {
    let mut s = String::from("b\"");
    for x in b {
        match x {
            b'\n' => s.push_str("\\n"),
            b'\t' => s.push_str("\\t"),
            b'"' => s.push_str("\\\""),
            b'\\' => s.push_str("\\\\"),
            0x20..=0x7e => s.push(*x as char),
            _ => s.push_str(&format!("\\x{x:02x}")),
        }
    }
    s.push('"');
    s
}

/// Đọc literal `int` (thập phân, `0x`/`0b`/`0o`, cho phép `_`, dấu `-`/`+`) — tách khỏi
/// `interp.rs` cho R1. **VD-266**: dùng chung bộ giải mã với checker/lexer; chỉ `-2^63` được
/// chạm độ lớn `2^63` (`i64::MIN`), dương `2^63` là lỗi.
pub fn parse_int_lit(text: &str) -> Option<i64> {
    vis_syntax::lit::decode_signed_int(text).ok()
}

/// Đọc literal `size` (`5mb`, `1kib`) — trả byte.
pub fn parse_size(text: &str) -> Option<i64> {
    let t = text.trim();
    let digits: String = t
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '_')
        .collect();
    let unit = &t[digits.len()..];
    let n: i64 = digits.replace('_', "").parse().ok()?;
    let factor = match unit {
        "b" => 1,
        "kb" => 1000,
        "mb" => 1_000_000,
        "gb" => 1_000_000_000,
        "tb" => 1_000_000_000_000,
        "kib" => 1024,
        "mib" => 1_048_576,
        "gib" => 1_073_741_824,
        "tib" => 1_099_511_627_776,
        _ => return None,
    };
    n.checked_mul(factor)
}

/// Đọc literal `duration` (`250ms`, `2h30m`) — trả nanosecond.
pub fn parse_duration(text: &str) -> Option<i64> {
    let mut total: i64 = 0;
    let mut num = String::new();
    let mut unit = String::new();
    let mut co_cap = false;
    let mut chars = text.trim().chars().peekable();
    while let Some(c) = chars.next() {
        if c.is_ascii_digit() || c == '_' {
            if !unit.is_empty() {
                total = total.checked_add(scale_duration(&num, &unit)?)?;
                co_cap = true;
                num.clear();
                unit.clear();
            }
            num.push(c);
        } else {
            unit.push(c);
            if let Some(next) = chars.peek() {
                if next.is_ascii_digit() && !num.is_empty() && !unit.is_empty() {
                    // hết một cặp số+đơn vị (đơn vị không thể dài quá 2 ký tự)
                    if unit.len() >= 2 {
                        total = total.checked_add(scale_duration(&num, &unit)?)?;
                        co_cap = true;
                        num.clear();
                        unit.clear();
                    }
                }
            }
        }
    }
    if !num.is_empty() && !unit.is_empty() {
        total = total.checked_add(scale_duration(&num, &unit)?)?;
        co_cap = true;
    }
    // **#196 / ST-312b**: chuỗi không có **cặp** `<số><đơn vị>` nào (`"sai"`, `""`, `"250"`)
    // không phải literal `duration` hợp lệ ⇒ `None` (trước đây trả `Some(0)`).
    if co_cap {
        Some(total)
    } else {
        None
    }
}

fn scale_duration(num: &str, unit: &str) -> Option<i64> {
    let n: i64 = num.replace('_', "").parse().ok()?;
    let factor = match unit {
        "ns" => 1,
        "us" => 1000,
        "ms" => 1_000_000,
        "s" => 1_000_000_000,
        "m" => 60_000_000_000,
        "h" => 3_600_000_000_000,
        "d" => 86_400_000_000_000,
        "w" => 604_800_000_000_000,
        _ => return None,
    };
    n.checked_mul(factor)
}

/// Đọc literal `percent` (`50pct`, `12.5pct`) — trả **tỉ lệ 0..1**.
///
/// **ST-240 / VD-243 (#172)**: literal không biểu diễn được (vô cực, hoặc phần trăm vượt miền
/// `i64` như `1e300pct`) bị từ chối **tại nơi tạo giá trị** — cả interp lẫn VM đều đi qua hàm này
/// (`interp::lit_value`), nên lỗi xuất hiện trước khi giá trị sai đi vào tính toán/`show`.
pub fn parse_percent(text: &str) -> Option<f64> {
    let t = text.trim().trim_end_matches("pct");
    let v: f64 = t.parse().ok()?;
    let ratio = v / 100.0;
    if !pct_hop_le(ratio) {
        return None;
    }
    Some(ratio)
}

/// Bỏ quote + xử lý escape cho `"..."`, `"""..."""`, `r"..."`, `r"""..."""`.
pub fn parse_str_lit(text: &str, raw: bool) -> String {
    let inner = strip_quotes(text);
    if raw {
        return inner.to_string();
    }
    unescape(inner)
}

/// Bỏ quote + `\xNN` cho `b"..."` → từng byte ASCII.
pub fn parse_bytes_lit(text: &str) -> Option<Vec<u8>> {
    let inner = strip_quotes(text);
    let mut out = Vec::new();
    let mut it = inner.chars();
    while let Some(c) = it.next() {
        if c != '\\' {
            if !c.is_ascii() {
                return None;
            }
            out.push(c as u8);
            continue;
        }
        match it.next()? {
            'n' => out.push(b'\n'),
            't' => out.push(b'\t'),
            'r' => out.push(b'\r'),
            '0' => out.push(0),
            '\\' => out.push(b'\\'),
            '"' => out.push(b'"'),
            '\'' => out.push(b'\''),
            'x' => {
                let hi = it.next()?.to_digit(16)? as u8;
                let lo = it.next()?.to_digit(16)? as u8;
                out.push(hi * 16 + lo);
            }
            _ => return None,
        }
    }
    Some(out)
}

/// Bỏ quote + escape cho `'a'` → `char`.
pub fn parse_char_lit(text: &str) -> Option<char> {
    let inner = text.strip_prefix('\'')?.strip_suffix('\'')?;
    unescape(inner).chars().next()
}

fn strip_quotes(text: &str) -> &str {
    // **R6a (VD-178)**: bóc cả **tiền tố** (`p"`, `r"`, `b"`) — trước đây giữ nguyên tiền tố
    // (`p"abc".len()` = 6). Thứ tự: dài trước, ngắn sau.
    for (open, close) in [
        ("r\"\"\"", "\"\"\""),
        ("p\"", "\""),
        ("r\"", "\""),
        ("b\"", "\""),
        ("\"\"\"", "\"\"\""),
        ("\"", "\""),
        ("'''", "'''"),
        ("'", "'"),
    ] {
        if let Some(rest) = text.strip_prefix(open) {
            if let Some(inner) = rest.strip_suffix(close) {
                return inner;
            }
        }
    }
    text
}

fn unescape(s: &str) -> String {
    let mut out = String::new();
    let mut it = s.chars();
    while let Some(c) = it.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match it.next() {
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('r') => out.push('\r'),
            Some('0') => out.push('\0'),
            Some('\\') => out.push('\\'),
            Some('"') => out.push('"'),
            Some('\'') => out.push('\''),
            Some('{') => out.push('{'),
            Some('}') => out.push('}'),
            Some('u') => {
                // `\u{XXXX}`
                let mut hex = String::new();
                for c2 in it.by_ref() {
                    match c2 {
                        '{' => {}
                        '}' => break,
                        h => hex.push(h),
                    }
                }
                if let Some(c2) = u32::from_str_radix(&hex, 16).ok().and_then(char::from_u32) {
                    out.push(c2);
                }
            }
            Some(other) => {
                out.push('\\');
                out.push(other);
            }
            None => out.push('\\'),
        }
    }
    out
}

impl Value {
    /// Chân trị cho `if`/`while`/`and`/`or`/`not`/guard.
    pub fn truthy(&self, span: vis_syntax::Span) -> crate::trap::R<bool> {
        match self {
            Value::Bool(b) => Ok(*b),
            Value::Enum(ea, va, ag) if ea == "Option" => {
                Ok(matches!((va.as_str(), ag.len()), ("Some", 1)))
            }
            other => Err(crate::trap::Abort::Trap(crate::trap::Trap::new(
                &vis_diag::RUNTIME_CODES[0],
                format!("điều kiện cần `bool`, được `{}`", other.show()),
                span,
            ))),
        }
    }
}

/// Mã thoát chương trình: **chỉ** `Result::Err` ⇒ `1`; còn lại `0`.
/// **VD-241 §3/T5 (A0, 2026-11-19)**: `main` trả `Result::Err` ⇒ 1, `Ok` ⇒ 0.
/// **Quyết định 2026-11-19 (đợt công thái học), mục 7**: **không** suy rộng sang `main -> int` —
/// giá trị `int` của `main` **không** phải mã thoát (VD-259).
pub fn exit_code(v: &Value) -> i32 {
    match v {
        Value::Enum(ea, va, _) if ea == "Result" && va == "Err" => 1,
        _ => 0,
    }
}

/// `Show` dùng chung (đơn vị `size`/`duration`/`percent`, map/record…).
pub fn show(v: &Value) -> String {
    v.show()
}

/// Bằng nhau **theo ngữ nghĩa VS0** (không phải `PartialEq` của Rust).
pub fn eq(a: &Value, b: &Value) -> bool {
    a.eq_value(b)
}

/// So sánh thứ tự (chỉ cùng kiểu so được) — `None` nếu không so được.
pub fn cmp(a: &Value, b: &Value) -> Option<std::cmp::Ordering> {
    a.cmp_value(b)
}

#[cfg(test)]
mod tests_show_char {
    use super::show_char;

    #[test]
    fn show_char_escape_ky_tu_dieu_khien() {
        // ST-13b (mục 9): ký tự điều khiển phải **escape**, không in ra ký tự thật.
        assert_eq!(show_char('\n'), "'\\n'");
        assert_eq!(show_char('\t'), "'\\t'");
        assert_eq!(show_char('\''), "'\\''");
        assert_eq!(show_char('a'), "'a'");
    }
}

#[cfg(test)]
mod tests_eq_record {
    use super::Value;

    /// Bản ghi `ten` có `n` trường `f0000…`, giá trị `Int(i)`; `dao` đảo thứ tự trường,
    /// `sua` thay giá trị tại một vị trí (dùng để tạo cặp khác nhau).
    fn ban_ghi(ten: &str, n: usize, dao: bool, sua: Option<(usize, i64)>) -> Value {
        let mut fs = Vec::with_capacity(n);
        for i in 0..n {
            let j = if dao { n - 1 - i } else { i };
            let v = match sua {
                Some((k, x)) if k == j => x,
                _ => j as i64,
            };
            fs.push((format!("f{j:04}"), Value::Int(v)));
        }
        Value::Record(ten.to_string(), fs)
    }

    /// **VD-270**: cùng tên kiểu khác rỗng ⇒ đường nhanh so theo vị trí, đúng ở 100 lẫn
    /// 1000 trường.
    #[test]
    fn cung_ten_kieu_khop_theo_vi_tri() {
        for n in [100, 1000] {
            let a = ban_ghi("Lon", n, false, None);
            let b = ban_ghi("Lon", n, false, None);
            assert!(a.eq_value(&b), "{n} trường cùng thứ tự phải bằng nhau");
        }
    }

    /// Cùng tên kiểu, cùng độ dài, một trường khác ⇒ `false` (không được "khớp nhầm" sang
    /// trường khác).
    #[test]
    fn cung_ten_kieu_khac_gia_tri() {
        for n in [100, 1000] {
            let a = ban_ghi("Lon", n, false, None);
            let b = ban_ghi("Lon", n, false, Some((n / 2, -1)));
            assert!(!a.eq_value(&b), "{n} trường, lệch giữa phải khác");
            let c = ban_ghi("Lon", n, false, Some((n - 1, -1)));
            assert!(!a.eq_value(&c), "{n} trường, lệch cuối phải khác");
        }
    }

    /// Cùng tên kiểu nhưng **khác thứ tự trường** ⇒ vẫn `true` qua fallback đa tập (đường
    /// dựng thật luôn theo thứ tự khai, nhưng giá trị `Value` có thể được dựng trực tiếp).
    #[test]
    fn cung_ten_kieu_khac_thu_tu_van_bang() {
        for n in [100, 1000] {
            let a = ban_ghi("Lon", n, false, None);
            let b = ban_ghi("Lon", n, true, None);
            assert!(a.eq_value(&b), "{n} trường đảo thứ tự vẫn phải bằng");
            assert!(b.eq_value(&a));
        }
    }

    /// Bản ghi **ẩn danh** (tên kiểu rỗng) giữ nguyên ngữ nghĩa đa tập, kể cả khi đảo thứ tự.
    #[test]
    fn ten_rong_khac_thu_tu_van_bang() {
        let a = Value::Record(
            String::new(),
            vec![
                ("b".to_string(), Value::Int(1)),
                ("a".to_string(), Value::Int(2)),
            ],
        );
        let b = Value::Record(
            String::new(),
            vec![
                ("a".to_string(), Value::Int(2)),
                ("b".to_string(), Value::Int(1)),
            ],
        );
        assert!(a.eq_value(&b));
        assert!(b.eq_value(&a));

        let lon_a = ban_ghi("", 1000, false, None);
        let lon_b = ban_ghi("", 1000, true, None);
        assert!(lon_a.eq_value(&lon_b));
    }

    /// Bản ghi ẩn danh **cùng thứ tự** dùng được đường nhanh theo vị trí; lệch một trường
    /// vẫn `false`, và đảo thứ tự vẫn `true` qua fallback (ca trên).
    #[test]
    fn ten_rong_cung_thu_tu_dung_duong_nhanh() {
        for n in [100, 1000] {
            let a = ban_ghi("", n, false, None);
            let b = ban_ghi("", n, false, None);
            assert!(a.eq_value(&b), "{n} trường ẩn danh cùng thứ tự phải bằng");
            assert!(b.eq_value(&a));
            let c = ban_ghi("", n, false, Some((n / 2, -1)));
            assert!(!a.eq_value(&c), "{n} trường ẩn danh lệch phải khác");
        }
    }

    /// Tên kiểu khác nhau (đều khác rỗng) **không** được lọc sớm: hành vi cũ bỏ qua tên kiểu
    /// khi so trường, nên hai bản ghi cùng ruột vẫn bằng — test khoá lại để không đổi ngữ nghĩa.
    #[test]
    fn khac_ten_kieu_cung_ruot_van_bang() {
        let a = ban_ghi("A", 100, false, None);
        let b = ban_ghi("B", 100, false, None);
        assert!(a.eq_value(&b));
    }

    /// **Đường dựng động** `let f = U; f(a: 1, a: 2, b: 3)` (kiểm bằng CLI) cho phép trùng
    /// tên trường, nên đường nhanh **phải fallback khi lệch**, không được trả `false` ngay:
    /// hai bản ghi trùng tên trường đảo giá trị vẫn bằng theo ngữ nghĩa đa tập cũ.
    #[test]
    fn trung_ten_truong_dong_van_theo_da_tap() {
        let a = Value::Record(
            "U".to_string(),
            vec![
                ("a".to_string(), Value::Int(1)),
                ("a".to_string(), Value::Int(2)),
                ("b".to_string(), Value::Int(3)),
            ],
        );
        let b = Value::Record(
            "U".to_string(),
            vec![
                ("a".to_string(), Value::Int(2)),
                ("a".to_string(), Value::Int(1)),
                ("b".to_string(), Value::Int(3)),
            ],
        );
        assert!(a.eq_value(&b));
        assert!(b.eq_value(&a));
    }

    /// Khác độ dài ⇒ `false`; bản ghi rỗng cùng tên kiểu ⇒ `true`.
    #[test]
    fn khac_do_dai_va_rong() {
        let a = ban_ghi("Lon", 100, false, None);
        let b = ban_ghi("Lon", 99, false, None);
        assert!(!a.eq_value(&b));
        assert!(!b.eq_value(&a));

        let r1 = ban_ghi("Lon", 0, false, None);
        let r2 = ban_ghi("Lon", 0, false, None);
        assert!(r1.eq_value(&r2));
    }
}

#[cfg(test)]
mod tests_name {
    use super::{Name, Value};
    use std::rc::Rc;

    /// **VD-267**: cùng nội dung ⇒ **cùng** `Rc<str>` (chỉ tăng refcount), `Debug`/`Display`
    /// vẫn như `String` cũ để đầu ra không đổi.
    #[test]
    fn name_chia_se_theo_noi_dung() {
        let a = Name::new("Option");
        let b = Name::from(String::from("Opt") + "ion");
        let c = Name::new("Option");
        assert!(Rc::ptr_eq(&a.0, &b.0), "phải dùng chung một Rc");
        assert!(
            Rc::ptr_eq(&a.0, &c.0),
            "đường nhanh theo con trỏ cũng chia sẻ"
        );
        assert_eq!(a, b);
        assert_eq!(a.as_str(), "Option");
        assert_eq!(format!("{a}"), "Option");
        assert_eq!(format!("{a:?}"), "\"Option\"");
        assert_eq!(a, "Option");
        assert_ne!(a, "Result");
        // Hai giá trị enum dựng độc lập vẫn bằng nhau + show y hệt trước đây.
        let v1 = Value::Enum(Name::new("E"), Name::new("A"), vec![Value::Int(1)]);
        let v2 = Value::Enum(Name::new("E"), Name::new("A"), vec![Value::Int(1)]);
        assert!(v1.eq_value(&v2));
        assert_eq!(v1.show(), "A(1)");
        assert_eq!(
            format!("{:?}", Value::some(Value::Int(1))),
            "Enum(\"Option\", \"Some\", [Int(1)])"
        );
    }
}
