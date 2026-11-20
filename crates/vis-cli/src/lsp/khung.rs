// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-67 (VD-102)** — khung LSP trên stdio: `Content-Length: N\r\n\r\n<N byte>`.
//!
//! Chỉ header `Content-Length` là **bắt buộc**; `Content-Type` và header lạ được **bỏ qua**.
//! stdout chỉ chứa frame — không bao giờ `println!` ở tầng này.

use std::io::{BufRead, Write};

/// Cap body (16 MiB) — chặn `Content-Length` khổng lồ làm OOM (không phải giới hạn LSP).
const MAX_BODY: usize = 16 * 1024 * 1024;

/// Đọc frame LSP từ `BufRead`. `Ok(None)` = **EOF sạch** (không còn byte nào ở đầu frame mới).
pub struct Doc<R> {
    r: R,
}

impl<R: BufRead> Doc<R> {
    pub fn moi(r: R) -> Self {
        Self { r }
    }

    pub fn doc_frame(&mut self) -> Result<Option<Vec<u8>>, String> {
        let mut content_length: Option<usize> = None;
        let mut dong = String::new();
        // **ST-240 / VD-242 §4 (nợ #184)**: phân biệt ba trường hợp (trước đây `dong.clear()`
        // khiến mọi EOF đều trông như "chưa đọc header" ⇒ exit 0 im lặng):
        //   (a) EOF trước khi nhận **bất kỳ byte nào** của frame mới ⇒ EOF sạch (`Ok(None)`);
        //   (b) EOF giữa các dòng header (header cụt / thiếu dòng trống) ⇒ lỗi rõ;
        //   (c) EOF giữa header và body ⇒ lỗi rõ, nêu số byte đã nhận.
        let mut co_byte_header = false;
        loop {
            dong.clear();
            let n = self
                .r
                .read_line(&mut dong)
                .map_err(|e| format!("lỗi đọc header LSP: {e}"))?;
            if n == 0 {
                if !co_byte_header {
                    return Ok(None); // (a) EOF sạch trước frame mới
                }
                return Err(if content_length.is_some() {
                    "EOF giữa header và body LSP: thiếu dòng trống kết thúc header".to_string()
                } else {
                    "EOF giữa header LSP: header cụt, thiếu `Content-Length`".to_string()
                });
            }
            co_byte_header = true;
            let t = dong.trim_end_matches(['\r', '\n']);
            if t.is_empty() {
                break;
            }
            let Some((k, v)) = t.split_once(':') else {
                continue; // header lạ không có `:` — bỏ qua
            };
            if k.trim().eq_ignore_ascii_case("content-length") {
                content_length = Some(
                    v.trim()
                        .parse::<usize>()
                        .map_err(|_| format!("Content-Length không hợp lệ: `{}`", v.trim()))?,
                );
            }
        }
        let n = content_length.ok_or("frame LSP thiếu `Content-Length`")?;
        if n > MAX_BODY {
            return Err(format!("frame LSP quá lớn: {n} byte (cap {MAX_BODY})"));
        }
        let mut body = vec![0u8; n];
        let mut da_doc = 0usize;
        while da_doc < n {
            match self.r.read(&mut body[da_doc..]) {
                Ok(0) => {
                    return Err(format!(
                        "EOF giữa header và body LSP: mới nhận {da_doc}/{n} byte body"
                    ))
                }
                Ok(k) => da_doc += k,
                Err(e) => return Err(format!("lỗi đọc body LSP ({n} byte): {e}")),
            }
        }
        Ok(Some(body))
    }
}

/// Ghi một frame LSP (header + body), **flush** ngay.
pub fn ghi<W: Write>(w: &mut W, body: &[u8]) -> Result<(), String> {
    write!(w, "Content-Length: {}\r\n\r\n", body.len())
        .map_err(|e| format!("lỗi ghi header LSP: {e}"))?;
    w.write_all(body)
        .and_then(|()| w.flush())
        .map_err(|e| format!("lỗi ghi body LSP: {e}"))
}

#[cfg(test)]
mod st240_tests {
    use super::*;
    use std::io::Cursor;

    /// **#184**: EOF trước khi nhận byte frame mới ⇒ EOF sạch, không phải lỗi.
    #[test]
    fn eof_truoc_frame_la_sach() {
        let mut d = Doc::moi(Cursor::new(b""));
        assert!(matches!(d.doc_frame(), Ok(None)));
    }

    /// **#184**: header cụt (có `Content-Type`, thiếu `Content-Length`) ⇒ lỗi rõ, exit ≠ 0.
    #[test]
    fn header_cut_thieu_content_length() {
        let raw = b"Content-Type: application/vscode-jsonrpc; charset=utf-8\r\n";
        let mut d = Doc::moi(Cursor::new(raw.to_vec()));
        let e = d.doc_frame().expect_err("phải lỗi, không được EOF sạch");
        assert!(
            e.contains("header cụt") && e.contains("Content-Length"),
            "{e}"
        );
    }

    /// **#184**: EOF giữa header và body ⇒ thông điệp **khác** EOF trước frame.
    #[test]
    fn eof_giua_header_va_body() {
        let raw = b"Content-Length: 10\r\n\r\n{";
        let mut d = Doc::moi(Cursor::new(raw.to_vec()));
        let e = d.doc_frame().expect_err("phải lỗi body cụt");
        assert!(
            e.contains("EOF giữa header và body") && e.contains("1/10"),
            "{e}"
        );
    }

    /// Nền tảng: frame đầy đủ vẫn đọc được (không hồi quy bởi kiểm EOF mới).
    #[test]
    fn frame_day_du_van_doc_duoc() {
        let body = br#"{"jsonrpc":"2.0"}"#;
        let mut raw = format!("Content-Length: {}\r\n\r\n", body.len()).into_bytes();
        raw.extend_from_slice(body);
        let mut d = Doc::moi(Cursor::new(raw));
        assert_eq!(d.doc_frame().expect("frame hợp lệ"), Some(body.to_vec()));
        assert!(matches!(d.doc_frame(), Ok(None)));
    }
}
