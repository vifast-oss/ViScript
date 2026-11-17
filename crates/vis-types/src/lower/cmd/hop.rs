// SPDX-License-Identifier: MIT OR Apache-2.0
//! Hạ **pipeline/nhóm/stage type** của `cmd` (ST-106 tách từ `cmd.rs`; thuần refactor).

use super::super::*;
use crate::cmdsig::CmdSig;
use vis_diag::COMMAND_CODES;

impl<'a> Lowerer<'a> {
    /// Chạy hạ chế độ lệnh: item (`cmd`/`fn`/`extern`) trước, rồi từng dòng lệnh thành `HStmt`.
    /// Chạy các dòng lệnh của CST chế độ lệnh; **bỏ** mọi node có `span.start < tu` (ST-44: dòng
    /// đã hạ trong **vùng khai báo** ở tiền tố tệp).
    pub(crate) fn run_shell_tu(&mut self, tu: usize) {
        self.declare_builtins();
        let kids: Vec<Node> = self.cst.root.children().cloned().collect();
        for child in &kids {
            if is_item(child.kind) && child.span.start >= tu {
                self.predeclare_item(child);
            }
        }
        for child in &kids {
            if is_item(child.kind) && child.span.start >= tu {
                if let Some(it) = self.build_item(child) {
                    self.items.push(it);
                }
            }
        }
        let mut sigs: Vec<CmdSig> = self
            .items
            .iter()
            .filter_map(|it| match it {
                HItem::Fn(f) if f.kind == FnKind::Cmd => Some(CmdSig {
                    name: f.name.clone(),
                    sym: Some(f.sym),
                    generics: f.generics.clone(),
                    params: f.params.clone(),
                    ret: f.ret.clone(),
                }),
                _ => None,
            })
            .collect();
        sigs.extend(self.cmd_sigs.clone());
        for child in &kids {
            if child.span.start < tu {
                continue;
            }
            match child.kind {
                NodeKind::CmdItems => self.ha_dong(child, &sigs),
                NodeKind::CmdBg => {
                    self.diag(
                        &COMMAND_CODES[4],
                        "chạy nền `&` chưa hỗ trợ (chưa có tiến trình)",
                        child.span,
                    );
                }
                k if !is_item(k) => {
                    let mut out = Vec::new();
                    if let Some(e) = self.ha_pipeline(child, &sigs) {
                        // ST-46: dòng **chỉ** gọi `print(…)` ⇒ không bọc `render` (không in thêm `()`).
                        let la_print = matches!(
                            &e,
                            HExpr::Call { callee, .. }
                                if matches!(&**callee, HExpr::Var { name, .. } if name == "print")
                        );
                        let e = if la_print {
                            e
                        } else {
                            self.boc_render(e, child.span)
                        };
                        out.push(HStmt::Expr(e));
                    }
                    self.stmts.extend(out);
                }
                _ => {}
            }
        }
    }

    /// `render(<expr>)` (ST-41) — §9.6: giá trị **cuối pipeline** hiển thị qua **`Render`**; `print`
    /// của người dùng **không** đổi (tên builtin riêng, chỉ dùng cho giá trị cuối).
    /// Một dòng lệnh (`items`): mỗi pipeline/bg là một câu lệnh, **theo thứ tự**.
    pub(super) fn ha_dong(&mut self, node: &Node, sigs: &[CmdSig]) {
        for con in node.children() {
            if con.kind == NodeKind::CmdBg {
                // (a)4 — VS0 chưa có tiến trình: báo đúng **VS0604**, không hạ thành no-op.
                self.diag(
                    &COMMAND_CODES[4],
                    "chạy nền `&` chưa hỗ trợ (chưa có tiến trình)",
                    con.span,
                );
                continue;
            }
            if let Some(e) = self.ha_pipeline(con, sigs) {
                let e = self.boc_render(e, con.span);
                self.stmts.push(HStmt::Expr(e));
            }
        }
    }

    /// `pipeline := stage ('|' stage)*` ⇒ **lồng call** theo khe ống (§9.5, ST-18 (a)).
    pub(crate) fn ha_pipeline(&mut self, node: &Node, sigs: &[CmdSig]) -> Option<HExpr> {
        let stage: Vec<&Node> = node
            .children()
            .filter(|n| n.kind == NodeKind::CmdStage)
            .collect();
        if stage.is_empty() {
            return None;
        }
        let mut truoc: Option<(HExpr, Type)> = None;
        let mut ket: Option<(HExpr, Type)> = None;
        self.last_stage_ty = Type::Error;
        for st in &stage {
            let e = self.ha_stage(st, truoc.take(), sigs)?;
            let ty = self.kieu_stage(st, sigs);
            truoc = Some((e, ty));
            ket = truoc.clone();
        }
        // ST-41: hook `where`/`sort_by`/`get`/`select` đã đặt kiểu **chính xác** (giữ nguyên); nếu chưa
        // (stage thường/group) ⇒ dùng `kieu_stage` (literal bản ghi ⇒ `list[{a: int}]`).
        if matches!(self.last_stage_ty, Type::Error) {
            if let Some(last) = stage.last() {
                self.last_stage_ty = self.kieu_stage(last, sigs);
            }
        }
        ket.map(|(e, _)| e)
    }
    pub(super) fn kieu_stage(&self, node: &Node, sigs: &[CmdSig]) -> Type {
        // **ST-59**: stage là **biến đã bind** ⇒ kiểu tĩnh của nó (nếu có chú thích) — nhờ vậy hook
        // hạ nguồn (`where`/`get`/`select`/`sort_by`) kiểm **tĩnh** theo kiểu biến.
        if node.children().count() == 0 && node.elems.len() == 1 {
            if let Some(ten) = self.ten_lenh(node) {
                if let Some(sym) = self.bien_gia_tri(&ten) {
                    if let Some(t) = self.type_env.get(&sym) {
                        return t.clone();
                    }
                }
            }
        }
        // ST-53: `to json` ⇒ `str` (shell sugar tự phân giải, **không** cần `use std.json`).
        if self.la_to_json(node) {
            return Type::Str;
        }
        // ST-37: stage `( expr )` — suy kiểu **literal** (list/str/int/bool) thay vì `{error}`.
        // ST-46: **chỉ** khi phần tử **đầu** của stage là nhóm thì mới là stage-biểu-thức;
        // `take ( … )`/`f()` có nhóm là **đối số** nên không rơi vào nhánh này.
        let head_group = match node.elems.first() {
            Some(Elem::Node(g)) if g.kind == NodeKind::CmdGroup => Some(&**g),
            _ => None,
        };
        if let Some(g) = head_group {
            if let Some(txt) = self.vach_text(g) {
                // ST-49: newline thành dấu cách (1 byte → 1 byte) trước khi suy kiểu biểu thức.
                let phang = txt.replace(['\n', '\r'], " ");
                if !phang.trim().is_empty() {
                    let t = self.kieu_text(phang.trim());
                    if !matches!(t, Type::Error) {
                        return t;
                    }
                    // ST-51: nhóm-không-ngoặc bắt đầu `{…}` ⇒ **list một phần tử** (đã bọc ở hạ).
                    if self.nhom_khong_ngoac(g) && phang.trim_start().starts_with('{') {
                        return Type::List(Box::new(Type::Error));
                    }
                    // Nhóm-lệnh (`( ls --long )`, `( ls | where … )`) ⇒ kiểu stage **đầu**.
                    let t2 = self.kieu_nhom(g, sigs);
                    if !matches!(t2, Type::Error) {
                        return t2;
                    }
                    return t;
                }
            }
        }
        let Some(ten) = self.ten_lenh(node) else {
            return Type::Error;
        };
        let mut r = sigs
            .iter()
            .find(|s| s.name == ten)
            .map(|s| s.ret.clone())
            .unwrap_or(Type::Error);
        // ST-46: `ls --long` ⇒ **bản ghi riêng** `FileEntryLong` (6 trường) cho kiểm tĩnh hạ nguồn.
        let stage_txt = self.text_of(node.span);
        if ten == "ls" && stage_txt.contains("--long") {
            if let Some(id) = self.file_long {
                r = Type::List(Box::new(Type::Record(id)));
            }
        }
        r
    }
    /// **ST-59**: ký hiệu **biến giá trị** đã bind cho `ten` (let/var/tham số/pattern) — **không**
    /// tính tên dựng sẵn, item cấp tệp, hay tham số kiểu; `None` ⇒ giữ đường lệnh như cũ.
    pub(super) fn bien_gia_tri(&self, ten: &str) -> Option<u32> {
        let sym = self.resolve(ten)?;
        let kind = self.symbols.syms.get(sym as usize).map(|s| s.kind)?;
        matches!(
            kind,
            SymKind::Var | SymKind::Param | SymKind::PatBind | SymKind::ElseBind
        )
        .then_some(sym)
    }
    /// Tên lệnh = **phần tử đầu** của stage (token `Word` hoặc node `CmdWord`).
    pub(super) fn ten_lenh(&self, node: &Node) -> Option<String> {
        match node.elems.first()? {
            Elem::Token(i) => {
                let t = self.tok(*i)?;
                if t.kind == Kind::Word || t.kind == Kind::Ident {
                    Some(t.text.clone())
                } else {
                    None
                }
            }
            Elem::Node(n) => n.tokens(&self.cst.tokens).next().map(|t| t.text.clone()),
        }
    }
}
