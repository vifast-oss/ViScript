// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-318c** — văn bản stage, kiểu literal và chuyển đổi từ thô sang literal (nguyên văn từ
//! `lower/cmd.rs`; 0 đổi hành vi). Tách ra để `cmd.rs` giữ R1.

use super::super::*;
use vis_hir::hir::Lit;

impl<'a> Lowerer<'a> {
    pub(super) fn nhom_khong_ngoac(&self, g: &Node) -> bool {
        match g.elems.first() {
            Some(Elem::Token(i)) => self.tok(*i).map(|t| t.text != "(").unwrap_or(false),
            _ => false,
        }
    }

    pub(crate) fn vach_text(&self, g: &Node) -> Option<String> {
        let first = g.elems.first().and_then(|e| match e {
            Elem::Token(i) => self.tok(*i).map(|t| (t.span, t.text.clone())),
            _ => None,
        })?;
        let last = g.elems.last().and_then(|e| match e {
            Elem::Token(i) => self.tok(*i).map(|t| (t.span, t.text.clone())),
            _ => None,
        })?;
        // **ST-51**: nhóm **không ngoặc** (stage đầu là biểu thức đóng) ⇒ lấy **nguyên span**.
        if first.1 == "(" && last.1 == ")" {
            self.text
                .get(first.0.end..last.0.start)
                .map(|s| s.to_string())
        } else {
            self.text
                .get(first.0.start..last.0.end)
                .map(|s| s.to_string())
        }
    }

    pub(super) fn lower_vach(&mut self, g: &Node) -> Option<HExpr> {
        let txt = self.vach_text(g)?;
        let bat_dau = match g.elems.first() {
            Some(Elem::Token(i)) => self
                .tok(*i)
                .map(|t| {
                    if t.text == "(" {
                        t.span.end
                    } else {
                        // ST-51: nhóm-không-ngoặc ⇒ biểu thức bắt đầu ngay tại token đầu.
                        t.span.start
                    }
                })
                .unwrap_or(0),
            _ => 0,
        };
        let sau_mo = &self.text[bat_dau.min(self.text.len())..];
        let lead = sau_mo
            .chars()
            .take_while(|c| *c == ' ' || *c == '\t')
            .count();
        self.lower_bieu_thuc_tai(&txt, bat_dau + lead)
    }

    /// Hạ **văn bản biểu thức** ở vị trí tuyệt đối `bat_dau` (ST-46) — dùng chung cho `( … )`
    /// (stage, đối số, lồng nhau).
    pub(super) fn lower_bieu_thuc_tai(&mut self, txt: &str, bat_dau: usize) -> Option<HExpr> {
        // ST-49: newline trong `( … )` **không** kết thúc biểu thức — đổi thành **dấu cách**
        // (1 byte → 1 byte) để span tuyệt đối không đổi.
        let txt = txt.replace(['\n', '\r'], " ");
        // ST-46: **đệm byte** để span của biểu thức là **tuyệt đối** trong tệp (chẩn đoán lúc chạy
        // báo đúng `dòng:cột`); đệm bằng **dấu cách** trong cùng dòng ⇒ không sinh `Indent`/`Dedent`.
        let dau = "let __vach = ";
        let pad = bat_dau.saturating_sub(dau.len());
        // Bọc `let` để **giữ** biểu thức (câu lệnh trần bị bỏ ở cấp tệp) — không chạy tyck ở đây.
        // Luôn bọc `let` (kể cả nhóm ở đầu tệp): ở đầu câu lệnh parser hiểu `fn`/`if`/`match`/…
        // là khai báo/câu lệnh chứ không phải biểu thức, và lambda thân `:` không được đặt trong `( … )`.
        let src = format!("{dau}{}{}", " ".repeat(pad), txt.trim());
        // **ST-122 (VD-127)**: `src` chỉ **đệm** cho span tuyệt đối; token phải mang `file` của tệp
        // đang hạ (`lex`/`parse` cũ cho `file = 0` ⇒ mọi node của `( biểu thức )` ở **module** bị quy
        // sang tệp chính).
        let mut lexed = vis_syntax::lex_file(&src, vis_syntax::Mode::Script, self.file);
        // **T-01**: biểu thức bắt đầu **trước** byte `dau.len()` (nhóm ở đầu tệp: `( [1, "a"] )`)
        // ⇒ `pad` bão hoà về 0, biểu thức nằm ở byte `dau.len()` thay vì `bat_dau` và mọi span lệch
        // phải `lech` byte (chẩn đoán rơi về cuối dòng / dòng sau). Khi đó **dời span token** về vị
        // trí tuyệt đối: token của biểu thức lùi `lech` byte, token của `let __vach =` thành rỗng ở
        // byte 0; văn bản đi kèm được dựng lại (đệm dấu cách + biểu thức) để `text_of(span)` vẫn
        // đọc đúng. `bat_dau == 0` (nhóm-không-ngoặc ST-51 ở byte 0) giữ nguyên đường cũ.
        let lech = dau.len().saturating_sub(bat_dau);
        if lech > 0 && bat_dau > 0 {
            for t in &mut lexed.tokens {
                if t.span.start >= dau.len() {
                    t.span.start -= lech;
                    t.span.end -= lech;
                } else {
                    t.span.start = 0;
                    t.span.end = 0;
                }
            }
            let than = lexed.text.get(dau.len()..).unwrap_or("").to_string();
            lexed.text = format!("{}{}", " ".repeat(bat_dau), than);
        }
        let parsed = vis_syntax::parse_voi_file(
            &lexed.text,
            lexed.tokens.clone(),
            vis_syntax::Mode::Script,
            self.file,
        );
        // ST-46: thân **không** phải biểu thức script (vd `ls ./photos`) ⇒ trả `None` để nơi gọi
        // thử đường **lệnh trong ngoặc** (stage) hoặc báo lỗi rõ (đối số).
        if !lexed.diagnostics.is_empty() || !parsed.diagnostics.is_empty() {
            return None;
        }
        // ST-38: hạ **vào bảng chung** (ST-25) — nếu hạ bằng bảng mới, `RecId` của bản ghi ẩn danh
        // trong nhóm không tồn tại ở bảng chính ⇒ tyck báo sai kiểu (`record#0` vs `record#1`).
        let h = crate::lower_into(
            self.symbols,
            self.tbl,
            &parsed.cst,
            &lexed,
            self.opts.clone(),
        );
        match h.hir.stmts.first().cloned()? {
            HStmt::Let { init, .. } => Some(init),
            HStmt::Expr(e) => Some(e),
            _ => None,
        }
    }

    /// ST-37: kiểu **literal** suy từ văn bản (`[1, 2]` ⇒ `list[int]`, `"x"` ⇒ `str`, …).
    /// Chỉ để chọn khe ống / buộc `T`; `{error}` nếu không chắc (tyck/lúc chạy kiểm sau).
    pub(crate) fn kieu_text(&self, raw: &str) -> Type {
        let t = raw.trim();
        if let Some(tron) = t.strip_prefix('[').and_then(|x| x.strip_suffix(']')) {
            let dau = super::super::chieu::tach_phay(tron)
                .first()
                .copied()
                .unwrap_or("")
                .trim();
            return Type::List(Box::new(if dau.is_empty() {
                Type::Error
            } else {
                self.kieu_text(dau)
            }));
        }
        if t.starts_with('"') {
            return Type::Str;
        }
        if t == "true" || t == "false" {
            return Type::Bool;
        }
        if t.parse::<i64>().is_ok() {
            return Type::Int;
        }
        if t.parse::<f64>().is_ok() {
            return Type::Float;
        }
        // ST-40a: literal có **đơn vị** — `5mb`/`2kb` (size), `3ms`/`1s`/`2m` (duration), `50pct` (percent).
        let so = t.trim_end_matches(|c: char| c.is_ascii_alphabetic() || c == '%');
        if let Ok(_n) = so.parse::<f64>() {
            if t.ends_with('%') || t.ends_with("pct") {
                return Type::Percent;
            }
            let duoi = &t[so.len()..];
            match duoi {
                "b" | "kb" | "mb" | "gb" | "tb" => return Type::Size,
                "ms" | "s" | "m" | "h" | "d" => return Type::Duration,
                _ => {}
            }
        }
        // ST-40a: literal bản ghi `{…}` khớp **đúng bộ trường** của một bản ghi prelude ⇒ kiểu đó
        // (cho `it` có kiểu thật khi pipeline bắt đầu bằng list literal bản ghi).
        if t.starts_with('{') && t.ends_with('}') {
            let trong = &t[1..t.len() - 1];
            let mut ten_truong: Vec<String> = Vec::new();
            let mut duoc = true;
            for f in super::super::chieu::tach_phay(trong) {
                match f.split_once(':') {
                    Some((k, _)) if !k.trim().is_empty() => {
                        ten_truong.push(k.trim().trim_matches('"').to_string())
                    }
                    _ => {
                        duoc = false;
                        break;
                    }
                }
            }
            if duoc {
                for (_, id) in &self.prelude_recs {
                    if let Some(rec) = self.tbl.rec(*id) {
                        let mut a: Vec<String> =
                            rec.fields.iter().map(|f| f.name.clone()).collect();
                        let mut b = ten_truong.clone();
                        a.sort();
                        b.sort();
                        if a == b {
                            return Type::Record(*id);
                        }
                    }
                }
            }
        }
        // ST-40a: `ten(...)` gọi lệnh `cmd` đã khai ⇒ kiểu ra của chữ ký (generic chưa suy ⇒ `{error}`).
        if let Some(goi) = t.strip_suffix(')') {
            if let Some((ten, _)) = goi.split_once('(') {
                let ten = ten.trim();
                if !ten.is_empty() {
                    if let Some(sig) = self.cmd_sigs.iter().find(|s| s.name == ten) {
                        return if sig.generics.is_empty() {
                            sig.ret.clone()
                        } else {
                            Type::Error
                        };
                    }
                }
            }
        }
        // ST-40a: tên biến/tham số **đã khai** với kiểu tường minh.
        if let Some(sym) = self.symbols.syms.iter().find(|x| x.name == t) {
            if let Some(ty) = &sym.ty {
                return ty.clone();
            }
        }
        Type::Error
    }
}

impl<'a> Lowerer<'a> {
    pub(super) fn chuyen_doi(&mut self, node: &Node, ty: &Type) -> Option<HExpr> {
        let raw = self.text_of(node.span).to_string();
        self.chuyen_doi_text(&raw, ty, node.span)
    }

    /// Như [`Self::chuyen_doi`] nhưng nhận **văn bản** (dùng cho token rời của stage).
    pub(super) fn chuyen_doi_text(&mut self, raw: &str, ty: &Type, span: Span) -> Option<HExpr> {
        let text = raw.trim();
        let (kind, ty_lit) = match ty {
            Type::Str => {
                // **VD-241 § fix(Windows/CRLF #psvn-A3)**: thoát các meta-char (`\`, `\"`, `\\`)
                // để **văn bản chưa escape** trở thành literal chuỗi hợp lệ — tránh `\t` trong path bị
                // `.vish` unescape cắn (vd. `ls C:\temp`).
                let esc = text.replace('\\', "\\\\").replace('"', "\\\"");
                let s = if text.starts_with('"') && text.ends_with('"') && text.len() >= 2 {
                    text.to_string()
                } else {
                    format!("\"{esc}\"")
                };
                return Some(HExpr::Lit {
                    lit: Lit {
                        kind: LitKind::Str,
                        text: s,
                    },
                    ty: Type::Str,
                    span,
                });
            }
            Type::Int => (LitKind::Int, Type::Int),
            Type::Float => (LitKind::Float, Type::Float),
            Type::Size => (LitKind::Size, Type::Size),
            Type::Bool => (LitKind::Bool, Type::Bool),
            _ => return None,
        };
        let hople = match kind {
            LitKind::Int => text.parse::<i64>().is_ok(),
            LitKind::Float => text.parse::<f64>().is_ok(),
            LitKind::Bool => text == "true" || text == "false",
            LitKind::Size => vis_syntax::lex::lit::size_parts(text).is_some(),
            _ => false,
        };
        if !hople {
            return None;
        }
        Some(HExpr::Lit {
            lit: Lit {
                kind,
                text: text.to_string(),
            },
            ty: ty_lit,
            span,
        })
    }
}
