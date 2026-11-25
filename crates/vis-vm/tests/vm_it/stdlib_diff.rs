// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-314a hậu-kiểm (low-peak 2026-11-25)** — differential **stdlib thuần** interp ↔ VM.
//!
//! Sinh chương trình **tất định** gọi các hàm `std.*` **thuần** (chỉ `str`/`list`/`path`/`json`)
//! với đầu vào biên (chuỗi Unicode/rỗng, số âm, list rỗng, map, JSON hỏng…), chạy **cả hai
//! engine** và so **byte-by-byte**: `stdout`, `exit`, và toàn bộ `(mã, dòng, cột, thông điệp)`
//! của chẩn đoán. Khác `test/conformance/**` (mỗi hàm một vài ca viết tay), phép diff này sinh
//! **tổ hợp ngẫu nhiên nhưng tất định** để bắt lệch ở đầu vào ít gặp.
//!
//! Số chương trình: `VIS_STDLIB_DIFF_N` (mặc định **400** cho cổng; đặt lớn khi chạy tay).
//! Hàm có tham số generic/closure (`Var`, `FnKhoa`) hoặc kiểu host (`Error`/`Json`/`SqlValue`…)
//! bị **bỏ qua có ghi nhận** — chúng thuộc nhóm cần dựng host, không phải stdlib thuần.

use std::panic::{catch_unwind, AssertUnwindSafe};

use vis_core::RunOutput;
use vis_hir::stdlib::{Ty, FUNCS};

/// Số chương trình — mặc định 400; `VIS_STDLIB_DIFF_N` để mở rộng khi chạy tay.
fn so_chuong_trinh() -> usize {
    std::env::var("VIS_STDLIB_DIFF_N")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(400)
}

/// LCG tất định (cùng họ với `diff_100k.rs`/fuzz) — cùng seed ⇒ cùng chương trình.
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0 >> 11
    }
    fn duoi(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next() % n as u64) as usize
        }
    }
    fn chon<'a, T>(&mut self, ds: &'a [T]) -> &'a T {
        &ds[self.duoi(ds.len())]
    }
}

/// Bảng đầu vào biên. Chuỗi cố ý gồm Unicode nhiều byte, rỗng, khoảng trắng, dấu phân cách,
/// literal số/JSON để `parse_*`/`json.decode` chạm cả đường lỗi lẫn đường đúng.
const STRS: [&str; 24] = [
    "",
    "a",
    "abc",
    "a,b,,c",
    " đ ",
    "đ😀x",
    "\n",
    "a\nb\r\nc",
    "   ",
    ",",
    ".",
    "/",
    "a/b/../c",
    "ABC",
    "ß",
    "İ",
    "ΣΣ",
    "0",
    "042",
    "-7",
    "1.5",
    "1e3",
    "{\"a\":1}",
    "[1,2,3]",
];

const INTS: [i64; 10] = [-10, -3, -1, 0, 1, 2, 3, 5, 7, 10];

const FLOATS: [&str; 8] = [
    "0.0", "-0.0", "1.0", "1.5", "-2.5", "0.1", "1e300", "1e-300",
];

fn lit_str(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            // Chuỗi ViScript nội suy `{…}` ⇒ `{`/`}` literal phải nhân đôi (như `{{`/`}}`).
            '{' => out.push_str("{{"),
            '}' => out.push_str("}}"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Sinh một biểu thức literal cho `ty`. `None` ⇒ kiểu không sinh được (generic/host).
fn sinh(rng: &mut Lcg, ty: Ty) -> Option<String> {
    match ty {
        Ty::Int => Some(rng.chon(&INTS).to_string()),
        Ty::Str => Some(lit_str(rng.chon(&STRS))),
        Ty::Bool => Some(if rng.duoi(2) == 0 { "true" } else { "false" }.to_string()),
        Ty::Float => Some((*rng.chon(&FLOATS)).to_string()),
        Ty::ListStr => {
            let n = rng.duoi(5);
            let mut ds = Vec::with_capacity(n);
            for _ in 0..n {
                ds.push(lit_str(rng.chon(&STRS)));
            }
            Some(format!("[{}]", ds.join(", ")))
        }
        Ty::List(t) => {
            let n = rng.duoi(5);
            let mut ds = Vec::with_capacity(n);
            for _ in 0..n {
                ds.push(sinh(rng, *t)?);
            }
            Some(format!("[{}]", ds.join(", ")))
        }
        Ty::Map(k, v) => {
            // Khoá **khác nhau** để literal map không trùng khoá (tránh lỗi tĩnh không liên quan).
            let n = rng.duoi(4);
            let mut ds = Vec::with_capacity(n);
            for i in 0..n {
                let kk = match *k {
                    Ty::Str => lit_str(&format!("{}{}", STRS[i % STRS.len()], i)),
                    _ => sinh(rng, *k)?,
                };
                let vv = sinh(rng, *v)?;
                ds.push(format!("{kk}: {vv}"));
            }
            Some(format!("{{{}}}", ds.join(", ")))
        }
        // Generic/closure/host: không sinh (xem đầu tệp).
        _ => None,
    }
}

/// Hàm **thuần** và **sinh được đối số**: loại `fs`/`net`/`http`/`sqlite`/`time` (host/đồng hồ)
/// và mọi chữ ký có `Var`/`FnKhoa`/`Error`/`Json`/`SqlValue`/`RowSet`/`Result*`/`Opt*`.
fn thuan_va_sinh_duoc(f: &vis_hir::stdlib::StdFn) -> bool {
    let m = f.name.split('.').next().unwrap_or("");
    if !matches!(m, "str" | "list" | "path" | "json") {
        return false;
    }
    f.params.iter().all(|t| sinh_duoc(*t))
}

/// Kiểu **sinh literal được** (đệ quy) — khớp đúng nhánh của [`sinh`].
fn sinh_duoc(t: Ty) -> bool {
    match t {
        Ty::Int | Ty::Str | Ty::Bool | Ty::Float | Ty::ListStr => true,
        Ty::List(x) => sinh_duoc(*x),
        Ty::Map(k, v) => sinh_duoc(*k) && sinh_duoc(*v),
        _ => false,
    }
}

fn std_mods(src: &str) -> Vec<String> {
    src.lines()
        .filter_map(|l| l.trim().strip_prefix("use std."))
        .map(|m| m.trim().trim_end_matches(';').to_string())
        .filter(|m| vis_hir::stdlib::module_ho_tro(m))
        .collect()
}

/// Chạy một nguồn trên cả hai engine; `None` nếu frontend không sạch (chương trình sinh bị bỏ).
fn chay_hai(src: &str) -> Option<(RunOutput, RunOutput)> {
    let lex = vis_syntax::lex(src, vis_syntax::Mode::Script);
    let p = vis_syntax::parse(src, vis_syntax::Mode::Script);
    if !p.diagnostics.is_empty() {
        return None;
    }
    let h = vis_types::lower_with(
        &p.cst,
        &lex,
        vis_types::LowerOptions {
            std_mods: std_mods(src),
            ..Default::default()
        },
    );
    if !h.is_clean() {
        return None;
    }
    let t = vis_types::check(&h.hir, &h.symbols, &h.types);
    if !t.is_clean() {
        return None;
    }
    let i = vis_interp::run(src, &h.hir, &h.symbols, &h.types, &[]);
    let m = vis_bytecode::compile(&h.hir, &h.symbols, src).ok()?;
    let v = vis_vm::run(&m, src, &[]);
    Some((i, v))
}

fn mo_ta(o: &RunOutput) -> String {
    let d: Vec<String> = o
        .diagnostics
        .iter()
        .map(|x| format!("{}:{}:{} {}", x.code_str(), x.line, x.col, x.message))
        .collect();
    format!("stdout={:?} exit={} diag={:?}", o.stdout, o.exit, d)
}

#[test]
fn stdlib_thuan_khop_interp_vm() {
    let n = so_chuong_trinh();
    let ds: Vec<&vis_hir::stdlib::StdFn> = FUNCS.iter().filter(|f| thuan_va_sinh_duoc(f)).collect();
    assert!(
        ds.len() >= 60,
        "chỉ còn {} hàm stdlib thuần sinh được — bảng chữ ký đổi bất thường?",
        ds.len()
    );
    let mut rng = Lcg(0x5EED_2026_1125);
    let mut da_chay = 0usize;
    let mut bo_qua = 0usize;
    let mut bo_qua_ten: Vec<&str> = Vec::new();
    let mut lech: Vec<String> = Vec::new();
    for i in 0..n {
        let f = ds[i % ds.len()];
        let Some(args) = f
            .params
            .iter()
            .map(|t| sinh(&mut rng, *t))
            .collect::<Option<Vec<_>>>()
        else {
            bo_qua += 1;
            continue;
        };
        let module = f.name.split('.').next().unwrap_or("");
        let src = format!(
            "use std.{module}\n\nfn main():\n    print({}({}))\n",
            f.name,
            args.join(", ")
        );
        let kq = catch_unwind(AssertUnwindSafe(|| chay_hai(&src)));
        match kq {
            Err(_) => lech.push(format!("#{i} PANIC {} \n{src}", f.name)),
            Ok(None) => {
                bo_qua += 1;
                bo_qua_ten.push(f.name);
                if std::env::var_os("VIS_STDLIB_DIFF_DEBUG").is_some() && bo_qua_ten.len() <= 3 {
                    eprintln!("BỎ QUA #{i} {}:\n{src}", f.name);
                }
            }
            Ok(Some((a, b))) => {
                da_chay += 1;
                if mo_ta(&a) != mo_ta(&b) {
                    lech.push(format!(
                        "#{i} LỆCH {}\ninterp: {}\nvm    : {}\n{src}",
                        f.name,
                        mo_ta(&a),
                        mo_ta(&b)
                    ));
                }
            }
        }
    }
    eprintln!(
        "stdlib_diff: {} hàm thuần, {n} chương trình, {da_chay} chạy được, {bo_qua} bỏ qua (frontend), {} lệch",
        ds.len(),
        lech.len()
    );
    if std::env::var_os("VIS_STDLIB_DIFF_DEBUG").is_some() {
        bo_qua_ten.sort_unstable();
        bo_qua_ten.dedup();
        eprintln!("bỏ qua theo hàm: {bo_qua_ten:?}");
    }
    assert!(
        lech.is_empty(),
        "{} chương trình lệch/panic (in tối đa 5):\n{}",
        lech.len(),
        lech.iter().take(5).cloned().collect::<Vec<_>>().join("\n")
    );
    assert!(
        da_chay > n / 2,
        "quá nhiều chương trình bị bỏ ({da_chay}/{n})"
    );
}

/// **ST-314a hậu-kiểm (low-peak 2026-11-25)** — differential dạng **phương thức**
/// (`recv.f(rest)`): cùng 73 hàm thuần, nhưng callee là `Field` ⇒ đi opcode `CallMethod`/nhánh
/// `f(a) ≡ std.<mod>.f(recv, a)` thay vì `CallBuiltin`. Sinh **tất định**; chương trình mà frontend
/// từ chối (hàm không có dạng phương thức) bị bỏ qua và đếm riêng.
///
/// Số chương trình: `VIS_STDLIB_SUGAR_N` (mặc định **200** cho cổng).
#[test]
fn stdlib_thuan_dang_phuong_thuc_khop_interp_vm() {
    let n = std::env::var("VIS_STDLIB_SUGAR_N")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(200);
    let ds: Vec<&vis_hir::stdlib::StdFn> = FUNCS
        .iter()
        .filter(|f| thuan_va_sinh_duoc(f))
        .filter(|f| {
            matches!(
                f.params.first(),
                Some(Ty::Str | Ty::ListStr | Ty::List(_) | Ty::Map(_, _))
            )
        })
        .collect();
    assert!(
        ds.len() >= 30,
        "chỉ còn {} hàm có dạng phương thức — bảng chữ ký đổi bất thường?",
        ds.len()
    );
    let mut rng = Lcg(0x5EED_2026_1126);
    let mut da_chay = 0usize;
    let mut bo_qua = 0usize;
    let mut lech: Vec<String> = Vec::new();
    for i in 0..n {
        let f = ds[i % ds.len()];
        let module = f.name.split('.').next().unwrap_or("");
        let ten_ngan = f.name.split('.').nth(1).unwrap_or("");
        let Some((recv, rest)) = f.params.split_first() else {
            continue;
        };
        let Some(recv_src) = sinh(&mut rng, *recv) else {
            bo_qua += 1;
            continue;
        };
        let Some(args) = rest
            .iter()
            .map(|t| sinh(&mut rng, *t))
            .collect::<Option<Vec<_>>>()
        else {
            bo_qua += 1;
            continue;
        };
        let src = format!(
            "use std.{module}\n\nfn main():\n    print({recv_src}.{ten_ngan}({}))\n",
            args.join(", ")
        );
        let kq = catch_unwind(AssertUnwindSafe(|| chay_hai(&src)));
        match kq {
            Err(_) => lech.push(format!("#{i} PANIC {} \n{src}", f.name)),
            Ok(None) => bo_qua += 1,
            Ok(Some((a, b))) => {
                da_chay += 1;
                if mo_ta(&a) != mo_ta(&b) {
                    lech.push(format!(
                        "#{i} LỆCH {} (dạng phương thức)\ninterp: {}\nvm    : {}\n{src}",
                        f.name,
                        mo_ta(&a),
                        mo_ta(&b)
                    ));
                }
            }
        }
    }
    eprintln!(
        "stdlib_sugar: {} hàm, {n} chương trình, {da_chay} chạy được, {bo_qua} bỏ qua (frontend), {} lệch",
        ds.len(),
        lech.len()
    );
    assert!(
        lech.is_empty(),
        "{} chương trình lệch/panic (tối đa 5):\n{}",
        lech.len(),
        lech.iter().take(5).cloned().collect::<Vec<_>>().join("\n")
    );
    assert!(
        da_chay > n / 4,
        "quá nhiều chương trình dạng phương thức bị bỏ ({da_chay}/{n})"
    );
}
