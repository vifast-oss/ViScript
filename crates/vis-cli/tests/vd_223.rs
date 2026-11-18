// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-209 / R5d (VD-223, VD-224 — uỷ quyền A0-proxy 2026-11-19)** — xung đột tên std module.
//!
//! **VD-224** huỷ VD-223(i) (VS0209): `--b` hợp lệ ở **mọi** hàm (docs §5.1); **giữ** cảnh báo
//! `VS0211` ở khai báo trùng tên module std đã import (VD-223(ii)).
//! Hai engine phải byte-identical.

use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

static DEM: AtomicU32 = AtomicU32::new(0);

fn tam(tag: &str) -> PathBuf {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-vd223-{tag}-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("tạo thư mục tạm");
    d
}

fn chay_ca_hai(tag: &str, src: &str) -> [(u32, String, String); 2] {
    let d = tam(tag);
    let p = d.join(format!("{tag}.vis"));
    std::fs::write(
        &p,
        format!("# SPDX-License-Identifier: MIT OR Apache-2.0\n{src}"),
    )
    .expect("ghi tệp tạm");
    let mo = |extra: &[&str]| {
        let mut c = Command::new(env!("CARGO_BIN_EXE_vis"));
        c.arg("run").arg("--no-cache");
        c.args(extra);
        c.arg(&p);
        let o = c.output().expect("chạy vis run");
        (
            o.status.code().unwrap_or(-1) as u32,
            String::from_utf8_lossy(&o.stdout).to_string(),
            String::from_utf8_lossy(&o.stderr).to_string(),
        )
    };
    let a = mo(&[]);
    let b = mo(&["--vm"]);
    let _ = std::fs::remove_dir_all(&d);
    [a, b]
}

/// (1) `--b` tham số **có tên** hợp lệ ở hàm thường (docs §5.1): gọi `f(--b: true)` chạy đúng.
#[test]
fn ten_cu_phap_flag_hop_le_ca_hai_duong() {
    let src = "fn f(--b: bool) -> bool:\n    b\n\nfn main():\n    print(f(--b: true))\n    print(f(--b: false))\n";
    let [a, b] = chay_ca_hai("flag", src);
    for r in [&a, &b] {
        assert_eq!(r.0, 0, "{r:?}");
        assert_eq!(r.1, "true\nfalse\n");
    }
}

/// (2) `let str = …` với `use std.str` ⇒ **cảnh báo `VS0211`** (không chặn), cả hai đường cùng byte.
#[test]
fn trung_ten_module_std_ra_vs0211() {
    let d = tam("shadow");
    let p = d.join("shadow.vis");
    std::fs::write(&p, "# SPDX-License-Identifier: MIT OR Apache-2.0\nuse std.str\n\nfn main():\n    let str = \"z\"\n    print(str)\n").expect("ghi");
    for engine in ["", "--vm"] {
        let mut c = Command::new(env!("CARGO_BIN_EXE_vis"));
        c.arg("run").arg("--no-cache");
        if !engine.is_empty() {
            c.arg(engine);
        }
        c.arg(&p);
        let o = c.output().expect("vis run");
        assert_eq!(
            o.status.code().unwrap_or(-1),
            0,
            "thoát 0 (cảnh báo không chặn)"
        );
        let tho = String::from_utf8_lossy(&o.stderr).to_string();
        assert!(tho.contains("VS0211"), "phải có VS0211: {tho}");
    }
    let _ = std::fs::remove_dir_all(&d);
}

/// (3) Không `use std.str` ⇒ không cảnh báo VS0211 (tên `str` bình thường).
#[test]
fn khong_use_thi_khong_canh_bao() {
    let src = "fn main():\n    let str = \"z\"\n    print(str)\n";
    let [a, b] = chay_ca_hai("nomod", src);
    for r in [&a, &b] {
        assert_eq!(r.0, 0);
        assert_eq!(r.1, "z\n");
        assert!(
            !r.2.contains("VS0211"),
            "không có VS0211 khi chưa use: {}",
            r.2
        );
    }
}

/// (4) **std module call giữ nguyên**: `str.upper(...)` khi đã `use std.str` vẫn là std, không upstream.
#[test]
fn std_call_van_giu_nguyen_sau_dang_ky_bien() {
    let src = "use std.str\n\nfn main():\n    print(str.upper(\"việt\"))\n    let s = \"z\"\n    print(str.join([\"a\", \"b\"], \"-\"))\n";
    let [a, b] = chay_ca_hai("keepstd", src);
    for r in [&a, &b] {
        assert_eq!(r.0, 0, "{r:?}");
        assert_eq!(r.1, "VIỆT\na-b\n");
    }
}

/// (5) `parse(std.str)` khi `str` là biến: builtin `str(…)` là gọi trần ⇒ lỗi tĩnh `VS0412` (không nuốt).
#[test]
fn goi_tran_str_sau_bien_tru_vs0412() {
    let d = tam("callshadow");
    let p = d.join("callshadow.vis");
    std::fs::write(&p, "# SPDX-License-Identifier: MIT OR Apache-2.0\nuse std.str\n\nfn main():\n    let str = \"z\"\n    print(str(\"a\"))\n").expect("ghi");
    let o = Command::new(env!("CARGO_BIN_EXE_vis"))
        .arg("run")
        .arg(&p)
        .output()
        .expect("vis run");
    let stderr = String::from_utf8_lossy(&o.stderr).to_string();
    let tho = String::from_utf8_lossy(&o.stdout).to_string();
    let tho2 = format!("{tho}{stderr}");
    assert_ne!(
        o.status.code().unwrap_or(-1),
        0,
        "không được chạy thành công"
    );
    assert!(tho2.contains("VS0412"), "phải có VS0412: {tho2}");
    let _ = std::fs::remove_dir_all(&d);
}
