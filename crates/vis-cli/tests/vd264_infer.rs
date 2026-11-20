// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-264 — suy kiểu sâu cho `Option`: pattern lồng + kết quả gọi hàm phải cùng ngữ nghĩa
//! ở hai engine**.
//!
//! Trước bản sửa, interp chỉ gán kiểu cho `HPat::Bind` trần (`let`/`for`) và `expr_ty` chỉ biết
//! `Var`/`Field`/`Index`; codegen đã đệ quy pattern nhưng `infer_ty`/`base_ty` vẫn mù khi kiểu
//! tuple do lower dựng chứa `Error` hoặc khi `base` là lời gọi hàm. Bốn ca gốc của review:
//!
//! 1. `branch4.vis` — biến bind trong pattern tuple: `let p=(Some(1),2); match p:(a,_): …`
//!    ⇒ cả hai engine trap `VS0501` vì `a`/`r` mất lớp `Option`.
//! 2. `pushcall2.vis` — `let xs = make().push(5)` với `make() -> list[int?]` ⇒ interp để `5`
//!    trần, VM nâng `Some(5)`.
//! 3. `optpat.vis` — `a?.f` với `a` bind từ pattern tuple ⇒ interp `Some(None)`, VM `None`.
//! 4. `optfieldcall2.vis` — `lay()?.b` với `lay() -> A?` ⇒ interp `None`, VM `Some(None)`.
//!
//! Hợp đồng sau bản sửa: kiểu tĩnh dùng cho việc nâng `Option` phải được suy **đệ quy** qua
//! pattern (Let/Var/For/Match arm/if-let/while-let; Tuple/List/Record/Ctor/Or) và qua biểu thức
//! (`Call` trả về, `Try` kể cả `xs[i]?`, `With`, `If`/`Match`, field/index). Mỗi ca chạy **cả
//! hai engine** trên cùng tệp: `vis run --no-cache` (interp) và `vis run --no-cache --vm`; so
//! byte exit/stdout/stderr.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

static DEM: AtomicU32 = AtomicU32::new(0);

fn tam(tag: &str) -> PathBuf {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-vd264-{tag}-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("tạo thư mục tạm");
    d
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

/// Chạy interp + VM trên cùng tệp, trả `[interp, vm]`.
fn hai_duong(tag: &str, src: &str) -> [(i32, String, String); 2] {
    let d = tam(tag);
    let p = d.join(format!("{tag}.vis"));
    std::fs::write(
        &p,
        format!("# SPDX-License-Identifier: MIT OR Apache-2.0\n{src}"),
    )
    .expect("ghi tệp tạm");
    let a = chay_tep(&p, false);
    let b = chay_tep(&p, true);
    let _ = std::fs::remove_dir_all(&d);
    [a, b]
}

/// Ca thành công: đòi exit + stdout + stderr **byte-khớp**, `exit = 0`, stderr rỗng.
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

// ===========================================================================
// 1 — Ca gốc review: biến bind trong pattern tuple, nhánh `if` lệch lớp `Option`
// ===========================================================================

#[test]
fn pattern_tuple_bien_option_trong_if() {
    thanh_cong(
        "pattern-tuple-if",
        r#"fn main():
    let p = (Some(1), 2)
    match p:
        (a, _):
            let r = if false: a else: 5
            match r:
                Some(x): print(x)
                None: print("none")
"#,
        "5\n",
    );
}

// ===========================================================================
// 2 — Ca gốc review: `push` trên **kết quả gọi hàm** `list[int?]`
// ===========================================================================

#[test]
fn push_tren_ket_qua_goi_ham() {
    thanh_cong(
        "push-call",
        r#"fn make() -> list[int?]:
    [None]

fn main():
    let xs = make().push(5)
    print(xs)
"#,
        "[None, Some(5)]\n",
    );
}

// ===========================================================================
// 3 — Ca gốc review: `a?.f` với `a` bind từ pattern tuple
// ===========================================================================

#[test]
fn optional_field_cua_bien_bind_pattern() {
    thanh_cong(
        "opt-field-pattern",
        r#"type U:
    f: int?

fn main():
    let p = (Some(U(f: None)), 1)
    match p:
        (a, _):
            print(a?.f)
"#,
        "Some(None)\n",
    );
}

// ===========================================================================
// 4 — Ca gốc review: `lay()?.b` với base là lời gọi hàm trả `A?`
// ===========================================================================

#[test]
fn optional_field_cua_ket_qua_goi_ham() {
    thanh_cong(
        "opt-field-call",
        r#"type A:
    b: int?

fn lay() -> A?:
    Some(A(b: None))

fn main():
    print(lay()?.b)
"#,
        "Some(None)\n",
    );
}

// ===========================================================================
// 5 — Biến thể của (1): chủ thể `match` là lời gọi trả tuple chứa `int?`
// ===========================================================================

#[test]
fn pattern_tuple_tu_ket_qua_goi_ham() {
    thanh_cong(
        "call-tuple-if",
        r#"fn mk() -> tuple[int?, int]:
    (Some(1), 2)

fn main():
    match mk():
        (a, _):
            let r = if false: a else: 5
            match r:
                Some(x): print(x)
                None: print("none")
"#,
        "5\n",
    );
}

// ===========================================================================
// 6 — Biến thể của (2): `list[int??]` — nâng đúng **hai** lớp tại `push`
// ===========================================================================

#[test]
fn push_tren_ket_qua_goi_ham_hai_lop_option() {
    thanh_cong(
        "push-call-nested",
        r#"fn make() -> list[int??]:
    [None]

fn main():
    let xs = make().push(Some(5))
    print(xs)
"#,
        "[None, Some(Some(5))]\n",
    );
}

// ===========================================================================
// 7 — `if let` / `while let` trên kết quả gọi hàm: biến bind có kiểu tĩnh
// ===========================================================================

#[test]
fn if_let_ket_qua_goi_ham() {
    thanh_cong(
        "if-let-call",
        r#"fn make() -> int?:
    Some(7)

fn main():
    if let Some(x) = make():
        print(x)
    else:
        print("none")
"#,
        "7\n",
    );
}

#[test]
fn while_let_tren_bien_option() {
    thanh_cong(
        "while-let",
        r#"fn main():
    var x: int? = Some(0)
    while let Some(v) = x:
        print(v)
        if v >= 1:
            x = None
        else:
            x = Some(v + 1)
"#,
        "0\n1\n",
    );
}

// ===========================================================================
// 8 — `match` trên lời gọi + or-pattern: arm bind có kiểu, nhánh nâng đúng
// ===========================================================================

#[test]
fn match_ket_qua_goi_ham() {
    thanh_cong(
        "match-call",
        r#"fn make() -> int?:
    Some(9)

fn main():
    match make():
        Some(x):
            let r = if false: x else: 0
            print(r)
        None:
            print("none")
"#,
        "0\n",
    );
}

#[test]
fn match_or_pattern_tren_ket_qua_goi_ham() {
    thanh_cong(
        "match-or",
        r#"fn make() -> int?:
    None

fn main():
    match make():
        Some(x) | None:
            let y: int = 1
            print(y)
"#,
        "1\n",
    );
}

// ===========================================================================
// 9 — `for` trên pattern tuple/record: kiểu pattern là **phần tử** của iter
// ===========================================================================

#[test]
fn for_pattern_tuple() {
    thanh_cong(
        "for-tuple",
        r#"fn main():
    for (a, _) in [(Some(3), 4)]:
        let r = if false: a else: 0
        match r:
            Some(x): print(x)
            None: print("none")
"#,
        "0\n",
    );
}

#[test]
fn for_pattern_map_suy_kieu_phan_tu() {
    thanh_cong(
        "for-map",
        r#"fn main():
    let m: map[str, int?] = {"a": None, "b": Some(2)}
    for (k, v) in m:
        let r = if false: v else: 5
        match r:
            Some(x): print(k)
            None: print("none")
"#,
        "a\nb\n",
    );
}

// ===========================================================================
// 10 — Pattern bản ghi bind từ kết quả gọi hàm
// ===========================================================================

#[test]
fn pattern_record_tu_ket_qua_goi_ham() {
    thanh_cong(
        "record-pattern-call",
        r#"type A:
    b: int?
    c: int

fn lay() -> A:
    A(b: None, c: 2)

fn main():
    match lay():
        {b, c}:
            print(b)
            print(c)
"#,
        "None\n2\n",
    );
}

// ===========================================================================
// 11 — `xs[i]?` (chỉ số có kiểm tra) giữ đúng lớp `Option` trong nhánh `if`
// ===========================================================================

#[test]
fn chi_so_co_kiem_tra_trong_if() {
    thanh_cong(
        "index-try",
        r#"fn main():
    let xs: list[int?] = [Some(7)]
    var i = 0
    let r = if true: xs[i]? else: Some(0)
    match r:
        Some(Some(x)): print(x)
        Some(None): print("inner-none")
        None: print("outer-none")
"#,
        "7\n",
    );
}
