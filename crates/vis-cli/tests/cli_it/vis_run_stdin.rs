// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270** — `vis run --stdin [-- args…]`: chạy **script mode** từ stdin dưới tệp ảo `-`,
//! cùng pipeline tệp (lex → parse → hạ → tyck → interp/VM), không đọc/ghi cache.
//!
//! Đỏ trước (main `0eabbd1`):
//! ```text
//! $ printf 'fn main():\n    print("xin chao")\n' | vis run --stdin
//! vis run: không hỗ trợ cờ `--stdin`
//! $ echo $?
//! 2
//! ```
//! Sau VD-270, `--stdin` là nguồn chương trình riêng: stdout/stderr/mã thoát **y như** chạy cùng
//! nguồn từ tệp (tên hiển thị `-`), trừ việc không có khoá cache ổn định nên luôn chạy như
//! `--no-cache`.
//!
//! Hợp đồng được kiểm:
//! - sạch ⇒ 0; `main -> Result::Err` ⇒ 1; lỗi dùng/kiểu ⇒ 2; trap lúc chạy ⇒ 101;
//! - `--vm` cho **cùng byte** stdout/stderr/mã thoát với interp;
//! - `-- args…` vào `main` như đường tệp; stdin rỗng là chương trình rỗng (0);
//! - `--tap` giữ TAP ở **stderr** (hành vi `vis run --tap` sẵn có) với tên `-`;
//! - stdin không phải UTF-8 ⇒ stderr + 2; `--stdin` + tệp / `--dump-bytecode` / `--dump-deps` /
//!   `--dry` ⇒ stderr + usage + 2, **không** đọc stdin hay tệp;
//! - `--strict-caps`/`--no-strict-caps` vẫn dùng được; `--stdin --vm` không ghi cache.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

type Kq = (i32, String, String);

/// Ghi stdin cho process con. `vis` có thể từ chối **trước** khi đọc stdin (vd `--stdin` + tệp)
/// ⇒ `BrokenPipe` là hợp lệ, không phải lỗi test.
fn bom_stdin(con: &mut std::process::Child, stdin: &[u8]) {
    let mut si = con.stdin.take().expect("stdin đã pipe");
    match si.write_all(stdin) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => {}
        Err(e) => panic!("ghi stdin: {e}"),
    }
}

/// Chạy `vis <args…>` với stdin **byte thô** (để thử cả UTF-8 không hợp lệ).
fn chay(args: &[&str], stdin: &[u8]) -> Kq {
    let mut con = Command::new(env!("CARGO_BIN_EXE_vis"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("phải chạy được binary `vis`");
    bom_stdin(&mut con, stdin);
    let o = con.wait_with_output().expect("chờ `vis`");
    (
        o.status.code().unwrap_or(-1),
        String::from_utf8(o.stdout).expect("stdout phải là UTF-8"),
        String::from_utf8_lossy(&o.stderr).into_owned(),
    )
}

fn chay_text(args: &[&str], stdin: &str) -> Kq {
    chay(args, stdin.as_bytes())
}

/// Như [`chay`] nhưng đặt thêm một biến môi trường (dùng cho `XDG_CACHE_HOME`).
fn chay_env(args: &[&str], stdin: &str, khoa: &str, gia_tri: &Path) -> Kq {
    let mut con = Command::new(env!("CARGO_BIN_EXE_vis"))
        .args(args)
        .env(khoa, gia_tri)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("phải chạy được binary `vis`");
    bom_stdin(&mut con, stdin.as_bytes());
    let o = con.wait_with_output().expect("chờ `vis`");
    (
        o.status.code().unwrap_or(-1),
        String::from_utf8(o.stdout).expect("stdout phải là UTF-8"),
        String::from_utf8_lossy(&o.stderr).into_owned(),
    )
}

/// Đếm tệp thường dưới `goc` (đệ quy) — dùng để chứng minh cache **không** bị ghi.
fn dem_tep(goc: &Path) -> usize {
    let Ok(ds) = std::fs::read_dir(goc) else {
        return 0;
    };
    let mut n = 0;
    for e in ds.flatten() {
        let p = e.path();
        if p.is_dir() {
            n += dem_tep(&p);
        } else {
            n += 1;
        }
    }
    n
}

fn tam(tag: &str) -> PathBuf {
    static DEM: AtomicUsize = AtomicUsize::new(0);
    let d = std::env::temp_dir().join(format!(
        "vis-run-stdin-{tag}-{}-{}",
        std::process::id(),
        DEM.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("thư mục tạm");
    d
}

/// (1) Chương trình sạch in stdout; stderr rỗng; exit 0.
#[test]
fn sach_in_stdout_va_exit_0() {
    let src = "fn main():\n    print(\"xin chao\")\n";
    for extra in [&["--stdin"][..], &["--stdin", "--vm"][..]] {
        let (ma, out, err) = chay_text(&[&["run"][..], extra].concat(), src);
        assert_eq!(ma, 0, "sạch ⇒ 0; args={extra:?}; stderr: {err}");
        assert_eq!(out, "xin chao\n", "args={extra:?}");
        assert_eq!(err, "", "args={extra:?}");
    }
}

/// (2) `-- args…` vào `main(args: list[str])` đúng thứ tự như đường tệp.
#[test]
fn args_sau_dau_gach_duoc_truyen_vao_main() {
    let src = "fn main(args: list[str]):\n    print(args)\n";
    let (ma, out, err) = chay_text(&["run", "--stdin", "--", "a", "b"], src);
    assert_eq!(ma, 0, "stderr: {err}");
    assert_eq!(out, "[a, b]\n");
    assert_eq!(err, "");
}

/// (3) `--vm` **byte-khớp** interp: exit + stdout + stderr, dùng cùng nguồn stdin.
#[test]
fn vm_byte_khop_interp() {
    let src = "fn main():\n    print(\"một\")\n    print(1 + 2)\n";
    let a = chay_text(&["run", "--stdin"], src);
    let b = chay_text(&["run", "--stdin", "--vm"], src);
    assert_eq!(a.0, b.0, "exit lệch\n interp {a:?}\n vm {b:?}");
    assert_eq!(a.1, b.1, "stdout lệch\n interp {a:?}\n vm {b:?}");
    assert_eq!(a.2, b.2, "stderr lệch\n interp {a:?}\n vm {b:?}");
    assert_eq!(a, (0, "một\n3\n".to_string(), String::new()));
}

/// (4) Lỗi kiểu tĩnh (`VS0412`) ⇒ exit 2, stdout rỗng, chẩn đoán trỏ tệp ảo `-`.
#[test]
fn loi_kieu_exit_2() {
    let (ma, out, err) = chay_text(&["run", "--stdin"], "let x: int = \"s\"\n");
    assert_eq!(ma, 2, "lỗi kiểu ⇒ 2; stderr: {err}");
    assert_eq!(out, "");
    assert!(
        err.contains("-:1:14: error VS0412:"),
        "chẩn đoán phải trỏ `-:1:14` + VS0412; stderr: {err}"
    );

    // `--tap` vẫn là TAP (thêm dòng `not ok`), exit 2 như đường tệp.
    let (ma, out, err) = chay_text(&["run", "--stdin", "--tap"], "let x: int = \"s\"\n");
    assert_eq!(ma, 2, "stderr: {err}");
    assert_eq!(out, "");
    assert!(
        err.starts_with("TAP version 13\n1..1\n"),
        "TAP ở stderr; stderr: {err}"
    );
    assert!(
        err.contains("not ok 1 - - (VS0412)"),
        "tên test phải là `-`; stderr: {err}"
    );
}

/// (5) Trap lúc chạy ⇒ exit 101; dòng trước đã in, dòng sau không chạy; không Rust panic.
#[test]
fn panic_exit_101() {
    let src = "fn main():\n    print(\"truoc\")\n    panic(\"hỏng\")\n    print(\"sau\")\n";
    for extra in [&[][..], &["--vm"][..]] {
        let (ma, out, err) = chay_text(&[&["run", "--stdin"][..], extra].concat(), src);
        assert_eq!(ma, 101, "trap ⇒ 101; extra={extra:?}; stderr: {err}");
        assert_eq!(out, "truoc\n", "extra={extra:?}");
        assert!(
            err.contains("VS0501") && err.contains("hỏng"),
            "stderr thiếu mã/thông điệp; extra={extra:?}; stderr: {err}"
        );
        assert!(
            !err.contains("panicked at"),
            "không được có Rust panic; extra={extra:?}; stderr: {err}"
        );
    }
}

/// (6) `--tap`: program stdout giữ nguyên trên **stdout**, TAP trên **stderr** với tên `-` và
/// `steps` **bằng** đường tệp cùng nguồn (byte-khớp sau khi thay tên tệp).
#[test]
fn tap_ten_gach_ngang_va_steps_bang_duong_tep() {
    let d = tam("tap");
    let f = d.join("t.vis");
    let src = "fn main():\n    print(\"tap\")\n";
    std::fs::write(&f, src).expect("ghi tệp");

    let tep = Command::new(env!("CARGO_BIN_EXE_vis"))
        .args(["run", "--tap"])
        .arg(&f)
        .output()
        .expect("vis run --tap <tệp>");
    let tep = (
        tep.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&tep.stdout).to_string(),
        String::from_utf8_lossy(&tep.stderr).to_string(),
    );
    let stdin = chay_text(&["run", "--stdin", "--tap"], src);

    assert_eq!(stdin.0, 0, "stderr: {}", stdin.2);
    assert_eq!(stdin.1, "tap\n", "TAP không được lẫn vào stdout");
    assert_eq!(
        tep.0, stdin.0,
        "exit TAP lệch\ntệp {tep:?}\nstdin {stdin:?}"
    );
    assert_eq!(
        tep.1, stdin.1,
        "stdout TAP lệch\ntệp {tep:?}\nstdin {stdin:?}"
    );
    let ten_tep = f.display().to_string();
    assert_eq!(
        stdin.2,
        tep.2.replace(&ten_tep, "-"),
        "stderr TAP phải giống đường tệp, chỉ đổi tên thành `-`"
    );
    assert!(
        stdin.2.contains("ok 1 - - (steps="),
        "TAP phải là `ok 1 - -`; stderr: {}",
        stdin.2
    );

    let _ = std::fs::remove_dir_all(&d);
}

/// (7) Stdin **rỗng** = chương trình rỗng hợp lệ: không stdout/stderr, exit 0.
#[test]
fn stdin_rong_la_chuong_trinh_rong() {
    let (ma, out, err) = chay(&["run", "--stdin"], b"");
    assert_eq!(ma, 0, "stderr: {err}");
    assert_eq!(out, "");
    assert_eq!(err, "");
}

/// (8) Stdin không phải UTF-8 ⇒ lỗi đầu vào: stderr + exit 2, **không** in gì ra stdout/TAP.
#[test]
fn utf8_khong_hop_le_exit_2() {
    for args in [
        &["run", "--stdin"][..],
        &["run", "--stdin", "--vm"][..],
        &["run", "--stdin", "--tap"][..],
    ] {
        let (ma, out, err) = chay(args, &[0xff, 0xfe, 0xfd]);
        assert_eq!(ma, 2, "stdin non-UTF8 ⇒ 2; args={args:?}");
        assert_eq!(out, "", "args={args:?}");
        assert_eq!(err, "vis run: stdin không phải UTF-8 hợp lệ\n");
    }
}

/// (9) `--stdin` không đi với tệp (cả hai thứ tự) — exit 2, usage, và **không** đọc tệp.
#[test]
fn stdin_khong_di_voi_tep() {
    for args in [
        &["run", "--stdin", "a.vis"][..],
        &["run", "a.vis", "--stdin"][..],
    ] {
        let (ma, out, err) = chay_text(args, "fn main():\n    print(\"khong chay\")\n");
        assert_eq!(ma, 2, "args={args:?}; stderr: {err}");
        assert_eq!(out, "", "args={args:?}");
        assert!(
            err.starts_with("vis run: `--stdin` không đi với tệp `a.vis`"),
            "args={args:?}; stderr: {err}"
        );
        assert!(
            err.contains("dùng: vis run"),
            "phải in usage; args={args:?}; stderr: {err}"
        );
    }
}

/// (9b) Ba cờ cần đường dẫn tệp bị từ chối **trước** khi đọc stdin: exit 2 + usage.
#[test]
fn co_can_tep_khong_di_voi_stdin() {
    for co in ["--dump-bytecode", "--dump-deps", "--dry"] {
        let (ma, out, err) = chay_text(&["run", "--stdin", co], "fn main():\n    print(1)\n");
        assert_eq!(ma, 2, "cờ `{co}` ⇒ 2; stderr: {err}");
        assert_eq!(out, "", "cờ `{co}`");
        assert!(
            err.starts_with(&format!("vis run: không hỗ trợ `{co}` khi dùng --stdin\n")),
            "cờ `{co}`: {err}"
        );
        assert!(err.contains("dùng: vis run"), "cờ `{co}`: {err}");
    }
}

/// (10) `main -> Result::Err` ⇒ exit 1 (không in lỗi), đúng quyết định VD-241 §3/T5.
#[test]
fn main_result_err_exit_1() {
    let src = "fn main() -> Result[int, str]:\n    Err(\"that bai\")\n";
    for extra in [&[][..], &["--vm"][..]] {
        let (ma, out, err) = chay_text(&[&["run", "--stdin"][..], extra].concat(), src);
        assert_eq!(ma, 1, "Result::Err ⇒ 1; extra={extra:?}; stderr: {err}");
        assert_eq!(out, "", "extra={extra:?}");
        assert_eq!(err, "", "extra={extra:?}");
    }
}

/// (11) Bằng chứng **đỏ trước**: cờ lạ vẫn bị từ chối y như `--stdin` từng bị ở HEAD; từ sau
/// VD-270 `--stdin` **không** còn đi vào nhánh này.
#[test]
fn co_la_van_bi_tu_choi_nhu_head() {
    let (ma, out, err) = chay_text(&["run", "--bogus"], "print(1)\n");
    assert_eq!(ma, 2);
    assert_eq!(out, "");
    assert_eq!(err, "vis run: không hỗ trợ cờ `--bogus`\n");

    // `--stdin` trước đây rơi đúng vào thông điệp trên; nay phải chạy được.
    let (ma, out, err) = chay_text(&["run", "--stdin"], "print(1)\n");
    assert_eq!(ma, 0, "stderr: {err}");
    assert_eq!(out, "1\n");
    assert!(!err.contains("không hỗ trợ cờ"), "stderr: {err}");
}

/// Không có `--stdin` và cũng không có tệp ⇒ usage + 2 (giữ nguyên hành vi đường tệp).
#[test]
fn thieu_nguon_van_la_usage_exit_2() {
    let (ma, out, err) = chay_text(&["run"], "");
    assert_eq!(ma, 2);
    assert_eq!(out, "");
    assert!(err.starts_with("dùng: vis run"), "{err}");
}

/// `--strict-caps`/`--no-strict-caps` vẫn có nghĩa với stdin: chấp nhận từng cờ; cả hai ⇒ 2.
#[test]
fn co_siet_quyen_van_dung_duoc_voi_stdin() {
    let src = "fn main():\n    print(\"ok\")\n";
    for co in ["--strict-caps", "--no-strict-caps"] {
        let (ma, out, err) = chay_text(&["run", "--stdin", co], src);
        assert_eq!(ma, 0, "`{co}` phải chạy được; stderr: {err}");
        assert_eq!(out, "ok\n", "`{co}`");
        assert_eq!(err, "", "`{co}`");
    }
    let (ma, out, err) = chay_text(
        &["run", "--stdin", "--strict-caps", "--no-strict-caps"],
        src,
    );
    assert_eq!(ma, 2, "hai cờ mâu thuẫn ⇒ 2; stderr: {err}");
    assert_eq!(out, "");
    assert!(
        err.contains("`--strict-caps` và `--no-strict-caps` mâu thuẫn nhau"),
        "{err}"
    );
}

/// `--stdin --vm` **không** đọc/ghi cache: chứng minh `XDG_CACHE_HOME` có hiệu lực bằng đường
/// tệp (cache được ghi), rồi xoá và chạy stdin hai lần — thư mục cache phải rỗng.
#[test]
fn stdin_vm_khong_doc_ghi_cache() {
    let d = tam("nocache");
    let cache = d.join("cache");
    std::fs::create_dir_all(&cache).expect("thư mục cache");
    let src = "fn main():\n    print(\"cache\")\n";

    // Đường tệp `--vm` (không `--no-cache`) phải ghi cache — nếu không, phép thử vô nghĩa.
    let f = d.join("c.vis");
    std::fs::write(&f, src).expect("ghi tệp");
    let tep = Command::new(env!("CARGO_BIN_EXE_vis"))
        .env("XDG_CACHE_HOME", &cache)
        .args(["run", "--vm"])
        .arg(&f)
        .output()
        .expect("vis run --vm <tệp>");
    assert_eq!(tep.status.code(), Some(0), "stderr: {tep:?}");
    assert!(
        dem_tep(&cache) > 0,
        "đường tệp --vm phải ghi cache để phép thử có hiệu lực"
    );

    std::fs::remove_dir_all(&cache).expect("xoá cache");
    std::fs::create_dir_all(&cache).expect("cache rỗng");
    for _ in 0..2 {
        let (ma, out, err) = chay_env(&["run", "--stdin", "--vm"], src, "XDG_CACHE_HOME", &cache);
        assert_eq!(ma, 0, "stderr: {err}");
        assert_eq!(out, "cache\n");
        assert_eq!(err, "");
    }
    assert_eq!(
        dem_tep(&cache),
        0,
        "`--stdin --vm` không được ghi tệp cache"
    );

    let _ = std::fs::remove_dir_all(&d);
}
