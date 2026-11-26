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
//!
//! **ST-317b**: tách `mod.rs` (1.415 dòng) — `Name` + bảng tên + singleton sang [`ten`], hiển thị/
//! đọc literal sang [`lit`]. `mod.rs` giữ `enum Value` + constructor/public API + test layout;
//! `size_of::<Value>() == 32` (ST-317a) không đổi.

use std::cell::{Cell, OnceCell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use vis_hir::hir::{HBody, HPat};

mod lit;
mod map;
mod runtime;
mod show_eq;
mod ten;

pub use lit::{
    parse_bytes_lit, parse_char_lit, parse_duration, parse_int_lit, parse_percent, parse_size,
    parse_str_lit, pct_hop_le, PCT_GIOI_I64,
};
pub use map::{MapIndex, VisMap, NGUONG_CHI_MUC};
pub use runtime::{Chan, ChanSend, ChanSendState, Pending, Task, TrangThai, Wait};
pub use ten::Name;

// `show_eq.rs` dùng các helper này qua glob `super::*` (ST-285) — giữ đường cũ.
pub(crate) use lit::{show_bytes, show_char, show_duration, show_float, show_percent, show_size};
pub(crate) use ten::Fnv;
use ten::{EMPTY_MAP, NONE_VALUE, TAGS};

thread_local! {
    /// **VD-259 (chỉ frame test)**: số lần phải dựng bản `'static` của thân closure.
    /// Một closure được gọi N lần chỉ được dựng **một** lần (xem `Closure::frame_body_static`).
    static FRAME_BODY_BUILDS: Cell<usize> = const { Cell::new(0) };
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
        if let Value::Enum(e) = v {
            if e.enum_name == "Result" && e.variant == "Err" && e.args.len() == 1 {
                return e.args.get(0).expect("độ dài 1").clone();
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
    List(Rc<Vec<Value>>),
    /// Giữ **thứ tự chèn** để `keys()`/`Show` ổn định.
    ///
    /// **ST-239 (VD-242 §3.4, ca 170)**: map là ánh xạ **khoá → giá trị**, mỗi khoá **một**
    /// entry. Runtime (literal có khoá do biến tính, `m[k] = v`, `json.decode`) dùng **last-wins**
    /// (giữ giá trị cuối, vị trí chèn đầu); `eq_value` cũng so theo ánh xạ đã gộp. Riêng
    /// `json.decode` chọn last-wins là **lựa chọn của ViScript, KHÔNG phải luật JSON bắt buộc**
    /// (JSON chỉ khuyến nghị unique key; decoder của ngôn ngữ khác có thể lấy đầu hoặc báo lỗi).
    /// Literal có khoá trùng **xác định được tĩnh** bị checker từ chối (không phải sửa ở runtime).
    ///
    /// **ST-315b (VD-291 §3.2)**: `Rc<VisMap>` — đọc `m.get(k)` chỉ tăng refcount (không chép
    /// sâu N cặp), ghi dùng `Rc::make_mut` (CoW). `VisMap` giữ hợp đồng thứ tự/last-wins và có
    /// chỉ mục băm lười cho map đủ lớn (xem [`VisMap`]).
    Map(Rc<VisMap>),
    /// Tên kiểu (rỗng nếu ẩn danh) + trường theo thứ tự khai, chia sẻ qua `Rc` + CoW.
    ///
    /// **ST-317a (VD-292 §4.1)**: đổi từ tuple 48 B sang `Rc<VisRecord>` (8 B) để
    /// `size_of::<Value>() == 32`; `x with f: v` sửa **tại chỗ** qua `Rc::make_mut`.
    Record(Rc<VisRecord>),
    /// Tên enum + tên biến thể + đối số.
    ///
    /// **VD-204 (1)**: `Option`/`Result` **là enum std thật** — `Some(5)` của `Option[int]`
    /// và `Ok("x")` của `Result[int, str]` được **dựng và so khớp như mọi biến thể enum khác**
    /// (tên enum `Option`/`Result` được cất trong giá trị). Không còn biến thể riêng `Opt`/`Res`.
    ///
    /// **VD-267**: hai tên là [`Name`] (`Rc<str>` nội bộ hoá) — dựng giá trị chỉ tăng refcount,
    /// không cấp phát `String` mỗi lần.
    ///
    /// **ST-317a**: bọc `Rc<VisEnum>` (8 B) — `Value` xuống 32 B; `Clone` chỉ tăng refcount.
    /// Đối số `0`/`1` phần tử nằm **trong** cấp phát đó ([`EnumArgs`]) nên không thêm `malloc`.
    Enum(Rc<VisEnum>),
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
    ///
    /// **ST-317a**: bọc `Box<CapVal>` — quyền hiếm khi `clone`, box giữ `Value` 32 B.
    Cap(Box<CapVal>),
}

/// **ST-317a** — payload `Value::Record`: tên kiểu (rỗng nếu ẩn danh) + trường theo thứ tự khai.
#[derive(Debug, Clone)]
pub struct VisRecord {
    pub name: String,
    pub fields: Vec<(String, Value)>,
}

/// **ST-317a** — đối số enum, tối ưu cho `Option`/`Result` (0/1 phần tử) và `SqlValue`.
///
/// `Zero`/`One` nằm **trong** cấp phát `Rc<VisEnum>` — cùng **một** lần cấp phát như
/// `Vec::with_capacity(1)` cũ, không thêm `malloc` cho đường nóng `Some(x)`; `None` dùng
/// singleton [`Value::none`] nên 0 cấp phát.
#[derive(Debug, Clone)]
pub enum EnumArgs {
    Zero,
    One(Value),
    Many(Vec<Value>),
}

impl EnumArgs {
    /// Dựng từ `Vec`: 0/1 phần tử vào dạng nội tuyến, còn lại giữ `Vec`.
    pub fn from_vec(v: Vec<Value>) -> Self {
        let mut v = v;
        match v.len() {
            0 => EnumArgs::Zero,
            1 => EnumArgs::One(v.pop().expect("độ dài 1")),
            _ => EnumArgs::Many(v),
        }
    }

    /// Đối số dưới dạng slice (nguồn duy nhất cho mọi điểm đọc).
    pub fn as_slice(&self) -> &[Value] {
        match self {
            EnumArgs::Zero => &[],
            EnumArgs::One(v) => std::slice::from_ref(v),
            EnumArgs::Many(v) => v,
        }
    }

    /// Số đối số.
    pub fn len(&self) -> usize {
        self.as_slice().len()
    }

    /// Không có đối số.
    pub fn is_empty(&self) -> bool {
        matches!(self, EnumArgs::Zero)
    }

    /// Đối số thứ `i`.
    pub fn get(&self, i: usize) -> Option<&Value> {
        self.as_slice().get(i)
    }

    /// Lặp đối số.
    pub fn iter(&self) -> std::slice::Iter<'_, Value> {
        self.as_slice().iter()
    }

    /// Bóc thành `Vec` **sở hữu** (chỉ dùng khi đã tách khỏi `Rc` chia sẻ).
    pub fn into_vec(self) -> Vec<Value> {
        match self {
            EnumArgs::Zero => Vec::new(),
            EnumArgs::One(v) => vec![v],
            EnumArgs::Many(v) => v,
        }
    }
}

impl std::ops::Deref for EnumArgs {
    type Target = [Value];

    fn deref(&self) -> &[Value] {
        self.as_slice()
    }
}

/// **ST-317a** — payload `Value::Enum` (xem [`Value::Enum`]).
#[derive(Debug, Clone)]
pub struct VisEnum {
    pub enum_name: Name,
    pub variant: Name,
    pub args: EnumArgs,
}

/// **ST-317a** — payload `Value::Cap` (xem [`Value::Cap`]).
#[derive(Debug, Clone)]
pub struct CapVal {
    pub kind: String,
    pub root: String,
    pub write: bool,
}

/// Hàm bytecode (ST-20): chỉ số hàm trong `Module` + biến bắt **snapshot**.
#[derive(Debug, Clone)]
pub struct VmFunc {
    pub idx: u32,
    pub captured: Vec<Value>,
}

impl Value {
    /// **ST-316a (VD-292 §4.1)** — dựng `Value::List` từ `Vec` (bọc `Rc` một lần).
    pub fn list(v: Vec<Value>) -> Value {
        Value::List(Rc::new(v))
    }

    /// Dựng `Value::List` khi đã có sẵn `Rc` (không cấp phát thêm).
    pub fn list_rc(v: Rc<Vec<Value>>) -> Value {
        Value::List(v)
    }

    /// **ST-317a** — dựng `Value::Record` (nguồn duy nhất cho bản ghi).
    pub fn record(name: impl Into<String>, fields: Vec<(String, Value)>) -> Value {
        Value::Record(Rc::new(VisRecord {
            name: name.into(),
            fields,
        }))
    }

    /// **ST-317a** — dựng `Value::Enum` (nguồn duy nhất cho enum thường).
    pub fn enum_val(en: impl Into<Name>, variant: impl Into<Name>, args: Vec<Value>) -> Value {
        Value::Enum(Rc::new(VisEnum {
            enum_name: en.into(),
            variant: variant.into(),
            args: EnumArgs::from_vec(args),
        }))
    }

    /// **ST-317a** — dựng `Value::Enum` khi đã có tên [`Name`] + [`EnumArgs`] (0 cấp phát tên).
    pub fn enum_tu(en: Name, variant: Name, args: EnumArgs) -> Value {
        Value::Enum(Rc::new(VisEnum {
            enum_name: en,
            variant,
            args,
        }))
    }

    /// **ST-317a** — dựng `Value::Cap`.
    pub fn cap(kind: impl Into<String>, root: impl Into<String>, write: bool) -> Value {
        Value::Cap(Box::new(CapVal {
            kind: kind.into(),
            root: root.into(),
            write,
        }))
    }

    /// **ST-317a** — đọc enum dưới dạng `(tên enum, biến thể, đối số)` **không sở hữu**.
    pub fn as_enum(&self) -> Option<(&Name, &Name, &[Value])> {
        match self {
            Value::Enum(e) => Some((&e.enum_name, &e.variant, e.args.as_slice())),
            _ => None,
        }
    }

    /// **ST-317a** — hai tên của enum (không lấy đối số).
    pub fn enum_names(&self) -> Option<(&Name, &Name)> {
        match self {
            Value::Enum(e) => Some((&e.enum_name, &e.variant)),
            _ => None,
        }
    }

    /// **ST-317a** — đối số enum (không sở hữu).
    pub fn enum_args(&self) -> Option<&[Value]> {
        match self {
            Value::Enum(e) => Some(e.args.as_slice()),
            _ => None,
        }
    }

    /// **ST-317a** — bóc enum thành `(tên enum, biến thể, đối số)` **sở hữu**.
    ///
    /// `Rc` dùng chung ⇒ **chép** một bản (CoW): người gọi được sửa `Vec` mà không đụng giá
    /// trị khác. Đây là chỗ duy nhất biến `Rc<VisEnum>` thành sở hữu.
    pub fn into_enum(self) -> Option<(Name, Name, Vec<Value>)> {
        match self {
            Value::Enum(rc) => Some(match Rc::try_unwrap(rc) {
                Ok(e) => (e.enum_name, e.variant, e.args.into_vec()),
                Err(rc) => (
                    rc.enum_name.clone(),
                    rc.variant.clone(),
                    rc.args.as_slice().to_vec(),
                ),
            }),
            _ => None,
        }
    }

    /// **ST-317a** — đọc bản ghi (không sở hữu).
    pub fn as_record(&self) -> Option<&VisRecord> {
        match self {
            Value::Record(r) => Some(r),
            _ => None,
        }
    }

    /// **ST-317a** — bóc bản ghi thành `(tên kiểu, trường)` **sở hữu** (CoW như [`Self::into_enum`]).
    pub fn into_record(self) -> Option<(String, Vec<(String, Value)>)> {
        match self {
            Value::Record(rc) => Some(match Rc::try_unwrap(rc) {
                Ok(r) => (r.name, r.fields),
                Err(rc) => (rc.name.clone(), rc.fields.clone()),
            }),
            _ => None,
        }
    }

    /// `true` nếu là `()` — tiện cho `each`.
    pub fn is_unit(&self) -> bool {
        matches!(self, Value::Unit)
    }

    /// Dựng `Value::Map` từ các cặp — **nguồn duy nhất** cho mọi literal/JSON/snapshot.
    ///
    /// **ST-315b (VD-291 Q1/Q2)**: last-wins **tại chỗ** (giữ khoá gốc + vị trí chèn đầu), gói
    /// trong `Rc<VisMap>` để `Value::clone` không chép sâu toàn map.
    ///
    /// **ST-317a (đóng F6 `ST-315b`)**: map rỗng `{}` dùng **singleton thread-local** — chỉ
    /// tăng refcount, **0 cấp phát**. Mọi đường ghi đều `Rc::make_mut` (strong_count ≥ 2 vì
    /// singleton giữ một) nên không thể sửa nhầm bản dùng chung.
    pub fn map(pairs: Vec<(Value, Value)>) -> Value {
        if pairs.is_empty() {
            return Value::map_rong();
        }
        Value::Map(Rc::new(VisMap::from_pairs(pairs)))
    }

    /// **ST-317a** — map rỗng chia sẻ (singleton theo thread): `{}` không cấp phát.
    pub fn map_rong() -> Value {
        EMPTY_MAP.with(|m| Value::Map(Rc::clone(m)))
    }

    /// Dựng `Value::Map` từ `VisMap` đã có (không sao chép entry).
    pub fn map_tu(m: VisMap) -> Value {
        if m.is_empty() {
            return Value::map_rong();
        }
        Value::Map(Rc::new(m))
    }

    /// `Some(v)` — enum `Option` thật (VD-204 (1)).
    pub fn some(v: Value) -> Value {
        TAGS.with(|t| Value::enum_tu(t.option.clone(), t.some.clone(), EnumArgs::One(v)))
    }

    /// `None` — enum `Option` thật (VD-204 (1)).
    ///
    /// **ST-317a**: singleton thread-local — `None` không cấp phát (trước đây 0 cấp phát `Vec`
    /// nhưng mỗi lần gọi vẫn dựng `Value::Enum` mới; nay chỉ tăng refcount).
    pub fn none() -> Value {
        NONE_VALUE.with(|v| v.clone())
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
        Value::record(
            "Error",
            vec![
                ("message".into(), Value::Str(message.into())),
                ("source".into(), Value::none()),
                ("span".into(), Value::opt(span.map(Value::Str))),
            ],
        )
    }

    /// `Ok(v)` — enum `Result` thật (VD-204 (1)).
    pub fn ok(v: Value) -> Value {
        TAGS.with(|t| Value::enum_tu(t.result.clone(), t.ok.clone(), EnumArgs::One(v)))
    }

    /// `Err(e)` — enum `Result` thật (VD-204 (1)).
    pub fn err(e: Value) -> Value {
        TAGS.with(|t| Value::enum_tu(t.result.clone(), t.err.clone(), EnumArgs::One(e)))
    }

    /// Bóc `Some(v)` (theo **tên enum**, không nhầm với biến thể `Some` của enum khác).
    pub fn into_some(self) -> Option<Value> {
        match self.into_enum() {
            Some((ea, va, mut ag)) if ea == "Option" && va == "Some" && ag.len() == 1 => ag.pop(),
            _ => None,
        }
    }

    /// `Some` payload by **reference** — tiện cho đường bóc `?.`/`x?.y` (không clone).
    pub fn opt_payload(&self) -> Option<&Value> {
        match self {
            Value::Enum(e)
                if e.enum_name == "Option" && e.variant == "Some" && e.args.len() == 1 =>
            {
                e.args.get(0)
            }
            _ => None,
        }
    }

    /// `true` nếu là `None` của enum `Option`.
    pub fn is_none(&self) -> bool {
        matches!(self, Value::Enum(e) if e.enum_name == "Option" && e.variant == "None" && e.args.is_empty())
    }

    /// `true` nếu là `Some(..)` của enum `Option`.
    pub fn is_some(&self) -> bool {
        matches!(self, Value::Enum(e) if e.enum_name == "Option" && e.variant == "Some" && e.args.len() == 1)
    }

    /// Bóc `Ok(v)` (theo **tên enum**).
    pub fn into_ok(self) -> Option<Value> {
        match self.into_enum() {
            Some((ea, va, mut ag)) if ea == "Result" && va == "Ok" && ag.len() == 1 => ag.pop(),
            _ => None,
        }
    }

    /// Bóc `Err(e)` (theo **tên enum**).
    pub fn into_err(self) -> Option<Value> {
        match self.into_enum() {
            Some((ea, va, mut ag)) if ea == "Result" && va == "Err" && ag.len() == 1 => ag.pop(),
            _ => None,
        }
    }
}
impl Value {
    /// Chân trị cho `if`/`while`/`and`/`or`/`not`/guard.
    pub fn truthy(&self, span: vis_syntax::Span) -> crate::trap::R<bool> {
        match self {
            Value::Bool(b) => Ok(*b),
            Value::Enum(e) if e.enum_name == "Option" => {
                Ok(matches!((e.variant.as_str(), e.args.len()), ("Some", 1)))
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
        Value::Enum(e) if e.enum_name == "Result" && e.variant == "Err" => 1,
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
        Value::record(ten, fs)
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
        let a = Value::record(
            String::new(),
            vec![
                ("b".to_string(), Value::Int(1)),
                ("a".to_string(), Value::Int(2)),
            ],
        );
        let b = Value::record(
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
        let a = Value::record(
            "U",
            vec![
                ("a".to_string(), Value::Int(1)),
                ("a".to_string(), Value::Int(2)),
                ("b".to_string(), Value::Int(3)),
            ],
        );
        let b = Value::record(
            "U",
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
mod tests_value_32 {
    use super::Value;
    use std::rc::Rc;

    /// **ST-317a (VD-292 §4.1)** — khoá kích thước `Value` xuống **32 B** trên 64-bit.
    ///
    /// Trước lát này `Value` là 64 B vì `Record` (48 B payload) và `Cap` (56 B) là biến thể
    /// lớn nhất. Nay `Record`/`Enum` bọc `Rc`, `Cap` bọc `Box` ⇒ payload lớn nhất còn 24 B
    /// (`Str`/`Bytes`/`Path`) + 8 B tag = 32 B.
    #[test]
    fn size_of_value_bang_32() {
        if std::mem::size_of::<usize>() == 8 {
            assert_eq!(
                std::mem::size_of::<Value>(),
                32,
                "Value phải đúng 32 B trên 64-bit"
            );
        } else {
            assert!(
                std::mem::size_of::<Value>() <= 32,
                "Value phải ≤ 32 B trên nền tảng khác"
            );
        }
    }

    /// **ST-317a** — `{}` dùng singleton: hai map rỗng chia sẻ **cùng** `Rc<VisMap>` và
    /// sửa một map (qua CoW) không làm map kia đổi.
    #[test]
    fn map_rong_chia_se_va_cow_khong_ro() {
        let (a, b) = (Value::map_rong(), Value::map_rong());
        let Value::Map(ra) = &a else {
            panic!("map_rong phải là Map")
        };
        let Value::Map(rb) = &b else {
            panic!("map_rong phải là Map")
        };
        assert!(Rc::ptr_eq(ra, rb), "hai map rỗng phải dùng chung singleton");

        // Ghi qua đúng đường `Rc::make_mut` của runtime: singleton giữ 1 refcount nên phải
        // nhân bản, không sửa bản dùng chung.
        let mut a = a;
        let Value::Map(ra) = &mut a else {
            panic!("map_rong phải là Map")
        };
        Rc::make_mut(ra).insert(Value::Int(1), Value::Int(2));
        assert_eq!(a.show(), "{1: 2}");
        assert!(
            matches!(&b, Value::Map(m) if m.is_empty()),
            "map rỗng kia phải vẫn rỗng"
        );
        assert_eq!(
            Value::map_rong().show(),
            "{}",
            "singleton vẫn rỗng sau khi bản sao bị sửa"
        );
    }
}
