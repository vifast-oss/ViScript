// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-217 / VD-212** (A0 2026-11-18) — pattern **phân biệt hoa thường**; sai hoa thường là lỗi.
//!
//! (1) Nguyên văn VD-212: `ok(s)`, `some(x)`, `none` trong pattern ⇒ **lỗi có mã** + gợi ý dạng đúng
//!     (**VS0203**). `none` đã là lỗi từ R2 (VS0203) — ca này là hồi quy.
//! (2) enum người dùng có hai biến thể `A` và `a`: mỗi arm khớp **đúng** biến thể, không lẫn.
//! (3) enum người dùng: pattern `circle(x)` (sai hoa thường) ⇒ **VS0203** + gợi ý `Circle`.
//! (4) `==` giữa hai enum khác tên vẫn là **VS0412** ở tầng kiểu (VD-211 không sinh nợ mới;
//!     ca khoá tầng chạy: enum hai loại cùng biến thể+cùng đối số không bằng nhau — vd211_enum_eq.rs).
//! (5) **VD-213**: `let Ctor = …` (pattern không khớp kiểu vô hốt) — thông báo VS0412 phải có gợi ý
//!     `match`/`if let`.
//!
//! Chạy **cả hai đường** (interp + `--vm`): ca chặn một đường là ca hỏng.

use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

static DEM: AtomicU32 = AtomicU32::new(0);

fn tam(tag: &str) -> PathBuf {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-vd212-{tag}-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("tạo thư mục tạm");
    d
}

/// Chạy `vis check` trên `src`; trả về `(exit, stdout)` (chẩn đoán in ra stdout TAP).
fn check(tag: &str, src: &str) -> (i32, String) {
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
    let out = String::from_utf8_lossy(&o.stdout).to_string();
    let _ = std::fs::remove_dir_all(&d);
    (o.status.code().unwrap_or(-1), out)
}

/// Chạy cả hai đường; đòi **cùng** stdout/exit; trả về `(exit, stdout)` (stdout thuần của chương trình).
fn hai_duong(tag: &str, src: &str) -> (i32, String) {
    let d = tam(tag);
    let p = d.join(format!("{tag}.vis"));
    std::fs::write(
        &p,
        format!("# SPDX-License-Identifier: MIT OR Apache-2.0\n{src}"),
    )
    .expect("ghi tệp tạm");
    let chay = |args: &[&std::ffi::OsStr]| -> (i32, String) {
        let o = Command::new(env!("CARGO_BIN_EXE_vis"))
            .arg("run")
            .arg("--no-cache")
            .args(args)
            .arg(&p)
            .output()
            .expect("chạy vis run");
        (
            o.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&o.stdout).to_string(),
        )
    };
    let a = chay(&[]);
    let b = chay(&[std::ffi::OsStr::new("--vm")]);
    let _ = std::fs::remove_dir_all(&d);
    assert_eq!(a, b, "[{tag}] hai đường lệch");
    a
}

#[test]
fn pattern_sai_hoa_thuong_la_loi() {
    for (tag, src) in [
        ("ok-pattern", "fn g() -> int:\n    Err(1)\nfn main():\n    match g():\n        ok(v): print(v)\n        err(e): print(e)\n"),
        ("some-pattern", "fn main():\n    let x: int? = Some(5)\n    match x:\n        some(v): print(v)\n        None: print(0)\n"),
        ("none-pattern", "fn main():\n    let x: int? = None\n    match x:\n        none: print(0)\n        Some(v): print(v)\n"),
    ] {
        let (code, out) = check(tag, src);
        assert_ne!(code, 0, "[{tag}] phải bị từ chối: {out}");
        assert!(out.contains("VS0203"), "[{tag}] phải báo VS0203: {out}");
    }
}

#[test]
fn pattern_dung_hoa_thuong_van_chay_hai_duong() {
    let (code, out) = hai_duong(
        "dung-hoa-thuong",
        "fn main():\n    let x: int? = Some(21)\n    match x:\n        Some(v): print(v)\n        None: print(0)\n",
    );
    assert_eq!(code, 0, "{out}");
    assert_eq!(out, "21\n");
}

#[test]
fn enum_hai_bien_the_a_va_hoa_a() {
    let src = concat!(
        "enum E:\n    A\n    a\n",
        "fn main():\n    let x = E.A\n    let y = E.a\n",
        "    match x:\n        a: print(\"a\")\n        A: print(\"A\")\n",
        "    match y:\n        a: print(\"a\")\n        A: print(\"A\")\n",
    );
    // Chạy cả hai đường — `A` khớp `A`, `a` khớp `a` (không lẫn).
    let (code, out) = hai_duong("a-a", src);
    assert_eq!(code, 0, "{out}");
    assert_eq!(out, "A\na\n");
}

#[test]
fn enum_bien_the_sai_hoa_thuong_la_loi() {
    let src = "enum E:\n    Circle(int)\nfn main():\n    match E.Circle(3):\n        circle(r): print(r)\n";
    let (code, out) = check("circle-case", src);
    assert_ne!(code, 0, "{out}");
    assert!(out.contains("VS0203"), "phải báo VS0203: {out}");
    assert!(out.contains("Circle"), "phải gợi ý `Circle`: {out}");
}

#[test]
fn let_ctor_khong_khop_kieu_has_goi_y() {
    // VD-213: mệnh đề `let Ctor = …` là pattern; khi không khớp phải có gợi ý `match`/`if let`.
    let (code, out) = check("let-err", "let Err = 1\n");
    assert_ne!(code, 0, "{out}");
    assert!(
        out.contains("match") && out.contains("if let"),
        "phải có gợi ý `match`/`if let`: {out}"
    );
}
