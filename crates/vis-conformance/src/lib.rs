// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-313c (VD-289)** — harness conformance dữ liệu: mỗi ca là một tệp `.vis` (+ `.out`/
//! `.err`/`.exit`) chạy **cả** interp và VM, so byte-by-byte.
//!
//! Định dạng ca trong `test/conformance/**`:
//!
//! - `<tên>.vis` — nguồn chương trình (bắt buộc).
//! - `<tên>.out` — stdout **chính xác** (tuỳ chọn; thiếu ⇒ rỗng).
//! - `<tên>.err` — các mã chẩn đoán mong đợi, mỗi dòng một mã (`VS0301`…) (tuỳ chọn).
//! - `<tên>.exit` — mã thoát mong đợi (tuỳ chọn; thiếu ⇒ 0).
//!
//! Không dùng `Command`, không cần binary `vis`: chạy thẳng API thư viện.

use std::path::{Path, PathBuf};

use vis_core::RunOutput;

/// Một ca dữ liệu đã nạp.
#[derive(Debug, Clone)]
pub struct Case {
    pub ten: String,
    pub duong_dan: PathBuf,
    pub src: String,
    pub stdout: String,
    pub ma_loi: Vec<String>,
    pub exit: i32,
    /// `.err` có mặt ⇒ ca **lỗi tĩnh** (parse/lower/tyck phải chứa các mã này).
    pub loi_tinh: bool,
    /// `.fmt` có mặt ⇒ `vis_fmt` phải **bất động** trên nguồn (format 2 lần bằng 1 lần).
    pub kiem_fmt: bool,
    /// `.out_prefix` có mặt ⇒ stdout chỉ cần **bắt đầu bằng** nội dung này.
    pub out_prefix: Option<String>,
}

/// Gốc repo (thư mục chứa `test/conformance`).
pub fn goc_repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Nạp toàn bộ ca dưới `goc/conformance`, sắp theo đường dẫn để tất định.
pub fn nap_corpus(goc: &Path) -> Vec<Case> {
    let mut ds = Vec::new();
    duyet(goc, goc, &mut ds);
    ds.sort_by(|a, b| a.duong_dan.cmp(&b.duong_dan));
    ds
}

fn duyet(goc: &Path, dir: &Path, out: &mut Vec<Case>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            duyet(goc, &p, out);
        } else if p.extension().is_some_and(|x| x == "vis") {
            let src =
                std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("đọc {}: {e}", p.display()));
            let stdout = doc_phu(&p, "out").unwrap_or_default();
            let ma_loi = doc_phu(&p, "err")
                .unwrap_or_default()
                .lines()
                .map(str::trim)
                .filter(|l| !l.is_empty())
                .map(str::to_string)
                .collect();
            let exit = doc_phu(&p, "exit")
                .and_then(|s| s.trim().parse().ok())
                .unwrap_or(0);
            let loi_tinh = p.with_extension("err").exists();
            let kiem_fmt = p.with_extension("fmt").exists();
            let out_prefix = doc_phu(&p, "out_prefix");
            let ten = p
                .strip_prefix(goc)
                .unwrap_or(&p)
                .with_extension("")
                .to_string_lossy()
                .replace('\\', "/");
            out.push(Case {
                ten,
                duong_dan: p,
                src,
                stdout,
                ma_loi,
                exit,
                loi_tinh,
                kiem_fmt,
                out_prefix,
            });
        }
    }
}

fn doc_phu(p: &Path, ext: &str) -> Option<String> {
    let q = p.with_extension(ext);
    std::fs::read_to_string(q).ok()
}

/// Chạy một ca trên **cả** interp và VM, trả `(interp, vm)`.
pub fn chay_hai_engine(src: &str) -> (RunOutput, RunOutput) {
    let lex = vis_syntax::lex(src, vis_syntax::Mode::Script);
    let p = vis_syntax::parse(src, vis_syntax::Mode::Script);
    assert!(p.diagnostics.is_empty(), "parse lỗi: {:?}", p.diagnostics);
    // `use std.<mod>` là module Rust (không phải tệp) — nạp như loader CLI.
    let std_mods: Vec<String> = src
        .lines()
        .filter_map(|l| l.trim().strip_prefix("use std."))
        .map(|m| m.trim().trim_end_matches(';').to_string())
        .filter(|m| vis_hir::stdlib::module_ho_tro(m))
        .collect();
    let opts = vis_types::LowerOptions {
        std_mods,
        ..Default::default()
    };
    let h = vis_types::lower_with(&p.cst, &lex, opts);
    assert!(h.is_clean(), "lower lỗi: {:?}", h.diagnostics);
    let t = vis_types::check(&h.hir, &h.symbols, &h.types);
    assert!(t.is_clean(), "tyck lỗi: {:?}", t.diags);
    let i = vis_interp::run(src, &h.hir, &h.symbols, &h.types, &[]);
    let m = vis_bytecode::compile(&h.hir, &h.symbols, src)
        .unwrap_or_else(|e| panic!("compile lỗi: {e:?}"));
    let v = vis_vm::run(&m, src, &[]);
    (i, v)
}

/// Khẳng định một ca: lỗi tĩnh (` .err`) hoặc chạy hai engine (` .out`).
pub fn khang_dinh(case: &Case) {
    if case.loi_tinh {
        khang_dinh_loi_tinh(case);
        return;
    }
    let (i, v) = chay_hai_engine(&case.src);
    let ma_i: Vec<&str> = i.codes();
    let ma_v: Vec<&str> = v.codes();
    assert_eq!(
        (i.exit, &i.stdout, ma_i.clone()),
        (v.exit, &v.stdout, ma_v.clone()),
        "{}: interp ↔ VM lệch\ninterp: exit={} stdout={:?} codes={:?}\nvm: exit={} stdout={:?} codes={:?}",
        case.ten, i.exit, i.stdout, ma_i, v.exit, v.stdout, ma_v
    );
    assert_eq!(i.exit, case.exit, "{}: exit sai", case.ten);
    match &case.out_prefix {
        Some(pre) => assert!(
            i.stdout.starts_with(pre),
            "{}: stdout phải bắt đầu bằng {:?}, được {:?}",
            case.ten,
            pre,
            i.stdout
        ),
        None => assert_eq!(i.stdout, case.stdout, "{}: stdout sai", case.ten),
    }
    let mong: Vec<&str> = case.ma_loi.iter().map(String::as_str).collect();
    assert_eq!(ma_i, mong, "{}: mã chẩn đoán sai", case.ten);
    if case.kiem_fmt {
        let a = vis_fmt::format(&case.src).expect("fmt lần 1");
        let b = vis_fmt::format(&a).expect("fmt lần 2");
        assert_eq!(a, b, "{}: format không bất động", case.ten);
    }
}

/// Ca lỗi tĩnh: `parse`+`lower`+`tyck` phải chứa **lần lượt** các mã trong `.err`.
fn khang_dinh_loi_tinh(case: &Case) {
    let lex = vis_syntax::lex(&case.src, vis_syntax::Mode::Script);
    let p = vis_syntax::parse(&case.src, vis_syntax::Mode::Script);
    let mut ma: Vec<&str> = p.diagnostics.iter().map(|d| d.code.short).collect();
    let std_mods: Vec<String> = case
        .src
        .lines()
        .filter_map(|l| l.trim().strip_prefix("use std."))
        .map(|m| m.trim().trim_end_matches(';').to_string())
        .filter(|m| vis_hir::stdlib::module_ho_tro(m))
        .collect();
    let h = vis_types::lower_with(
        &p.cst,
        &lex,
        vis_types::LowerOptions {
            std_mods,
            ..Default::default()
        },
    );
    ma.extend(h.diagnostics.iter().map(|d| d.code.short));
    let t = vis_types::check(&h.hir, &h.symbols, &h.types);
    ma.extend(t.diags.iter().map(|d| d.code.short));
    let mut idx = 0;
    for mong in &case.ma_loi {
        let Some(pos) = ma[idx..].iter().position(|m| m == mong) else {
            panic!(
                "{}: thiếu mã `{mong}` trong {:?} (đã thấy {:?})",
                case.ten,
                ma,
                &ma[..idx.min(ma.len())]
            );
        };
        idx += pos + 1;
    }
}
