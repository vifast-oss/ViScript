// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-304b** — completion theo hệ kiểu: path, trường record, biến thể enum, cờ `cmd`.
//!
//! `CompletionContext` nhận metadata kiểu (record/enum/cmd/biến) hoặc trích từ CST bằng
//! [`CompletionContext::from_source`]; `complete` chỉ trả ứng viên đúng ngữ cảnh đang gõ.

use std::collections::BTreeMap;
use std::path::PathBuf;

use vis_syntax::{Elem, Kind, Mode, Node, NodeKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CandidateKind {
    Path,
    RecordField,
    EnumVariant,
    CmdFlag,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    pub text: String,
    pub kind: CandidateKind,
}

#[derive(Debug, Clone)]
pub struct CompletionContext {
    pub cwd: PathBuf,
    records: BTreeMap<String, Vec<String>>,
    enums: BTreeMap<String, Vec<String>>,
    cmds: BTreeMap<String, Vec<String>>,
    vars: BTreeMap<String, String>,
}

impl Default for CompletionContext {
    fn default() -> Self {
        Self {
            cwd: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            records: BTreeMap::new(),
            enums: BTreeMap::new(),
            cmds: BTreeMap::new(),
            vars: BTreeMap::new(),
        }
    }
}

impl CompletionContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_cwd(mut self, cwd: impl Into<PathBuf>) -> Self {
        self.cwd = cwd.into();
        self
    }

    pub fn add_record(&mut self, ten: &str, fields: &[&str]) {
        self.records.insert(
            ten.to_string(),
            fields.iter().map(|s| s.to_string()).collect(),
        );
    }

    pub fn add_enum(&mut self, ten: &str, variants: &[&str]) {
        self.enums.insert(
            ten.to_string(),
            variants.iter().map(|s| s.to_string()).collect(),
        );
    }

    pub fn add_cmd(&mut self, ten: &str, flags: &[&str]) {
        self.cmds.insert(
            ten.to_string(),
            flags.iter().map(|s| s.to_string()).collect(),
        );
    }

    pub fn add_var(&mut self, ten: &str, ty: &str) {
        self.vars.insert(ten.to_string(), ty.to_string());
    }

    /// Trích record/enum/cmd/biến từ CST của một nguồn hoàn chỉnh (dùng cho ViShell).
    pub fn from_source(src: &str) -> Self {
        let mut c = Self::default();
        let parsed = vis_syntax::parse(src, Mode::Script);
        c.walk(&parsed.cst.root, &parsed.text);
        c.vars_tu_token(&parsed.cst.tokens);
        c
    }

    fn walk(&mut self, n: &Node, src: &str) {
        match n.kind {
            NodeKind::TypeDecl => {
                if let Some(ten) = first_ident(n, src) {
                    let fields = descendants(n)
                        .filter(|x| x.kind == NodeKind::TypeField)
                        .filter_map(|x| first_ident(x, src))
                        .collect::<Vec<_>>();
                    self.records.insert(ten, fields);
                }
            }
            NodeKind::EnumDecl => {
                if let Some(ten) = first_ident(n, src) {
                    let variants = descendants(n)
                        .filter(|x| x.kind == NodeKind::Variant)
                        .filter_map(|x| first_ident(x, src))
                        .collect::<Vec<_>>();
                    self.enums.insert(ten, variants);
                }
            }
            NodeKind::CmdDecl => {
                if let Some(ten) = first_ident(n, src) {
                    let flags = descendants(n)
                        .filter(|x| x.kind == NodeKind::Param)
                        .filter_map(|p| flag_cua(p, src))
                        .collect::<Vec<_>>();
                    self.cmds.insert(ten, flags);
                }
            }
            NodeKind::Param => {
                let ws: Vec<&str> = src[n.span.start..n.span.end.min(src.len())]
                    .split_whitespace()
                    .collect();
                if ws.len() >= 2 {
                    let ten = clean_ident(ws[0]);
                    let ty = clean_ident(ws[1]);
                    if !ten.is_empty() && !ty.is_empty() {
                        self.vars.insert(ten, ty);
                    }
                }
            }
            _ => {}
        }
        for e in &n.elems {
            if let Elem::Node(c) = e {
                self.walk(c, src);
            }
        }
    }

    fn vars_tu_token(&mut self, ts: &[vis_syntax::Token]) {
        for i in 0..ts.len().saturating_sub(4) {
            if ts[i].kind == Kind::Kw
                && matches!(ts[i].text.as_str(), "let" | "var")
                && ts[i + 1].kind == Kind::Ident
                && ts[i + 2].text == ":"
                && ts[i + 3].kind == Kind::Ident
            {
                self.vars
                    .insert(ts[i + 1].text.clone(), ts[i + 3].text.clone());
            }
        }
        // Tham số hàm/closure: `x: Type` trong `Param`.
        // (Duyệt token toàn cục đã đủ cho `let`; param lấy từ CST khi caller gọi from_source.)
    }

    /// Ứng viên tại `cursor` (byte, trong `src`).
    pub fn complete(&self, src: &str, cursor: usize) -> Vec<Candidate> {
        let mut cur = cursor.min(src.len());
        while !src.is_char_boundary(cur) {
            cur -= 1;
        }
        let start = src[..cur].rfind('\n').map(|i| i + 1).unwrap_or(0);
        let line = &src[start..cur];
        let word = line
            .rsplit(|c: char| c.is_whitespace() || "()[]{}:,;".contains(c))
            .next()
            .unwrap_or("");
        let mut out = Vec::new();

        // Trường record / biến thể enum: `x.<prefix>` hoặc `Enum.<prefix>`.
        if let Some(dot) = line.rfind('.') {
            let before = &line[..dot];
            let ten = before
                .rsplit(|c: char| c.is_whitespace() || "()[]{}:,;".contains(c))
                .next()
                .unwrap_or("");
            let prefix = &line[dot + 1..];
            let ty = self
                .vars
                .get(ten)
                .cloned()
                .or_else(|| self.enums.contains_key(ten).then(|| ten.to_string()));
            if let Some(ty) = ty {
                if let Some(fs) = self.records.get(&ty) {
                    for f in fs {
                        if f.starts_with(prefix) {
                            out.push(Candidate {
                                text: f.clone(),
                                kind: CandidateKind::RecordField,
                            });
                        }
                    }
                } else if let Some(vs) = self.enums.get(&ty) {
                    for v in vs {
                        if v.starts_with(prefix) {
                            out.push(Candidate {
                                text: v.clone(),
                                kind: CandidateKind::EnumVariant,
                            });
                        }
                    }
                }
            }
            // `Color.<prefix>` trực tiếp.
            if let Some(vs) = self.enums.get(ten) {
                for v in vs {
                    if v.starts_with(prefix) {
                        out.push(Candidate {
                            text: v.clone(),
                            kind: CandidateKind::EnumVariant,
                        });
                    }
                }
            }
        }

        // Cờ `cmd`: dòng bắt đầu bằng tên lệnh đã biết.
        let cmd = line.split_whitespace().next().unwrap_or("");
        if let Some(flags) = self.cmds.get(cmd) {
            let p = if word.starts_with('-') { word } else { "" };
            for f in flags {
                if f.starts_with(p) {
                    out.push(Candidate {
                        text: f.clone(),
                        kind: CandidateKind::CmdFlag,
                    });
                }
            }
        }

        // Path: chỉ khi có `/` hoặc bắt đầu `.`/`..` để không nhiễu completion thường.
        if word.contains('/') || word.starts_with('.') {
            out.extend(self.paths(word));
        }

        out.sort_by(|a, b| a.text.cmp(&b.text));
        out.dedup_by(|a, b| a.text == b.text);
        out
    }

    fn paths(&self, word: &str) -> Vec<Candidate> {
        let (dir, base) = match word.rsplit_once('/') {
            Some((d, b)) => (d, b),
            None => ("", word),
        };
        let p = if dir.is_empty() {
            self.cwd.clone()
        } else if PathBuf::from(dir).is_absolute() {
            PathBuf::from(dir)
        } else {
            self.cwd.join(dir)
        };
        let Ok(rd) = std::fs::read_dir(p) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            if !name.starts_with(base) {
                continue;
            }
            let text = if dir.is_empty() {
                if e.path().is_dir() {
                    format!("{name}/")
                } else {
                    name
                }
            } else if e.path().is_dir() {
                format!("{dir}/{name}/")
            } else {
                format!("{dir}/{name}")
            };
            out.push(Candidate {
                text,
                kind: CandidateKind::Path,
            });
        }
        out
    }
}

fn descendants(n: &Node) -> impl Iterator<Item = &Node> {
    let mut out = Vec::new();
    fn go<'a>(n: &'a Node, out: &mut Vec<&'a Node>) {
        for e in &n.elems {
            if let Elem::Node(c) = e {
                out.push(c);
                go(c, out);
            }
        }
    }
    go(n, &mut out);
    out.into_iter()
}

fn flag_cua(n: &Node, src: &str) -> Option<String> {
    let text = src.get(n.span.start..n.span.end)?;
    text.split_whitespace()
        .map(|w| w.trim_matches(|c: char| !(c.is_alphanumeric() || c == '-' || c == '_')))
        .find(|w| w.starts_with("--"))
        .map(str::to_string)
}

fn first_ident(n: &Node, src: &str) -> Option<String> {
    let text = src.get(n.span.start..n.span.end)?;
    let ws: Vec<&str> = text.split_whitespace().collect();
    let raw = match n.kind {
        NodeKind::TypeDecl | NodeKind::EnumDecl => ws.get(1)?,
        NodeKind::CmdDecl => {
            let i = ws.iter().position(|w| *w == "cmd")?;
            ws.get(i + 1)?
        }
        _ => ws.first()?,
    };
    Some(clean_ident(raw.split('(').next().unwrap_or(raw)))
}

fn clean_ident(s: &str) -> String {
    s.trim_matches(|c: char| !(c.is_alphanumeric() || c == '_' || c == '-'))
        .to_string()
}

/// Tiện ích: completion với context cho trước.
pub fn complete(src: &str, cursor: usize, ctx: &CompletionContext) -> Vec<Candidate> {
    ctx.complete(src, cursor)
}
