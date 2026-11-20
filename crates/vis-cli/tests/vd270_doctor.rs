// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (doctor)** — `vis doctor [--json]`: thông tin chẩn đoán **tất định**.
//!
//! Bằng chứng **đỏ trước** (main `5dead58`, khi `doctor` chưa phải lệnh):
//! ```text
//! $ vis doctor >out 2>err; echo $?
//! 2
//! $ wc -c out && head -1 err
//! 0 out
//! dùng: vis explain ... | vis pkg add|install|list
//! ```
//! Tức `doctor` rơi vào nhánh lệnh lạ của `main` (`_ => usage()`): exit 2 + usage gốc trên
//! stderr. Ca [`khong_con_roi_vao_nhanh_lenh_la`] khoá việc lệnh đã được nhận thật.
//!
//! Hợp đồng được kiểm:
//! - `vis doctor`: stdout đúng **5 dòng** theo thứ tự `vis`/`os`/`arch`/`sqlite`/`features`,
//!   exit 0, stderr rỗng; phiên bản lấy từ `CARGO_PKG_VERSION` của chính package;
//! - `vis doctor --json`: **một** object nén, **một dòng** + newline, khoá theo thứ tự
//!   `version`/`os`/`arch`/`sqlite`/`features`, parse lại được bằng `json.decode`;
//! - cờ/đối số lạ và `--json` kèm đối số ⇒ exit 2, **stdout rỗng**, usage `vis doctor` trên stderr;
//! - không phụ thuộc `cwd`/môi trường (cùng output ở hai thư mục khác nhau);
//! - usage gốc của `vis` (không đối số) nêu `vis doctor [--json]`.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

fn vis(args: &[&str]) -> (i32, String, String) {
    let o = Command::new(env!("CARGO_BIN_EXE_vis"))
        .args(args)
        .output()
        .expect("chạy được `vis`");
    (
        o.status.code().unwrap_or(-1),
        String::from_utf8(o.stdout).expect("stdout phải là UTF-8"),
        String::from_utf8_lossy(&o.stderr).into_owned(),
    )
}

fn tam(tag: &str) -> PathBuf {
    static DEM: AtomicUsize = AtomicUsize::new(0);
    let d = std::env::temp_dir().join(format!(
        "vis-vd270-doctor-{tag}-{}-{}",
        std::process::id(),
        DEM.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("tạo thư mục tạm");
    d
}

/// Danh sách feature **cùng nguồn** với sản phẩm: hiện chỉ `sqlite` (nếu bật lúc biên dịch test).
fn features() -> Vec<&'static str> {
    if cfg!(feature = "sqlite") {
        vec!["sqlite"]
    } else {
        Vec::new()
    }
}

fn sqlite_bat() -> bool {
    cfg!(feature = "sqlite")
}

fn ky_vong_thuong() -> String {
    let features = features();
    let dong_features = if features.is_empty() {
        "features".to_string()
    } else {
        format!("features {}", features.join(","))
    };
    format!(
        "vis {}\nos {}\narch {}\nsqlite {}\n{}\n",
        env!("CARGO_PKG_VERSION"),
        std::env::consts::OS,
        std::env::consts::ARCH,
        if sqlite_bat() { "on" } else { "off" },
        dong_features
    )
}

fn ky_vong_json() -> String {
    let features = features()
        .iter()
        .map(|f| format!("\"{f}\""))
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{{\"version\":\"{}\",\"os\":\"{}\",\"arch\":\"{}\",\"sqlite\":{},\"features\":[{}]}}\n",
        env!("CARGO_PKG_VERSION"),
        std::env::consts::OS,
        std::env::consts::ARCH,
        sqlite_bat(),
        features
    )
}

/// (1) Plain: đúng 5 dòng, đúng thứ tự, exit 0, stderr rỗng.
#[test]
fn thuong_nam_dong_dung_thu_tu() {
    let (ma, ra, loi) = vis(&["doctor"]);
    assert_eq!(ma, 0, "phải thoát 0; stderr: {loi}");
    assert_eq!(loi, "", "plain không được in stderr");
    assert_eq!(ra, ky_vong_thuong(), "sai nội dung/thứ tự 5 dòng");
    assert_eq!(ra.lines().count(), 5, "đúng 5 dòng: {ra:?}");
    assert!(ra.ends_with('\n'), "dòng cuối phải có newline");
    let dong_dau = format!("vis {}", env!("CARGO_PKG_VERSION"));
    assert_eq!(
        ra.lines().next(),
        Some(dong_dau.as_str()),
        "dòng đầu: {ra:?}"
    );
    assert!(
        ra.lines().nth(3) == Some("sqlite on") || ra.lines().nth(3) == Some("sqlite off"),
        "dòng sqlite phải on/off: {ra:?}"
    );
}

/// (2) JSON: đúng chuỗi, đúng một dòng + newline, parse lại được và đủ 5 trường.
#[test]
fn json_mot_dong_dung_khoa_va_parse_duoc() {
    let (ma, ra, loi) = vis(&["doctor", "--json"]);
    assert_eq!(ma, 0, "phải thoát 0; stderr: {loi}");
    assert_eq!(loi, "", "JSON không được in stderr");
    assert_eq!(ra, ky_vong_json(), "sai nội dung/thứ tự khoá JSON");
    assert_eq!(ra.matches('\n').count(), 1, "đúng một dòng: {ra:?}");
    assert!(ra.ends_with('\n'), "JSON phải kết thúc bằng newline");

    let v = vis_interp::json::decode(ra.trim_end_matches('\n')).expect("JSON phải parse được");
    let vis_interp::value::Value::Map(kv) = v else {
        panic!("JSON phải là object");
    };
    assert_eq!(kv.len(), 5, "đúng 5 khoá: {kv:?}");
    let khoa: Vec<&str> = kv
        .iter()
        .map(|(k, _)| match k {
            vis_interp::value::Value::Str(s) => s.as_str(),
            _ => panic!("khoá phải là chuỗi"),
        })
        .collect();
    assert_eq!(
        khoa,
        ["version", "os", "arch", "sqlite", "features"],
        "thứ tự khoá cố định"
    );
    let tim = |ten: &str| {
        kv.iter()
            .find(|(k, _)| matches!(k, vis_interp::value::Value::Str(s) if s == ten))
            .map(|(_, v)| v)
            .unwrap_or_else(|| panic!("thiếu khoá {ten}"))
    };
    let vis_interp::value::Value::Str(version) = tim("version") else {
        panic!("version phải là chuỗi");
    };
    assert_eq!(version.as_str(), env!("CARGO_PKG_VERSION"));
    let vis_interp::value::Value::Str(os) = tim("os") else {
        panic!("os phải là chuỗi");
    };
    assert_eq!(os.as_str(), std::env::consts::OS);
    let vis_interp::value::Value::Str(arch) = tim("arch") else {
        panic!("arch phải là chuỗi");
    };
    assert_eq!(arch.as_str(), std::env::consts::ARCH);
    let vis_interp::value::Value::Bool(sqlite) = tim("sqlite") else {
        panic!("sqlite phải là bool");
    };
    assert_eq!(*sqlite, sqlite_bat());
    let vis_interp::value::Value::List(f) = tim("features") else {
        panic!("features phải là mảng");
    };
    let f: Vec<String> = f
        .iter()
        .map(|v| match v {
            vis_interp::value::Value::Str(s) => s.clone(),
            _ => panic!("features phải chứa chuỗi"),
        })
        .collect();
    let mong: Vec<String> = features().iter().map(|s| (*s).to_string()).collect();
    assert_eq!(f, mong);
}

/// (3) `--json` kèm đối số khác ⇒ exit 2, stdout rỗng, usage `vis doctor` trên stderr.
#[test]
fn json_them_doi_so_la_loi_dung() {
    for args in [
        &["doctor", "--json", "x"][..],
        &["doctor", "--json", "--json"][..],
        &["doctor", "--json", "--la"][..],
        &["doctor", "x", "--json"][..],
    ] {
        let (ma, ra, loi) = vis(args);
        assert_eq!(ma, 2, "args {args:?} phải exit 2");
        assert_eq!(ra, "", "args {args:?} không được in stdout");
        assert!(
            loi.contains("dùng: vis doctor"),
            "args {args:?} phải in usage: {loi}"
        );
    }
}

/// (4) Cờ/đối số lạ ⇒ exit 2, stdout rỗng, usage trên stderr.
#[test]
fn co_la_va_doi_so_la_loi_dung_stdout_rong() {
    for args in [
        &["doctor", "--wat"][..],
        &["doctor", "--json=true"][..],
        &["doctor", "-x"][..],
        &["doctor", "x"][..],
        &["doctor", "x", "y"][..],
    ] {
        let (ma, ra, loi) = vis(args);
        assert_eq!(ma, 2, "args {args:?} phải exit 2");
        assert_eq!(ra, "", "args {args:?} không được in stdout");
        assert!(
            loi.contains("dùng: vis doctor"),
            "args {args:?} phải in usage: {loi}"
        );
    }
}

/// (5) Không phụ thuộc `cwd`/môi trường: hai thư mục khác nhau ⇒ output y hệt.
#[test]
fn khong_phu_thuoc_cwd() {
    let a = tam("a");
    let b = tam("b");
    let chay = |d: &Path| {
        let o = Command::new(env!("CARGO_BIN_EXE_vis"))
            .args(["doctor", "--json"])
            .current_dir(d)
            .output()
            .expect("chạy được `vis doctor`");
        (
            o.status.code().unwrap_or(-1),
            String::from_utf8(o.stdout).expect("stdout UTF-8"),
        )
    };
    let (ma_a, ra_a) = chay(&a);
    let (ma_b, ra_b) = chay(&b);
    assert_eq!((ma_a, &ra_a), (0, &ky_vong_json()));
    assert_eq!((ma_b, &ra_b), (0, &ky_vong_json()));
    let _ = std::fs::remove_dir_all(&a);
    let _ = std::fs::remove_dir_all(&b);
}

/// (6) Usage gốc của `vis` nêu lệnh mới (và vẫn là lỗi dùng: exit 2, stdout rỗng).
#[test]
fn usage_goc_neu_doctor() {
    let (ma, ra, loi) = vis(&[]);
    assert_eq!(ma, 2);
    assert_eq!(ra, "");
    assert!(
        loi.contains("vis doctor [--json]"),
        "usage gốc phải nêu `vis doctor [--json]`: {loi}"
    );
}

/// **Bằng chứng đỏ-trước**: trước `5dead58` lời gọi này rơi vào nhánh lệnh lạ (`_ => usage()`)
/// ⇒ usage gốc + exit 2. Nay nó phải là lệnh thật: exit 0, stderr rỗng, stdout là 5 dòng
/// `doctor` (không phải usage).
#[test]
fn khong_con_roi_vao_nhanh_lenh_la() {
    let (ma, ra, loi) = vis(&["doctor"]);
    assert_eq!(ma, 0, "đỏ-trước: exit 2 vì lệnh lạ");
    assert_eq!(loi, "", "đỏ-trước: usage gốc trên stderr");
    assert!(
        !loi.contains("dùng: vis explain"),
        "không còn rơi vào usage gốc"
    );
    assert_eq!(ra, ky_vong_thuong());
    // Lệnh lạ thật vẫn giữ hành vi cũ (exit 2, stdout rỗng) — sửa không nới nhánh `_`.
    let (ma, ra, loi) = vis(&["khong-co-lenh-nay"]);
    assert_eq!(ma, 2);
    assert_eq!(ra, "");
    assert!(loi.contains("dùng: vis explain"), "usage gốc: {loi}");
}
