// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-116b**: `vis fmt` — tách nguyên khối từ `main.rs`, không đổi hành vi.

use crate::*;
use std::process::ExitCode;

/// **ST-85**: đếm dòng lệch thụt lề của `.vish` theo chuẩn **4 × độ sâu khối**; `None` nếu tệp
/// không parse được (tab cũng là lỗi lexer `VS0107`).
fn dem_lech_thut_le(text: &str) -> Option<usize> {
    use vis_syntax::{Mode, Node, NodeKind};
    let out = vis_syntax::parse(text, Mode::Script);
    if out.diagnostics.is_empty() {
        fn walk(n: &Node, depth: u32, out: &mut Vec<(usize, usize, u32)>) {
            for c in n.children() {
                let d = if matches!(c.kind, NodeKind::Suite | NodeKind::CmdBlock) {
                    out.push((c.span.start, c.span.end, depth + 1));
                    depth + 1
                } else {
                    depth
                };
                walk(c, d, out);
            }
        }
        let mut suites: Vec<(usize, usize, u32)> = Vec::new();
        walk(&out.cst.root, 0, &mut suites);
        let mut lech = 0;
        let mut off = 0;
        for line in text.split_inclusive('\n') {
            let noi_dung = line.trim_end_matches(['\n', '\r']);
            if !noi_dung.trim().is_empty() {
                let indent = noi_dung.len() - noi_dung.trim_start_matches([' ', '\t']).len();
                let mut depth = 0;
                for (s, e, d) in &suites {
                    if *s <= off && off < *e {
                        depth = depth.max(*d);
                    }
                }
                if indent != (depth * 4) as usize {
                    lech += 1;
                }
            }
            off += line.len();
        }
        return Some(lech);
    }
    // Thân `cmd` chỉ là **lệnh** (script parse không qua) ⇒ ước lượng khối theo dòng: dòng cột 1
    // mở/đóng khối `cmd`/`impl`/`type`; dòng thụt trong khối chuẩn **4**. Đây là **đếm** cho báo
    // cáo, không dùng để sửa.
    let mut trong = false;
    let mut lech = 0;
    let mut co_noi_dung = false;
    for line in text.lines() {
        let noi_dung = line.trim_end();
        if noi_dung.trim().is_empty() {
            continue;
        }
        co_noi_dung = true;
        let indent = noi_dung.len() - noi_dung.trim_start_matches([' ', '\t']).len();
        if indent == 0 {
            let t = noi_dung.trim_start();
            trong = t.ends_with(':')
                && (t.starts_with("cmd ") || t.starts_with("impl ") || t.starts_with("type "));
        }
        let mong = if trong && indent != 0 { 4 } else { 0 };
        if indent != mong {
            lech += 1;
        }
    }
    if co_noi_dung {
        Some(lech)
    } else {
        Some(0)
    }
}

fn usage_fmt() -> ExitCode {
    eprintln!(
        "dùng: vis fmt <tệp|thư-mục> | vis fmt --write <tệp|thư-mục…> | vis fmt --check <tệp|thư-mục…> | vis fmt --indent-report <tệp|thư-mục…> | vis lsp --stdio | vis pkg add|install|list"
    );
    ExitCode::from(2)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum FmtMode {
    /// **ST-85**: chỉ **đếm** dòng lệch thụt lề của `.vish` (không sửa, không đỏ).
    IndentReport,
    Stdout,
    Write,
    Check,
}

/// `vis fmt` — chuẩn hoá **thụt lề** (ST-07, VD-39). Thoát 0 (xong / đã đúng), 1 (`--check` có tệp
/// cần sửa), 2 (dùng sai, tệp lỗi cú pháp, `.vish`).
/// **ST-77**: gom `.vis`/`.vish` dưới `goc` theo thứ tự byte (sắp sau khi gom).
fn gom_nguon(goc: &std::path::Path, ra: &mut Vec<std::path::PathBuf>) {
    let Ok(ds) = std::fs::read_dir(goc) else {
        return;
    };
    for e in ds.flatten() {
        let p = e.path();
        if p.is_dir() {
            gom_nguon(&p, ra);
        } else if p
            .extension()
            .map(|x| x == "vis" || x == "vish")
            .unwrap_or(false)
        {
            ra.push(p);
        }
    }
}

pub(crate) fn run_fmt(args: &[String]) -> ExitCode {
    let (mode, files): (FmtMode, &[String]) = match args.first().map(String::as_str) {
        None => return usage_fmt(),
        Some("--write") => (FmtMode::Write, &args[1..]),
        Some("--check") => (FmtMode::Check, &args[1..]),
        Some("--indent-report") => (FmtMode::IndentReport, &args[1..]),
        Some(f) if f.starts_with('-') => {
            eprintln!("vis fmt: không hỗ trợ cờ `{f}`");
            return usage_fmt();
        }
        Some(_) => {
            if args.len() != 1 {
                return usage_fmt();
            }
            (FmtMode::Stdout, args)
        }
    };
    if files.is_empty() {
        return usage_fmt();
    }
    // **ST-77 (VD-108)**: nhận **thư mục** (đệ quy `.vis`/`.vish`) để `vis fmt --check test/`
    // kiểm được toàn bộ fixture; tệp lẻ giữ nguyên hành vi cũ.
    let mut ds: Vec<std::path::PathBuf> = Vec::new();
    for f in files {
        let p = std::path::Path::new(f);
        if p.is_dir() {
            gom_nguon(p, &mut ds);
        } else {
            ds.push(p.to_path_buf());
        }
    }
    // **ST-77**: bỏ hai loại **input cố ý** khỏi kiểm `fmt` (allowlist nhỏ nhất, có lý do):
    // (1) `test/fmt/**` = fixture của chính formatter (sai thụt lề/tab/crlf — đầu vào để sửa);
    // (2) `test/check/**` + `test/doc-examples/**` = fixture **cố ý sai** (ca âm của `check` và
    //     khối tài liệu sai theo thiết kế) — fmt từ chối đúng, không phải lỗi formatter;
    // (3) tệp có sibling `.err` = ca **âm** cố ý sai cú pháp.
    ds.retain(|p| {
        let comps: Vec<_> = p.components().map(|c| c.as_os_str()).collect();
        let la_fixture = comps.windows(3).any(|w| {
            w[0] == "test" && matches!(w[1].to_str(), Some("fmt" | "check" | "doc-examples"))
        });
        !la_fixture && !p.with_extension("err").exists()
    });
    ds.sort();
    if mode == FmtMode::IndentReport {
        let mut tong_dong = 0usize;
        let mut so_tep = 0usize;
        for path in &ds {
            if path.extension().map(|x| x != "vish").unwrap_or(true) {
                continue;
            }
            let Ok(src) = std::fs::read_to_string(path) else {
                continue;
            };
            if let Some(n) = dem_lech_thut_le(&src) {
                if n > 0 {
                    so_tep += 1;
                    tong_dong += n;
                    println!("{}: {n} dòng lệch", path.display());
                }
            }
        }
        println!("indent-report: {so_tep} tệp, {tong_dong} dòng lệch (chuẩn 4 × độ sâu khối)");
        return ExitCode::SUCCESS;
    }
    let mut ra = String::new();
    let mut can_sua = false;
    let mut co_loi = false;
    for path in &ds {
        let src = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("# không đọc được: {e}");
                co_loi = true;
                continue;
            }
        };
        // `.vish` = chế độ lệnh (CmdCST); `.vis` = script. Tệp khác (nếu người dùng chỉ định
        // thẳng) giữ đường script như trước.
        let che_do = if path.extension().map(|x| x == "vish").unwrap_or(false) {
            Mode::Command
        } else {
            Mode::Script
        };
        match vis_fmt::format_mode(&src, che_do) {
            Err(e) => {
                // (d) tệp lỗi: báo lỗi đầu, **không** ghi.
                eprintln!("# {}", dong_chan_doan(e.code, e.line, e.col, &e.message));
                co_loi = true;
            }
            Ok(moi) => match mode {
                // **ST-85**: thụt lề `.vish` **có** ngữ nghĩa ở cấp tệp (đo được) ⇒ **không** sửa;
                // `--check` chỉ **cảnh báo** (không đỏ), `--indent-report` đếm.
                FmtMode::IndentReport => {}
                FmtMode::Check if che_do == Mode::Command => {
                    if let Some(n) = dem_lech_thut_le(&src) {
                        if n > 0 {
                            eprintln!(
                                "# cảnh báo: {}: {n} dòng thụt lề khác chuẩn 4×độ sâu — KHÔNG tự sửa (thụt lề .vish có ngữ nghĩa; xem agent/HOST-BAO-DAM.md)",
                                path.display()
                            );
                        }
                    }
                    if moi != src {
                        can_sua = true;
                        ra.push_str(&format!("{}\n", path.display()));
                    }
                }
                FmtMode::Stdout => ra.push_str(&moi),
                FmtMode::Write => {
                    if moi == src {
                        ra.push_str(&format!("giữ nguyên {}\n", path.display()));
                    } else if let Err(e) = std::fs::write(path, &moi) {
                        eprintln!("# không ghi được {}: {e}", path.display());
                        co_loi = true;
                    } else {
                        can_sua = true;
                        ra.push_str(&format!("đã sửa {}\n", path.display()));
                    }
                }
                FmtMode::Check => {
                    if moi != src {
                        can_sua = true;
                        ra.push_str(&format!("{}\n", path.display()));
                    }
                }
            },
        }
    }
    if !xuat_stdout(&ra) {
        return ExitCode::from(2);
    }
    if co_loi {
        return ExitCode::from(2);
    }
    if mode == FmtMode::Check && can_sua {
        return ExitCode::from(1);
    }
    ExitCode::SUCCESS
}
