// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-217** (VD-204) — ca khoá cho tên dự trữ của enum std `Option`/`Result`.
//!
//! (1) **Mới** (đỏ-trước): `Option`/`Result`/`Some`/`None`/`Ok`/`Err` **không** được khai lại hay
//!     che tên ở **phạm vi toàn cục** ⇒ **VS0302** (mã `NAME_CODES[1]`) — lỗi **có mã**.
//! (2) **Hồi quy** (xanh trước và sau): `match` trên `Option` thiếu nhánh ⇒ **VS0402**.
//! (3) **Hồi quy** (xanh trước và sau): `?` hành xử không đổi.
//!
//! Chú ý: `vis check` in TAP ra **stdout** (chẩn đoán trên dòng `# …`), thoát 1 khi lỗi.

use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

static DEM: AtomicU32 = AtomicU32::new(0);

fn tam(tag: &str) -> PathBuf {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-st217-{tag}-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("tạo thư mục tạm");
    d
}

/// Chạy `vis check` trên `src`; trả về `(exit, stdout)`.
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

#[test]
fn ten_du_tru_toan_cuc_bi_tu_choi() {
    for (tag, src, mot) in [
        ("type-option", "type Option:\n    a: int\n", None),
        ("type-result", "type Result:\n    a: int\n", None),
        ("fn-option", "fn Option(x: int) -> int:\n    x\n", None),
        ("fn-result", "fn Result(x: int) -> int:\n    x\n", None),
        ("fn-some", "fn Some(x: int) -> int:\n    x\n", None),
        ("fn-none", "fn None() -> int:\n    1\n", None),
        ("fn-ok", "fn Ok(x: int) -> int:\n    x\n", None),
        ("fn-err", "fn Err(x: int) -> int:\n    x\n", None),
        ("enum-option", "enum Option:\n    A\n", None),
        ("enum-result", "enum Result:\n    A\n", None),
        ("enum-some", "enum Some:\n    A\n", None),
        ("enum-none", "enum None:\n    A\n", None),
        ("enum-ok", "enum Ok:\n    A\n", None),
        ("enum-err", "enum Err:\n    A\n", None),
        ("const-ok", "const Ok: int = 1\n", None),
        (
            "let-err",
            "let Err = 1\n",
            // `let <Ctor> = …` là **pattern**, không phải khai báo ⇒ VS0412 (lỗi có mã); vẫn bị từ chối.
            Some("VS0412"),
        ),
    ] {
        let (code, out) = check(tag, src);
        assert_ne!(code, 0, "[{tag}] phải bị từ chối: {out}");
        let can = mot.unwrap_or("VS0302");
        assert!(
            out.contains(can),
            "[{tag}] phải báo {can} (mã có tên dự trữ): {out}"
        );
    }
}

#[test]
fn ten_du_tru_khong_chan_ben_trong_ham() {
    // VD-204 (4): dự trữ **chỉ ở phạm vi toàn cục** — tham số/`var` trong hàm vẫn tự do.
    let src = "fn f(Ok: int, Err: int, Some: int, None: int) -> int:\n    var Option = Ok\n    var Result = Err\n    Option + Result + Some + None\nprint(f(1, 2, 3, 4))\n";
    let (code, out) = check("local-ok", src);
    assert_eq!(code, 0, "[local-ok] không được báo: {out}");
}

#[test]
fn match_option_thieu_nhanh_la_loi_hoi_quy() {
    // Hồi quy — phải xanh **cả trước và sau** (VS0402 đã là hành vi hiện tại).
    let src = concat!(
        "let x: int? = Some(5)\n",
        "match x:\n",
        "    Some(v): print(v)\n",
    );
    let (code, out) = check("match-thieu", src);
    assert_ne!(code, 0, "match thiếu nhánh phải bị từ chối: {out}");
    assert!(out.contains("VS0402"), "phải báo VS0402: {out}");
}

#[test]
fn toan_tu_hoi_quy_van_dung() {
    // Hồi quy: `?` dùng được trên `Option[int]` trong hàm trả `Option[int]` — hành xử không đổi.
    let src = concat!(
        "fn g() -> int?:\n    let x: int? = Some(21)\n    print(x)\n    x\n",
        "g()\n",
    );
    let (code, out) = check("try-ok", src);
    assert_eq!(code, 0, "không được báo: {out}");
}
