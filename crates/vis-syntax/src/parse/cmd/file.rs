// SPDX-License-Identifier: MIT OR Apache-2.0
//! Toàn tệp chế độ lệnh (`cmd_file`), phân loại thân `cmd` và khối khai báo `.vish`.
//!
//! **ST-318f** — tách **nguyên văn** khỏi `parse/cmd.rs` (R1 mềm): chỉ đổi vị trí,
//! sexp/CST/thứ tự chẩn đoán/API không đổi.

use super::phan_loai::{ten_mac_dinh, tham_so_cua, than_khai_lenh_voi, KieuThan};
use crate::cst::{Elem, NodeKind};
use crate::lex::token::{Kind, Span};
use crate::parse::{PResult, Parser};

impl<'a> Parser<'a> {
    pub(super) fn tok_span(&self, e: &Elem) -> Span {
        match e {
            Elem::Token(i) => self.toks[*i as usize].span,
            Elem::Node(n) => n.span,
        }
    }

    pub(super) fn text_at(&self, span: Span) -> String {
        self.text[span.start.min(self.text.len())..span.end.min(self.text.len())].to_string()
    }

    /// Toàn tệp chế độ lệnh: `(shell <item>*)`, mỗi dòng một item.
    pub fn cmd_file(&mut self) -> PResult<(crate::cst::Node, String)> {
        let mut elems = Vec::new();
        let mut items: Vec<String> = Vec::new();
        while !self.at_eof() {
            if self.at_kind(Kind::Newline) {
                elems.push(self.bump());
                continue;
            }
            // **ST-48**: thân lệnh của `cmd` là dòng **thụt** ⇒ lexer sinh `INDENT`/`DEDENT`;
            // chúng là dấu cấu trúc của **script**, không phải stage — bỏ qua ở chế độ lệnh.
            if self.at_kind(Kind::Indent) || self.at_kind(Kind::Dedent) {
                self.bump();
                continue;
            }
            let (e, s) = self.cmd_line()?;
            let vua_khai = matches!(&e, Elem::Node(n) if n.kind == NodeKind::CmdKhai);
            elems.push(e);
            items.push(s);
            if self.at_kind(Kind::Newline) {
                elems.push(self.bump());
            } else if vua_khai && !self.at_eof() {
                // **ST-48**: sau `cmd …:` (thân lệnh, có thể **rỗng**) — khối khai báo đã dừng sau
                // dòng `:`; dòng kế tiếp (thân thụt hoặc stage mới) do vòng lặp này parse tiếp.
                continue;
            } else if !self.at_eof() {
                return Err(self.err_here(format!(
                    "mong xuống dòng sau lệnh, thật `{}`",
                    self.tok().text
                )));
            }
        }
        let node = self.node(NodeKind::CmdShell, elems);
        let sexp = match items.len() {
            0 => "(shell)".to_string(),
            1 => items.pop().unwrap_or_default(),
            _ => format!("(shell {})", items.join(" ")),
        };
        Ok((node, sexp))
    }

    /// **ST-48**: phân loại thân `cmd` đang đứng ở dòng hiện tại (dòng `:` đầu tiên của nó).
    /// `Script` = thân script (ST-44 giữ nguyên); `Command` = thân một pipeline lệnh; `Mixed` =
    /// mơ hồ (nhiều dòng mà **cả** script **và** lệnh đều parse sạch) ⇒ tầng hạ báo `VS0201`.
    fn than_cmd_kiem(&self) -> (KieuThan, Vec<String>) {
        let start = self.tok().span.start;
        let dau = self.text[..start.min(self.text.len())]
            .rfind('\n')
            .map(|i| i + 1)
            .unwrap_or(0);
        let sau_dong = self.text[dau..]
            .find('\n')
            .map(|i| dau + i + 1)
            .unwrap_or(self.text.len());
        let mut dong: Vec<String> = Vec::new();
        let mut p = sau_dong;
        while p < self.text.len() {
            let het = self.text[p..]
                .find('\n')
                .map(|i| p + i)
                .unwrap_or(self.text.len());
            let than = &self.text[p..het];
            if !than.trim().is_empty() && !than.starts_with([' ', '\t']) {
                break; // dòng cột 1 ⇒ hết thân
            }
            dong.push(than.to_string());
            p = if het < self.text.len() { het + 1 } else { het };
        }
        let khac: Vec<String> = dong
            .iter()
            .filter(|l| !l.trim().is_empty())
            .cloned()
            .collect();
        if khac.is_empty() {
            return (KieuThan::Command, khac); // thân rỗng ⇒ tầng hạ báo `VS0201`
        }
        let le = khac
            .iter()
            .map(|l| l.len() - l.trim_start().len())
            .min()
            .unwrap_or(0);
        let noi: Vec<String> = khac
            .iter()
            .map(|l| l.get(le..).unwrap_or(l).to_string())
            .collect();
        let than = noi.join("\n");
        let script = crate::parse::parse(&than, crate::lex::Mode::Script)
            .diagnostics
            .is_empty();
        let lenh = crate::parse::parse(&than, crate::lex::Mode::Command)
            .diagnostics
            .is_empty();
        // **ST-59**: khai báo + **một** pipeline/biểu thức cuối ⇒ gom **raw** như Script (không
        // parse lệnh); tầng hạ tự tách khai báo / phần cuối. (Đặt **trước** nhánh `(script, lenh)`
        // để thân "let … + biến" không rơi vào `Mixed` ⇒ tránh parse lệnh trên dòng khai báo.)
        let mut bind = ten_mac_dinh();
        for p in tham_so_cua(&self.cmd_dong_nay()) {
            bind.insert(p);
        }
        if than_khai_lenh_voi(&noi, &bind) {
            return (KieuThan::Script, khac);
        }
        let kieu = match (script, lenh) {
            (true, false) => KieuThan::Script,
            (false, _) => KieuThan::Command,
            (true, true) => {
                if noi.len() == 1 {
                    KieuThan::Script
                } else {
                    KieuThan::Mixed
                }
            }
        };
        (kieu, khac)
    }

    /// Văn bản **dòng hiện tại** (từ đầu dòng tới trước `\n`) — dùng cho luật `let` khai báo.
    pub(super) fn cmd_dong_nay(&self) -> String {
        let start = self.tok().span.start;
        let base = self.text[..start.min(self.text.len())]
            .rfind('\n')
            .map(|i| i + 1)
            .unwrap_or(0);
        let end = self.text[base..]
            .find('\n')
            .map(|i| base + i)
            .unwrap_or(self.text.len());
        self.text[base..end].to_string()
    }

    /// Khối khai báo `.vish`: gom token từ dòng cột 1 tới **hết** các dòng thụt (thân item), dừng
    /// trước dòng cột 1 kế tiếp. Không sinh sexp cú pháp (nội dung hạ bằng đường script).
    pub(super) fn cmd_khai(&mut self) -> PResult<(Elem, String)> {
        // **ST-48**: `cmd` có thể có **thân lệnh** (pipeline) — thử parse thân như **script**;
        // hỏng ⇒ thân lệnh ⇒ khối khai báo **dừng sau dòng `:`**, các dòng thân do parser lệnh
        // chính parse thành stage (span tuyệt đối, cùng CST).
        if self.at_kind(Kind::Word) && self.tok().text == "cmd" {
            let (kieu, _) = self.than_cmd_kiem();
            if kieu != KieuThan::Script {
                let mut elems: Vec<Elem> = Vec::new();
                while !self.at_eof() {
                    let het = self.at_kind(Kind::Newline);
                    elems.push(self.bump());
                    if het {
                        break;
                    }
                }
                return Ok((
                    Elem::Node(Box::new(self.node(NodeKind::CmdKhai, elems))),
                    "(khai)".to_string(),
                ));
            }
        }
        let mut elems: Vec<Elem> = Vec::new();
        let mut sau_dong_dau = false;
        loop {
            if self.at_eof() {
                break;
            }
            if sau_dong_dau {
                let t = self.tok();
                if !self.at_kind(Kind::Newline) && t.col == 1 {
                    break;
                }
            }
            if self.at_kind(Kind::Newline) {
                // Kết thúc khối **trước** newline cuối nếu dòng kế tiếp là cột 1 (hoặc EOF) — để
                // `cmd_file` tự tiêu thụ newline như mọi dòng.
                let mut j = self.pos;
                while j < self.sig.len() && self.toks[self.sig[j] as usize].kind == Kind::Newline {
                    j += 1;
                }
                let het = match self.sig.get(j) {
                    None => true,
                    Some(&ti) => {
                        let t = &self.toks[ti as usize];
                        t.kind == Kind::Eof || t.col == 1
                    }
                };
                if het {
                    break;
                }
                elems.push(self.bump());
                sau_dong_dau = true;
                continue;
            }
            elems.push(self.bump());
        }
        Ok((
            Elem::Node(Box::new(self.node(NodeKind::CmdKhai, elems))),
            "(khai)".to_string(),
        ))
    }
}
