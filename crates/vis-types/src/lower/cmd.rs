// SPDX-License-Identifier: MIT OR Apache-2.0
//! `cmd_lower` — hạ **CST chế độ lệnh** (`.vish`, khối `cmd { … }`) xuống **cùng HIR** của script
//! (ST-18, VD-22 pha 2). Nguyên tắc: `cmd` là **hàm** (`FnKind::Cmd`) nên mọi thứ chạy qua máy interp
//! sẵn có; ở đây chỉ thêm **bảng chữ ký** + **khe ống** + 4 mã `06xx`.

use super::*;
use crate::cmdsig::{hop_nhat_cmd, CmdSig};
use crate::hir::Lit;
use vis_diag::{COMMAND_CODES, TYPE_CODES};

mod doc;
mod hop;

/// Tên kiểu ngắn để đưa vào thông điệp `VS0602`/`VS0603`.
/// **ST-122 (VD-127)**: giá trị `default` của builtin được dựng **một lần** trong
/// [`crate::cmdsig::BuiltinSigs::mac_dinh`] với `Span::new(0, 0)` (`file = 0`). Khi **chép** nó vào
/// HIR của một tệp, node đó là **node tổng hợp của tệp đang hạ** ⇒ phải mang `file` của tệp ấy.
/// Giữ nguyên `start/end` (giá trị mặc định **không có** vị trí nguồn — chỉ `file` là thông tin đúng).
fn mac_dinh_theo_tep(e: HExpr, file: u16) -> HExpr {
    match e {
        HExpr::Lit { lit, ty, span } => HExpr::Lit {
            lit,
            ty,
            span: span.with_file(file),
        },
        // Các dạng khác chưa có trong bảng builtin; nếu thêm, **audit** (`audit_span`) sẽ bắt.
        other => other,
    }
}

impl<'a> Lowerer<'a> {
    /// ST-37: hạ **nội dung** stage `( … )` — parser chỉ giữ token nên hạ lại văn bản trong ngoặc
    /// ở chế độ script. Chỉ dùng cho **biểu thức thuần** (literal/phép toán); biến trong nhóm chưa hỗ trợ.
    /// **ST-51**: nhóm **không có ngoặc** (stage đầu là biểu thức đóng).
    pub(super) fn nhom_khong_ngoac(&self, g: &Node) -> bool {
        match g.elems.first() {
            Some(Elem::Token(i)) => self.tok(*i).map(|t| t.text != "(").unwrap_or(false),
            _ => false,
        }
    }

    pub(super) fn vach_text(&self, g: &Node) -> Option<String> {
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
    pub(super) fn kieu_text(&self, raw: &str) -> Type {
        let t = raw.trim();
        if let Some(tron) = t.strip_prefix('[').and_then(|x| x.strip_suffix(']')) {
            let dau = super::chieu::tach_phay(tron)
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
            for f in super::chieu::tach_phay(trong) {
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
pub(super) fn ten_kieu(t: &Type) -> String {
    match t {
        Type::Int => "int".into(),
        Type::Float => "float".into(),
        Type::Size => "size".into(),
        Type::Duration => "duration".into(),
        Type::Percent => "percent".into(),
        Type::Str => "str".into(),
        Type::Bool => "bool".into(),
        Type::Bytes => "bytes".into(),
        Type::Path => "path".into(),
        Type::Char => "char".into(),
        Type::Unit => "unit".into(),
        Type::List(x) => format!("list[{}]", ten_kieu(x)),
        Type::Map(k, v) => format!("map[{}, {}]", ten_kieu(k), ten_kieu(v)),
        Type::Opt(x) => format!("{}?", ten_kieu(x)),
        Type::Result(a, b) => format!("Result[{}, {}]", ten_kieu(a), ten_kieu(b)),
        other => format!("{other:?}").to_lowercase(),
    }
}

impl<'a> Lowerer<'a> {
    /// Một stage: `(group expr)` hoặc `(cmd|ext tên)` + đối số, có thể nhận giá trị từ pipe.
    fn ha_stage(
        &mut self,
        node: &Node,
        truoc: Option<(HExpr, Type)>,
        sigs: &[CmdSig],
    ) -> Option<HExpr> {
        // ST-44: nhóm `()` **rỗng đứng một mình** (không phải `f()`) ⇒ lỗi rõ, không coi là lệnh `(`.
        if let Some(Elem::Node(g)) = node.elems.first() {
            if g.kind == NodeKind::CmdGroup
                && node.elems.len() == 1
                && self
                    .vach_text(g)
                    .map(|t| t.trim().is_empty())
                    .unwrap_or(false)
            {
                self.diag(
                    &vis_diag::SYNTAX_CODES[0],
                    "`( … )` rỗng không phải biểu thức (chỉ `f()` mới hợp lệ làm lời gọi)",
                    g.span,
                );
                return None;
            }
        }
        // ST-34: `| complete` — stage **giả**, chỉ dùng sau chương trình ngoài; nó **đổi tên** callee
        // của stage trước thành `exec.complete.<ten>` (không thêm biến thể HIR ⇒ codegen không đổi).
        if self.ten_lenh(node).as_deref() == Some("complete") {
            if node.child(NodeKind::CmdGroup).is_some() || node.elems.len() > 1 {
                // `complete` không nhận đối số.
                self.diag(&COMMAND_CODES[1], "`complete` không nhận đối số", node.span);
                return None;
            }
            let Some((up, _)) = truoc else {
                self.diag(
                    &COMMAND_CODES[1],
                    "complete chỉ dùng sau chương trình ngoài",
                    node.span,
                );
                return None;
            };
            if let HExpr::Call {
                callee,
                args,
                named,
                span,
            } = up
            {
                if let HExpr::Var { name, sym, .. } = *callee {
                    if let Some(con) = name.strip_prefix("exec.") {
                        return Some(HExpr::Call {
                            callee: Box::new(HExpr::Var {
                                name: format!("exec.complete.{con}"),
                                sym,
                                span,
                            }),
                            args,
                            named,
                            span,
                        });
                    }
                }
            }
            self.diag(
                &COMMAND_CODES[1],
                "complete chỉ dùng sau chương trình ngoài",
                node.span,
            );
            return None;
        }
        // ST-46: chỉ khi **phần tử đầu** là nhóm ⇒ stage-biểu-thức (`( ls … )`); nhóm làm **đối số**
        // (`take ( … )`) do vòng đối số xử lý.
        let head_group = match node.elems.first() {
            Some(Elem::Node(g)) if g.kind == NodeKind::CmdGroup => Some(&**g),
            _ => None,
        };
        if let Some(g) = head_group {
            // ST-37: parser **không** gắn node biểu thức vào `CmdGroup` (chỉ giữ token) ⇒ hạ lại
            // **văn bản trong ngoặc** ở chế độ script (biểu thức thuần: literal/phép toán).
            // ST-46: không phải biểu thức ⇒ thử **một lệnh** trong ngoặc (`( ls ./photos )`).
            // ST-49: biểu thức script (kể cả **nhiều dòng**) → pipeline lệnh (cờ, nhiều stage).
            // ST-51: nhóm **không ngoặc** bắt đầu bằng `{…}` (bản ghi) ⇒ **bọc list một phần tử**
            // để hạ nguồn (`where`/`take`/`select`/bảng) chạy như một dòng; kiểu `list` cho hook.
            let v = self
                .lower_vach(g)
                .map(|e| {
                    let la_ban_ghi = self.nhom_khong_ngoac(g)
                        && self
                            .vach_text(g)
                            .map(|t| t.trim_start().starts_with('{'))
                            .unwrap_or(false);
                    if la_ban_ghi {
                        (
                            HExpr::List {
                                items: vec![e],
                                elem: Type::Error,
                                span: g.span,
                            },
                            Type::List(Box::new(Type::Error)),
                        )
                    } else {
                        (e, Type::Error)
                    }
                })
                .or_else(|| self.lower_nhom(g, sigs));
            if v.is_none() {
                if let Some(e) = truoc.as_ref() {
                    let _ = e;
                    self.diag(
                        &vis_diag::SYNTAX_CODES[0],
                        "trong `( … )`: không phải biểu thức hay lệnh chạy được",
                        g.span,
                    );
                }
            }
            if let Some((e, _ty)) = v {
                if truoc.is_some() {
                    // Stage `( expr )` không có khe ống ⇒ giá trị trước không đi đâu được.
                    self.diag(
                        &COMMAND_CODES[2],
                        "pipeline không nối được: stage dạng `( … )` không nhận giá trị từ pipe",
                        node.span,
                    );
                }
                return Some(e);
            }
        }
        // **ST-59**: token đầu stage là **biến đã bind** (let/var/tham số) ⇒ **stage biểu thức**:
        // `xs | take 1` nhận **giá trị** `xs`; `a` đứng một mình in giá trị biến. Định danh **chưa**
        // bind (và mọi tên dựng sẵn/item) giữ nguyên đường **lệnh**/chương trình ngoài (§9.5).
        if node.children().count() == 0 && node.elems.len() == 1 {
            if let Some(ten) = self.ten_lenh(node) {
                if let Some(sym) = self.bien_gia_tri(&ten) {
                    return Some(HExpr::Var {
                        name: ten,
                        sym: Some(sym),
                        span: node.span,
                    });
                }
            }
        }
        if let Some(ten) = self.ten_lenh(node) {
            if crate::lower::is_builtin_value(&ten) && !sigs.iter().any(|s| s.name == ten) {
                let nhom: Vec<&Node> = node
                    .children()
                    .filter(|c| c.kind == NodeKind::CmdGroup)
                    .collect();
                let cac_phan_tu = node.elems.len();
                if !nhom.is_empty() && cac_phan_tu == nhom.len() + 1 {
                    let mut args = Vec::new();
                    for g in nhom {
                        args.push(self.lower_vach(g)?);
                    }
                    return Some(HExpr::Call {
                        callee: Box::new(HExpr::Var {
                            name: ten,
                            sym: None,
                            span: node.span,
                        }),
                        args,
                        named: Vec::new(),
                        span: node.span,
                    });
                }
            }
        }
        // ST-53: `to json` (`--pretty`) — chi tiết ở `sugar.rs` (R1).
        if self.la_to_json(node) {
            return self.ha_to_json(node, truoc);
        }
        let Some(ten) = self.ten_lenh(node) else {
            // Chương trình ngoài (`^git`) — VS0 chưa có tiến trình ⇒ VS0604, không hạ no-op.
            self.diag(
                &COMMAND_CODES[5],
                "chương trình ngoài (`^…`) chưa hỗ trợ",
                node.span,
            );
            return None;
        };
        if std::env::var("VIS_DBG44").is_ok() {
            eprintln!(
                "stage ten={ten:?} sigs={:?} tu_child={:?}",
                sigs.iter().map(|s| s.name.as_str()).collect::<Vec<_>>(),
                node.span
            );
        }
        let Some(sig) = sigs.iter().find(|s| s.name == ten).cloned() else {
            // ST-27: lệnh **lạ** ⇒ chương trình ngoài (§9.3) — trừ khi host tắt (`--no-extern`).
            if !self.opts.cho_ngoai {
                self.diag(
                    &COMMAND_CODES[0],
                    format!(
                        "không có lệnh `{ten}` trong bảng chữ ký và chương trình ngoài đang tắt (`--no-extern`)"
                    ),
                    node.span,
                );
                return None;
            }
            return Some(self.ngoai_call(&ten, node, truoc.map(|(e, _)| e)));
        };
        // ST-40b: `get name` / `select pid, name` — chiếu trường của `it` (VD-71/73).
        if sig.name == "get" || sig.name == "select" {
            if let Some((up, up_ty)) = &truoc {
                let la_closure = node.children().any(|c| c.kind == NodeKind::CmdClosure);
                if !la_closure {
                    let mut bang = Vec::new();
                    let _ = hop_nhat_cmd(&sig.params[0].ty, up_ty, &sig.generics, &mut bang);
                    let elem = super::generic::thay(
                        &Type::TypeRef {
                            name: "T".into(),
                            args: Vec::new(),
                        },
                        &bang,
                    );
                    if let Some(dau) = self.arg_start(node) {
                        let het = self.stage_end(dau);
                        if let Some(lam) = self.ha_chieu(&sig.name, dau, het, &elem, node.span) {
                            let kieu_ra = match &lam {
                                HExpr::Lambda { ret, .. } => ret.clone(),
                                _ => Type::Error,
                            };
                            self.last_stage_ty = Type::List(Box::new(kieu_ra));
                            return Some(HExpr::Call {
                                callee: Box::new(HExpr::Var {
                                    name: sig.name.clone(),
                                    sym: sig.sym,
                                    span: node.span,
                                }),
                                args: vec![up.clone(), lam],
                                named: Vec::new(),
                                span: node.span,
                            });
                        }
                    }
                }
            }
        }
        // ST-39: khoá ngầm cho `sort_by` (§9.7): `sort_by modified --desc` ⇒ `it => it.modified`, `desc=true`.
        if sig.name == "sort_by" {
            if let Some((up, up_ty)) = &truoc {
                let la_closure = node.children().any(|c| c.kind == NodeKind::CmdClosure);
                if !la_closure {
                    let mut bang = Vec::new();
                    let _ = hop_nhat_cmd(&sig.params[0].ty, up_ty, &sig.generics, &mut bang);
                    let fty = super::generic::thay(&sig.params[1].ty, &bang);
                    let (mut pty, mut rty) = match &fty {
                        Type::Fn(ps, r) if ps.len() == 1 => (ps[0].clone(), (**r).clone()),
                        _ => (Type::Error, Type::Error),
                    };
                    if matches!(&pty, Type::TypeRef { args, .. } if args.is_empty()) {
                        pty = Type::Error;
                    }
                    if matches!(&rty, Type::TypeRef { args, .. } if args.is_empty()) {
                        rty = Type::Error;
                    }
                    if let Some(dau) = self.arg_start(node) {
                        let het_stage = self.stage_end(dau);
                        let vung = self.text.get(dau..het_stage).unwrap_or("").to_string();
                        let body_end = dau + vung.find("--").unwrap_or(vung.len());
                        let co_text = &vung[vung.find("--").unwrap_or(vung.len())..];
                        if let Some(lam) = self.ha_sugar(dau, Some(body_end), pty.clone(), rty) {
                            self.last_stage_ty = Type::List(Box::new(pty.clone()));
                            let mut args = vec![up.clone(), lam];
                            let mut loi = false;
                            for raw in co_text.split_whitespace() {
                                if !raw.starts_with("--") {
                                    continue;
                                }
                                let (ten, gt) = raw
                                    .trim_start_matches('-')
                                    .split_once('=')
                                    .map(|(a, b)| (a.to_string(), Some(b.to_string())))
                                    .unwrap_or((raw.trim_start_matches('-').to_string(), None));
                                if ten != "desc" {
                                    self.diag(
                                        &COMMAND_CODES[1],
                                        format!("`sort_by` không có cờ `--{ten}`"),
                                        node.span,
                                    );
                                    loi = true;
                                    continue;
                                }
                                let b = gt.map(|v| v == "true").unwrap_or(true);
                                args.push(HExpr::Lit {
                                    ty: Type::Bool,
                                    lit: Lit {
                                        kind: LitKind::Bool,
                                        text: b.to_string(),
                                    },
                                    span: node.span,
                                });
                            }
                            if !loi {
                                return Some(HExpr::Call {
                                    callee: Box::new(HExpr::Var {
                                        name: sig.name.clone(),
                                        sym: sig.sym,
                                        span: node.span,
                                    }),
                                    args,
                                    named: Vec::new(),
                                    span: node.span,
                                });
                            }
                            return None;
                        }
                    }
                }
            }
        }
        // ST-38: **vị từ ngầm `it`** cho `where` (§9.7): `where size > 5mb` ⇒ `it => it.size > 5mb`.
        // Chỉ áp khi có pipe vào và đối số **không** phải closure; luật đổi tên ở [`Self::ha_sugar`].
        if sig.name == "where" {
            if let Some((up, up_ty)) = &truoc {
                let la_closure = node
                    .children()
                    .filter(|c| c.kind != NodeKind::CmdWord)
                    .any(|c| c.kind == NodeKind::CmdClosure);
                if !la_closure {
                    let mut bang = Vec::new();
                    let _ = hop_nhat_cmd(&sig.params[0].ty, up_ty, &sig.generics, &mut bang);
                    let fty = super::generic::thay(&sig.params[1].ty, &bang);
                    let (mut pty, mut rty) = match &fty {
                        Type::Fn(ps, r) if ps.len() == 1 => (ps[0].clone(), (**r).clone()),
                        _ => (Type::Error, Type::Error),
                    };
                    // Chưa suy được `T` (stage trước kiểu `{error}`) ⇒ dùng `{error}` **im lặng**:
                    // để `TypeRef{"T"}` lọt xuống tyck sẽ thành `VS0301` (không có kiểu `T`).
                    if matches!(&pty, Type::TypeRef { args, .. } if args.is_empty()) {
                        pty = Type::Error;
                    }
                    if matches!(&rty, Type::TypeRef { args, .. } if args.is_empty()) {
                        rty = Type::Error;
                    }
                    // ST-46: `where ( x => x )` — nhóm chứa **lambda** ⇒ hạ thẳng thành biểu thức
                    // (không đưa qua sugar `it`, tránh đổi tên `x` thành `it.x`).
                    if let Some(g) = node.children().find(|c| c.kind == NodeKind::CmdGroup) {
                        let txt = self.vach_text(g).unwrap_or_default();
                        if txt.contains("=>") {
                            if let Some(mut lam) = self.lower_vach(g) {
                                // ST-50: gán **kiểu tham số** lambda = phần tử pty (nếu đang
                                // `{error}`) ⇒ `where ( x => x.khong_co )` báo **VS0405** tĩnh.
                                if let HExpr::Lambda { params, .. } = &mut lam {
                                    if let Some(p) = params.first_mut() {
                                        if p.ty.is_error() {
                                            p.ty = pty.clone();
                                        }
                                    }
                                }
                                self.last_stage_ty = Type::List(Box::new(pty));
                                return Some(HExpr::Call {
                                    callee: Box::new(HExpr::Var {
                                        name: sig.name.clone(),
                                        sym: sig.sym,
                                        span: node.span,
                                    }),
                                    args: vec![up.clone(), lam],
                                    named: Vec::new(),
                                    span: node.span,
                                });
                            }
                        }
                    }
                    // Vùng văn bản của đối số: từ token đầu **sau tên lệnh** tới hết stage.
                    if let Some(dau) = self.arg_start(node) {
                        let pty_hint = pty.clone();
                        // ST-49: kết thúc **theo span stage** (trong nhóm, `)` không thuộc stage)
                        let lam = self.ha_sugar(dau, Some(node.span.end), pty, rty);
                        if let Some(lam) = lam {
                            self.last_stage_ty = Type::List(Box::new(pty_hint));
                            return Some(HExpr::Call {
                                callee: Box::new(HExpr::Var {
                                    name: sig.name.clone(),
                                    sym: sig.sym,
                                    span: node.span,
                                }),
                                args: vec![up.clone(), lam],
                                named: Vec::new(),
                                span: node.span,
                            });
                        }
                    }
                }
            }
        }
        // Khe ống (§9.5, ST-18 (a)1): tham số vị trí **đầu tiên** khớp kiểu với giá trị vào —
        // **giữ chỗ trước**, để đối số vị trí còn lại không chiếm mất (`take 1` ⇒ `take(xs=pipe, 1)`).
        let mut vi_tri: Vec<Option<HExpr>> = vec![None; sig.params.len()];
        let mut khe: Option<usize> = None;
        let mut bang: Vec<(String, Type)> = Vec::new();
        if let Some((up, up_ty)) = &truoc {
            // ST-37 (§9.5): khe chọn theo **khả năng nhận** — hợp nhất một chiều để buộc `T`.
            // - `up_ty = {error}` (stage nhóm không suy được) ⇒ chọn **vị trí đầu** (an toàn).
            // - KHÔNG đặt khe vào tham số `--cờ`; không đè bind tường minh (khe giữ chỗ trước).
            let mut chon: Option<usize> = None;
            if matches!(up_ty, Type::Error) {
                chon = sig.params.iter().position(|p| !p.named);
            } else {
                for (i, p) in sig.params.iter().enumerate() {
                    if p.named {
                        continue;
                    }
                    let mut b = bang.clone();
                    if hop_nhat_cmd(&p.ty, up_ty, &sig.generics, &mut b) {
                        chon = Some(i);
                        bang = b;
                        break;
                    }
                }
            }
            match chon {
                Some(i) => {
                    vi_tri[i] = Some(up.clone());
                    khe = Some(i);
                }
                None => {
                    self.diag(
                        &COMMAND_CODES[2],
                        format!(
                            "pipeline không nối được: `{ten}` không có khe nhận `{}`",
                            ten_kieu(up_ty)
                        ),
                        node.span,
                    );
                    return None;
                }
            }
        }
        // Đối số tường minh — duyệt **elems** của stage (token lẫn node, theo đúng thứ tự nguồn).
        let mut theo_ten: Vec<(String, HExpr)> = Vec::new();
        let mut i_vi_tri = 0usize;
        let mut bo_qua = 1usize; // phần tử đầu là tên lệnh
        for e in &node.elems {
            if bo_qua > 0 {
                bo_qua -= 1;
                continue;
            }
            let (la_co, text, con_span, gt) = match e {
                Elem::Token(i) => {
                    let Some(t) = self.tok(*i) else { continue };
                    if t.kind == Kind::Comment {
                        continue;
                    }
                    let la = t.text.starts_with('-') && t.text != "-" && t.text != "--";
                    (la, t.text.clone(), t.span, None)
                }
                Elem::Node(n) => match n.kind {
                    NodeKind::CmdFlag => {
                        let ten_co = n
                            .tokens(&self.cst.tokens)
                            .next()
                            .map(|t| t.text.clone())
                            .unwrap_or_default();
                        (true, ten_co, n.span, n.children().next().cloned())
                    }
                    NodeKind::CmdGroup | NodeKind::CmdClosure => {
                        (false, String::new(), n.span, Some((**n).clone()))
                    }
                    NodeKind::CmdVar | NodeKind::CmdEnv => {
                        self.diag(&COMMAND_CODES[3], "biến `$x`/`$env.X` chưa hỗ trợ", n.span);
                        return None;
                    }
                    _ => (
                        false,
                        n.tokens(&self.cst.tokens)
                            .map(|t| t.text.clone())
                            .collect::<String>(),
                        n.span,
                        None,
                    ),
                },
            };
            // `--x` có thể đến dưới dạng **token Word** hoặc node `CmdWord` ⇒ nhận theo văn bản.
            // Ngoại lệ VD-40 #2: `-5` là **số** nếu tham số kế tiếp kiểu số.
            let so_am = text
                .strip_prefix('-')
                .and_then(|r| r.parse::<f64>().ok())
                .is_some();
            let kieu_ke = {
                let mut k = i_vi_tri;
                while k < sig.params.len() && (sig.params[k].named || Some(k) == khe) {
                    k += 1;
                }
                sig.params.get(k).map(|p| p.ty.clone())
            };
            let la_co = (la_co
                || (text.starts_with('-') && text != "-" && text != "--" && gt.is_none()))
                && !(so_am
                    && matches!(
                        kieu_ke,
                        Some(Type::Int)
                            | Some(Type::Float)
                            | Some(Type::Size)
                            | Some(Type::Percent)
                            | Some(Type::Duration)
                    ));
            if la_co {
                // ST-27: `--x=v` — tên cờ và giá trị tách bằng `=`, **không** lấy khoảng trắng.
                let (ten_sach, gt_bang) = match text.trim_start_matches('-').split_once('=') {
                    Some((n, v)) => (n.to_string(), Some(v.to_string())),
                    None => (text.trim_start_matches('-').to_string(), None),
                };
                if text == "--" {
                    continue; // `--` kết thúc cờ (VD-40 #4)
                }
                let Some(p) = sig
                    .params
                    .iter()
                    .find(|p| p.named && p.name == ten_sach)
                    .cloned()
                else {
                    self.diag(
                        &COMMAND_CODES[1],
                        format!("`{ten}` không có cờ `{text}`"),
                        con_span,
                    );
                    return None;
                };
                let gt = if let Some(v) = gt_bang {
                    // `--x=v` ⇒ giá trị `v` (đã bỏ khoảng trắng quanh `=`), đổi theo kiểu cờ.
                    match self.chuyen_doi_text(&v, &p.ty, con_span) {
                        Some(e) => e,
                        None => {
                            self.diag(
                                &COMMAND_CODES[1],
                                format!(
                                    "cờ `--{ten_sach}={v}` sai kiểu (cần `{}`)",
                                    ten_kieu(&p.ty)
                                ),
                                con_span,
                            );
                            return None;
                        }
                    }
                } else {
                    match gt {
                        Some(v) if v.kind == NodeKind::CmdGroup => match v.children().next() {
                            Some(x) => self.lower_expr(x),
                            None => continue,
                        },
                        Some(v) => match self.chuyen_doi(&v, &p.ty) {
                            Some(x) => x,
                            None => {
                                self.diag(
                                    &COMMAND_CODES[1],
                                    format!("cờ `{text}` sai kiểu (cần `{}`)", ten_kieu(&p.ty)),
                                    con_span,
                                );
                                return None;
                            }
                        },
                        None if p.ty == Type::Bool => HExpr::Lit {
                            lit: Lit {
                                kind: LitKind::Bool,
                                text: "true".into(),
                            },
                            ty: Type::Bool,
                            span: con_span,
                        },
                        None => {
                            self.diag(
                                &COMMAND_CODES[1],
                                format!("cờ `{text}` cần giá trị nhưng chưa hỗ trợ"),
                                con_span,
                            );
                            return None;
                        }
                    }
                };
                theo_ten.push((ten_sach, gt));
                continue;
            }
            // ST-44: `f()` — nhóm **rỗng** là dấu **lời gọi không đối số**, không phải đối số.
            // Nhóm **không rỗng** làm đối số vẫn là **nợ VSH1b** ⇒ giữ `VS0603` như trước.
            if let Some(g) = gt.as_ref() {
                if g.kind == NodeKind::CmdGroup
                    && self
                        .vach_text(g)
                        .map(|t| t.trim().is_empty())
                        .unwrap_or(false)
                {
                    continue; // `f()` — lời gọi **không đối số**
                }
            }
            // Đối số vị trí (bỏ qua ô **khe ống** đã giữ chỗ).
            while i_vi_tri < sig.params.len()
                && (sig.params[i_vi_tri].named || Some(i_vi_tri) == khe)
            {
                i_vi_tri += 1;
            }
            if i_vi_tri >= sig.params.len() {
                self.diag(&COMMAND_CODES[1], format!("`{ten}` thừa đối số"), con_span);
                return None;
            }
            // ST-37: thay tham số kiểu đã suy (từ khe hoặc từ đối số trước) trước khi kiểm đối số.
            let ty = super::generic::thay(&sig.params[i_vi_tri].ty, &bang);
            // ST-45: **`each` không kiểm kiểu trả** (giống `for` trong Rust) — khai kiểu trả lambda là
            // `{error}` để tyck **không** `coerce` về `()`; thân vẫn được kiểm (field/kiểu/cú pháp).
            let ty = if sig.name == "each" {
                match ty {
                    Type::Fn(ps, _) => Type::Fn(ps, Box::new(Type::Error)),
                    other => other,
                }
            } else {
                ty
            };
            // ST-37: buộc `T` từ **kiểu suy được của đối số** (literal) trước khi chuyển.
            let suy = match gt.as_ref() {
                Some(g) if g.kind == NodeKind::CmdGroup => self
                    .vach_text(g)
                    .map(|t| self.kieu_text(&t))
                    .unwrap_or(Type::Error),
                _ => self.kieu_text(&text),
            };
            let _ = hop_nhat_cmd(&ty, &suy, &sig.generics, &mut bang);
            let expr = match gt {
                Some(g) if g.kind == NodeKind::CmdClosure => {
                    let lam = self.ha_closure(&g, &ty);
                    // ST-39: `.vish` **không** qua tyck ⇒ kiểm khoá ở đây khi biết kiểu: thân chỉ là
                    // chính tham số (`x => x`) ⇒ khoá = kiểu tham số; `bool`/`map`/`record`/`list`
                    // không so được ⇒ `VS0412`.
                    if sig.name == "sort_by" {
                        if let (Some(HExpr::Lambda { params, body, .. }), Type::Fn(_, _)) =
                            (lam.as_ref(), &ty)
                        {
                            let khoa = match (body, params.first()) {
                                (crate::hir::HBody::Expr(e), Some(p)) => match e.as_ref() {
                                    HExpr::Var { name, .. } if *name == p.name => {
                                        Some(p.ty.clone())
                                    }
                                    _ => None,
                                },
                                _ => None,
                            };
                            if let Some(k) = khoa {
                                if matches!(
                                    &k,
                                    Type::Bool | Type::Map(..) | Type::Record(..) | Type::List(..)
                                ) {
                                    self.diag(
                                        &TYPE_CODES[4],
                                        format!(
                                            "`sort_by`: khoá không so được: `{}`",
                                            self.tbl.show(&k)
                                        ),
                                        g.span,
                                    );
                                }
                            }
                        }
                    }
                    lam
                }
                Some(g) if g.kind == NodeKind::CmdGroup => {
                    // ST-46: `( … )` ở **vị trí đối số** = **biểu thức** (bỏ `VS0603`); kiểm kiểu
                    // **tĩnh** khi `kieu_text` suy được (`1 + 1`/lambda ⇒ `{error}` ⇒ kiểm động).
                    let txt = self.vach_text(&g).unwrap_or_default();
                    let suy = self.kieu_text(&txt);
                    let khop = matches!(suy, Type::Error) || matches!(ty, Type::Error) || suy == ty;
                    if !khop {
                        self.diag(
                            &TYPE_CODES[4],
                            format!(
                                "`( … )` làm đối số: cần `{}`, được `{}`",
                                ten_kieu(&ty),
                                ten_kieu(&suy)
                            ),
                            g.span,
                        );
                        return None;
                    }
                    self.lower_vach(&g)
                }
                Some(g) => {
                    let inner = g.children().next().cloned();
                    match inner {
                        Some(inner) => {
                            let raw = self.text_of(inner.span).to_string();
                            self.chuyen_doi_text(&raw, &ty, inner.span)
                        }
                        None => None,
                    }
                }
                None => {
                    let raw = if text.is_empty() {
                        self.text_of(con_span).to_string()
                    } else {
                        text.clone()
                    };
                    self.chuyen_doi_text(&raw, &ty, con_span)
                }
            };
            let Some(expr) = expr else {
                self.diag(
                    &COMMAND_CODES[1],
                    format!(
                        "`{ten}`: đối số `{}` sai kiểu (cần `{}`)",
                        text,
                        ten_kieu(&ty)
                    ),
                    con_span,
                );
                return None;
            };
            vi_tri[i_vi_tri] = Some(expr);
            i_vi_tri += 1;
        }
        // Builtin: interp nhận **đối số vị trí** (không có binding theo tên) ⇒ đặt cờ vào đúng ô,
        // điền default cho các ô trống phía trước.
        if sig.sym.is_none() {
            for (ten_co, gt) in &theo_ten {
                if let Some(i) = sig.params.iter().position(|p| p.named && &p.name == ten_co) {
                    for (j, o) in vi_tri.iter_mut().enumerate().take(i) {
                        if o.is_none() {
                            *o = sig.params[j]
                                .default
                                .clone()
                                .map(|d| mac_dinh_theo_tep(d, self.file));
                        }
                    }
                    vi_tri[i] = Some(gt.clone());
                }
            }
            theo_ten.clear();
            // ST-42: điền default cho **mọi** ô còn trống (kể cả ô **sau** cờ, vd `long` sau `--all`),
            // nếu không `day_du` sai ⇒ cờ bị **bỏ** khỏi call.
            for (j, o) in vi_tri.iter_mut().enumerate() {
                if o.is_none() {
                    *o = sig.params[j]
                        .default
                        .clone()
                        .map(|d| mac_dinh_theo_tep(d, self.file));
                }
            }
            let day_du = vi_tri.iter().all(|x| x.is_some());
            if day_du {
                let args: Vec<HExpr> = vi_tri.into_iter().flatten().collect();
                return Some(HExpr::Call {
                    callee: Box::new(HExpr::Var {
                        name: sig.name.clone(),
                        sym: None,
                        span: node.span,
                    }),
                    args,
                    named: Vec::new(),
                    span: node.span,
                });
            }
        }
        // Thiếu tham số **bắt buộc** (không default) ⇒ VS0602.
        let mut args: Vec<HExpr> = Vec::new();
        for (i, p) in sig.params.iter().enumerate() {
            if p.named {
                continue;
            }
            match &vi_tri[i] {
                Some(e) => args.push(e.clone()),
                None if p.default.is_none() => {
                    self.diag(
                        &COMMAND_CODES[1],
                        format!("`{ten}` thiếu đối số `{}`", p.name),
                        node.span,
                    );
                    return None;
                }
                None => {
                    // Default ở giữa: truyền theo tên để interp tự điền default.
                    break;
                }
            }
        }
        let callee = match sig.sym {
            Some(sym) => HExpr::Var {
                name: sig.name.clone(),
                sym: Some(sym),
                span: node.span,
            },
            None => HExpr::Var {
                name: sig.name.clone(),
                sym: None,
                span: node.span,
            },
        };
        Some(HExpr::Call {
            callee: Box::new(callee),
            args,
            named: theo_ten,
            span: node.span,
        })
    }

    /// Chuyển một **từ trần**/chuỗi thành literal **đúng kiểu tham số** (pha 2 của VD-22).
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
