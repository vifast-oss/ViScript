// SPDX-License-Identifier: MIT OR Apache-2.0
//! Đường dẫn, `vis.lock`, gói cài và vị trí `dòng:cột` — helper thuần của [`super`].

use std::path::{Path, PathBuf};

use vis_diag::MODULE_CODES;
use vis_hir::HItem;
use vis_syntax::{Mode, Span};

use crate::lock_meta::{
    bam as bam_noi_dung, chuan_hoa as chuan_hoa_noi_dung, doc_khoa_cua, GoiKhoa,
};

pub(super) fn chuan_hoa(p: &Path) -> PathBuf {
    std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf())
}

pub(super) fn rel_cua(main_dir: &Path, p: &Path) -> String {
    let c = chuan_hoa(p);
    let goc = chuan_hoa(main_dir);
    // **ST-71**: tệp trong gói cài (`.vis/pkg/<tên>/<ver>/…`) ⇒ rel tổng hợp `pkg/<tên>/<ver>/…`
    // (không dùng `file_name` trần: hai gói cùng `src/main.vis` phải khác khoá cache + chẩn đoán).
    if let Ok(r) = c.strip_prefix(goc.join(".vis").join("pkg")) {
        return format!("pkg/{}", r.to_string_lossy().replace('\\', "/"));
    }
    match c.strip_prefix(&goc) {
        Ok(r) => r.to_string_lossy().replace('\\', "/"),
        Err(_) => c
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default(),
    }
}

/// Thông điệp "chưa cài gói" (ST-71) — nêu **tên** + gợi ý lệnh.
pub(super) fn chua_cai(ten: &str) -> String {
    format!("chưa cài gói `{ten}` — chạy `vis pkg add {ten}`")
}

/// Tách đường dẫn dưới `.vis/pkg/<tên>/<ver>/` ⇒ `(tên, ver, path trong gói)`.
pub(super) fn tach_pkg(main_dir: &Path, path: &Path) -> Option<(String, String, String)> {
    let c = chuan_hoa(path);
    let goc = chuan_hoa(main_dir).join(".vis").join("pkg");
    let r = c.strip_prefix(&goc).ok()?;
    let mut it = r.components();
    let ten = it.next()?.as_os_str().to_str()?.to_string();
    let ver = it.next()?.as_os_str().to_str()?.to_string();
    let con: PathBuf = it.collect();
    Some((ten, ver, con.to_string_lossy().replace('\\', "/")))
}

/// Chọn tệp module trong gói: không có module ⇒ entry quy ước (`src/main.vis` → `main.vis` → tệp
/// `.vis` đầu theo byte); có module ⇒ `<m>.vis` hoặc `src/<m>.vis` (kể cả `.vish`).
pub(super) fn chon_tep_pkg(goi: &GoiKhoa, duoi: &[String]) -> Result<String, String> {
    let co = |p: &str| goi.files.iter().any(|f| f.path == p);
    let mut ung: Vec<String> = Vec::new();
    if duoi.is_empty() {
        // **ST-90 (VD-110)**: lock có `entry` ⇒ dùng **đúng** nó; thiếu tệp trong lock ⇒ lỗi nêu
        // tên (đường gọi map thành `VS0701`). Lock cũ (không `entry`) ⇒ fallback quy ước bên dưới.
        if let Some(e) = &goi.entry {
            if co(e) {
                return Ok(e.clone());
            }
            return Err(format!(
                "gói `{}@{}`: `entry` `{e}` không có trong `vis.lock` (VS0701)",
                goi.name, goi.version
            ));
        }
        for c in ["src/main.vis", "main.vis"] {
            if co(c) {
                ung.push(c.to_string());
            }
        }
        if ung.is_empty() {
            let mut ds: Vec<&str> = goi
                .files
                .iter()
                .map(|f| f.path.as_str())
                .filter(|p| p.ends_with(".vis"))
                .collect();
            ds.sort();
            if let Some(p) = ds.first() {
                ung.push((*p).to_string());
            }
        }
    } else {
        let m = duoi.join("/");
        for c in [
            format!("{m}.vis"),
            format!("src/{m}.vis"),
            format!("{m}.vish"),
            format!("src/{m}.vish"),
        ] {
            if co(&c) && !ung.contains(&c) {
                ung.push(c);
            }
        }
    }
    match ung.len() {
        1 => Ok(ung.remove(0)),
        0 => Err(format!(
            "gói `{}@{}` không có tệp module `{}`",
            goi.name,
            goi.version,
            duoi.join(".")
        )),
        _ => Err(format!(
            "gói `{}@{}`: module `{}` mơ hồ (nhiều tệp khớp)",
            goi.name,
            goi.version,
            duoi.join(".")
        )),
    }
}

/// Đọc + **kiểm sha** một tệp của gói theo `vis.lock`; trả nội dung **đã chuẩn hoá**.
pub(super) fn doc_tep_goi(
    main_dir: &Path,
    ten: &str,
    ver: &str,
    relpath: &str,
) -> Result<String, (&'static vis_diag::Code, String)> {
    // **R9 (VD-178)**: `pkg` lỗi ⇒ `VS0706`; băm lệch `vis.lock` ⇒ `VS0707`.
    let khoa = doc_khoa_cua(main_dir).map_err(|e| (&MODULE_CODES[5], e))?;
    let goi = khoa
        .iter()
        .find(|g| g.name == ten && g.version == ver)
        .ok_or_else(|| {
            (
                &MODULE_CODES[5],
                format!("gói `{ten}@{ver}` không có trong `vis.lock`"),
            )
        })?;
    let tep = goi
        .files
        .iter()
        .find(|f| f.path == relpath)
        .ok_or_else(|| {
            (
                &MODULE_CODES[5],
                format!("tệp `{relpath}` không có trong lock của `{ten}@{ver}`"),
            )
        })?;
    let full = main_dir
        .join(".vis")
        .join("pkg")
        .join(ten)
        .join(ver)
        .join(relpath);
    let bytes = std::fs::read(&full).map_err(|e| {
        (
            &MODULE_CODES[5],
            format!("không đọc được `{relpath}` của gói `{ten}@{ver}`: {e}"),
        )
    })?;
    let text = chuan_hoa_noi_dung(&bytes).map_err(|e| (&MODULE_CODES[5], e))?;
    if bam_noi_dung(&text) != tep.sha256 {
        return Err((
            &MODULE_CODES[6],
            format!("gói `{ten}@{ver}` bị sửa: `{relpath}` — băm lệch `vis.lock`"),
        ));
    }
    Ok(text)
}

/// Ứng viên tệp theo chế độ: `.vish` trước ở chế độ lệnh, `.vis` trước ở chế độ script.
pub(super) fn ung_vien(goc: &Path, ten: &str, mode: Mode) -> Vec<PathBuf> {
    let base = goc.join(ten);
    if base.extension().is_some() {
        return vec![base];
    }
    let (a, b) = if mode == Mode::Command {
        ("vish", "vis")
    } else {
        ("vis", "vish")
    };
    vec![base.with_extension(a), base.with_extension(b)]
}

/// Vị trí `dòng:cột` (cột theo scalar, tab = 4 như `lower`).
pub fn vi_tri(text: &str, span: Span) -> (u32, u32) {
    let mut line = 1u32;
    let mut col = 1u32;
    for (i, c) in text.char_indices() {
        if i >= span.start {
            break;
        }
        match c {
            '\n' => {
                line += 1;
                col = 1;
            }
            '\t' => col += 4,
            _ => col += 1,
        }
    }
    (line, col)
}

pub(super) fn span_cua(it: &HItem) -> Span {
    match it {
        HItem::Fn(f) => f.span,
        HItem::Rec(r) => r.span,
        HItem::Enum(e) => e.span,
        HItem::Alias(a) => a.span,
        HItem::Extern(e) => e.span,
        HItem::Const(c) => c.span,
        HItem::Use(u) => u.span,
        HItem::Trait(t) => t.span,
        HItem::Impl(i) => i.span,
    }
}
