// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (mkdir-all)** — `std.fs.mkdir_all(path) -> Result[(), Error]`.
//!
//! Khoá ngữ nghĩa: tạo **đệ quy** mọi thư mục còn thiếu (như `std::fs::create_dir_all`);
//! thư mục đã tồn tại ⇒ `Ok(())` (**idempotent**); một thành phần đã tồn tại là **tệp** hoặc
//! **symlink** (kể cả đích cuối) ⇒ `Err(Error)` **dữ liệu**, không trap. Cùng mô hình quyền với
//! `fs.mkdir`: cần `cap[Dir] (write)`, ra ngoài root ⇒ trap `VS0604` (exit 2). Cùng một host
//! (`--fs-mock`) và **interp ↔ VM byte-identical**.
//!
//! **RED-BEFORE** (trước khi thêm hàm, cây @ `146cf32`, lệnh
//! `/home/ps/.cargo/bin/cargo test -p vis-cli --test vd270_mkdir_all`):
//! **0/7 xanh** — cả 7 ca đỏ vì checker báo `VS0301` «không có hàm `mkdir_all`»; ca
//! `sai_kieu_va_arity` cũng đỏ vì thông điệp vẫn là «không có hàm» thay vì `VS0412`/`VS0401`.
//! Sau khi thêm hàm: 7/7 xanh (xem CHANGELOG/báo cáo).

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

static DEM: AtomicU32 = AtomicU32::new(0);

/// Thư mục tạm riêng cho từng ca (test chạy song song).
fn tam(tag: &str) -> PathBuf {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!(
        "vis-vd270-mkdirall-{tag}-{}-{n}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("tạo thư mục tạm");
    d
}

fn duong(p: &Path) -> String {
    p.display().to_string().replace('\\', "/")
}

fn vis(args: &[&std::ffi::OsStr]) -> (i32, String, String) {
    let mut c = Command::new(env!("CARGO_BIN_EXE_vis"));
    for a in args {
        c.arg(a);
    }
    let o = c.output().expect("chạy `vis`");
    (
        o.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&o.stdout).to_string(),
        String::from_utf8_lossy(&o.stderr).to_string(),
    )
}

fn viet_tep(p: &Path, src: &str) {
    std::fs::write(
        p,
        format!("# SPDX-License-Identifier: MIT OR Apache-2.0\n{src}"),
    )
    .expect("ghi chương trình");
}

/// Hai đường dùng **hai thư mục đích khác nhau** — cho thao tác tạo thư
/// mục không chạy lặp được trên cùng đường dẫn (idempotent nhưng trạng thái đĩa khác nhau giữa
/// hai lần chạy). Hai nguồn chỉ khác đường dẫn; stdout phải **byte-identical**.
fn hai_duong_khac(d: &Path, tag: &str, src_interp: &str, src_vm: &str) -> String {
    let pi = d.join(format!("{tag}-i.vis"));
    let pv = d.join(format!("{tag}-v.vis"));
    viet_tep(&pi, src_interp);
    viet_tep(&pv, src_vm);
    let (c1, o1, e1) = vis(&[std::ffi::OsStr::new("run"), pi.as_os_str()]);
    let (c2, o2, e2) = vis(&[
        std::ffi::OsStr::new("run"),
        std::ffi::OsStr::new("--vm"),
        std::ffi::OsStr::new("--no-cache"),
        pv.as_os_str(),
    ]);
    assert_eq!(c1, 0, "[{tag}] interp exit {c1}: {e1}");
    assert_eq!(c2, 0, "[{tag}] vm exit {c2}: {e2}");
    assert_eq!(o1, o2, "[{tag}] hai đường phải khớp: vm [{e2}]");
    o1
}

/// Như [`hai_duong`] nhưng cả hai đường chạy với `--fs-mock` (mỗi tiến trình nạp kịch bản mới ⇒
/// thao tác ghi trong bộ nhớ không rò sang lần chạy sau).
fn hai_duong_mock(d: &Path, tag: &str, mock: &str, src: &str) -> String {
    let m = d.join(format!("{tag}.json"));
    std::fs::write(&m, mock).expect("ghi kịch bản mock");
    let p = d.join(format!("{tag}.vis"));
    viet_tep(&p, src);
    let (c1, o1, e1) = vis(&[
        std::ffi::OsStr::new("run"),
        std::ffi::OsStr::new("--fs-mock"),
        m.as_os_str(),
        p.as_os_str(),
    ]);
    let (c2, o2, e2) = vis(&[
        std::ffi::OsStr::new("run"),
        std::ffi::OsStr::new("--vm"),
        std::ffi::OsStr::new("--no-cache"),
        std::ffi::OsStr::new("--fs-mock"),
        m.as_os_str(),
        p.as_os_str(),
    ]);
    assert_eq!(c1, 0, "[{tag}] interp exit {c1}: {e1}");
    assert_eq!(c2, 0, "[{tag}] vm exit {c2}: {e2}");
    assert_eq!(o1, o2, "[{tag}] hai đường phải khớp: vm [{e2}]");
    o1
}

fn chay(dir: &Path, args: &[&str]) -> (i32, String, String) {
    let o = Command::new(env!("CARGO_BIN_EXE_vis"))
        .current_dir(dir)
        .args(args)
        .output()
        .expect("chạy `vis`");
    (
        o.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&o.stdout).to_string(),
        String::from_utf8_lossy(&o.stderr).to_string(),
    )
}

// ---------- tạo đệ quy + idempotent ----------

/// `a/b/c` thiếu **cả ba** cấp ⇒ tạo hết; gọi lần hai (đích đã là thư mục) ⇒ vẫn `Ok`.
#[test]
fn tao_de_quy_va_idempotent() {
    let d = tam("de-quy");
    let nd = |tag: &str| d.join(format!("goc-{tag}")).join("a").join("b").join("c");
    let ct = |p: &Path| {
        let goc = p.parent().unwrap().parent().unwrap().parent().unwrap();
        format!(
            "use std.fs\nfn main() -> ():\n    match fs.mkdir_all(\"{}\"):\n        Ok(_):\n            print(\"ok\")\n        Err(e):\n            print(\"err\")\n    print(fs.exists(\"{}\"))\n    print(fs.exists(\"{}\"))\n    print(fs.exists(\"{}\"))\n    match fs.mkdir_all(\"{}\"):\n        Ok(_):\n            print(\"ok2\")\n        Err(e):\n            print(\"err2\")\n",
            duong(p),
            duong(&goc.join("a")),
            duong(&goc.join("a").join("b")),
            duong(p),
            duong(p)
        )
    };
    let o = hai_duong_khac(&d, "de-quy", &ct(&nd("i")), &ct(&nd("v")));
    assert_eq!(o, "ok\ntrue\ntrue\ntrue\nok2\n");
    assert!(nd("i").is_dir(), "interp phải tạo đủ a/b/c");
    assert!(nd("v").is_dir(), "vm phải tạo đủ a/b/c");
    let _ = std::fs::remove_dir_all(&d);
}

/// Đích **đã là thư mục** từ trước (không do hàm tạo trong lần chạy này) ⇒ `Ok`, không lỗi.
#[test]
fn thu_muc_da_ton_tai_la_ok() {
    let d = tam("da-co");
    let pi = d.join("i").join("san-co");
    let pv = d.join("v").join("san-co");
    std::fs::create_dir_all(&pi).expect("thư mục sẵn i");
    std::fs::create_dir_all(&pv).expect("thư mục sẵn v");
    let ct = |p: &Path| {
        format!(
            "use std.fs\nfn main() -> ():\n    match fs.mkdir_all(\"{}\"):\n        Ok(_):\n            print(\"ok\")\n        Err(e):\n            print(\"err\")\n",
            duong(p)
        )
    };
    let o = hai_duong_khac(&d, "da-co", &ct(&pi), &ct(&pv));
    assert_eq!(o, "ok\n");
    let _ = std::fs::remove_dir_all(&d);
}

// ---------- tệp / symlink trên đường dẫn ⇒ Err dữ liệu ----------

/// Một thành phần giữa đường là **tệp**, hoặc **đích cuối** là tệp ⇒ `Err(Error)`, exit 0
/// (không trap), và không tạo thư mục con nào.
#[test]
fn tep_tren_duong_la_err() {
    let d = tam("tep");
    let i = d.join("i");
    let v = d.join("v");
    std::fs::create_dir_all(&i).expect("i");
    std::fs::create_dir_all(&v).expect("v");
    std::fs::write(i.join("chan"), "x").expect("tệp chặn i");
    std::fs::write(v.join("chan"), "x").expect("tệp chặn v");
    let ct = |goc: &Path| {
        format!(
            "use std.fs\nfn main() -> ():\n    match fs.mkdir_all(\"{}/chan/a/b\"):\n        Ok(_):\n            print(\"ok1\")\n        Err(e):\n            print(\"err1\")\n    match fs.mkdir_all(\"{}/chan\"):\n        Ok(_):\n            print(\"ok2\")\n        Err(e):\n            print(\"err2\")\n    print(fs.exists(\"{}/chan/a\"))\n",
            duong(goc),
            duong(goc),
            duong(goc)
        )
    };
    let o = hai_duong_khac(&d, "tep", &ct(&i), &ct(&v));
    assert_eq!(o, "err1\nerr2\nfalse\n");
    assert!(
        !i.join("chan").join("a").exists(),
        "không được tạo dưới tệp"
    );
    assert!(
        !v.join("chan").join("a").exists(),
        "không được tạo dưới tệp"
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// **Unix**: symlink ở **thành phần giữa** hoặc ở **đích cuối** ⇒ `Err` (không đi theo link),
/// không tạo thư mục trong đích của link.
#[cfg(unix)]
#[test]
fn symlink_tren_duong_la_err() {
    use std::os::unix::fs::symlink;
    let d = tam("symlink");
    let i = d.join("i");
    let v = d.join("v");
    for goc in [&i, &v] {
        std::fs::create_dir_all(goc.join("that")).expect("thư mục thật");
        symlink(goc.join("that"), goc.join("link")).expect("symlink");
    }
    let ct = |goc: &Path| {
        format!(
            "use std.fs\nfn main() -> ():\n    match fs.mkdir_all(\"{}/link/con\"):\n        Ok(_):\n            print(\"ok1\")\n        Err(e):\n            print(\"err1\")\n    match fs.mkdir_all(\"{}/link\"):\n        Ok(_):\n            print(\"ok2\")\n        Err(e):\n            print(\"err2\")\n",
            duong(goc),
            duong(goc)
        )
    };
    let o = hai_duong_khac(&d, "symlink", &ct(&i), &ct(&v));
    assert_eq!(o, "err1\nerr2\n");
    assert!(
        !i.join("that").join("con").exists(),
        "không được đi theo link"
    );
    assert!(
        !v.join("that").join("con").exists(),
        "không được đi theo link"
    );
    let _ = std::fs::remove_dir_all(&d);
}

// ---------- quyền: không lách FsGuard ----------

/// `cap[Dir]` **không** `(write)` ⇒ `VS0604` nêu `(write)`; đích **ngoài** root (kể cả đường
/// `root/../…) ⇒ `VS0604`; exit 2, cả hai đường cùng kết quả, không tạo thư mục nào.
#[test]
fn quyen_read_only_va_ngoai_root_bi_chan() {
    let d = tam("quyen");
    let root = d.join("root");
    let ngoai = d.join("ngoai");
    std::fs::create_dir_all(&root).expect("root");
    std::fs::create_dir_all(&ngoai).expect("ngoai");
    let r = duong(&root);
    let ops = [
        (format!("fs.mkdir_all(\"{r}/con/sau\")"), true, "root/con"),
        (
            format!("fs.mkdir_all(\"{}/con/sau\")", duong(&ngoai)),
            false,
            "ngoai/con",
        ),
        (
            format!("fs.mkdir_all(\"{r}/../ngoai/con2/sau\")"),
            false,
            "ngoai/con2",
        ),
    ];
    for (i, (op, trong_root, tao)) in ops.iter().enumerate() {
        let src = format!(
            "use std.fs\n\nfn main(r: cap[Dir] @ p\"{r}\"):\n    match {op}:\n        Ok(_):\n            print(\"OK\")\n        Err(e):\n            print(\"ERR\")\n"
        );
        viet_tep(&d.join("f.vis"), &src);
        let r1 = chay(&d, &["run", "f.vis"]);
        let r2 = chay(&d, &["run", "--vm", "--no-cache", "f.vis"]);
        assert_eq!(r1.0, 2, "ca {i}: interp phải exit 2: {r1:?}");
        assert_eq!(r2.0, 2, "ca {i}: vm phải exit 2: {r2:?}");
        assert_eq!(r1.1, "", "ca {i}: không được in kết quả");
        assert_eq!(r1.1, r2.1, "ca {i}: stdout lệch");
        for (duong_chay, rr) in [("interp", &r1), ("vm", &r2)] {
            assert!(
                rr.2.contains("VS0604"),
                "ca {i} ({duong_chay}): thiếu VS0604: {rr:?}"
            );
            if *trong_root {
                assert!(
                    rr.2.contains("(write)"),
                    "ca {i} ({duong_chay}): thiếu `(write)`: {rr:?}"
                );
            } else {
                assert!(
                    rr.2.contains("cap[Dir]"),
                    "ca {i} ({duong_chay}): thiếu `cap[Dir]`: {rr:?}"
                );
            }
        }
        assert!(!d.join(tao).exists(), "ca {i}: bị chặn mà vẫn tạo `{tao}`");
    }
    let _ = std::fs::remove_dir_all(&d);
}

// ---------- host giả lập ----------

/// `--fs-mock`: tạo **đệ quy trong bộ nhớ**, đích đã có ⇒ `Ok`, tệp trên đường ⇒ `Err`; hai đường
/// byte-identical; mỗi tiến trình nạp kịch bản mới.
#[test]
fn mock_de_quy_va_parity() {
    let d = tam("mock");
    let mock = r#"{ "dirs": { "co": [] }, "files": { "tep": "x" } }"#;
    let o = hai_duong_mock(
        &d,
        "mock",
        mock,
        "use std.fs\nfn main() -> ():\n    match fs.mkdir_all(\"x/y/z\"):\n        Ok(_):\n            print(\"ok\")\n        Err(e):\n            print(\"err\")\n    print(fs.exists(\"x/y\"))\n    match fs.is_dir(\"x/y/z\"):\n        Ok(b):\n            print(b)\n        Err(e):\n            print(\"dir-err\")\n    match fs.mkdir_all(\"co\"):\n        Ok(_):\n            print(\"co-ok\")\n        Err(e):\n            print(\"co-err\")\n    match fs.mkdir_all(\"tep/a\"):\n        Ok(_):\n            print(\"tep-ok\")\n        Err(e):\n            print(\"tep-err\")\n    match fs.mkdir_all(\"x\"):\n        Ok(_):\n            print(\"lai-ok\")\n        Err(e):\n            print(\"lai-err\")\n",
    );
    assert_eq!(o, "ok\ntrue\ntrue\nco-ok\ntep-err\nlai-ok\n");
    let _ = std::fs::remove_dir_all(&d);
}

// ---------- kiểu/arity sai ----------

/// Sai kiểu hoặc sai số đối số ⇒ lỗi **tĩnh** (exit 2), nêu đúng tên hàm; không chạy chương trình.
#[test]
fn sai_kieu_va_arity() {
    let d = tam("kieu");
    let cases = [
        (
            "use std.fs\nfn main() -> ():\n    match fs.mkdir_all(123):\n        Ok(_):\n            print(\"ok\")\n        Err(e):\n            print(\"e\")\n",
            "VS0412",
        ),
        (
            "use std.fs\nfn main() -> ():\n    match fs.mkdir_all():\n        Ok(_):\n            print(\"ok\")\n        Err(e):\n            print(\"e\")\n",
            "VS0401",
        ),
        (
            "use std.fs\nfn main() -> ():\n    match fs.mkdir_all(\"a\", \"b\"):\n        Ok(_):\n            print(\"ok\")\n        Err(e):\n            print(\"e\")\n",
            "VS0401",
        ),
    ];
    for (i, (src, ma_loi)) in cases.iter().enumerate() {
        viet_tep(&d.join("f.vis"), src);
        let (ma, out, err) = chay(&d, &["run", "f.vis"]);
        assert_eq!(
            ma, 2,
            "ca {i}: phải là lỗi tĩnh exit 2: out={out:?} err={err:?}"
        );
        assert_eq!(out, "", "ca {i}: không được chạy/in");
        assert!(
            err.contains("mkdir_all"),
            "ca {i}: thiếu tên `mkdir_all`: {err:?}"
        );
        assert!(
            err.contains(ma_loi),
            "ca {i}: thiếu mã `{ma_loi}` (không phải VS0301 thiếu hàm): {err:?}"
        );
        assert!(
            !err.contains("không có hàm"),
            "ca {i}: hàm đã có, không được báo VS0301: {err:?}"
        );
    }
    let _ = std::fs::remove_dir_all(&d);
}
