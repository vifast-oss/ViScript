// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-299a (VD-277)** — pipeline **một tệp** dùng chung cho binary `vis-wasm-run` và test native.
//!
//! `parse → lower → tyck → (compile) → interp | VM`, cắm [`WasiHost`] vào cả hai đường. Không
//! cache, không module đa tệp (ST-299b mới nối `vis-loader`), không `std::thread` (WASI không có
//! luồng) — gọi runtime trực tiếp trên luồng hiện tại.
//!
//! Lỗi front end ⇒ exit **2**; lỗi codegen ⇒ **1**; trap runtime `VS05xx` ⇒ **101**; từ chối preopen
//! `VS0604` của host ⇒ **2** (cùng quy ước quyền của CLI); còn lại theo `RunOutput.exit`.

use vis_diag::Severity;
use vis_syntax::{Diagnostic, NodeKind};
use vis_types::hir::HItem;

use crate::{Preopen, WasiHost};

/// Đường chạy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Engine {
    /// Thông dịch cây HIR.
    Interp,
    /// Biên dịch VBC1 rồi chạy VM.
    Vm,
}

/// Yêu cầu một lần chạy.
#[derive(Debug, Clone)]
pub struct YeuCau {
    /// Toàn văn nguồn (đã đọc từ tệp/stdin).
    pub nguon: String,
    /// Tên hiển thị trong chẩn đoán (`-` cho stdin).
    pub ten: String,
    /// Chế độ parse (`Script`/`Command`).
    pub mode: vis_syntax::Mode,
    /// Đối số chương trình (`main`/`args`).
    pub args: Vec<String>,
    /// Đường chạy.
    pub engine: Engine,
    /// Preopen cấp cho `std.fs`.
    pub preopens: Vec<Preopen>,
}

/// Kết quả một lần chạy (stdout/stderr đã tách để test tất định).
#[derive(Debug, Clone)]
pub struct KetQua {
    /// Toàn bộ stdout chương trình (`print`).
    pub stdout: String,
    /// Chẩn đoán + lỗi host của runner.
    pub stderr: String,
    /// Mã thoát đã ánh xạ.
    pub exit: u8,
    /// Số bước ngữ nghĩa/lệnh.
    pub steps: usize,
    /// Đường đã chạy.
    pub engine: Engine,
}

impl KetQua {
    /// `true` nếu sạch (exit 0, không chẩn đoán).
    pub fn sach(&self) -> bool {
        self.exit == 0 && self.stderr.is_empty()
    }
}

fn in_chan_doan(ra: &mut String, ten: &str, ds: &[Diagnostic]) -> bool {
    let mut co_loi = false;
    for d in ds {
        let nhan = match d.code.severity {
            Severity::Error => {
                co_loi = true;
                "error"
            }
            _ => "warning",
        };
        ra.push_str(&format!(
            "{ten}:{}:{}: {nhan} {}: {}\n",
            d.line, d.col, d.code.short, d.message
        ));
    }
    co_loi
}

/// HIR có `async fn` (hoặc phương thức async của `impl`/`trait`) ⇒ dùng frame scheduler.
fn co_async(h: &vis_types::Hir) -> bool {
    h.items.iter().any(|it| match it {
        HItem::Fn(f) => f.is_async,
        HItem::Impl(i) => i.methods.iter().any(|f| f.is_async),
        HItem::Trait(t) => t.methods.iter().any(|f| f.is_async),
        _ => false,
    })
}

/// Các module `std.*` đã `use` (cùng luật CLI) — cần cho `LowerOptions::std_mods`.
fn std_mods(cst: &vis_syntax::Cst, text: &str) -> Vec<String> {
    let mut ra: Vec<String> = Vec::new();
    for n in cst.root.children() {
        if n.kind != NodeKind::UseDecl {
            continue;
        }
        let Some(dp) = n.child(NodeKind::DottedPath) else {
            continue;
        };
        let raw = text.get(dp.span.start..dp.span.end).unwrap_or("").trim();
        let seg: Vec<&str> = if raw.contains('/') {
            vec![raw]
        } else {
            raw.split('.').collect()
        };
        if seg.first() == Some(&"std") {
            if let Some(m2) = seg.get(1) {
                if vis_types::stdlib::module_ho_tro(m2) && !ra.iter().any(|x| x == m2) {
                    ra.push((*m2).to_string());
                }
            }
        }
    }
    ra
}

fn ma_thoat(out: &vis_interp::RunOutput) -> u8 {
    if out.diagnostics.is_empty() {
        return (out.exit & 0xff) as u8;
    }
    // Từ chối preopen của host là lỗi **quyền** (exit 2), không phải lỗi chương trình.
    if out
        .diagnostics
        .iter()
        .any(|d| d.message.contains("wasm preopen"))
    {
        return 2;
    }
    if out
        .diagnostics
        .iter()
        .any(|d| d.code.short.starts_with("VS05"))
    {
        101
    } else {
        1
    }
}

/// **ST-299b** — bỏ `main` khi chạy `.vish` có câu lệnh cấp tệp (cùng luật CLI).
fn loc_main(hir: &mut vis_types::Hir, mode: vis_syntax::Mode) {
    if mode == vis_syntax::Mode::Command
        && !hir.stmts.is_empty()
        && hir
            .items
            .iter()
            .any(|it| matches!(it, HItem::Fn(f) if f.name == "main"))
    {
        hir.items
            .retain(|it| !matches!(it, HItem::Fn(f) if f.name == "main"));
    }
}

/// Chạy HIR đã frontend + tyck với host WASI.
#[allow(clippy::too_many_arguments)]
fn chay_hir(
    ten: &str,
    nguon: &str,
    hir: &vis_types::Hir,
    symbols: &vis_types::Symbols,
    types: &vis_types::TypeTable,
    args: &[String],
    engine: Engine,
    preopens: &[Preopen],
) -> KetQua {
    let mut stderr = String::new();
    let mut host = match WasiHost::moi(preopens) {
        Ok(h) => h,
        Err(e) => {
            stderr.push_str(&format!("error host: {e}\n"));
            return KetQua {
                stdout: String::new(),
                stderr,
                exit: 2,
                steps: 0,
                engine,
            };
        }
    };
    let tp = host.tach();
    let la_async = co_async(hir);
    let out = match engine {
        Engine::Interp => {
            if la_async {
                vis_interp::run_async_with_hosts_caps(
                    nguon,
                    hir,
                    symbols,
                    types,
                    args,
                    Some(tp.proc),
                    Some(tp.fs),
                    Some(tp.net),
                    Some(tp.db),
                    &[],
                )
            } else {
                vis_interp::run_with_hosts_caps(
                    nguon,
                    hir,
                    symbols,
                    types,
                    args,
                    Some(tp.proc),
                    Some(tp.fs),
                    Some(tp.net),
                    Some(tp.db),
                    &[],
                )
            }
        }
        Engine::Vm => match vis_bytecode::compile_voi_bang(hir, symbols, Some(types), nguon) {
            Ok(m) => {
                if la_async {
                    vis_vm::run_async_with_full_hosts_caps(
                        &m,
                        nguon,
                        args,
                        Some(tp.proc),
                        Some(tp.fs),
                        Some(tp.net),
                        Some(tp.db),
                        &[],
                    )
                } else {
                    vis_vm::run_with_full_hosts_caps(
                        &m,
                        nguon,
                        args,
                        Some(tp.proc),
                        Some(tp.fs),
                        Some(tp.net),
                        Some(tp.db),
                        &[],
                    )
                }
            }
            Err(e) => {
                stderr.push_str(&format!("codegen lỗi: {e}\n"));
                return KetQua {
                    stdout: String::new(),
                    stderr,
                    exit: 1,
                    steps: 0,
                    engine,
                };
            }
        },
    };
    for d in &out.diagnostics {
        stderr.push_str(&format!(
            "{ten}:{}:{}: error {}: {}\n",
            d.line, d.col, d.code.short, d.message
        ));
    }
    let exit = ma_thoat(&out);
    KetQua {
        stdout: out.stdout,
        stderr,
        exit,
        steps: out.steps,
        engine,
    }
}

/// Chạy **một nguồn** với host WASI (đường stdin/test một tệp — giữ nguyên).
pub fn chay(yc: &YeuCau) -> KetQua {
    let mut stderr = String::new();
    let parsed = vis_syntax::parse(&yc.nguon, yc.mode);
    if in_chan_doan(&mut stderr, &yc.ten, &parsed.diagnostics) {
        return KetQua {
            stdout: String::new(),
            stderr,
            exit: 2,
            steps: 0,
            engine: yc.engine,
        };
    }
    let lexed = vis_syntax::lex(&yc.nguon, yc.mode);
    let opts = vis_types::LowerOptions {
        std_mods: std_mods(&parsed.cst, &parsed.text),
        ..Default::default()
    };
    let mut h = vis_types::lower_with(&parsed.cst, &lexed, opts);
    if in_chan_doan(&mut stderr, &yc.ten, &h.diagnostics) {
        return KetQua {
            stdout: String::new(),
            stderr,
            exit: 2,
            steps: 0,
            engine: yc.engine,
        };
    }
    if let Some(e) = &h.error {
        stderr.push_str(&format!("{}: error hạ: {e:?}\n", yc.ten));
        return KetQua {
            stdout: String::new(),
            stderr,
            exit: 2,
            steps: 0,
            engine: yc.engine,
        };
    }
    loc_main(&mut h.hir, yc.mode);
    let t = vis_types::check(&h.hir, &h.symbols, &h.types);
    if in_chan_doan(&mut stderr, &yc.ten, &t.diags) {
        return KetQua {
            stdout: String::new(),
            stderr,
            exit: 2,
            steps: 0,
            engine: yc.engine,
        };
    }
    chay_hir(
        &yc.ten,
        &yc.nguon,
        &h.hir,
        &h.symbols,
        &h.types,
        &yc.args,
        yc.engine,
        &yc.preopens,
    )
}

/// **ST-299b** — chạy tệp chính **đa tệp** qua [`vis_loader::modules::load_full`].
///
/// `use ./x` được nạp theo thư mục tệp chính; lỗi thiếu tệp/vòng/kiểu trả `exit=2` kèm tên
/// tệp. Mọi đọc tệp đi qua `std::fs` (trên WASI: preopen của runtime), không cache/thread.
pub fn chay_tep(tep: &str, engine: Engine, args: &[String], preopens: &[Preopen]) -> KetQua {
    let mode = if tep.ends_with(".vish") {
        vis_syntax::Mode::Command
    } else {
        vis_syntax::Mode::Script
    };
    let loaded = match vis_loader::modules::load_full(tep, true, true) {
        Ok(l) => l,
        Err(e) => {
            return KetQua {
                stdout: String::new(),
                stderr: format!("{tep}: error nạp: {e}\n"),
                exit: 2,
                steps: 0,
                engine,
            };
        }
    };
    let mut stderr = String::new();
    let mut co_loi = false;
    for (rel, d) in &loaded.diags {
        let nhan = match d.code.severity {
            Severity::Error => {
                co_loi = true;
                "error"
            }
            _ => "warning",
        };
        stderr.push_str(&format!(
            "{rel}:{}:{}: {nhan} {}: {}\n",
            d.line, d.col, d.code.short, d.message
        ));
    }
    if co_loi {
        return KetQua {
            stdout: String::new(),
            stderr,
            exit: 2,
            steps: 0,
            engine,
        };
    }
    let nguon = loaded
        .tep_chinh()
        .map(|f| f.text.clone())
        .unwrap_or_default();
    let mut hir = loaded.hir;
    loc_main(&mut hir, mode);
    let mut kq = chay_hir(
        tep,
        &nguon,
        &hir,
        &loaded.symbols,
        &loaded.types,
        args,
        engine,
        preopens,
    );
    if !kq.stderr.is_empty() {
        kq.stderr = format!("{stderr}{}", kq.stderr);
    } else {
        kq.stderr = stderr;
    }
    kq
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chay_nguon(src: &str, engine: Engine) -> KetQua {
        chay(&YeuCau {
            nguon: src.to_string(),
            ten: "test.vis".to_string(),
            mode: vis_syntax::Mode::Script,
            args: Vec::new(),
            engine,
            preopens: Vec::new(),
        })
    }

    #[test]
    fn in_so_ca_hai_duong_khop() {
        let src = "print(1 + 2)\nprint(\"xin chào\")\n";
        let a = chay_nguon(src, Engine::Interp);
        let b = chay_nguon(src, Engine::Vm);
        assert_eq!(a.stdout, "3\nxin chào\n", "interp: {}", a.stderr);
        assert_eq!(b.stdout, a.stdout, "VM lệch interp: {}", b.stderr);
        assert_eq!(a.exit, 0);
        assert_eq!(b.exit, 0);
    }

    #[test]
    fn loi_cu_phap_exit_2() {
        let kq = chay_nguon("fn main(:\n", Engine::Interp);
        assert_eq!(kq.exit, 2, "{}", kq.stderr);
        assert!(kq.stderr.contains("error"), "{}", kq.stderr);
    }

    #[test]
    fn thieu_preopen_thi_std_fs_bi_tu_choi() {
        let src = "use std.fs\nfn main():\n    match fs.read(\"/data/x\"):\n        Ok(s):\n            print(s)\n        Err(e):\n            print(\"CHAN\")\n";
        let kq = chay_nguon(src, Engine::Interp);
        // Thiếu preopen ⇒ host trả `DAU_QUYEN` ⇒ trap VS0604, không phải Err dữ liệu.
        assert_eq!(kq.exit, 2, "stdout={:?} stderr={}", kq.stdout, kq.stderr);
        assert!(kq.stderr.contains("VS0604"), "{}", kq.stderr);
        assert_eq!(kq.stdout, "");
    }

    #[test]
    fn tu_choi_net_ro_rang() {
        let src = "use std.net\nfn main():\n    match net.connect(\"127.0.0.1:9\"):\n        Ok(t):\n            print(\"NO\")\n        Err(e):\n            print(\"CHAN\")\n";
        for engine in [Engine::Interp, Engine::Vm] {
            let kq = chay_nguon(src, engine);
            assert_eq!(kq.exit, 0, "{:?}: {}", engine, kq.stderr);
            assert_eq!(kq.stdout, "CHAN\n", "{:?}: {}", engine, kq.stderr);
        }
    }
}
