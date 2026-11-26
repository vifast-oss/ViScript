// SPDX-License-Identifier: MIT OR Apache-2.0
//! Một **dòng** lệnh: `cmd_line` (`;`/pipeline) và stage đầu là biểu thức đóng.
//!
//! **ST-318f** — tách **nguyên văn** khỏi `parse/cmd.rs` (R1 mềm): chỉ đổi vị trí,
//! sexp/CST/thứ tự chẩn đoán/API không đổi.

use super::phan_loai::{let_la_khai_voi, ten_khai_bao, tham_so_cua, KHAI};
use crate::cst::{Elem, NodeKind};
use crate::lex::token::Kind;
use crate::parse::{PResult, Parser};

impl<'a> Parser<'a> {
    pub(super) fn cmd_at_line_end(&self) -> bool {
        self.at_kind(Kind::Newline) || self.at_eof()
    }

    pub(super) fn cmd_line(&mut self) -> PResult<(Elem, String)> {
        // ST-44: dòng **cột 1** mở đầu bằng từ khoá khai báo (hoặc `let`) ⇒ khối **raw**; nội dung
        // được parse lại bằng `Mode::Script` ở tầng hạ (tiền tố tệp). Không đổi `.vish` không khai báo.
        if self.at_kind(Kind::Word) && self.tok().col == 1 {
            let w = self.tok().text.clone();
            let let_khai = w == "let" && {
                let dong = self.cmd_dong_nay();
                let_la_khai_voi(&dong, &self.da_bind)
            };
            if KHAI.contains(&w.as_str()) || let_khai {
                // **ST-60b**: ghi tên vừa khai vào tập đã bind **trước** khi xử lý dòng kế.
                let dong = self.cmd_dong_nay();
                if let Some(ten) = ten_khai_bao(&dong) {
                    self.da_bind.insert(ten);
                }
                let kq = self.cmd_khai();
                // `cmd f(…)` có tham số ⇒ tham số **đã bind** trong thân (cho `let b = a`).
                for p in tham_so_cua(&dong) {
                    self.da_bind.insert(p);
                }
                return kq;
            }
        }
        // `let` = `shell_let` (chỉ ở đầu dòng, khi **không** phải khai báo cấp tệp).
        if self.at_kind(Kind::Word) && self.tok().text == "let" {
            let (e, s) = self.cmd_let()?;
            if !(self.cmd_at_line_end() || self.at_punct(";")) {
                return Err(self.err_here(format!(
                    "mong xuống dòng sau `let`, thật `{}`",
                    self.tok().text
                )));
            }
            return Ok((e, s));
        }
        // **ST-51**: dòng **bắt đầu bằng biểu thức đóng** (`[`, `{`, hoặc chuỗi) **và** có `|`
        // ở độ sâu 0 ⇒ stage đầu là **biểu thức** (nới **hẹp nhất**, chỉ chế độ lệnh).
        if let Some(r) = self.cmd_bieu_thuc_dau()? {
            return Ok(r);
        }
        let mut elems = Vec::new();
        let mut its = Vec::new();
        loop {
            let (e, s) = self.cmd_pipeline()?;
            elems.push(e);
            its.push(s);
            if self.at_punct(";") {
                elems.push(self.bump());
                if self.cmd_at_line_end() {
                    return Err(self.err_here("`;` không có vế sau"));
                }
                continue;
            }
            break;
        }
        if its.len() == 1 {
            let e = elems.pop().expect("một item");
            Ok((e, its.pop().unwrap_or_default()))
        } else {
            let node = self.node(NodeKind::CmdItems, elems);
            Ok((
                Elem::Node(Box::new(node)),
                format!("(items {})", its.join(" ")),
            ))
        }
    }

    /// **ST-51**: stage đầu là **biểu thức đóng** — chỉ nhận khi dòng **bắt đầu** bằng `[`,
    /// `{`, hoặc **chuỗi**, **và** có `|` độ sâu 0; tiền tố phải parse **sạch** ở `Mode::Script`.
    /// `None` ⇒ không đụng (`pos` giữ nguyên) ⇒ đường cũ báo `VS0201` như trước.
    fn cmd_bieu_thuc_dau(&mut self) -> PResult<Option<(Elem, String)>> {
        let la_mo = match self.tok().kind {
            Kind::Str => true,
            Kind::Punct => self.tok().is_punct("[") || self.tok().is_punct("{"),
            _ => false,
        };
        if !la_mo {
            return Ok(None);
        }
        let start = self.tok().span.start;
        let mut k = 0usize;
        let mut depth = 0i32;
        let mut gap = None;
        loop {
            let t = self.tok_at(k);
            if t.kind == Kind::Eof || t.kind == Kind::Newline {
                break;
            }
            match t.text.as_str() {
                "(" | "[" | "{" => depth += 1,
                ")" | "]" | "}" => depth -= 1,
                "|" if depth == 0 => {
                    gap = Some(k);
                    break;
                }
                ";" | "&" if depth == 0 => break,
                _ => {}
            }
            k += 1;
        }
        let Some(k) = gap else { return Ok(None) };
        let pipe = self.tok_at(k).span;
        let dau = self
            .text
            .get(start..pipe.start)
            .unwrap_or("")
            .trim_end()
            .to_string();
        if crate::parse::parse(&dau, crate::lex::Mode::Script)
            .diagnostics
            .iter()
            .any(|_| true)
        {
            return Ok(None);
        }
        let sub = crate::parse::parse(&dau, crate::lex::Mode::Script);
        let inner = sub.cst.sexp_expr().unwrap_or_else(|| "expr".to_string());
        // Token của **biểu thức** (chưa tới `|`).
        let mut tok_elems: Vec<Elem> = Vec::new();
        for _ in 0..k {
            tok_elems.push(self.bump());
        }
        let gnode = self.node(NodeKind::CmdGroup, tok_elems);
        let mut elems: Vec<Elem> = Vec::new();
        let mut ss: Vec<String> = vec![format!("(group {inner})")];
        let stage = self.node(NodeKind::CmdStage, vec![Elem::Node(Box::new(gnode))]);
        elems.push(Elem::Node(Box::new(stage)));
        while self.at_punct("|") {
            elems.push(self.bump());
            let (e, s2) = self.cmd_stage()?;
            elems.push(e);
            ss.push(s2);
        }
        let pipe_node = self.node(NodeKind::CmdPipe, elems);
        Ok(Some((
            Elem::Node(Box::new(pipe_node)),
            format!("(pipe {})", ss.join(" ")),
        )))
    }
}
