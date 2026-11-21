// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (eq-record)** — tối ưu `Value::eq_value` cho bản ghi cùng tên kiểu **không** đổi
//! ngữ nghĩa quan sát được: bản ghi ẩn danh, khác tên kiểu và khác thứ tự trường vẫn so theo
//! đa tập như cũ. (Trường **trùng tên** chỉ dựng được ở interp qua `let f = U; f(a: 1, a: 2)`
//! — VM chưa hỗ trợ dùng ctor làm giá trị — nên ca đó khoá ở unit test `value.rs`.)
//!
//! Ca này chạy **cả hai engine** và đòi byte-identical; nó khoá đường fallback khi thứ tự
//! trường lệch — đúng đường mà fast path theo vị trí phải nhường lại.

use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

static DEM: AtomicU32 = AtomicU32::new(0);

fn tam() -> PathBuf {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-vd270-eq-rec-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("tạo thư mục tạm");
    d
}

#[test]
fn eq_record_giu_ngu_nghia_da_tap_ca_hai_duong() {
    let d = tam();
    let p = d.join("eq_record.vis");
    std::fs::write(
        &p,
        r#"# SPDX-License-Identifier: MIT OR Apache-2.0
type U:
    a: int
    b: int

fn main():
    let p = U(a: 1, b: 2)
    let q = U(b: 2, a: 1)
    print(p == q)

    let z = U(a: 3, b: 4)
    let w = U(a: 3, b: 4)
    print(z == w)
    let khac = U(a: 3, b: 5)
    print(z == khac)

    let anon1 = {b: 1, a: 2}
    let anon2 = {a: 2, b: 1}
    print(anon1 == anon2)
"#,
    )
    .expect("ghi tệp tạm");

    let chay = |vm: bool| -> (i32, String) {
        let mut c = Command::new(env!("CARGO_BIN_EXE_vis"));
        c.arg("run").arg("--no-cache");
        if vm {
            c.arg("--vm");
        }
        c.arg(&p);
        let o = c.output().expect("chạy vis run");
        (
            o.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&o.stdout).to_string(),
        )
    };
    let a = chay(false);
    let b = chay(true);
    let _ = std::fs::remove_dir_all(&d);
    assert_eq!(a, b, "hai đường lệch");
    assert_eq!(a.0, 0, "{}", a.1);
    assert_eq!(a.1, "true\ntrue\nfalse\ntrue\n");
}
