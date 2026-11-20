// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-266 / int literal** — `i64::MIN` viết trực tiếp phải hợp lệ, còn **độ lớn** `i64::MAX + 1`
//! không dấu vẫn là lỗi **tĩnh** `VS0104`.
//!
//! Bối cảnh: lexer cũ giải mã thẳng vào `i64` nên từ chối cả `-9223372036854775808`,
//! `-0x8000000000000000`, `-0b1000…` — dù dấu `-` là toán tử một ngôi và về mặt giá trị hoàn toàn
//! biểu diễn được. VD-266 cho phép **độ lớn** `i64::MAX + 1` ở tầng token, gộp dấu `-` một ngôi
//! quanh literal số ở tầng hạ, và để tầng kiểm kiểu từ chối literal **dương** `9223372036854775808`
//! (cùng `0x8000000000000000`/dạng nhị phân) bằng `VS0104`.
//!
//! Mọi ca chạy **cả hai** engine (`vis run` và `vis run --vm --no-cache`), so **byte**
//! stdout/stderr/exit. Ca lỗi tĩnh kiểm thêm `vis check` (exit 1 theo ST-54) và `vis run`
//! (exit 2) — không đổi quy ước exit hiện hành.
//!
//! `i64::MIN // -1` và `i64::MIN % -1` phải **trap** `VS0507` (exit 101) trên cả hai engine,
//! không panic Rust. (`/` luôn ra `float` theo R7 nên không thuộc ca tràn số nguyên.)
//!
//! **Đỏ-trước** (main `40063f8`, chưa sửa): chạy cả tệp được **5 đỏ / 3 xanh** — 5 ca MIN
//! (`min_thap_phan_hex_nhi_phan…`, `min_co_gach_duoi…`, `max_va_min_round_trip…`,
//! `ba_dau_tru_lien_tiep…`, `min_chia_nguyen_va_du…`) đều chết vì lexer trả `VS0104` cho
//! `9223372036854775808`; 3 ca còn lại là ca **từ chối** nên lexer cũ đã "xanh" sẵn.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

static DEM: AtomicU32 = AtomicU32::new(0);

fn tam(tag: &str) -> PathBuf {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-vd266-{tag}-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("tạo thư mục tạm");
    d
}

fn ghi(tag: &str, src: &str) -> (PathBuf, PathBuf) {
    let d = tam(tag);
    let p = d.join(format!("{tag}.vis"));
    std::fs::write(
        &p,
        format!("# SPDX-License-Identifier: MIT OR Apache-2.0\n{src}"),
    )
    .expect("ghi tệp tạm");
    (d, p)
}

fn chay(args: &[&OsStr]) -> (i32, String, String) {
    let o = Command::new(env!("CARGO_BIN_EXE_vis"))
        .args(args)
        .output()
        .expect("chạy vis");
    (
        o.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&o.stdout).to_string(),
        String::from_utf8_lossy(&o.stderr).to_string(),
    )
}

fn chay_check(p: &Path) -> (i32, String, String) {
    chay(&[OsStr::new("check"), p.as_os_str()])
}

fn chay_run(p: &Path, vm: bool) -> (i32, String, String) {
    let mut a: Vec<&OsStr> = vec![OsStr::new("run"), OsStr::new("--no-cache")];
    if vm {
        a.push(OsStr::new("--vm"));
    }
    a.push(p.as_os_str());
    chay(&a)
}

/// Chương trình hợp lệ: `check` sạch, hai engine chạy exit 0, stdout đúng, stderr rỗng,
/// và hai engine **byte-identical**.
fn chay_sach(tag: &str, src: &str, want: &str) {
    let (d, p) = ghi(tag, src);
    let (cc, co, ce) = chay_check(&p);
    assert_eq!(
        cc, 0,
        "{tag}: `vis check` phải sạch\nstdout: {co}\nstderr: {ce}"
    );
    assert!(ce.is_empty(), "{tag}: `vis check` stderr phải rỗng: {ce}");

    let a = chay_run(&p, false);
    let b = chay_run(&p, true);
    let _ = std::fs::remove_dir_all(&d);
    assert_eq!(
        a, b,
        "{tag}: interp↔VM lệch byte (exit/stdout/stderr)\ninterp: {a:?}\nvm: {b:?}"
    );
    assert_eq!(
        a.0, 0,
        "{tag}: phải chạy sạch\nstdout: {}\nstderr: {}",
        a.1, a.2
    );
    assert!(a.2.is_empty(), "{tag}: stderr phải rỗng: {}", a.2);
    assert_eq!(a.1, want, "{tag}: stdout sai");
}

/// Lỗi **tĩnh** `VS0104`: `check` đỏ (exit 1 — ST-54), cả hai engine `run` từ chối (exit 2),
/// hai engine **byte-identical**, và không có ca nào chạm runtime (`VS05xx`).
fn tu_choi_tinh(tag: &str, src: &str) {
    let (d, p) = ghi(tag, src);
    let (cc, co, ce) = chay_check(&p);
    assert_eq!(
        cc, 1,
        "{tag}: `vis check` phải đỏ exit 1\nstdout: {co}\nstderr: {ce}"
    );
    assert!(
        co.contains("VS0104") || ce.contains("VS0104"),
        "{tag}: `vis check` thiếu VS0104\nstdout: {co}\nstderr: {ce}"
    );
    assert!(
        !(co.contains("VS050") || ce.contains("VS050")),
        "{tag}: lỗi phải là **tĩnh**, không phải runtime\nstdout: {co}\nstderr: {ce}"
    );

    let a = chay_run(&p, false);
    let b = chay_run(&p, true);
    let _ = std::fs::remove_dir_all(&d);
    assert_eq!(
        a, b,
        "{tag}: interp↔VM lệch byte khi từ chối tĩnh\ninterp: {a:?}\nvm: {b:?}"
    );
    assert_eq!(
        a.0, 2,
        "{tag}: `vis run` phải từ chối tĩnh exit 2, được {}\nstdout: {}\nstderr: {}",
        a.0, a.1, a.2
    );
    assert!(
        a.1.contains("VS0104") || a.2.contains("VS0104"),
        "{tag}: `vis run` thiếu VS0104\nstdout: {}\nstderr: {}",
        a.1,
        a.2
    );
    assert!(
        !(a.1.contains("VS050") || a.2.contains("VS050")),
        "{tag}: run không được chạm runtime\nstdout: {}\nstderr: {}",
        a.1,
        a.2
    );
}

/// Số nhị phân `2^63` — `0b1` + 63 số `0`.
fn bin_2_63() -> String {
    format!("0b1{}", "0".repeat(63))
}

#[test]
fn min_thap_phan_hex_nhi_phan_chay_hai_engine() {
    let bin = bin_2_63();
    let src = format!(
        "fn main():\n    print(-9223372036854775808)\n    print(-0x8000000000000000)\n    print(-{bin})\n"
    );
    chay_sach(
        "min-bases",
        &src,
        "-9223372036854775808\n-9223372036854775808\n-9223372036854775808\n",
    );
}

#[test]
fn min_co_gach_duoi_va_long_ngoac_van_la_min() {
    let src = r#"fn main():
    print(-9_223_372_036_854_775_808)
    print(-(9223372036854775808))
    print(-(0x8000000000000000))
"#;
    chay_sach(
        "min-underscore",
        src,
        "-9223372036854775808\n-9223372036854775808\n-9223372036854775808\n",
    );
}

#[test]
fn max_va_min_round_trip_in_va_so_sanh() {
    let src = r#"fn main():
    let mx = 9223372036854775807
    let mn = -9223372036854775808
    print(mx)
    print(mn)
    print(mx == 9223372036854775807)
    print(mn == -9223372036854775808)
    print(mn < mx)
    print(mn + 1)
    print(mn // 2)
    print(mn % 2)
"#;
    chay_sach(
        "roundtrip",
        src,
        "9223372036854775807\n-9223372036854775808\ntrue\ntrue\ntrue\n-9223372036854775807\n-4611686018427387904\n0\n",
    );
}

#[test]
fn ba_dau_tru_lien_tiep_quay_ve_min() {
    // `---M` = MIN; từng bước gộp dấu phải đúng ở biên 2^63, không tràn giả.
    // Cả dạng `let` lẫn đối số gọi (parser chỉ coi `--` là named arg khi sau nó là định danh).
    let src = r#"fn main():
    let a = ---9223372036854775808
    let b = ---0x8000000000000000
    print(---9223372036854775808)
    print(a)
    print(b)
"#;
    chay_sach(
        "triple-neg",
        src,
        "-9223372036854775808\n-9223372036854775808\n-9223372036854775808\n",
    );
}

#[test]
fn hai_dau_tru_tren_min_la_tran_duong_tinh() {
    // `--M` = +2^63 — độ lớn hợp lệ nhưng dương `i64` không biểu diễn được ⇒ VS0104 tĩnh.
    // (`--` trước một **định danh** vẫn là named arg; trước số là hai dấu âm một ngôi.)
    let bin = bin_2_63();
    for (tag, src) in [
        (
            "double-neg-dec",
            "fn main():\n    let x = --9223372036854775808\n    print(x)\n".to_string(),
        ),
        (
            "double-neg-hex",
            "fn main():\n    let x = --0x8000000000000000\n    print(x)\n".to_string(),
        ),
        (
            "double-neg-bin",
            format!("fn main():\n    let x = --{bin}\n    print(x)\n"),
        ),
        (
            "double-neg-call-dec",
            "fn main():\n    print(--9223372036854775808)\n".to_string(),
        ),
        (
            "double-neg-call-hex",
            "fn main():\n    print(--0x8000000000000000)\n".to_string(),
        ),
        (
            "double-neg-paren",
            "fn main():\n    print(-(-9223372036854775808))\n".to_string(),
        ),
    ] {
        tu_choi_tinh(tag, &src);
    }
}

#[test]
fn max_cong_1_khong_dau_bi_tu_choi_tinh() {
    let bin = bin_2_63();
    for (tag, src) in [
        (
            "pos-dec",
            "fn main():\n    print(9223372036854775808)\n".to_string(),
        ),
        (
            "pos-hex",
            "fn main():\n    print(0x8000000000000000)\n".to_string(),
        ),
        ("pos-bin", format!("fn main():\n    print({bin})\n")),
        (
            "pos-let",
            "fn main():\n    let x = 9223372036854775808\n    print(x)\n".to_string(),
        ),
        (
            "pos-underscore",
            "fn main():\n    print(9_223_372_036_854_775_808)\n".to_string(),
        ),
    ] {
        tu_choi_tinh(tag, &src);
    }
}

#[test]
fn lon_hon_max_cong_1_van_la_vs0104() {
    let bin = format!("0b1{}1", "0".repeat(62)); // 2^63 + 1
    for (tag, src) in [
        (
            "over-dec",
            "fn main():\n    print(-9223372036854775809)\n".to_string(),
        ),
        (
            "over-hex",
            "fn main():\n    print(0x8000000000000001)\n".to_string(),
        ),
        ("over-bin", format!("fn main():\n    print(-{bin})\n")),
        (
            "over-size",
            "fn main():\n    print(9223372036854775808kb)\n".to_string(),
        ),
        (
            "over-duration",
            "fn main():\n    print(9223372036854775808ms)\n".to_string(),
        ),
    ] {
        tu_choi_tinh(tag, &src);
    }
}

#[test]
fn min_chia_nguyen_va_du_voi_tru_1_trap_khong_panic() {
    for (tag, expr) in [("min-floor-div", "mn // -1"), ("min-rem", "mn % -1")] {
        let src = format!(
            "fn main():\n    let mn = -9223372036854775808\n    print(mn)\n    print({expr})\n"
        );
        let (d, p) = ghi(tag, &src);
        let a = chay_run(&p, false);
        let b = chay_run(&p, true);
        let _ = std::fs::remove_dir_all(&d);
        assert_eq!(a, b, "{tag}: interp↔VM lệch byte\ninterp: {a:?}\nvm: {b:?}");
        assert_eq!(
            a.0, 101,
            "{tag}: phải trap tràn i64 (exit 101), được {}\nstdout: {}\nstderr: {}",
            a.0, a.1, a.2
        );
        assert!(
            a.2.contains("VS0507"),
            "{tag}: thiếu VS0507 (tràn i64)\nstdout: {}\nstderr: {}",
            a.1,
            a.2
        );
        assert_eq!(
            a.1, "-9223372036854775808\n",
            "{tag}: phải in MIN trước khi trap"
        );
    }
}
