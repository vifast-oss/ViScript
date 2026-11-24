// SPDX-License-Identifier: MIT OR Apache-2.0
//! **Bảng ngữ nghĩa dùng chung** (ST-20, VD-54 #4): toán tử, 11 tên dựng sẵn, bộ phương thức lõi.
//!
//! `vis-interp` và `vis-vm` **gọi** các hàm ở đây — **không** chép ngữ nghĩa. Hàm nào cần tác dụng
//! phụ thì nhận [`Host`] (ghi `stdout`, gọi closure cho `map/filter/each`).
//!
//! Tập phương thức đã khoá (tối thiểu để chạy VS0): `list`: `len get first last push map filter each
//! sum join`; `str`: `len upper lower split chars contains`; `map`: `len get has keys values`. Ngoài
//! tập này ⇒ **`VS0501` "chưa hỗ trợ ở VS0"** (không panic).

use vis_hir::hir::BinOp;
use vis_syntax::Span;

pub(crate) use vis_core::host::{DbHost, FsHost, KetQua, MucFs, NetHost};

mod method;
pub use method::*;

/// **ST-311d (VD-286)** — 6 tệp phẳng `builtin_*.rs` gom về `builtin/` (giữ đăng ký/API).
pub mod assert;
pub mod list;
pub(crate) mod ls;
pub mod panic;
pub(crate) mod sig;
pub(crate) mod value;
pub(crate) use method::{overflow, pct_moi, pct_nhan_f64};

pub(crate) use self::ls::{chuoi_quyen, thoi_gian_utc};

use crate::codes::{DIV0, RT};
use vis_core::trap::{Abort, Trap};
use vis_core::value::Value;

/// **Chủ nhà** cho các hàm ngữ nghĩa cần tác dụng phụ.
/// **Chương trình ngoài** (ST-27): `vis-interp` **không** biết OS — chỉ biết trait này.
///
/// `vis-cli` cài bằng `std::process::Command`; test/không host ⇒ `None` ⇒ `VS0604`.
pub trait Host {
    /// Bộ đệm `stdout` của chương trình.
    fn out(&mut self) -> &mut String;
    /// Gọi một giá trị hàm (closure/builtin) với đối số đã đánh giá.
    fn call(&mut self, f: &Value, args: Vec<Value>, span: Span) -> vis_core::trap::R<Value>;
    /// Hệ thống tệp (ST-33). Mặc định: `None` ⇒ môi trường không có host ⇒ `VS0604`.
    fn fs(&mut self) -> Option<&mut dyn FsHost> {
        None
    }

    /// **ST-63 (VD-105)**: host mạng (TCP). Mặc định: `None` ⇒ `VS0604`.
    fn net(&mut self) -> Option<&mut dyn NetHost> {
        None
    }

    /// **VD-241 (Gate 1)**: host cơ sở dữ liệu (SQLite bundled ở `vis-cli`). Mặc định: `None` ⇒
    /// `VS0604` (host không có SQLite vẫn báo lỗi rõ, không panic, không hỏng tính năng khác).
    fn db(&mut self) -> Option<&mut dyn DbHost> {
        None
    }

    /// Chương trình **ngoài** (ST-27). Mặc định: `None` ⇒ môi trường không có host ⇒ `VS0604`.
    fn ngoai(
        &mut self,
        _ten: &str,
        _args: &[String],
        _raw: bool,
        _stdin: &str,
        _span: Span,
    ) -> Option<vis_core::trap::R<Value>> {
        None
    }

    /// **ST-60 (VD-95)**: hàng đợi **tác vụ** của môi trường (`$tac_vu`/`$spawn`/`$await` dùng).
    /// Một `RefCell` để hai lời gọi lồng nhau (`await` trong thân tác vụ) **không** mượn chồng.
    fn sched(&self) -> &std::cell::RefCell<crate::rt::Scheduler>;

    /// **ST-60**: chạy **thân** tác vụ theo **handle của host** — interp: `sym` của `HFn`; VM: chỉ
    /// số hàm bytecode. Handle **không** bao giờ lộ ra chương trình.
    fn goi_than(&mut self, fi: u32, args: Vec<Value>, span: Span) -> vis_core::trap::R<Value>;

    /// **ST-60**: tên hàm theo handle (cho thông điệp `VS0506`); mặc định rỗng.
    fn ten_than(&self, _fi: u32) -> String {
        String::new()
    }

    /// **VS3-3b/W5 (P3)** — spawn task qua host (interp: scheduler cũ; VM: pool ở mode Parallel).
    fn spawn_task(
        &mut self,
        than: u32,
        args: Vec<Value>,
        ten: String,
        span: Span,
    ) -> vis_core::trap::R<Value>;

    /// **VS3-3b/W5 (P3)** — await tác vụ qua host (mặc định Off: `rt::await_task`).
    fn await_task(
        &mut self,
        t: &std::rc::Rc<crate::rt::Task>,
        span: Span,
    ) -> vis_core::trap::R<Value>;

    /// **VS3-3b/W5 (P3)** — join cuối qua host (mặc định Off: `rt::join_all`).
    fn join_tasks(&mut self) -> vis_core::trap::R<()>;

    /// Như [`Host::ngoai`] nhưng trả **đủ** `stdout/stderr/code` (ST-34, cho `| complete`).
    fn ngoai_day_du(
        &mut self,
        _ten: &str,
        _args: &[String],
        _stdin: &str,
        _span: Span,
    ) -> Option<Result<KetQua, vis_core::trap::Abort>> {
        None
    }

    // -----------------------------------------------------------------------
    // **ST-292 (VD-275/VD-280/VD-281/VD-282)** — hook evaluator worker cho frame driver.
    // Mặc định no-op ⇒ `Off`/`Local`/non-frame P1..P3 giữ nguyên hành vi. Interp và VM
    // override để pump outcome/cancel/final join mà driver không cần biết `Scheduler.exec`.
    // -----------------------------------------------------------------------

    /// `true` nếu `id` là task evaluator **worker** (đang chờ/gửi pool), không phải frame local.
    fn worker_is_task(&self, _id: u32) -> bool {
        false
    }

    /// `true` nếu còn job/task worker chưa có outcome (pool đang in-flight).
    fn worker_in_flight(&self) -> bool {
        false
    }

    /// Rút outcome đã sẵn sàng; `block` ⇒ chờ tối đa ~50 ms rồi trả. Trả số task vừa đánh thức.
    fn worker_pump(&mut self, _block: bool) -> vis_core::trap::R<usize> {
        Ok(0)
    }

    /// Final join worker: chờ pool, cache mọi task, flush output chưa flush theo spawn order.
    fn worker_final_join(&mut self) -> vis_core::trap::R<()> {
        Ok(())
    }

    /// Huỷ hợp tác task worker (idempotent); không preempt task đang chờ host/chan.
    fn worker_cancel(&mut self, _id: u32) {}
}

/// Lỗi runtime kèm `span` (cùng cấu trúc với `Interp::trap`).
pub(crate) fn trap(code: &'static vis_diag::Code, msg: impl Into<String>, span: Span) -> Abort {
    Abort::Trap(Trap::new(code, msg, span))
}

/// **VD-267** — bảng chữ ký tách sang `builtin/sig.rs` (R1); giữ nguyên đường dẫn công khai
/// `crate::builtin::{SIGS, sig, builtin_name, BuiltinSig}`.
pub use self::sig::{builtin_name, sig, BuiltinSig, SIGS};

pub fn bin(op: BinOp, l: Value, r: Value, span: Span) -> vis_core::trap::R<Value> {
    use BinOp::*;
    match op {
        // **R7 (VD-178)**: `//` — chia lấy phần nguyên (làm tròn **xuống**).
        FloorDiv => arith(op, l, r, span),
        Range | RangeIncl => match (&l, &r) {
            (Value::Int(a), Value::Int(b)) => Ok(Value::Range(*a, *b, matches!(op, RangeIncl))),
            _ => Err(trap(
                RT,
                format!(
                    "`{}` cần hai `int`, được `{}` và `{}`",
                    op.text(),
                    l.show(),
                    r.show()
                ),
                span,
            )),
        },
        Eq => Ok(Value::Bool(l.eq_value(&r))),
        Ne => Ok(Value::Bool(!l.eq_value(&r))),
        Lt | Le | Gt | Ge => {
            let Some(ord) = l.cmp_value(&r) else {
                return Err(trap(
                    RT,
                    format!("không so sánh được `{}` và `{}`", l.show(), r.show()),
                    span,
                ));
            };
            use std::cmp::Ordering::*;
            let b = match op {
                Lt => ord == Less,
                Le => ord != Greater,
                Gt => ord == Greater,
                _ => ord != Less,
            };
            Ok(Value::Bool(b))
        }
        Add | Sub | Mul | Div | Rem | Pow => arith(op, l, r, span),
        And | Or | Coalesce | Pipe => Err(trap(
            RT,
            format!("toán tử `{op:?}` phải xử lý ở tầng biểu thức"),
            span,
        )),
    }
}

fn arith(op: BinOp, l: Value, r: Value, span: Span) -> vis_core::trap::R<Value> {
    use BinOp::*;
    // `int` ↔ `float`: nâng literal int (thiết kế §3.5, ngoại lệ duy nhất).
    let (l, r) = match (l, r) {
        (Value::Int(a), Value::Float(b)) => (Value::Float(a as f64), Value::Float(b)),
        (Value::Float(a), Value::Int(b)) => (Value::Float(a), Value::Float(b as f64)),
        (l, r) => (l, r),
    };
    // `percent` với số: `percent * int -> int`, `percent * size -> size` (§3.6).
    // **ST-240 / VD-243 (#172)**: mọi đường `f64 -> i64` ở đây đều phải **kiểm miền**, không được
    // dùng `as i64` (Rust bão hoà im lặng ở `200pct * i64::MAX`). Tính trung gian bằng `f64` rồi
    // kiểm **kết quả cuối** — không thay máy móc bằng `checked_mul` trên int (VD-243 §2).
    if let (Value::Percent(p), rhs) = (&l, &r) {
        match (op, rhs) {
            (Mul, Value::Int(n)) => return Ok(Value::Int(pct_nhan_f64(*p, *n as f64, span)?)),
            (Mul, Value::Size(n)) => return Ok(Value::Size(pct_nhan_f64(*p, *n as f64, span)?)),
            (Mul, Value::Duration(n)) => {
                return Ok(Value::Duration(pct_nhan_f64(*p, *n as f64, span)?))
            }
            (Add | Sub, Value::Percent(q)) => {
                let r = if matches!(op, Add) { p + q } else { p - q };
                return Ok(Value::Percent(pct_moi(r, span)?));
            }
            (Div, Value::Int(n)) => {
                if *n == 0 {
                    return Err(trap(DIV0, "chia cho 0", span));
                }
                return Ok(Value::Percent(pct_moi(p / (*n as f64), span)?));
            }
            _ => {}
        }
    }
    if let (Value::Int(n), Value::Percent(p)) = (&l, &r) {
        if matches!(op, Mul) {
            return Ok(Value::Int(pct_nhan_f64(*p, *n as f64, span)?));
        }
    }
    // ST-13b (mục 5): đối xứng `size/duration * percent` và `percent / percent` (VD-48: **float**).
    if let (Value::Size(n), Value::Percent(p)) = (&l, &r) {
        if matches!(op, Mul) {
            return Ok(Value::Size(pct_nhan_f64(*p, *n as f64, span)?));
        }
    }
    if let (Value::Duration(n), Value::Percent(p)) = (&l, &r) {
        if matches!(op, Mul) {
            return Ok(Value::Duration(pct_nhan_f64(*p, *n as f64, span)?));
        }
    }
    if let (Value::Percent(a), Value::Percent(b)) = (&l, &r) {
        match op {
            // VD-48(b): `percent / percent` ⇒ **float** (tỉ lệ của hai tỉ lệ).
            Div => {
                if *b == 0.0 {
                    return Err(trap(DIV0, "chia cho 0", span));
                }
                return Ok(Value::Float(a / b));
            }
            Rem => {
                if *b == 0.0 {
                    return Err(trap(DIV0, "lấy dư cho 0", span));
                }
                return Ok(Value::Percent(pct_moi(a % b, span)?));
            }
            _ => {}
        }
    }
    if let (Value::Size(a), Value::Size(b)) = (&l, &r) {
        if matches!(op, Rem) {
            if *b == 0 {
                return Err(trap(DIV0, "lấy dư cho 0", span));
            }
            return Ok(Value::Size(a % b));
        }
    }
    if let (Value::Duration(a), Value::Duration(b)) = (&l, &r) {
        if matches!(op, Rem) {
            if *b == 0 {
                return Err(trap(DIV0, "lấy dư cho 0", span));
            }
            return Ok(Value::Duration(a % b));
        }
    }
    match (op, l, r) {
        // **R6b (VD-178)**: `str + str` **nối** chuỗi.
        (Add, Value::Str(a), Value::Str(b)) => Ok(Value::Str(format!("{a}{b}"))),
        // int
        (Add, Value::Int(a), Value::Int(b)) => {
            Ok(Value::Int(a.checked_add(b).ok_or_else(|| overflow(span))?))
        }
        (Sub, Value::Int(a), Value::Int(b)) => {
            Ok(Value::Int(a.checked_sub(b).ok_or_else(|| overflow(span))?))
        }
        (Mul, Value::Int(a), Value::Int(b)) => {
            Ok(Value::Int(a.checked_mul(b).ok_or_else(|| overflow(span))?))
        }
        // **R7 (VD-178)**: `/` **luôn** `float` (kể cả `int / int`).
        (Div, Value::Int(a), Value::Int(b)) => {
            if b == 0 {
                return Err(trap(DIV0, "chia cho 0 (`/0`)", span));
            }
            Ok(Value::Float(a as f64 / b as f64))
        }
        // **R7 (VD-178)**: `//` chia lấy phần nguyên — làm tròn **xuống** (Python).
        // **R7-fix**: `div_euclid` làm tròn **lên** khi `b < 0` ⇒ trừ 1 khi dư ≠ 0; dùng
        // `checked_*` để `i64::MIN // -1` ra `VS0501` (tràn) thay vì panic Rust.
        (FloorDiv, Value::Int(a), Value::Int(b)) => {
            if b == 0 {
                return Err(trap(DIV0, "chia cho 0 (`//0`)", span));
            }
            let q = a.checked_div_euclid(b).ok_or_else(|| overflow(span))?;
            let r = a.checked_rem_euclid(b).ok_or_else(|| overflow(span))?;
            Ok(Value::Int(if r != 0 && b < 0 { q - 1 } else { q }))
        }
        // **R7 (VD-178)**: `%` khớp `//` — dư **cùng dấu số chia** (Python).
        (Rem, Value::Int(a), Value::Int(b)) => {
            if b == 0 {
                return Err(trap(DIV0, "lấy dư cho 0 (`%0`)", span));
            }
            let r = a.checked_rem_euclid(b).ok_or_else(|| overflow(span))?;
            Ok(Value::Int(if r != 0 && b < 0 { r + b } else { r }))
        }
        (Pow, Value::Int(a), Value::Int(b)) => {
            if b < 0 {
                return Err(trap(RT, "`int ** int` với số mũ âm (VD-43 #2)", span));
            }
            let e = u32::try_from(b).map_err(|_| overflow(span))?;
            Ok(Value::Int(a.checked_pow(e).ok_or_else(|| overflow(span))?))
        }
        // float (IEEE 754 — `/0.0` không lỗi)
        (Add, Value::Float(a), Value::Float(b)) => Ok(Value::Float(a + b)),
        (Sub, Value::Float(a), Value::Float(b)) => Ok(Value::Float(a - b)),
        (Mul, Value::Float(a), Value::Float(b)) => Ok(Value::Float(a * b)),
        (Div, Value::Float(a), Value::Float(b)) => Ok(Value::Float(a / b)),
        (Rem, Value::Float(a), Value::Float(b)) => Ok(Value::Float(a % b)),
        (Pow, Value::Float(a), Value::Float(b)) => Ok(Value::Float(a.powf(b))),
        // size
        (Add, Value::Size(a), Value::Size(b)) => {
            Ok(Value::Size(a.checked_add(b).ok_or_else(|| overflow(span))?))
        }
        (Sub, Value::Size(a), Value::Size(b)) => {
            Ok(Value::Size(a.checked_sub(b).ok_or_else(|| overflow(span))?))
        }
        (Mul, Value::Size(a), Value::Int(b)) => {
            Ok(Value::Size(a.checked_mul(b).ok_or_else(|| overflow(span))?))
        }
        (Mul, Value::Int(a), Value::Size(b)) => {
            Ok(Value::Size(a.checked_mul(b).ok_or_else(|| overflow(span))?))
        }
        (Div, Value::Size(a), Value::Int(b)) => {
            if b == 0 {
                return Err(trap(DIV0, "chia cho 0", span));
            }
            Ok(Value::Size(a / b))
        }
        (Div, Value::Size(a), Value::Size(b)) => {
            if b == 0 {
                return Err(trap(DIV0, "chia cho 0", span));
            }
            Ok(Value::Float(a as f64 / b as f64))
        }
        (Rem, Value::Size(a), Value::Int(b)) => {
            if b == 0 {
                return Err(trap(DIV0, "lấy dư cho 0", span));
            }
            Ok(Value::Size(a % b))
        }
        // duration
        (Add, Value::Duration(a), Value::Duration(b)) => Ok(Value::Duration(
            a.checked_add(b).ok_or_else(|| overflow(span))?,
        )),
        (Sub, Value::Duration(a), Value::Duration(b)) => Ok(Value::Duration(
            a.checked_sub(b).ok_or_else(|| overflow(span))?,
        )),
        (Mul, Value::Duration(a), Value::Int(b)) => Ok(Value::Duration(
            a.checked_mul(b).ok_or_else(|| overflow(span))?,
        )),
        (Mul, Value::Int(a), Value::Duration(b)) => Ok(Value::Duration(
            a.checked_mul(b).ok_or_else(|| overflow(span))?,
        )),
        (Div, Value::Duration(a), Value::Int(b)) => {
            if b == 0 {
                return Err(trap(DIV0, "chia cho 0", span));
            }
            Ok(Value::Duration(a / b))
        }
        (Div, Value::Duration(a), Value::Duration(b)) => {
            if b == 0 {
                return Err(trap(DIV0, "chia cho 0", span));
            }
            Ok(Value::Float(a as f64 / b as f64))
        }
        (Rem, Value::Duration(a), Value::Int(b)) => {
            if b == 0 {
                return Err(trap(DIV0, "lấy dư cho 0", span));
            }
            Ok(Value::Duration(a % b))
        }
        // percent (phòng thủ — các ca trên đã xử lý sớm; giữ cùng kiểm miền VD-243)
        (Add, Value::Percent(a), Value::Percent(b)) => Ok(Value::Percent(pct_moi(a + b, span)?)),
        (Sub, Value::Percent(a), Value::Percent(b)) => Ok(Value::Percent(pct_moi(a - b, span)?)),
        // còn lại: chưa hỗ trợ ở VS0
        (op, l, r) => Err(trap(
            RT,
            format!(
                "`{op:?}` chưa hỗ trợ cho `{}` và `{}` ở VS0",
                l.show(),
                r.show()
            ),
            span,
        )),
    }
}

pub fn un(op: vis_hir::hir::UnOp, v: Value, span: Span) -> vis_core::trap::R<Value> {
    match op {
        vis_hir::hir::UnOp::Neg => match v {
            Value::Int(i) => i
                .checked_neg()
                .map(Value::Int)
                .ok_or_else(|| overflow(span)),
            Value::Float(f) => Ok(Value::Float(-f)),
            other => Err(trap(
                RT,
                format!("`-` không dùng được cho `{}`", other.show()),
                span,
            )),
        },
        vis_hir::hir::UnOp::Not => Ok(Value::Bool(!v.truthy(span)?)),
    }
}

/// Tách `stdout` thành `list[str]` — **bỏ** newline cuối, không sinh dòng rỗng thừa (ST-34).
fn dong_cua(s: &str) -> Value {
    let mut t = s;
    if let Some(x) = t.strip_suffix('\n') {
        t = x;
    }
    let mut dong: Vec<Value> = Vec::new();
    if !t.is_empty() {
        for d in t.split('\n') {
            dong.push(Value::Str(d.to_string()));
        }
    }
    Value::List(dong)
}

/// Record `{stdout: list[str], stderr: list[str], code: int}` cho `| complete` (ST-34).
fn nha_ket_qua(kq: KetQua) -> Value {
    Value::Record(
        String::new(),
        vec![
            ("stdout".into(), dong_cua(&kq.stdout)),
            ("stderr".into(), dong_cua(&kq.stderr)),
            ("code".into(), Value::Int(kq.code as i64)),
        ],
    )
}

/// **ST-53**: lõi **so sánh khoá dùng chung** cho shell `sort_by` và `std.list.sort_by`.
/// Khoá tính **một lần**/phần tử; sắp xếp **ổn định**; `desc` chỉ đảo comparator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum KieuKhoa {
    /// `std.list.sort_by` **ST-56**: `int` **hoặc** `str`, nhưng **một** kiểu trong cả lần gọi
    /// (trộn ⇒ `VS0501` nêu **cả hai** kiểu thật). Cùng comparator `cmp_value` với shell ⇒ thứ tự
    /// chuỗi **nhất quán** shell `sort_by` (byte UTF-8).
    ChiIntHoacStr,
    /// Shell `sort_by`: `int/float/size/duration/percent/str` (bool/không so được ⇒ lỗi).
    Moi,
}

pub(crate) fn sap_xep_theo_khoa(
    host: &mut dyn Host,
    xs: &[Value],
    f: &Value,
    desc: bool,
    kieu: KieuKhoa,
    ten: &str,
    span: Span,
) -> vis_core::trap::R<Value> {
    let mut cap: Vec<(Value, Value)> = Vec::with_capacity(xs.len());
    // ST-56: kiểu khoá **đã gặp** ở `ChiIntHoacStr` — trộn `int`/`str` ⇒ lỗi (không so được).
    let mut loai_da_gap: Option<&'static str> = None;
    for x in xs.iter().cloned() {
        let k = host.call(f, vec![x.clone()], span)?;
        match kieu {
            KieuKhoa::ChiIntHoacStr => {
                let loai = match &k {
                    Value::Int(_) => "int",
                    Value::Str(_) => "str",
                    other => {
                        return Err(trap(
                            RT,
                            format!(
                                "`{ten}`: khoá phải là `int` hoặc `str`, được `{}`",
                                other.show()
                            ),
                            span,
                        ))
                    }
                };
                match loai_da_gap {
                    None => loai_da_gap = Some(loai),
                    Some(truoc) if truoc != loai => {
                        return Err(trap(
                            RT,
                            format!(
                                "`{ten}`: khoá trộn kiểu trong một lần gọi (`{truoc}` và `{loai}`)"
                            ),
                            span,
                        ))
                    }
                    Some(_) => {}
                }
            }
            KieuKhoa::Moi => {
                if matches!(k, Value::Bool(_)) || k.cmp_value(&k).is_none() {
                    return Err(trap(
                        RT,
                        format!("`{ten}`: khoá không so được: `{}`", k.show()),
                        span,
                    ));
                }
            }
        }
        cap.push((k, x));
    }
    cap.sort_by(|a, b| {
        let o = a.0.cmp_value(&b.0).unwrap_or(std::cmp::Ordering::Equal);
        if desc {
            o.reverse()
        } else {
            o
        }
    });
    Ok(Value::List(cap.into_iter().map(|(_, v)| v).collect()))
}

pub fn call(
    host: &mut dyn Host,
    name: &str,
    args: Vec<Value>,
    span: Span,
) -> vis_core::trap::R<Value> {
    // **ST-302 (#8)**: runtime `derives Json` — descriptor do lower sinh, người dùng không thấy.
    if name == "$json:to" || name == "$json:from" {
        if args.len() != 2 {
            return Err(trap(RT, "derives Json: sai số đối số nội bộ", span));
        }
        let Value::Str(desc) = &args[1] else {
            return Err(trap(RT, "derives Json: thiếu descriptor", span));
        };
        if name == "$json:to" {
            return vis_core::json::derive::to_json(&args[0], desc)
                .map_err(|e| trap(RT, format!("to_json: {e}"), span));
        }
        return match vis_core::json::derive::from_json(&args[0], desc) {
            Ok(v) => Ok(Value::ok(v)),
            Err(e) => Ok(Value::err(Value::ban_ghi_error(e, None))),
        };
    }
    // **VD-208 (2)**: `$value:<tên>` — dựng **giá trị dựng sẵn** (dùng được như hàm ở cả hai
    // đường). `:` không hợp lệ trong tên ⇒ người dùng không viết được; `vis_fmt`/`check` không đụng.
    // **VD-218 (2026-11-18)**: tên `$value:…` **không bao giờ** xuất hiện ở thông báo lỗi/stack trace.
    if let Some(ten) = name.strip_prefix("$value:") {
        return self::value::value_builtin(ten)
            .ok_or_else(|| trap(RT, "giá trị dựng sẵn không xác định", span));
    }
    // **ST-293 (VD-274/VD-280)**: select nhiều kênh — logic/wait-set ở `rt/select.rs`.
    if matches!(
        name,
        "select_recv" | "select_recv_timeout" | "select_send" | "select_send_timeout"
    ) {
        return crate::rt::select::call(host, name, args, span);
    }
    // **ST-60 (VD-95)**: ba builtin **nội bộ** của async — tên có `$` ⇒ người dùng **không** viết
    // được. `Int` của chúng là **handle của host** (interp: `sym`; VM: chỉ số hàm bytecode),
    // **không** phải giá trị chương trình. Phòng thủ: bytecode đột biến ⇒ `VS0501`, **không** panic.
    if let Some(kieu) = name.strip_prefix('$') {
        // **ST-61 (VD-96)**: `$kenh(n)` — tạo kênh đệm `n` (`n ≥ 0`; âm ⇒ `VS0501`).
        if kieu == "kenh" {
            return match args.into_iter().next() {
                Some(Value::Int(cap)) => crate::rt::tao_kenh(host, cap, span),
                _ => Err(trap(
                    crate::codes::RT,
                    "`chan[T](n)` cần dung lượng `int`",
                    span,
                )),
            };
        }
        // **VD-241 §3** — nội bộ: nâng giá trị tới đủ `depth` lớp `Opt` (đối số 2).
        if kieu == "lift-opt" {
            return Ok(crate::lift::lift_opt_args(args));
        }
        if kieu == "await" {
            return match args.into_iter().next() {
                Some(Value::Task(t)) => host.await_task(&t, span),
                _ => Err(trap(crate::codes::RT, "`await` cần một `Task[T]`", span)),
            };
        }
        let fi = match args.first() {
            Some(Value::Int(i)) if *i >= 0 && *i <= u32::MAX as i64 => *i as u32,
            _ => {
                return Err(trap(
                    crate::codes::RT,
                    format!("`{name}`: handle tác vụ không hợp lệ"),
                    span,
                ))
            }
        };
        let con: Vec<Value> = args.into_iter().skip(1).collect();
        return match kieu {
            "tac_vu" => {
                let t = host
                    .sched()
                    .borrow_mut()
                    .tao(fi, con, host.ten_than(fi), span);
                Ok(Value::Task(t))
            }
            "spawn" => {
                let ten = host.ten_than(fi);
                host.spawn_task(fi, con, ten, span)
            }
            _ => Err(trap(
                crate::codes::RT,
                format!("builtin nội bộ `{name}` chưa hỗ trợ"),
                span,
            )),
        };
    }
    // ST-27/ST-34: `exec.[complete.][raw.]<ten>` — chương trình ngoài, do **host** chạy.
    // - đối số 0: `argv` (`list[str]`); đối số 1 (ST-34): **stdin** (`str` hoặc `list[str]`);
    // - tiền tố `complete.` ⇒ trả **record** `{stdout, stderr, code}`, exit≠0 **không** là lỗi.
    if let Some(mut ten) = name.strip_prefix("exec.") {
        let mut complete = false;
        if let Some(t) = ten.strip_prefix("complete.") {
            ten = t;
            complete = true;
        }
        let mut raw = false;
        if let Some(t) = ten.strip_prefix("raw.") {
            ten = t;
            raw = true;
        }
        let ds: Vec<String> = match args.first() {
            Some(Value::List(xs)) => xs
                .iter()
                .map(|v| match v {
                    Value::Str(s) => s.clone(),
                    other => other.show(),
                })
                .collect(),
            _ => Vec::new(),
        };
        // **Một** quy tắc cho cả hai đường (ST-34): `list[str]` ⇒ nối `\n` **+ newline cuối**;
        // `str` ⇒ **nguyên văn** (không thêm newline); khác ⇒ `show()`.
        let stdin = match args.get(1) {
            Some(Value::Str(s)) => s.clone(),
            Some(Value::List(xs)) => {
                let mut t = xs
                    .iter()
                    .map(|v| match v {
                        Value::Str(s) => s.clone(),
                        other => other.show(),
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                if !t.is_empty() {
                    t.push('\n');
                }
                t
            }
            Some(other) => other.show(),
            None => String::new(),
        };
        if complete {
            return match host.ngoai_day_du(ten, &ds, &stdin, span) {
                Some(Ok(kq)) => Ok(nha_ket_qua(kq)),
                Some(Err(e)) => Err(e),
                None => Err(trap(
                    &vis_diag::COMMAND_CODES[3],
                    format!(
                        "chương trình ngoài `{ten}` chưa hỗ trợ ở môi trường này (không có host)"
                    ),
                    span,
                )),
            };
        }
        return match host.ngoai(ten, &ds, raw, &stdin, span) {
            Some(kq) => kq,
            None => Err(trap(
                &vis_diag::COMMAND_CODES[3],
                format!("chương trình ngoài `{ten}` chưa hỗ trợ ở môi trường này (không có host)"),
                span,
            )),
        };
    }
    // ST-26: `std.*` — một hiện thực dùng chung cho interp và VM (VD-55).
    if name.starts_with("std.") {
        return crate::stdlib::call(host, name.trim_start_matches("std."), args, span);
    }
    match name {
        "print" => {
            // **R6d (VD-178)**: nhiều đối số cách nhau **1 dấu cách**; `print()` ⇒ chỉ `\n`.
            let dong = args.iter().map(Value::show).collect::<Vec<_>>().join(" ");
            host.out().push_str(&dong);
            host.out().push('\n');
            Ok(Value::Unit)
        }
        "str" => {
            if args.len() != 1 {
                return Err(trap(RT, "`str` cần 1 đối số", span));
            }
            Ok(Value::Str(args[0].show()))
        }
        // **ST-79 (VD-108)**: `assert(điều_kiện)` cho `test fn`; sai ⇒ trap `VS0501` để
        // `vis test` in `not ok` (không thêm mã mới — yêu cầu ST-79).
        "assert" => self::assert::assert_builtin(&args, span),
        // **VD-267**: `panic(msg)` — trap `VS0501` (exit 101), không Rust panic/abort.
        "panic" => self::panic::panic_builtin(&args, span),
        // ST-18 — 3 lệnh builtin chế độ lệnh (`echo`/`lines`/`take`), thuần, không FS.
        "echo" => {
            if args.len() > 4 {
                return Err(trap(RT, "`echo` nhận tối đa 4 đối số", span));
            }
            let upper = match args.get(3) {
                Some(Value::Bool(b)) => *b,
                None => false,
                Some(_) => return Err(trap(RT, "`--upper` cần `bool`", span)),
            };
            let mut phan: Vec<String> = Vec::new();
            for a in args.iter().take(3) {
                match a {
                    Value::Str(s) => {
                        if !s.is_empty() {
                            phan.push(s.clone());
                        }
                    }
                    _ => return Err(trap(RT, "`echo` cần đối số `str`", span)),
                }
            }
            let kq = phan.join(" ");
            Ok(Value::Str(if upper { kq.to_uppercase() } else { kq }))
        }
        "lines" => {
            if args.len() != 1 {
                return Err(trap(RT, "`lines` cần 1 đối số", span));
            }
            let Value::Str(s) = &args[0] else {
                return Err(trap(RT, "`lines` cần `str`", span));
            };
            let mut ds: Vec<Value> = s.split('\n').map(|x| Value::Str(x.to_string())).collect();
            if matches!(ds.last(), Some(Value::Str(x)) if x.is_empty()) {
                ds.pop();
            }
            Ok(Value::List(ds))
        }
        "take" => {
            if args.len() != 2 {
                return Err(trap(RT, "`take` cần 2 đối số", span));
            }
            let (Value::List(xs), Value::Int(n)) = (&args[0], &args[1]) else {
                return Err(trap(RT, "`take` cần `list[str]` và `int`", span));
            };
            let n = (*n).max(0) as usize;
            Ok(Value::List(xs.iter().take(n).cloned().collect()))
        }
        // ST-42: `ls`/`open` qua `FsHost` (thật hoặc `--fs-mock`).
        "ls" => {
            let p = match args.first() {
                Some(Value::Str(s)) => s.clone(),
                None => ".".to_string(),
                Some(other) => {
                    return Err(trap(
                        RT,
                        format!("`ls`: đường dẫn cần `str`, được `{}`", other.show()),
                        span,
                    ))
                }
            };
            let all = matches!(args.get(1), Some(Value::Bool(true)));
            let host = host.fs().ok_or_else(|| {
                trap(
                    RT,
                    "`ls` chưa hỗ trợ ở môi trường này (không có host hệ tệp)",
                    span,
                )
            })?;
            let muc = host
                .list_entries(&p)
                .map_err(|e| trap(RT, format!("`ls {p}`: {e}"), span))?;
            let mut ds: Vec<MucFs> = muc
                .into_iter()
                .filter(|m| all || !m.name.starts_with('.'))
                .collect();
            // **Sắp theo tên (byte)** — ổn định giữa hệ tệp.
            ds.sort_by(|a, b| a.name.as_bytes().cmp(b.name.as_bytes()));
            let long = matches!(args.get(2), Some(Value::Bool(true)));
            Ok(Value::List(
                ds.into_iter()
                    .map(|m| {
                        // **ST-240 / VD-242 §4 (nợ #171)**: host ép `SystemTime` (u128 ns) sang
                        // `i64`; mtime > 2262 tràn thành số âm ⇒ `thoi_gian_utc` in năm 1861.
                        // Thời gian trước epoch là bất biến sai ở đây ⇒ thay bằng `–` (được VD-242
                        // cho phép) và kẹp `modified` về biên, không in số âm vô nghĩa.
                        let ns = m.modified_ns.max(0);
                        let hien_thoi = m.modified_ns >= 0;
                        let mut truong = vec![
                            ("name".to_string(), Value::Str(m.name)),
                            ("size".to_string(), Value::Size(m.size as i64)),
                            ("modified".to_string(), Value::Duration(ns)),
                            ("is_file".to_string(), Value::Bool(m.is_file)),
                        ];
                        let ten = if long {
                            truong.push(("mode".to_string(), Value::Str(chuoi_quyen(m.mode))));
                            truong.push((
                                "modified_time".to_string(),
                                Value::Str(if hien_thoi {
                                    thoi_gian_utc(ns)
                                } else {
                                    "–".to_string()
                                }),
                            ));
                            "FileEntryLong"
                        } else {
                            "FileEntry"
                        };
                        Value::Record(ten.to_string(), truong)
                    })
                    .collect(),
            ))
        }
        "open" => {
            // ST-56 (phòng thủ): bytecode bị **đột biến** (`fuzz_mutate`) có thể gọi với `argc`
            // sai ⇒ `args[0]` panic. Front end **đã** kiểm số đối số ⇒ mã hợp lệ không đổi.
            let Some(Value::Str(p)) = args.first() else {
                return Err(trap(RT, "`open` cần `str`", span));
            };
            let host = host.fs().ok_or_else(|| {
                trap(
                    RT,
                    "`open` chưa hỗ trợ ở môi trường này (không có host hệ tệp)",
                    span,
                )
            })?;
            let s = host
                .read(p)
                .map_err(|e| trap(RT, format!("`open {p}`: {e}"), span))?;
            Ok(Value::Str(s))
        }
        // ST-41: `Render` — **chỉ** cho giá trị cuối pipeline (`boc_render`); `print` giữ nguyên.
        "render" => {
            // ST-56 (phòng thủ): `argc = 0` từ bytecode **đột biến** từng panic ở đây.
            let Some(v) = args.first() else {
                return Err(trap(RT, "`render` cần 1 đối số", span));
            };
            let co_ban_ghi = matches!(args.get(1), Some(Value::Bool(true)));
            match crate::render::in_gia_tri(v, co_ban_ghi) {
                Some(s) => {
                    host.out().push_str(&s);
                    Ok(Value::Unit)
                }
                None => {
                    host.out().push_str(&v.show());
                    host.out().push('\n');
                    Ok(Value::Unit)
                }
            }
        }
        // ST-40b: `get`/`select` — chiếu từng phần tử qua closure (cùng một hiện thực cho hai đường).
        "get" | "select" => {
            // ST-56 (phòng thủ): số đối số — xem ghi chú ở `open`.
            let (Some(Value::List(xs)), Some(f)) = (args.first(), args.get(1)) else {
                return Err(trap(RT, format!("`{name}` cần `list` và hàm"), span));
            };
            let f = f.clone();
            let mut out: Vec<Value> = Vec::with_capacity(xs.len());
            for x in xs.clone() {
                out.push(host.call(&f, vec![x], span)?);
            }
            Ok(Value::List(out))
        }
        // ST-39: `sort_by` — khoá `int/size/duration/percent/str`; **ổn định** cả hai chiều (đảo
        // comparator, `sort_by` của Rust ổn định), `--desc` mặc định `false`.
        "sort_by" => {
            // ST-56 (phòng thủ): số đối số — xem ghi chú ở `open`.
            let (Some(Value::List(xs)), Some(f)) = (args.first(), args.get(1)) else {
                return Err(trap(RT, "`sort_by` cần `list` và hàm", span));
            };
            let desc = match args.get(2) {
                None => false,
                Some(Value::Bool(b)) => *b,
                Some(other) => {
                    return Err(trap(
                        RT,
                        format!("`sort_by`: `--desc` cần `bool`, được `{}`", other.show()),
                        span,
                    ))
                }
            };
            // ST-53: cùng lõi với `std.list.sort_by` (khoá rộng hơn, `--desc`).
            sap_xep_theo_khoa(host, xs, f, desc, KieuKhoa::Moi, "sort_by", span)
        }
        // ST-38: `where`/`each` — closure-arg; **một** hiện thực cho cả hai đường (`Host::call`).
        "where" => {
            if args.len() != 2 {
                return Err(trap(RT, "`where` cần 2 đối số", span));
            }
            let Value::List(xs) = &args[0] else {
                return Err(trap(RT, "`where` cần `list`", span));
            };
            let f = args[1].clone();
            let mut out: Vec<Value> = Vec::new();
            for x in xs.clone() {
                match host.call(&f, vec![x.clone()], span)? {
                    Value::Bool(true) => out.push(x),
                    Value::Bool(false) => {}
                    other => {
                        return Err(trap(
                            RT,
                            format!("`where`: hàm lọc phải trả `bool`, được `{}`", other.show()),
                            span,
                        ))
                    }
                }
            }
            Ok(Value::List(out))
        }
        "each" => {
            if args.len() != 2 {
                return Err(trap(RT, "`each` cần 2 đối số", span));
            }
            let Value::List(xs) = &args[0] else {
                return Err(trap(RT, "`each` cần `list`", span));
            };
            let f = args[1].clone();
            for x in xs.clone() {
                host.call(&f, vec![x], span)?;
            }
            Ok(Value::Unit)
        }
        "first" => {
            if args.len() != 1 {
                return Err(trap(RT, "`first` cần 1 đối số", span));
            }
            let Value::List(xs) = &args[0] else {
                return Err(trap(RT, "`first` cần `list`", span));
            };
            Ok(Value::opt(xs.first().cloned()))
        }
        "count" => {
            if args.len() != 1 {
                return Err(trap(RT, "`count` cần 1 đối số", span));
            }
            let Value::List(xs) = &args[0] else {
                return Err(trap(RT, "`count` cần `list`", span));
            };
            Ok(Value::Int(xs.len() as i64))
        }
        "len" => {
            if args.len() != 1 {
                return Err(trap(RT, "`len` cần 1 đối số", span));
            }
            match &args[0] {
                Value::Str(s) => Ok(Value::Int(s.chars().count() as i64)),
                Value::List(xs) => Ok(Value::Int(xs.len() as i64)),
                Value::Map(kv) => Ok(Value::Int(kv.len() as i64)),
                Value::Bytes(b) => Ok(Value::Int(b.len() as i64)),
                other => Err(trap(
                    RT,
                    format!("`len` chưa hỗ trợ cho `{}` ở VS0", other.show()),
                    span,
                )),
            }
        }
        "Some" => {
            if args.len() != 1 {
                return Err(trap(RT, "`Some` cần 1 đối số", span));
            }
            let mut it = args.into_iter();
            let v = it.next().unwrap_or(Value::Unit);
            Ok(Value::some(v))
        }
        "None" => Ok(Value::none()),
        "Ok" => {
            if args.len() != 1 {
                return Err(trap(RT, "`Ok` cần 1 đối số", span));
            }
            let mut it = args.into_iter();
            let v = it.next().unwrap_or(Value::Unit);
            Ok(Value::ok(v))
        }
        "Err" => {
            if args.len() != 1 {
                return Err(trap(RT, "`Err` cần 1 đối số", span));
            }
            let mut it = args.into_iter();
            let v = it.next().unwrap_or(Value::Unit);
            Ok(Value::err(v))
        }
        // ST-56 (VD-92): `ref(x)` là **biểu thức** hạ identity — cái tên `ref` **không** phải hàm.
        "ref" => Err(trap(
            RT,
            "`ref` chỉ là biểu thức `ref(x)`, không phải hàm",
            span,
        )),
        other => Err(trap(RT, format!("tên `{other}` chưa hỗ trợ ở VS0"), span)),
    }
}
