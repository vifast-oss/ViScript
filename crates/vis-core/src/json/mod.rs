// SPDX-License-Identifier: MIT OR Apache-2.0
//! **`json.decode`** (ST-33) — parse JSON thành [`Value`] của VS0, **không** panic.
//!
//! Ánh xạ (VD-64 (2)):
//! - object ⇒ `map[str, Any]` giữ **thứ tự xuất hiện**; trùng khoá ⇒ **giá trị cuối**, vị trí **đầu**
//! - array ⇒ `list[Any]`; number: **có** `.`/`e`/`E` ⇒ `float`, còn lại ⇒ `int` (tràn `i64` ⇒ lỗi)
//! - string (giải escape **đủ**: `\" \\ \/ \b \f \n \r \t \uXXXX`, **surrogate pair** ghép thành một
//!   scalar `\u{…}`; nửa cặp ⇒ lỗi); `true`/`false`; `null` ⇒ `None` (`Option` kiểu `{error}`)
//!
//! Lỗi ⇒ `Err("JSON không hợp lệ ở byte N: …")`, `N` là **byte** trong chuỗi gốc (không phải char).

use std::rc::Rc;

use crate::value::Value;

pub mod derive;
pub mod esc;
pub use esc::esc;

/// Ghi một chuỗi JSON **có dấu bao** bằng bộ escape duy nhất [`esc`].
fn viet_chuoi(s: &str, out: &mut String) {
    out.push('"');
    out.push_str(&esc(s));
    out.push('"');
}

/// Độ sâu tối đa — chặn đệ quy phá ngăn xếp (`[[[[…`).
const MAX_DEPTH: usize = 128;

/// Parse `s` thành [`Value`]; lỗi cú pháp ⇒ `Err` **có byte offset** (không panic).
pub fn decode(s: &str) -> Result<Value, String> {
    let mut p = P {
        s,
        b: s.as_bytes(),
        i: 0,
    };
    p.khoang_trang();
    let v = p.gia_tri(0)?;
    p.khoang_trang();
    if p.i != p.b.len() {
        return Err(p.loi("còn ký tự thừa sau giá trị"));
    }
    Ok(v)
}

struct P<'a> {
    s: &'a str,
    b: &'a [u8],
    i: usize,
}

impl<'a> P<'a> {
    fn loi(&self, msg: &str) -> String {
        format!("JSON không hợp lệ ở byte {}: {msg}", self.i)
    }

    fn khoang_trang(&mut self) {
        while self.i < self.b.len() && matches!(self.b[self.i], b' ' | b'\t' | b'\n' | b'\r') {
            self.i += 1;
        }
    }

    fn gia_tri(&mut self, depth: usize) -> Result<Value, String> {
        if depth > MAX_DEPTH {
            return Err(self.loi("lồng quá sâu"));
        }
        let Some(&c) = self.b.get(self.i) else {
            return Err(self.loi("hết chuỗi khi đang chờ giá trị"));
        };
        match c {
            b'{' => self.doi_tuong(depth),
            b'[' => self.mang(depth),
            b'"' => Ok(Value::Str(self.chuoi()?)),
            b't' => self.hang("true", Value::Bool(true)),
            b'f' => self.hang("false", Value::Bool(false)),
            b'n' => self.hang("null", Value::none()),
            b'-' | b'0'..=b'9' => self.so(),
            _ => Err(self.loi("ký tự không mở đầu giá trị JSON hợp lệ")),
        }
    }

    fn hang(&mut self, tu: &str, v: Value) -> Result<Value, String> {
        if self.b[self.i..].starts_with(tu.as_bytes()) {
            self.i += tu.len();
            Ok(v)
        } else {
            Err(self.loi("literal không hợp lệ (chờ `true`/`false`/`null`)"))
        }
    }

    fn doi_tuong(&mut self, depth: usize) -> Result<Value, String> {
        self.i += 1; // `{`
        let mut kv: Vec<(Value, Value)> = Vec::new();
        self.khoang_trang();
        if self.b.get(self.i) == Some(&b'}') {
            self.i += 1;
            return Ok(Value::map(kv));
        }
        loop {
            self.khoang_trang();
            if self.b.get(self.i) != Some(&b'"') {
                return Err(self.loi("khoá object phải là chuỗi"));
            }
            let k = self.chuoi()?;
            self.khoang_trang();
            if self.b.get(self.i) != Some(&b':') {
                return Err(self.loi("thiếu `:` sau khoá object"));
            }
            self.i += 1;
            self.khoang_trang();
            let v = self.gia_tri(depth + 1)?;
            // **ST-315b (VD-291 Q2)**: đẩy **mọi** cặp; `VisMap::from_pairs` gộp last-wins
            // (giá trị cuối, vị trí đầu) trong O(1) khấu hao nhờ chỉ mục — bỏ vòng quét O(N²).
            kv.push((Value::Str(k), v));
            self.khoang_trang();
            match self.b.get(self.i) {
                Some(b',') => {
                    self.i += 1;
                }
                Some(b'}') => {
                    self.i += 1;
                    return Ok(Value::map(kv));
                }
                _ => return Err(self.loi("thiếu `,` hoặc `}` trong object")),
            }
        }
    }

    fn mang(&mut self, depth: usize) -> Result<Value, String> {
        self.i += 1; // `[`
        let mut xs: Vec<Value> = Vec::new();
        self.khoang_trang();
        if self.b.get(self.i) == Some(&b']') {
            self.i += 1;
            return Ok(Value::List(xs));
        }
        loop {
            self.khoang_trang();
            xs.push(self.gia_tri(depth + 1)?);
            self.khoang_trang();
            match self.b.get(self.i) {
                Some(b',') => {
                    self.i += 1;
                }
                Some(b']') => {
                    self.i += 1;
                    return Ok(Value::List(xs));
                }
                _ => return Err(self.loi("thiếu `,` hoặc `]` trong array")),
            }
        }
    }

    fn chuoi(&mut self) -> Result<String, String> {
        self.i += 1; // `"`
        let mut out = String::new();
        loop {
            let Some(&c) = self.b.get(self.i) else {
                return Err(self.loi("hết chuỗi khi đang đọc string"));
            };
            match c {
                b'"' => {
                    self.i += 1;
                    return Ok(out);
                }
                b'\\' => self.escape(&mut out)?,
                0x00..=0x1f => return Err(self.loi("ký tự điều khiển thô trong string")),
                _ => {
                    // Giữ nguyên UTF-8: chép **nguyên scalar** bắt đầu ở `self.i`.
                    let ch = self.s[self.i..]
                        .chars()
                        .next()
                        .ok_or_else(|| self.loi("UTF-8 hỏng"))?;
                    out.push(ch);
                    self.i += ch.len_utf8();
                }
            }
        }
    }

    fn escape(&mut self, out: &mut String) -> Result<(), String> {
        self.i += 1; // `\`
        let Some(&c) = self.b.get(self.i) else {
            return Err(self.loi("hết chuỗi sau `\\`"));
        };
        self.i += 1;
        match c {
            b'"' => out.push('"'),
            b'\\' => out.push('\\'),
            b'/' => out.push('/'),
            b'b' => out.push('\u{8}'),
            b'f' => out.push('\u{c}'),
            b'n' => out.push('\n'),
            b'r' => out.push('\r'),
            b't' => out.push('\t'),
            b'u' => {
                let h = self.hex4()?;
                let cp = if (0xD800..=0xDBFF).contains(&h) {
                    // Nửa CAO của surrogate pair ⇒ **bắt buộc** `\uDC00..\uDFFF` theo sau.
                    if self.b.get(self.i) == Some(&b'\\') && self.b.get(self.i + 1) == Some(&b'u') {
                        self.i += 2;
                        let lo = self.hex4()?;
                        if !(0xDC00..=0xDFFF).contains(&lo) {
                            return Err(self.loi("surrogate thấp không hợp lệ"));
                        }
                        0x1_0000 + ((h - 0xD800) as u32) * 0x400 + (lo - 0xDC00) as u32
                    } else {
                        return Err(self.loi("surrogate cao đơn (thiếu nửa thấp)"));
                    }
                } else if (0xDC00..=0xDFFF).contains(&h) {
                    return Err(self.loi("surrogate thấp đơn (thiếu nửa cao)"));
                } else {
                    h as u32
                };
                let ch = char::from_u32(cp).ok_or_else(|| self.loi("mã Unicode không hợp lệ"))?;
                out.push(ch);
            }
            _ => return Err(self.loi("escape không hợp lệ")),
        }
        Ok(())
    }

    fn hex4(&mut self) -> Result<u16, String> {
        if self.i + 4 > self.b.len() {
            return Err(self.loi("`\\u` thiếu 4 chữ số hex"));
        }
        let mut v: u16 = 0;
        for k in 0..4 {
            let d = self.b[self.i + k];
            let n = match d {
                b'0'..=b'9' => d - b'0',
                b'a'..=b'f' => d - b'a' + 10,
                b'A'..=b'F' => d - b'A' + 10,
                _ => return Err(self.loi("`\\u` có ký tự không phải hex")),
            };
            v = v * 16 + n as u16;
        }
        self.i += 4;
        Ok(v)
    }

    fn so(&mut self) -> Result<Value, String> {
        let dau = self.i;
        if self.b.get(self.i) == Some(&b'-') {
            self.i += 1;
        }
        let bd = self.i;
        while matches!(self.b.get(self.i), Some(b'0'..=b'9')) {
            self.i += 1;
        }
        if self.i == bd {
            return Err(self.loi("số thiếu chữ số"));
        }
        let mut la_float = false;
        if self.b.get(self.i) == Some(&b'.') {
            la_float = true;
            self.i += 1;
            let fd = self.i;
            while matches!(self.b.get(self.i), Some(b'0'..=b'9')) {
                self.i += 1;
            }
            if self.i == fd {
                return Err(self.loi("phần thập phân thiếu chữ số"));
            }
        }
        if matches!(self.b.get(self.i), Some(b'e') | Some(b'E')) {
            la_float = true;
            self.i += 1;
            if matches!(self.b.get(self.i), Some(b'+') | Some(b'-')) {
                self.i += 1;
            }
            let ed = self.i;
            while matches!(self.b.get(self.i), Some(b'0'..=b'9')) {
                self.i += 1;
            }
            if self.i == ed {
                return Err(self.loi("phần mũ thiếu chữ số"));
            }
        }
        let tok = &self.s[dau..self.i];
        // Lỗi số ⇒ báo ở **byte bắt đầu token** (không phải cuối token).
        let bat_dau = dau;
        if la_float {
            let f: f64 = tok
                .parse()
                .map_err(|_| format!("JSON không hợp lệ ở byte {bat_dau}: số thực không hợp lệ"))?;
            if !f.is_finite() {
                return Err(format!(
                    "JSON không hợp lệ ở byte {bat_dau}: số thực tràn (không hữu hạn)"
                ));
            }
            Ok(Value::Float(f))
        } else {
            let n: i64 = tok
                .parse()
                .map_err(|_| format!("JSON không hợp lệ ở byte {bat_dau}: số nguyên tràn i64"))?;
            Ok(Value::Int(n))
        }
    }
}

/// **VD-222 (điểm 4)** — bọc kết quả parse JSON vào cây enum `Json` thật
/// (`Null`/`Bool`/`Num`/`Str`/`Arr`/`Obj`). Đầu vào là giá trị **cây cũ** (`Value::Int`,
/// `Value::Map`, `Value::List`, `Value::none()`…) như `decode` sinh ra.
pub fn boc(v: Value) -> Value {
    match v {
        Value::Int(n) => Value::Enum("Json".into(), "Num".into(), vec![Value::Int(n)]),
        Value::Float(f) => Value::Enum("Json".into(), "Num".into(), vec![Value::Float(f)]),
        Value::Bool(b) => Value::Enum("Json".into(), "Bool".into(), vec![Value::Bool(b)]),
        Value::Str(s) => Value::Enum("Json".into(), "Str".into(), vec![Value::Str(s)]),
        Value::List(xs) => Value::Enum(
            "Json".into(),
            "Arr".into(),
            vec![Value::List(xs.into_iter().map(boc).collect())],
        ),
        Value::Map(kv) => {
            let entries = match Rc::try_unwrap(kv) {
                Ok(m) => m.into_entries(),
                Err(rc) => rc.iter().cloned().collect(),
            };
            Value::Enum(
                "Json".into(),
                "Obj".into(),
                vec![Value::map(
                    entries.into_iter().map(|(k, v)| (k, boc(v))).collect(),
                )],
            )
        }
        // `null` (cây cũ dùng `Option.None`) ⇒ biến thể `Null`.
        Value::Enum(ea, va, _) if ea == "Option" && va == "None" => {
            Value::Enum("Json".into(), "Null".into(), vec![])
        }
        other => other,
    }
}

/// **ST-238 / VD-242 (nợ #169)**: `true` nếu cây giá trị chứa số `Float`/`Percent` không hữu hạn.
/// Đi cấu trúc `Value` thuần, không gọi mã người dùng.
pub fn co_so_phi_huu_han(v: &Value) -> bool {
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
pub fn zero_dau(s: &str) -> Option<usize> {
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
///
/// **VD-258**: `Display` của Rust in số rất nhỏ/lớn thành hàng trăm chữ số (ví dụ `5e-324` →
/// 326 chữ số thập phân); `str::parse::<f64>` (decoder JSON) từ chối một số dạng đó, nên encode
/// xong decode lại **hỏng**. Khi chuỗi quá dài, dùng dạng khoa học `{:e}` (luôn có `e` ⇒ vẫn giữ
/// dấu float và round-trip qua decoder).
pub fn float_json(f: f64) -> String {
    if !f.is_finite() {
        return "null".to_string();
    }
    let s = format!("{f}");
    if s.len() > 96 {
        let e = format!("{f:e}");
        if e.contains('e') || e.contains('E') {
            return e;
        }
    }
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
    match v {
        Value::Unit => out.push_str("null"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Int(n) => out.push_str(&n.to_string()),
        Value::Float(f) => {
            if f.is_finite() {
                out.push_str(&crate::json::float_json(*f))
            } else {
                out.push_str("null")
            }
        }
        Value::Size(n) | Value::Duration(n) => out.push_str(&n.to_string()),
        // `percent` lưu dưới dạng `f64` ⇒ cùng quy ước giữ dấu float (#173).
        Value::Percent(p) => {
            if p.is_finite() {
                out.push_str(&crate::json::float_json(*p))
            } else {
                out.push_str("null")
            }
        }
        Value::Str(s) | Value::Path(s) => viet_chuoi(s, out),
        Value::Char(c) => viet_chuoi(&c.to_string(), out),
        Value::Bytes(b) => viet_chuoi(&String::from_utf8_lossy(b), out),
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
                viet_chuoi(&kk, out);
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
            viet_chuoi("message", out);
            out.push_str(if pretty { ": " } else { ":" });
            match trong.get("message") {
                Some(Value::Str(t)) => viet_chuoi(t, out),
                Some(other) => json_tai(other, pretty, depth + 1, out),
                None => out.push_str("\"\""),
            }
            for extra in ["source", "span"] {
                if let Some(v) = trong.get(extra) {
                    out.push(',');
                    mo(pretty, depth, out);
                    viet_chuoi(extra, out);
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
                viet_chuoi(f, out);
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
            viet_chuoi("ok", out);
            out.push_str(if pretty { ": " } else { ":" });
            json_tai(&ag[0], pretty, depth + 1, out);
            dong(pretty, depth, out);
            out.push('}');
        }
        Value::Enum(ea, va, ag) if ea == "Result" && va == "Err" && ag.len() == 1 => {
            out.push('{');
            mo(pretty, depth, out);
            viet_chuoi("err", out);
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
                        out.push_str(&crate::json::float_json(*f));
                    } else {
                        out.push_str("null");
                    }
                }
                _ => out.push_str("null"),
            },
            "Str" => {
                if let Some(Value::Str(t)) = ag.first() {
                    viet_chuoi(t, out);
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
                        viet_chuoi(&kk, out);
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
            viet_chuoi("variant", out);
            out.push_str(if pretty { ": " } else { ":" });
            viet_chuoi(var, out);
            out.push(',');
            mo(pretty, depth, out);
            viet_chuoi("value", out);
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
            viet_chuoi("range", out);
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
