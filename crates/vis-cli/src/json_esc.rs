// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-101b (VD-113)** — escape chuỗi JSON **một nguồn** cho `check --json` + `test --json`:
//! `"`→`\"`, `\`→`\\`, `\b`/`\f`/`\n`/`\r`/`\t`, ký tự `< 0x20` ⇒ `\u00XX`; **giữ UTF-8 thô**
//! (tiếng Việt/emoji) — hợp lệ JSON, không đổi giá trị sau parse. Không dùng serde.

/// Escape một chuỗi thành **nội dung** trong dấu `"` của JSON (không thêm dấu bao).
pub(crate) fn esc(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 8);
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\u{8}' => o.push_str("\\b"),
            '\u{c}' => o.push_str("\\f"),
            '\n' => o.push_str("\\n"),
            '\r' => o.push_str("\\r"),
            '\t' => o.push_str("\\t"),
            c if (c as u32) < 0x20 => o.push_str(&format!("\\u{:04x}", c as u32)),
            c => o.push(c),
        }
    }
    o
}

#[cfg(test)]
mod tests {
    use super::*;
    use vis_interp::json;
    use vis_interp::value::Value;

    fn giai_ma(s: &str) -> String {
        let body = format!("{{\"m\":\"{}\"}}", esc(s));
        let v = json::decode(&body).expect("JSON phải hợp lệ");
        let Value::Map(kv) = v else {
            panic!("không phải map")
        };
        kv.iter()
            .find_map(|(k, v)| match (k, v) {
                (Value::Str(k), Value::Str(v)) if k == "m" => Some(v.clone()),
                _ => None,
            })
            .expect("thiếu m")
    }

    /// (a) `"` + `\` + `\b` + `\f` ⇒ parse được, khôi phục **đúng từng ký tự**.
    #[test]
    fn quote_va_backslash_roundtrip() {
        let goc = "a\"b\\c\u{8}\u{c}";
        assert_eq!(giai_ma(goc), goc);
    }

    /// (b) newline/tab/CR/control `< 0x20` ⇒ parse được, khôi phục đúng.
    #[test]
    fn newline_tab_control_roundtrip() {
        let goc = "dòng1\n\t\r\u{1}\u{1f}";
        assert_eq!(giai_ma(goc), goc);
    }

    /// (c) UTF-8 **thô**: tiếng Việt + emoji không bị escape, parse khôi phục đúng.
    #[test]
    fn utf8_tho_giu_nguyen() {
        let goc = "Việt 😀 日本語";
        assert_eq!(giai_ma(goc), goc);
        assert!(!esc(goc).contains("\\u"), "không được escape UTF-8");
    }
}
