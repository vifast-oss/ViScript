// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-316b (`#186`)** — chẩn đoán interp ↔ VM thống nhất cho chương trình **không hợp lệ** có
//! cả `let` cấp tệp lẫn `fn main`.
//!
//! Trước lát này: `x = 2` với `let x` khai ở cấp tệp bị **interp** trap lúc chạy
//! (`VS0413`, exit 1) nhưng **VM** im lặng ghi vào ô global (exit 0) — tức một bên vẫn thực thi
//! tác dụng phụ. Nay tyck đánh dấu ràng buộc `let` cấp tệp trước khi kiểm thân hàm ⇒ **cả hai**
//! từ chối **tĩnh** với cùng mã/thông điệp/vị trí (exit 2 như mọi lỗi kiểu).

use std::process::Command;

const S: &str = "# SPDX-License-Identifier: MIT OR Apache-2.0\n";

fn tam(ten: &str, body: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("vis-st316b-{ten}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("thư mục tạm");
    let p = d.join(format!("{ten}.vis"));
    std::fs::write(&p, format!("{S}{body}")).expect("ghi");
    p
}

fn chay(args: &[&str]) -> (i32, String, String) {
    let o = Command::new(env!("CARGO_BIN_EXE_vis"))
        .args(args)
        .output()
        .expect("chạy `vis`");
    (
        o.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&o.stdout).to_string(),
        String::from_utf8_lossy(&o.stderr).to_string(),
    )
}

/// `let` cấp tệp + gán trong `fn main`: hai engine **cùng** `VS0413`, cùng vị trí, **cùng**
/// thông điệp; `vis check` cũng đỏ (lỗi tĩnh, không phải lỗi chạy).
#[test]
fn gan_vao_let_cap_tep_hai_engine_cung_tu_choi() {
    let p = tam(
        "letcap",
        "let x: int = 1\nfn main():\n    x = 2\n    print(x)\n",
    );
    let ps = p.to_str().expect("utf8");
    let (ci, oi, ei) = chay(&["run", ps]);
    let (cv, ov, ev) = chay(&["run", "--vm", "--no-cache", ps]);
    assert_eq!(ci, cv, "exit lệch: interp {ci} ({ei}) vs VM {cv} ({ev})");
    assert_eq!(ci, 2, "lỗi tĩnh phải exit 2: interp {ei} / VM {ev}");
    assert_eq!(ei, ev, "stderr phải **y hệt** (mã + thông điệp + vị trí)");
    assert!(ei.contains("VS0413"), "phải nêu VS0413: {ei}");
    assert!(
        ei.contains("không gán được cho `let`/tham số"),
        "thông điệp phải khớp: {ei}"
    );
    assert!(ei.contains(":4:5:"), "vị trí phải ở `x` dòng 4 cột 5: {ei}");
    assert_eq!(oi, "", "không được in gì trước khi từ chối: {oi}");
    assert_eq!(ov, "");
    // `vis check` cùng kết luận (đỏ) — trước lát này `check` xanh oan.
    let (cc, oc, _) = chay(&["check", ps]);
    assert_ne!(cc, 0, "`vis check` phải đỏ: {oc}");
    assert!(oc.contains("VS0413"), "check: {oc}");
}

/// Ràng buộc `let` bằng **pattern** ở cấp tệp cũng bất biến (không chỉ `HPat::Bind` trần).
#[test]
fn gan_vao_let_pattern_cap_tep_cung_tu_choi() {
    let p = tam(
        "letpat",
        "let (a, b) = (1, 2)\nfn main():\n    a = 3\n    print(b)\n",
    );
    let ps = p.to_str().expect("utf8");
    let (ci, _, ei) = chay(&["run", ps]);
    let (cv, _, ev) = chay(&["run", "--vm", "--no-cache", ps]);
    assert_eq!(ci, cv, "exit lệch: {ei} vs {ev}");
    assert_eq!(ci, 2, "{ei} / {ev}");
    assert_eq!(ei, ev);
    assert!(ei.contains("VS0413"), "{ei}");
}

/// Đối chứng dương: `var` cấp tệp **được** gán trong `main`, hai engine cùng in `2`.
#[test]
fn var_cap_tep_van_gan_duoc_hai_engine() {
    let p = tam(
        "varcap",
        "var x: int = 1\nfn main():\n    x = 2\n    print(x)\n",
    );
    let ps = p.to_str().expect("utf8");
    let (ci, oi, ei) = chay(&["run", ps]);
    let (cv, ov, ev) = chay(&["run", "--vm", "--no-cache", ps]);
    assert_eq!((ci, oi.clone()), (0, "2\n".to_string()), "interp: {ei}");
    assert_eq!((cv, ov.clone()), (0, "2\n".to_string()), "vm: {ev}");
}
