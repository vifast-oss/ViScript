// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-67 (VD-102)** — khung LSP trên stdio: `Content-Length: N\r\n\r\n<N byte>`.
//!
//! Chỉ header `Content-Length` là **bắt buộc**; `Content-Type` và header lạ được **bỏ qua**.
//! stdout chỉ chứa frame — không bao giờ `println!` ở tầng này.

use std::io::{BufRead, Read, Write};

/// Cap body (16 MiB) — chặn `Content-Length` khổng lồ làm OOM (không phải giới hạn LSP).
const MAX_BODY: usize = 16 * 1024 * 1024;
/// Cap **tổng** header của một frame (64 KiB) — chặn dòng header/dãy header vô hạn. LSP thật
/// chỉ dùng vài trăm byte; cap này không ảnh hưởng client hợp lệ.
const MAX_HEADER: usize = 64 * 1024;

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
        let mut tong_header = 0usize;
        loop {
            dong.clear();
            let con = MAX_HEADER.saturating_sub(tong_header);
            if con == 0 {
                return Err(format!(
                    "header LSP quá lớn: vượt cap {MAX_HEADER} byte — từ chối frame"
                ));
            }
            // Đọc tối đa `con + 1` byte cho dòng này: nếu không có `\n` trong giới hạn, đó là
            // header quá lớn (không để `read_line` cấp phát vô hạn). `Interrupted` được thử lại.
            let mut gioi_han = (&mut self.r).take(con as u64 + 1);
            let n = loop {
                match gioi_han.read_line(&mut dong) {
                    Ok(n) => break n,
                    Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                    Err(e) => return Err(format!("lỗi đọc header LSP: {e}")),
                }
            };
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
            tong_header += n;
            if tong_header > MAX_HEADER || dong.len() > con {
                return Err(format!(
                    "header LSP quá lớn: vượt cap {MAX_HEADER} byte — từ chối frame"
                ));
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
                // **VD-258**: `read` có thể trả `Interrupted` (tín hiệu) — thử lại, không coi là
                // lỗi giao thức.
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
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

    /// **VD-258**: header không có newline/dài quá cap ⇒ lỗi rõ, không cấp phát vô hạn.
    #[test]
    fn header_qua_lon_bi_tu_choi() {
        let raw = "X".repeat(MAX_HEADER + 1);
        let mut d = Doc::moi(Cursor::new(raw.into_bytes()));
        let e = d.doc_frame().expect_err("header quá lớn phải lỗi");
        assert!(
            e.contains("header LSP quá lớn") && e.contains("65536"),
            "{e}"
        );
    }

    /// **VD-258**: `Interrupted` ở body (tín hiệu) được thử lại, không thành lỗi giao thức.
    #[test]
    fn interrupted_duoc_thu_lai() {
        struct Ngat {
            du_lieu: Vec<u8>,
            vi_tri: usize,
            da_ngat: bool,
        }
        impl std::io::Read for Ngat {
            fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
                if self.vi_tri >= self.du_lieu.len() {
                    return Ok(0);
                }
                if !self.da_ngat {
                    self.da_ngat = true;
                    return Err(std::io::Error::from(std::io::ErrorKind::Interrupted));
                }
                let n = buf.len().min(self.du_lieu.len() - self.vi_tri);
                buf[..n].copy_from_slice(&self.du_lieu[self.vi_tri..self.vi_tri + n]);
                self.vi_tri += n;
                Ok(n)
            }
        }
        impl BufRead for Ngat {
            fn fill_buf(&mut self) -> std::io::Result<&[u8]> {
                if self.vi_tri >= self.du_lieu.len() {
                    return Ok(&[]);
                }
                if !self.da_ngat {
                    self.da_ngat = true;
                    return Err(std::io::Error::from(std::io::ErrorKind::Interrupted));
                }
                Ok(&self.du_lieu[self.vi_tri..])
            }
            fn consume(&mut self, n: usize) {
                self.vi_tri = (self.vi_tri + n).min(self.du_lieu.len());
            }
        }
        let body = br#"{"jsonrpc":"2.0"}"#;
        let mut raw = format!("Content-Length: {}\r\n\r\n", body.len()).into_bytes();
        raw.extend_from_slice(body);
        let mut d = Doc::moi(Ngat {
            du_lieu: raw,
            vi_tri: 0,
            da_ngat: false,
        });
        assert_eq!(
            d.doc_frame().expect("Interrupted phải retry"),
            Some(body.to_vec())
        );
    }
}
