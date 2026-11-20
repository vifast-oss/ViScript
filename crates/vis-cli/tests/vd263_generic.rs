// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-263 — generic Option field lift**: `T?` của một instantiation generic (`Box[int?]`,
//! `G[int?]`, `Hop[int?]`) phải được nâng ở **mọi** biên dựng/đọc/ghi, không chỉ ở bản khai báo
//! (nơi `T` đã bị xoá thành `{error}`).
//!
//! Trước bản sửa: `type Box[T]: v: T` + `let b: Box[int?] = Box(v: 5)` →
//! interp/VM đều lưu `5` trần (không `Some(5)`), `with`/gán trường không nâng, và
//! `enum Hop[T]` + `Box(5)` trap ở `match` lồng `Some(y)`/`None`.
//!
//! Hợp đồng khoá bởi tệp này:
//! 1. Constructor bản ghi/enum dùng **kiểu instantiation** tại chỗ dựng (HIR mang id instantiation);
//! 2. `?.` giữ thêm một lớp `Option` của trường `T?`;
//! 3. `with` và `x.f = v` nâng đúng lớp `Opt` của trường instantiation;
//! 4. interp và `--vm` **byte-khớp** (exit/stdout/stderr) trên cùng tệp;
//! 5. sai kiểu theo instantiation bị checker từ chối (`VS0412`).

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

static DEM: AtomicU32 = AtomicU32::new(0);

fn tam(tag: &str) -> PathBuf {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-vd263-{tag}-{}-{n}", std::process::id()));
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
// 1 — record ctor + `?.`
// ===========================================================================

#[test]
fn record_ctor_va_cham_hoi_nang_dung_lop() {
    thanh_cong(
        "record-ctor",
        "type G[T]:\n    f: T\n\nfn main():\n\
         \x20   let g: G[int?]? = Some(G(f: 5))\n\
         \x20   print(g?.f)\n\
         \x20   let n: G[int?]? = Some(G(f: None))\n\
         \x20   print(n?.f)\n",
        "Some(Some(5))\nSome(None)\n",
    );
}

#[test]
fn record_ctor_truc_tiep_nang_truong_opt() {
    thanh_cong(
        "record-truc-tiep",
        "type Box[T]:\n    v: T\n\nfn main():\n\
         \x20   let b: Box[int?] = Box(v: 5)\n\
         \x20   print(b.v)\n\
         \x20   match b.v:\n\
         \x20       Some(y): print(y)\n\
         \x20       None: print(\"none\")\n",
        "Some(5)\n5\n",
    );
}

#[test]
fn record_ctor_truc_tiep_duoi_annot_opt() {
    // `G[int?]?` không có `Some(…)`: constructor dựng lõi `G[int?]` rồi biên annot nâng `Some`.
    thanh_cong(
        "record-ctor-opt",
        "type G[T]:\n    f: T\n\nfn main():\n\
         \x20   let g: G[int?]? = G(f: 5)\n\
         \x20   print(g?.f)\n",
        "Some(Some(5))\n",
    );
}

#[test]
fn literal_ban_ghi_an_danh_theo_instantiation() {
    thanh_cong(
        "record-literal",
        "type Hop[T]:\n    gia_tri: T\n\nfn main():\n\
         \x20   let a: Hop[int?] = {gia_tri: 5}\n\
         \x20   print(a.gia_tri)\n",
        "Some(5)\n",
    );
}

// ===========================================================================
// 2 — enum ctor + `match`
// ===========================================================================

#[test]
fn enum_ctor_va_match_long_nang_dung_lop() {
    thanh_cong(
        "enum-ctor",
        "enum Hop[T]:\n    Box(v: T)\n    Empty\n\nfn main():\n\
         \x20   let h: Hop[int?] = Box(5)\n\
         \x20   match h:\n\
         \x20       Box(x):\n\
         \x20           match x:\n\
         \x20               Some(y): print(y)\n\
         \x20               None: print(\"none\")\n\
         \x20       Empty:\n\
         \x20           print(\"empty\")\n",
        "5\n",
    );
}

#[test]
fn enum_ctor_duoi_annot_opt_van_nang_truong() {
    thanh_cong(
        "enum-opt",
        "enum Hop[T]:\n    Box(v: T)\n    Empty\n\nfn main():\n\
         \x20   let h: Hop[int?]? = Box(9)\n\
         \x20   match h:\n\
         \x20       Some(x):\n\
         \x20           match x:\n\
         \x20               Box(y):\n\
         \x20                   match y:\n\
         \x20                       Some(z): print(z)\n\
         \x20                       None: print(\"none\")\n\
         \x20               Empty:\n\
         \x20                   print(\"empty\")\n\
         \x20       None:\n\
         \x20           print(\"none-hop\")\n",
        "9\n",
    );
}

#[test]
fn enum_ctor_co_tien_to_enum_cung_nang() {
    thanh_cong(
        "enum-tien-to",
        "enum Hop[T]:\n    Box(v: T)\n    Empty\n\nfn main():\n\
         \x20   let h: Hop[int?] = Hop.Box(7)\n\
         \x20   match h:\n\
         \x20       Box(x):\n\
         \x20           match x:\n\
         \x20               Some(y): print(y)\n\
         \x20               None: print(\"none\")\n\
         \x20       Empty:\n\
         \x20           print(\"empty\")\n",
        "7\n",
    );
}

// ===========================================================================
// 3 — `with`
// ===========================================================================

#[test]
fn with_nang_truong_opt_cua_instantiation() {
    thanh_cong(
        "with",
        "type Box[T]:\n    v: T\n\nfn main():\n\
         \x20   let b: Box[int?] = Box(v: 5)\n\
         \x20   let c = b with v: 6\n\
         \x20   print(c.v)\n\
         \x20   let d = c with v: None\n\
         \x20   print(d.v)\n",
        "Some(6)\nNone\n",
    );
}

// ===========================================================================
// 4 — gán trường
// ===========================================================================

#[test]
fn gan_truong_nang_truong_opt_cua_instantiation() {
    thanh_cong(
        "gan-truong",
        "type Box[T]:\n    v: T\n\nfn main():\n\
         \x20   var b: Box[int?] = Box(v: None)\n\
         \x20   b.v = 7\n\
         \x20   print(b.v)\n\
         \x20   b.v = None\n\
         \x20   print(b.v)\n",
        "Some(7)\nNone\n",
    );
}

#[test]
fn gan_lai_bien_bang_constructor_theo_instantiation() {
    thanh_cong(
        "gan-lai-bien",
        "type Box[T]:\n    v: T\n\nfn main():\n\
         \x20   var b: Box[int?] = Box(v: None)\n\
         \x20   b = Box(v: 8)\n\
         \x20   print(b.v)\n",
        "Some(8)\n",
    );
}

#[test]
fn tham_so_va_return_mang_instantiation() {
    thanh_cong(
        "ham-bien",
        "type Box[T]:\n    v: T\n\n\
         fn nhan(b: Box[int?]):\n\
         \x20   print(b.v)\n\n\
         fn tao() -> Box[int?]:\n\
         \x20   Box(v: 5)\n\n\
         fn tao_return() -> Box[int?]:\n\
         \x20   return Box(v: 6)\n\n\
         fn main():\n\
         \x20   nhan(Box(v: 5))\n\
         \x20   let a = tao()\n\
         \x20   print(a.v)\n\
         \x20   let b = tao_return()\n\
         \x20   print(b.v)\n",
        "Some(5)\nSome(5)\nSome(6)\n",
    );
}

// ===========================================================================
// 5 — checker từ chối sai kiểu theo instantiation
// ===========================================================================

#[test]
fn checker_tu_choi_truong_sai_kieu_instantiation() {
    let d = tam("checker-record");
    let p = ghi(
        &d,
        "sai-record",
        "type Box[T]:\n    v: T\n\nfn main():\n    let b: Box[int?] = Box(v: \"x\")\n",
    );
    let (code, out) = check(&p);
    let _ = std::fs::remove_dir_all(&d);
    assert_ne!(code, 0, "phải từ chối: {out}");
    assert!(out.contains("VS0412"), "phải báo VS0412: {out}");
}

#[test]
fn checker_tu_choi_bien_the_sai_kieu_instantiation() {
    let d = tam("checker-enum");
    let p = ghi(
        &d,
        "sai-enum",
        "enum Hop[T]:\n    Box(v: T)\n    Empty\n\nfn main():\n    let h: Hop[int?] = Box(\"x\")\n",
    );
    let (code, out) = check(&p);
    let _ = std::fs::remove_dir_all(&d);
    assert_ne!(code, 0, "phải từ chối: {out}");
    assert!(out.contains("VS0412"), "phải báo VS0412: {out}");
}
