// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-228** (proxy A0-uỷ quyền 2026-11-19) — lift `T → T?` tại ranh giới hàm `-> T?` (spec §4.4).
//!
//! Trước VD-228, `fn pick() -> int?: 42` trả plain `Int(42)` trên cả interp lẫn VM — sai spec.
//! Sau VD-228, cả hai engine tự nâng `Value::some(42)` ở biên hàm, đồng nhất byte-for-byte.

use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

static DEM: AtomicU32 = AtomicU32::new(0);

fn tam(tag: &str) -> PathBuf {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-vd228-{tag}-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("tạo thư mục tạm");
    d
}

fn chay(tag: &str, src: &str, args: &[&str]) -> (i32, String, String) {
    let d = tam(tag);
    let p = d.join(format!("{tag}.vis"));
    std::fs::write(
        &p,
        format!("# SPDX-License-Identifier: MIT OR Apache-2.0\n{src}"),
    )
    .expect("ghi tệp tạm");
    let mut c = Command::new(env!("CARGO_BIN_EXE_vis"));
    c.arg("run").arg("--no-cache");
    c.args(args);
    c.arg(&p);
    let o = c.output().expect("chạy vis run");
    let r = (
        o.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&o.stdout).to_string(),
        String::from_utf8_lossy(&o.stderr).to_string(),
    );
    let _ = std::fs::remove_dir_all(&d);
    r
}

fn chay_ca_hai(tag: &str, src: &str) -> [(u32, String, String); 2] {
    let a = chay(tag, src, &[]);
    let b = chay(tag, src, &["--vm"]);
    [(a.0 as u32, a.1, a.2), (b.0 as u32, b.1, b.2)]
}

/// (1) `fn pick(i: int) -> int?: 42` — plain `Int` tự nâng ở biên `-> T?`.
#[test]
fn vd228_plain_int_lift() {
    let [a, b] = chay_ca_hai(
        "plain_int",
        "fn pick(i: int) -> int?:\n    42\n\nfn main():\n    print(pick(0))\n",
    );
    assert_eq!(a.0, 0, "interp: {}", a.2);
    assert_eq!(a.1, "Some(42)\n");
    assert_eq!(a.1, b.1);
}

/// (2) `fn k() -> str?:` — plain str lift.
#[test]
fn vd228_plain_str_lift() {
    let [a, b] = chay_ca_hai(
        "plain_str",
        "fn k() -> str?:\n    \"x\"\n\nfn main():\n    print(k())\n",
    );
    assert_eq!(a.1, "Some(x)\n");
    assert_eq!(a.1, b.1);
}

/// (3) `fn h() -> int?: None` — `None` giữ nguyên (đã là `Option`).
#[test]
fn vd228_none_stays() {
    let [a, b] = chay_ca_hai(
        "none",
        "fn h() -> int?:\n    None\n\nfn main():\n    print(h())\n",
    );
    assert_eq!(a.1, "None\n");
    assert_eq!(a.1, b.1);
}

/// (4) `fn pick() -> int?: if i < 1: 42 else: None` — branch join lifts cả hai.
#[test]
fn vd228_branch_join() {
    let [a, b] = chay_ca_hai(
        "branch",
        "fn pick(i: int) -> int?:\n    if i < 1:\n        42\n    else:\n        None\n\nfn main():\n    match pick(0):\n        Some(v): print(\"lifted {v}\")\n        None: print(\"none\")\n",
    );
    assert_eq!(a.1, "lifted 42\n");
    assert_eq!(a.1, b.1);
}

/// (5) `fn ret_early() -> int?: return 99` — return sớm lift (hai engine).
#[test]
fn vd228_early_return_lift() {
    let [a, b] = chay_ca_hai(
        "early",
        "fn ret_early() -> int?:\n    return 99\n\nlet m = ret_early()\nprint(m)\n",
    );
    assert_eq!(a.1, "Some(99)\n");
    assert_eq!(a.1, b.1);
}

/// (6) impl-method `-> int?` lift (hai engine).
#[test]
fn vd228_impl_method_lift() {
    let [a, b] = chay_ca_hai(
        "impl_lift",
        "type P:\n    x: int\n\nimpl P:\n    fn pick(self) -> int?:\n        self.x\n\nfn main():\n    let p: P = {x: 42}\n    print(p.pick())\n",
    );
    assert_eq!(a.0, 0, "interp: {}", a.2);
    assert_eq!(a.1, "Some(42)\n");
    assert_eq!(a.1, b.1);
}

/// (7) Không bọc hai lớp: `fn f() -> int?: Some(42)` ⇒ `Some(42)` (không phải `Some(Some(42))`).
#[test]
fn vd228_no_double_wrap() {
    let [a, b] = chay_ca_hai(
        "no_dbl",
        "fn f() -> int?:\n    Some(42)\n\nfn main():\n    print(f())\n",
    );
    assert_eq!(a.0, 0, "interp: {}", a.2);
    assert_eq!(a.1, "Some(42)\n");
    assert_eq!(a.1, b.1);
}

/// (8) Tham số annot `int?` — plain int 42 tự lift ở biên (interp + VM).
#[test]
fn vd228_param_int_lift() {
    let [a, b] = chay_ca_hai(
        "param_int",
        "fn g(x: int?) -> int:\n    match x:\n        Some(v): v\n        None: 9\n\nfn main():\n    print(g(42))\n",
    );
    assert_eq!(a.0, 0, "interp: {}", a.2);
    assert_eq!(a.1, "42\n");
    assert_eq!(a.1, b.1);
}

/// (9) `None` truyền thẳng cho `int?` — giữ nguyên None.
#[test]
fn vd228_param_none_passthrough() {
    let [a, b] = chay_ca_hai(
        "param_none",
        "fn g(x: int?) -> int:\n    match x:\n        Some(v): v\n        None: 9\n\nfn main():\n    print(g(None))\n",
    );
    assert_eq!(a.1, "9\n");
    assert_eq!(a.1, b.1);
}

/// (10) `Some(7)` truyền thẳng — không bọc hai lớp.
#[test]
fn vd228_param_some_passthrough() {
    let [a, b] = chay_ca_hai(
        "param_some",
        "fn g(x: int?) -> int:\n    match x:\n        Some(v): v\n        None: 9\n\nfn main():\n    print(g(Some(7)))\n",
    );
    assert_eq!(a.1, "7\n");
    assert_eq!(a.1, b.1);
}

/// (11) Tham số có tên `--n: int? = 3` (default plain int) — default nâng (interp + VM).
#[test]
fn vd228_param_named_default_lift() {
    let [a, b] = chay_ca_hai(
        "param_def",
        "fn h(--n: int? = 3) -> int:\n    match n:\n        Some(v): v\n        None: 9\n\nfn main():\n    print(h())\n",
    );
    assert_eq!(a.1, "3\n");
    assert_eq!(a.1, b.1);
}

/// (12) `str?` với default `\"mặc định\"` (named, plain str) — default nâng.
#[test]
fn vd228_param_named_str_default_lift() {
    let [a, b] = chay_ca_hai(
        "param_def_s",
        "fn k(--q: str? = \"mac\") -> str:\n    match q:\n        Some(v): v\n        None: \"trong\"\n\nfn main():\n    print(k())\n",
    );
    assert_eq!(a.1, "mac\n");
    assert_eq!(a.1, b.1);
}

/// (13) Closure `fn(x) -> int?:` thân plain — biên `-> T?` lift (interp + VM, byte-identical).
#[test]
fn vd228_closure_ret_lift() {
    let [a, b] = chay_ca_hai(
        "closure_ret",
        "fn main():\n    let f = fn(x) -> int?:\n        x\n    let r = f(41)\n    match r:\n        Some(v): print(\"some-{v}\")\n        None: print(\"none\")\n",
    );
    assert_eq!(a.0, 0, "interp: {}", a.2);
    assert_eq!(a.1, "some-41\n");
    assert_eq!(a.1, b.1);
}

/// (14) S1 (§7, VD-232) — repro chính xác `fn f(i: int) -> int?:` thân `if i < 1: 42 else: None`
/// + `match f(): Some(v) ...` — hai engine in cùng byte.
#[test]
fn vd228_s1_exact_repro() {
    let [a, b] = chay_ca_hai(
        "s1",
        "fn f(i: int) -> int?:\n    if i < 1:\n        42\n    else:\n        None\n\nfn main():\n    match f(0):\n        Some(v): print(\"lifted {v}\")\n        None: print(\"none\")\n",
    );
    assert_eq!(a.0, 0, "interp: {}", a.2);
    assert_eq!(a.1, "lifted 42\n");
    assert_eq!(a.1, b.1);
}

/// (15) `T??` — `Some(Some(5))` không bọc hai lớp; `None` (ngoài) vẫn phẳng (biên lift).
#[test]
fn vd228_nested_opt_real() {
    // **VD-241 §3 (A0, 2026-11-19)** — `T??` là `Option[Option[T]]` thật (không làm phẳng);
    // `None` trần chỉ ở `T?` — thay cho quy tắc "nhiều lớp" của VD-178 R2 đã bị A0 huỷ.
    let [a, b] = chay_ca_hai(
        "nested_opt_real",
        "fn on() -> int??:\n    Some(None)\n\nfn two() -> int??:\n    Some(Some(5))\n\nfn main():\n    print(on())\n    print(two())\n",
    );
    assert_eq!(a.1, "Some(None)\nSome(Some(5))\n");
    assert_eq!(a.1, b.1);
}

/// (16) `T??` khớp cả `let`-lift (VD-227): `let v: (int?)? = Some(Some(2))` không bọc đôi.
#[test]
fn vd228_nested_let_flat() {
    let [a, b] = chay_ca_hai(
        "nested_let",
        "fn main():\n    let v: (int?)? = Some(Some(2))\n    match v:\n        Some(inner):\n            match inner:\n                Some(n): print(\"nested-{n}\")\n                None: print(\"none-inner\")\n        None: print(\"none-outer\")\n",
    );
    assert_eq!(a.0, 0, "interp: {}", a.2);
    assert_eq!(a.1, "nested-2\n");
    assert_eq!(a.1, b.1);
}
