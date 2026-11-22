// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VS3-5a (VD-271)** — base64 + tar **ustar** tối giản cho tarball registry HTTP.
//!
//! Client HTTP hiện có (`vis_interp::net::http_get`) trả `str`, nên tarball đi qua **base64** trên
//! dây; sau giải mã, byte tar được băm **SHA-256** và kiểm **trước khi** ghi bất cứ thứ gì vào
//! `.vis/`. Không crate mới, không `unsafe`.
//!
//! Decoder **chỉ** nhận tệp thường (`typeflag` `0`/NUL); từ chối symlink/thư mục, đường dẫn tuyệt
//! đối/`..`/`\` (dùng chung [`super::kiem_path`]), header sai checksum, dữ liệu bị cắt, trùng path,
//! và vượt trần kích thước/số tệp.

use std::collections::BTreeMap;

/// Trần tarball sau giải mã (client HTTP đã cap 8 MiB base64; đây là hàng rào thứ hai).
const MAX_TAR: usize = 16 * 1024 * 1024;
/// Trần số tệp trong một tarball.
const MAX_TEP: usize = 4_096;

/// Giải **base64 chuẩn** (RFC 4648, có padding); bỏ khoảng trắng hai đầu, **không** nhận khoảng
/// trắng/ký tự lạ ở giữa.
pub(super) fn base64_giai(s: &str) -> Result<Vec<u8>, String> {
    let s = s.trim_matches(|c: char| c.is_ascii_whitespace());
    if s.is_empty() {
        return Err("base64 rỗng".to_string());
    }
    let b = s.as_bytes();
    if b.len() % 4 != 0 {
        return Err(format!("base64 dài {} byte, không chia hết cho 4", b.len()));
    }
    let mut out = Vec::with_capacity(b.len() / 4 * 3);
    let mut pad = 0usize;
    let mut buf = 0u32;
    let mut bits = 0u32;
    for (i, &c) in b.iter().enumerate() {
        if c == b'=' {
            pad += 1;
            continue;
        }
        if pad > 0 {
            return Err(format!("base64 có `=` ở giữa (byte {i})"));
        }
        let v = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => {
                return Err(format!(
                    "base64 có ký tự lạ `{}` tại byte {i}",
                    char::from(c)
                ))
            }
        };
        buf = (buf << 6) | u32::from(v);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8);
        }
    }
    if pad > 2 {
        return Err("base64 có quá 2 dấu `=`".to_string());
    }
    // Bit thấp còn dư phải bằng 0 (chuẩn RFC 4648); từ chối encoder cẩu thả.
    if bits > 0 && buf & ((1u32 << bits) - 1) != 0 {
        return Err("base64 có bit đệm khác 0".to_string());
    }
    Ok(out)
}

/// Đọc trường số **octal** của header tar (dừng ở NUL/space; header cũ dùng space).
fn doc_octal(b: &[u8]) -> Result<u64, String> {
    let het = b.iter().position(|&c| c == 0).unwrap_or(b.len());
    let s = std::str::from_utf8(&b[..het])
        .map_err(|e| format!("trường số tar không phải UTF-8: {e}"))?;
    let t = s.trim_matches(' ');
    if t.is_empty() {
        return Ok(0);
    }
    if !t.bytes().all(|c| (b'0'..=b'7').contains(&c)) {
        return Err(format!("trường số tar không phải octal: `{t}`"));
    }
    u64::from_str_radix(t, 8).map_err(|e| format!("trường số tar tràn: `{t}`: {e}"))
}

/// Chuỗi C (dừng ở NUL) trong một trường header.
fn chuoi_c(b: &[u8]) -> &[u8] {
    let het = b.iter().position(|&c| c == 0).unwrap_or(b.len());
    &b[..het]
}

/// Giải tar **ustar** ⇒ `path → nội dung` (thứ tự byte của path, tất định).
pub(super) fn tar_doc(bytes: &[u8]) -> Result<BTreeMap<String, Vec<u8>>, String> {
    if bytes.is_empty() {
        return Err("tarball rỗng".to_string());
    }
    if bytes.len() > MAX_TAR {
        return Err(format!("tarball vượt giới hạn {MAX_TAR} byte"));
    }
    let mut out: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    let mut off = 0usize;
    while off + 512 <= bytes.len() {
        let h = &bytes[off..off + 512];
        if h.iter().all(|&b| b == 0) {
            break; // khối kết thúc
        }
        // Checksum: tổng byte header với trường checksum (148..156) tính là space.
        let cks_ghi = doc_octal(&h[148..156])?;
        let mut tong: u32 = 0;
        for (i, &b) in h.iter().enumerate() {
            tong += if (148..156).contains(&i) {
                u32::from(b' ')
            } else {
                u32::from(b)
            };
        }
        if u64::from(tong) != cks_ghi {
            return Err(format!(
                "header tar hỏng: checksum ghi {cks_ghi}, thật {tong}"
            ));
        }
        let typeflag = h[156];
        if typeflag != 0 && typeflag != b'0' {
            return Err(format!(
                "tar có mục không phải tệp thường (type `{}`) — không nhận",
                char::from(typeflag)
            ));
        }
        let name = chuoi_c(&h[..100]);
        let prefix = chuoi_c(&h[345..500]);
        let path = if prefix.is_empty() {
            name.to_vec()
        } else if name.is_empty() {
            prefix.to_vec()
        } else {
            let mut p = prefix.to_vec();
            p.push(b'/');
            p.extend_from_slice(name);
            p
        };
        let path = String::from_utf8(path)
            .map_err(|e| format!("tên tệp trong tar không phải UTF-8: {e}"))?;
        super::kiem_path(&path)?;
        if path.ends_with('/') {
            return Err(format!("tar có thư mục `{path}` — chỉ nhận tệp"));
        }
        let size = doc_octal(&h[124..136])?;
        let size = usize::try_from(size).map_err(|_| format!("tệp `{path}`: cỡ quá lớn"))?;
        if size > MAX_TAR {
            return Err(format!("tệp `{path}`: cỡ {size} vượt giới hạn {MAX_TAR}"));
        }
        let start = off + 512;
        let end = start
            .checked_add(size)
            .ok_or_else(|| format!("tệp `{path}`: cỡ tràn"))?;
        if end > bytes.len() {
            return Err(format!("tar bị cắt: thiếu dữ liệu của `{path}`"));
        }
        if out
            .insert(path.clone(), bytes[start..end].to_vec())
            .is_some()
        {
            return Err(format!("tar có tệp trùng `{path}`"));
        }
        if out.len() > MAX_TEP {
            return Err(format!("tar có quá {MAX_TEP} tệp"));
        }
        let pad = (512 - (size % 512)) % 512;
        off = end
            .checked_add(pad)
            .ok_or_else(|| format!("tệp `{path}`: cỡ tràn khi padding"))?;
    }
    if out.is_empty() {
        return Err("tarball không có tệp thường nào".to_string());
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::{base64_giai, tar_doc};

    fn hex(b: &[u8]) -> String {
        b.iter().map(|x| format!("{x:02x}")).collect()
    }

    #[test]
    fn base64_khop_vector_va_tu_choi_sai() {
        assert_eq!(base64_giai("aGVsbG8=").unwrap(), b"hello");
        assert_eq!(base64_giai("").unwrap_err(), "base64 rỗng");
        assert_eq!(
            base64_giai("aGVsbG8").unwrap_err(),
            "base64 dài 7 byte, không chia hết cho 4"
        );
        assert!(base64_giai("aG=sbG8=").unwrap_err().contains("`=` ở giữa"));
        assert!(base64_giai("aGVsbG8*").unwrap_err().contains("ký tự lạ"));
        assert!(base64_giai("====").unwrap_err().contains("quá 2 dấu"));
        // Bit đệm khác 0: "aGVsbG9=" giải ra "hello" nhưng bit cuối sai.
        assert!(base64_giai("aGVsbG9=").unwrap_err().contains("bit đệm"));
        // 0x00 0xff 0x10 — vector nhị phân không phải UTF-8.
        let nhi_phan = base64_giai("AP8Q").unwrap();
        assert_eq!(hex(&nhi_phan), "00ff10");
    }

    fn header(path: &str, len: usize, typeflag: u8) -> [u8; 512] {
        let mut h = [0u8; 512];
        let p = path.as_bytes();
        assert!(p.len() <= 100);
        h[..p.len()].copy_from_slice(p);
        h[100..108].copy_from_slice(b"0000644\0");
        h[124..136].copy_from_slice(format!("{len:011o}\0").as_bytes());
        h[136..148].copy_from_slice(b"00000000000\0");
        for x in &mut h[148..156] {
            *x = b' ';
        }
        h[156] = typeflag;
        h[257..263].copy_from_slice(b"ustar\0");
        h[263..265].copy_from_slice(b"00");
        let sum: u32 = h.iter().map(|&x| u32::from(x)).sum();
        h[148..156].copy_from_slice(format!("{sum:06o}\0 ").as_bytes());
        h
    }

    fn tar(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut out = Vec::new();
        for (path, data) in entries {
            out.extend_from_slice(&header(path, data.len(), b'0'));
            out.extend_from_slice(data);
            let pad = (512 - (data.len() % 512)) % 512;
            out.extend(std::iter::repeat_n(0u8, pad));
        }
        out.extend(std::iter::repeat_n(0u8, 1_024));
        out
    }

    #[test]
    fn tar_doc_dung_va_tu_choi_nguy_hiem() {
        let t = tar(&[("src/a.vis", b"print(1)\n"), ("b.vis", b"x\r\n")]);
        let ds = tar_doc(&t).unwrap();
        assert_eq!(ds["src/a.vis"], b"print(1)\n");
        assert_eq!(ds["b.vis"], b"x\r\n");
        assert_eq!(ds.len(), 2);

        // Đường dẫn `..` bị từ chối (cùng luật `kiem_path` của `vis pkg`).
        let xau = tar(&[("../x", b"x")]);
        assert!(tar_doc(&xau).unwrap_err().contains("tương đối"));
        // Header sai checksum.
        let mut hong = t.clone();
        hong[0] = b'X';
        assert!(tar_doc(&hong).unwrap_err().contains("checksum"));
        // Symlink/thư mục bị từ chối.
        let mut lien_ket = Vec::new();
        lien_ket.extend_from_slice(&header("l", 0, b'2'));
        lien_ket.extend(std::iter::repeat_n(0u8, 1_024));
        assert!(tar_doc(&lien_ket)
            .unwrap_err()
            .contains("không phải tệp thường"));
        // Dữ liệu bị cắt: header thứ hai ở 1024..1536, data `x\r\n` ở 1536..1539.
        assert_eq!(t.len(), 3_072);
        assert!(tar_doc(&t[..1_537]).unwrap_err().contains("bị cắt"));
        // Tar rỗng / chỉ khối 0.
        assert!(tar_doc(&[]).unwrap_err().contains("rỗng"));
        assert!(tar_doc(&[0u8; 1_024]).unwrap_err().contains("không có tệp"));
    }
}
