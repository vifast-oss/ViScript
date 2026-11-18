// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-217 / VD-218** (A0 2026-11-18) — tên dựng sẵn nội bộ `$value:<tên>` **không lộ** ở
//! bất kỳ bề mặt nào người dùng nhìn thấy (thông báo lỗi, stack trace, output hàm, hover,
//! disassemble). `print`/lỗi phải trả cùng một chuỗi ở interp và VM.

use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

static DEM: AtomicU32 = AtomicU32::new(0);

fn tam(tag: &str) -> PathBuf {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-vd218-{tag}-{}-{n}", std::process::id()));
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

#[test]
fn builtin_len_as_value_khong_lo_prefix() {
    // Lỗi danh số gọi builtin-as-value: thông báo chỉ in tên trần, không in `$value:`.
    // (ViScript đòi kiểu tham số hàm — `f: fn(int, int) -> int`. Hai engine chạy **cùng tệp**
    // nên đường dẫn trong thông báo giống nhau byte-đối-byte.)
    let d = tam("arity_shared");
    let p = d.join("arity.vis");
    std::fs::write(&p, "# SPDX-License-Identifier: MIT OR Apache-2.0\nfn h(f: fn(int, int) -> int) -> int:\n    f(1, 2)\nfn main():\n    print(h(Some))\n").expect("ghi tệp tạm");
    let chay_mot = |args: &[&str]| {
        let mut c = Command::new(env!("CARGO_BIN_EXE_vis"));
        c.arg("run").arg("--no-cache");
        c.args(args);
        c.arg(&p);
        let o = c.output().expect("chạy vis run");
        (
            o.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&o.stdout).to_string(),
            String::from_utf8_lossy(&o.stderr).to_string(),
        )
    };
    let a = chay_mot(&[]);
    let b = chay_mot(&["--vm"]);
    let _ = std::fs::remove_dir_all(&d);
    assert_eq!(
        a.1, b.1,
        "lỗi phải giống nhau giữa hai đường: {a:?} / {b:?}"
    );
    assert_eq!(a.2, b.2, "stderr phải giống nhau");
    assert!(
        !a.2.contains("$value"),
        "stderr không được lộ `$value:`: {}",
        a.2
    );
    assert_ne!(a.0, 0, "phải lỗi");
}

#[test]
fn in_gia_tri_some_o_ca_hai_duong() {
    let src = "fn main():\n    let f = Some\n    let g = Ok\n    print(f)\n    print(g)\n";
    let a = chay("print", src, &[]);
    let b = chay("print-vm", src, &["--vm"]);
    assert_eq!(a.1, b.1, "stdout phải giống nhau");
    assert!(a.1.contains("<builtin Some>"), "interp: {a:?}");
    assert_eq!(a.1, "<builtin Some>\n<builtin Ok>\n");
}

#[test]
fn disassemble_khong_lo_prefix() {
    let src = "fn main():\n    let f = Some\n    print(f)\n";
    let (_, out, err) = chay("dump", src, &["--vm", "--dump-bytecode"]);
    assert_eq!(out, "<builtin Some>\n", "stdout: {out}");
    assert!(
        !err.contains("$value"),
        "disassemble không được lộ `$value:`: {err}"
    );
    assert!(
        err.contains("<builtin Some>"),
        "disassemble in builtin: {err}"
    );
}
