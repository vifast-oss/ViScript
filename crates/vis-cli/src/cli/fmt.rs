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
        "dùng: vis fmt <tệp|thư-mục> | vis fmt --write <tệp|thư-mục…> | vis fmt --check <tệp|thư-mục…> | vis fmt --indent-report <tệp|thư-mục…> | vis fmt --stdin [--check] [--mode script|command] | vis lsp --stdio | vis pkg add|install|list"
    );
    eprintln!(
        "  vis fmt --stdin [--check] [--mode script|command]: đọc UTF-8 từ stdin, in bản đã định dạng ra stdout; --mode chọn script (mặc định) hoặc command (buffer .vish); --check không in gì, thoát 0 (đã đúng) / 1 (cần sửa) / 2 (lỗi cú pháp); không nhận tệp"
    );
    ExitCode::from(2)
}

/// **VD-269**: giá trị hợp lệ của `--mode` — đúng hai chế độ parser; `None` nếu lạ.
fn mode_tu_chuoi(s: &str) -> Option<Mode> {
    match s {
        "script" => Some(Mode::Script),
        "command" => Some(Mode::Command),
        _ => None,
    }
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
fn gom_nguon(goc: &std::path::Path, ra: &mut Vec<(std::path::PathBuf, bool)>) {
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
            // `false` = phát hiện khi duyệt thư mục (chịu quy tắc loại trừ).
            ra.push((p, false));
        }
    }
}

/// **VD-267**: `vis fmt --stdin [--check]` — đọc mã UTF-8 từ stdin, **không** đụng đĩa.
/// `--check` chỉ so sánh (không in), thoát 0 (đã đúng) / 1 (cần sửa) / 2 (dùng sai hoặc lỗi cú pháp).
/// Đường tệp biến thể `.vis` mặc định script; stdin không có phần mở rộng nên cũng dùng script.
/// **VD-269**: `--mode script|command` chọn chế độ cho buffer không đuôi (mặc định script).
fn run_fmt_stdin(args: &[String]) -> ExitCode {
    use std::io::Read;
    let mut check = false;
    let mut che_do = Mode::Script;
    let mut da_mode = false;
    let mut i = 0;
    while i < args.len() {
        let a = args[i].as_str();
        match a {
            "--stdin" => {}
            "--check" => check = true,
            "--mode" => {
                let Some(v) = args.get(i + 1) else {
                    eprintln!("vis fmt: --mode thiếu giá trị (script|command)");
                    return usage_fmt();
                };
                if da_mode {
                    eprintln!("vis fmt: --mode lặp lại");
                    return usage_fmt();
                }
                let Some(m) = mode_tu_chuoi(v) else {
                    eprintln!("vis fmt: --mode nhận `script` hoặc `command`, không nhận `{v}`");
                    return usage_fmt();
                };
                che_do = m;
                da_mode = true;
                i += 1; // bỏ qua giá trị vừa đọc
            }
            f if f.starts_with("--mode=") => {
                if da_mode {
                    eprintln!("vis fmt: --mode lặp lại");
                    return usage_fmt();
                }
                let v = &f["--mode=".len()..];
                let Some(m) = mode_tu_chuoi(v) else {
                    eprintln!("vis fmt: --mode nhận `script` hoặc `command`, không nhận `{v}`");
                    return usage_fmt();
                };
                che_do = m;
                da_mode = true;
            }
            "--write" => {
                eprintln!("vis fmt: --stdin không đi với --write (chế độ stdin không ghi đĩa)");
                return usage_fmt();
            }
            "--indent-report" => {
                eprintln!("vis fmt: --stdin không đi với --indent-report (chỉ nhận --check)");
                return usage_fmt();
            }
            f if f.starts_with('-') => {
                eprintln!("vis fmt: không hỗ trợ cờ `{f}` khi dùng --stdin");
                return usage_fmt();
            }
            _ => {
                eprintln!("vis fmt: --stdin không nhận đối số `{a}` (không nhận tệp/thư mục)");
                return usage_fmt();
            }
        }
        i += 1;
    }
    let mut src = String::new();
    if let Err(e) = std::io::stdin().read_to_string(&mut src) {
        eprintln!("vis fmt: không đọc được stdin (cần UTF-8): {e}");
        return ExitCode::from(2);
    }
    // Đầu vào rỗng ⇒ đầu ra rỗng, kể cả `--check` (điểm bất động của chính nó).
    if src.is_empty() {
        return ExitCode::SUCCESS;
    }
    match vis_fmt::format_mode(&src, che_do) {
        Err(e) => {
            eprintln!("# {}", dong_chan_doan(e.code, e.line, e.col, &e.message));
            ExitCode::from(2)
        }
        Ok(moi) if check => {
            if moi == src {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            }
        }
        Ok(moi) => {
            if xuat_stdout(&moi) {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(2)
            }
        }
    }
}

pub(crate) fn run_fmt(args: &[String]) -> ExitCode {
    // **VD-267**: `--stdin` là chế độ riêng — không lẫn với đường tệp/thư mục bên dưới.
    if args.iter().any(|a| a == "--stdin") {
        return run_fmt_stdin(args);
    }
    // **VD-269**: đường tệp tự suy chế độ từ đuôi (`.vish` = command, còn lại = script);
    // `--mode` ở đây là nhầm lẫn ⇒ từ chối rõ thay vì coi là tên tệp.
    if args
        .iter()
        .any(|a| a == "--mode" || a.starts_with("--mode="))
    {
        eprintln!("vis fmt: --mode chỉ dùng với --stdin (đường tệp tự suy chế độ từ .vish/.vis)");
        return usage_fmt();
    }
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
    // `bool` = người dùng **nêu đích danh** (`true`) hay phát hiện khi duyệt thư mục (`false`).
    let mut ds: Vec<(std::path::PathBuf, bool)> = Vec::new();
    for f in files {
        let p = std::path::Path::new(f);
        if p.is_dir() {
            gom_nguon(p, &mut ds);
        } else {
            ds.push((p.to_path_buf(), true));
        }
    }
    // **ST-77**: bỏ hai loại **input cố ý** khỏi kiểm `fmt` (allowlist nhỏ nhất, có lý do):
    // (1) `test/fmt/**` = fixture của chính formatter (sai thụt lề/tab/crlf — đầu vào để sửa);
    // (2) `test/check/**` + `test/doc-examples/**` = fixture **cố ý sai** (ca âm của `check` và
    //     khối tài liệu sai theo thiết kế) — fmt từ chối đúng, không phải lỗi formatter;
    // (3) tệp có sibling `.err` = ca **âm** cố ý sai cú pháp.
    // **ST-240 / VD-242 §4 (nợ #182)**: quy tắc trên **chỉ** áp cho tệp phát hiện khi duyệt thư
    // mục; tệp người dùng nêu tên phải được xử lý và báo lỗi rõ, không bị bỏ qua im lặng.
    ds.retain(|(p, tu_nguon)| {
        if *tu_nguon {
            return true;
        }
        let comps: Vec<_> = p.components().map(|c| c.as_os_str()).collect();
        let la_fixture = comps.windows(3).any(|w| {
            w[0] == "test" && matches!(w[1].to_str(), Some("fmt" | "check" | "doc-examples"))
        });
        !la_fixture && !p.with_extension("err").exists()
    });
    ds.sort_by(|a, b| a.0.cmp(&b.0));
    let ds: Vec<std::path::PathBuf> = ds.into_iter().map(|(p, _)| p).collect();
    // **#183**: chỉ cần nêu tên tệp khi chạy **nhiều** tệp — ca một tệp giữ định dạng cũ
    // (`# VS0201 …`) để không đổi chẩn đoán/golden hiện hành.
    let nhieu_tep = ds.len() > 1;
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
                // **ST-240 / VD-242 §4 (nợ #183)**: chạy nhiều tệp phải nêu tên tệp.
                let tien = if nhieu_tep {
                    format!("{}: ", path.display())
                } else {
                    String::new()
                };
                eprintln!("# {tien}không đọc được: {e}");
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
                // (d) tệp lỗi: báo lỗi đầu, **không** ghi. #183: kèm tên tệp khi đa tệp.
                let tien = if nhieu_tep {
                    format!("{}: ", path.display())
                } else {
                    String::new()
                };
                eprintln!(
                    "# {}{}",
                    tien,
                    dong_chan_doan(e.code, e.line, e.col, &e.message)
                );
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
