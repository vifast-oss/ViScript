// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-286 (R0c)** — helper đường dẫn/lệnh của `--grant` `cap[Proc]`: nhận diện tệp chạy được,
//! dò `PATH` (thêm `PATHEXT` trên Windows), phân giải lệnh **một lần lúc khởi động**.
//! Tách khỏi `quyen.rs` để giữ luật R1 (≤ 1500 dòng/tệp); **0 đổi hành vi**.

use std::path::{Path, PathBuf};

/// **ST-97**: tệp thực thi (Unix) — dùng để bắt `--grant` Proc gán nhầm cho `cap[Dir]`.
#[cfg(unix)]
pub(super) fn la_tep_thuc_thi(p: &Path) -> bool {
    la_tep_lenh(p)
}

#[cfg(not(unix))]
pub(super) fn la_tep_thuc_thi(p: &Path) -> bool {
    // **VD-241 § fix(Windows/CRLF #psvn-A4)**: trên non-unix, không có bit `x` — chấp nhận tệp thường
    // làm "chạy được" (khớp `la_tep_lenh`); đối tượng `cap[Dir]` / `cap[File]` nhận tệp thật để check.
    la_tep_lenh(p)
}

/// **ST-123 (VD-130c)**: tệp **chạy được** — Unix: tệp thường + bit `x`; nền khác: tệp thường.
fn la_tep_lenh(p: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::metadata(p)
            .map(|m| m.is_file() && (m.permissions().mode() & 0o111) != 0)
            .unwrap_or(false)
    }
    #[cfg(not(unix))]
    {
        std::fs::metadata(p).map(|m| m.is_file()).unwrap_or(false)
    }
}

/// **ST-123 (VD-130c)**: phân giải **giá trị `--grant` của `cap[Proc]`** thành đường dẫn tuyệt đối
/// **đã canonicalize**, **một lần lúc khởi động** (không phải mỗi lần chạy lệnh).
///
/// - Giá trị **không có `/`** ⇒ dò `PATH` (chỉ nhận [tệp chạy được](la_tep_lenh)), mục khớp đầu
///   tiên — **cùng** luật `execvp` sẽ dùng khi chạy.
/// - Giá trị **có `/`** ⇒ ghép `goc` nếu tương đối, rồi `canonicalize` (theo symlink).
/// - Không phân giải được ⇒ `Err` có chẩn đoán rõ (CLI in ra rồi exit 2), **không** im lặng nhận.
///
/// Các tên khả dĩ để dò trong `PATH`: nguyên tên + các đuôi thực thi Windows (PATHEXT).
/// Unix: chỉ nguyên tên (đúng `execvp`).
#[cfg(not(unix))]
fn ung_vien_path(thu_muc: &Path, v: &str) -> Vec<PathBuf> {
    let mut kq = vec![thu_muc.join(v)];
    if Path::new(v).extension().is_none() {
        let pat = std::env::var("PATHEXT").unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".into());
        for e in pat.split(';') {
            let e = e.trim();
            if !e.is_empty() {
                kq.push(thu_muc.join(format!("{v}{e}")));
            }
        }
    }
    kq
}

#[cfg(unix)]
fn ung_vien_path(thu_muc: &Path, v: &str) -> Vec<PathBuf> {
    vec![thu_muc.join(v)]
}

pub fn phan_giai_lenh(v: &str, goc: &Path) -> Result<PathBuf, String> {
    if !v.contains('/') {
        let path = std::env::var_os("PATH").unwrap_or_default();
        for thu_muc in std::env::split_paths(&path) {
            for ung in ung_vien_path(&thu_muc, v) {
                if let Ok(c) = std::fs::canonicalize(&ung) {
                    if la_tep_lenh(&c) {
                        return Ok(c);
                    }
                }
            }
        }
        return Err(format!(
            "không thấy lệnh `{v}` trong `PATH` — cần đường dẫn tuyệt đối tới tệp thực thi có thật lúc khởi động"
        ));
    }
    let p = Path::new(v);
    let ung = if p.is_absolute() {
        p.to_path_buf()
    } else {
        goc.join(p)
    };
    let c = std::fs::canonicalize(&ung)
        .map_err(|e| format!("không phân giải được `{}`: {e}", ung.display()))?;
    if !la_tep_lenh(&c) {
        return Err(format!(
            "`{}` không phải tệp thực thi (cần tệp thường có quyền chạy)",
            c.display()
        ));
    }
    Ok(c)
}

/// **ST-123 (VD-130c)**: phân giải **lệnh sẽ chạy** (`ten` do chương trình gọi) theo **cùng luật**
/// với `--grant`, tại thời điểm chạy; `None` = không phân giải được ⇒ không thể khớp allowlist.
pub fn chuan_lenh(ten: &str) -> Option<PathBuf> {
    let goc = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    phan_giai_lenh(ten, &goc).ok()
}
