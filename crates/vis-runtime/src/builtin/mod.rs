// SPDX-License-Identifier: MIT OR Apache-2.0
//! **Bảng ngữ nghĩa dùng chung** (ST-20, VD-54 #4): toán tử, 11 tên dựng sẵn, bộ phương thức lõi.
//!
//! `vis-interp` và `vis-vm` **gọi** các hàm ở đây — **không** chép ngữ nghĩa. Hàm nào cần tác dụng
//! phụ thì nhận [`Host`] (ghi `stdout`, gọi closure cho `map/filter/each`).
//!
//! Tập phương thức đã khoá (tối thiểu để chạy VS0): `list`: `len get first last push map filter each
//! sum join`; `str`: `len upper lower split chars contains`; `map`: `len get has keys values`. Ngoài
//! tập này ⇒ **`VS0501` "chưa hỗ trợ ở VS0"** (không panic).

use vis_syntax::Span;

use crate::codes::RT;

pub(crate) use vis_core::host::DbHost;

mod goi;
mod host;
mod method;
mod toan_tu;

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

pub use self::goi::call;
pub use self::host::Host;
pub use self::toan_tu::{bin, un};

use vis_core::trap::{Abort, Trap};
use vis_core::value::Value;

/// Lỗi runtime kèm `span` (cùng cấu trúc với `Interp::trap`).
pub(crate) fn trap(code: &'static vis_diag::Code, msg: impl Into<String>, span: Span) -> Abort {
    Abort::Trap(Trap::new(code, msg, span))
}

/// **VD-267** — bảng chữ ký tách sang `builtin/sig.rs` (R1); giữ nguyên đường dẫn công khai
/// `crate::builtin::{SIGS, sig, builtin_name, BuiltinSig}`.
pub use self::sig::{builtin_name, sig, BuiltinSig, SIGS};

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
    Ok(Value::list(cap.into_iter().map(|(_, v)| v).collect()))
}
