// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-217 / VD-208** (A0 2026-11-18) — `Option`/`Result` **hành xử như enum std** (tầng chạy).
//! Gộp lần đóng ST-217. Các ca VD-208:
//! 1. match Option/Result thiếu nhánh ⇒ lỗi VS0402 (hồi quy; cũng ở `test/sem/result/thieu-nhanh.err`)
//! 2. `Some`/`None`/`Ok`/`Err` dùng được như **hàm** (gán biến, truyền `map`, cả interp + VM)
//! 3. Không khai lại/che tên ở phạm vi toàn cục (ca khoá: `tests/vd_204.rs`)
//! 4. Thông báo lỗi/LSP hover giữ `int?` (ca khoá: `tests/lsp.rs::lsp_hover_option_in_int`)
//!
//! Chú ý: user-enum ctor làm giá trị vẫn **chưa** được hỗ trợ (`E.A` ⇒ VS0412, cả hai đường) —
//! Option **vượt** enum người dùng về dùng-như-hàm, không cần A0; không phải ca của VD-208.

use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

static DEM: AtomicU32 = AtomicU32::new(0);

fn tam(tag: &str) -> PathBuf {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-vd208-{tag}-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("tạo thư mục tạm");
    d
}

fn hai_duong(tag: &str, src: &str) -> (i32, String) {
    let d = tam(tag);
    let p = d.join(format!("{tag}.vis"));
    std::fs::write(
        &p,
        format!("# SPDX-License-Identifier: MIT OR Apache-2.0\n{src}"),
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
    assert_eq!(a, b, "[{tag}] hai đường lệch");
    a
}

#[test]
fn ctor_dung_nhu_ham_ca_hai_duong() {
    let (code, out) = hai_duong(
        "ctor-fn",
        "fn main():\n    let f = Some\n    print(f(5))\n    let xs = [1, 2]\n    print(xs.map(Some))\n",
    );
    assert_eq!(code, 0, "{out}");
    assert_eq!(out, "Some(5)\n[Some(1), Some(2)]\n");

    let (code, out) = hai_duong(
        "ctor-fn2",
        "fn main():\n    let f = Ok\n    let g = Err\n    print(f(1))\n    print(g(\"hỏng\"))\n    let h = None\n    print(h)\n",
    );
    assert_eq!(code, 0, "{out}");
    assert_eq!(out, "Ok(1)\nErr(hỏng)\nNone\n");
}
