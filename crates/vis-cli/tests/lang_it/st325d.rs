// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-325d (nợ #2)** — `match` ở **ngữ cảnh giá trị** rơi khỏi mọi nhánh phải trap `VS0501`.
//!
//! Trước lát này: tyck xét `Some(1)` + `None` là phủ (chỉ so **tên ctor**), nhưng giá trị
//! `Some(2)` không khớp nhánh nào. **Interp** trap `VS0501` (exit 101); **VM** đẩy `false`
//! rồi exit 0 — lệch engine. Codegen nhánh câu lệnh đã trap từ ST-238; lát này kéo nhánh
//! **giá trị** (`codegen/expr.rs`) lên cùng hành vi.

use std::process::Command;

const S: &str = "# SPDX-License-Identifier: MIT OR Apache-2.0\n";
const MSG: &str = "`match` không khớp nhánh nào (không xét hết bị lọt)";

fn tam(ten: &str, body: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("vis-st325d-{ten}-{}", std::process::id()));
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

/// Ca tối thiểu (#2): `check` xanh (tyck tưởng phủ), nhưng runtime rơi ⇒ **cả hai** engine
/// cùng trap `VS0501`, cùng exit 101, không in `false`.
#[test]
fn value_match_fallthrough_hai_engine_cung_vs0501() {
    let p = tam(
        "roi",
        "fn main():\n    let x: int? = Some(2)\n    let v = match x:\n        Some(1): 10\n        None: 20\n    print(v)\n",
    );
    let ps = p.to_str().expect("utf8");

    // Tiền điều kiện: ca này lọt qua tyck — không được đỏ VS0402.
    let (cc, co, ce) = chay(&["check", ps]);
    assert_eq!(
        cc, 0,
        "`check` phải xanh để ca runtime có ý nghĩa: {ce} / {co}"
    );

    let (ci, oi, ei) = chay(&["run", ps]);
    let (cv, ov, ev) = chay(&["run", "--vm", "--no-cache", ps]);
    assert_eq!(ci, cv, "exit lệch: interp {ci} ({ei}) vs VM {cv} ({ev})");
    assert_eq!(ci, 101, "trap VS0501 ⇒ exit 101: interp {ei} / VM {ev}");
    assert!(ei.contains("VS0501"), "interp: {ei}");
    assert!(ev.contains("VS0501"), "VM: {ev}");
    assert!(ei.contains(MSG), "thông điệp interp: {ei}");
    assert!(ev.contains(MSG), "thông điệp VM: {ev}");
    assert_eq!(oi, "", "không được in `false`: interp {oi}");
    assert_eq!(ov, "", "không được in `false`: VM {ov}");
}

/// Đối chứng dương: nhánh khớp thật vẫn trả giá trị, hai engine cùng in `10`.
#[test]
fn value_match_khop_van_tra_gia_tri() {
    let p = tam(
        "khop",
        "fn main():\n    let x: int? = Some(1)\n    let v = match x:\n        Some(1): 10\n        None: 20\n    print(v)\n",
    );
    let ps = p.to_str().expect("utf8");
    let (ci, oi, ei) = chay(&["run", ps]);
    let (cv, ov, ev) = chay(&["run", "--vm", "--no-cache", ps]);
    assert_eq!((ci, oi.clone()), (0, "10\n".to_string()), "interp: {ei}");
    assert_eq!((cv, ov.clone()), (0, "10\n".to_string()), "vm: {ev}");
}

/// Đối chứng nhánh **câu lệnh** (đã đúng từ ST-238) — giữ parity, không để lát sau hỏng lại.
#[test]
fn statement_match_fallthrough_hai_engine_cung_vs0501() {
    let p = tam(
        "cau",
        "fn main():\n    let x: int? = Some(2)\n    match x:\n        Some(1): print(10)\n        None: print(20)\n    print(30)\n",
    );
    let ps = p.to_str().expect("utf8");
    let (ci, oi, ei) = chay(&["run", ps]);
    let (cv, ov, ev) = chay(&["run", "--vm", "--no-cache", ps]);
    assert_eq!(ci, cv, "exit lệch: interp {ci} ({ei}) vs VM {cv} ({ev})");
    assert_eq!(ci, 101, "trap VS0501 ⇒ exit 101: interp {ei} / VM {ev}");
    assert!(ei.contains("VS0501") && ei.contains(MSG), "interp: {ei}");
    assert!(ev.contains("VS0501") && ev.contains(MSG), "VM: {ev}");
    assert_eq!(oi, "", "không được in sau trap: interp {oi}");
    assert_eq!(ov, "", "không được in sau trap: VM {ov}");
}
