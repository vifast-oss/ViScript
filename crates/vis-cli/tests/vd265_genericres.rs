// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-265 — residual generic instantiation**: bốn chỗ còn rơi instantiation sau VD-263.
//!
//! VD-263 đã nâng `T?` ở biên dựng/đọc/ghi của **constructor trực tiếp** khi biết kiểu tại chỗ
//! (`let b: Box[int?] = Box(v: 5)`). Bốn đường sau vẫn mất instantiation vì kiểu mong đợi không
//! tới được biểu thức con:
//!
//! 1. **Callee khai sau call-site** — `fn main(): nhan(Box(v: 5))` trước
//!    `fn nhan(b: Box[int?])`: `fn_params` chỉ có hàm **đã hạ** ⇒ in `5` (đúng: `Some(5)`).
//! 2. **`if`/`match` ở đuôi thân** — `fn tao(ok: bool) -> Box[int?]: if ok: Box(v: 5) else: …`
//!    in `5`/`6` (đúng: `Some(5)`/`Some(6)`). Câu cuối dạng **câu lệnh** không nhận kiểu trả về.
//! 3. **Constructor lồng trong trường literal ẩn danh** — `let w: Wrap[Box[int?]] = {inner: Box(v: 5)}`
//!    in `5` (đúng: `Some(5)`): literal hạ giá trị trường trước khi biết instantiation.
//! 4. **RHS của `Field`/`Index` + gọi phương thức/closure/giá trị hàm** — `g.f = Box(v: 5)`,
//!    `xs[0] = Box(v: 5)`, `h.lay(Box(v: 5))`, `let f = fn(b: Box[int?])` rồi `f(Box(v: 5))`,
//!    `let f = nhan` (hàm khai sau) rồi `f(Box(v: 5))` đều in `5` (đúng: `Some(5)`).
//!
//! Hợp đồng khoá bởi tệp này:
//! 1. Pre-pass **chữ ký** trước khi hạ thân: template `type`/`enum` generic + `fn_params`/`fn_ret`
//!    của **mọi** hàm/phương thức ⇒ gọi hàm khai **sau** vẫn mang instantiation.
//! 2. Câu cuối dạng `if`/`match`/khối của thân hàm nhận kiểu trả về làm kiểu mong đợi.
//! 3. Literal bản ghi ẩn danh hạ **từng trường** với kiểu trường của instantiation.
//! 4. Đích `Field`/`Index`, `with`, phương thức/closure/giá trị hàm truyền kiểu mong đợi.
//! 5. `enum Hop[T]` / `Result[G[int?], str]` / `?.` giữ nguyên ngữ nghĩa VD-263.
//! 6. interp và `--vm` **byte-khớp** (exit/stdout/stderr); checker vẫn từ chối sai kiểu `VS0412`.
//!
//! Mỗi ca chạy **cả hai engine** trên cùng tệp: `vis run --no-cache` (interp) và
//! `vis run --no-cache --vm`; so byte exit/stdout/stderr.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

static DEM: AtomicU32 = AtomicU32::new(0);

fn tam(tag: &str) -> PathBuf {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-vd265-{tag}-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("tạo thư mục tạm");
    d
}

fn ghi(d: &Path, tag: &str, src: &str) -> PathBuf {
    let p = d.join(format!("{tag}.vis"));
    std::fs::write(
        &p,
        format!("# SPDX-License-Identifier: MIT OR Apache-2.0\n{src}"),
    )
    .expect("ghi tệp tạm");
    p
}

fn chay_tep(p: &Path, vm: bool) -> (i32, String, String) {
    let mut c = Command::new(env!("CARGO_BIN_EXE_vis"));
    c.arg("run").arg("--no-cache");
    if vm {
        c.arg("--vm");
    }
    let o = c.arg(p).output().expect("chạy `vis run`");
    (
        o.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&o.stdout).to_string(),
        String::from_utf8_lossy(&o.stderr).to_string(),
    )
}

/// Chạy interp + VM trên cùng tệp, trả `[interp, vm]`. Không tự so gì.
fn hai_duong(tag: &str, src: &str) -> [(i32, String, String); 2] {
    let d = tam(tag);
    let p = ghi(&d, tag, src);
    let a = chay_tep(&p, false);
    let b = chay_tep(&p, true);
    let _ = std::fs::remove_dir_all(&d);
    [a, b]
}

/// Ca thành công: exit + stdout + stderr **byte-khớp**, exit 0, stderr rỗng.
fn thanh_cong(tag: &str, src: &str, stdout: &str) {
    let [a, b] = hai_duong(tag, src);
    assert_eq!(
        (a.0, &a.1, &a.2),
        (b.0, &b.1, &b.2),
        "{tag}: interp ≠ VM (exit/stdout/stderr)\n  interp: {a:?}\n  vm: {b:?}"
    );
    assert_eq!(a.0, 0, "{tag}: phải chạy sạch, stderr: {}", a.2);
    assert_eq!(a.1, stdout, "{tag}: stdout");
    assert_eq!(a.2, "", "{tag}: stderr phải rỗng");
}

fn check(p: &Path) -> (i32, String) {
    let o = Command::new(env!("CARGO_BIN_EXE_vis"))
        .arg("check")
        .arg(p)
        .output()
        .expect("chạy `vis check`");
    (
        o.status.code().unwrap_or(-1),
        format!(
            "{}{}",
            String::from_utf8_lossy(&o.stdout),
            String::from_utf8_lossy(&o.stderr)
        ),
    )
}

// ===========================================================================
// 1 — callee khai **sau** call-site
// ===========================================================================

#[test]
fn goi_ham_khai_sau_tra_ve_mang_instantiation() {
    // Trước bản sửa: `Some(5)` thành `5` (tham số `b` hạ như `Box` trần).
    thanh_cong(
        "goi-sau-tra-ve",
        "type Box[T]:\n    v: T\n\nfn main():\n\
         \x20   print(nhan(Box(v: 5)).v)\n\n\
         fn nhan(b: Box[int?]) -> Box[int?]:\n\
         \x20   b\n",
        "Some(5)\n",
    );
}

#[test]
fn goi_ham_khai_sau_dung_tham_so_trong_than() {
    // Trước bản sửa: `print(b.v)` in `5`.
    thanh_cong(
        "goi-sau-than",
        "type Box[T]:\n    v: T\n\nfn main():\n\
         \x20   nhan(Box(v: 5))\n\n\
         fn nhan(b: Box[int?]):\n\
         \x20   print(b.v)\n",
        "Some(5)\n",
    );
}

// ===========================================================================
// 2 — `if`/`match` ở **đuôi thân** hàm generic
// ===========================================================================

#[test]
fn duoi_than_if_mang_instantiation() {
    // Trước bản sửa: `Some(5)`/`Some(6)` thành `5`/`6`.
    thanh_cong(
        "duoi-if",
        "type Box[T]:\n    v: T\n\n\
         fn tao(ok: bool) -> Box[int?]:\n\
         \x20   if ok:\n\
         \x20       Box(v: 5)\n\
         \x20   else:\n\
         \x20       Box(v: 6)\n\n\
         fn main():\n\
         \x20   print(tao(true).v)\n\
         \x20   print(tao(false).v)\n",
        "Some(5)\nSome(6)\n",
    );
}

#[test]
fn duoi_than_match_mang_instantiation() {
    // Trước bản sửa: `Some(5)`/`Some(6)` thành `5`/`6`.
    thanh_cong(
        "duoi-match",
        "type Box[T]:\n    v: T\n\n\
         fn tao(ok: bool) -> Box[int?]:\n\
         \x20   match ok:\n\
         \x20       true:\n\
         \x20           Box(v: 5)\n\
         \x20       false:\n\
         \x20           Box(v: 6)\n\n\
         fn main():\n\
         \x20   print(tao(true).v)\n\
         \x20   print(tao(false).v)\n",
        "Some(5)\nSome(6)\n",
    );
}

#[test]
fn if_trong_let_va_elif_mang_instantiation() {
    // Trước bản sửa: `Some(7)`/`Some(8)` thành `7`/`8` (nhánh `if` biểu thức).
    thanh_cong(
        "if-let",
        "type Box[T]:\n    v: T\n\nfn main():\n\
         \x20   let a: Box[int?] = if true: Box(v: 7) else: Box(v: 8)\n\
         \x20   print(a.v)\n\
         \x20   let b: Box[int?] = if false:\n\
         \x20       Box(v: 1)\n\
         \x20   elif true:\n\
         \x20       Box(v: 8)\n\
         \x20   else:\n\
         \x20       Box(v: 2)\n\
         \x20   print(b.v)\n",
        "Some(7)\nSome(8)\n",
    );
}

// ===========================================================================
// 3 — constructor lồng trong literal bản ghi ẩn danh
// ===========================================================================

#[test]
fn literal_an_danh_long_nhan_instantiation() {
    // Trước bản sửa: `w.inner.v` in `5`.
    thanh_cong(
        "literal-long",
        "type Box[T]:\n    v: T\n\ntype Wrap[T]:\n    inner: T\n\nfn main():\n\
         \x20   let w: Wrap[Box[int?]] = {inner: Box(v: 5)}\n\
         \x20   print(w.inner.v)\n",
        "Some(5)\n",
    );
}

#[test]
fn goi_ham_khai_sau_nhan_literal_an_danh_long() {
    // Trước bản sửa: `print(w.inner.v)` in `6` (literal ẩn danh trong đối số).
    thanh_cong(
        "literal-doi-so",
        "type Box[T]:\n    v: T\n\ntype Wrap[T]:\n    inner: T\n\nfn main():\n\
         \x20   nhan2({inner: Box(v: 6)})\n\n\
         fn nhan2(w: Wrap[Box[int?]]):\n\
         \x20   print(w.inner.v)\n",
        "Some(6)\n",
    );
}

#[test]
fn constructor_long_trong_doi_so_ban_ghi_co_ten() {
    // Trước bản sửa: `print(w.inner.v)` in `7`.
    thanh_cong(
        "ctor-long",
        "type Box[T]:\n    v: T\n\ntype Wrap[T]:\n    inner: T\n\nfn main():\n\
         \x20   nhan2(Wrap(inner: Box(v: 7)))\n\n\
         fn nhan2(w: Wrap[Box[int?]]):\n\
         \x20   print(w.inner.v)\n",
        "Some(7)\n",
    );
}

// ===========================================================================
// 4 — RHS của `Field`/`Index`
// ===========================================================================

#[test]
fn gan_truong_bang_constructor_long() {
    // Trước bản sửa: `Some(5)` thành `5` (RHS không biết kiểu trường `Box[int?]`).
    thanh_cong(
        "gan-truong-long",
        "type Box[T]:\n    v: T\n\ntype G[T]:\n    f: T\n\nfn main():\n\
         \x20   var g: G[Box[int?]] = G(f: Box(v: 0))\n\
         \x20   g.f = Box(v: 5)\n\
         \x20   print(g.f.v)\n",
        "Some(5)\n",
    );
}

#[test]
fn gan_chi_so_bang_constructor_long() {
    // Trước bản sửa: `Some(6)` thành `6` (RHS không biết kiểu phần tử `Box[int?]`).
    thanh_cong(
        "gan-chi-so-long",
        "type Box[T]:\n    v: T\n\nfn main():\n\
         \x20   var xs: list[Box[int?]] = [Box(v: 0)]\n\
         \x20   xs[0] = Box(v: 6)\n\
         \x20   print(xs[0].v)\n",
        "Some(6)\n",
    );
}

// ===========================================================================
// 5 — gọi phương thức / closure / giá trị hàm
// ===========================================================================

#[test]
fn goi_phuong_thuc_mang_instantiation() {
    // Trước bản sửa: `Some(5)`/`Some(6)` thành `5`/`6`.
    thanh_cong(
        "phuong-thuc",
        "type Box[T]:\n    v: T\n\ntype Holder:\n    n: int\n\n\
         impl Holder:\n\
         \x20   fn lay(self, b: Box[int?]):\n\
         \x20       print(b.v)\n\
         \x20   fn tra(self, b: Box[int?]) -> Box[int?]:\n\
         \x20       b\n\n\
         fn main():\n\
         \x20   let h: Holder = Holder(n: 1)\n\
         \x20   h.lay(Box(v: 5))\n\
         \x20   print(h.tra(Box(v: 6)).v)\n",
        "Some(5)\nSome(6)\n",
    );
}

#[test]
fn goi_closure_va_gia_tri_ham_mang_instantiation() {
    // Trước bản sửa: `Some(7)`/`Some(8)`/`Some(9)` thành `7`/`8`/`9`.
    thanh_cong(
        "closure-gia-tri-ham",
        "type Box[T]:\n    v: T\n\nfn main():\n\
         \x20   let f = fn(b: Box[int?]):\n\
         \x20       print(b.v)\n\
         \x20   f(Box(v: 7))\n\
         \x20   let g = nhan\n\
         \x20   g(Box(v: 8))\n\
         \x20   let h: fn(Box[int?]) -> () = nhan\n\
         \x20   h(Box(v: 9))\n\n\
         fn nhan(b: Box[int?]):\n\
         \x20   print(b.v)\n",
        "Some(7)\nSome(8)\nSome(9)\n",
    );
}

#[test]
fn with_long_mang_instantiation() {
    // Trước bản sửa: `Some(9)` thành `9` (giá trị `with` không biết kiểu trường).
    thanh_cong(
        "with-long",
        "type Box[T]:\n    v: T\n\ntype G[T]:\n    f: T\n\nfn main():\n\
         \x20   let w: G[Box[int?]] = G(f: Box(v: 0))\n\
         \x20   let w2 = w with f: Box(v: 9)\n\
         \x20   print(w2.f.v)\n",
        "Some(9)\n",
    );
}

#[test]
fn goi_tiep_tren_ket_qua_ham_mang_instantiation() {
    // Trước bản sửa: `tao_h().lay(…)` báo VS0403 "kiểu không tĩnh"; `with`/gọi closure mất lớp.
    thanh_cong(
        "goi-tiep-ket-qua",
        "type Box[T]:\n    v: T\n\ntype G[T]:\n    f: T\n\ntype Holder:\n    n: int\n\n\
         impl Holder:\n\
         \x20   fn lay(self, b: Box[int?]):\n\
         \x20       print(b.v)\n\n\
         fn tao_h() -> Holder:\n\
         \x20   Holder(n: 1)\n\n\
         fn tao_g() -> G[Box[int?]]:\n\
         \x20   G(f: Box(v: 0))\n\n\
         fn main():\n\
         \x20   tao_h().lay(Box(v: 5))\n\
         \x20   let w = tao_g() with f: Box(v: 6)\n\
         \x20   print(w.f.v)\n\
         \x20   let f = fn(b: Box[int?]) -> Box[int?]: b\n\
         \x20   print(f(Box(v: 7)).v)\n",
        "Some(5)\nSome(6)\nSome(7)\n",
    );
}

#[test]
fn closure_duoi_if_va_return_if_mang_instantiation() {
    // Trước bản sửa: `Some(5)`/`Some(8)`/`Some(7)` thành `5`/`8`/`7`.
    thanh_cong(
        "closure-if-return",
        "type Box[T]:\n    v: T\n\nfn main():\n\
         \x20   let f = fn() -> Box[int?]:\n\
         \x20       if true:\n\
         \x20           Box(v: 5)\n\
         \x20       else:\n\
         \x20           Box(v: 6)\n\
         \x20   print(f().v)\n\
         \x20   print(ret_if(true).v)\n\
         \x20   let g = fn(b: Box[int?]) -> Box[int?]:\n\
         \x20       if b.v == None:\n\
         \x20           Box(v: 1)\n\
         \x20       else:\n\
         \x20           b\n\
         \x20   print(g(Box(v: 7)).v)\n\n\
         fn ret_if(ok: bool) -> Box[int?]:\n\
         \x20   return if ok: Box(v: 8) else: Box(v: 9)\n",
        "Some(5)\nSome(8)\nSome(7)\n",
    );
}

// ===========================================================================
// 6 — `enum Hop[T]` / `Result[G[int?], str]` / `?.`
// ===========================================================================

#[test]
fn enum_hop_khai_sau_mang_instantiation() {
    // Khoá ngữ nghĩa VD-263 cho `enum Hop[T]` khi chữ ký hàm **khai sau** call-site (pre-pass).
    thanh_cong(
        "enum-hop",
        "enum Hop[T]:\n    Box(v: T)\n    Empty\n\nfn main():\n\
         \x20   let a: Hop[int?] = Hop.Empty\n\
         \x20   print(doc(a))\n\
         \x20   print(doc(Box(5)))\n\n\
         fn doc(h: Hop[int?]) -> int?:\n\
         \x20   match h:\n\
         \x20       Box(x): x\n\
         \x20       Empty: None\n",
        "None\nSome(5)\n",
    );
}

#[test]
fn result_long_mang_instantiation() {
    // Khoá ngữ nghĩa `Result[G[int?], str]` khi chữ ký hàm **khai sau** call-site (pre-pass).
    thanh_cong(
        "result-long",
        "type G[T]:\n    f: T\n\nfn main():\n\
         \x20   print(res(Ok(G(f: 5))))\n\
         \x20   print(res(Err(\"x\")))\n\n\
         fn res(r: Result[G[int?], str]) -> int?:\n\
         \x20   match r:\n\
         \x20       Ok(g): g.f\n\
         \x20       Err(_): None\n",
        "Some(5)\nNone\n",
    );
}

#[test]
fn cham_hoi_tren_instantiation_giu_lop_opt() {
    // `?.` giữ thêm **một** lớp `Option` của trường `T?` (hợp đồng VD-263) — cả hai engine.
    thanh_cong(
        "cham-hoi",
        "type Box[T]:\n    v: T\n\ntype G[T]:\n    f: T\n\nfn main():\n\
         \x20   let g: G[Box[int?]]? = Some(G(f: Box(v: 5)))\n\
         \x20   print(g?.f)\n\
         \x20   let n: G[Box[int?]]? = None\n\
         \x20   print(n?.f)\n",
        "Some(Box(v: Some(5)))\nNone\n",
    );
}

// ===========================================================================
// 7 — checker vẫn từ chối sai kiểu theo instantiation (không nới lỏng)
// ===========================================================================

#[test]
fn checker_tu_choi_goi_ham_khai_sau_sai_instantiation() {
    let d = tam("checker-sau");
    let p = ghi(
        &d,
        "sai-sau",
        "type Box[T]:\n    v: T\n\nfn main():\n    nhan(Box(v: \"x\"))\n\n\
         fn nhan(b: Box[int?]):\n    print(b.v)\n",
    );
    let (code, out) = check(&p);
    let _ = std::fs::remove_dir_all(&d);
    assert_ne!(code, 0, "phải từ chối: {out}");
    assert!(out.contains("VS0412"), "phải báo VS0412: {out}");
}
