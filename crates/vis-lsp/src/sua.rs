// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VS3-7 (VD-271)** — áp `textDocument/didChange` **incremental** (`contentChanges` có `range`).
//!
//! Vị trí LSP là **0-based UTF-16**; dòng kết thúc theo `\n`, `\r\n` hoặc `\r` (LSP 3.17). Văn bản
//! lưu trong LSP là **thô** (giữ CRLF); lex/tyck tự chuẩn hoá khi phân tích, nên ở đây chỉ cần
//! chuyển `(line, character)` → byte offset đúng trên văn bản thô.
//!
//! Nhiều `contentChanges` được áp **tuần tự** (mỗi range tính trên kết quả của thay đổi trước), đúng
//! LSP. Thay đổi **không có** `range` = thay toàn tài liệu (tương thích full sync cũ).
//!
//! Không panic: range ngược/không hợp lệ ⇒ `Err`; caller bỏ qua sự kiện (không làm hỏng buffer).

use super::jsonrpc::ThayDoi;

/// Áp toàn bộ `changes` lên `text`, trả văn bản mới hoặc lỗi mô tả.
pub(super) fn ap_dung(text: &str, changes: &[ThayDoi]) -> Result<String, String> {
    let mut out = text.to_string();
    for (i, c) in changes.iter().enumerate() {
        match c.range {
            None => out = c.text.clone(),
            Some(((sl, sc), (el, ec))) => {
                let start = byte_tu_vi_tri(&out, sl, sc);
                let end = byte_tu_vi_tri(&out, el, ec);
                if end < start {
                    return Err(format!(
                        "contentChanges[{i}]: range ngược ({sl}:{sc} → {el}:{ec})"
                    ));
                }
                if !out.is_char_boundary(start) || !out.is_char_boundary(end) {
                    return Err(format!(
                        "contentChanges[{i}]: range không rơi trên biên ký tự UTF-8"
                    ));
                }
                out.replace_range(start..end, &c.text);
            }
        }
    }
    Ok(out)
}

/// `(line, character)` UTF-16 0-based → byte offset trong `text` **thô**.
///
/// Dòng kết thúc `\r\n` (một dòng), `\n`, hoặc `\r`. `character` vượt quá dòng ⇒ kẹp về **trước**
/// dấu kết thúc dòng (hoặc cuối tệp). `line` vượt tệp ⇒ `text.len()`. Character rơi vào giữa cặp
/// surrogate (dữ liệu client không hợp lệ) ⇒ kẹp về **đầu** ký tự đó.
fn byte_tu_vi_tri(text: &str, line: u32, character: u32) -> usize {
    let mut l = 0u32;
    let mut i = 0usize;
    while l < line && i < text.len() {
        let ch = text[i..].chars().next().expect("biên UTF-8");
        match ch {
            '\r' => {
                i += 1;
                if text[i..].starts_with('\n') {
                    i += 1;
                }
                l += 1;
            }
            '\n' => {
                i += 1;
                l += 1;
            }
            _ => i += ch.len_utf8(),
        }
    }
    if l < line {
        return text.len();
    }
    let mut u = 0u32;
    while i < text.len() {
        let ch = text[i..].chars().next().expect("biên UTF-8");
        if ch == '\r' || ch == '\n' {
            break;
        }
        let w = ch.len_utf16() as u32;
        if u + w > character {
            break;
        }
        u += w;
        i += ch.len_utf8();
        if u == character {
            break;
        }
    }
    i
}

#[cfg(test)]
mod tests {
    use super::{ap_dung, byte_tu_vi_tri};
    use crate::jsonrpc::ThayDoi;

    fn doi(range: ((u32, u32), (u32, u32)), text: &str) -> ThayDoi {
        ThayDoi {
            range: Some(range),
            text: text.to_string(),
        }
    }

    #[test]
    fn vi_tri_lf_crlf_va_cuoi_dong() {
        let t = "a\r\nbb\nc";
        assert_eq!(byte_tu_vi_tri(t, 0, 0), 0);
        assert_eq!(byte_tu_vi_tri(t, 0, 1), 1);
        assert_eq!(byte_tu_vi_tri(t, 1, 0), 3);
        assert_eq!(byte_tu_vi_tri(t, 1, 2), 5);
        assert_eq!(byte_tu_vi_tri(t, 2, 0), 6);
        assert_eq!(byte_tu_vi_tri(t, 2, 9), 7);
        assert_eq!(byte_tu_vi_tri(t, 9, 9), 7);
        // Cuối dòng 1 (trước `\n`) kể cả character vượt.
        assert_eq!(byte_tu_vi_tri(t, 1, 99), 5);
    }

    #[test]
    fn utf16_emoji_va_thay_the() {
        let t = "let s = \"😀\"\n";
        // Quote mở ở 8; emoji chiếm 2 code unit (9..11); quote đóng ở 11.
        assert_eq!(byte_tu_vi_tri(t, 0, 9), 9);
        // 11 = sau emoji (trước quote đóng): dừng ngay khi đủ code unit.
        assert_eq!(byte_tu_vi_tri(t, 0, 11), 13);
        let moi = ap_dung(t, &[doi(((0, 9), (0, 11)), "")]).unwrap();
        assert_eq!(moi, "let s = \"\"\n");
        // Chèn lại emoji trước quote đóng (UTF-16).
        let lai = ap_dung(&moi, &[doi(((0, 9), (0, 9)), "😀")]).unwrap();
        assert_eq!(lai, t);
    }

    #[test]
    fn nhieu_change_tuan_tu_va_full() {
        let t = "let x: int = 1\n";
        let ds = vec![
            doi(((0, 13), (0, 14)), "\"s\""),
            doi(((0, 0), (0, 0)), "let y = 2\n"),
            doi(((1, 4), (1, 5)), "z"),
        ];
        let moi = ap_dung(t, &ds).unwrap();
        assert_eq!(moi, "let y = 2\nlet z: int = \"s\"\n");
        // Full replace (không range) rồi range tiếp theo.
        let full = ThayDoi {
            range: None,
            text: "abc\n".to_string(),
        };
        let sau = ap_dung(t, &[full, doi(((0, 1), (0, 2)), "X")]).unwrap();
        assert_eq!(sau, "aXc\n");
    }

    #[test]
    fn range_ngoai_va_nguoc_bi_tu_choi() {
        let t = "abc\n";
        assert!(ap_dung(t, &[doi(((0, 3), (0, 1)), "x")]).is_err());
        // Vượt dòng ⇒ kẹp về cuối dòng (không panic), thay được.
        assert_eq!(
            ap_dung(t, &[doi(((0, 9), (0, 99)), "X")]).unwrap(),
            "abcX\n"
        );
    }
}
