// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270** — byte thô của `std.fs`: `read_bytes`, `write_bytes`.
//!
//! Hợp đồng (đóng băng bằng test):
//! - `fs.read_bytes(path) -> Result[list[int], Error]`: đọc **toàn bộ** tệp thành `list[int]`,
//!   mỗi phần tử `0..=255`; thiếu tệp/thư mục/lỗi đọc ⇒ `Err(Error)` dữ liệu, **không** trap.
//! - `fs.write_bytes(path, data) -> Result[(), Error]`: tạo mới hoặc **cắt** rồi ghi đúng từng
//!   byte; **mọi** phần tử phải `0..=255` — kiểm **hết** trước khi chạm đĩa, sai ⇒ `Err(Error)`,
//!   **không** tạo/cắt tệp (không ghi một phần).
//! - quyền: `read_bytes` là thao tác **đọc**, `write_bytes` là thao tác **ghi** — vẫn đi qua
//!   `FsGuard` (ngoài root/thiếu `(write)` ⇒ `VS0604`, exit 2), không lách.
//! - host thật (`ngoai.rs`), `--fs-mock`, interp và VM dùng **cùng** một ngữ nghĩa.
//!
//! RED-BEFORE (ghi lúc làm VD-270): trước khi thêm hai hàm, chương trình báo lỗi tĩnh
//! «không có hàm `read_bytes`/`write_bytes`» (`VS0301`, exit 2) — xem báo cáo bàn giao.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

static DEM: AtomicU32 = AtomicU32::new(0);

/// Thư mục tạm riêng cho từng ca (test chạy song song).
fn tam(tag: &str) -> PathBuf {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-vd270-fsb-{tag}-{}-{n}", std::process::id()));
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
/// stdout/exit; trả stdout. Chương trình phải **tất định** khi chạy lặp (mỗi đường một lần).
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

/// Như [`hai_duong`] nhưng cả hai đường chạy với `--fs-mock`.
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

// ---------- roundtrip 0..=255 ----------

/// Ghi đủ **256 byte** `0x00..=0xFF` (gồm `0x00` và `0xFF`) rồi đọc lại: đúng độ dài, đúng các
/// byte biên; tệp trên đĩa **byte-identical** với dãy gốc. Hai đường khớp.
#[test]
fn roundtrip_du_256_byte_gom_00_va_ff() {
    let d = tam("rt");
    let p = d.join("nhi.bin");
    let o = hai_duong(
        &d,
        "rt",
        &format!(
            "use std.fs\nfn main() -> ():\n    var xs: list[int] = []\n    for i in 0..256:\n        xs.push(i)\n    match fs.write_bytes(\"{}\", xs):\n        Ok(_):\n            print(\"w-ok\")\n        Err(e):\n            print(\"w-err\")\n    match fs.read_bytes(\"{}\"):\n        Ok(ys):\n            print(ys.len())\n            print(ys[0])\n            print(ys[1])\n            print(ys[127])\n            print(ys[254])\n            print(ys[255])\n        Err(e):\n            print(\"r-err\")\n",
            duong(&p),
            duong(&p)
        ),
    );
    assert_eq!(o, "w-ok\n256\n0\n1\n127\n254\n255\n");
    let tren_dia: Vec<u8> = (0..=255).collect();
    assert_eq!(
        std::fs::read(&p).expect("đọc lại tệp"),
        tren_dia,
        "tệp phải chứa đúng chuỗi byte 0..=255"
    );
    let _ = std::fs::remove_dir_all(&d);
}

// ---------- rỗng + ghi rồi đọc ----------

/// `write_bytes` với danh sách rỗng tạo tệp **0 byte** (không phải lỗi); `read_bytes` tệp rỗng
/// trả `Ok([])`; ghi `[0, 128, 255]` rồi đọc lại đúng thứ tự. Hai đường khớp.
#[test]
fn rong_va_ghi_doc_lai() {
    let d = tam("rong");
    let rong = d.join("rong.bin");
    let ba = d.join("ba.bin");
    let o = hai_duong(
        &d,
        "rong",
        &format!(
            "use std.fs\nfn main() -> ():\n    let khong: list[int] = []\n    match fs.write_bytes(\"{}\", khong):\n        Ok(_):\n            print(\"rong-ok\")\n        Err(e):\n            print(\"rong-err\")\n    print(fs.exists(\"{}\"))\n    match fs.size(\"{}\"):\n        Ok(n):\n            print(n)\n        Err(e):\n            print(\"size-err\")\n    match fs.read_bytes(\"{}\"):\n        Ok(ys):\n            print(ys.len())\n        Err(e):\n            print(\"doc-err\")\n    let ba: list[int] = [0, 128, 255]\n    match fs.write_bytes(\"{}\", ba):\n        Ok(_):\n            print(\"ba-ok\")\n        Err(e):\n            print(\"ba-err\")\n    match fs.read_bytes(\"{}\"):\n        Ok(ys):\n            for b in ys:\n                print(b)\n        Err(e):\n            print(\"doc2-err\")\n",
            duong(&rong),
            duong(&rong),
            duong(&rong),
            duong(&rong),
            duong(&ba),
            duong(&ba)
        ),
    );
    assert_eq!(o, "rong-ok\ntrue\n0\n0\nba-ok\n0\n128\n255\n");
    assert_eq!(
        std::fs::read(&rong).expect("đọc tệp rỗng"),
        Vec::<u8>::new(),
        "tệp rỗng phải 0 byte"
    );
    assert_eq!(
        std::fs::read(&ba).expect("đọc tệp ba byte"),
        vec![0u8, 128, 255],
        "ghi rồi đọc phải giữ nguyên byte"
    );
    let _ = std::fs::remove_dir_all(&d);
}

// ---------- lỗi dữ liệu: thiếu tệp / thư mục ----------

/// Tệp thiếu và **thư mục** ⇒ `Err(Error)` dữ liệu (`err`), chương trình chạy tiếp — không trap.
#[test]
fn thieu_tep_va_thu_muc_la_err() {
    let d = tam("loi");
    let thieu = d.join("khong-co.bin");
    let thu_muc = d.join("thu-muc");
    std::fs::create_dir_all(&thu_muc).expect("mkdir");
    let o = hai_duong(
        &d,
        "loi",
        &format!(
            "use std.fs\nfn main() -> ():\n    match fs.read_bytes(\"{}\"):\n        Ok(ys):\n            print(\"thieu-ok\")\n        Err(e):\n            print(\"thieu-err\")\n    match fs.read_bytes(\"{}\"):\n        Ok(ys):\n            print(\"dir-ok\")\n        Err(e):\n            print(\"dir-err\")\n    print(\"xong\")\n",
            duong(&thieu),
            duong(&thu_muc)
        ),
    );
    assert_eq!(o, "thieu-err\ndir-err\nxong\n");
    let _ = std::fs::remove_dir_all(&d);
}

// ---------- lỗi dữ liệu: ngoài 0..=255, không ghi một phần ----------

/// Phần tử `< 0` hoặc `> 255` ⇒ `Err(Error)`; tệp **chưa có** vẫn **không** được tạo, tệp đã có
/// **không** bị cắt/ghi đè (kiểm hết trước khi chạm đĩa). Hai đường khớp.
#[test]
fn ngoai_pham_vi_la_err_khong_tao_khong_cat() {
    let d = tam("mien");
    let moi = d.join("moi.bin");
    let giu = d.join("giu.bin");
    std::fs::write(&giu, b"OLD").expect("tệp cũ");
    let o = hai_duong(
        &d,
        "mien",
        &format!(
            "use std.fs\nfn main() -> ():\n    let cao: list[int] = [0, 256]\n    match fs.write_bytes(\"{moi}\", cao):\n        Ok(_):\n            print(\"cao-ok\")\n        Err(e):\n            print(\"cao-err\")\n    print(fs.exists(\"{moi}\"))\n    let am: list[int] = [-1]\n    match fs.write_bytes(\"{moi}\", am):\n        Ok(_):\n            print(\"am-ok\")\n        Err(e):\n            print(\"am-err\")\n    print(fs.exists(\"{moi}\"))\n    let hong: list[int] = [7, 255, 999999999999]\n    match fs.write_bytes(\"{giu}\", hong):\n        Ok(_):\n            print(\"giu-ok\")\n        Err(e):\n            print(\"giu-err\")\n    match fs.read(\"{giu}\"):\n        Ok(s):\n            print(s)\n        Err(e):\n            print(\"doc-err\")\n",
            moi = duong(&moi),
            giu = duong(&giu)
        ),
    );
    assert_eq!(o, "cao-err\nfalse\nam-err\nfalse\ngiu-err\nOLD\n");
    assert!(!moi.exists(), "write_bytes lỗi không được tạo tệp mới");
    assert_eq!(
        std::fs::read(&giu).expect("đọc tệp cũ"),
        b"OLD",
        "write_bytes lỗi không được cắt/ghi đè tệp đã có"
    );
    let _ = std::fs::remove_dir_all(&d);
}

// ---------- kiểu/arity sai ----------

/// Sai kiểu hoặc sai số đối số ⇒ lỗi **tĩnh** (exit 2), nêu đúng tên hàm; không chạy chương trình.
#[test]
fn sai_kieu_va_arity() {
    let d = tam("kieu");
    let cases = [
        (
            "use std.fs\nfn main() -> ():\n    match fs.read_bytes(123):\n        Ok(x):\n            print(x)\n        Err(e):\n            print(\"e\")\n",
            "read_bytes",
            "VS0412",
        ),
        (
            "use std.fs\nfn main() -> ():\n    match fs.write_bytes(\"a.bin\", \"abc\"):\n        Ok(_):\n            print(\"ok\")\n        Err(e):\n            print(\"e\")\n",
            "write_bytes",
            "VS0412",
        ),
        (
            "use std.fs\nfn main() -> ():\n    match fs.write_bytes(\"a.bin\"):\n        Ok(_):\n            print(\"ok\")\n        Err(e):\n            print(\"e\")\n",
            "write_bytes",
            "VS0401",
        ),
        (
            "use std.fs\nfn main() -> ():\n    match fs.read_bytes():\n        Ok(x):\n            print(x)\n        Err(e):\n            print(\"e\")\n",
            "read_bytes",
            "VS0401",
        ),
    ];
    for (i, (src, ten, ma_loi)) in cases.iter().enumerate() {
        viet_tep(&d.join("f.vis"), src);
        let (ma, out, err) = chay(&d, &["run", "f.vis"]);
        assert_eq!(
            ma, 2,
            "ca {i}: phải là lỗi tĩnh exit 2: out={out:?} err={err:?}"
        );
        assert_eq!(out, "", "ca {i}: không được chạy/in");
        assert!(err.contains(ten), "ca {i}: thiếu tên `{ten}`: {err:?}");
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

// ---------- host giả lập ----------

/// `--fs-mock`: đọc byte từ chuỗi kịch bản (UTF-8), ghi byte trong bộ nhớ, đọc lại đúng; byte
/// không phải UTF-8 làm `fs.read` (chuỗi) lỗi nhưng `read_bytes` vẫn đọc được. Hai đường khớp.
#[test]
fn mock_host_bytes() {
    let d = tam("mock");
    let mock = r#"{ "files": { "a.txt": "abc" } }"#;
    let o = hai_duong_mock(
        &d,
        "mock",
        mock,
        "use std.fs\nfn main() -> ():\n    match fs.read_bytes(\"a.txt\"):\n        Ok(xs):\n            print(xs.len())\n            for b in xs:\n                print(b)\n        Err(e):\n            print(\"r-err\")\n    let moi: list[int] = [0, 255, 128]\n    match fs.write_bytes(\"a.txt\", moi):\n        Ok(_):\n            print(\"w-ok\")\n        Err(e):\n            print(\"w-err\")\n    match fs.read_bytes(\"a.txt\"):\n        Ok(xs):\n            for b in xs:\n                print(b)\n        Err(e):\n            print(\"r2-err\")\n    match fs.size(\"a.txt\"):\n        Ok(n):\n            print(n)\n        Err(e):\n            print(\"size-err\")\n    match fs.read(\"a.txt\"):\n        Ok(s):\n            print(s)\n        Err(e):\n            print(\"str-err\")\n    match fs.write_bytes(\"moi.bin\", [1, 2, 3]):\n        Ok(_):\n            print(\"moi-ok\")\n        Err(e):\n            print(\"moi-err\")\n    print(fs.exists(\"moi.bin\"))\n",
    );
    assert_eq!(
        o,
        "3\n97\n98\n99\nw-ok\n0\n255\n128\n3\nstr-err\nmoi-ok\ntrue\n"
    );
    let _ = std::fs::remove_dir_all(&d);
}

// ---------- quyền: không lách FsGuard ----------

/// `read_bytes` ngoài `cap[Dir]` ⇒ `VS0604` (exit 2); `write_bytes` trong root nhưng thiếu
/// `(write)` ⇒ `VS0604`; hai đường cùng kết quả, không in nội dung.
#[test]
fn quyen_read_bytes_va_write_bytes() {
    let d = tam("quyen");
    let root = d.join("root");
    std::fs::create_dir_all(&root).expect("root");
    std::fs::write(root.join("a.txt"), "x").expect("tệp");
    let ngoai = d.join("ngoai.bin");
    std::fs::write(&ngoai, [0u8, 1, 255]).expect("tệp ngoài");
    let r = duong(&root);
    let ops = [
        format!("fs.read_bytes(\"{r}/../ngoai.bin\")"),
        format!("fs.write_bytes(\"{r}/moi.bin\", [1, 2])"),
    ];
    for (i, op) in ops.iter().enumerate() {
        let src = format!(
            "use std.fs\n\nfn main(r: cap[Dir] @ p\"{r}\"):\n    match {op}:\n        Ok(v):\n            print(\"OK\")\n        Err(e):\n            print(\"ERR\")\n"
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
        }
    }
    assert!(
        !root.join("moi.bin").exists(),
        "write_bytes bị chặn không được tạo tệp"
    );
    let _ = std::fs::remove_dir_all(&d);
}
