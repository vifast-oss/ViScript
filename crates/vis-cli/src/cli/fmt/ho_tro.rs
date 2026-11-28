// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-324c** — helper của `vis fmt`, tách **nguyên văn** từ `cli/fmt.rs` (R1 headroom).

use super::*;

/// **VD-270**: một tệp trong `vis fmt --check --json` — `path`, `formatted`, `error`.
/// `None` ⇒ `"error":null`; `Some` ⇒ object lỗi với `code`/`line`/`col` **1-based**.
pub(super) fn tep_json(
    path: &str,
    formatted: bool,
    error: Option<(&str, u32, u32, &str)>,
) -> String {
    let mut s = format!(
        "{{\"path\":\"{}\",\"formatted\":{formatted},",
        json_esc::esc(path)
    );
    match error {
        None => s.push_str("\"error\":null}"),
        Some((code, line, col, message)) => {
            s.push_str("\"error\":{\"code\":\"");
            s.push_str(&json_esc::esc(code));
            s.push_str(&format!("\",\"line\":{line},\"col\":{col},\"message\":\""));
            s.push_str(&json_esc::esc(message));
            s.push_str("\"}}");
        }
    }
    s
}

/// **VD-270**: unified diff **một hunk** (common prefix/suffix) cho `--check --diff`.
/// Trả `None` nếu nội dung giống hệt; chỉ so theo dòng, không thêm timestamp.
pub(super) fn diff_dong(src: &str, moi: &str) -> Option<String> {
    if src == moi {
        return None;
    }
    let a: Vec<&str> = src.lines().collect();
    let b: Vec<&str> = moi.lines().collect();
    if a == b {
        // Chỉ khác newline cuối: hunk thay toàn bộ dòng để không im lặng.
        let mut s = format!("@@ -1,{} +1,{} @@\n", a.len(), b.len());
        for l in &a {
            s.push('-');
            s.push_str(l);
            s.push('\n');
        }
        for l in &b {
            s.push('+');
            s.push_str(l);
            s.push('\n');
        }
        return Some(s);
    }
    let mut pre = 0usize;
    while pre < a.len() && pre < b.len() && a[pre] == b[pre] {
        pre += 1;
    }
    let mut suf = 0usize;
    while suf < a.len() - pre && suf < b.len() - pre && a[a.len() - 1 - suf] == b[b.len() - 1 - suf]
    {
        suf += 1;
    }
    let old = &a[pre..a.len() - suf];
    let new = &b[pre..b.len() - suf];
    let start = pre + 1;
    let mut s = format!("@@ -{start},{} +{start},{} @@\n", old.len(), new.len());
    for l in old {
        s.push('-');
        s.push_str(l);
        s.push('\n');
    }
    for l in new {
        s.push('+');
        s.push_str(l);
        s.push('\n');
    }
    Some(s)
}

/// **VD-270**: `vis fmt --check --diff` — in unified diff cho từng tệp cần sửa.
pub(super) fn xuat_fmt_diff(ds: &[std::path::PathBuf]) -> ExitCode {
    let mut ra = String::new();
    let mut co_loi = false;
    let mut can_sua = false;
    for path in ds {
        match doc_va_dinh_dang(path) {
            KetQuaDoc::Xong { src, moi, .. } => {
                if let Some(h) = diff_dong(&src, &moi) {
                    can_sua = true;
                    ra.push_str(&format!(
                        "--- {}\n+++ {}\n{}",
                        path.display(),
                        path.display(),
                        h
                    ));
                }
            }
            KetQuaDoc::ParseLoi(e) => {
                co_loi = true;
                eprintln!(
                    "# {}: {}",
                    path.display(),
                    dong_chan_doan(e.code, e.line, e.col, &e.message)
                );
            }
            KetQuaDoc::DocLoi(e) => {
                co_loi = true;
                eprintln!("# {}: không đọc được: {e}", path.display());
            }
        }
    }
    if !xuat_stdout(&ra) {
        return ExitCode::from(2);
    }
    ExitCode::from(if co_loi {
        2
    } else if can_sua {
        1
    } else {
        0
    })
}

/// **VD-270**: in **một** object JSON nén trên stdout rồi trả mã thoát đã chọn (0/1/2).
pub(super) fn xuat_fmt_json(entries: &[String], ok: bool, ma_thoat: u8) -> ExitCode {
    let body = format!("{{\"ok\":{ok},\"files\":[{}]}}\n", entries.join(","));
    if !xuat_stdout(&body) {
        return ExitCode::from(2);
    }
    ExitCode::from(ma_thoat)
}
