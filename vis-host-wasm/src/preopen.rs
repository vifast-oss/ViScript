// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-299a (VD-277/VD-282)** — chuẩn hoá đường dẫn **thuần từ vựng** cho preopen.
//!
//! Không chạm đĩa: mọi hàm ở đây chỉ tách/ghép chuỗi, nên test được trên native và trên
//! `wasm32-wasip1`. Tầng [`crate::fs`] mới là nơi kiểm containment thật (canonicalize).

/// Tách một đường dẫn **khách** (guest) tuyệt đối thành các thành phần đã chuẩn hoá.
///
/// Quy ước: `/` ⇒ `vec![]`; `.` bị bỏ; `..` lùi một thành phần; lùi quá gốc ⇒ lỗi (không kẹp
/// im lặng — đường dẫn sai phải bị từ chối rõ, không đoán ý).
pub fn chuan_hoa_tuyet_doi(p: &str) -> Result<Vec<String>, String> {
    if p.is_empty() {
        return Err("đường dẫn rỗng".to_string());
    }
    if p.contains('\0') {
        return Err("đường dẫn chứa NUL".to_string());
    }
    if !p.starts_with('/') {
        return Err(format!("preopen phải là đường dẫn tuyệt đối: `{p}`"));
    }
    let mut ra: Vec<String> = Vec::new();
    for seg in p.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                if ra.pop().is_none() {
                    return Err(format!("`..` vượt gốc trong `{p}`"));
                }
            }
            s => ra.push(s.to_string()),
        }
    }
    Ok(ra)
}

/// Tách một đường dẫn **yêu cầu** (từ chương trình) thành `(tuyệt_đối, thành_phần)`.
///
/// Đường dẫn tương đối được phép (giải theo preopen đầu tiên ở tầng `fs`); `..` vượt gốc tương
/// đối cũng bị từ chối — không cho phép leo ra ngoài bằng cú pháp.
pub fn chuan_hoa_yeu_cau(p: &str) -> Result<(bool, Vec<String>), String> {
    if p.is_empty() {
        return Err("đường dẫn rỗng".to_string());
    }
    if p.contains('\0') {
        return Err("đường dẫn chứa NUL".to_string());
    }
    let tuyet_doi = p.starts_with('/');
    let mut ra: Vec<String> = Vec::new();
    for seg in p.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                if ra.pop().is_none() {
                    return Err(format!("`..` vượt gốc trong `{p}`"));
                }
            }
            s => ra.push(s.to_string()),
        }
    }
    Ok((tuyet_doi, ra))
}

/// Ghép các thành phần thành đường dẫn khách tuyệt đối; `[]` ⇒ `/`.
pub fn hien(parts: &[String]) -> String {
    if parts.is_empty() {
        "/".to_string()
    } else {
        format!("/{}", parts.join("/"))
    }
}

/// `true` nếu `con` nằm trong (hoặc bằng) `goc` theo **thành phần** (không theo tiền tố byte:
/// `/data2` **không** thuộc `/data`).
pub fn trong_goc(con: &[String], goc: &[String]) -> bool {
    con.len() >= goc.len() && con[..goc.len()] == *goc
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chuan_hoa_goc() {
        assert_eq!(chuan_hoa_tuyet_doi("/").unwrap(), Vec::<String>::new());
        assert_eq!(
            chuan_hoa_tuyet_doi("/data//a/./b").unwrap(),
            ["data", "a", "b"]
        );
        assert_eq!(chuan_hoa_tuyet_doi("/data/x/..").unwrap(), ["data"]);
        assert!(chuan_hoa_tuyet_doi("/..").is_err());
        assert!(chuan_hoa_tuyet_doi("data").is_err());
        assert!(chuan_hoa_tuyet_doi("").is_err());
        assert!(chuan_hoa_tuyet_doi("/a\0b").is_err());
    }

    #[test]
    fn chuan_hoa_yeu_cau_ok() {
        assert_eq!(
            chuan_hoa_yeu_cau("a/b").unwrap(),
            (false, vec!["a".to_string(), "b".to_string()])
        );
        assert_eq!(
            chuan_hoa_yeu_cau("/data/../x").unwrap(),
            (true, vec!["x".to_string()])
        );
        assert!(chuan_hoa_yeu_cau("../x").is_err());
        assert!(chuan_hoa_yeu_cau("").is_err());
    }

    #[test]
    fn tien_to_theo_thanh_phan() {
        let goc = vec!["data".to_string()];
        assert!(trong_goc(&["data".to_string(), "a".to_string()], &goc));
        assert!(!trong_goc(&["data2".to_string()], &goc));
        assert!(trong_goc(&[], &[]));
    }

    #[test]
    fn hien_goc() {
        assert_eq!(hien(&[]), "/");
        assert_eq!(hien(&["a".to_string(), "b".to_string()]), "/a/b");
    }
}
