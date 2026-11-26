// SPDX-License-Identifier: MIT OR Apache-2.0
//! **Hiển thị + đọc literal** (`size`/`duration`/`percent`/`str`/`bytes`/`char`) và escape chuỗi
//! — tách khỏi `mod.rs` ở `ST-317b` (thuần move, 0 đổi ngữ nghĩa). `show_eq.rs` vẫn dùng các
//! helper này qua glob `super::*` (mod.rs re-export nội bộ).

/// `char` phải **escape** ký tự điều khiển (`'\n'` in ra `'\\n'`, không phải xuống dòng thật).
pub(crate) fn show_char(c: char) -> String {
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
pub(crate) fn show_float(f: f64) -> String {
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

pub(crate) fn show_size(n: i64) -> String {
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

pub(crate) fn show_duration(n: i64) -> String {
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

pub(crate) fn show_percent(r: f64) -> String {
    let p = r * 100.0;
    if (p - p.round()).abs() < 1e-9 && (-PCT_GIOI_I64..=PCT_GIOI_I64).contains(&p) {
        format!("{}pct", p.round() as i64)
    } else {
        // Phòng thủ: giá trị lọt qua (bytecode tay, test nội bộ) thì in dạng float thật,
        // **không** bão hoà thành `9223372036854775807pct`.
        format!("{p}pct")
    }
}

pub(crate) fn show_bytes(b: &[u8]) -> String {
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
