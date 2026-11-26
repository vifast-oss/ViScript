// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-285 (VD-272)** — `Show` (`Value::show`) + so khớp/so sánh (`eq_value`/`cmp_value`) và
//! các helper băm/so map đi kèm, tách nguyên văn khỏi `value.rs` (R0 refactor, 0 đổi hành vi).
//!
//! Mô-đun con của `value` nên vẫn gọi được helper private của cha (`show_size`, `show_float`, …)
//! và ngược lại; không nhân bản luật.

use super::*;

impl Value {
    /// `Show` — `str(x)` = `x.show()` (§3.5).
    pub fn show(&self) -> String {
        match self {
            Value::Unit => "()".into(),
            Value::Int(i) => i.to_string(),
            Value::Float(f) => show_float(*f),
            Value::Size(n) => show_size(*n),
            Value::Duration(n) => show_duration(*n),
            Value::Percent(r) => show_percent(*r),
            Value::Str(s) => s.clone(),
            Value::Bool(b) => b.to_string(),
            Value::Bytes(b) => show_bytes(b),
            Value::Char(c) => show_char(*c),
            Value::Path(p) => p.clone(),
            Value::List(xs) => {
                let body: Vec<String> = xs.iter().map(Value::show).collect();
                format!("[{}]", body.join(", "))
            }
            Value::Map(kv) => {
                let body: Vec<String> = kv
                    .iter()
                    .map(|(k, v)| format!("{}: {}", show_map_key(k), v.show()))
                    .collect();
                format!("{{{}}}", body.join(", "))
            }
            Value::Record(name, fields) => {
                // **VD-222 (điểm 4)** — bản ghi `Error` dựng sẵn in **thanh gọn** = chỉ `{message}`
                // để các ca `Err(e): print(e)` byte-identical với thời `err(str)`; `source`/`span`
                // vẫn hiện khi in đủ (`{e.message}`) hoặc in bằng `{e}` khi cần đầy đủ.
                if name == "Error" {
                    return fields
                        .iter()
                        .find(|(n, _)| n == "message")
                        .map(|(_, v)| v.show())
                        .unwrap_or_else(|| "{unknown Error}".to_string());
                }
                let body: Vec<String> = fields
                    .iter()
                    .map(|(n, v)| format!("{n}: {}", v.show()))
                    .collect();
                if name.is_empty() {
                    format!("{{{}}}", body.join(", "))
                } else {
                    format!("{name}({})", body.join(", "))
                }
            }
            Value::Enum(_, variant, args) => {
                if args.is_empty() {
                    variant.to_string()
                } else {
                    let body: Vec<String> = args.iter().map(Value::show).collect();
                    format!("{variant}({})", body.join(", "))
                }
            }
            // **VD-204 (1)**: `Option`/`Result` in cùng luật với mọi biến thể enum (`Enum`).
            Value::Range(a, b, incl) => {
                format!("{a}..{}{b}", if *incl { "=" } else { "" })
            }
            // **VD-241 §7/8 (A0, 2026-11-19)** — closure Show đồng nhất một chuỗi: `<fn>` khắp
            // mọi máy (interp lẫn VM), kể cả anonymous. Trước đây interp `<fn>` còn VM `<hàm>`.
            Value::Builtin(name) => format!("<builtin {name}>"),
            Value::Fn(c) => {
                if c.name.is_empty() {
                    "<fn>".into()
                } else {
                    format!("<fn {}>", c.name)
                }
            }
            Value::Vm(_) => "<fn>".to_string(),
            Value::Task(_) => "<tác vụ>".to_string(),
            Value::Chan(_) => "<kênh>".to_string(),
            Value::Cap { kind, root, .. } => {
                if root.is_empty() {
                    format!("<cap[{kind}]>")
                } else {
                    format!("<cap[{kind}] {root}>")
                }
            }
        }
    }

    /// So khớp **cấu trúc** (dùng cho `==`/`!=` và pattern literal); `Percent` so với `Float`
    /// **theo tỉ lệ** (§3.6: `50pct == 0.5`).
    pub fn eq_value(&self, other: &Value) -> bool {
        match (self, other) {
            (Value::Percent(a), Value::Float(b)) | (Value::Float(b), Value::Percent(a)) => *a == *b,
            (Value::Int(a), Value::Float(b)) | (Value::Float(b), Value::Int(a)) => {
                (*a as f64) == *b
            }
            (Value::Int(a), Value::Int(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => a == b,
            (Value::Size(a), Value::Size(b)) | (Value::Duration(a), Value::Duration(b)) => a == b,
            (Value::Percent(a), Value::Percent(b)) => a == b,
            (Value::Str(a), Value::Str(b)) => a == b,
            (Value::Path(a), Value::Path(b)) => a == b,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Char(a), Value::Char(b)) => a == b,
            (Value::Bytes(a), Value::Bytes(b)) => a == b,
            (Value::Unit, Value::Unit) => true,
            (Value::List(a), Value::List(b)) => {
                a.len() == b.len() && a.iter().zip(b).all(|(x, y)| x.eq_value(y))
            }
            (Value::Map(a), Value::Map(b)) => {
                // **ST-239 (VD-242 §3.4)**: so **ánh xạ khoá → giá trị**, không phụ thuộc entry
                // trùng hay thứ tự chèn. Gộp last-wins hai vế rồi so từng khoá một.
                // **VD-270 (perf-map-eq)**: khoá vô hướng + map đủ lớn ⇒ chỉ mục băm O(n);
                // còn lại ⇒ nguyên logic O(n²) cũ.
                // **ST-315b**: hai vế đều có chỉ mục băm (map đủ lớn, khoá vô hướng) ⇒ tra
                // thẳng bucket O(1)/khoá; bất biến "một khoá một entry" cho phép so độ dài.
                if a.co_chi_muc() && b.co_chi_muc() {
                    a.len() == b.len() && a.iter().all(|(k, v)| b.co_cap(k, v))
                } else {
                    match eq_map_vo_huong(a.as_slice(), b.as_slice()) {
                        Some(kq) => kq,
                        None => eq_map_cham(a.as_slice(), b.as_slice()),
                    }
                }
            }
            (Value::Record(_ta, a), Value::Record(_tb, b)) => {
                if a.len() != b.len() {
                    return false;
                }
                // **VD-270**: mọi cặp bản ghi cùng độ dài đều thử **đường nhanh theo vị trí**
                // trước — nếu tên + giá trị từng trường khớp thì chắc chắn phép đa tập cũng
                // khớp, bất kể tên kiểu rỗng/khác nhau (ngữ nghĩa cũ **bỏ qua** tên kiểu).
                // Lệch bất kỳ vị trí nào ⇒ **rơi về** phép so đa tập cũ, nên không đổi ngữ nghĩa
                // với bản ghi đảo thứ tự hoặc có tên trường trùng (đường dựng động).
                if a.iter()
                    .zip(b)
                    .all(|((n1, v1), (n2, v2))| n1 == n2 && v1.eq_value(v2))
                {
                    true
                } else {
                    a.iter()
                        .all(|(n, v)| b.iter().any(|(n2, v2)| n == n2 && v.eq_value(v2)))
                }
            }
            (Value::Enum(ea, va, aa), Value::Enum(eb, vb, ab)) => {
                // **VD-204 (1)**: so khớp **cả tên enum** — `Option.Some(5)` ≠ `Foo.Some(5)`
                // (tránh nhập nhằng sau khi gộp `Opt`/`Res` vào `Enum`).
                ea == eb
                    && va == vb
                    && aa.len() == ab.len()
                    && aa.iter().zip(ab).all(|(x, y)| x.eq_value(y))
            }
            _ => false,
        }
    }

    /// `total_cmp` cho kiểu sắp được; `None` = không so được.
    pub fn cmp_value(&self, other: &Value) -> Option<std::cmp::Ordering> {
        match (self, other) {
            (Value::Int(a), Value::Int(b)) => Some(a.cmp(b)),
            (Value::Size(a), Value::Size(b)) | (Value::Duration(a), Value::Duration(b)) => {
                Some(a.cmp(b))
            }
            (Value::Int(a), Value::Float(b)) => (*a as f64).partial_cmp(b),
            (Value::Float(a), Value::Int(b)) => a.partial_cmp(&(*b as f64)),
            (Value::Float(a), Value::Float(b)) => a.partial_cmp(b),
            (Value::Float(a), Value::Percent(b)) | (Value::Percent(b), Value::Float(a)) => {
                a.partial_cmp(b)
            }
            (Value::Percent(a), Value::Percent(b)) => a.partial_cmp(b),
            (Value::Str(a), Value::Str(b)) | (Value::Path(a), Value::Path(b)) => Some(a.cmp(b)),
            (Value::Char(a), Value::Char(b)) => Some(a.cmp(b)),
            (Value::Bytes(a), Value::Bytes(b)) => Some(a.cmp(b)),
            (Value::Bool(a), Value::Bool(b)) => Some(a.cmp(b)),
            _ => None,
        }
    }
}

/// **VD-270 (perf-map-eq)** — khoá **vô hướng** dùng được chỉ mục băm; biến thể khác
/// (`List`/`Map`/`Record`/`Enum`/`Range`/`Fn`/`Builtin`/`Task`/…) ⇒ `false` để dùng O(n²) cũ.
fn khoa_vo_huong(v: &Value) -> bool {
    matches!(v, Value::Int(_) | Value::Float(_))
        || matches!(v, Value::Size(_) | Value::Duration(_) | Value::Percent(_))
        || matches!(v, Value::Str(_) | Value::Path(_) | Value::Bool(_))
        || matches!(v, Value::Char(_) | Value::Bytes(_))
}

/// Băm **miền số** (`Int`/`Float`/`Percent`) qua `f64`, nhất quán `eq_value`: `Int(1) == Float(1.0)`,
/// `Percent(0.5) == Float(0.5)`; `-0.0 == +0.0` ⇒ cùng băm; NaN giữ bit (bucket không khớp); `Int`
/// > 2^53 có thể trùng băm — bucket còn kiểm.
fn bam_so(x: f64) -> u64 {
    let mut h = Fnv::default();
    h.write_u8(1);
    h.write_u64(if x == 0.0 { 0.0 } else { x }.to_bits());
    h.finish()
}

fn bam_raw(tag: u8, bs: &[u8]) -> u64 {
    let mut h = Fnv::default();
    h.write_u8(tag);
    h.write(bs);
    h.finish()
}

/// Băm khoá vô hướng **nhất quán với `eq_value`**: `Size`/`Duration`, `Str`/`Path`,
/// `Bool`/`Char`/`Bytes` mỗi loại tag riêng; trùng băm (collision/khác kiểu) do bucket quyết.
///
/// **ST-315b**: `pub(super)` để [`super::map::MapIndex`] dùng **cùng** hợp đồng băm với đường
/// so map — một nguồn, không nhân bản luật.
pub(super) fn bam_khoa_vo_huong(v: &Value) -> Option<u64> {
    Some(match v {
        Value::Int(i) => bam_so(*i as f64),
        Value::Float(f) | Value::Percent(f) => bam_so(*f),
        Value::Size(n) => bam_raw(2, &n.to_ne_bytes()),
        Value::Duration(n) => bam_raw(3, &n.to_ne_bytes()),
        Value::Str(s) => bam_raw(4, s.as_bytes()),
        Value::Path(p) => bam_raw(5, p.as_bytes()),
        Value::Bool(b) => bam_raw(6, &[u8::from(*b)]),
        Value::Char(c) => bam_raw(7, &(*c as u32).to_ne_bytes()),
        Value::Bytes(b) => bam_raw(8, b),
        _ => return None,
    })
}

/// Chỉ mục băm một vế map đã gộp (`dau` mở, `sau` liên kết; không `Vec` cho từng bucket).
#[derive(Default)]
struct ChiMucKhoa {
    dau: HashMap<u64, usize, BuildHasherDefault<Fnv>>,
    sau: Vec<usize>,
}

impl ChiMucKhoa {
    /// Tìm entry khớp băm + khoá (+ giá trị nếu `gia_tri`); chain mới→cũ nên khi chỉ tìm khoá
    /// thì ghi đè ⇒ trả entry **cũ nhất**, đúng `gop` cũ (eq_value không bắc cầu).
    fn tim(
        &self,
        xs: &[(Value, Value)],
        gop: &[(usize, u64)],
        h: u64,
        k: &Value,
        gia_tri: Option<&Value>,
    ) -> Option<usize> {
        let mut i = *self.dau.get(&h)?;
        let mut tim = None;
        while i != usize::MAX {
            let (k2, v2) = &xs[gop[i].0];
            if k2.eq_value(k) && gia_tri.is_none_or(|v| v.eq_value(v2)) {
                if gia_tri.is_some() {
                    return Some(i);
                }
                tim = Some(i);
            }
            i = self.sau[i];
        }
        tim
    }

    fn them(&mut self, h: u64) {
        let i = self.sau.len();
        self.sau.push(self.dau.insert(h, i).unwrap_or(usize::MAX));
    }
}

/// Gộp **last-wins** đúng `gop` cũ + dựng chỉ mục băm song song; `None` nếu khoá không vô hướng.
fn gop_vo_huong(xs: &[(Value, Value)]) -> Option<(Vec<(usize, u64)>, ChiMucKhoa)> {
    let mut gop: Vec<(usize, u64)> = Vec::with_capacity(xs.len());
    let mut cm = ChiMucKhoa::default();
    for (j, (k, _)) in xs.iter().enumerate() {
        let h = bam_khoa_vo_huong(k)?;
        match cm.tim(xs, &gop, h, k, None) {
            Some(i) => gop[i].0 = j,
            None => {
                cm.them(h);
                gop.push((j, h));
            }
        }
    }
    Some((gop, cm))
}

const NGUONG_MAP_NHANH: usize = 200;

/// **VD-270 (perf-map-eq)** — đường nhanh O(n) trung bình khi mọi khoá hai vế vô hướng và map
/// đủ lớn: gộp last-wins + chỉ mục băm, tra từng khoá trong bucket (`eq_value`). `None` ⇒ logic cũ.
fn eq_map_vo_huong(a: &[(Value, Value)], b: &[(Value, Value)]) -> Option<bool> {
    // Nhỏ hơn ngưỡng (crossover debug ~96 khoá/vế): O(n²) cũ rẻ hơn dựng chỉ mục.
    if a.len() + b.len() < NGUONG_MAP_NHANH {
        return None;
    }
    if a.iter().any(|(k, _)| !khoa_vo_huong(k)) || b.iter().any(|(k, _)| !khoa_vo_huong(k)) {
        return None;
    }
    let (ga, _) = gop_vo_huong(a)?;
    let (gb, cmb) = gop_vo_huong(b)?;
    if ga.len() != gb.len() {
        return Some(false);
    }
    for &(ia, h) in &ga {
        let (ka, va) = &a[ia];
        if cmb.tim(b, &gb, h, ka, Some(va)).is_none() {
            return Some(false);
        }
    }
    Some(true)
}

/// Logic map O(n²) cũ — **nguyên văn** cho map có khoá không vô hướng.
fn eq_map_cham(a: &[(Value, Value)], b: &[(Value, Value)]) -> bool {
    fn gop(xs: &[(Value, Value)]) -> Vec<&(Value, Value)> {
        let mut out: Vec<&(Value, Value)> = Vec::with_capacity(xs.len());
        for e in xs {
            if let Some(prev) = out.iter_mut().find(|p| p.0.eq_value(&e.0)) {
                *prev = e;
            } else {
                out.push(e);
            }
        }
        out
    }
    let (a, b) = (gop(a), gop(b));
    a.len() == b.len()
        && a.iter()
            .all(|(k, v)| b.iter().any(|(k2, v2)| k.eq_value(k2) && v.eq_value(v2)))
}

fn show_map_key(k: &Value) -> String {
    match k {
        Value::Str(s) => format!("\"{s}\""),
        other => other.show(),
    }
}
