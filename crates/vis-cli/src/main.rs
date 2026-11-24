// SPDX-License-Identifier: MIT OR Apache-2.0
//! `vis` — lệnh dòng ViScript. Vòng 1 (ST-02b) mới có `explain`.

use std::process::ExitCode;

mod cli;
// **ST-310b (VD-285)**: `vis repl` dùng chung thân với thư viện (test PTY giả/viterm).
mod repl;
// **ST-300b (VD-283)**: host native (quyền/FS/net/TLS/SQLite) sang `vis-host-native`; re-export
// module tại gốc để mọi `crate::quyen::…`/`crate::net::…`/`crate::db::…` cũ không đổi.
pub(crate) use vis_host_native::{db, fsmock, ngoai, quyen};
// **ST-296b**: `modules`/`cache`/`json_esc` (+ audit test) chuyển sang `vis-loader`; LSP sang
// `vis-lsp`. Re-export giữ nguyên mọi đường gọi `crate::modules::…`/`cache::…` trong crate này.
pub(crate) use cli::run_cmd::{dong_chan_doan, xuat_stdout};
pub(crate) use vis_loader::{cache, json_esc, modules};
pub(crate) use vis_pkg::pkg;
use vis_syntax::Mode;
mod vistest;

use vis_diag::{all_codes, lookup_str, severity_label, Code, Lang, Severity};

fn usage() -> ExitCode {
    eprintln!(
        "dùng: vis --version | vis -V | vis explain [--json] [--lang vi|en|ja] <VSxxxx> | vis explain --list [--json] [--lang vi|en|ja] | vis run [--tap] [--stdin] [--time] [--strict-caps] <tệp.vis> [-- args…] | vis test [--vm] [--json] [--timeout <ms>] [--filter <substr>] [--fail-fast] [--list] [--repeat <N>] [--shuffle [--seed <u64>]] [--shard <I>/<N>] [--jobs <N>] [--slowest <N>] [thư mục|tệp] | vis check [--syntax] [--no-extern] [--json] <tệp…> | vis check --stdin [--syntax] [--no-extern] [--json] | vis fmt <tệp> | vis fmt --write <tệp…> | vis fmt --check [--json|--list|--diff|--stat] <tệp…> | vis fmt --indent-report <tệp…> | vis fmt --stdin [--check [--json|--list|--diff|--stat]] [--mode script|command] | vis lsp --stdio | vis pkg add|install|list | vis doctor [--json] | vis version [--json] | vis new <dir> [--template bin|lib] | vis doc <path> [--format md|json] | vis repl [--format text|viterm] | vis plugin check|run <plugin.wasm> …"
    );
    eprintln!(
        "  vis test --timeout <ms>: áp mọi ca — `test fn` và fixture `.out/.err`; quá hạn ⇒ kill + `not ok` (`quá hạn <ms> ms`), mặc định không giới hạn"
    );
    ExitCode::from(2)
}

/// **ST-78/ST-83**: trợ giúp riêng của `vis explain` — đúng số khối/số mã (đếm động) và **theo
/// `--lang`** (vi/en/ja); không viết cứng số.
fn usage_explain(lang: Lang) -> ExitCode {
    let khoi = vis_diag::BLOCKS_RESERVED.len();
    let ma = vis_diag::all_codes().count();
    match lang {
        Lang::Vi => {
            eprintln!(
                "dùng: vis explain [--json] <VSxxxx> [--lang vi|en|ja] | vis explain --list [--json] [--lang vi|en|ja]"
            );
            eprintln!(
                "sổ mã: {khoi} khối, {ma} mã — `vis explain --list` in toàn bộ (mã + câu ngắn); `--json` in một dòng máy đọc"
            );
        }
        Lang::En => {
            eprintln!(
                "usage: vis explain [--json] <VSxxxx> [--lang vi|en|ja] | vis explain --list [--json] [--lang vi|en|ja]"
            );
            eprintln!(
                "code book: {khoi} blocks, {ma} codes — `vis explain --list` prints all (code + short text); `--json` prints one machine-readable line"
            );
        }
        Lang::Ja => {
            eprintln!(
                "使い方: vis explain [--json] <VSxxxx> [--lang vi|en|ja] | vis explain --list [--json] [--lang vi|en|ja]"
            );
            eprintln!(
                "コード帳: {khoi} ブロック、{ma} コード — `vis explain --list` で全件表示（コード + 短い説明）、`--json` は機械可読な1行"
            );
        }
    }
    ExitCode::from(2)
}

/// **VD-270** — mức của mã ở dạng **ổn định cho máy đọc** (không phụ thuộc `--lang`).
fn severity_json(sev: Severity) -> &'static str {
    match sev {
        Severity::Error => "error",
        Severity::Warning => "warning",
    }
}

/// **VD-270** — một mã trong `explain --list --json`: chỉ `code`/`severity`/`title`.
fn explain_json_ngan(c: &Code, lang: Lang) -> String {
    format!(
        "{{\"code\":\"{}\",\"severity\":\"{}\",\"title\":\"{}\"}}",
        c.short,
        severity_json(c.severity),
        json_esc::esc(c.message(lang))
    )
}

/// **VD-270** — một mã cho `explain --json`: `title` là câu ngắn của sổ, `body` là **đúng dòng
/// giải thích** của chế độ người đọc (không có `\n`). Sổ hiện chưa có mô tả dài riêng, nên
/// `body` là dạng đầy đủ của cùng thông điệp; khi sổ có thêm phần thân, chỉ đổi hàm này.
fn explain_json_mot(c: &Code, lang: Lang) -> String {
    let than = format!(
        "{} — {}: {}",
        c.short,
        severity_label(c.severity, lang),
        c.message(lang)
    );
    format!(
        "{{\"code\":\"{}\",\"severity\":\"{}\",\"title\":\"{}\",\"body\":\"{}\"}}",
        c.short,
        severity_json(c.severity),
        json_esc::esc(c.message(lang)),
        json_esc::esc(&than)
    )
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        // Hardening (A0): cờ phiên bản cấp cao nhất; không đụng `vis run --version`/lệnh con.
        Some("--version") | Some("-V") => {
            println!("vis {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        // ST-23: quản lý cache bytecode.
        Some("cache") => match args.get(1).map(String::as_str) {
            Some("--clear") => {
                let n = cache::xoa();
                eprintln!("vis cache: đã xoá {n} tệp");
                ExitCode::from(0)
            }
            Some("--path") | None => {
                match cache::thu_muc() {
                    Some(p) => println!("{}", p.display()),
                    None => eprintln!("vis cache: không xác định được thư mục cache"),
                }
                ExitCode::from(0)
            }
            Some(khac) => {
                eprintln!("vis cache: cờ lạ `{khac}` (dùng --clear | --path)");
                ExitCode::from(2)
            }
        },
        Some("explain") => {
            let mut lang = Lang::Vi;
            let mut list = false;
            // **VD-270**: đầu ra máy đọc — một dòng JSON nén; không trộn văn bản người đọc.
            let mut json_ra = false;
            let mut ma: Option<String> = None;
            let mut i = 1;
            while i < args.len() {
                match args[i].as_str() {
                    "--list" => list = true,
                    "--json" => json_ra = true,
                    "--lang" => match args.get(i + 1) {
                        Some(v) => match Lang::parse(v) {
                            Some(l) => {
                                lang = l;
                                i += 1; // bỏ qua giá trị ngôn ngữ
                            }
                            None => {
                                eprintln!("vis explain: ngôn ngữ không hỗ trợ `{v}` (vi|en|ja)");
                                return ExitCode::from(2);
                            }
                        },
                        None => {
                            eprintln!("vis explain: `--lang` cần giá trị vi|en|ja");
                            return ExitCode::from(2);
                        }
                    },
                    a if a.starts_with('-') => {
                        eprintln!("vis explain: không hỗ trợ cờ `{a}`");
                        return usage_explain(lang);
                    }
                    a => ma = Some(a.to_string()),
                }
                i += 1;
            }
            if list {
                // `explain --list` phủ **cả ba** sổ đã cấp (01xx + 02xx + 03xx, VD-44 #7).
                if json_ra {
                    let mut ra = String::from("{\"codes\":[");
                    for (n, c) in all_codes().enumerate() {
                        if n > 0 {
                            ra.push(',');
                        }
                        ra.push_str(&explain_json_ngan(c, lang));
                    }
                    ra.push_str("]}");
                    println!("{ra}");
                } else {
                    for c in all_codes() {
                        println!(
                            "{}  {:<8} {}",
                            c.short,
                            severity_label(c.severity, lang),
                            c.message(lang)
                        );
                    }
                }
                return ExitCode::SUCCESS;
            }
            match ma {
                None => usage_explain(lang),
                Some(code) => match lookup_str(&code) {
                    Some(c) => {
                        if json_ra {
                            println!("{}", explain_json_mot(c, lang));
                        } else {
                            println!(
                                "{} — {}: {}",
                                c.short,
                                severity_label(c.severity, lang),
                                c.message(lang)
                            );
                        }
                        ExitCode::SUCCESS
                    }
                    None => {
                        // VD-20: exit 1 dành cho lỗi từ main; mã sai là **lỗi dùng** ⇒ 2.
                        eprintln!("vis: không có mã {code} trong sổ (SPEC-lex §7)");
                        ExitCode::from(2)
                    }
                },
            }
        }
        Some("run") => cli::run_cmd::run_run(&args[1..]),
        Some("check") => cli::check::run_check(&args[1..]),
        // VD-270: thông tin chẩn đoán tất định (không đọc môi trường/tệp/mạng).
        Some("doctor") => cli::doctor::run_doctor(&args[1..]),
        Some("version") => cli::version::run(&args[1..]),
        Some("new") => cli::new::run(&args[1..]),
        Some("doc") => cli::doc::run(&args[1..]),
        Some("repl") => repl::run(&args[1..]),
        // ST-307 (VD-278/VD-283): plugin động WASI/WASM, tắt cứng ở --strict-caps/--sandbox.
        Some("plugin") => cli::plugin::run_plugin(&args[1..]),
        // ST-30: chạy bộ ca `test/sem/**` in TAP.
        Some("test") => vistest::run(&args[1..]),
        // ST-67 (VD-102): server LSP tối thiểu trên stdio (khung + JSON-RPC).
        Some("lsp") => vis_lsp::run(&args[1..]),
        // ST-70 (VD-101): registry **local** cho gói ViScript.
        Some("pkg") => {
            // **ST-312d (VD-289)**: `vis-pkg` không còn phụ thuộc `vis-host-native`;
            // CLI tiêm host mạng thật tại biên.
            let mut net = vis_host_native::net::Net::moi(false);
            pkg::run(&args[1..], &mut net)
        }
        Some("fmt") => cli::fmt::run_fmt(&args[1..]),
        _ => usage(),
    }
}
