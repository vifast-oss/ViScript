// SPDX-License-Identifier: MIT OR Apache-2.0
// **ST-276 (VD-256)**: `F` (bộ phát lệnh của một hàm) tách sang tệp riêng để giữ R1 ≤ 1500
// dòng cho `codegen.rs`. Nội dung chuyển nguyên văn, chỉ mở visibility cho module cha/submodule.
use std::collections::HashMap;

use vis_syntax::Span;

use crate::cerr::CodegenError;
use crate::fee::{FeeEntry, FeeKind};
use crate::format::{Func, Instr};

/// Hàm đang phát.
pub(crate) struct F {
    pub(crate) name: String,
    pub(crate) arity: u16,
    pub(crate) nparams: u16,
    pub(crate) nlocals: u16,
    pub(crate) code: Vec<Instr>,
    pub(crate) spans: Vec<crate::format::Span>,
    /// **ST-276 (VD-256)** — bảng phụ S: sự kiện ngữ nghĩa theo PC, **có thứ tự**.
    pub(crate) fee: Vec<FeeEntry>,
    /// `sym` → ô cục bộ.
    pub(crate) locals: HashMap<u32, u16>,
    pub(crate) cur: i32,
    pub(crate) max: i32,
}

impl F {
    pub(crate) fn new(name: String) -> Self {
        Self {
            name,
            arity: 0,
            nparams: 0,
            nlocals: 0,
            code: Vec::new(),
            spans: Vec::new(),
            fee: Vec::new(),
            locals: HashMap::new(),
            cur: 0,
            max: 0,
        }
    }

    pub(crate) fn slot(&mut self, sym: u32, span: Span) -> Result<u16, CodegenError> {
        if let Some(s) = self.locals.get(&sym) {
            return Ok(*s);
        }
        let s = self.nlocals;
        if s == u16::MAX {
            return Err(CodegenError::TooMany(s));
        }
        self.nlocals += 1;
        self.locals.insert(sym, s);
        let _ = span;
        Ok(s)
    }

    pub(crate) fn push(&mut self, ins: Instr, span: Span, dang: i32) {
        self.code.push(ins);
        self.spans.push(crate::format::Span {
            start: span.start as u32,
            end: span.end as u32,
        });
        // Ước lượng trên cho `max_stack`; verifier ST-21 kiểm độ sâu thật từ byte stream.
        self.cur = (self.cur + dang).max(0);
        if self.cur > self.max {
            self.max = self.cur;
        }
    }

    /// **ST-276 (VD-256)**: ghi một sự kiện ngữ nghĩa tại PC hiện tại.
    pub(crate) fn charge(&mut self, kind: FeeKind, span: Span) {
        self.fee
            .push(FeeEntry::new(self.code.len(), kind, span.start as u32));
    }

    /// Vị trí lệnh kế (để tính đích nhảy tương đối).
    pub(crate) fn here(&self) -> usize {
        self.code.len()
    }

    pub(crate) fn patch(&mut self, at: usize, target: usize) {
        // Quy ước: `đích = chỉ_số_lệnh_nhảy + offset` (VM: `pc_sau_khi_đọc - 1 + off`).
        let off = (target as i64 - at as i64) as i32;
        if let Some(ins) = self.code.get_mut(at) {
            *ins = match *ins {
                Instr::Jump(_) => Instr::Jump(off),
                Instr::JumpIfFalse(_) => Instr::JumpIfFalse(off),
                Instr::JumpIfNotTrue(_) => Instr::JumpIfNotTrue(off),
                Instr::JumpIfNone(_) => Instr::JumpIfNone(off),
                Instr::JumpIfSome(_) => Instr::JumpIfSome(off),
                Instr::IterNext(_) => Instr::IterNext(off),
                other => other,
            };
        }
    }

    /// Chốt hàm: `max_stack` lấy từ **chính** hàm mô phỏng của verifier (một nguồn sự thật).
    /// Trả kèm bảng phụ S để `compile_with_fee` đóng vào trailer artifact.
    pub(crate) fn finish(self) -> Result<(Func, Vec<FeeEntry>), CodegenError> {
        if std::env::var("VBC_DBG").is_ok() {
            for (i, ins) in self.code.iter().enumerate() {
                eprintln!("{i:3} {ins:?}");
            }
        }
        let need = crate::verify::need_stack(&self.code, 0).map_err(CodegenError::KhongHopLe)?;
        let fee = self.fee;
        Ok((
            Func {
                name: self.name,
                arity: self.arity,
                nparams: self.nparams,
                nlocals: self.nlocals.max(1),
                max_stack: need,
                code: self.code,
                spans: self.spans,
            },
            fee,
        ))
    }
}
