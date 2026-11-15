// SPDX-License-Identifier: MIT OR Apache-2.0
//! Hạ phần **closure/chương trình ngoài/render** của `cmd` (ST-106 tách từ `cmd.rs`; thuần
//! refactor, không đổi logic/chẩn đoán).

use super::super::*;
use crate::hir::Lit;

/// Tách **từ** của một stage lệnh, tôn trọng nháy đơn/nháy kép (ST-27).
fn tach_tu(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut trong: Option<char> = None;
    let mut co = false;
    for c in s.chars() {
        match trong {
            Some(q) => {
                if c == q {
                    trong = None;
                } else {
                    cur.push(c);
                }
            }
            None => match c {
                '\'' | '"' => {
                    trong = Some(c);
                    co = true;
                }
                c if c.is_whitespace() => {
                    if co || !cur.is_empty() {
                        out.push(std::mem::take(&mut cur));
                        co = false;
                    }
                }
                _ => cur.push(c),
            },
        }
    }
    if co || !cur.is_empty() {
        out.push(cur);
    }
    out
}

fn lit(kind: LitKind, text: &str) -> HExpr {
    let l = Lit {
        kind,
        text: text.to_string(),
    };
    HExpr::Lit {
        ty: l.ty(),
        lit: l,
        span: Span::new(0, 0),
    }
}
impl<'a> Lowerer<'a> {
    /// Vị trí **bắt đầu** vùng đối số của stage (token đầu sau tên lệnh).
    /// ST-38: hạ **thân closure** `x => <biểu thức>` thành `HExpr::Lambda`.
    ///
    /// Span: parser re-parse thân từ **slice** (span tương đối) ⇒ ở đây lex/parse lại văn bản thân
    /// nhưng **đệm trước bằng đúng số byte** của vị trí thân trong tệp (prefix `let …=` cố định) ⇒
    /// mọi `Span` là **tuyệt đối** (đúng `dòng:cột`, kể cả khi tệp có ký tự nhiều byte trước đó).
    pub(super) fn ha_closure(&mut self, g: &Node, want: &Type) -> Option<HExpr> {
        let (mut pty, rty) = match want {
            Type::Fn(ps, r) if ps.len() == 1 => (ps[0].clone(), (**r).clone()),
            _ => (Type::Error, Type::Error),
        };
        if matches!(&pty, Type::TypeRef { args, .. } if args.is_empty()) {
            pty = Type::Error;
        }
        let mut rty = rty;
        if matches!(&rty, Type::TypeRef { args, .. } if args.is_empty()) {
            rty = Type::Error;
        }
        // Tên tham số: token đầu; có thể viết liền `x=>`.
        let mut toks: Vec<Span> = Vec::new();
        for e in &g.elems {
            if let Elem::Token(i) = e {
                if let Some(t) = self.tok(*i) {
                    toks.push(t.span);
                }
            }
        }
        let (Some(dau), Some(cuoi)) = (toks.first().copied(), toks.last().copied()) else {
            return None;
        };
        let raw = self.text.get(dau.start..dau.end)?.to_string();
        let ten = raw.trim_end_matches('=').to_string();
        // Vị trí thân: sau token `>`.
        let gt = toks.iter().position(|sp| {
            self.text
                .get(sp.start..sp.end)
                .map(|x| x == ">")
                .unwrap_or(false)
        })?;
        let body_start = toks.get(gt + 1).map(|sp| sp.start)?;
        let body_end = cuoi.end;
        if body_end <= body_start {
            return None;
        }
        let body = self.text.get(body_start..body_end)?.to_string();
        // Tên biến tạm **không** trùng người dùng: bộ hạ con dùng bảng ký hiệu **chung**.
        self.tam_n += 1;
        let prefix = format!("let _c{} = ", self.tam_n);
        if body_start < prefix.len() {
            return None;
        }
        let pad = body_start - prefix.len();
        let src = format!("{prefix}{}{body}", " ".repeat(pad));
        let lexed = vis_syntax::lex(&src, vis_syntax::Mode::Script);
        let parsed = vis_syntax::parse(&src, vis_syntax::Mode::Script);
        // ST-46: thân **không** phải biểu thức script (vd `ls ./photos`) ⇒ trả `None` để nơi gọi
        // thử đường **lệnh trong ngoặc** (stage) hoặc báo lỗi rõ (đối số).
        if !parsed.diagnostics.is_empty() {
            return None;
        }
        // Khung riêng cho tham số + bảng **chung** (symbols/types) ⇒ tyck thấy đúng `sym`.
        self.push_frame(FrameKind::Fn, g.span);
        let psym = self.declare(&ten, SymKind::Param, dau, Some(pty.clone()));
        let scope = self.scope.clone();
        let mut sub = super::Lowerer::new(
            &parsed.cst,
            &lexed,
            self.opts.clone(),
            self.symbols,
            self.tbl,
        );
        sub.scope = scope;
        sub.run();
        let out = sub.finish();
        self.pop_frame();
        self.diags.extend(out.diagnostics);
        let init = match out.hir.stmts.first() {
            Some(crate::hir::HStmt::Let { init, .. }) => init.clone(),
            Some(crate::hir::HStmt::Expr(e)) => e.clone(),
            _ => return None,
        };
        Some(HExpr::Lambda {
            params: vec![HParam {
                name: ten,
                span: dau,
                sym: psym,
                ty: pty,
                named: false,
                is_self: false,
                is_var_self: false,
                variadic: false,
                default: None,
            }],
            ret: rty,
            body: crate::hir::HBody::Expr(Box::new(init)),
            span: g.span,
        })
    }
    /// Dựng call **chương trình ngoài** `exec.<tên>` (ST-27): từ thô của stage, bỏ tên lệnh; cờ
    /// `--raw` của Vis ⇒ trả `str` thay vì `list[str]`.
    pub(super) fn ngoai_call(&mut self, ten: &str, node: &Node, stdin: Option<HExpr>) -> HExpr {
        let raw = self
            .text
            .get(node.span.start..node.span.end)
            .unwrap_or("")
            .trim()
            .to_string();
        let mut tu = tach_tu(&raw);
        if !tu.is_empty() {
            tu.remove(0);
        }
        let mut co_raw = false;
        if tu.last().map(|s| s.as_str()) == Some("--raw") {
            tu.pop();
            co_raw = true;
        }
        let items: Vec<HExpr> = tu
            .iter()
            .map(|w| lit(LitKind::Str, &format!("{w:?}")))
            .collect();
        let ds = HExpr::List {
            items,
            elem: Type::Str,
            span: node.span,
        };
        let name = if co_raw {
            format!("exec.raw.{ten}")
        } else {
            format!("exec.{ten}")
        };
        // ST-34: đối số thứ 2 = **stdin** (giá trị từ pipe) — `builtin::call` quy đổi (một quy tắc).
        let vao = stdin.unwrap_or_else(|| {
            let mut rong = lit(LitKind::Str, "\"\"");
            if let HExpr::Lit { span, .. } = &mut rong {
                *span = node.span;
            }
            rong
        });
        HExpr::Call {
            callee: Box::new(HExpr::Var {
                name,
                sym: None,
                span: node.span,
            }),
            args: vec![ds, vao],
            named: Vec::new(),
            span: node.span,
        }
    }

    /// Chạy hạ chế độ lệnh: item (`cmd`/`fn`/`extern`) trước, rồi từng dòng lệnh thành `HStmt`.
    /// Chạy các dòng lệnh của CST chế độ lệnh; **bỏ** mọi node có `span.start < tu` (ST-44: dòng
    /// đã hạ trong **vùng khai báo** ở tiền tố tệp).
    /// `render(<expr>)` (ST-41) — §9.6: giá trị **cuối pipeline** hiển thị qua **`Render`**; `print`
    /// của người dùng **không** đổi (tên builtin riêng, chỉ dùng cho giá trị cuối).
    pub(super) fn boc_render(&mut self, e: HExpr, span: Span) -> HExpr {
        let co_ban_ghi =
            matches!(&self.last_stage_ty, Type::List(el) if matches!(el.as_ref(), Type::Record(_)));
        HExpr::Call {
            callee: Box::new(HExpr::Var {
                name: "render".to_string(),
                sym: None,
                span,
            }),
            args: vec![
                e,
                HExpr::Lit {
                    ty: Type::Bool,
                    lit: Lit {
                        kind: LitKind::Bool,
                        text: co_ban_ghi.to_string(),
                    },
                    span,
                },
            ],
            named: Vec::new(),
            span,
        }
    }
}
