// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (parse-int)** — `std.str.parse_int(s) -> int?`.
//!
//! Hợp đồng: parse **thập phân chặt** như `i64::from_str` (dấu `+`/`-` tuỳ chọn + chữ số ASCII);
//! sai cú pháp/tràn ⇒ `None`; `i64::MIN`/`MAX` biên vẫn `Some`; sugar `s.parse_int()`.
//! Mọi ca chạy cả hai engine và so byte stdout/stderr/exit.
//!
//! **RED-BEFORE**: trước khi cài, `VS0301 module 'std.str' không có hàm 'parse_int'`.

use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

static DEM: AtomicU32 = AtomicU32::new(0);

fn tam() -> PathBuf {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-vd270-parse-int-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("tạo thư mục tạm");
    d
}

fn chay(src: &str, vm: bool) -> (i32, String, String) {
    let d = tam();
    let p = d.join("main.vis");
    std::fs::write(
        &p,
        format!("# SPDX-License-Identifier: MIT OR Apache-2.0\n{src}"),
    )
    .expect("ghi");
    let mut c = Command::new(env!("CARGO_BIN_EXE_vis"));
    c.arg("run").arg("--no-cache");
    if vm {
        c.arg("--vm");
    }
    let o = c.arg(&p).output().expect("chạy `vis run`");
    let _ = std::fs::remove_dir_all(&d);
    (
        o.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&o.stdout).to_string(),
        String::from_utf8_lossy(&o.stderr).to_string(),
    )
}

fn hai_engine(src: &str) -> (i32, String, String) {
    let a = chay(src, false);
    let b = chay(src, true);
    assert_eq!(a, b, "interp ≠ `--vm`\n  interp: {a:?}\n  vm: {b:?}");
    a
}

#[test]
fn parse_hop_le_va_sugar() {
    const SRC: &str = r#"use std.str

fn main():
    print(str.parse_int("42"))
    print(str.parse_int("+7"))
    print(str.parse_int("-0"))
    print(str.parse_int("007"))
    print("123".parse_int())
    print("9223372036854775807".parse_int())
    print("-9223372036854775808".parse_int())
"#;
    let (ma, out, err) = hai_engine(SRC);
    assert_eq!(ma, 0, "stderr: {err}");
    assert_eq!(
        out,
        "Some(42)\nSome(7)\nSome(0)\nSome(7)\nSome(123)\nSome(9223372036854775807)\nSome(-9223372036854775808)\n"
    );
}

#[test]
fn parse_sai_va_tran_tra_none() {
    const SRC: &str = r#"use std.str

fn main():
    print(str.parse_int(""))
    print(str.parse_int(" 42"))
    print(str.parse_int("42 "))
    print(str.parse_int("0x10"))
    print(str.parse_int("1_000"))
    print(str.parse_int("4.2"))
    print(str.parse_int("abc"))
    print(str.parse_int("9223372036854775808"))
    print(str.parse_int("-9223372036854775809"))
"#;
    let (ma, out, err) = hai_engine(SRC);
    assert_eq!(ma, 0, "stderr: {err}");
    assert_eq!(
        out,
        "None\nNone\nNone\nNone\nNone\nNone\nNone\nNone\nNone\n"
    );
}

#[test]
fn sai_kieu_va_arity_bi_checker_chan() {
    for src in [
        "fn main():\n    print(1.parse_int())\n",
        "use std.str\n\nfn main():\n    print(str.parse_int(1))\n",
    ] {
        let (ma, _out, err) = hai_engine(src);
        assert_eq!(ma, 2, "checker phải chặn trước khi chạy; stderr: {err}");
        assert!(err.contains("VS04"), "phải có chẩn đoán VS04: {err}");
    }
}
