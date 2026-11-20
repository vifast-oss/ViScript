// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-258** — hồi quy lỗ hổng soundness của checker/2 engine:
//!
//! 1. `Result?` — `Result` **không** phải lớp `Option`: `let x: Result[int, str]? = Ok(7)` phải
//!    lưu `Some(Ok(7))` ở cả interp và VM.
//! 2. Trường bản ghi — `U(age: "hello")`, `U(age: None)`, `U(bogus: …)` và default sai kiểu phải
//!    bị từ chối tĩnh; default đúng vẫn dựng được.
//! 3. `None` — có kiểu `Option[?]`: `[None]` cho `list[int]`, `if None`, `1 + None`,
//!    `xs.push(None)`, `E.A(None)` đều đỏ; `None` cho đích `T?` vẫn xanh.
//! 4. `?.` — field `f: int?` cho `u?.f: int??`: `None` ⇒ `Some(None)`, `Some(3)` ⇒
//!    `Some(Some(3))` — không được lift một lớp mù.
//! 5. Nâng Option ở biên dựng bản ghi/enum, `with`, `list.push`.
//! 6. Đa tệp — `Show` dựng sẵn ở module thứ 2; `Json`/`SqlValue` ctor dưới `Strict`.
//!
//! Mọi ca chạy **cả hai engine** (`vis run` và `vis run --vm --no-cache`), so byte stdout.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

static DEM: AtomicU32 = AtomicU32::new(0);

fn tam(tag: &str) -> PathBuf {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-vd258-{tag}-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("tạo thư mục tạm");
    d
}

fn chay_tep(p: &Path, extra: &[&str]) -> (i32, String, String) {
    let mut c = Command::new(env!("CARGO_BIN_EXE_vis"));
    c.arg("run").arg("--no-cache");
    c.args(extra);
    c.arg(p);
    let o = c.output().expect("chạy vis run");
    (
        o.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&o.stdout).to_string(),
        String::from_utf8_lossy(&o.stderr).to_string(),
    )
}

/// Chạy interp + VM, trả `[interp, vm]`, đã assert hai engine giống **byte** stdout/stderr/exit.
fn chay_ca_hai(tag: &str, src: &str) -> [(i32, String, String); 2] {
    let d = tam(tag);
    let p = d.join(format!("{tag}.vis"));
    std::fs::write(
        &p,
        format!("# SPDX-License-Identifier: MIT OR Apache-2.0\n{src}"),
    )
    .expect("ghi tệp tạm");
    let a = chay_tep(&p, &[]);
    let b = chay_tep(&p, &["--vm"]);
    let _ = std::fs::remove_dir_all(&d);
    assert_eq!(a.0, b.0, "{tag}: exit interp↔VM lệch");
    assert_eq!(a.1, b.1, "{tag}: stdout interp↔VM lệch");
    assert_eq!(a.2, b.2, "{tag}: stderr interp↔VM lệch");
    [a, b]
}

fn check_src(tag: &str, src: &str) -> (i32, String, String) {
    let d = tam(tag);
    let p = d.join(format!("{tag}.vis"));
    std::fs::write(
        &p,
        format!("# SPDX-License-Identifier: MIT OR Apache-2.0\n{src}"),
    )
    .expect("ghi tệp tạm");
    let o = Command::new(env!("CARGO_BIN_EXE_vis"))
        .arg("check")
        .arg(&p)
        .output()
        .expect("chạy vis check");
    let _ = std::fs::remove_dir_all(&d);
    (
        o.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&o.stdout).to_string(),
        String::from_utf8_lossy(&o.stderr).to_string(),
    )
}

fn assert_reject(tag: &str, src: &str, code: &str) {
    let (c, out, err) = check_src(tag, src);
    assert_ne!(
        c, 0,
        "{tag}: `vis check` phải đỏ\nstdout: {out}\nstderr: {err}"
    );
    assert!(
        out.contains(code) || err.contains(code),
        "{tag}: thiếu mã `{code}`\nstdout: {out}\nstderr: {err}"
    );
}

// ===========================================================================
// 1 — `Result` KHÔNG phải lớp Option
// ===========================================================================

#[test]
fn result_opt_let_khong_bi_coi_la_option_layer() {
    let [a, b] = chay_ca_hai(
        "result_let",
        r#"fn main():
    let x: Result[int, str]? = Ok(7)
    match x:
        Some(r):
            match r:
                Ok(v): print("ok {v}")
                Err(e): print("err {e}")
        None: print("none")
"#,
    );
    assert_eq!(a.0, 0, "interp: {}", a.2);
    assert_eq!(a.1, "ok 7\n");
    assert_eq!(b.1, "ok 7\n");
}

#[test]
fn result_opt_param() {
    let [a, _b] = chay_ca_hai(
        "result_param",
        r#"fn f(x: Result[int, str]?) -> int:
    match x:
        Some(r):
            match r:
                Ok(v): v
                Err(e): -1
        None: 0

fn main():
    print(f(Ok(7)))
    print(f(None))
"#,
    );
    assert_eq!(a.0, 0, "interp: {}", a.2);
    assert_eq!(a.1, "7\n0\n");
}

#[test]
fn result_opt_return() {
    let [a, _b] = chay_ca_hai(
        "result_return",
        r#"fn f() -> Result[int, str]?:
    Ok(7)

fn g() -> Result[int, str]?:
    return Err("x")

fn main():
    print(f())
    print(g())
"#,
    );
    assert_eq!(a.0, 0, "interp: {}", a.2);
    assert_eq!(a.1, "Some(Ok(7))\nSome(Err(x))\n");
}

#[test]
fn result_opt_tu_field_va_index() {
    let [a, _b] = chay_ca_hai(
        "result_field_index",
        r#"type U:
    r: Result[int, str]

fn main():
    let u = U(r: Ok(7))
    let a: Result[int, str]? = u.r
    let xs: list[Result[int, str]] = [Ok(8)]
    let b: Result[int, str]? = xs[0]
    print(a)
    print(b)
"#,
    );
    assert_eq!(a.0, 0, "interp: {}", a.2);
    assert_eq!(a.1, "Some(Ok(7))\nSome(Ok(8))\n");
}

#[test]
fn result_opt_hai_lop_van_nang_dung_mot_lop() {
    let [a, _b] = chay_ca_hai(
        "result_nested",
        r#"fn main():
    let x: Result[int, str]?? = Some(Ok(7))
    match x:
        Some(Some(r)):
            match r:
                Ok(v): print("ok {v}")
                Err(e): print("err {e}")
        Some(None): print("some-none")
        None: print("none")
    let y: Result[int, str]?? = Some(Err("x"))
    match y:
        Some(Some(r)):
            match r:
                Ok(v): print("ok {v}")
                Err(e): print("err {e}")
        Some(None): print("some-none")
        None: print("none")
"#,
    );
    assert_eq!(a.0, 0, "interp: {}", a.2);
    assert_eq!(a.1, "ok 7\nerr x\n");
}

// ===========================================================================
// 2 — bản ghi: kiểu trường + default
// ===========================================================================

#[test]
fn record_ctor_sai_kieu_bi_tu_choi() {
    assert_reject(
        "rec_wrong",
        "type U:\n    age: int\n\nfn main():\n    let u = U(age: \"hello\")\n    print(u)\n",
        "VS0412",
    );
}

#[test]
fn record_ctor_none_vao_int_bi_tu_choi_va_giu_vd238() {
    let (c, out, err) = check_src(
        "rec_none",
        "type U:\n    age: int\n\nfn main():\n    let u = U(age: None)\n    print(u)\n",
    );
    assert_ne!(c, 0, "phải đỏ\nstdout: {out}\nstderr: {err}");
    assert!(out.contains("VS0412"), "stdout: {out}");
    assert!(
        out.contains("None` chỉ gán được cho `T?`"),
        "phải giữ thông điệp VD-238; stdout: {out}"
    );
}

#[test]
fn record_ctor_truong_la_bi_tu_choi() {
    assert_reject(
        "rec_unknown",
        "type U:\n    age: int\n\nfn main():\n    let u = U(age: 1, bogus: 2)\n    print(u)\n",
        "VS0403",
    );
}

#[test]
fn record_default_sai_kieu_bi_tu_choi() {
    assert_reject(
        "rec_default_wrong",
        "type U:\n    age: int = \"hello\"\n\nfn main():\n    print(\"x\")\n",
        "VS0412",
    );
}

#[test]
fn record_default_none_vao_int_bi_tu_choi_va_giu_vd238() {
    let (c, out, _err) = check_src(
        "rec_default_none",
        "type U:\n    age: int = None\n\nfn main():\n    print(\"x\")\n",
    );
    assert_ne!(c, 0, "phải đỏ; stdout: {out}");
    assert!(
        out.contains("None` chỉ gán được cho `T?`"),
        "phải giữ thông điệp VD-238; stdout: {out}"
    );
}

#[test]
fn record_default_dung_van_dung_duoc() {
    let [a, _b] = chay_ca_hai(
        "rec_default_ok",
        r#"type U:
    age: int
    name: str = "An"

fn main():
    let u = U(age: 3)
    print(u)
"#,
    );
    assert_eq!(a.0, 0, "interp: {}", a.2);
    assert_eq!(a.1, "U(age: 3, name: An)\n");
}

// ===========================================================================
// 3 — `None` có kiểu Option[?]
// ===========================================================================

#[test]
fn none_vao_list_int_bi_tu_choi() {
    assert_reject(
        "none_list_int",
        "fn main():\n    let xs: list[int] = [None]\n    print(xs)\n",
        "VS0412",
    );
}

#[test]
fn none_lam_dieu_kien_bi_tu_choi() {
    assert_reject(
        "none_if",
        "fn main():\n    if None:\n        print(\"x\")\n",
        "VS0412",
    );
}

#[test]
fn none_trong_phep_cong_bi_tu_choi() {
    let (c, out, err) = check_src(
        "none_add",
        "fn main():\n    let n = 1 + None\n    print(n)\n",
    );
    assert_ne!(c, 0, "phải đỏ\nstdout: {out}\nstderr: {err}");
    assert!(
        out.contains("VS0414") || out.contains("VS0412"),
        "stdout: {out}"
    );
}

#[test]
fn none_push_vao_list_int_bi_tu_choi() {
    assert_reject(
        "none_push",
        "fn main():\n    var xs: list[int] = [1]\n    xs.push(None)\n    print(xs)\n",
        "VS0412",
    );
}

#[test]
fn none_lam_doi_so_bien_the_bi_tu_choi() {
    assert_reject(
        "none_enum_arg",
        "enum E:\n    A(x: int)\n    B\n\nfn main():\n    let e = E.A(None)\n    print(e)\n",
        "VS0412",
    );
}

#[test]
fn none_cho_dich_option_van_hop_le_ca_hai_engine() {
    let [a, _b] = chay_ca_hai(
        "none_ok",
        r#"fn f(x: int??) -> int:
    match x:
        Some(Some(v)): v
        Some(v): 1
        None: 2

fn main():
    let x: int? = None
    let y: int?? = None
    print(x)
    print(y)
    print(f(None))
"#,
    );
    assert_eq!(a.0, 0, "interp: {}", a.2);
    assert_eq!(a.1, "None\nNone\n2\n");
}

// ===========================================================================
// 4 — `?.` lồng đúng kiểu `Opt(field)`
// ===========================================================================

#[test]
fn hoi_cham_tren_field_optional_khong_flatten() {
    let [a, _b] = chay_ca_hai(
        "hoi_cham_nested",
        r#"type U:
    f: int?

fn main():
    let a: U? = Some(U(f: None))
    let b: U? = Some(U(f: Some(3)))
    let c: U? = Some(U(f: 5))
    let d: U? = None
    print(a?.f)
    print(b?.f)
    print(c?.f)
    print(d?.f)
"#,
    );
    assert_eq!(a.0, 0, "interp: {}", a.2);
    assert_eq!(a.1, "Some(None)\nSome(Some(3))\nSome(Some(5))\nNone\n");
}

// ===========================================================================
// 5 — nâng Option ở biên dựng bản ghi/enum, `with`, `list.push`
// ===========================================================================

#[test]
fn nang_option_o_bien_dung_ban_ghi_enum_with_va_push() {
    let [a, _b] = chay_ca_hai(
        "lift_sites",
        r#"type U:
    f: int?
    r: Result[int, str]?

enum E:
    A(x: int?)
    B

fn main():
    let u = U(f: 5, r: Ok(9))
    print(u.f)
    print(u.r)
    let v = u with f: 6
    print(v.f)
    let e = E.A(11)
    print(e)
    let e2 = E.B
    print(e2)
    var xs: list[int?] = []
    xs.push(4)
    print(xs)
    print(xs[0])
"#,
    );
    assert_eq!(a.0, 0, "interp: {}", a.2);
    assert_eq!(
        a.1,
        "Some(5)\nSome(Ok(9))\nSome(6)\nA(Some(11))\nB\n[Some(4)]\nSome(4)\n"
    );
}

#[test]
fn nhanh_if_match_lech_lop_option_duoc_nang() {
    let [a, _b] = chay_ca_hai(
        "branch_lift",
        r#"fn main():
    let a = if true: 1 else: None
    let b = if false: 1 else: None
    let c = if true: None else: 2
    let d = match true:
        true: 3
        false: None
    let e = match false:
        true: 3
        false: None
    let f = if true: Some(5) else: None
    let g = if false: None else: 2
    let h = if true: 1 else: Some(2)
    let i = match false:
        true: 3
        false: Some(4)
    let o: int? = Some(9)
    let j = if false: o else: 2
    print(a)
    print(b)
    print(c)
    print(d)
    print(e)
    print(f)
    print(g)
    print(h)
    print(i)
    print(j)
"#,
    );
    assert_eq!(a.0, 0, "interp: {}", a.2);
    assert_eq!(
        a.1,
        "Some(1)\nNone\nNone\nSome(3)\nNone\nSome(5)\nSome(2)\nSome(1)\nSome(4)\nSome(2)\n"
    );
}

#[test]
fn nhanh_voi_hoi_cham_field_va_chi_so() {
    let [a, _b] = chay_ca_hai(
        "branch_field_index",
        r#"type U:
    f: int
type V:
    g: int?

fn main():
    let u: U? = Some(U(f: 1))
    let x = if false: u?.f else: 2
    let xs: list[int] = [9]
    let y = if false: xs[0]? else: 2
    let v: V? = Some(V(g: Some(7)))
    let z = if false: v?.g else: Some(3)
    print(x)
    print(y)
    print(z)
"#,
    );
    assert_eq!(a.0, 0, "interp: {}", a.2);
    assert_eq!(a.1, "Some(2)\nSome(2)\nSome(Some(3))\n");
}

#[test]
fn const_khong_tu_nang_option_bi_tu_choi() {
    assert_reject(
        "const_opt_no_lift",
        "const X: int? = 42\nfn main():\n    print(X)\n",
        "VS0412",
    );
}

#[test]
fn literal_list_tron_option_va_plain_bi_tu_choi() {
    assert_reject(
        "list_mix",
        "fn main():\n    let xs: list[int?] = [Some(1), 2]\n    print(xs)\n",
        "VS0412",
    );
}

#[test]
fn literal_map_tron_option_va_plain_bi_tu_choi() {
    assert_reject(
        "map_mix",
        "fn main():\n    let m: map[str, int?] = {\"a\": Some(1), \"b\": 2}\n    print(m)\n",
        "VS0412",
    );
}

// ===========================================================================
// 6 — đa tệp: trait dựng sẵn + Json/SqlValue dưới Strict
// ===========================================================================

fn run_da_tep(tag: &str, files: &[(&str, &str)], main: &str) -> [(i32, String, String); 2] {
    let d = tam(tag);
    for (ten, src) in files {
        std::fs::write(
            d.join(ten),
            format!("# SPDX-License-Identifier: MIT OR Apache-2.0\n{src}"),
        )
        .expect("ghi tệp module");
    }
    let p = d.join(main);
    let a = chay_tep(&p, &[]);
    let b = chay_tep(&p, &["--vm"]);
    let _ = std::fs::remove_dir_all(&d);
    assert_eq!(a.0, b.0, "{tag}: exit lệch");
    assert_eq!(a.1, b.1, "{tag}: stdout lệch");
    assert_eq!(a.2, b.2, "{tag}: stderr lệch");
    [a, b]
}

#[test]
fn da_tep_impl_show_o_module_thu_hai() {
    let [a, _b] = run_da_tep(
        "mf_show",
        &[
            ("base.vis", "pub type Base:\n    n: int\n"),
            (
                "util.vis",
                "use ./base\n\npub type Hop:\n    x: int\n\nimpl Show for Hop:\n    fn show(self) -> str:\n        \"hop\"\n\npub fn lam() -> str:\n    let h: Hop = {x: 1}\n    h.show()\n",
            ),
            ("main.vis", "use ./util\n\nfn main():\n    print(lam())\n"),
        ],
        "main.vis",
    );
    assert_eq!(a.0, 0, "interp: {}", a.2);
    assert_eq!(a.1, "hop\n");
}

#[test]
fn da_tep_json_ctor_duoi_strict() {
    let [a, _b] = run_da_tep(
        "mf_json",
        &[
            ("base.vis", "pub type Base:\n    n: int\n"),
            (
                "util.vis",
                "use ./base\n\npub fn lam() -> Json:\n    Json.Arr([Json.Num(1)])\n",
            ),
            ("main.vis", "use ./util\n\nfn main():\n    print(lam())\n"),
        ],
        "main.vis",
    );
    assert_eq!(a.0, 0, "interp: {}", a.2);
    assert_eq!(a.1, "Arr([Num(1)])\n");
}

#[test]
fn da_tep_json_variant_tran_duoi_strict() {
    let [a, _b] = run_da_tep(
        "mf_json_bare",
        &[
            ("base.vis", "pub type Base:\n    n: int\n"),
            (
                "util.vis",
                "use ./base\n\npub fn lam() -> Json:\n    Arr([Num(1)])\n",
            ),
            ("main.vis", "use ./util\n\nfn main():\n    print(lam())\n"),
        ],
        "main.vis",
    );
    assert_eq!(a.0, 0, "interp: {}", a.2);
    assert_eq!(a.1, "Arr([Num(1)])\n");
}

#[test]
fn da_tep_sqlvalue_ctor_duoi_strict() {
    let [a, _b] = run_da_tep(
        "mf_sql",
        &[
            ("base.vis", "pub type Base:\n    n: int\n"),
            (
                "util.vis",
                "use ./base\nuse std.sqlite\n\npub fn lam() -> SqlValue:\n    SqlValue.Integer(3)\n",
            ),
            ("main.vis", "use ./util\n\nfn main():\n    print(lam())\n"),
        ],
        "main.vis",
    );
    assert_eq!(a.0, 0, "interp: {}", a.2);
    assert_eq!(a.1, "Integer(3)\n");
}
