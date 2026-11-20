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
        "dùng: vis fmt <tệp|thư-mục> | vis fmt --write <tệp|thư-mục…> | vis fmt --check [--json|--list|--diff|--stat] <tệp|thư-mục…> | vis fmt --indent-report <tệp|thư-mục…> | vis fmt --stdin [--check [--json|--list|--diff|--stat]] [--mode script|command] | vis lsp --stdio | vis pkg add|install|list"
    );
    eprintln!(
        "  vis fmt --stdin [--check [--json|--list|--diff|--stat]] [--mode script|command]: đọc UTF-8 từ stdin, in bản đã định dạng ra stdout; --mode chọn script (mặc định) hoặc command (buffer .vish); --check không in gì, thoát 0 (đã đúng) / 1 (cần sửa) / 2 (lỗi cú pháp); không nhận tệp"
    );
    eprintln!(
        "  --json: chỉ đi với --check (tệp/thư mục hoặc --stdin); in một object JSON nén trên stdout: {{\"ok\":bool,\"files\":[{{\"path\":…,\"formatted\":bool,\"error\":null|{{\"code\":\"VSxxxx\",\"line\":n,\"col\":n,\"message\":…}}}}]}}; dòng/cột 1-based; thoát 0 (mọi tệp đã đúng) / 1 (cần sửa) / 2 (lỗi đọc/cú pháp hoặc dùng sai)"
    );
    eprintln!(
        "  --list: chỉ đi với --check (tệp/thư mục hoặc --stdin --check); in đúng một đường dẫn mỗi dòng cho tệp cần sửa, không in gì khác (thứ tự đối số, trong thư mục duyệt đệ quy đã sắp); --stdin --check --list in `-` khi stdin cần sửa; không đi với --json/--write/--indent-report; thoát 0 (không tệp nào cần sửa) / 1 (có tệp cần sửa) / 2 (lỗi đọc/cú pháp hoặc dùng sai)\n  --diff: chỉ đi với --check (tệp/thư mục hoặc --stdin --check); in unified diff một hunk mỗi tệp cần sửa (---/+++/@@); không đi với --json/--list/--write/--indent-report; thoát 0/1/2 như --check"
    );
    eprintln!(
        "  --stat: chỉ đi với --check (tệp/thư mục hoặc --stdin --check); in đúng một dòng stdout `fmt-check: <N> tệp, <M> cần sửa, <K> lỗi` (N tệp đã kiểm, M tệp cần sửa, K lỗi đọc/cú pháp), không in gì khác; lỗi chi tiết ra stderr; không đi với --json/--list/--diff/--write/--indent-report; thoát 0 (M=K=0) / 1 (M>0, K=0) / 2 (K>0 hoặc dùng sai)"
    );
    ExitCode::from(2)
}

/// **VD-270**: một tệp trong `vis fmt --check --json` — `path`, `formatted`, `error`.
/// `None` ⇒ `"error":null`; `Some` ⇒ object lỗi với `code`/`line`/`col` **1-based**.
fn tep_json(path: &str, formatted: bool, error: Option<(&str, u32, u32, &str)>) -> String {
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
fn diff_dong(src: &str, moi: &str) -> Option<String> {
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
fn xuat_fmt_diff(ds: &[std::path::PathBuf]) -> ExitCode {
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
fn xuat_fmt_json(entries: &[String], ok: bool, ma_thoat: u8) -> ExitCode {
    let body = format!("{{\"ok\":{ok},\"files\":[{}]}}\n", entries.join(","));
    if !xuat_stdout(&body) {
        return ExitCode::from(2);
    }
    ExitCode::from(ma_thoat)
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
/// **VD-270**: `--check --list` chỉ in đường dẫn tệp cần sửa — một dòng mỗi tệp, không gì khác.
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

/// **ST-77 / VD-270**: gom nguồn **theo từng đối số** (giữ thứ tự người dùng nêu); thư mục được
/// duyệt đệ quy như [`gom_nguon`], tệp lẻ là một nhóm. Người gọi tự sắp trong nhóm nếu cần.
fn gom_theo_doi_so(files: &[String]) -> Vec<Vec<(std::path::PathBuf, bool)>> {
    files
        .iter()
        .map(|f| {
            let p = std::path::Path::new(f);
            let mut ds = Vec::new();
            if p.is_dir() {
                gom_nguon(p, &mut ds);
            } else {
                ds.push((p.to_path_buf(), true));
            }
            ds
        })
        .collect()
}

/// **ST-77**: input **cố ý** không thuộc kiểm `fmt` (allowlist nhỏ nhất, có lý do):
/// (1) `test/fmt/**` = fixture của chính formatter (sai thụt lề/tab/crlf — đầu vào để sửa);
/// (2) `test/check/**` + `test/doc-examples/**` = fixture **cố ý sai** (ca âm của `check` và
///     khối tài liệu sai theo thiết kế) — fmt từ chối đúng, không phải lỗi formatter;
/// (3) tệp có sibling `.err` = ca **âm** cố ý sai cú pháp.
/// **ST-240 / VD-242 §4 (nợ #182)**: quy tắc trên **chỉ** áp cho tệp phát hiện khi duyệt thư
/// mục; tệp người dùng nêu tên phải được xử lý và báo lỗi rõ, không bị bỏ qua im lặng.
fn la_input_co_y(p: &std::path::Path) -> bool {
    let comps: Vec<_> = p.components().map(|c| c.as_os_str()).collect();
    let la_fixture = comps
        .windows(3)
        .any(|w| w[0] == "test" && matches!(w[1].to_str(), Some("fmt" | "check" | "doc-examples")));
    !la_fixture && !p.with_extension("err").exists()
}

/// **VD-267**: `vis fmt --stdin [--check]` — đọc mã UTF-8 từ stdin, **không** đụng đĩa.
/// `--check` chỉ so sánh (không in), thoát 0 (đã đúng) / 1 (cần sửa) / 2 (dùng sai hoặc lỗi cú pháp).
/// Đường tệp biến thể `.vis` mặc định script; stdin không có phần mở rộng nên cũng dùng script.
/// **VD-269**: `--mode script|command` chọn chế độ cho buffer không đuôi (mặc định script).
fn run_fmt_stdin(args: &[String]) -> ExitCode {
    use std::io::Read;
    let mut check = false;
    let mut json_ra = false;
    // **VD-270**: `--list` — biến thể máy đọc của `--check`; chỉ in `-` khi stdin cần sửa.
    let mut list_ra = false;
    // **VD-270**: `--diff` — unified diff của stdin khi `--check`.
    let mut diff_ra = false;
    // **VD-270**: `--stat` — một dòng tổng hợp của stdin khi `--check`.
    let mut stat_ra = false;
    let mut che_do = Mode::Script;
    let mut da_mode = false;
    let mut i = 0;
    while i < args.len() {
        let a = args[i].as_str();
        match a {
            "--stdin" => {}
            "--check" => check = true,
            // **VD-270**: `--json`/`--list` chỉ có nghĩa khi `--check`; kiểm sau khi đọc hết cờ.
            "--json" => json_ra = true,
            "--list" => list_ra = true,
            "--diff" => diff_ra = true,
            "--stat" => stat_ra = true,
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
    // **VD-270**: `--stat` là đầu ra tổng hợp riêng — không đi với các đầu ra máy/ghi khác và
    // chỉ có nghĩa khi `--check`; kiểm trước khi chạm stdin.
    if stat_ra {
        if json_ra || list_ra || diff_ra {
            eprintln!("vis fmt: --stat không đi với --json/--list/--diff (chọn một đầu ra)");
            return usage_fmt();
        }
        if !check {
            eprintln!("vis fmt: --stat chỉ dùng với --check");
            return usage_fmt();
        }
    }
    // **VD-270**: `--list` + `--json` là hai đầu ra máy khác nhau ⇒ từ chối rõ (JSON đã đủ).
    if list_ra && json_ra {
        eprintln!("vis fmt: --list không đi với --json (JSON đã là đầu ra máy)");
        return usage_fmt();
    }
    // **VD-270**: `--json` không đi với `--stdin` trần — phải có `--check`; câu chữ giữ cụm
    // "không hỗ trợ cờ" để ca cũ (VD-267) vẫn đúng lý do.
    if json_ra && !check {
        eprintln!(
            "vis fmt: --json chỉ dùng với --check (không hỗ trợ cờ `--json` khi thiếu --check)"
        );
        return usage_fmt();
    }
    if list_ra && !check {
        eprintln!("vis fmt: --list chỉ dùng với --check");
        return usage_fmt();
    }
    if diff_ra && (json_ra || list_ra) {
        eprintln!("vis fmt: --diff không đi với --json/--list");
        return usage_fmt();
    }
    if diff_ra && !check {
        eprintln!("vis fmt: --diff chỉ dùng với --check");
        return usage_fmt();
    }
    let mut src = String::new();
    if let Err(e) = std::io::stdin().read_to_string(&mut src) {
        let msg = format!("không đọc được stdin (cần UTF-8): {e}");
        if stat_ra {
            eprintln!("vis fmt: {msg}");
            return xuat_fmt_stat_stdin(false, true);
        }
        if json_ra {
            let entry = tep_json("-", false, Some(("VS0201", 1, 1, &msg)));
            return xuat_fmt_json(&[entry], false, 2);
        }
        eprintln!("vis fmt: {msg}");
        return ExitCode::from(2);
    }
    // Đầu vào rỗng ⇒ đầu ra rỗng, kể cả `--check` (điểm bất động của chính nó).
    // **VD-270**: `--stat` vẫn tính stdin rỗng là **1 tệp**, 0 cần sửa.
    if src.is_empty() {
        if stat_ra {
            return xuat_fmt_stat_stdin(false, false);
        }
        if json_ra {
            return xuat_fmt_json(&[tep_json("-", true, None)], true, 0);
        }
        return ExitCode::SUCCESS;
    }
    match vis_fmt::format_mode(&src, che_do) {
        Err(e) => {
            if stat_ra {
                eprintln!("# {}", dong_chan_doan(e.code, e.line, e.col, &e.message));
                return xuat_fmt_stat_stdin(false, true);
            }
            if json_ra {
                let entry = tep_json("-", false, Some((e.code, e.line, e.col, &e.message)));
                return xuat_fmt_json(&[entry], false, 2);
            }
            eprintln!("# {}", dong_chan_doan(e.code, e.line, e.col, &e.message));
            ExitCode::from(2)
        }
        Ok(moi) if check => {
            // **VD-270**: `--stat` — một dòng tổng hợp, không in đường dẫn/nội dung.
            if stat_ra {
                return xuat_fmt_stat_stdin(moi != src, false);
            }
            // **VD-270**: `--diff` — in unified diff của `-` khi cần sửa.
            if diff_ra {
                if let Some(h) = diff_dong(&src, &moi) {
                    let body = format!("--- -\n+++ -\n{h}");
                    return if xuat_stdout(&body) {
                        ExitCode::from(1)
                    } else {
                        ExitCode::from(2)
                    };
                }
                return ExitCode::SUCCESS;
            }
            // **VD-270**: `--list` chỉ in đường dẫn (`-`) khi cần sửa; đã đúng ⇒ im lặng.
            if list_ra {
                if moi == src {
                    return ExitCode::SUCCESS;
                }
                return if xuat_stdout("-\n") {
                    ExitCode::from(1)
                } else {
                    ExitCode::from(2)
                };
            }
            if json_ra {
                let formatted = moi == src;
                let entry = tep_json("-", formatted, None);
                return xuat_fmt_json(&[entry], formatted, if formatted { 0 } else { 1 });
            }
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

/// **VD-270**: kết quả đọc + định dạng **một** tệp, dùng chung cho đường văn bản và `--json`
/// (không lặp lại logic chọn chế độ/so sánh).
enum KetQuaDoc {
    /// Không đọc được tệp (I/O) — exit 2, JSON dùng `VS0201` như `vis check`.
    DocLoi(String),
    /// Formatter từ chối vì cú pháp/từ vựng — exit 2.
    ParseLoi(vis_fmt::FmtError),
    /// Đọc + định dạng xong; `formatted` = `moi == src`.
    Xong {
        src: String,
        moi: String,
        che_do: Mode,
    },
}

/// Đọc **một** tệp và gọi đúng formatter hiện có: `.vish` = chế độ lệnh, còn lại = script.
fn doc_va_dinh_dang(path: &std::path::Path) -> KetQuaDoc {
    let src = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => return KetQuaDoc::DocLoi(e.to_string()),
    };
    let che_do = if path.extension().map(|x| x == "vish").unwrap_or(false) {
        Mode::Command
    } else {
        Mode::Script
    };
    match vis_fmt::format_mode(&src, che_do) {
        Ok(moi) => KetQuaDoc::Xong { src, moi, che_do },
        Err(e) => KetQuaDoc::ParseLoi(e),
    }
}

/// **VD-270**: `vis fmt --check --json` trên danh sách đã gom/sắp — một entry mỗi tệp theo đúng
/// thứ tự đó; thoát 2 nếu có lỗi đọc/cú pháp, ngược lại 1 nếu còn tệp cần sửa, 0 nếu sạch.
fn xuat_fmt_json_tep(ds: &[std::path::PathBuf]) -> ExitCode {
    let mut entries: Vec<String> = Vec::with_capacity(ds.len());
    let mut co_loi = false;
    let mut can_sua = false;
    for path in ds {
        let hien = path.display().to_string();
        match doc_va_dinh_dang(path) {
            KetQuaDoc::Xong { src, moi, .. } => {
                let formatted = moi == src;
                if !formatted {
                    can_sua = true;
                }
                entries.push(tep_json(&hien, formatted, None));
            }
            KetQuaDoc::ParseLoi(e) => {
                co_loi = true;
                entries.push(tep_json(
                    &hien,
                    false,
                    Some((e.code, e.line, e.col, &e.message)),
                ));
            }
            KetQuaDoc::DocLoi(msg) => {
                co_loi = true;
                let msg = format!("không đọc được: {msg}");
                entries.push(tep_json(&hien, false, Some(("VS0201", 1, 1, &msg))));
            }
        }
    }
    let ma = if co_loi {
        2
    } else if can_sua {
        1
    } else {
        0
    };
    xuat_fmt_json(&entries, ma == 0, ma)
}

/// **VD-270**: `vis fmt --check --list` trên danh sách đã gom/sắp — in **đúng một** đường dẫn
/// mỗi dòng cho tệp cần sửa, không in gì khác; lỗi đọc/cú pháp ra stderr (luôn kèm tên tệp) và
/// làm mã thoát thành 2. Thứ tự do nơi gom quyết định: đối số theo thứ tự người dùng, trong mỗi
/// thư mục đã sắp đệ quy.
fn xuat_fmt_list(ds: &[std::path::PathBuf]) -> ExitCode {
    let mut ra = String::new();
    let mut co_loi = false;
    let mut can_sua = false;
    for path in ds {
        match doc_va_dinh_dang(path) {
            KetQuaDoc::Xong { src, moi, .. } => {
                if moi != src {
                    can_sua = true;
                    ra.push_str(&format!("{}\n", path.display()));
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
    let ma = if co_loi {
        2
    } else if can_sua {
        1
    } else {
        0
    };
    ExitCode::from(ma)
}

/// **VD-270**: dòng tổng hợp cố định của `--check --stat` — một dòng stdout, không gì khác.
fn dong_fmt_stat(so_tep: usize, can_sua: usize, loi: usize) -> String {
    format!("fmt-check: {so_tep} tệp, {can_sua} cần sửa, {loi} lỗi\n")
}

/// **VD-270**: `vis fmt --check --stat` trên danh sách đã gom/sắp — đếm **N** tệp đã kiểm,
/// **M** tệp cần sửa, **K** lỗi đọc/cú pháp. Chỉ in đúng một dòng tổng hợp ra stdout; chi tiết
/// lỗi ra stderr (luôn kèm tên tệp). Exit 0 (M=K=0) / 1 (M>0, K=0) / 2 (K>0).
fn xuat_fmt_stat(ds: &[std::path::PathBuf]) -> ExitCode {
    let mut can_sua = 0usize;
    let mut loi = 0usize;
    for path in ds {
        match doc_va_dinh_dang(path) {
            KetQuaDoc::Xong { src, moi, .. } => {
                if moi != src {
                    can_sua += 1;
                }
            }
            KetQuaDoc::ParseLoi(e) => {
                loi += 1;
                eprintln!(
                    "# {}: {}",
                    path.display(),
                    dong_chan_doan(e.code, e.line, e.col, &e.message)
                );
            }
            KetQuaDoc::DocLoi(e) => {
                loi += 1;
                eprintln!("# {}: không đọc được: {e}", path.display());
            }
        }
    }
    let ma = if loi > 0 {
        2
    } else if can_sua > 0 {
        1
    } else {
        0
    };
    if !xuat_stdout(&dong_fmt_stat(ds.len(), can_sua, loi)) {
        return ExitCode::from(2);
    }
    ExitCode::from(ma)
}

/// **VD-270**: biến thể `--stdin --check --stat` — N luôn = 1 (kể cả stdin rỗng); M/K là 0|1.
fn xuat_fmt_stat_stdin(can_sua: bool, loi: bool) -> ExitCode {
    let m = if can_sua { 1 } else { 0 };
    let k = if loi { 1 } else { 0 };
    let ma = if loi {
        2
    } else if can_sua {
        1
    } else {
        0
    };
    if !xuat_stdout(&dong_fmt_stat(1, m, k)) {
        return ExitCode::from(2);
    }
    ExitCode::from(ma)
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
    // **VD-270**: `--json`/`--list` chỉ hợp lệ khi `--check`; lọc cờ trước khi chọn chế độ để
    // nhận cả `--check --json …` lẫn `--json --check …`. Không đổi cách nhận diện các chế độ cũ.
    let json_ra = args.iter().any(|a| a == "--json");
    let list_ra = args.iter().any(|a| a == "--list");
    // **VD-270**: `--diff` — unified diff của tệp cần sửa; chỉ đi với `--check`.
    let diff_ra = args.iter().any(|a| a == "--diff");
    // **VD-270**: `--stat` — một dòng tổng hợp cho CI; chỉ đi với `--check`.
    let stat_ra = args.iter().any(|a| a == "--stat");
    // **VD-270**: `--stat` là đầu ra tổng hợp riêng — không đi với các đầu ra máy/ghi khác.
    if stat_ra {
        if json_ra || list_ra || diff_ra {
            eprintln!("vis fmt: --stat không đi với --json/--list/--diff (chọn một đầu ra)");
            return usage_fmt();
        }
        if args
            .iter()
            .any(|a| a == "--write" || a == "--indent-report")
        {
            eprintln!(
                "vis fmt: --stat chỉ dùng với --check (không đi với --write/--indent-report)"
            );
            return usage_fmt();
        }
        if !args.iter().any(|a| a == "--check") {
            eprintln!("vis fmt: --stat chỉ dùng với --check");
            return usage_fmt();
        }
    }
    // **VD-270**: `--list` là đầu ra máy dạng danh sách — không đi với JSON (đã đủ cho máy),
    // không đi với chế độ ghi/báo cáo, và chỉ có nghĩa khi `--check`.
    if list_ra {
        if json_ra {
            eprintln!("vis fmt: --list không đi với --json (JSON đã là đầu ra máy)");
            return usage_fmt();
        }
        if args
            .iter()
            .any(|a| a == "--write" || a == "--indent-report")
        {
            eprintln!(
                "vis fmt: --list chỉ dùng với --check (không đi với --write/--indent-report)"
            );
            return usage_fmt();
        }
        if !args.iter().any(|a| a == "--check") {
            eprintln!("vis fmt: --list chỉ dùng với --check");
            return usage_fmt();
        }
    }
    if json_ra {
        if args
            .iter()
            .any(|a| a == "--write" || a == "--indent-report")
        {
            eprintln!(
                "vis fmt: --json chỉ dùng với --check (không đi với --write/--indent-report)"
            );
            return usage_fmt();
        }
        if !args.iter().any(|a| a == "--check") {
            eprintln!("vis fmt: --json chỉ dùng với --check");
            return usage_fmt();
        }
    }
    if diff_ra {
        if json_ra || list_ra {
            eprintln!("vis fmt: --diff không đi với --json/--list");
            return usage_fmt();
        }
        if args
            .iter()
            .any(|a| a == "--write" || a == "--indent-report")
        {
            eprintln!(
                "vis fmt: --diff chỉ dùng với --check (không đi với --write/--indent-report)"
            );
            return usage_fmt();
        }
        if !args.iter().any(|a| a == "--check") {
            eprintln!("vis fmt: --diff chỉ dùng với --check");
            return usage_fmt();
        }
    }
    let loc: Vec<String> = if json_ra || list_ra || diff_ra || stat_ra {
        args.iter()
            .filter(|a| {
                a.as_str() != "--json"
                    && a.as_str() != "--list"
                    && a.as_str() != "--diff"
                    && a.as_str() != "--stat"
            })
            .cloned()
            .collect()
    } else {
        Vec::new()
    };
    let args: &[String] = if json_ra || list_ra || diff_ra || stat_ra {
        &loc
    } else {
        args
    };
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
    // `--json`/`--list` đứng sau tên tệp (không phải đầu) cũng là dùng sai — không rơi vào chế
    // độ stdout.
    if json_ra && mode != FmtMode::Check {
        eprintln!("vis fmt: --json chỉ dùng với --check");
        return usage_fmt();
    }
    if list_ra && mode != FmtMode::Check {
        eprintln!("vis fmt: --list chỉ dùng với --check");
        return usage_fmt();
    }
    if diff_ra && mode != FmtMode::Check {
        eprintln!("vis fmt: --diff chỉ dùng với --check");
        return usage_fmt();
    }
    if stat_ra && mode != FmtMode::Check {
        eprintln!("vis fmt: --stat chỉ dùng với --check");
        return usage_fmt();
    }
    if files.is_empty() {
        return usage_fmt();
    }
    // **ST-77 (VD-108)**: nhận **thư mục** (đệ quy `.vis`/`.vish`) để `vis fmt --check test/`
    // kiểm được toàn bộ fixture; tệp lẻ giữ nguyên hành vi cũ.
    // **VD-270**: `--json`/`--list` giữ **thứ tự đối số**, chỉ sắp trong từng thư mục; đường văn
    // bản giữ nguyên sắp xếp toàn cục như trước (byte-identical).
    // **VD-270**: `--stat` — gom như các đầu ra máy (thứ tự đối số, sắp trong từng thư mục;
    // cùng quy tắc loại trừ) rồi in đúng một dòng tổng hợp.
    if stat_ra {
        let mut ds: Vec<std::path::PathBuf> = Vec::new();
        for mut nhom in gom_theo_doi_so(files) {
            nhom.retain(|(p, tu_nguon)| *tu_nguon || la_input_co_y(p));
            nhom.sort_by(|a, b| a.0.cmp(&b.0));
            ds.extend(nhom.into_iter().map(|(p, _)| p));
        }
        return xuat_fmt_stat(&ds);
    }
    if diff_ra {
        let mut ds: Vec<std::path::PathBuf> = Vec::new();
        for mut nhom in gom_theo_doi_so(files) {
            nhom.retain(|(p, tu_nguon)| *tu_nguon || la_input_co_y(p));
            nhom.sort_by(|a, b| a.0.cmp(&b.0));
            ds.extend(nhom.into_iter().map(|(p, _)| p));
        }
        return xuat_fmt_diff(&ds);
    }
    if list_ra {
        let mut ds: Vec<std::path::PathBuf> = Vec::new();
        for mut nhom in gom_theo_doi_so(files) {
            nhom.retain(|(p, tu_nguon)| *tu_nguon || la_input_co_y(p));
            nhom.sort_by(|a, b| a.0.cmp(&b.0));
            ds.extend(nhom.into_iter().map(|(p, _)| p));
        }
        return xuat_fmt_list(&ds);
    }
    if json_ra {
        let mut ds: Vec<std::path::PathBuf> = Vec::new();
        for mut nhom in gom_theo_doi_so(files) {
            nhom.retain(|(p, tu_nguon)| *tu_nguon || la_input_co_y(p));
            nhom.sort_by(|a, b| a.0.cmp(&b.0));
            ds.extend(nhom.into_iter().map(|(p, _)| p));
        }
        return xuat_fmt_json_tep(&ds);
    }
    let mut ds: Vec<(std::path::PathBuf, bool)> =
        gom_theo_doi_so(files).into_iter().flatten().collect();
    ds.retain(|(p, tu_nguon)| *tu_nguon || la_input_co_y(p));
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
        // **VD-270**: đọc + chọn chế độ + định dạng nằm trong một helper dùng chung với `--json`.
        let (src, moi, che_do) = match doc_va_dinh_dang(path) {
            KetQuaDoc::DocLoi(e) => {
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
            KetQuaDoc::ParseLoi(e) => {
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
                continue;
            }
            KetQuaDoc::Xong { src, moi, che_do } => (src, moi, che_do),
        };
        match mode {
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
