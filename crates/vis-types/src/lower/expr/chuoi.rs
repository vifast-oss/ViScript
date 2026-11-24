// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-296c** — literal chuỗi/nội suy/escape của `lower::expr`.

use super::*;

impl<'a> Lowerer<'a> {
    pub(super) fn lower_literal(&mut self, node: &Node) -> HExpr {
        let span = node.span;
        let Some(t) = node.tokens(&self.cst.tokens).next() else {
            return HExpr::Error { span };
        };
        if t.kind == Kind::Kw {
            return match t.text.as_str() {
                // **R2 (VD-178)**: từ khoá `none` đã **bỏ** — dùng `None` (`VS0203`).
                // Hạ vẫn tạo Opt(None) để biên dịch tiếp (chẩn đoán là cổng chặn).
                "none" => {
                    self.diag(
                        &vis_diag::SYNTAX_CODES[2],
                        "`none` đã bỏ — dùng `None` (từ khoá đã bỏ, dùng biến thể `None` của `Option`)",
                        span,
                    );
                    HExpr::Opt {
                        value: None,
                        ty: Type::Opt(Box::new(Type::Error)),
                        span,
                    }
                }
                "true" | "false" => HExpr::Lit {
                    lit: Lit {
                        kind: LitKind::Bool,
                        text: t.text.clone(),
                    },
                    ty: Type::Bool,
                    span,
                },
                _ => HExpr::Error { span },
            };
        }
        let Some(kind) = lit_kind(t.kind) else {
            return HExpr::Error { span };
        };
        // ST-14 (VD-47a): chuỗi có `{expr}` ⇒ hạ **từng mảnh** thành `HExpr::Interp`.
        // **R6a (VD-178)**: `p"…"` cũng **có** nội suy (nhưng **không** escape).
        if (kind == LitKind::Str || kind == LitKind::Path)
            && t.kind != Kind::RawStr
            && t.kind != Kind::RawMulti
            && t.text.contains('{')
        {
            let (dau, cuoi) = match t.kind {
                Kind::MultiStr => (3, 3),
                Kind::Path => (2, 1),
                _ => (1, 1),
            };
            if t.text.len() >= dau + cuoi {
                let trong = &t.text[dau..t.text.len() - cuoi];
                return self.chuoi_noi_suy(trong, dau, t.kind == Kind::Path, span);
            }
        }
        let lit = Lit {
            kind,
            text: t.text.clone(),
        };
        let ty = lit.ty();
        HExpr::Lit { lit, ty, span }
    }

    /// Hạ chuỗi **có nội suy** (ST-14): tách mảnh chữ + `{expr}`, hạ từng `{expr}` **bằng chính
    /// `Lowerer` này** (chia sẻ `symbols`/`scope` ⇒ tên trong nội suy giải đúng ngữ cảnh), với CST
    /// của mảnh được **đệm khoảng trắng** tới đúng **offset tuyệt đối** trong tệp để `span` (và do đó
    /// `dòng:cột`) trỏ **đúng chỗ**.
    pub(super) fn chuoi_noi_suy(
        &mut self,
        trong: &str,
        ngoac: usize,
        no_escape: bool,
        span: Span,
    ) -> HExpr {
        let mut parts: Vec<InterpPart> = Vec::new();
        let mut chu = String::new();
        let mut i = 0usize;
        let mut hong = false;
        while i < trong.len() {
            let c = trong[i..].chars().next().unwrap();
            let n = c.len_utf8();
            let sau = &trong[i + n..];
            match c {
                '{' if sau.starts_with('{') => {
                    chu.push('{');
                    i += n + 1;
                }
                '}' if sau.starts_with('}') => {
                    chu.push('}');
                    i += n + 1;
                }
                '{' => {
                    // quét tới `}` cân bằng (cho phép `{ f({a: 1}) }` ở mức ngoặc)
                    let start = i + n;
                    let mut depth = 1usize;
                    let mut j = start;
                    let mut end = None;
                    while j < trong.len() {
                        let d = trong[j..].chars().next().unwrap();
                        if d == '{' {
                            depth += 1;
                        } else if d == '}' {
                            depth -= 1;
                            if depth == 0 {
                                end = Some(j);
                                break;
                            }
                        }
                        j += d.len_utf8();
                    }
                    let Some(end) = end else {
                        self.diag(
                            &vis_diag::SYNTAX_CODES[0],
                            "nội suy `{` không có `}` đóng (§A.6)",
                            Span::new_file(
                                self.file,
                                span.start + ngoac + i,
                                span.start + ngoac + trong.len(),
                            ),
                        );
                        hong = true;
                        break;
                    };
                    if !chu.is_empty() {
                        parts.push(InterpPart::Str(std::mem::take(&mut chu)));
                    }
                    let off = span.start + ngoac + start;
                    // **R6c (VD-178)**: tách `:spec` ở độ sâu ngoặc 0 (`.N` = N chữ số thập phân).
                    let manh = &trong[start..end];
                    let (src, spec) = tach_spec(manh);
                    match self.ha_manh(src, off) {
                        Some(e) => match spec {
                            Some(sp) => parts.push(InterpPart::ExprSpec {
                                expr: Box::new(e),
                                spec: sp,
                            }),
                            None => parts.push(InterpPart::Expr(Box::new(e))),
                        },
                        None => {
                            hong = true;
                            break;
                        }
                    }
                    i = end + 1;
                }
                '\\' if !no_escape => match giai_escape(sau) {
                    Ok((c2, dung)) => {
                        chu.push(c2);
                        i += n + dung;
                    }
                    Err(_) => {
                        let file = self.file;
                        self.diag(
                                &vis_diag::CODES[1],
                                "escape chuỗi không hợp lệ (`\\u{XXXXXX}`?) (§3) — `vis explain VS0102`".to_string(),
                                Span::new_file(
                                    file,
                                    span.start + ngoac + i,
                                    span.start + ngoac + (i + n),
                                ),
                            );
                        hong = true;
                        i += n + 1;
                    }
                },
                _ => {
                    chu.push(c);
                    i += n;
                }
            }
        }
        if !chu.is_empty() {
            parts.push(InterpPart::Str(chu));
        }
        if hong && parts.is_empty() {
            return HExpr::Error { span };
        }
        HExpr::Interp { parts, span }
    }

    /// Hạ **một** mảnh `{…}` tại offset tuyệt đối `off` — đệm khoảng trắng để `span` tuyệt đối.
    pub(super) fn ha_manh(&mut self, manh: &str, off: usize) -> Option<HExpr> {
        // Đệm **đúng `off` byte** nhưng chèn `\n` ở byte cuối: giữ offset tuyệt đối mà **không**
        // sinh `INDENT` giả ở đầu tệp (lexer coi dòng 1 là dòng trắng, mảnh bắt đầu ở cột 1).
        let dem = " ".repeat(off.saturating_sub(1));
        let src = if off == 0 {
            manh.to_string()
        } else {
            format!("{dem}\n{manh}")
        };
        let parsed = parse(&src, Mode::Script);
        if !parsed.diagnostics.is_empty() {
            for d in &parsed.diagnostics {
                self.diag(
                    d.code,
                    d.message.clone(),
                    Span::new_file(self.file, off, off + manh.len()),
                );
            }
            return None;
        }
        let cst: &'static Cst = Box::leak(Box::new(parsed.cst));
        let text: &'static str = Box::leak(src.into_boxed_str());
        let _lex: &'static LexOutput = Box::leak(Box::new(lex(text, Mode::Script)));
        let cu_cst = self.cst;
        let cu_text = self.text;
        self.cst = cst;
        self.text = text;
        let mut ket_qua = None;
        let con: Vec<Node> = self.cst.root.children().cloned().collect();
        // Mảnh phải là **đúng một** câu lệnh biểu thức.
        match con.as_slice() {
            [child] => match self.lower_stmt(child) {
                Some(vis_hir::hir::HStmt::Expr(expr)) => ket_qua = Some(expr),
                _ => self.diag(
                    &vis_diag::SYNTAX_CODES[0],
                    "nội suy `{…}` chỉ nhận **một biểu thức**",
                    Span::new_file(self.file, off, off + manh.len()),
                ),
            },
            _ => self.diag(
                &vis_diag::SYNTAX_CODES[0],
                "nội suy `{…}` chỉ nhận **một biểu thức**",
                Span::new_file(self.file, off, off + manh.len()),
            ),
        }
        self.cst = cu_cst;
        self.text = cu_text;
        ket_qua
    }
}
