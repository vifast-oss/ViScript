// SPDX-License-Identifier: MIT OR Apache-2.0
//! Giải mã literal — `docs/SPEC-lex.md` §2 (số), §3 (chuỗi/escape/nội suy).
//!
//! Lexer chỉ *tìm biên* token; mọi việc *hiểu* nội dung nằm ở đây để ST-05 trở đi dùng lại
//! (parser/kiểm kiểu cần đúng các giá trị này), và để test golden đối chiếu **giá trị** chứ
//! không chỉ độ dài token.

use std::fmt;

/// Lỗi giải mã: mã `VS01xx` + offset byte **bên trong literal** (lexer cộng thêm gốc token).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LitError {
    /// Số 3 chữ số cuối của mã (`104` ⇒ `VS0104`).
    pub code: u16,
    pub at: usize,
}

impl LitError {
    const fn new(code: u16, at: usize) -> Self {
        Self { code, at }
    }
}

impl fmt::Display for LitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "VS{:04} tại offset {} trong literal", self.code, self.at)
    }
}

/// Một mảnh của literal chuỗi: văn bản đã giải mã, hoặc biểu thức nội suy (nguyên văn, chưa lex).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StrPart {
    Text(String),
    /// `{expr}` hoặc `{expr:spec}` — `src` là nguyên văn biểu thức, `spec` là phần sau `:` ở
    /// độ sâu ngoặc 0 (cú pháp spec của Rust format, tài liệu thiết kế §2.4).
    Interp {
        src: String,
        spec: Option<String>,
    },
}

/// Nội dung một literal chuỗi đã giải mã.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StrLit {
    pub parts: Vec<StrPart>,
}

impl StrLit {
    /// `true` nếu literal có ít nhất một chỗ nội suy.
    pub fn has_interp(&self) -> bool {
        self.parts
            .iter()
            .any(|p| matches!(p, StrPart::Interp { .. }))
    }

    /// Nối các mảnh văn bản (bỏ phần nội suy) — dùng cho `STRING(…)` của golden và cho
    /// trường hợp không nội suy (khi đó bằng toàn bộ nội dung).
    pub fn text(&self) -> String {
        let mut s = String::new();
        for p in &self.parts {
            if let StrPart::Text(t) = p {
                s.push_str(t);
            }
        }
        s
    }
}

/// Đơn vị kích thước có trong bảng `SIZE` (§1) — dài trước để khớp maximal munch.
pub const SIZE_UNITS: [&str; 9] = ["kib", "mib", "gib", "tib", "kb", "mb", "gb", "tb", "b"];

/// Đơn vị thời lượng theo thứ tự **tăng dần**; chồng chất phải **giảm dần** (§1, §2).
/// `(tên, bậc)` — bậc lớn hơn là đơn vị lớn hơn (`w` = 7).
pub const DURATION_UNITS: [(&str, u8); 8] = [
    ("ns", 0),
    ("us", 1),
    ("ms", 2),
    ("s", 3),
    ("m", 4),
    ("h", 5),
    ("d", 6),
    ("w", 7),
];

/// Bậc của một đơn vị thời lượng, `None` nếu không phải đơn vị.
pub fn duration_rank(unit: &str) -> Option<u8> {
    DURATION_UNITS
        .iter()
        .find(|(u, _)| *u == unit)
        .map(|(_, r)| *r)
}

/// **VD-266**: độ lớn lớn nhất mà lexer chấp nhận cho một literal số nguyên — `i64::MAX + 1`
/// (`2^63`). Đủ để gộp `-` một ngôi thành `i64::MIN`; lớn hơn vẫn là `VS0104` ngay lúc lex.
pub const INT_MAGNITUDE_MAX: u64 = i64::MAX as u64 + 1;

/// Giải mã **độ lớn** của literal số nguyên **không dấu**: `42`, `1_000_000`, `0xff`, `0b101`,
/// `0o17`. `_` chỉ được **giữa hai chữ số** (§2); độ lớn > [`INT_MAGNITUDE_MAX`] là `VS0104` (§1).
///
/// **VD-266**: trả `u64` để tầng từ vựng biểu diễn được `2^63` — giá trị **dương** đó vẫn bị
/// tầng kiểm kiểu từ chối, nhưng `-2^63` (`i64::MIN`) là literal hợp lệ sau khi gộp dấu.
pub fn decode_int_magnitude(text: &str) -> Result<u64, LitError> {
    let (radix, digits, base_off) = if let Some(rest) = text.strip_prefix("0x") {
        (16u32, rest, 2usize)
    } else if let Some(rest) = text.strip_prefix("0b") {
        (2u32, rest, 2usize)
    } else if let Some(rest) = text.strip_prefix("0o") {
        (8u32, rest, 2usize)
    } else {
        (10u32, text, 0usize)
    };
    let mut value: u64 = 0;
    let mut any = false;
    let mut digit_before = false;
    for (idx, ch) in digits.char_indices() {
        if ch == '_' {
            if !digit_before {
                return Err(LitError::new(104, base_off + idx));
            }
            digit_before = false;
            continue;
        }
        let d = ch
            .to_digit(radix)
            .ok_or_else(|| LitError::new(104, base_off + idx))?;
        let mul = value
            .checked_mul(u64::from(radix))
            .ok_or_else(|| LitError::new(104, base_off + idx))?;
        let next = mul
            .checked_add(u64::from(d))
            .ok_or_else(|| LitError::new(104, base_off + idx))?;
        if next > INT_MAGNITUDE_MAX {
            return Err(LitError::new(104, base_off + idx));
        }
        value = next;
        any = true;
        digit_before = true;
    }
    if !any || !digit_before {
        return Err(LitError::new(104, base_off + digits.len()));
    }
    Ok(value)
}

/// Giải mã số nguyên **không dấu** vào `i64`: `42`, `1_000_000`, `0xff`, `0b101`.
/// `_` chỉ được **giữa hai chữ số** (§2); độ lớn > `i64::MAX` là `VS0104` (§1).
///
/// Giữ API cũ cho `size_parts` (kích thước phải nằm trong `i64`); literal `int` của ngôn ngữ
/// đi qua [`decode_int_magnitude`] (lexer) hoặc [`decode_signed_int`] (HIR/pattern).
pub fn decode_int(text: &str) -> Result<i64, LitError> {
    let magnitude = decode_int_magnitude(text)?;
    i64::try_from(magnitude).map_err(|_| LitError::new(104, text.len()))
}

/// Giải mã literal `int` **đã có dấu** (văn bản HIR/pattern, không phải token): `-42`,
/// `+7`, `-0x8000000000000000`.
///
/// Chỉ ca âm được chạm độ lớn `2^63` — đó là `i64::MIN`; dương `2^63` và mọi độ lớn lớn hơn là
/// `VS0104`. Đây là cửa duy nhất để kiểm literal sau khi gộp dấu, và là bộ giải mã dùng chung
/// cho checker + interp + VM.
pub fn decode_signed_int(text: &str) -> Result<i64, LitError> {
    let (negative, body, sign_off) = match text.as_bytes().first() {
        Some(b'-') => (true, &text[1..], 1usize),
        Some(b'+') => (false, &text[1..], 1usize),
        _ => (false, text, 0usize),
    };
    let magnitude =
        decode_int_magnitude(body).map_err(|e| LitError::new(e.code, e.at + sign_off))?;
    if negative {
        // `magnitude <= 2^63`; chỉ `2^63` không ép được vào `i64` dương.
        if magnitude == INT_MAGNITUDE_MAX {
            Ok(i64::MIN)
        } else {
            Ok(-(magnitude as i64))
        }
    } else if magnitude <= i64::MAX as u64 {
        Ok(magnitude as i64)
    } else {
        Err(LitError::new(104, sign_off + body.len()))
    }
}

/// **VD-266**: gộp một dấu `-` một ngôi quanh literal `int` **ở biên** `2^63`.
///
/// `text` là văn bản literal trong HIR: không dấu (`9223372036854775808`, `0x8000000000000000`,
/// `0b1000…`) hoặc đã gộp trước đó (`-9223372036854775808`). Trả văn bản **thập phân** của kết
/// quả — có thể là `9223372036854775808` (dương tràn) để tầng kiểm kiểu báo `VS0104` — hoặc
/// `None` khi độ lớn ≠ `2^63` (literal thường giữ `HExpr::Un` như trước, không đổi HIR).
pub fn fold_neg_min_int(text: &str) -> Option<String> {
    let (negative, body) = match text.as_bytes().first() {
        Some(b'-') => (true, &text[1..]),
        Some(b'+') => (false, &text[1..]),
        _ => (false, text),
    };
    let magnitude = decode_int_magnitude(body).ok()?;
    if magnitude != INT_MAGNITUDE_MAX {
        return None;
    }
    let signed = if negative {
        -(magnitude as i128)
    } else {
        magnitude as i128
    };
    Some((-signed).to_string())
}

/// Tách `5kib`/`0b` thành `(5, "kib")`/`(0, "b")`. Dùng chung cho lexer và golden.
pub fn size_parts(text: &str) -> Option<(i64, &str)> {
    let digits_end = text
        .find(|c: char| !(c.is_ascii_digit() || c == '_'))
        .unwrap_or(text.len());
    let unit = &text[digits_end..];
    if unit.is_empty() || !SIZE_UNITS.contains(&unit) {
        return None;
    }
    let value = decode_int(&text[..digits_end]).ok()?;
    Some((value, unit))
}

/// Giải mã literal `char` (`'ố'`): **đúng một** Unicode scalar, không escape (§1 — escape chỉ
/// có trong các dạng chuỗi, §3). Nhiều hơn/ít hơn một scalar là `VS0105`.
pub fn decode_char(text: &str) -> Result<char, LitError> {
    let body = text
        .strip_prefix('\'')
        .and_then(|s| s.strip_suffix('\''))
        .ok_or_else(|| LitError::new(105, 0))?;
    let mut it = body.chars();
    let ch = it.next().ok_or_else(|| LitError::new(105, 0))?;
    if it.next().is_some() {
        return Err(LitError::new(105, ch.len_utf8()));
    }
    Ok(ch)
}

/// Escape hợp lệ ngoài `\xNN`/`\u{…}` (SPEC-lex §3): `\n \t \r \0 \\ \" \' \{ \}`.
fn simple_escape(c: char) -> Option<char> {
    Some(match c {
        'n' => '\n',
        't' => '\t',
        'r' => '\r',
        '0' => '\0',
        '\\' => '\\',
        '"' => '"',
        '\'' => '\'',
        '{' => '{',
        '}' => '}',
        _ => return None,
    })
}

/// Giải mã `\u{X..X}` tại `chars` (đã bỏ `\u`), trả `(ký tự, số byte đã dùng)`.
fn decode_unicode_escape(rest: &str) -> Result<(char, usize), LitError> {
    let open = rest.find('{').ok_or_else(|| LitError::new(102, 0))?;
    if open != 0 {
        return Err(LitError::new(102, 0));
    }
    let close = rest.find('}').ok_or_else(|| LitError::new(102, 0))?;
    let hex = &rest[1..close];
    if hex.is_empty() || hex.len() > 6 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(LitError::new(102, 0));
    }
    let v = u32::from_str_radix(hex, 16).map_err(|_| LitError::new(102, 0))?;
    let ch = char::from_u32(v).ok_or_else(|| LitError::new(102, 0))?;
    Ok((ch, close + 1))
}

/// Bỏ dấu phân cách của một literal chuỗi theo `kind`, trả thân literal.
fn string_body(kind: crate::lex::token::Kind, text: &str) -> Result<&str, LitError> {
    use crate::lex::token::Kind;
    // Chế độ lệnh: `'…'` là RAW_STRING thô (SPEC-lex §6) — cùng cách giải mã với `r"…"`.
    if kind == Kind::RawStr {
        if let Some(body) = text.strip_prefix('\'').and_then(|s| s.strip_suffix('\'')) {
            return Ok(body);
        }
    }
    let (open, close) = match kind {
        Kind::Str => ("\"", "\""),
        Kind::Path => ("p\"", "\""),
        Kind::RawStr => ("r\"", "\""),
        Kind::MultiStr => ("\"\"\"", "\"\"\""),
        Kind::RawMulti => ("r\"\"\"", "\"\"\""),
        Kind::Bytes => ("b\"", "\""),
        _ => return Err(LitError::new(102, 0)),
    };
    text.strip_prefix(open)
        .and_then(|s| s.strip_suffix(close))
        .ok_or_else(|| LitError::new(101, 0))
}

/// Giải mã một literal chuỗi (mọi dạng trừ `b"…"`).
///
/// - `RawStr`/`RawMulti`: **không** escape, **không** nội suy — `\` và `{` là ký tự thường (§3).
/// - `Path` (**R6a/VD-178**): **không** escape (đường dẫn Windows), **có** nội suy.
/// - `Str`/`MultiStr`: escape + nội suy; `{{`/`}}` cho ra `{`/`}`; `{` không đóng là
///   `VS0108`; escape sai là `VS0102`.
pub fn decode_str(kind: crate::lex::token::Kind, text: &str) -> Result<StrLit, LitError> {
    use crate::lex::token::Kind;
    let body = string_body(kind, text)?;
    if matches!(kind, Kind::RawStr | Kind::RawMulti) {
        return Ok(StrLit {
            parts: vec![StrPart::Text(body.to_string())],
        });
    }
    let mut parts: Vec<StrPart> = Vec::new();
    let mut cur = String::new();
    let bytes = body.as_bytes();
    let mut i = 0usize;
    // **R6a (VD-178)**: `p"…"` không escape — `\` là ký tự thường.
    let no_escape = kind == Kind::Path;
    while i < bytes.len() {
        let ch = body[i..].chars().next().unwrap_or('\u{0}');
        if ch == '\\' && !no_escape {
            let rest = &body[i + 1..];
            if let Some(e) = rest.chars().next() {
                if e == 'u' {
                    let (c, used) =
                        decode_unicode_escape(&rest[1..]).map_err(|_| LitError::new(102, i))?;
                    cur.push(c);
                    i += 1 + 1 + used;
                    continue;
                }
                if e == 'x' {
                    // `\xNN` chỉ hợp lệ trong `b"…"` (§3).
                    return Err(LitError::new(102, i));
                }
                if let Some(c) = simple_escape(e) {
                    cur.push(c);
                    i += 1 + e.len_utf8();
                    continue;
                }
            }
            return Err(LitError::new(102, i));
        }
        if ch == '{' {
            if body[i + 1..].starts_with('{') {
                cur.push('{');
                i += 2;
                continue;
            }
            let (src, spec, used) = scan_interp(body, i)?;
            if !cur.is_empty() {
                parts.push(StrPart::Text(std::mem::take(&mut cur)));
            }
            parts.push(StrPart::Interp { src, spec });
            i += used;
            continue;
        }
        if ch == '}' && body[i + 1..].starts_with('}') {
            cur.push('}');
            i += 2;
            continue;
        }
        cur.push(ch);
        i += ch.len_utf8();
    }
    if !cur.is_empty() || parts.is_empty() {
        parts.push(StrPart::Text(cur));
    }
    Ok(StrLit { parts })
}

/// Quét một chỗ nội suy bắt đầu ở `start` (ký tự `{`), trả `(biểu thức, spec, số byte đã dùng)`.
/// `{` không đóng ⇒ `VS0108` (§3).
fn scan_interp(body: &str, start: usize) -> Result<(String, Option<String>, usize), LitError> {
    let mut depth = 0usize;
    let mut i = start;
    while i < body.len() {
        let ch = body[i..].chars().next().unwrap_or('\u{0}');
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    let inner = &body[start + 1..i];
                    let (src, spec) = split_spec(inner);
                    return Ok((src, spec, i + 1 - start));
                }
            }
            _ => {}
        }
        i += ch.len_utf8();
    }
    Err(LitError::new(108, start))
}

/// Tách `expr` và `:spec` ở **độ sâu ngoặc 0** của một chỗ nội suy.
fn split_spec(inner: &str) -> (String, Option<String>) {
    let mut depth = 0usize;
    for (idx, ch) in inner.char_indices() {
        match ch {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth = depth.saturating_sub(1),
            ':' if depth == 0 => {
                return (inner[..idx].to_string(), Some(inner[idx + 1..].to_string()));
            }
            _ => {}
        }
    }
    (inner.to_string(), None)
}

/// Giải mã `b"…"`: **chỉ ASCII + `\xNN`** (SPEC-lex §1 hàng `BYTES`, §3 hàng `b"..."`).
///
/// Ký tự > `0x7F` là `VS0106`; escape **khác** `\xNN` (kể cả `\\`, `\n`, `\u{…}`) là `VS0102`
/// — bảng §3 ghi "chỉ `\xNN`" cho dạng này. Xem `agent/BLOCKED.md` §3 Q4: tài liệu thiết kế §2.4
/// lại xếp `b"..."` vào nhóm có đủ bộ escape; ST-04 chọn cách đọc **chặt** của SPEC-lex (nguồn
/// đứng trước tài liệu thiết kế) và chờ Agent 0 chốt.
pub fn decode_bytes(text: &str) -> Result<Vec<u8>, LitError> {
    let body = string_body(crate::lex::token::Kind::Bytes, text)?;
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < body.len() {
        let ch = body[i..].chars().next().unwrap_or('\u{0}');
        if ch == '\\' {
            let rest = &body[i + 1..];
            let hex = rest
                .strip_prefix('x')
                .ok_or_else(|| LitError::new(102, i))?;
            let two = hex.get(..2).ok_or_else(|| LitError::new(102, i))?;
            if !two.chars().all(|c| c.is_ascii_hexdigit()) {
                return Err(LitError::new(102, i));
            }
            let v = u8::from_str_radix(two, 16).map_err(|_| LitError::new(102, i))?;
            out.push(v);
            i += 1 + 1 + 2;
            continue;
        }
        if !ch.is_ascii() {
            return Err(LitError::new(106, i));
        }
        out.push(ch as u8);
        i += 1;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lex::token::Kind;

    #[test]
    fn so_nguyen_moi_co_so() {
        assert_eq!(decode_int("42"), Ok(42));
        assert_eq!(decode_int("1_000_000"), Ok(1_000_000));
        assert_eq!(decode_int("0xff"), Ok(255));
        assert_eq!(decode_int("0b101"), Ok(5));
        assert_eq!(decode_int("9223372036854775807"), Ok(i64::MAX));
        // `0o` không do lexer sinh ra nhưng HIR/pattern có thể mang; dùng chung bộ giải mã.
        assert_eq!(decode_int("0o17"), Ok(15));
    }

    #[test]
    fn so_nguyen_sai_va_tran() {
        assert_eq!(decode_int("1_").map_err(|e| e.code), Err(104));
        assert_eq!(decode_int("1__0").map_err(|e| e.code), Err(104));
        assert_eq!(
            decode_int("9223372036854775808").map_err(|e| e.code),
            Err(104)
        );
        assert_eq!(
            decode_int_magnitude("9223372036854775808"),
            Ok(INT_MAGNITUDE_MAX)
        );
        assert_eq!(
            decode_int_magnitude("0x8000000000000000"),
            Ok(INT_MAGNITUDE_MAX)
        );
        // Lớn hơn `i64::MAX + 1` (mọi cơ số) vẫn là lỗi từ vựng.
        assert_eq!(
            decode_int_magnitude("9223372036854775809").map_err(|e| e.code),
            Err(104)
        );
        assert_eq!(
            decode_int_magnitude("0x8000000000000001").map_err(|e| e.code),
            Err(104)
        );
        assert_eq!(
            decode_int_magnitude("0xffffffffffffffff").map_err(|e| e.code),
            Err(104)
        );
    }

    #[test]
    fn so_nguyen_co_dau_chi_am_cham_2_mu_63() {
        assert_eq!(decode_signed_int("-42"), Ok(-42));
        assert_eq!(decode_signed_int("+7"), Ok(7));
        assert_eq!(decode_signed_int("-0x8000000000000000"), Ok(i64::MIN));
        assert_eq!(decode_signed_int("-9223372036854775808"), Ok(i64::MIN));
        assert_eq!(
            decode_signed_int("-9_223_372_036_854_775_808"),
            Ok(i64::MIN)
        );
        // Dương `2^63` không biểu diễn được; âm quá `2^63` cũng vậy.
        assert_eq!(
            decode_signed_int("9223372036854775808").map_err(|e| e.code),
            Err(104)
        );
        assert_eq!(
            decode_signed_int("+9223372036854775808").map_err(|e| e.code),
            Err(104)
        );
        assert_eq!(
            decode_signed_int("-9223372036854775809").map_err(|e| e.code),
            Err(104)
        );
    }

    #[test]
    fn gop_dau_tru_chi_o_bien_2_mu_63() {
        assert_eq!(
            fold_neg_min_int("9223372036854775808").as_deref(),
            Some("-9223372036854775808")
        );
        assert_eq!(
            fold_neg_min_int("0x8000000000000000").as_deref(),
            Some("-9223372036854775808")
        );
        assert_eq!(
            fold_neg_min_int("-9223372036854775808").as_deref(),
            Some("9223372036854775808")
        );
        assert_eq!(
            fold_neg_min_int("-0x8000000000000000").as_deref(),
            Some("9223372036854775808")
        );
        // Literal thường **không** gộp — HIR giữ `Un` như trước.
        assert_eq!(fold_neg_min_int("5"), None);
        assert_eq!(fold_neg_min_int("-5"), None);
        assert_eq!(fold_neg_min_int("9223372036854775807"), None);
    }

    #[test]
    fn chuoi_escape_va_noi_suy() {
        let s = decode_str(Kind::Str, "\"a\\nb\"").expect("hợp lệ");
        assert_eq!(s.text(), "a\nb");
        assert!(!s.has_interp());

        let s = decode_str(Kind::Str, "\"a{x:>8}b\"").expect("hợp lệ");
        assert!(s.has_interp());

        let s = decode_str(Kind::Str, "\"{{\"").expect("hợp lệ");
        assert_eq!(s.text(), "{");

        assert_eq!(
            decode_str(Kind::Str, "\"{x\"").map_err(|e| e.code),
            Err(108)
        );
        assert_eq!(
            decode_str(Kind::Str, "\"a\\qb\"").map_err(|e| e.code),
            Err(102)
        );
    }

    #[test]
    fn chuoi_tho_giu_nguyen() {
        let s = decode_str(Kind::RawStr, "r\"C:\\raw\"").expect("hợp lệ");
        assert_eq!(s.text(), "C:\\raw");
        assert!(!s.has_interp());
    }

    #[test]
    fn char_dung_mot_scalar() {
        assert_eq!(decode_char("'ố'"), Ok('ố'));
        assert_eq!(decode_char("'ab'").map_err(|e| e.code), Err(105));
        assert_eq!(decode_char("''").map_err(|e| e.code), Err(105));
    }

    #[test]
    fn bytes_chi_ascii_va_chi_escape_xnn() {
        assert_eq!(decode_bytes("b\"\\x00\\x01\""), Ok(vec![0, 1]));
        assert_eq!(decode_bytes("b\"ố\"").map_err(|e| e.code), Err(106));
        // §1/§3: dạng `b"..."` **chỉ** có `\xNN` — mọi escape khác là VS0102.
        assert_eq!(decode_bytes("b\"a\\nb\"").map_err(|e| e.code), Err(102));
        assert_eq!(decode_bytes("b\"\\\\\"").map_err(|e| e.code), Err(102));
        assert_eq!(decode_bytes("b\"\\u{41}\"").map_err(|e| e.code), Err(102));
    }

    #[test]
    fn size_parts_tach_dung() {
        assert_eq!(size_parts("5kb"), Some((5, "kb")));
        assert_eq!(size_parts("0b"), Some((0, "b")));
        assert_eq!(size_parts("5mib"), Some((5, "mib")));
        assert_eq!(size_parts("5ms"), None);
    }
}
