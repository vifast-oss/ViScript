// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (R5c arity)** — sugar phương thức `x.f(…)` phải kiểm **số đối số** ở checker, không
//! để rơi xuống tầng chạy thành trap.
//!
//! Hợp đồng khoá ở đây:
//! - Sugar R5c phân giải được về hàm `std.*` (receiver khớp tham số đầu) mà **sai số đối số**
//!   (không tính receiver) ⇒ `VS0401` tại **span lời gọi**, đúng thông điệp module call:
//!   `` `str.trim_matches` cần 1 đối số, được 0 ``.
//! - `vis check` đỏ **exit 1**; `vis run` và `vis run --vm` từ chối tĩnh **exit 2** (mọi lỗi
//!   checker — cùng đường module call), **không** còn `VS0501`/exit 101.
//! - Valid sugar giữ nguyên hành vi (interp ↔ VM byte-identical); module call không đổi.
//! - Không mã chẩn đoán mới; ca sai **kiểu** (`VS0412`) / **không có hàm** (`VS0403`) giữ nguyên.
//!
//! **RED-BEFORE** (HEAD `c75c0da`, trước khi vá `kiem_std_method` — `vis` của cây này):
//!
//! ```text
//! $ vis check s.vis                 # fn main(): print("a".trim_matches())
//! TAP version 13
//! 1..1
//! ok 1 - s.vis
//! exit=0
//! $ vis run --no-cache s.vis
//! s.vis:3:11: error VS0501: `str.trim_matches` cần 2 đối số, được 1
//! exit=101
//! $ vis run --no-cache --vm s.vis   ⇒ exit 101 (cùng VS0501)
//! ```
//!
//! Module call cùng ca (`str.trim_matches("a")` + `use std.str`) **đã** là `VS0401`/exit 1 —
//! bản vá chỉ đưa sugar về cùng đường.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

static DEM: AtomicU32 = AtomicU32::new(0);

fn tam(tag: &str) -> PathBuf {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-vd270-arity-{tag}-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("tạo thư mục tạm");
    d
}

fn ghi_tep(tag: &str, src: &str) -> (PathBuf, PathBuf) {
    let d = tam(tag);
    let p = d.join(format!("{tag}.vis"));
    std::fs::write(
        &p,
        format!("# SPDX-License-Identifier: MIT OR Apache-2.0\n{src}"),
    )
    .expect("ghi tệp tạm");
    (d, p)
}

/// `vis run --no-cache [--vm]` ⇒ `(exit, stdout, stderr)`.
fn chay_tep(p: &Path, extra: &[&str]) -> (i32, String, String) {
    let mut c = Command::new(env!("CARGO_BIN_EXE_vis"));
    c.arg("run").arg("--no-cache").args(extra).arg(p);
    let o = c.output().expect("chạy vis run");
    (
        o.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&o.stdout).to_string(),
        String::from_utf8_lossy(&o.stderr).to_string(),
    )
}

/// `vis check` ⇒ `(exit, stdout, stderr)`; TAP ghi ra stdout.
fn check_tep(p: &Path) -> (i32, String, String) {
    let o = Command::new(env!("CARGO_BIN_EXE_vis"))
        .arg("check")
        .arg(p)
        .output()
        .expect("chạy vis check");
    (
        o.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&o.stdout).to_string(),
        String::from_utf8_lossy(&o.stderr).to_string(),
    )
}

/// Ca **tĩnh**: `vis check` exit 1 + `VS0401` + **đúng** thông điệp; `vis run`/`--vm` exit 2 +
/// cùng thông điệp, **không** `VS0501`, không exit 101.
fn sai_arity(tag: &str, src: &str, mong: &str) {
    let (d, p) = ghi_tep(tag, src);
    let (code, out, err) = check_tep(&p);
    assert_eq!(
        code, 1,
        "{tag}: `check` phải đỏ (exit 1), được {code}\nstdout: {out}\nstderr: {err}"
    );
    assert!(
        out.contains("VS0401"),
        "{tag}: `check` thiếu VS0401\nstdout: {out}\nstderr: {err}"
    );
    assert!(
        out.contains(mong),
        "{tag}: `check` thiếu thông điệp `{mong}`\nstdout: {out}\nstderr: {err}"
    );
    for (i, extra) in [&[][..], &["--vm"][..]].iter().enumerate() {
        let (code, so, se) = chay_tep(&p, extra);
        assert_eq!(
            code, 2,
            "{tag} engine={i}: lỗi checker trong `run` phải exit 2 (không trap), được {code}\nstdout: {so}\nstderr: {se}"
        );
        assert!(
            se.contains("VS0401") && se.contains(mong),
            "{tag} engine={i}: thiếu VS0401/`{mong}`\nstderr: {se}"
        );
        assert!(
            !se.contains("VS0501"),
            "{tag} engine={i}: không được còn trap VS0501\nstderr: {se}"
        );
    }
    let _ = std::fs::remove_dir_all(&d);
}

/// Chương trình hợp lệ: hai engine cùng exit 0, stdout đúng, stderr rỗng.
fn dung(tag: &str, src: &str, stdout: &str) {
    let (d, p) = ghi_tep(tag, src);
    let a = chay_tep(&p, &[]);
    let b = chay_tep(&p, &["--vm"]);
    let _ = std::fs::remove_dir_all(&d);
    assert_eq!(
        a.0, 0,
        "{tag}: interp phải chạy sạch\nstdout: {}\nstderr: {}",
        a.1, a.2
    );
    assert_eq!(
        b.0, 0,
        "{tag}: VM phải chạy sạch\nstdout: {}\nstderr: {}",
        b.1, b.2
    );
    assert_eq!(
        (a.0, a.1.as_str(), a.2.as_str()),
        (b.0, b.1.as_str(), b.2.as_str()),
        "{tag}: interp ↔ VM lệch"
    );
    assert_eq!(a.1, stdout, "{tag}: stdout sai");
    assert!(a.2.is_empty(), "{tag}: stderr phải rỗng: {}", a.2);
}

// ===========================================================================
// 1 — str: trim_matches thiếu/thừa
// ===========================================================================

/// Sugar `s.trim_matches(…)` ⇔ `str.trim_matches(s, …)`; thiếu/thừa 1 đối số ⇒ `VS0401` nêu
/// đúng tên `str.trim_matches` và số lượng thật, tại span lời gọi.
#[test]
fn str_trim_matches_sai_arity_bi_chan() {
    sai_arity(
        "str-thieu",
        "fn main():\n    print(\"a\".trim_matches())\n",
        "`str.trim_matches` cần 1 đối số, được 0",
    );
    sai_arity(
        "str-thua",
        "fn main():\n    print(\"a\".trim_matches(\"x\", \"y\"))\n",
        "`str.trim_matches` cần 1 đối số, được 2",
    );
}

// ===========================================================================
// 2 — list: binary_search thiếu/thừa, flatten thừa
// ===========================================================================

/// Sugar `xs.binary_search(v)` ⇔ `list.binary_search(xs, v)` — thiếu/thừa đều `VS0401`.
#[test]
fn list_binary_search_sai_arity_bi_chan() {
    sai_arity(
        "bs-thieu",
        "fn main():\n    let xs = [1, 2]\n    print(xs.binary_search())\n",
        "`list.binary_search` cần 1 đối số, được 0",
    );
    sai_arity(
        "bs-thua",
        "fn main():\n    let xs = [1, 2]\n    print(xs.binary_search(1, 2))\n",
        "`list.binary_search` cần 1 đối số, được 2",
    );
}

/// Sugar `xs.flatten()` ⇔ `list.flatten(xs)` — arity 0; thừa đối số ⇒ `VS0401` (receiver
/// `list[list[int]]` khớp nên **không** rơi về `VS0403`).
#[test]
fn list_flatten_sai_arity_bi_chan() {
    sai_arity(
        "fl-thua",
        "fn main():\n    let xs: list[list[int]] = [[1], [2]]\n    print(xs.flatten(1))\n",
        "`list.flatten` cần 0 đối số, được 1",
    );
}

// ===========================================================================
// 3 — path: normalize thừa (module đích khác module lõi của receiver)
// ===========================================================================

/// `s.normalize()` là sugar của `path.normalize(s)` (không phải `str.normalize`); thừa đối số
/// ⇒ `VS0401` nêu `path.normalize`, không trap ở tầng chạy.
#[test]
fn path_normalize_sai_arity_bi_chan() {
    sai_arity(
        "pn-thua-1",
        "fn main():\n    print(\"a/./b\".normalize(1))\n",
        "`path.normalize` cần 0 đối số, được 1",
    );
    sai_arity(
        "pn-thua-2",
        "fn main():\n    print(\"a/./b\".normalize(\"a\"))\n",
        "`path.normalize` cần 0 đối số, được 1",
    );
}

// ===========================================================================
// 4 — valid sugar giữ nguyên hành vi, hai engine byte-identical
// ===========================================================================

/// Đúng arity vẫn chạy như cũ (không cần `use std.*`): interp ↔ VM byte-identical.
#[test]
fn sugar_hop_le_van_chay() {
    dung(
        "hop-le",
        r#"fn main():
    print("--x--".trim_matches("-"))
    let xs = [1, 2, 3]
    print(xs.binary_search(2))
    print(xs.binary_search(9))
    let zs: list[list[int]] = [[1], [2, 3], []]
    print(zs.flatten())
    print("a//./b/../c".normalize())
"#,
        "x\nSome(1)\nNone\n[1, 2, 3]\na/c\n",
    );
}

// ===========================================================================
// 5 — module call không đổi: vẫn `VS0401` (không phải ca sugar)
// ===========================================================================

/// Gọi module trực tiếp (`str.trim_matches(s)`) vẫn `VS0401` ở checker với thông điệp cũ
/// (tính cả receiver trong chữ ký), `vis check` exit 1 / `run` exit 2 — hành vi không đổi.
#[test]
fn module_call_van_vs0401() {
    let (d, p) = ghi_tep(
        "module",
        "use std.str\n\nfn main():\n    print(str.trim_matches(\"a\"))\n",
    );
    let (code, out, err) = check_tep(&p);
    assert_eq!(
        code, 1,
        "module: `check` phải exit 1\nstdout: {out}\nstderr: {err}"
    );
    assert!(
        out.contains("`str.trim_matches` cần 2 đối số, được 1"),
        "module: thông điệp đổi\nstdout: {out}\nstderr: {err}"
    );
    for (i, extra) in [&[][..], &["--vm"][..]].iter().enumerate() {
        let (code, so, se) = chay_tep(&p, extra);
        assert_eq!(
            code, 2,
            "module engine={i}: `run` phải exit 2, được {code}\nstdout: {so}\nstderr: {se}"
        );
        assert!(
            se.contains("VS0401") && !se.contains("VS0501"),
            "module engine={i}: phải VS0401, không trap\nstderr: {se}"
        );
    }
    let _ = std::fs::remove_dir_all(&d);
}

// ===========================================================================
// 6 — không nới lỏng ca khác: sai kiểu / receiver không khớp giữ nguyên
// ===========================================================================

/// Receiver **không khớp** tham số đầu ⇒ vẫn `VS0403` (`list[int].flatten()`), không bị đổi
/// thành `VS0401`; sai **kiểu** đối số ⇒ vẫn `VS0412`.
#[test]
fn ca_khac_khong_doi() {
    let (d, p) = ghi_tep("khong-doi", "fn main():\n    print([1, 2].flatten())\n");
    let (code, out, err) = check_tep(&p);
    assert_eq!(code, 1, "flatten-receiver: check exit 1\n{out}\n{err}");
    assert!(
        out.contains("VS0403"),
        "flatten-receiver: phải VS0403 (không có trường), được:\n{out}\n{err}"
    );
    assert!(
        !out.contains("VS0401"),
        "flatten-receiver: không được đổi thành VS0401:\n{out}"
    );
    let _ = std::fs::remove_dir_all(&d);

    let (d, p) = ghi_tep(
        "khong-doi-kieu",
        "fn main():\n    let xs = [1, 2]\n    print(xs.binary_search(\"x\"))\n",
    );
    let (code, out, err) = check_tep(&p);
    assert_eq!(code, 1, "bs-kieu: check exit 1\n{out}\n{err}");
    assert!(
        out.contains("VS0412"),
        "bs-kieu: phải VS0412, được:\n{out}\n{err}"
    );
    assert!(
        !out.contains("VS0401"),
        "bs-kieu: không được đổi thành VS0401:\n{out}"
    );
    let _ = std::fs::remove_dir_all(&d);
}
