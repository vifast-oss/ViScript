// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-46 (VD-42)** — `ls --long`: quyền POSIX + thời gian UTC — tách khỏi `builtin.rs`
//! để giữ luật **R1** (≤ 1.500 dòng/tệp).
//!
//! **VD-258** — thêm helper `std.path` thuần chuỗi (nhận biết Windows/ổ đĩa) ở đây để
//! `stdlib.rs` không vượt R1; cả hai đều là tiện ích chuỗi không chạm đĩa.

/// `ls --long` (ST-46): quyền `rw-r--r--` từ bit POSIX; `0` ⇒ `-`.
pub fn chuoi_quyen(mode: u32) -> String {
    if mode == 0 {
        return "-".to_string();
    }
    let mut s = String::with_capacity(9);
    for (bit, ch) in [
        (0o400, 'r'),
        (0o200, 'w'),
        (0o100, 'x'),
        (0o040, 'r'),
        (0o020, 'w'),
        (0o010, 'x'),
        (0o004, 'r'),
        (0o002, 'w'),
        (0o001, 'x'),
    ] {
        s.push(if mode & bit != 0 { ch } else { '-' });
    }
    s
}

/// `ls --long` (ST-46): `YYYY-MM-DD HH:MM` theo **UTC** (civil-from-days, không thêm crate).
pub fn thoi_gian_utc(ns: i64) -> String {
    if ns == 0 {
        return "-".to_string();
    }
    let mut giay = ns.div_euclid(1_000_000_000);
    let ngay = giay.div_euclid(86_400);
    giay = giay.rem_euclid(86_400);
    // Howard Hinnant: days since 1970-01-01 → (y, m, d).
    let z = ngay + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!(
        "{y:04}-{m:02}-{d:02} {:02}:{:02}",
        giay / 3600,
        (giay % 3600) / 60
    )
}

// ===========================================================================
// VD-258 — `std.path` nhận biết nền tảng: trên Windows chấp nhận cả `\` lẫn `/`,
// ổ đĩa `C:\`/`C:/` (và UNC `\\server\share`); POSIX giữ **nguyên** hành vi cũ.
// Helper nhận cờ `windows` để test thuần trên Linux (không cần runtime Windows).
// ===========================================================================

pub(crate) fn la_phan_cach(c: char, windows: bool) -> bool {
    c == '/' || (windows && c == '\\')
}

/// Vị trí sau `X:` nếu `p` mở đầu bằng ổ đĩa Windows (`C:` / `c:`).
fn o_dia(p: &str) -> Option<usize> {
    let b = p.as_bytes();
    (b.len() >= 2 && b[0].is_ascii_alphabetic() && b[1] == b':').then_some(2)
}

/// Tuyệt đối? POSIX: bắt đầu `/`. Windows: `X:\`/`X:/`, UNC `\\server\share`, hoặc `//server`.
/// `C:` (drive-relative) và `\foo` (rooted, không drive) **không** tuyệt đối — cùng quy ước
/// `std::path::Path::is_absolute` của Rust.
pub(crate) fn la_tuyet_doi(p: &str, windows: bool) -> bool {
    if !windows {
        return p.starts_with('/');
    }
    if p.starts_with("\\\\") || p.starts_with("//") {
        return true;
    }
    match o_dia(p) {
        Some(n) => matches!(p.as_bytes().get(n), Some(b'/') | Some(b'\\')),
        None => false,
    }
}

fn noi_duong_posix(a: &str, b: &str) -> String {
    if b.starts_with('/') || a.is_empty() {
        return b.to_string();
    }
    let a = a.trim_end_matches('/');
    format!("{a}/{b}")
}

pub(crate) fn noi_duong_voi(a: &str, b: &str, windows: bool) -> String {
    if !windows {
        return noi_duong_posix(a, b);
    }
    if la_tuyet_doi(b, true) || a.is_empty() {
        return b.to_string();
    }
    // Đường dẫn có ổ đĩa (kể cả drive-relative `D:foo`) thay thế `a` — cùng quy ước
    // `PathBuf::push` trên Windows; không ghép thành `C:\a\D:foo`.
    if o_dia(b).is_some() {
        return b.to_string();
    }
    // `\foo` / `/foo`: rooted nhưng không drive — giữ drive của `a` nếu có (`C:` + `\foo`).
    if b.starts_with('\\') || b.starts_with('/') {
        return match o_dia(a) {
            Some(n) => format!("{}{}", &a[..n], b),
            None => b.to_string(),
        };
    }
    let a_trim = a.trim_end_matches(|c| la_phan_cach(c, true));
    let a_trim = if a_trim.is_empty() { a } else { a_trim };
    if a_trim.is_empty() {
        return b.to_string();
    }
    // Giữ kiểu separator của `a` nếu nó đã dùng `/`; còn lại dùng `\` (mặc định Windows).
    let sep = if a_trim.contains('/') && !a_trim.contains('\\') {
        '/'
    } else {
        '\\'
    };
    // `C:` sau khi trim có thể đến từ `C:` trần / `C:\` / `C:/` — giữ separator gốc nếu có
    // (`C:/` + `x` ⇒ `C:/x`), còn `C:` trần dùng mặc định `\`.
    let sep = if a_trim.ends_with(':') {
        match a.as_bytes().get(a_trim.len()) {
            Some(b'/') => '/',
            _ => '\\',
        }
    } else {
        sep
    };
    format!("{a_trim}{sep}{b}")
}

fn basename_posix(p: &str) -> String {
    let trimmed = p.trim_end_matches('/');
    // **ST-240 / VD-242 §4 (nợ #174)**: POSIX — gốc `/` (và `//`, `///`, …) có basename `/`.
    // Trước đây `trim_end_matches('/')` biến nó thành `""`. Chuỗi rỗng vẫn trả rỗng.
    if trimmed.is_empty() {
        return if p.is_empty() {
            String::new()
        } else {
            "/".to_string()
        };
    }
    match trimmed.rsplit_once('/') {
        Some((_, ten)) if !ten.is_empty() => ten.to_string(),
        _ => trimmed.to_string(),
    }
}

pub(crate) fn basename_voi(p: &str, windows: bool) -> String {
    if !windows {
        return basename_posix(p);
    }
    let trimmed = p.trim_end_matches(|c| la_phan_cach(c, true));
    if trimmed.is_empty() {
        return if p.is_empty() {
            String::new()
        } else {
            "\\".to_string()
        };
    }
    if let Some(n) = o_dia(trimmed) {
        let rest = &trimmed[n..];
        // `C:` / `C:\` ⇒ không có tên; `C:foo` ⇒ `foo` (drive-relative).
        if rest.is_empty() {
            return String::new();
        }
        if !rest.starts_with('\\') && !rest.starts_with('/') {
            return rest.to_string();
        }
    }
    match trimmed.rsplit(|c| la_phan_cach(c, true)).next() {
        Some(ten) if !ten.is_empty() => ten.to_string(),
        _ => trimmed.to_string(),
    }
}

fn dirname_posix(p: &str) -> String {
    let p2 = p.trim_end_matches('/');
    // **ST-240 / VD-242 §4 (nợ #174)**: `dirname("/") == "/"` (không phải `.`). Chuỗi rỗng
    // giữ hành vi cũ (`.`).
    if p2.is_empty() {
        return if p.is_empty() {
            ".".to_string()
        } else {
            "/".to_string()
        };
    }
    match p2.rsplit_once('/') {
        Some((d, _)) => {
            if d.is_empty() {
                "/".to_string()
            } else {
                d.to_string()
            }
        }
        None => ".".to_string(),
    }
}

pub(crate) fn dirname_voi(p: &str, windows: bool) -> String {
    if !windows {
        return dirname_posix(p);
    }
    let p2 = p.trim_end_matches(|c| la_phan_cach(c, true));
    if p2.is_empty() {
        return if p.is_empty() {
            ".".to_string()
        } else {
            "\\".to_string()
        };
    }
    if let Some(n) = o_dia(p2) {
        // `C:` (sau khi bỏ separator cuối) là gốc ổ ⇒ `C:\` / `C:/`.
        if n == p2.len() {
            let sep = if p.as_bytes().get(n) == Some(&b'/') {
                '/'
            } else {
                '\\'
            };
            return format!("{}{sep}", &p2[..n]);
        }
    }
    match p2.rsplit_once(|c| la_phan_cach(c, true)) {
        Some((d, _)) => {
            if d.is_empty() {
                "\\".to_string()
            } else if o_dia(d) == Some(d.len()) {
                // `C:` + separator ⇒ giữ kiểu separator của chuỗi gốc nếu có.
                let sep = if p2.as_bytes().get(d.len()) == Some(&b'/') {
                    '/'
                } else {
                    '\\'
                };
                format!("{d}{sep}")
            } else {
                d.to_string()
            }
        }
        None => ".".to_string(),
    }
}

pub(crate) fn ext_voi(p: &str, windows: bool) -> String {
    let ten = basename_voi(p, windows);
    match ten.rsplit_once('.') {
        Some((_, e)) if !e.is_empty() && !ten.starts_with('.') => e.to_string(),
        _ => String::new(),
    }
}

pub(crate) fn stem_voi(p: &str, windows: bool) -> String {
    let ten = basename_voi(p, windows);
    match ten.rsplit_once('.') {
        Some((s, e)) if !e.is_empty() && !ten.starts_with('.') => s.to_string(),
        _ => ten,
    }
}

#[cfg(test)]
mod vd258_tests {
    use super::*;

    #[test]
    fn path_windows_o_dia_va_separator() {
        // Tuyệt đối: ổ đĩa + root, UNC; `C:`/`C:foo`/`\foo` không tuyệt đối.
        for p in [
            "C:\\x",
            "C:/x",
            "c:\\",
            "\\\\server\\share",
            "//server/share",
        ] {
            assert!(la_tuyet_doi(p, true), "Windows phải tuyệt đối: {p:?}");
        }
        for p in ["C:", "C:foo", "\\foo", "/foo", "foo", ""] {
            assert!(!la_tuyet_doi(p, true), "Windows không tuyệt đối: {p:?}");
        }
        // POSIX giữ nguyên: chỉ `/` đầu.
        assert!(la_tuyet_doi("/x", false));
        assert!(!la_tuyet_doi("C:\\x", false));
        assert!(!la_tuyet_doi("\\x", false));
    }

    #[test]
    fn path_windows_join_basename_dirname_ext_stem() {
        assert_eq!(noi_duong_voi("C:\\a", "b.txt", true), "C:\\a\\b.txt");
        assert_eq!(noi_duong_voi("C:/a", "b.txt", true), "C:/a/b.txt");
        assert_eq!(noi_duong_voi("C:\\a\\", "b", true), "C:\\a\\b");
        assert_eq!(noi_duong_voi("C:", "b", true), "C:\\b");
        // Tuyệt đối thắng `a`.
        assert_eq!(noi_duong_voi("C:\\a", "D:\\b", true), "D:\\b");
        // Drive-relative cũng thay thế `a` (không ghép `C:\a\D:foo`).
        assert_eq!(noi_duong_voi("C:\\a", "D:foo", true), "D:foo");
        // Rooted không drive: giữ drive của `a`.
        assert_eq!(noi_duong_voi("C:\\a", "\\b", true), "C:\\b");
        // **VD-259** — UNC/root edge: ghép giữ separator, basename root share, đuôi nhiều chấm.
        assert_eq!(
            noi_duong_voi("\\\\server\\share", "x", true),
            "\\\\server\\share\\x"
        );
        assert_eq!(noi_duong_voi("C:\\", "x", true), "C:\\x");
        assert_eq!(noi_duong_voi("C:/", "x", true), "C:/x");
        assert_eq!(basename_voi("\\\\server\\share\\", true), "share");
        assert_eq!(
            dirname_voi("\\\\server\\share\\x", true),
            "\\\\server\\share"
        );
        assert_eq!(ext_voi("C:\\a\\b.tar.gz", true), "gz");
        assert_eq!(stem_voi("C:\\a\\b.tar.gz", true), "b.tar");
        // `\\?\` (verbatim) và UNC đều tuyệt đối; `C:foo` vẫn là drive-relative.
        assert!(la_tuyet_doi("\\\\?\\C:\\x", true));
        assert!(!la_tuyet_doi("C:foo", true));

        assert_eq!(basename_voi("C:\\a\\b.txt", true), "b.txt");
        assert_eq!(basename_voi("C:/a/b.txt", true), "b.txt");
        assert_eq!(basename_voi("C:\\a\\", true), "a");
        assert_eq!(basename_voi("C:\\", true), "");
        assert_eq!(basename_voi("C:foo", true), "foo");
        assert_eq!(dirname_voi("C:\\a\\b.txt", true), "C:\\a");
        assert_eq!(dirname_voi("C:/a/b.txt", true), "C:/a");
        assert_eq!(dirname_voi("C:\\a", true), "C:\\");
        assert_eq!(dirname_voi("C:/a", true), "C:/");
        assert_eq!(dirname_voi("a", true), ".");
        assert_eq!(ext_voi("C:\\a\\b.txt", true), "txt");
        assert_eq!(stem_voi("C:\\a\\b.txt", true), "b");
        assert_eq!(ext_voi("C:\\a\\.bashrc", true), "");
        assert_eq!(stem_voi("C:\\a\\.bashrc", true), ".bashrc");
    }

    #[test]
    fn path_posix_khong_doi() {
        assert_eq!(noi_duong_voi("/a/b", "c.txt", false), "/a/b/c.txt");
        assert_eq!(noi_duong_voi("/a/b", "/c", false), "/c");
        assert_eq!(noi_duong_voi("", "c", false), "c");
        assert_eq!(basename_voi("/a/b.txt", false), "b.txt");
        assert_eq!(basename_voi("/", false), "/");
        assert_eq!(dirname_voi("/a/b.txt", false), "/a");
        assert_eq!(dirname_voi("/", false), "/");
        assert_eq!(dirname_voi("a", false), ".");
        assert_eq!(ext_voi("/a/b.txt", false), "txt");
        assert_eq!(stem_voi("/a/b.txt", false), "b");
        // Backslash là ký tự tên tệp hợp lệ trên POSIX — không phải separator.
        assert_eq!(basename_voi("a\\b.txt", false), "a\\b.txt");
        assert_eq!(dirname_voi("a\\b.txt", false), ".");
    }
}
