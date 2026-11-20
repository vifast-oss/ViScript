// SPDX-License-Identifier: MIT OR Apache-2.0
//! **`std.*` lõi** (ST-26): ngữ nghĩa dùng **chung** cho interp và VM (VD-55) — một hiện thực duy nhất.
//!
//! Tên đầy đủ có tiền tố module (`str.split`, `list.sort`, `time.now_ms`) — `tyck` kiểm chữ ký theo
//! [`vis_types::stdlib`], tầng này kiểm **kiểu lúc chạy** (sai ⇒ `VS0501` nêu hàm + kiểu nhận được).
//!
//! Mọi hàm **thuần** trừ `std.time.*` (đọc đồng hồ trong Rust, **không** thêm method vào [`Host`]);
//! `std.fs` **không** ở đây (ST-27), `std.json.decode` **chưa** có (VS1b — `VS0301` ở tầng hạ).

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::OnceLock;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use vis_diag::{COMMAND_CODES, RUNTIME_CODES};
use vis_syntax::Span;
use vis_types::stdlib::{sig_of, Ty};

use crate::builtin::Host;
use crate::interp::{Abort, Trap, R};
use crate::rt::ReactorOp;
use crate::value::{eq, Value};

/// `Err(String)` của host ⇒ `Result` của VS0 (`Err(msg)`), **không** phải trap.
/// **ST-72b (VD-103/VD-106)**: dấu **cưỡng chế quyền** trong `Err` của host — `hop_nhat_fs` đổi
/// thành **trap `VS0604`** (không bắt được), thay vì `Err(str)` dữ liệu (vượt quyền là lỗi **môi
/// trường**, không phải dữ liệu chương trình).
pub const DAU_QUYEN: &str = "\u{1}VS0604:";

/// **ST-238 / VD-242 (nợ #167)**: trần `max_bytes` cho `net.read`. Input người dùng (kể cả
/// `i64::MAX`, âm, hay giá trị chuyển kiểu overflow) **không** được phép ép host cấp phát bộ nhớ
/// tùy ý (OOM ⇒ abort). Vượt trần ⇒ `Result.Err` dữ liệu, không trap, không panic.
pub const NET_READ_CAP: i64 = 8 * 1024 * 1024;

// ===========================================================================
// ST-277 / VD-256 — connect/write nonblocking trên ĐƯỜNG FRAME TEST (chỉ test).
//
// `rt.rs`/trait `FrameReactorBackend` KHÔNG đổi (zero-dep, cross-platform). Adapter
// thật nằm trong integration test Linux; core chỉ cần (a) ghi dữ liệu `net.write` vào
// một kênh chung và (b) đi qua `frame_reactor_poll_io(ReactorOp::Read, ...)` với handle
// được mã hoá `N:<addr>` / `W:<tay>`. Adapter đọc kênh này, tự làm nonblocking
// connect (kiểm tra SO_ERROR khi readiness) và partial write (lưu offset). Mọi cơ chế
// deadline/token/generation/cleanup đã có trong `frame_real_poll` được TÁI DÙNG nguyên
// văn — không helper mới trong `rt.rs`. Runtime mặc định (`frame.enabled == false`)
// và API ngôn ngữ (`net.connect`/`net.write`, chữ ký, giá trị trả) không đổi.
// ===========================================================================

/// Tiền tố handle cho một lần `net.connect` trên đường frame test (`N:<dia_chi>`).
#[doc(hidden)]
pub const FRAME_NET_CONNECT: &str = "N:";
/// Tiền tố handle cho một lần `net.write` trên đường frame test (`W:<tay>`).
#[doc(hidden)]
pub const FRAME_NET_WRITE: &str = "W:";
/// Hạn mặc định của `net.write` trên đường frame test (ngôn ngữ không có đối số timeout).
/// Chọn rộng cho loopback; vẫn là hạn monotonic DUY NHẤT, không reset theo readiness.
#[doc(hidden)]
pub const FRAME_WRITE_TIMEOUT_MS: i64 = 5_000;

#[derive(Default)]
struct FrameNetIo {
    /// `W:<tay>` → bytes cần gửi (adapter `take` sang trạng thái riêng ở lần poll đầu).
    writes: HashMap<String, Vec<u8>>,
    /// Test có thể thu hẹp hạn write để kiểm chứng timeout mà không đổi API ngôn ngữ.
    write_timeout_ms: Option<i64>,
}
thread_local! {
    static FRAME_NET_IO: RefCell<FrameNetIo> = RefCell::new(FrameNetIo::default());
}

/// **Chỉ test**: ghi bytes cần gửi cho khoá `W:<tay>` trước khi gọi `frame_reactor_poll_io`.
#[doc(hidden)]
pub fn frame_net_set_write(key: &str, data: &[u8]) {
    FRAME_NET_IO.with(|x| {
        x.borrow_mut().writes.insert(key.to_string(), data.to_vec());
    });
}

/// **Chỉ test**: lấy (và xoá) bytes cần gửi của khoá `W:<tay>`.
#[doc(hidden)]
pub fn frame_net_take_write(key: &str) -> Option<Vec<u8>> {
    FRAME_NET_IO.with(|x| x.borrow_mut().writes.remove(key))
}

/// **Chỉ test**: đặt lại hạn write (ms); `None` ⇒ dùng [`FRAME_WRITE_TIMEOUT_MS`].
#[doc(hidden)]
pub fn frame_net_set_write_timeout(ms: Option<i64>) {
    FRAME_NET_IO.with(|x| x.borrow_mut().write_timeout_ms = ms);
}

/// **Chỉ test**: hạn write hiệu lực, luôn dương (tránh trần chờ selector vô hạn).
#[doc(hidden)]
pub fn frame_net_write_timeout() -> i64 {
    FRAME_NET_IO.with(|x| {
        x.borrow()
            .write_timeout_ms
            .unwrap_or(FRAME_WRITE_TIMEOUT_MS)
            .max(1)
    })
}

/// **Chỉ test**: `net.write` trả `Result<(), str>` — đổi payload `Ok(str)` của reactor
/// thành `Ok(())`, giữ nguyên `Err` (hợp đồng số byte/lỗi KHÔNG đổi).
fn frame_unit_result(v: Value) -> Value {
    match &v {
        Value::Enum(ea, va, ag) if ea == "Result" && va == "Ok" && ag.len() == 1 => {
            Value::ok(Value::Unit)
        }
        _ => v,
    }
}

/// **ST-238 / VD-242 (nợ #167)**: validate `max_bytes` từ người dùng. `i64::MAX` (overflow khi
/// chuyển `usize`), âm, và giá trị vượt trần đều bị từ chối **trước** khi host cấp phát.
fn gioi_han_net_read(toi_da: i64) -> Result<usize, String> {
    if toi_da < 0 {
        return Err("net.read: `max_bytes` không được âm".to_string());
    }
    if toi_da > NET_READ_CAP {
        return Err(format!(
            "net.read: `max_bytes` {toi_da} vượt giới hạn {NET_READ_CAP} byte"
        ));
    }
    Ok(toi_da as usize)
}

// **VD-241 pre-merge §1 (2026-11-19)**: `DAU_LOI_GOI` (VS0501) **đã bị bỏ** — mọi lỗi host
// (handle giả/đã đóng/sai loại, SQL quá dài, params thiếu/thừa/quá, vượt rows/bytes/connections,
// timeout, authorizer/policy từ chối, lỗi SQLite…) đều là **lỗi dữ liệu/runtime** ⇒ `Result.Err`
// (`Err(Error)`), KHÔNG trap `VS0501`. Chỉ còn **hai** lớp ngoài `Result`: (a) lỗi tĩnh/checker-time
// (sai kiểu/sai arity — tầng `kiem`, không đổi phạm vi), (b) bất biến nội bộ bị phá (bug nội bộ —
// cơ chế chẩn đoán nội bộ hiện hành). Lỗi data **không** mang dấu prefix — cùng mẫu `DAU_QUYEN`
// (lọc bằng `strip_prefix`), khác ở chỗ **không** cần dấu: mọi `Err` host không có `DAU_QUYEN`
// ⇒ `Err(Error)` dữ liệu.

fn hop_nhat_fs(r: Result<Value, String>, span: Span) -> Result<Value, Abort> {
    match r {
        Ok(v) => Ok(Value::ok(v)),
        Err(e) => match e.strip_prefix(DAU_QUYEN) {
            Some(msg) => Err(Abort::Trap(Trap::new(
                &COMMAND_CODES[3],
                msg.to_string(),
                span,
            ))),
            // Mọi lỗi host khác ⇒ `Err(Error)` dữ liệu (VD-241 pre-merge §1).
            None => Ok(Value::err(Value::ban_ghi_error(e, None))),
        },
    }
}

/// Không có host ⇒ `VS0604` (cùng câu với chương trình ngoài, VD-60 (2)).
fn khong_host(name: &str, span: Span) -> Abort {
    Abort::Trap(Trap::new(
        &COMMAND_CODES[3],
        format!("`std.{name}` chưa hỗ trợ ở môi trường này (không có host)"),
        span,
    ))
}

fn loi(msg: impl Into<String>, span: Span) -> Abort {
    Abort::Trap(Trap::new(&RUNTIME_CODES[0], msg, span))
}

/// Kiểm số đối số + kiểu theo bảng chữ ký; trả `Err(Abort)` nếu sai.
fn kiem(name: &str, args: &[Value], span: Span) -> Result<(), Abort> {
    let Some(sig) = sig_of(name) else {
        return Err(loi(
            format!("hàm `std.{name}` không có trong bảng chữ ký"),
            span,
        ));
    };
    if args.len() != sig.so_doi() {
        return Err(loi(
            format!("`{name}` cần {} đối số, được {}", sig.so_doi(), args.len()),
            span,
        ));
    }
    for (i, (a, t)) in args.iter().zip(sig.params.iter()).enumerate() {
        let ok = match t {
            // **ST-58 (VD-94/VD-99)**: biến kiểu / `list[T]` / `map[K,V]` — thân hàm tự kiểm như
            // trước với `Any` (tyck đã chặn **tĩnh** phần lớn; dữ liệu không kiểu vẫn tới đây).
            Ty::Var(_) | Ty::Map(..) => true,
            // **VD-241**: `list[SqlValue]` (params của `std.sqlite`) — kiểm **phần tử** là `SqlValue`.
            Ty::List(inner) if **inner == Ty::SqlValue => {
                matches!(a, Value::List(xs) if xs.iter().all(|v| matches!(v, Value::Enum(e, _, _) if e == "SqlValue")))
            }
            Ty::List(_) => true,
            Ty::Int => matches!(a, Value::Int(_)),
            Ty::Str => matches!(a, Value::Str(_)),
            Ty::Bool => matches!(a, Value::Bool(_)),
            Ty::ListStr => {
                matches!(a, Value::List(xs) if xs.iter().all(|v| matches!(v, Value::Str(_))))
            }
            Ty::FnKhoa => matches!(a, Value::Fn(_) | Value::Vm(_) | Value::Builtin(_)),
            // **ST-33: `std.fs` trả `Result[..]` — kiểu **trả**, không dùng để kiểm đối số.
            // **VD-222**: `Error`/`Json`/`Result[Json, Error]` — kiểu **trả** của `std.*` (lỗi/JSON),
            // không dùng để kiểm đối số người dùng; runtime giữ nguyên **tín hiệu** như trước.
            Ty::ResultStrStr
            | Ty::ResultUnitStr
            | Ty::ResultListStrStr
            | Ty::ResultHead
            | Ty::ResultJsonErr
            | Ty::Error
            | Ty::Json
            // **VD-241 (Gate 1)**: `Result[RowSet, Error]` cũng là kiểu **trả** của `std.sqlite.query`.
            | Ty::ResultIntErr
            | Ty::ResultBoolErr
            | Ty::ResultRowSetErr => true,
            // **VD-241**: `SqlValue` tham số (chưa dùng ở vị trí đối số)/`RowSet` — kiểm runtime nếu đi qua.
            Ty::SqlValue => matches!(a, Value::Enum(e, _, _) if e == "SqlValue"),
            Ty::RowSet => matches!(a, Value::Record(n, _) if n == "RowSet"),
        };
        if !ok {
            return Err(loi(
                format!(
                    "`{name}`: đối số {} cần `{}`, được `{}`",
                    i + 1,
                    t.ten(),
                    a.show()
                ),
                span,
            ));
        }
    }
    Ok(())
}

fn chuoi<'a>(name: &str, v: &'a Value, i: usize, span: Span) -> Result<&'a str, Abort> {
    match v {
        Value::Str(s) => Ok(s),
        other => Err(loi(
            format!(
                "`{name}`: đối số {} cần `str`, được `{}`",
                i + 1,
                other.show()
            ),
            span,
        )),
    }
}

fn so(name: &str, v: &Value, i: usize, span: Span) -> Result<i64, Abort> {
    match v {
        Value::Int(n) => Ok(*n),
        other => Err(loi(
            format!(
                "`{name}`: đối số {} cần `int`, được `{}`",
                i + 1,
                other.show()
            ),
            span,
        )),
    }
}

fn danh_sach<'a>(name: &str, v: &'a Value, i: usize, span: Span) -> Result<&'a [Value], Abort> {
    match v {
        Value::List(xs) => Ok(xs),
        other => Err(loi(
            format!(
                "`{name}`: đối số {} cần `list`, được `{}`",
                i + 1,
                other.show()
            ),
            span,
        )),
    }
}

fn ban_do<'a>(
    name: &str,
    v: &'a Value,
    i: usize,
    span: Span,
) -> Result<&'a [(Value, Value)], Abort> {
    match v {
        Value::Map(kv) => Ok(kv),
        other => Err(loi(
            format!(
                "`{name}`: đối số {} cần `map`, được `{}`",
                i + 1,
                other.show()
            ),
            span,
        )),
    }
}

/// Gọi một hàm `std.*` theo tên đầy đủ.
pub fn call(host: &mut dyn Host, name: &str, args: Vec<Value>, span: Span) -> R<Value> {
    kiem(name, &args, span)?;
    match name {
        // ----- std.str -----
        "str.split" => {
            let s = chuoi(name, &args[0], 0, span)?;
            let sep = chuoi(name, &args[1], 1, span)?;
            let ds: Vec<Value> = if sep.is_empty() {
                // Tách theo **ký tự** (Unicode scalar) khi dấu phân cách rỗng.
                s.chars().map(|c| Value::Str(c.to_string())).collect()
            } else {
                s.split(sep).map(|x| Value::Str(x.to_string())).collect()
            };
            Ok(Value::List(ds))
        }
        "str.join" => {
            let xs = danh_sach(name, &args[0], 0, span)?;
            let sep = chuoi(name, &args[1], 1, span)?;
            let mut phan: Vec<String> = Vec::with_capacity(xs.len());
            for (i, v) in xs.iter().enumerate() {
                match v {
                    Value::Str(s) => phan.push(s.clone()),
                    other => {
                        return Err(loi(
                            format!("`{name}`: phần tử {} cần `str`, được `{}`", i, other.show()),
                            span,
                        ))
                    }
                }
            }
            Ok(Value::Str(phan.join(sep)))
        }
        "str.trim" | "str.upper" | "str.lower" => {
            let s = chuoi(name, &args[0], 0, span)?;
            Ok(Value::Str(match name {
                "str.trim" => s.trim().to_string(),
                "str.upper" => s.to_uppercase(),
                _ => s.to_lowercase(),
            }))
        }
        "str.starts_with" | "str.ends_with" | "str.contains" => {
            let s = chuoi(name, &args[0], 0, span)?;
            let p = chuoi(name, &args[1], 1, span)?;
            let ok = match name {
                "str.starts_with" => s.starts_with(p),
                "str.ends_with" => s.ends_with(p),
                _ => s.contains(p),
            };
            Ok(Value::Bool(ok))
        }
        "str.repeat" => {
            let s = chuoi(name, &args[0], 0, span)?;
            let n = so(name, &args[1], 1, span)?;
            if n < 0 {
                return Err(loi(
                    format!("`{name}`: số lần lặp phải ≥ 0, được {n}"),
                    span,
                ));
            }
            // Cap an toàn: 10^7 ký tự (như cap range) — tránh cấp phát khổng lồ.
            let tong = s.len().saturating_mul(n as usize);
            if tong > 10_000_000 {
                return Err(loi(
                    format!("`{name}`: kết quả quá lớn ({tong} byte > 10^7)"),
                    span,
                ));
            }
            Ok(Value::Str(s.repeat(n as usize)))
        }
        "str.replace" => {
            let s = chuoi(name, &args[0], 0, span)?;
            let tu = chuoi(name, &args[1], 1, span)?;
            let den = chuoi(name, &args[2], 2, span)?;
            Ok(Value::Str(s.replace(tu, den)))
        }
        // ----- std.list -----
        "list.sort" => {
            let xs = danh_sach(name, &args[0], 0, span)?.to_vec();
            let mut ds = xs.clone();
            // Phần tử đơn cũng phải kiểm: `sort_by` không gọi comparator cho danh sách 1 phần tử.
            if xs.iter().any(|x| x.cmp_value(x).is_none()) {
                return Err(loi(
                    "`list.sort`: không so được các phần tử (NaN hoặc kiểu chưa có thứ tự)",
                    span,
                ));
            }
            // **ST-240 / VD-242 §3.5 (nợ #168)**: không được `unwrap_or(Equal)` — đó là im lặng
            // giữ nguyên thứ tự khi gặp `NaN` (hoặc kiểu chưa có thứ tự). Báo lỗi theo cơ chế
            // hiện hành (`VS0501`, cùng `sort_by`). Kết quả sắp một phần bị **huỷ hoàn toàn**
            // (trả `Err`, không trả danh sách), nên không có "sắp nửa rồi mới báo lỗi".
            let mut khong_so_duoc = false;
            ds.sort_by(|a, b| match a.cmp_value(b) {
                Some(o) => o,
                None => {
                    khong_so_duoc = true;
                    std::cmp::Ordering::Equal
                }
            });
            if khong_so_duoc {
                return Err(loi(
                    "`list.sort`: không so được các phần tử (NaN hoặc kiểu chưa có thứ tự)",
                    span,
                ));
            }
            Ok(Value::List(ds))
        }
        // ST-53: **cùng lõi** `sap_xep_theo_khoa` với shell `sort_by`; ST-56: khoá nhận **`int`
        // hoặc `str`** (một kiểu trong cả lần gọi) — ca `int` cũ **không đổi**.
        "list.sort_by" => {
            let xs = danh_sach(name, &args[0], 0, span)?.to_vec();
            crate::builtin::sap_xep_theo_khoa(
                host,
                &xs,
                &args[1],
                false,
                crate::builtin::KieuKhoa::ChiIntHoacStr,
                name,
                span,
            )
        }
        "list.reverse" => {
            let mut xs = danh_sach(name, &args[0], 0, span)?.to_vec();
            xs.reverse();
            Ok(Value::List(xs))
        }
        "list.unique" => {
            let xs = danh_sach(name, &args[0], 0, span)?;
            let mut out: Vec<Value> = Vec::new();
            for x in xs {
                if !out.iter().any(|y| eq(y, x)) {
                    out.push(x.clone());
                }
            }
            Ok(Value::List(out))
        }
        "list.concat" => {
            let a = danh_sach(name, &args[0], 0, span)?.to_vec();
            let b = danh_sach(name, &args[1], 1, span)?;
            let mut out = a;
            out.extend_from_slice(b);
            Ok(Value::List(out))
        }
        "list.chunk" => {
            let xs = danh_sach(name, &args[0], 0, span)?;
            let n = so(name, &args[1], 1, span)?;
            if n <= 0 {
                return Err(loi(
                    format!("`{name}`: kích thước khối phải ≥ 1, được {n}"),
                    span,
                ));
            }
            let n = n as usize;
            Ok(Value::List(
                xs.chunks(n).map(|c| Value::List(c.to_vec())).collect(),
            ))
        }
        "list.take" | "list.drop" => {
            let xs = danh_sach(name, &args[0], 0, span)?;
            let n = so(name, &args[1], 1, span)?;
            let n = n.max(0) as usize;
            let out: Vec<Value> = if name == "list.take" {
                xs.iter().take(n).cloned().collect()
            } else {
                xs.iter().skip(n).cloned().collect()
            };
            Ok(Value::List(out))
        }
        // ----- std.map -----
        "map.keys" | "map.values" => {
            let kv = ban_do(name, &args[0], 0, span)?;
            let out: Vec<Value> = if name == "map.keys" {
                kv.iter().map(|(k, _)| k.clone()).collect()
            } else {
                kv.iter().map(|(_, v)| v.clone()).collect()
            };
            Ok(Value::List(out))
        }
        "map.merge" => {
            let a = ban_do(name, &args[0], 0, span)?.to_vec();
            let b = ban_do(name, &args[1], 1, span)?;
            let mut out = a;
            for (k, v) in b {
                match out.iter_mut().find(|(k0, _)| eq(k0, k)) {
                    Some(slot) => slot.1 = v.clone(),
                    None => out.push((k.clone(), v.clone())),
                }
            }
            Ok(Value::Map(out))
        }
        "map.get_or" => {
            let kv = ban_do(name, &args[0], 0, span)?;
            let k = chuoi(name, &args[1], 1, span)?;
            let mac_dinh = args[2].clone();
            let kvk = Value::Str(k.to_string());
            Ok(kv
                .iter()
                .find(|(k0, _)| eq(k0, &kvk))
                .map(|(_, v)| v.clone())
                .unwrap_or(mac_dinh))
        }
        "map.remove" => {
            let kv = ban_do(name, &args[0], 0, span)?;
            let k = chuoi(name, &args[1], 1, span)?;
            let kvk = Value::Str(k.to_string());
            Ok(Value::Map(
                kv.iter().filter(|(k0, _)| !eq(k0, &kvk)).cloned().collect(),
            ))
        }
        // ----- std.path (thuần chuỗi, không chạm đĩa) -----
        "path.join" => {
            let a = chuoi(name, &args[0], 0, span)?;
            let b = chuoi(name, &args[1], 1, span)?;
            Ok(Value::Str(noi_duong(a, b)))
        }
        "path.basename" | "path.dirname" | "path.ext" | "path.stem" => {
            let p = chuoi(name, &args[0], 0, span)?;
            Ok(Value::Str(match name {
                "path.basename" => basename(p),
                "path.dirname" => dirname(p),
                "path.ext" => ext(p),
                _ => stem(p),
            }))
        }
        "path.is_abs" => {
            let p = chuoi(name, &args[0], 0, span)?;
            Ok(Value::Bool(p.starts_with('/')))
        }
        // ----- std.time (đọc đồng hồ trong Rust; **không** qua Host) -----
        "time.now_ms" => {
            let ms = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_millis() as i64)
                .unwrap_or(0);
            Ok(Value::Int(ms))
        }
        "time.monotonic_ms" => {
            static GOC: OnceLock<Instant> = OnceLock::new();
            let goc = GOC.get_or_init(Instant::now);
            Ok(Value::Int(goc.elapsed().as_millis() as i64))
        }
        // ----- std.json (encode + decode — ST-33) -----
        "json.decode" => {
            let s = chuoi(name, &args[0], 0, span)?;
            // **ST-240 / VD-242 §3.5 (nợ #175)**: `01` phải bị từ chối. Lỗi là **dữ liệu**
            // (`Result.Err` của chương trình), không phải trap — cùng mẫu với lỗi cú pháp JSON
            // của `crate::json::decode`.
            if let Some(off) = json_zero_dau(s) {
                let e = format!("JSON không hợp lệ ở byte {off}: số có số 0 đầu");
                return Ok(Value::err(Value::ban_ghi_error(e, None)));
            }
            match crate::json::decode(s) {
                // **VD-222 (điểm 4)**: trả `Result[Json, Error]` — bọc cây JSON vào enum `Json`.
                Ok(v) => Ok(Value::ok(crate::json::boc(v))),
                Err(e) => Ok(Value::err(Value::ban_ghi_error(e, None))),
            }
        }
        "json.encode" | "json.encode_pretty" => {
            // **ST-238 / VD-242 (nợ #169, Q4b, §3.5)**: NaN/vô cực (kể cả `Percent`) ⇒ **lỗi**,
            // không xuất token `inf`/`NaN` và không tự đổi `null`. Kiểm đệ thuỳ trước khi mã hoá.
            if co_so_phi_huu_han(&args[0]) {
                return Err(loi(
                    "json.encode: giá trị số không hữu hạn (NaN/inf) không được mã hoá",
                    span,
                ));
            }
            let mut s = String::new();
            json(&args[0], name.ends_with("_pretty"), &mut s);
            Ok(Value::Str(s))
        }
        // ----- std.fs (ST-33) — **chỉ qua `Host`**, không OS trong crate này -----
        "fs.read" => {
            let p = chuoi(name, &args[0], 0, span)?;
            let host = host.fs().ok_or_else(|| khong_host(name, span))?;
            hop_nhat_fs(host.read(p).map(Value::Str), span)
        }
        "fs.write" => {
            let p = chuoi(name, &args[0], 0, span)?;
            let v = chuoi(name, &args[1], 1, span)?;
            let host = host.fs().ok_or_else(|| khong_host(name, span))?;
            hop_nhat_fs(host.write(p, v).map(|()| Value::Unit), span)
        }
        "fs.exists" => {
            let p = chuoi(name, &args[0], 0, span)?;
            let host = host.fs().ok_or_else(|| khong_host(name, span))?;
            // **ST-83**: `exists` trả **bool** (không phải Result) nhưng vẫn **trap `VS0604`** khi
            // ngoài quyền — không còn `false` im lặng.
            match host.exists_quyen(p) {
                Ok(b) => Ok(Value::Bool(b)),
                Err(e) => match e.strip_prefix(DAU_QUYEN) {
                    Some(msg) => Err(Abort::Trap(Trap::new(
                        &COMMAND_CODES[3],
                        msg.to_string(),
                        span,
                    ))),
                    None => Ok(Value::Bool(false)),
                },
            }
        }
        "fs.list" => {
            let p = chuoi(name, &args[0], 0, span)?;
            let host = host.fs().ok_or_else(|| khong_host(name, span))?;
            hop_nhat_fs(
                host.list(p)
                    .map(|xs| Value::List(xs.into_iter().map(Value::Str).collect())),
                span,
            )
        }
        // ----- std.net (ST-63, VD-100/VD-105) — **chỉ qua `Host::net`**, không OS trong crate này.
        // Thiếu host ⇒ `VS0604`; lỗi mạng ⇒ `Err(str)` (dữ liệu, **không** trap). -----
        "net.connect" | "net.connect_timeout" => {
            let dia_chi = chuoi(name, &args[0], 0, span)?;
            let timeout = if args.len() == 2 {
                so(name, &args[1], 1, span)?
            } else {
                // Mặc định 1 s — đủ rộng cho loopback, không treo chương trình.
                1_000
            };
            // ST-277/VD-256: đường frame có adapter thật ⇒ connect nonblocking. Handle
            // `N:<addr>` chỉ là khoá core; adapter khởi tạo connect, đăng ký WRITABLE,
            // và CHỈ trả tay khi readiness kèm kiểm tra kết quả nối thực tế (SO_ERROR).
            // Deadline/token/gen/cleanup là của `frame_reactor_poll_io` (không đổi rt.rs).
            if host.sched().borrow().frame.enabled && host.sched().borrow().frame.backend.is_some()
            {
                let khoa = format!("{FRAME_NET_CONNECT}{dia_chi}");
                return host.sched().borrow_mut().frame_reactor_poll_io(
                    ReactorOp::Read,
                    &khoa,
                    0,
                    timeout,
                    span,
                );
            }
            let host = host.net().ok_or_else(|| khong_host(name, span))?;
            let r = host.connect(dia_chi, timeout.max(0) as u64);
            hop_nhat_fs(r.map(Value::Str), span)
        }
        "net.read" => {
            let tay = chuoi(name, &args[0], 0, span)?;
            let toi_da = so(name, &args[1], 1, span)?;
            let timeout = so(name, &args[2], 2, span)?;
            // ST-268/VD-252 (Lát 2): đường frame test dùng reactor giả — KHÔNG đọc socket
            // thật, KHÔNG busy-poll. Runtime mặc định (frame.enabled == false) đi nguyên văn.
            if host.sched().borrow().frame.enabled {
                let n = match gioi_han_net_read(toi_da) {
                    Ok(n) => n,
                    Err(e) => return Ok(Value::err(Value::ban_ghi_error(e, None))),
                };
                let _ = n; // trần `NET_READ_CAP` đã kiểm; reactor giả không cấp phát byte.
                return host.sched().borrow_mut().frame_reactor_poll_io(
                    ReactorOp::Read,
                    tay,
                    n,
                    timeout,
                    span,
                );
            }
            let host = host.net().ok_or_else(|| khong_host(name, span))?;
            // **ST-238 / VD-242 (nợ #167)**: `so()` đã chặn sai kiểu; ở đây chặn âm/overflow/cap
            // trước khi host cấp phát. Mọi trường hợp invalid ⇒ `Result.Err`, không abort.
            let n = match gioi_han_net_read(toi_da) {
                Ok(n) => n,
                Err(e) => return Ok(Value::err(Value::ban_ghi_error(e, None))),
            };
            let r = host.read(tay, n, timeout.max(0) as u64);
            // Body là **byte**; ngôn ngữ chỉ có `str` ⇒ giải mã **lossy** (không cắt giữa ký tự).
            // `Ok(vec![])` = EOF ⇒ `""` (không phải lỗi). Nợ: API byte thật cho tới khi có `buf`.
            hop_nhat_fs(
                r.map(|b| Value::Str(String::from_utf8_lossy(&b).into_owned())),
                span,
            )
        }
        "net.write" => {
            let tay = chuoi(name, &args[0], 0, span)?;
            let du = chuoi(name, &args[1], 1, span)?;
            // ST-277/VD-256: đường frame có adapter thật ⇒ partial-write có offset.
            // Resume gọi lại builtin với ĐỐI SỐ ĐÃ CACHE (xem `Redispatch`/`frame.rs`),
            // nên bytes được ghi lại đúng từ registry/offset của adapter — KHÔNG gửi lại
            // phần đã gửi. Hạn write là một deadline monotonic duy nhất, không reset.
            if host.sched().borrow().frame.enabled && host.sched().borrow().frame.backend.is_some()
            {
                let khoa = format!("{FRAME_NET_WRITE}{tay}");
                frame_net_set_write(&khoa, du.as_bytes());
                let han = frame_net_write_timeout();
                let r = host.sched().borrow_mut().frame_reactor_poll_io(
                    ReactorOp::Read,
                    &khoa,
                    du.len(),
                    han,
                    span,
                );
                frame_net_take_write(&khoa); // dọn phần adapter chưa kịp take
                return r.map(frame_unit_result);
            }
            let host = host.net().ok_or_else(|| khong_host(name, span))?;
            hop_nhat_fs(host.write(tay, du.as_bytes()).map(|()| Value::Unit), span)
        }
        "net.close" => {
            let tay = chuoi(name, &args[0], 0, span)?;
            // Lát 3: trên đường frame có adapter thật ⇒ đóng socket + vô hiệu mọi
            // đăng ký/stale event của tay này, không cần host mạng mặc định.
            let backend = host.sched().borrow().frame.backend.clone();
            if let Some(b) = backend {
                host.sched().borrow_mut().frame_reactor_cancel_handle(tay);
                // ST-277: một `net.write` đang treo đăng ký dưới khoá `W:<tay>` — phải
                // huỷ cả khoá đó thì lần retry mới thấy tay đã đóng (Err đúng một lần),
                // không đợi tới deadline rồi trả timeout muộn.
                let wk = format!("{FRAME_NET_WRITE}{tay}");
                host.sched().borrow_mut().frame_reactor_cancel_handle(&wk);
                return hop_nhat_fs(b.borrow_mut().close(tay).map(|()| Value::Unit), span);
            }
            let host = host.net().ok_or_else(|| khong_host(name, span))?;
            hop_nhat_fs(host.close(tay).map(|()| Value::Unit), span)
        }
        // **ST-65 (VD-100/VD-105)**: server TCP — `listen`/`local_addr`/`accept(_timeout)`/
        // `close_listen`; host giữ listener thật; thiếu host ⇒ `VS0604`; `accept` không timeout = chặn.
        "net.listen" => {
            let dia_chi = chuoi(name, &args[0], 0, span)?;
            let host = host.net().ok_or_else(|| khong_host(name, span))?;
            hop_nhat_fs(host.listen(dia_chi).map(Value::Str), span)
        }
        "net.local_addr" => {
            let tay = chuoi(name, &args[0], 0, span)?;
            let host = host.net().ok_or_else(|| khong_host(name, span))?;
            hop_nhat_fs(host.local_addr(tay).map(Value::Str), span)
        }
        "net.accept" | "net.accept_timeout" => {
            let tay = chuoi(name, &args[0], 0, span)?;
            let timeout = if args.len() == 2 {
                so(name, &args[1], 1, span)?
            } else {
                0
            };
            // ST-268/VD-252 (Lát 2): accept trên đường frame test = reactor giả.
            if host.sched().borrow().frame.enabled {
                return host.sched().borrow_mut().frame_reactor_poll_io(
                    ReactorOp::Accept,
                    tay,
                    0,
                    timeout,
                    span,
                );
            }
            let host = host.net().ok_or_else(|| khong_host(name, span))?;
            hop_nhat_fs(
                host.accept(tay, timeout.max(0) as u64).map(Value::Str),
                span,
            )
        }
        "net.close_listen" => {
            let tay = chuoi(name, &args[0], 0, span)?;
            let host = host.net().ok_or_else(|| khong_host(name, span))?;
            hop_nhat_fs(host.close(tay).map(|()| Value::Unit), span)
        }
        // **ST-64 (VD-100/VD-105)**: giao thức HTTP **trong crate này** (`net.rs`), socket qua host.
        // Lỗi HTTP/mạng là `Err(str)` (dữ liệu); thiếu host ⇒ `VS0604`. Mặc định 5000 ms.
        "http.get" | "http.get_timeout" => {
            let url = chuoi(name, &args[0], 0, span)?;
            let timeout = if args.len() == 2 {
                so(name, &args[1], 1, span)?
            } else {
                5_000
            };
            let host = host.net().ok_or_else(|| khong_host(name, span))?;
            hop_nhat_fs(
                crate::net::http_get(host, url, timeout.max(0) as u64).map(Value::Str),
                span,
            )
        }
        // **ST-109**: `http.head` — chỉ header, không đọc thân; không theo redirect.
        "http.head" | "http.head_timeout" => {
            let url = chuoi(name, &args[0], 0, span)?;
            let timeout = if args.len() == 2 {
                so(name, &args[1], 1, span)?
            } else {
                5_000
            };
            let host = host.net().ok_or_else(|| khong_host(name, span))?;
            let kq = crate::net::http_head(host, url, timeout.max(0) as u64).map(|(st, hs)| {
                let headers = Value::Map(
                    hs.into_iter()
                        .map(|(k, v)| (Value::Str(k), Value::Str(v)))
                        .collect(),
                );
                Value::Map(vec![
                    (Value::Str("status".to_string()), Value::Int(st as i64)),
                    (Value::Str("headers".to_string()), headers),
                ])
            });
            hop_nhat_fs(kq, span)
        }
        // ----- std.sqlite (VD-241, Gate 1) — **chỉ qua `Host::db`**, không backend trong crate này.
        // Thiếu host ⇒ `VS0604`; mọi lỗi host (handle/SQL/params/giới hạn/authorizer/timeout/lỗi
        // SQLite) ⇒ `Err(Error)` dữ liệu (VD-241 pre-merge §1). -----
        "sqlite.open" => {
            let p = chuoi(name, &args[0], 0, span)?;
            let host = host.db().ok_or_else(|| khong_host(name, span))?;
            // Mặc định `create = true` (A0 I); busy mặc định 5 s (hữu hạn ≤ 60 s host kẹp).
            hop_nhat_fs(host.db_open(p, true, 5_000).map(Value::Str), span)
        }
        "sqlite.open_with" => {
            let p = chuoi(name, &args[0], 0, span)?;
            let Value::Bool(create) = &args[1] else {
                return Err(loi(
                    format!("`{name}`: đối số 2 cần `bool`, được `{}`", args[1].show()),
                    span,
                ));
            };
            let busy = so(name, &args[2], 2, span)?;
            let create = *create;
            let host = host.db().ok_or_else(|| khong_host(name, span))?;
            hop_nhat_fs(host.db_open(p, create, busy).map(Value::Str), span)
        }
        "sqlite.close" => {
            let h = chuoi(name, &args[0], 0, span)?;
            let host = host.db().ok_or_else(|| khong_host(name, span))?;
            hop_nhat_fs(host.db_close(h).map(|()| Value::Unit), span)
        }
        "sqlite.execute" => {
            let h = chuoi(name, &args[0], 0, span)?;
            let sql = chuoi(name, &args[1], 1, span)?;
            let params = danh_sach(name, &args[2], 2, span)?;
            let host = host.db().ok_or_else(|| khong_host(name, span))?;
            hop_nhat_fs(host.db_execute(h, sql, params).map(Value::Int), span)
        }
        "sqlite.query" => {
            let h = chuoi(name, &args[0], 0, span)?;
            let sql = chuoi(name, &args[1], 1, span)?;
            let params = danh_sach(name, &args[2], 2, span)?;
            let host = host.db().ok_or_else(|| khong_host(name, span))?;
            match host.db_query(h, sql, params) {
                Ok((columns, rows)) => {
                    let columns = Value::List(columns.into_iter().map(Value::Str).collect());
                    let rows = Value::List(rows.into_iter().map(Value::List).collect());
                    Ok(Value::ok(Value::Record(
                        "RowSet".into(),
                        vec![("columns".into(), columns), ("rows".into(), rows)],
                    )))
                }
                Err(e) => hop_nhat_fs(Err(e), span),
            }
        }
        "sqlite.begin" => {
            let h = chuoi(name, &args[0], 0, span)?;
            let host = host.db().ok_or_else(|| khong_host(name, span))?;
            hop_nhat_fs(host.db_execute(h, "BEGIN", &[]).map(|_| Value::Unit), span)
        }
        "sqlite.commit" => {
            let h = chuoi(name, &args[0], 0, span)?;
            let host = host.db().ok_or_else(|| khong_host(name, span))?;
            hop_nhat_fs(host.db_execute(h, "COMMIT", &[]).map(|_| Value::Unit), span)
        }
        "sqlite.rollback" => {
            let h = chuoi(name, &args[0], 0, span)?;
            let host = host.db().ok_or_else(|| khong_host(name, span))?;
            hop_nhat_fs(
                host.db_execute(h, "ROLLBACK", &[]).map(|_| Value::Unit),
                span,
            )
        }
        "sqlite.in_transaction" => {
            let h = chuoi(name, &args[0], 0, span)?;
            let host = host.db().ok_or_else(|| khong_host(name, span))?;
            hop_nhat_fs(host.db_in_transaction(h).map(Value::Bool), span)
        }
        other => Err(loi(
            format!("`std.{other}` chưa cài (bảng chữ ký có nhưng tầng chạy thiếu)"),
            span,
        )),
    }
}

/// **ST-238 / VD-242 (nợ #169)**: `true` nếu cây giá trị chứa số `Float`/`Percent` không hữu hạn.
/// Đi cấu trúc `Value` thuần, không gọi mã người dùng.
fn co_so_phi_huu_han(v: &Value) -> bool {
    match v {
        Value::Float(f) | Value::Percent(f) => !f.is_finite(),
        Value::List(xs) => xs.iter().any(co_so_phi_huu_han),
        Value::Map(kv) => kv
            .iter()
            .any(|(k, val)| co_so_phi_huu_han(k) || co_so_phi_huu_han(val)),
        Value::Record(_, fs) => fs.iter().any(|(_, x)| co_so_phi_huu_han(x)),
        Value::Enum(_, _, args) => args.iter().any(co_so_phi_huu_han),
        _ => false,
    }
}

/// **ST-240 / VD-242 §3.5 (nợ #175)**: JSON cấm số có số `0` đầu (`01`, `-01`, `00`). Tầng
/// `crate::json::decode` dùng `str::parse::<i64>()` nên vẫn nhận các token đó; chặn ở **cửa
/// `std.json.decode`** (không đụng tầng parse dùng chung với LSP/CLI). Quét theo token số, bỏ qua
/// nội dung chuỗi (kể cả escape) để không đụng `"01"` hợp lệ. Trả byte offset của token sai.
fn json_zero_dau(s: &str) -> Option<usize> {
    let b = s.as_bytes();
    let mut i = 0usize;
    let mut trong_chuoi = false;
    let mut escape = false;
    while i < b.len() {
        if trong_chuoi {
            if escape {
                escape = false;
            } else if b[i] == b'\\' {
                escape = true;
            } else if b[i] == b'"' {
                trong_chuoi = false;
            }
            i += 1;
            continue;
        }
        match b[i] {
            b'"' => {
                trong_chuoi = true;
                i += 1;
            }
            c if c == b'-' || c.is_ascii_digit() => {
                let bat_dau = i;
                if c == b'-' {
                    i += 1;
                }
                // `0` theo sau bởi chữ số ⇒ phần nguyên có số 0 đầu (JSON không hợp lệ).
                if b.get(i) == Some(&b'0') && i + 1 < b.len() && b[i + 1].is_ascii_digit() {
                    return Some(bat_dau);
                }
                // Tiêu thụ **toàn bộ** token số — `100` (hai số 0 cuối) phải hợp lệ.
                while i < b.len() && matches!(b[i], b'0'..=b'9' | b'.' | b'e' | b'E' | b'+' | b'-')
                {
                    i += 1;
                }
            }
            _ => i += 1,
        }
    }
    None
}

/// **ST-240 / VD-242 §3.5 (nợ #173)**: giữ dấu float trong JSON. `format!("{}", 1.0_f64)` cho
/// `"1"`, decoder sẽ đọc thành `Int`; thêm `.0` khi chuỗi in ra không có dấu thập phân/mũ.
fn json_float(f: f64) -> String {
    let s = format!("{f}");
    if s.contains('.') || s.contains('e') || s.contains('E') {
        s
    } else {
        format!("{s}.0")
    }
}

/// Mã hoá JSON **compact** (hoặc pretty 2 dấu cách) cho giá trị VS0.
///
/// Quy ước: `map` theo **thứ tự chèn**; bản ghi theo thứ tự trường; `None` ⇒ `null`; `Result` ⇒
/// `{"ok":…}`/`{"err":…}`; enum ⇒ `{"variant":…, "value":…}`. Khoá chuỗi được escape.
///
/// **Lưu ý**: `Float` không hữu hạn vẫn in `null` ở hàm thấp-level này (dùng cho LSP/CLI); cửa
/// `std.json.encode` chặn riêng ở trên theo VD-242 Q4b.
pub fn json(v: &Value, pretty: bool, out: &mut String) {
    json_tai(v, pretty, 0, out);
}

fn thut(depth: usize, out: &mut String) {
    for _ in 0..depth {
        out.push_str("  ");
    }
}

fn mo(pretty: bool, depth: usize, out: &mut String) {
    if pretty {
        out.push('\n');
        thut(depth + 1, out);
    }
}

fn dong(pretty: bool, depth: usize, out: &mut String) {
    if pretty {
        out.push('\n');
        thut(depth, out);
    }
}

fn json_tai(v: &Value, pretty: bool, depth: usize, out: &mut String) {
    fn esc(s: &str, out: &mut String) {
        out.push('"');
        for c in s.chars() {
            match c {
                '"' => out.push_str("\\\""),
                '\\' => out.push_str("\\\\"),
                '\n' => out.push_str("\\n"),
                '\r' => out.push_str("\\r"),
                '\t' => out.push_str("\\t"),
                c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
                c => out.push(c),
            }
        }
        out.push('"');
    }
    match v {
        Value::Unit => out.push_str("null"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Int(n) => out.push_str(&n.to_string()),
        Value::Float(f) => {
            if f.is_finite() {
                out.push_str(&json_float(*f))
            } else {
                out.push_str("null")
            }
        }
        Value::Size(n) | Value::Duration(n) => out.push_str(&n.to_string()),
        // `percent` lưu dưới dạng `f64` ⇒ cùng quy ước giữ dấu float (#173).
        Value::Percent(p) => {
            if p.is_finite() {
                out.push_str(&json_float(*p))
            } else {
                out.push_str("null")
            }
        }
        Value::Str(s) | Value::Path(s) => esc(s, out),
        Value::Char(c) => esc(&c.to_string(), out),
        Value::Bytes(b) => esc(&String::from_utf8_lossy(b), out),
        Value::List(xs) => {
            out.push('[');
            for (i, x) in xs.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                mo(pretty, depth, out);
                json_tai(x, pretty, depth + 1, out);
            }
            if !xs.is_empty() {
                dong(pretty, depth, out);
            }
            out.push(']');
        }
        Value::Map(kv) => {
            out.push('{');
            for (i, (k, val)) in kv.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                mo(pretty, depth, out);
                let kk = match k {
                    Value::Str(s) | Value::Path(s) => s.clone(),
                    other => other.show(),
                };
                esc(&kk, out);
                out.push_str(if pretty { ": " } else { ":" });
                json_tai(val, pretty, depth + 1, out);
            }
            if !kv.is_empty() {
                dong(pretty, depth, out);
            }
            out.push('}');
        }
        // Bản ghi `Error` dựng sẵn (VD-222 điểm 4): in JSON `{"message": …, …}`.
        Value::Record(name, fs) if name == "Error" => {
            let trong: std::collections::BTreeMap<&str, &Value> =
                fs.iter().map(|(n, v)| (n.as_str(), v)).collect();
            out.push('{');
            mo(pretty, depth, out);
            esc("message", out);
            out.push_str(if pretty { ": " } else { ":" });
            match trong.get("message") {
                Some(Value::Str(t)) => esc(t, out),
                Some(other) => json_tai(other, pretty, depth + 1, out),
                None => out.push_str("\"\""),
            }
            for extra in ["source", "span"] {
                if let Some(v) = trong.get(extra) {
                    out.push(',');
                    mo(pretty, depth, out);
                    esc(extra, out);
                    out.push_str(if pretty { ": " } else { ":" });
                    json_tai(v, pretty, depth + 1, out);
                }
            }
            dong(pretty, depth, out);
            out.push('}');
        }
        Value::Record(_, fs) => {
            out.push('{');
            for (i, (f, val)) in fs.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                mo(pretty, depth, out);
                esc(f, out);
                out.push_str(if pretty { ": " } else { ":" });
                json_tai(val, pretty, depth + 1, out);
            }
            if !fs.is_empty() {
                dong(pretty, depth, out);
            }
            out.push('}');
        }
        // **VD-204 (1)**: `Option`/`Result` thật — biến thể dựng sẵn của enum `Option`/`Result`.
        // `None` ⇒ `null`; `Some(x)` ⇒ encode `x`; `Result` ⇒ `{"ok": v}` / `{"err": e}`.
        Value::Enum(ea, va, ag) if ea == "Option" && va == "None" => out.push_str("null"),
        Value::Enum(ea, va, ag) if ea == "Option" && va == "Some" && ag.len() == 1 => {
            json_tai(&ag[0], pretty, depth, out)
        }
        Value::Enum(ea, va, ag) if ea == "Result" && va == "Ok" && ag.len() == 1 => {
            out.push('{');
            mo(pretty, depth, out);
            esc("ok", out);
            out.push_str(if pretty { ": " } else { ":" });
            json_tai(&ag[0], pretty, depth + 1, out);
            dong(pretty, depth, out);
            out.push('}');
        }
        Value::Enum(ea, va, ag) if ea == "Result" && va == "Err" && ag.len() == 1 => {
            out.push('{');
            mo(pretty, depth, out);
            esc("err", out);
            out.push_str(if pretty { ": " } else { ":" });
            json_tai(&ag[0], pretty, depth + 1, out);
            dong(pretty, depth, out);
            out.push('}');
        }
        // **VD-222 (điểm 4)** — `Json` enum thật: in đúng JSON cho cả 6 biến thể.
        Value::Enum(ea, va, ag) if ea == "Json" => match va.as_str() {
            "Null" => out.push_str("null"),
            "Bool" => {
                if matches!(ag.first(), Some(Value::Bool(true))) {
                    out.push_str("true");
                } else {
                    out.push_str("false");
                }
            }
            "Num" => match ag.first() {
                Some(Value::Int(n)) => out.push_str(&n.to_string()),
                Some(Value::Float(f)) => {
                    if f.is_finite() {
                        out.push_str(&format!("{f}"));
                    } else {
                        out.push_str("null");
                    }
                }
                _ => out.push_str("null"),
            },
            "Str" => {
                if let Some(Value::Str(t)) = ag.first() {
                    esc(t, out);
                } else {
                    out.push_str("\"\"");
                }
            }
            "Arr" => {
                if let Some(Value::List(xs)) = ag.first() {
                    out.push('[');
                    for (i, x) in xs.iter().enumerate() {
                        if i > 0 {
                            out.push(',');
                        }
                        mo(pretty, depth, out);
                        json_tai(x, pretty, depth + 1, out);
                    }
                    if !xs.is_empty() {
                        dong(pretty, depth, out);
                    }
                    out.push(']');
                } else {
                    out.push_str("[]");
                }
            }
            "Obj" => {
                if let Some(Value::Map(kv)) = ag.first() {
                    out.push('{');
                    for (i, (k, val)) in kv.iter().enumerate() {
                        if i > 0 {
                            out.push(',');
                        }
                        mo(pretty, depth, out);
                        let kk = match k {
                            Value::Str(t) | Value::Path(t) => t.clone(),
                            other => other.show(),
                        };
                        esc(&kk, out);
                        out.push_str(if pretty { ": " } else { ":" });
                        json_tai(val, pretty, depth + 1, out);
                    }
                    if !kv.is_empty() {
                        dong(pretty, depth, out);
                    }
                    out.push('}');
                } else {
                    out.push_str("{}");
                }
            }
            _ => out.push_str("null"),
        },
        Value::Enum(_, var, args) => {
            out.push('{');
            mo(pretty, depth, out);
            esc("variant", out);
            out.push_str(if pretty { ": " } else { ":" });
            esc(var, out);
            out.push(',');
            mo(pretty, depth, out);
            esc("value", out);
            out.push_str(if pretty { ": " } else { ":" });
            match args.len() {
                0 => out.push_str("null"),
                1 => json_tai(&args[0], pretty, depth + 1, out),
                _ => {
                    out.push('[');
                    for (i, a) in args.iter().enumerate() {
                        if i > 0 {
                            out.push(',');
                        }
                        mo(pretty, depth + 1, out);
                        json_tai(a, pretty, depth + 2, out);
                    }
                    dong(pretty, depth + 1, out);
                    out.push(']');
                }
            }
            dong(pretty, depth, out);
            out.push('}');
        }
        Value::Range(a, b, _) => {
            out.push('{');
            mo(pretty, depth, out);
            esc("range", out);
            out.push_str(if pretty { ": " } else { ":" });
            out.push('[');
            out.push_str(&a.to_string());
            out.push(',');
            out.push_str(&b.to_string());
            out.push(']');
            dong(pretty, depth, out);
            out.push('}');
        }
        // Hàm/closure không mã hoá được ⇒ `null` (không panic).
        _ => out.push_str("null"),
    }
}

fn noi_duong(a: &str, b: &str) -> String {
    if b.starts_with('/') || a.is_empty() {
        return b.to_string();
    }
    let a = a.trim_end_matches('/');
    format!("{a}/{b}")
}

fn basename(p: &str) -> String {
    let trimmed = p.trim_end_matches('/');
    // **ST-240 / VD-242 §4 (nợ #174)**: POSIX — gốc `/` (và `//`, `///`, …) có basename `/`.
    // Trước đây `trim_end_matches('/')` biến nó thành `""`. Chuỗi rỗng vẫn trả rỗng.
    if trimmed.is_empty() {
        return if p.is_empty() {
            String::new()
        } else {
            "/".to_string()
        };
    }
    match trimmed.rsplit_once('/') {
        Some((_, ten)) if !ten.is_empty() => ten.to_string(),
        _ => trimmed.to_string(),
    }
}

fn dirname(p: &str) -> String {
    let p2 = p.trim_end_matches('/');
    // **ST-240 / VD-242 §4 (nợ #174)**: `dirname("/") == "/"` (không phải `.`). Chuỗi rỗng
    // giữ hành vi cũ (`.`).
    if p2.is_empty() {
        return if p.is_empty() {
            ".".to_string()
        } else {
            "/".to_string()
        };
    }
    match p2.rsplit_once('/') {
        Some((d, _)) => {
            if d.is_empty() {
                "/".to_string()
            } else {
                d.to_string()
            }
        }
        None => ".".to_string(),
    }
}

fn ext(p: &str) -> String {
    let ten = basename(p);
    match ten.rsplit_once('.') {
        Some((_, e)) if !e.is_empty() && !ten.starts_with('.') => e.to_string(),
        _ => String::new(),
    }
}

fn stem(p: &str) -> String {
    let ten = basename(p);
    match ten.rsplit_once('.') {
        Some((s, e)) if !e.is_empty() && !ten.starts_with('.') => s.to_string(),
        _ => ten,
    }
}

#[cfg(test)]
mod st238_tests {
    use super::*;

    #[test]
    fn net_read_ky_so_vao_khong_duoc_cap_phat() {
        assert!(gioi_han_net_read(-1).is_err(), "âm bị từ chối");
        assert!(gioi_han_net_read(i64::MAX).is_err(), "i64::MAX bị từ chối");
        assert!(
            gioi_han_net_read(NET_READ_CAP + 1).is_err(),
            "vượt trần bị từ chối"
        );
        assert_eq!(
            gioi_han_net_read(NET_READ_CAP).unwrap(),
            NET_READ_CAP as usize
        );
        assert_eq!(gioi_han_net_read(0).unwrap(), 0);
        assert_eq!(gioi_han_net_read(1024).unwrap(), 1024);
    }

    #[test]
    fn json_non_finite_bi_phat_hien() {
        assert!(co_so_phi_huu_han(&Value::Float(f64::NAN)));
        assert!(co_so_phi_huu_han(&Value::Float(f64::INFINITY)));
        assert!(co_so_phi_huu_han(&Value::Percent(f64::NEG_INFINITY)));
        assert!(co_so_phi_huu_han(&Value::List(vec![
            Value::Int(1),
            Value::Float(f64::NAN)
        ])));
        assert!(!co_so_phi_huu_han(&Value::Float(1.5)));
        assert!(!co_so_phi_huu_han(&Value::Int(1)));
    }
}
