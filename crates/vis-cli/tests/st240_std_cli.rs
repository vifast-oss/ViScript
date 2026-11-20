// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-240 / VD-242 §3.5/§4 + VD-243** — hồi quy std/CLI hardening Đợt B (luồng C).
//!
//! Mỗi ca: repro đỏ baseline đã ghi trong `agent/JOURNAL/st235-audit-std-cli.md` và
//! `st237-ma-tran-28.md`; ở đây là trạng thái **xanh** sau sửa. `#169` (ST-238), `#170`/Option
//! (ST-239) chỉ kiểm tích hợp — không sửa/đóng lại; `#176` bị bác bỏ, không mở lại.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

static DEM: AtomicU32 = AtomicU32::new(0);

fn tam(tag: &str) -> PathBuf {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-st240-{tag}-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("tạo thư mục tạm");
    d
}

fn viet(d: &Path, ten: &str, src: &str) -> PathBuf {
    let p = d.join(ten);
    if let Some(par) = p.parent() {
        std::fs::create_dir_all(par).expect("tạo thư mục con");
    }
    std::fs::write(&p, src).expect("ghi tệp tạm");
    p
}

/// Chạy `vis run` trên **cả hai engine**; trả `(mã thoát, stdout, stderr)`.
fn chay_hai(d: &Path, ten: &str) -> [(i32, String, String); 2] {
    let mut kq = Vec::new();
    for extra in [&[][..], &["--vm"][..]] {
        let mut c = Command::new(env!("CARGO_BIN_EXE_vis"));
        c.arg("run").arg("--no-cache");
        c.args(extra);
        c.arg(ten);
        c.current_dir(d);
        let o = c.output().expect("chạy vis run");
        kq.push((
            o.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&o.stdout).to_string(),
            String::from_utf8_lossy(&o.stderr).to_string(),
        ));
    }
    [kq.remove(0), kq.remove(0)]
}

fn mot(d: &Path, args: &[&str]) -> (i32, String, String) {
    let mut c = Command::new(env!("CARGO_BIN_EXE_vis"));
    c.args(args);
    c.current_dir(d);
    let o = c.output().expect("chạy vis");
    (
        o.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&o.stdout).to_string(),
        String::from_utf8_lossy(&o.stderr).to_string(),
    )
}

// ---------------------------------------------------------------- #168
#[test]
fn sort_nan_bao_loi_khong_sap_mot_phan() {
    let d = tam("sort");
    viet(
        &d,
        "sort.vis",
        "use std.list\nprint(list.sort([3.0, 0.0/0.0, 1.0, 2.0]))\n",
    );
    for (ma, out, err) in chay_hai(&d, "sort.vis") {
        assert_ne!(ma, 0, "sort NaN phải lỗi: {out} / {err}");
        assert!(err.contains("VS0501"), "{err}");
        assert!(
            out.is_empty(),
            "không được in danh sách sắp một phần: {out}"
        );
    }
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn sort_mot_phan_tu_nan_cung_bao_loi() {
    let d = tam("sort1");
    viet(&d, "s.vis", "use std.list\nprint(list.sort([0.0/0.0]))\n");
    for (ma, out, err) in chay_hai(&d, "s.vis") {
        assert_ne!(ma, 0, "phần tử đơn NaN cũng phải lỗi: {out}");
        assert!(err.contains("VS0501"), "{err}");
    }
    let _ = std::fs::remove_dir_all(&d);
}

// ------------------------------------------------------------ #173/#175
#[test]
fn json_float_giu_dau_va_zero_dau_bi_tu_choi() {
    let d = tam("json");
    viet(
        &d,
        "j.vis",
        concat!(
            "use std.json\n",
            "print(json.encode(1.0))\n",
            "match json.decode(\"01\"):\n",
            "  Ok(v): print(\"chuoi01\")\n",
            "  Err(e): print(\"loi01\")\n",
            "match json.decode(\"100\"):\n",
            "  Ok(v): print(\"hop_le100\")\n",
            "  Err(e): print(\"sai100\")\n",
        ),
    );
    for (ma, out, err) in chay_hai(&d, "j.vis") {
        assert_eq!(ma, 0, "{out} / {err}");
        assert_eq!(out, "1.0\nloi01\nhop_le100\n", "{out}");
    }
    let _ = std::fs::remove_dir_all(&d);
}

// --------------------------------------------------------------- #169/#174
#[test]
fn json_non_finite_va_path_goc() {
    let d = tam("std");
    viet(
        &d,
        "p.vis",
        "use std.path\nprint(path.basename(\"/\"))\nprint(path.dirname(\"/\"))\n",
    );
    for (ma, out, err) in chay_hai(&d, "p.vis") {
        assert_eq!(ma, 0, "{err}");
        assert_eq!(out, "/\n/\n", "{out}");
    }
    // Chuỗi rỗng giữ hành vi cũ (không đổi ngoài ý muốn).
    viet(
        &d,
        "r.vis",
        "use std.path\nprint(path.basename(\"\"))\nprint(path.dirname(\"\"))\n",
    );
    for (ma, out, _) in chay_hai(&d, "r.vis") {
        assert_eq!(ma, 0);
        assert_eq!(out, "\n.\n", "{out}");
    }
    // #169 (đóng ở ST-238) — kiểm tích hợp: non-finite vẫn bị chặn.
    viet(&d, "nf.vis", "use std.json\nprint(json.encode(0.0/0.0))\n");
    for (ma, out, err) in chay_hai(&d, "nf.vis") {
        assert_ne!(ma, 0, "non-finite phải lỗi: {out}");
        assert!(err.contains("không hữu hạn"), "{err}");
    }
    let _ = std::fs::remove_dir_all(&d);
}

// ---------------------------------------------------------------- #172
#[test]
fn percent_vuot_mien_loi_khong_bao_hoa_hai_engine() {
    let d = tam("pct");
    let big = "9223372036854775807";
    // a) nhân hai chiều ⇒ VS0507
    viet(
        &d,
        "a.vis",
        &format!("let big = {big}\nprint(str(200pct * big))\n"),
    );
    // b) literal không biểu diễn được ⇒ từ chối trước khi dùng (cả hai engine)
    viet(&d, "b.vis", "print(str(1e300pct))\n");
    for ten in ["a.vis", "b.vis"] {
        for (ma, out, err) in chay_hai(&d, ten) {
            assert_ne!(ma, 0, "{ten}: không được bão hoà im lặng: {out} / {err}");
            assert!(
                err.contains("VS0507") || err.contains("literal percent"),
                "{err}"
            );
            assert!(out.is_empty(), "{ten}: {out}");
        }
    }
    // c) biên trên hợp lệ: 100% của i64::MAX = i64::MAX (không bị từ chối máy móc)
    viet(&d, "edge.vis", &format!("print(str(100pct * {big}))\n"));
    for (ma, out, err) in chay_hai(&d, "edge.vis") {
        assert_eq!(ma, 0, "{err}");
        assert_eq!(out, format!("{big}\n"), "{out}");
    }
    // d) trung gian lớn nhưng kết quả cuối hợp lệ — tính f64 rồi kiểm kết quả cuối.
    viet(
        &d,
        "mid.vis",
        "print(str(50pct * 1000000000000000000))\nprint(str(25pct * 4000000000000000000))\n",
    );
    for (ma, out, err) in chay_hai(&d, "mid.vis") {
        assert_eq!(ma, 0, "{err}");
        assert_eq!(out, "500000000000000000\n1000000000000000000\n", "{out}");
    }
    // e) số nguyên thường không đổi: tràn i64 vẫn VS0507 như trước.
    viet(&d, "int.vis", &format!("print(str({big} * 2))\n"));
    for (ma, _, err) in chay_hai(&d, "int.vis") {
        assert_ne!(ma, 0);
        assert!(err.contains("VS0507"), "{err}");
    }
    let _ = std::fs::remove_dir_all(&d);
}

// ---------------------------------------------------------------- #177
#[test]
fn trap_trong_module_bao_dung_tep_hai_engine() {
    let d = tam("module");
    viet(&d, "rt/a.vis", "# main\nuse ./b\n\nboom()\n");
    viet(
        &d,
        "rt/b.vis",
        "pub fn boom():\n    let xs = [1, 2]\n    print(xs[9])\n",
    );
    // VD-186/Windows path contract: tên tệp tương đối trong chẩn đoán dùng dấu phân cách của nền
    // (`/` trên Unix, `\` trên Windows). Chỉ đổi dấu phân cách trong kỳ vọng; KHÔNG nới tiêu chí
    // "phải trỏ đúng module + dòng" hay bỏ ca.
    let module = if cfg!(windows) {
        "rt\\b.vis:3:"
    } else {
        "rt/b.vis:3:"
    };
    let chinh = if cfg!(windows) {
        "rt\\a.vis:"
    } else {
        "rt/a.vis:"
    };
    for (ma, out, err) in chay_hai(&d, "rt/a.vis") {
        assert_ne!(ma, 0, "{out}");
        assert!(err.contains(module), "phải trỏ vào module: {err}");
        assert!(!err.contains(chinh), "không được gán sang tệp chính: {err}");
    }
    let _ = std::fs::remove_dir_all(&d);
}

// ------------------------------------------------------------ #178/#179
#[test]
fn crlf_cung_vi_tri_voi_lf() {
    let d = tam("crlf");
    let lf = "# dong 1\n# dong 2\nlet x: int = \"hello\"\nprint(x)\n";
    viet(&d, "lf.vis", lf);
    viet(&d, "crlf.vis", &lf.replace('\n', "\r\n"));
    let a = mot(&d, &["check", "lf.vis"]);
    let b = mot(&d, &["check", "crlf.vis"]);
    assert_eq!(a.0, 1);
    assert_eq!(b.0, 1);
    // Bỏ tên tệp trong dòng `not ok`, so phần `# VS0412 dòng:cột`.
    let cat = |s: &str| {
        s.lines()
            .find(|l| l.starts_with("# VS0412"))
            .unwrap_or("")
            .to_string()
    };
    assert_eq!(
        cat(&a.2),
        cat(&b.2),
        "CRLF phải cùng vị trí LF: {:?} vs {:?}",
        a.2,
        b.2
    );
    let _ = std::fs::remove_dir_all(&d);
}

// ---------------------------------------------------------------- #180
#[test]
fn check_json_warning_mem_tra_ok_va_exit_zero() {
    let d = tam("warn");
    viet(
        &d,
        "w.vis",
        "use std.str\n\nfn main():\n    let str = \"z\"\n    print(str)\n",
    );
    let tap = mot(&d, &["check", "w.vis"]);
    let js = mot(&d, &["check", "--json", "w.vis"]);
    assert_eq!(tap.0, 0, "{:?}", tap);
    assert_eq!(js.0, 0, "{:?}", js);
    assert!(js.1.contains("\"ok\":true"), "{}", js.1);
    assert!(
        js.1.contains("VS0211"),
        "warning vẫn phải có trong JSON: {}",
        js.1
    );
    let _ = std::fs::remove_dir_all(&d);
}

// ------------------------------------------------------------ #182/#183
#[test]
fn fmt_tep_neu_ten_khong_bi_bo_qua_va_co_ten_trong_chan_doan() {
    let d = tam("fmt");
    viet(&d, "test/fmt/bad.vis", "fn bad(:\n    print(\"x\")\n");
    viet(&d, "f_bad.vis", "fn bad(:\n");
    viet(&d, "f_bad2.vis", "fn bad(:\n");
    // #182: tệp nằm trong `test/fmt/**` nhưng được nêu đích danh ⇒ phải báo lỗi.
    let a = mot(&d, &["fmt", "--check", "test/fmt/bad.vis"]);
    assert_eq!(a.0, 2, "#182 không được bỏ qua im lặng: {:?}", a);
    assert!(a.2.contains("VS0201"), "{:?}", a);
    // #183: nhiều tệp ⇒ mỗi chẩn đoán có tên tệp.
    let b = mot(&d, &["fmt", "--check", "f_bad.vis", "f_bad2.vis"]);
    assert_eq!(b.0, 2, "{:?}", b);
    assert!(b.2.contains("f_bad.vis:"), "{:?}", b);
    assert!(b.2.contains("f_bad2.vis:"), "{:?}", b);
    // Quy tắc loại trừ vẫn áp khi **duyệt thư mục**.
    let c = mot(&d, &["fmt", "--check", "test"]);
    assert_eq!(c.0, 0, "duyệt thư mục vẫn bỏ fixture: {:?}", c);
    let _ = std::fs::remove_dir_all(&d);
}

// ---------------------------------------------------------------- #184
#[test]
fn lsp_header_cut_va_eof_sach_phan_biet_ro() {
    let d = tam("lsp");
    // Header cụt ⇒ exit 1 + stderr rõ (không phải EOF sạch exit 0).
    let mut c = Command::new(env!("CARGO_BIN_EXE_vis"));
    c.args(["lsp", "--stdio"])
        .current_dir(&d)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    let mut con = c.spawn().expect("spawn vis lsp");
    use std::io::Write;
    con.stdin
        .take()
        .unwrap()
        .write_all(b"Content-Type: application/vscode-jsonrpc; charset=utf-8\r\n")
        .expect("ghi stdin");
    let o = con.wait_with_output().expect("đợi vis lsp");
    assert_eq!(o.status.code(), Some(1), "header cụt phải exit 1");
    let err = String::from_utf8_lossy(&o.stderr);
    assert!(err.contains("EOF giữa header"), "{err}");
    // EOF sạch (không có byte nào) vẫn exit 0 — không bị "làm đỏ" quá đà.
    let sach = mot(&d, &["lsp", "--stdio"]);
    assert_eq!(sach.0, 0, "{:?}", sach);
    let _ = std::fs::remove_dir_all(&d);
}

// ---------------------------------------------------------------- #171
#[test]
fn ls_long_mtime_tuong_lai_khong_in_nam_sai() {
    let d = tam("ls");
    viet(&d, "future_dir/future.txt", "hi\n");
    let fut = d.join("future_dir/future.txt");
    let ok = Command::new("touch")
        .args(["-d", "2446-01-01", fut.to_str().unwrap()])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if !ok {
        eprintln!("bỏ qua #171: không đặt được mtime tương lai bằng `touch`");
        return;
    }
    viet(&d, "long.vish", "ls ./future_dir --long\n");
    let (ma, out, err) = mot(&d, &["run", "long.vish"]);
    assert_eq!(ma, 0, "{err}");
    assert!(!out.contains("1861"), "không được in năm tràn: {out}");
    assert!(
        out.contains('–'),
        "phải có dấu `–` cho mtime ngoài miền: {out}"
    );
    let _ = std::fs::remove_dir_all(&d);
}
