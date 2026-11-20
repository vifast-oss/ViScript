// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-266** — bốn helper `std.fs` thực dụng: `read_lines`, `append`, `mkdir`, `remove`.
//!
//! Khoá: ngữ nghĩa dòng khớp `lines()` (bỏ `\r`, dòng cuối rỗng do newline cuối không xuất hiện),
//! `append` **không** cắt tệp và tạo được tệp thiếu, `mkdir` **một cấp** (lỗi nếu đã tồn tại),
//! `remove` **chỉ tệp** (từ chối thư mục), cùng **một host giả lập** (`--fs-mock`), và các thao tác
//! ghi vẫn đi qua `FsGuard` ⇒ thiếu `(write)`/ra ngoài root là `VS0604` (exit 2), **không** lách.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

static DEM: AtomicU32 = AtomicU32::new(0);

/// Thư mục tạm riêng cho từng ca (test chạy song song).
fn tam(tag: &str) -> PathBuf {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-vd266-{tag}-{}-{n}", std::process::id()));
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

/// Chạy **cả hai đường** (interp + `--vm --no-cache`) trên **cùng** tệp chương trình, đòi cùng
/// stdout/exit; trả stdout. Chỉ dùng cho chương trình **tất định** khi chạy lặp (mỗi đường một lần).
fn hai_duong(d: &Path, tag: &str, src: &str) -> String {
    let p = d.join(format!("{tag}.vis"));
    viet_tep(&p, src);
    let (c1, o1, e1) = vis(&[std::ffi::OsStr::new("run"), p.as_os_str()]);
    let (c2, o2, e2) = vis(&[
        std::ffi::OsStr::new("run"),
        std::ffi::OsStr::new("--vm"),
        std::ffi::OsStr::new("--no-cache"),
        p.as_os_str(),
    ]);
    assert_eq!(c1, 0, "[{tag}] interp exit {c1}: {e1}");
    assert_eq!(c2, 0, "[{tag}] vm exit {c2}: {e2}");
    assert_eq!(o1, o2, "[{tag}] hai đường phải khớp: vm [{e2}]");
    o1
}

/// Như [`hai_duong`] nhưng hai đường dùng **hai tệp chương trình khác nhau** — cho thao tác
/// **không chạy lặp được** trên cùng đường dẫn (`mkdir` lỗi khi đã tồn tại). Hai nguồn chỉ khác
/// đường dẫn; stdout phải **byte-identical**.
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

// ---------- read_lines ----------

/// CRLF + newline cuối: bỏ `\r`, không sinh dòng rỗng cuối; tệp rỗng ⇒ `[]`. Cả hai đường khớp.
#[test]
fn read_lines_crlf_va_newline_cuoi() {
    let d = tam("rl");
    let p = d.join("dulieu.txt");
    std::fs::write(&p, "một\r\nhai\r\nba\n").expect("ghi dữ liệu");
    let rong = d.join("rong.txt");
    std::fs::write(&rong, "").expect("ghi rỗng");
    let o = hai_duong(
        &d,
        "rl",
        &format!(
            "use std.fs\nfn main() -> ():\n    match fs.read_lines(\"{}\"):\n        Ok(xs):\n            print(xs.len())\n            for x in xs:\n                print(x)\n        Err(e):\n            print(\"err\")\n    match fs.read_lines(\"{}\"):\n        Ok(xs):\n            print(xs.len())\n        Err(e):\n            print(\"err\")\n",
            duong(&p),
            duong(&rong)
        ),
    );
    assert_eq!(o, "3\nmột\nhai\nba\n0\n");
    let _ = std::fs::remove_dir_all(&d);
}

/// Không newline cuối vẫn giữ dòng cuối; dòng trống giữa tệp vẫn là phần tử rỗng.
#[test]
fn read_lines_khong_newline_cuoi_va_dong_trong() {
    let d = tam("rl2");
    let p = d.join("dulieu.txt");
    std::fs::write(&p, "a\n\nb").expect("ghi dữ liệu");
    let o = hai_duong(
        &d,
        "rl2",
        &format!(
            "use std.fs\nfn main() -> ():\n    match fs.read_lines(\"{}\"):\n        Ok(xs):\n            print(xs.len())\n            for x in xs:\n                print(x)\n        Err(e):\n            print(\"err\")\n",
            duong(&p)
        ),
    );
    assert_eq!(o, "3\na\n\nb\n");
    let _ = std::fs::remove_dir_all(&d);
}

/// Tệp thiếu ⇒ `Err` dữ liệu (in `err`), **không** trap.
#[test]
fn read_lines_tep_thieu_la_err() {
    let d = tam("rl3");
    let p = d.join("khong.txt");
    let o = hai_duong(
        &d,
        "rl3",
        &format!(
            "use std.fs\nfn main() -> ():\n    match fs.read_lines(\"{}\"):\n        Ok(xs):\n            print(\"ok\")\n        Err(e):\n            print(\"err\")\n",
            duong(&p)
        ),
    );
    assert_eq!(o, "err\n");
    let _ = std::fs::remove_dir_all(&d);
}

// ---------- append ----------

/// `fs.write` đặt nội dung gốc rồi `append` hai lần ⇒ `ABB`, **không** cắt tệp. Chương trình tự
/// `write` đầu mỗi lần chạy nên chạy được cả hai đường trên cùng tệp mà vẫn tất định.
#[test]
fn append_hai_lan_khong_truncate() {
    let d = tam("ap");
    let p = d.join("nhat-ky.txt");
    let o = hai_duong(
        &d,
        "ap",
        &format!(
            "use std.fs\nfn main() -> ():\n    match fs.write(\"{}\", \"A\"):\n        Ok(_):\n            print(\"w\")\n        Err(e):\n            print(\"err\")\n    match fs.append(\"{}\", \"B\"):\n        Ok(_):\n            print(\"a1\")\n        Err(e):\n            print(\"err\")\n    match fs.append(\"{}\", \"B\"):\n        Ok(_):\n            print(\"a2\")\n        Err(e):\n            print(\"err\")\n    match fs.read(\"{}\"):\n        Ok(s):\n            print(s)\n        Err(e):\n            print(\"err\")\n",
            duong(&p),
            duong(&p),
            duong(&p),
            duong(&p)
        ),
    );
    assert_eq!(o, "w\na1\na2\nABB\n");
    assert_eq!(
        std::fs::read_to_string(&p).expect("đọc lại"),
        "ABB",
        "append phải không cắt tệp"
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// `append` tạo tệp khi thiếu; chương trình `remove` trước (bỏ qua lỗi khi chưa có) nên lặp được.
#[test]
fn append_tao_tep_khi_thieu() {
    let d = tam("ap2");
    let p = d.join("moi.txt");
    let o = hai_duong(
        &d,
        "ap2",
        &format!(
            "use std.fs\nfn main() -> ():\n    match fs.remove(\"{}\"):\n        Ok(_):\n            print(\"r\")\n        Err(e):\n            print(\"r\")\n    match fs.append(\"{}\", \"moi\"):\n        Ok(_):\n            print(\"a\")\n        Err(e):\n            print(\"err\")\n    match fs.read(\"{}\"):\n        Ok(s):\n            print(s)\n        Err(e):\n            print(\"err\")\n",
            duong(&p),
            duong(&p),
            duong(&p)
        ),
    );
    assert_eq!(o, "r\na\nmoi\n");
    let _ = std::fs::remove_dir_all(&d);
}

// ---------- mkdir / remove ----------

/// `mkdir` **một cấp**: tạo được, lần hai ⇒ `Err` (đã tồn tại). Mỗi đường dùng thư mục riêng vì
/// không có thao tác xoá thư mục (chủ ý: `remove` từ chối thư mục).
#[test]
fn mkdir_mot_cap_va_loi_khi_da_ton_tai() {
    let d = tam("mk");
    let nd = |tag: &str| d.join(format!("thu-muc-{tag}"));
    let ct = |p: &Path| {
        format!(
            "use std.fs\nfn main() -> ():\n    match fs.mkdir(\"{}\"):\n        Ok(_):\n            print(\"ok\")\n        Err(e):\n            print(\"err\")\n    print(fs.exists(\"{}\"))\n    match fs.mkdir(\"{}\"):\n        Ok(_):\n            print(\"ok2\")\n        Err(e):\n            print(\"err2\")\n",
            duong(p),
            duong(p),
            duong(p)
        )
    };
    let o = hai_duong_khac(&d, "mk", &ct(&nd("i")), &ct(&nd("v")));
    assert_eq!(o, "ok\ntrue\nerr2\n");
    assert!(nd("i").is_dir(), "interp phải tạo thư mục thật");
    assert!(nd("v").is_dir(), "vm phải tạo thư mục thật");
    let _ = std::fs::remove_dir_all(&d);
}

/// `mkdir` thiếu thư mục cha ⇒ `Err` (một cấp, không phải `mkdir -p`).
#[test]
fn mkdir_thieu_cha_la_err() {
    let d = tam("mk2");
    let p = d.join("chua-co").join("con");
    let o = hai_duong(
        &d,
        "mk2",
        &format!(
            "use std.fs\nfn main() -> ():\n    match fs.mkdir(\"{}\"):\n        Ok(_):\n            print(\"ok\")\n        Err(e):\n            print(\"err\")\n",
            duong(&p)
        ),
    );
    assert_eq!(o, "err\n");
    let _ = std::fs::remove_dir_all(&d);
}

/// `remove` xoá **tệp**; từ chối **thư mục** (không đệ quy). Chương trình tự tạo lại tệp mỗi lần
/// chạy nên hai đường dùng chung thư mục tất định.
#[test]
fn remove_chi_tep_tu_choi_thu_muc() {
    let d = tam("rm");
    let p = d.join("tep.txt");
    let sub = d.join("thu-muc");
    std::fs::create_dir_all(&sub).expect("mkdir");
    let o = hai_duong(
        &d,
        "rm",
        &format!(
            "use std.fs\nfn main() -> ():\n    match fs.write(\"{}\", \"x\"):\n        Ok(_):\n            print(\"w\")\n        Err(e):\n            print(\"err\")\n    match fs.remove(\"{}\"):\n        Ok(_):\n            print(\"rm\")\n        Err(e):\n            print(\"err\")\n    print(fs.exists(\"{}\"))\n    match fs.remove(\"{}\"):\n        Ok(_):\n            print(\"rmdir\")\n        Err(e):\n            print(\"err-dir\")\n",
            duong(&p),
            duong(&p),
            duong(&p),
            duong(&sub)
        ),
    );
    assert_eq!(o, "w\nrm\nfalse\nerr-dir\n");
    assert!(sub.is_dir(), "remove **không** được xoá thư mục");
    let _ = std::fs::remove_dir_all(&d);
}

// ---------- host giả lập ----------

/// `--fs-mock`: cả bốn helper chạy **trong bộ nhớ**, hai đường byte-identical; mỗi tiến trình nạp
/// kịch bản mới nên `append`/`remove` không rò trạng thái sang lần chạy sau.
#[test]
fn mock_host_bon_helper() {
    let d = tam("mock");
    let mock = r#"{ "files": { "a.txt": "một\r\nhai\n" } }"#;
    let o = hai_duong_mock(
        &d,
        "mock",
        mock,
        "use std.fs\nfn main() -> ():\n    match fs.read_lines(\"a.txt\"):\n        Ok(xs):\n            print(xs.len())\n            for x in xs:\n                print(x)\n        Err(e):\n            print(\"err-read\")\n    match fs.append(\"a.txt\", \"-them\"):\n        Ok(_):\n            print(\"append-ok\")\n        Err(e):\n            print(\"append-err\")\n    match fs.read(\"a.txt\"):\n        Ok(s):\n            print(s)\n        Err(e):\n            print(\"err\")\n    match fs.mkdir(\"thu-muc\"):\n        Ok(_):\n            print(\"mkdir-ok\")\n        Err(e):\n            print(\"mkdir-err\")\n    match fs.remove(\"a.txt\"):\n        Ok(_):\n            print(\"remove-ok\")\n        Err(e):\n            print(\"remove-err\")\n    print(fs.exists(\"a.txt\"))\n",
    );
    assert_eq!(
        o,
        "2\nmột\nhai\nappend-ok\nmột\r\nhai\n-them\nmkdir-ok\nremove-ok\nfalse\n"
    );
    let _ = std::fs::remove_dir_all(&d);
}

// ---------- quyền: không lách FsGuard ----------

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

/// Ba thao tác **ghi** (`append`/`mkdir`/`remove`) trong `cap[Dir]` **không** `(write)` ⇒ `VS0604`
/// nêu `(write)`, exit 2. Cả hai đường cùng kết quả.
#[test]
fn ghi_thieu_write_bi_chan() {
    let d = tam("quyen-ghi");
    let root = d.join("root");
    std::fs::create_dir_all(&root).expect("root");
    std::fs::write(root.join("a.txt"), "x").expect("tệp");
    let ops = [
        format!("fs.append(\"{}/a.txt\", \"y\")", duong(&root)),
        format!("fs.mkdir(\"{}/moi\")", duong(&root)),
        format!("fs.remove(\"{}/a.txt\")", duong(&root)),
    ];
    for (i, op) in ops.iter().enumerate() {
        let src = format!(
            "use std.fs\n\nfn main(r: cap[Dir] @ p\"{}\"):\n    match {op}:\n        Ok(_):\n            print(\"OK\")\n        Err(e):\n            print(\"ERR\")\n",
            duong(&root)
        );
        viet_tep(&d.join("f.vis"), &src);
        let r1 = chay(&d, &["run", "f.vis"]);
        let r2 = chay(&d, &["run", "--vm", "--no-cache", "f.vis"]);
        assert_eq!(r1.0, 2, "ca {i}: interp phải exit 2: {r1:?}");
        assert_eq!(r2.0, 2, "ca {i}: vm phải exit 2: {r2:?}");
        assert_eq!(r1.1, "", "ca {i}: không được in kết quả");
        assert_eq!(r1.1, r2.1, "ca {i}: stdout lệch");
        for (duong_chay, r) in [("interp", &r1), ("vm", &r2)] {
            assert!(
                r.2.contains("VS0604"),
                "ca {i} ({duong_chay}): thiếu VS0604: {r:?}"
            );
            assert!(
                r.2.contains("(write)"),
                "ca {i} ({duong_chay}): thiếu `(write)`: {r:?}"
            );
        }
    }
    let _ = std::fs::remove_dir_all(&d);
}

/// `read_lines` **ngoài** root ⇒ `VS0604` (đọc vẫn qua `FsGuard::read` mở-rồi-kiểm), exit 2.
#[test]
fn read_lines_ngoai_quyen_bi_chan() {
    let d = tam("quyen-doc");
    let root = d.join("root");
    std::fs::create_dir_all(&root).expect("root");
    let ngoai = d.join("ngoai.txt");
    std::fs::write(&ngoai, "bi mat\n").expect("tệp ngoài");
    let src = format!(
        "use std.fs\n\nfn main(r: cap[Dir] @ p\"{}\"):\n    match fs.read_lines(\"{}/../ngoai.txt\"):\n        Ok(xs):\n            print(xs.len())\n        Err(e):\n            print(\"ERR\")\n",
        duong(&root),
        duong(&root)
    );
    viet_tep(&d.join("f.vis"), &src);
    let r1 = chay(&d, &["run", "f.vis"]);
    let r2 = chay(&d, &["run", "--vm", "--no-cache", "f.vis"]);
    assert_eq!(r1.0, 2, "interp phải exit 2: {r1:?}");
    assert_eq!(r2.0, 2, "vm phải exit 2: {r2:?}");
    assert_eq!(r1.1, "", "không được in nội dung ngoài root");
    assert_eq!(r1.1, r2.1, "stdout lệch");
    assert!(r1.2.contains("VS0604"), "interp: {r1:?}");
    assert!(r2.2.contains("VS0604"), "vm: {r2:?}");
    let _ = std::fs::remove_dir_all(&d);
}
