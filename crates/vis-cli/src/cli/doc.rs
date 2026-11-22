// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-303 (VD-283)** — `vis doc <path> [--format md|json]`: trích tài liệu `##` gắn với
//! item cấp tệp bằng **CST** của `vis-syntax` (không regex trên văn bản thô).

use std::path::Path;
use std::process::ExitCode;

use vis_syntax::{Elem, Kind, Mode, NodeKind};

fn usage() -> ExitCode {
    eprintln!("dùng: vis doc <path> [--format md|json]");
    ExitCode::from(2)
}

fn la_item(k: NodeKind) -> bool {
    matches!(
        k,
        NodeKind::UseDecl
            | NodeKind::FnDecl
            | NodeKind::CmdDecl
            | NodeKind::ExternDecl
            | NodeKind::TypeDecl
            | NodeKind::EnumDecl
            | NodeKind::AliasDecl
            | NodeKind::NewtypeDecl
            | NodeKind::TraitDecl
            | NodeKind::ImplDecl
            | NodeKind::ConstDecl
            | NodeKind::TestDecl
    )
}

struct Muc {
    line: u32,
    signature: String,
    docs: Vec<String>,
}

fn mot_dong(s: &str) -> String {
    let s = s.trim();
    if s.chars().count() <= 160 {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(159).collect();
        out.push('…');
        out
    }
}

fn esc(s: &str) -> String {
    crate::json_esc::esc(s)
}

fn in_json(path: &str, items: &[Muc]) -> String {
    let mut s = format!("{{\"path\":\"{}\",\"items\":[", esc(path));
    for (i, m) in items.iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        s.push_str(&format!(
            "{{\"line\":{},\"signature\":\"{}\",\"doc\":[",
            m.line,
            esc(&m.signature)
        ));
        for (j, d) in m.docs.iter().enumerate() {
            if j > 0 {
                s.push(',');
            }
            s.push('"');
            s.push_str(&esc(d));
            s.push('"');
        }
        s.push_str("]}");
    }
    s.push_str("]}\n");
    s
}

pub(crate) fn run(args: &[String]) -> ExitCode {
    let mut path: Option<String> = None;
    let mut format = "md".to_string();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--format" => {
                let Some(v) = args.get(i + 1) else {
                    return usage();
                };
                format = v.clone();
                i += 2;
            }
            a if a.starts_with('-') => {
                eprintln!("vis doc: không hỗ trợ cờ `{a}`");
                return usage();
            }
            a => {
                if path.is_some() {
                    eprintln!("vis doc: chỉ nhận một tệp");
                    return usage();
                }
                path = Some(a.to_string());
                i += 1;
            }
        }
    }
    if format != "md" && format != "json" {
        eprintln!("vis doc: `--format` phải là `md` hoặc `json`");
        return usage();
    }
    let Some(path) = path else {
        return usage();
    };
    let Ok(text) = std::fs::read_to_string(&path) else {
        eprintln!("vis doc: không đọc được `{path}`");
        return ExitCode::from(2);
    };
    let mode = if Path::new(&path)
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e == "vish")
    {
        Mode::Command
    } else {
        Mode::Script
    };
    let parsed = vis_syntax::parse(&text, mode);
    if !parsed.diagnostics.is_empty() {
        for d in &parsed.diagnostics {
            eprintln!("{}:{}:{}: {}", d.line, d.col, d.code.short, d.message);
        }
        eprintln!("vis doc: `{path}` không parse được");
        return ExitCode::from(2);
    }
    let mut ds: Vec<(u32, u32, String)> = Vec::new();
    for e in &parsed.cst.root.elems {
        let Elem::Node(n) = e else { continue };
        if !la_item(n.kind) {
            continue;
        }
        let sig = mot_dong(
            parsed
                .text
                .get(n.span.start..n.span.end)
                .unwrap_or("")
                .lines()
                .next()
                .unwrap_or(""),
        );
        ds.push((n.span.start as u32, n.span.end as u32, sig));
    }
    // `##` là token riêng trong CST; gắn vào item cấp tệp **kế tiếp** theo span.
    let mut items: Vec<Muc> = ds
        .iter()
        .map(|(start, _end, sig)| Muc {
            line: 1 + parsed.text[..*start as usize].matches('\n').count() as u32,
            signature: sig.clone(),
            docs: Vec::new(),
        })
        .collect();
    for t in &parsed.cst.tokens {
        if t.kind != Kind::DocComment {
            continue;
        }
        let Some(k) = ds.iter().position(|(s, _, _)| *s > t.span.end as u32) else {
            continue;
        };
        let d = t.text.trim_start_matches("##").trim();
        if !d.is_empty() {
            items[k].docs.push(d.to_string());
        }
    }
    if format == "json" {
        print!("{}", in_json(&path, &items));
    } else {
        println!("# {path}\n");
        if items.iter().all(|m| m.docs.is_empty()) {
            println!("(không có tài liệu `##`)");
        }
        for m in items.iter().filter(|m| !m.docs.is_empty()) {
            println!("## {}\n", m.signature);
            for d in &m.docs {
                println!("{d}\n");
            }
        }
    }
    ExitCode::SUCCESS
}
