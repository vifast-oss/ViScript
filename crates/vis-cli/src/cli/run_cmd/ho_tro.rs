// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-322a** — helper của `vis run`, tách **nguyên văn** từ `cli/run_cmd.rs` (R1 headroom).

use super::*;

/// **ST-240 / VD-242 §4 (nợ #177)**: trap runtime mang `Span.file` của module, nhưng
/// `Trap::to_diag` tính `line/col` trên văn bản **tệp chính**. Ánh xạ lại về đúng tệp + văn bản đã
/// chuẩn hoá CRLF→LF (cùng thứ tự byte với `span`) trước khi in. Với tệp chính, giữ nguyên đường
/// dẫn người dùng nhập (`tep`) để không đổi định dạng chẩn đoán cũ.
pub(crate) fn vi_tri_runtime(
    file_rel: &[String],
    files: &[modules::FileUnit],
    tep: &str,
    d: &vis_syntax::Diagnostic,
) -> (String, u32, u32) {
    let rel_chinh = file_rel.first().cloned().unwrap_or_default();
    let rel = file_rel
        .get(d.span.file as usize)
        .cloned()
        .unwrap_or_else(|| rel_chinh.clone());
    // **VM (#177)**: bytecode `Span` chỉ lưu `start/end` (không có `file`), nên trap trong module
    // mang `file = 0`. Nếu tệp chính **không chứa** được khoảng byte này nhưng đúng **một** module
    // chứa ⇒ gán về module đó. Điều kiện "duy nhất" tránh đoán sai khi nhiều tệp cùng đủ dài.
    let mut rel = rel;
    if d.span.file == 0 {
        let cover = |t: &str| {
            let n = vis_syntax::lex::normalize_source(t);
            d.span.end <= n.len()
                && n.is_char_boundary(d.span.start)
                && n.is_char_boundary(d.span.end)
        };
        let chinh_cover = files
            .iter()
            .find(|f| f.rel == rel_chinh)
            .map(|f| cover(&f.text))
            .unwrap_or(false);
        if !chinh_cover {
            let mut ung_vien: Vec<&modules::FileUnit> = files
                .iter()
                .filter(|f| f.rel != rel_chinh && cover(&f.text))
                .collect();
            if ung_vien.len() == 1 {
                rel = ung_vien.remove(0).rel.clone();
            }
        }
    }
    // Đường dẫn hiển thị: tệp chính giữ nguyên đối số người dùng; module ⇒ `thư_mục_chính/rel`.
    let hien = if rel == rel_chinh {
        tep.to_string()
    } else {
        std::path::Path::new(tep)
            .parent()
            .unwrap_or_else(|| std::path::Path::new(""))
            .join(&rel)
            .to_string_lossy()
            .to_string()
    };
    let (mut line, mut col) = (d.line, d.col);
    if let Some(f) = files.iter().find(|f| f.rel == rel) {
        let norm = vis_syntax::lex::normalize_source(&f.text);
        (line, col) = modules::vi_tri(&norm, d.span);
    }
    (hien, line, col)
}

/// Ghi ra stdout **một lần**; `BrokenPipe` (vd `vis fmt … | head`) ⇒ im lặng thoát 0, không panic.
/// Lỗi I/O khác ⇒ báo stderr và trả `false` (caller thoát 2).
pub(crate) fn xuat_stdout(s: &str) -> bool {
    use std::io::Write;
    let stdout = std::io::stdout();
    let mut w = stdout.lock();
    match w.write_all(s.as_bytes()).and_then(|()| w.flush()) {
        Ok(()) => true,
        Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => std::process::exit(0),
        Err(e) => {
            eprintln!("# không ghi được stdout: {e}");
            false
        }
    }
}

/// Escape ký tự điều khiển để chẩn đoán nằm gọn trên **một** dòng (TAP/stderr).
pub(crate) fn dong_chan_doan(code: &str, line: u32, col: u32, message: &str) -> String {
    format!(
        "{code} {line}:{col} {}",
        message
            .replace('\n', "\\n")
            .replace('\r', "\\r")
            .replace('\t', "\\t")
    )
}

/// **ST-238 / VD-242 (nợ #181)**: `--dry`/`--sandbox-check` không được nuốt lỗi cú pháp rồi in
/// “không khai báo quyền” với exit 0. Probes chỉ đọc CST nên trước đây bỏ qua chẩn đoán parse.
/// Ta kiểm parse ngay sau khi đọc tệp, in cùng định dạng `vis run` và trả exit 2 như đường thường.
pub(crate) fn kiem_cu_phap_probe(tep: &str, text: &str, mode: Mode) -> Option<ExitCode> {
    let parsed = vis_syntax::parse(text, mode);
    let loi: Vec<_> = parsed
        .diagnostics
        .iter()
        .filter(|d| d.code.severity == vis_diag::Severity::Error)
        .collect();
    if loi.is_empty() {
        return None;
    }
    for d in loi {
        eprintln!(
            "{tep}:{}:{}: error {}: {}",
            d.line, d.col, d.code.short, d.message
        );
    }
    Some(ExitCode::from(2))
}

/// **VD-270**: đo **đồng hồ tường** quanh **đúng một** lời gọi runtime (`f`).
///
/// Trả `(kết quả, Some(µs))` khi `bat` (`--time`); `(kết quả, None)` khi không xin (không đo).
/// Ranh giới: từ ngay **trước** lời gọi interp/VM (sau nạp–parse–tyck–codegen–cache) tới ngay
/// **sau** khi runtime trả về; **không** tính in stdout/TAP/chẩn đoán hay dọn phiên DB.
pub(super) fn do_luong_chay<T>(bat: bool, f: impl FnOnce() -> T) -> (T, Option<u128>) {
    if !bat {
        return (f(), None);
    }
    let t0 = std::time::Instant::now();
    let kq = f();
    (kq, Some(t0.elapsed().as_micros()))
}

/// **VS3-1d (VD-271)** — chọn frame runtime khi HIR có `async fn`. Chương trình sync không có
/// `async fn` ⇒ giữ nguyên `Interp::run`/`Vm::run` (byte-identical). Không cần cờ CLI.
pub(crate) fn hir_co_async(h: &vis_hir::Hir) -> bool {
    h.items.iter().any(|it| match it {
        vis_hir::hir::HItem::Fn(f) => f.is_async,
        vis_hir::hir::HItem::Impl(i) => i.methods.iter().any(|f| f.is_async),
        vis_hir::hir::HItem::Trait(t) => t.methods.iter().any(|f| f.is_async),
        _ => false,
    })
}
