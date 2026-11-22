// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-267 / nợ hiệu năng #152** — tên enum/biến thể **nội bộ hoá** (`Name` = `Rc<str>`).
//!
//! Trước bản sửa, `Value::Enum(String, String, Vec<Value>)` cấp phát hai `String` cho **mỗi**
//! giá trị enum; vòng lặp nóng dựng `Some(i)`/`Ok(i)`/`E.A(i)` trả hai `malloc` + hai `free`
//! mỗi vòng. Sau bản sửa, tên lấy từ bảng tên thread-local (`Name`), chỉ tăng refcount.
//!
//! Hợp đồng **không đổi** (các ca dưới đây khoá lại):
//! 1. `show`/`str` và `==` của enum vẫn theo **nội dung** tên enum + biến thể;
//! 2. pattern khớp theo tên biến thể (kể cả biến thể rỗng `E.B`);
//! 3. `json.encode` của `Option`/`Result` giữ nguyên (`5`/`null`/`{"ok":5}`);
//! 4. interp và VM cho **byte** stdout/stderr/exit giống nhau trên cùng tệp.
//!
//! Mỗi ca chạy `vis run --no-cache` (interp) + `vis run --no-cache --vm`, so byte. Số vòng
//! `80_000` để tổng lệnh VM nằm dưới hàng rào fuel `10^7` (không nới fuel, không đổi ngữ nghĩa).

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

static DEM: AtomicU32 = AtomicU32::new(0);

fn tam(tag: &str) -> PathBuf {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-vd267-{tag}-{}-{n}", std::process::id()));
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

fn chay_tep(p: &Path, extra: &[&str]) -> (i32, String, String) {
    let mut c = Command::new(env!("CARGO_BIN_EXE_vis"));
    c.arg("run").arg("--no-cache");
    c.args(extra);
    let o = c.arg(p).output().expect("chạy vis run");
    (
        o.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&o.stdout).to_string(),
        String::from_utf8_lossy(&o.stderr).to_string(),
    )
}

/// Chạy interp + VM; assert hai engine giống **byte** stdout/stderr/exit, rồi trả kết quả.
fn chay_ca_hai(tag: &str, src: &str) -> (i32, String, String) {
    let (d, p) = ghi_tep(tag, src);
    let a = chay_tep(&p, &[]);
    let b = chay_tep(&p, &["--vm"]);
    let _ = std::fs::remove_dir_all(&d);
    assert_eq!(
        a.0, b.0,
        "{tag}: exit interp↔VM lệch\ninterp: {a:?}\nvm: {b:?}"
    );
    assert_eq!(
        a.1, b.1,
        "{tag}: stdout interp↔VM lệch\ninterp: {a:?}\nvm: {b:?}"
    );
    assert_eq!(
        a.2, b.2,
        "{tag}: stderr interp↔VM lệch\ninterp: {a:?}\nvm: {b:?}"
    );
    a
}

fn dung(tag: &str, src: &str, stdout: &str) {
    let (code, out, err) = chay_ca_hai(tag, src);
    assert_eq!(
        code, 0,
        "{tag}: phải chạy sạch\nstdout: {out}\nstderr: {err}"
    );
    assert!(err.is_empty(), "{tag}: stderr phải rỗng: {err}");
    assert_eq!(out, stdout, "{tag}: stdout sai");
}

/// Ba vòng lặp nóng: enum người dùng (có đối số + biến thể rỗng), `Option`, `Result`.
/// Tên enum/biến thể phải khớp đúng sau khi nội bộ hoá — sai tên ⇒ pattern rơi nhánh hoặc trap.
#[test]
fn vong_lap_enum_nang_giu_nguyen_ket_qua() {
    let src = "\
enum E:
    A(x: int)
    B

fn main():
    var i = 0
    var s = 0
    while i < 80000:
        let e = if i % 2 == 0: E.A(i) else: E.B
        s += match e:
            A(v): v
            B: 0
        i += 1
    print(s)

    var j = 0
    var t = 0
    while j < 80000:
        let o = if j % 2 == 0: Some(j) else: None
        t += match o:
            Some(v): v
            None: 0
        j += 1
    print(t)

    var k = 0
    var u = 0
    while k < 80000:
        let r: Result[int, str] = if k % 3 == 0: Err(\"x\") else: Ok(k)
        u += match r:
            Ok(v): v
            Err(_): 0
        k += 1
    print(u)
";
    // Tổng i chẵn 0..80000 = 1599960000; tổng k không chia hết 3 = 2133306667.
    dung("vong-lap", src, "1599960000\n1599960000\n2133306667\n");
}

/// `show`/`==`/clone/`json.encode` — các đường đọc tên enum không được đổi đầu ra.
#[test]
fn show_eq_clone_va_json_giu_nguyen() {
    let src = "\
use std.json

enum E:
    A(x: int)
    B

fn main():
    print(str(Some(5)))
    print(str(None))
    print(str(Ok(5)))
    print(str(Err(\"x\")))
    print(str(E.A(1)))
    print(str(E.B))
    print(E.A(1) == E.A(1))
    print(E.A(1) == E.B)
    let xs = [E.A(1), E.B, E.A(2)]
    print(xs)
    let m = {\"a\": E.A(3)}
    print(m[\"a\"])
    var ys = xs
    ys.push(E.A(9))
    print(xs)
    print(ys)
    print(json.encode(Some(5)))
    print(json.encode(None))
    print(json.encode(Ok(5)))
";
    dung(
        "show-eq",
        src,
        "Some(5)\nNone\nOk(5)\nErr(x)\nA(1)\nB\ntrue\nfalse\n[A(1), B, A(2)]\nA(3)\n[A(1), B, A(2)]\n[A(1), B, A(2), A(9)]\n5\nnull\n{\"ok\":5}\n",
    );
}
