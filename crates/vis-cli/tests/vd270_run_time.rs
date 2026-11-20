// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270** — `vis run --time`: một dòng **đồng hồ tường** của **lời gọi runtime** ra stderr.
//!
//! Đỏ trước (main `e8e5332`):
//! ```text
//! $ vis run --time ok.vis
//! vis run: không hỗ trợ cờ `--time`
//! $ echo $?
//! 2
//! ```
//!
//! Hợp đồng được kiểm:
//! - định dạng **đúng** một dòng `# time: <số nguyên> ms` (hoặc `<số nguyên> us` khi < 1 ms),
//!   luôn ra **stderr**, không bao giờ lẫn stdout;
//! - đo từ ngay **trước** lời gọi interp/VM (sau nạp–parse–tyck–codegen–cache) tới ngay **sau**
//!   khi runtime trả về; không tính in stdout/TAP/chẩn đoán hay dọn phiên DB;
//! - chỉ in khi runtime **đã chạy**: lỗi đọc/parse/tyck (kể cả `--vm` codegen) và `--dry`
//!   ⇒ **không** có dòng thời gian; trap/lỗi **lúc chạy** ⇒ vẫn có, sau chẩn đoán;
//! - `--vm`, `--stdin`, `--stdin --vm` cho cùng hợp đồng; stdout/mã thoát **không đổi** so với
//!   lần chạy không `--time`;
//! - `--tap`: dòng thời gian là **chú thích TAP** (`#`), nằm **sau** `ok|not ok`, vẫn TAP hợp lệ;
//!   lỗi trước khi chạy ⇒ TAP **không** đổi (không có dòng thời gian);
//! - `--time` chỉ là cờ **trước** `-- args…`; `--time` sau `--` vào `main` như đối số thường;
//! - cờ lạ vẫn bị từ chối y như trước (bằng chứng đỏ-trước không bị nới).

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

type Kq = (i32, String, String);

fn vis() -> &'static str {
    env!("CARGO_BIN_EXE_vis")
}

/// Thư mục tạm **riêng** cho từng ca (không đụng ca chạy song song).
fn tam(tag: &str) -> PathBuf {
    static DEM: AtomicUsize = AtomicUsize::new(0);
    let d = std::env::temp_dir().join(format!(
        "vis-vd270-time-{tag}-{}-{}",
        std::process::id(),
        DEM.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("thư mục tạm");
    d
}

fn ghi(d: &Path, ten: &str, src: &str) -> PathBuf {
    let p = d.join(ten);
    std::fs::write(&p, src).expect("ghi tệp tạm");
    p
}

fn ket_qua(o: std::process::Output) -> Kq {
    (
        o.status.code().unwrap_or(-1),
        String::from_utf8(o.stdout).expect("stdout phải là UTF-8"),
        String::from_utf8_lossy(&o.stderr).into_owned(),
    )
}

/// Chạy `vis run <co…> <tệp>`; `co` đặt **trước** đường dẫn tệp.
fn chay_tep(co: &[&str], tep: &Path) -> Kq {
    let o = Command::new(vis())
        .arg("run")
        .args(co)
        .arg(tep)
        .output()
        .expect("chạy vis run");
    ket_qua(o)
}

/// Chạy `vis run <args…>` với stdin chương trình. `vis` có thể từ chối **trước** khi đọc stdin
/// (vd cờ lạ) ⇒ `BrokenPipe` là hợp lệ.
fn chay_stdin(args: &[&str], src: &str) -> Kq {
    let mut con = Command::new(vis())
        .arg("run")
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("chạy vis run --stdin");
    let mut si = con.stdin.take().expect("stdin đã pipe");
    match si.write_all(src.as_bytes()) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => {}
        Err(e) => panic!("ghi stdin: {e}"),
    }
    // Đóng đầu ghi để `vis run --stdin` thấy EOF (nếu không, hai bên chờ nhau).
    drop(si);
    ket_qua(con.wait_with_output().expect("chờ vis run"))
}

/// Định dạng **chính xác**: `^# time: [0-9]+ (ms|us)$` (một dòng, không ký tự thừa).
fn khop_dinh_dang(dong: &str) -> bool {
    let Some(rest) = dong.strip_prefix("# time: ") else {
        return false;
    };
    let Some((so, don_vi)) = rest.split_once(' ') else {
        return false;
    };
    if so.is_empty() || (so.len() > 1 && so.starts_with('0')) {
        return false;
    }
    so.bytes().all(|b| b.is_ascii_digit()) && matches!(don_vi, "ms" | "us")
}

/// Số micro-giây đọc từ **một** dòng thời gian hợp lệ (để so sánh ngưỡng khi cần).
fn doc_us(dong: &str) -> u128 {
    assert!(
        khop_dinh_dang(dong),
        "dòng thời gian sai định dạng: {dong:?}"
    );
    let rest = dong.strip_prefix("# time: ").unwrap();
    let (so, don_vi) = rest.split_once(' ').unwrap();
    let n: u128 = so.parse().expect("số nguyên");
    match don_vi {
        "ms" => n * 1000,
        "us" => n,
        _ => unreachable!(),
    }
}

/// Trả dòng thời gian **duy nhất** (nếu có); panic nếu có nhiều hơn một.
fn dong_thoi_gian(err: &str) -> Option<&str> {
    let mut tim = None;
    for dong in err.lines() {
        if dong.starts_with("# time:") {
            assert!(
                khop_dinh_dang(dong),
                "dòng thời gian sai định dạng: {dong:?}; stderr đầy đủ: {err:?}"
            );
            assert!(
                tim.is_none(),
                "chỉ được **một** dòng thời gian; stderr: {err:?}"
            );
            tim = Some(dong);
        }
    }
    tim
}

/// Dòng cuối của stderr **phải** là dòng thời gian; trả số µs.
fn dong_thoi_gian_cuoi(err: &str) -> u128 {
    let cuoi = err
        .strip_suffix('\n')
        .and_then(|s| s.rsplit('\n').next())
        .expect("stderr phải kết thúc bằng dòng thời gian");
    assert!(khop_dinh_dang(cuoi), "dòng cuối sai định dạng: {cuoi:?}");
    assert_eq!(
        err.lines().filter(|d| d.starts_with("# time:")).count(),
        1,
        "chỉ được một dòng thời gian; stderr: {err:?}"
    );
    doc_us(cuoi)
}

/// (1) Chương trình sạch: stdout nguyên vẹn, stderr **đúng một** dòng thời gian, exit 0.
#[test]
fn sach_in_stdout_va_mot_dong_thoi_gian() {
    let d = tam("sach");
    let f = ghi(&d, "ok.vis", "fn main():\n    print(\"xin chao\")\n");
    for co in [&["--time"][..], &["--time", "--vm"][..]] {
        let (ma, out, err) = chay_tep(co, &f);
        assert_eq!(ma, 0, "co={co:?}; stderr: {err}");
        assert_eq!(out, "xin chao\n", "co={co:?}");
        assert_eq!(err.lines().count(), 1, "co={co:?}; stderr: {err:?}");
        assert!(
            khop_dinh_dang(err.trim_end_matches('\n')),
            "co={co:?}: {err:?}"
        );
    }
    let _ = std::fs::remove_dir_all(&d);
}

/// (2) Không có `--time` ⇒ stderr **rỗng** (không hồi quy đường cũ).
#[test]
fn khong_co_time_thi_stderr_rong() {
    let d = tam("khong-co");
    let f = ghi(&d, "ok.vis", "fn main():\n    print(1)\n");
    for co in [&[][..], &["--vm"][..]] {
        let (ma, out, err) = chay_tep(co, &f);
        assert_eq!(ma, 0, "co={co:?}; stderr: {err}");
        assert_eq!(out, "1\n", "co={co:?}");
        assert_eq!(err, "", "co={co:?}; không được có dòng thời gian");
    }
    let _ = std::fs::remove_dir_all(&d);
}

/// (3) `--time` **không** đổi stdout/mã thoát so với lần chạy không cờ; khác đúng **một**
/// dòng thời gian trên stderr (cả interp và VM).
#[test]
fn time_khong_doi_stdout_va_ma_thoat() {
    let d = tam("khong-doi");
    let f = ghi(
        &d,
        "ok.vis",
        "fn main():\n    print(\"a\")\n    print(2 + 3)\n",
    );
    for co in [&[][..], &["--vm"][..]] {
        let khong = chay_tep(co, &f);
        let co_time = chay_tep(&[co, &["--time"][..]].concat(), &f);
        assert_eq!(khong.0, co_time.0, "mã thoát lệch; co={co:?}");
        assert_eq!(khong.1, co_time.1, "stdout lệch; co={co:?}");
        assert_eq!(khong.2, "", "không cờ thì stderr rỗng; co={co:?}");
        assert_eq!(
            co_time.2.lines().count(),
            1,
            "có cờ thì đúng một dòng; co={co:?}; stderr: {:?}",
            co_time.2
        );
        let _ = dong_thoi_gian(&co_time.2).expect("phải có dòng thời gian");
    }
    let _ = std::fs::remove_dir_all(&d);
}

/// (4) Lỗi **trước khi chạy** (kiểu tĩnh) ⇒ exit 2, **không** có dòng thời gian — cả tệp,
/// `--vm`, `--stdin`, `--tap`, `--dry`.
#[test]
fn loi_truoc_khi_chay_khong_co_dong_thoi_gian() {
    let d = tam("loi-kieu");
    let f = ghi(&d, "type.vis", "let x: int = \"s\"\n");
    for co in [
        &["--time"][..],
        &["--time", "--vm"][..],
        &["--time", "--tap"][..],
    ] {
        let (ma, out, err) = chay_tep(co, &f);
        assert_eq!(ma, 2, "co={co:?}; stderr: {err}");
        assert_eq!(out, "", "co={co:?}");
        assert!(
            dong_thoi_gian(&err).is_none(),
            "lỗi tyck chưa chạy ⇒ không có dòng thời gian; co={co:?}; stderr: {err:?}"
        );
        assert!(
            err.contains("VS0412"),
            "vẫn phải in chẩn đoán; co={co:?}; stderr: {err}"
        );
    }
    // `--stdin` giữ cùng quyết định.
    let (ma, out, err) = chay_stdin(&["--time", "--stdin"], "let x: int = \"s\"\n");
    assert_eq!(ma, 2, "stderr: {err}");
    assert_eq!(out, "");
    assert!(dong_thoi_gian(&err).is_none(), "stderr: {err:?}");
    // `--dry` chỉ đọc CST, không chạy ⇒ không có dòng thời gian.
    let (ma, out, err) = chay_tep(&["--time", "--dry"], &f);
    assert_eq!(ma, 0, "stderr: {err}");
    assert_eq!(out, "");
    assert!(dong_thoi_gian(&err).is_none(), "stderr: {err:?}");
    let _ = std::fs::remove_dir_all(&d);
}

/// (5) Trap **lúc chạy** ⇒ exit 101, stdout trước trap giữ nguyên, dòng thời gian **có** và
/// nằm **cuối** (sau chẩn đoán); `--vm` cùng hợp đồng.
#[test]
fn trap_luc_chay_van_co_dong_thoi_gian() {
    let d = tam("panic");
    let f = ghi(
        &d,
        "panic.vis",
        "fn main():\n    print(\"truoc\")\n    panic(\"hỏng\")\n    print(\"sau\")\n",
    );
    for co in [&["--time"][..], &["--time", "--vm"][..]] {
        let (ma, out, err) = chay_tep(co, &f);
        assert_eq!(ma, 101, "trap ⇒ 101; co={co:?}; stderr: {err}");
        assert_eq!(out, "truoc\n", "co={co:?}");
        assert!(err.contains("VS0501") && err.contains("hỏng"), "{err}");
        let us = dong_thoi_gian_cuoi(&err);
        assert!(us < 60_000_000, "thời gian vô lý: {us} us");
    }
    let _ = std::fs::remove_dir_all(&d);
}

/// (6) `main -> Result::Err` ⇒ exit 1, không in lỗi, **vẫn** có dòng thời gian (đã chạy).
#[test]
fn result_err_exit_1_van_co_dong_thoi_gian() {
    let d = tam("result-err");
    let f = ghi(
        &d,
        "err.vis",
        "fn main() -> Result[int, str]:\n    Err(\"that bai\")\n",
    );
    for co in [&["--time"][..], &["--time", "--vm"][..]] {
        let (ma, out, err) = chay_tep(co, &f);
        assert_eq!(ma, 1, "co={co:?}; stderr: {err}");
        assert_eq!(out, "", "co={co:?}");
        assert_eq!(err.lines().count(), 1, "co={co:?}; stderr: {err:?}");
        dong_thoi_gian_cuoi(&err);
    }
    let _ = std::fs::remove_dir_all(&d);
}

/// (7) `--tap`: TAP ở stderr vẫn hợp lệ; dòng thời gian là **chú thích TAP** và nằm **sau**
/// `ok 1 - …`; stdout chương trình không bị trộn.
#[test]
fn tap_giu_hop_le_va_time_sau_dong_ok() {
    let d = tam("tap");
    let f = ghi(&d, "ok.vis", "fn main():\n    print(\"tap\")\n");
    let (ma, out, err) = chay_tep(&["--time", "--tap"], &f);
    assert_eq!(ma, 0, "stderr: {err}");
    assert_eq!(out, "tap\n", "TAP không được lẫn vào stdout");
    let dong: Vec<&str> = err.lines().collect();
    assert_eq!(
        dong.len(),
        4,
        "TAP + time phải đúng 4 dòng; stderr: {err:?}"
    );
    assert_eq!(dong[0], "TAP version 13");
    assert_eq!(dong[1], "1..1");
    assert!(
        dong[2].starts_with("ok 1 - ") && dong[2].contains("(steps="),
        "dòng kết quả TAP: {:?}",
        dong[2]
    );
    assert!(
        khop_dinh_dang(dong[3]),
        "dòng cuối phải là chú thích `# time`: {:?}",
        dong[3]
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// (8) `--tap` + trap: chú thích thời gian nằm **sau** `not ok`; vẫn đúng 1 dòng, exit 101.
#[test]
fn tap_trap_time_sau_dong_not_ok() {
    let d = tam("tap-panic");
    let f = ghi(&d, "panic.vis", "fn main():\n    panic(\"hỏng\")\n");
    let (ma, out, err) = chay_tep(&["--time", "--tap"], &f);
    assert_eq!(ma, 101, "stderr: {err}");
    assert_eq!(out, "");
    let dong: Vec<&str> = err.lines().collect();
    let cuoi = dong.last().copied().expect("stderr không rỗng");
    assert!(khop_dinh_dang(cuoi), "dòng cuối: {cuoi:?}");
    assert!(
        dong.iter().any(|d| d.starts_with("not ok 1 - ")),
        "phải có TAP `not ok`; stderr: {err:?}"
    );
    let vt_not_ok = dong
        .iter()
        .position(|d| d.starts_with("not ok 1 - "))
        .unwrap();
    assert_eq!(
        vt_not_ok,
        dong.len() - 2,
        "`not ok` phải ngay trước dòng thời gian; stderr: {err:?}"
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// (9) `--stdin` và `--stdin --vm`: chạy được, stdout đúng, đúng một dòng thời gian, exit 0;
/// `-- args…` sau `--` vẫn vào `main`.
#[test]
fn stdin_va_vm_cung_hop_dong() {
    let src = "fn main(args: list[str]):\n    print(args)\n";
    for co in [
        &["--time", "--stdin"][..],
        &["--time", "--stdin", "--vm"][..],
    ] {
        let (ma, out, err) = chay_stdin(co, src);
        assert_eq!(ma, 0, "co={co:?}; stderr: {err}");
        assert_eq!(out, "[]\n", "co={co:?}");
        assert_eq!(err.lines().count(), 1, "co={co:?}; stderr: {err:?}");
        dong_thoi_gian_cuoi(&err);
    }
    let (ma, out, err) = chay_stdin(&["--time", "--stdin", "--", "--time", "x"], src);
    assert_eq!(ma, 0, "stderr: {err}");
    assert_eq!(out, "[--time, x]\n", "đối số sau `--` phải vào main");
    assert_eq!(
        err.lines().filter(|d| d.starts_with("# time:")).count(),
        1,
        "chỉ cờ trước `--` mới được hiểu; stderr: {err:?}"
    );
}

/// (10) Ranh giới `--`: `--time` **trước** `--` là cờ của `vis`; `--time` **sau** `--` là đối
/// số chương trình. Cờ đặt sau đường dẫn tệp nhưng trước `--` vẫn có hiệu lực.
#[test]
fn time_chi_co_hieu_luc_truoc_dau_gach() {
    let d = tam("args");
    let f = ghi(
        &d,
        "args.vis",
        "fn main(args: list[str]):\n    print(args)\n",
    );

    // Cờ sau đường dẫn tệp, trước `--`.
    let o = Command::new(vis())
        .arg("run")
        .arg(&f)
        .arg("--time")
        .args(["--", "a", "b"])
        .output()
        .expect("vis run <tệp> --time -- a b");
    let (ma, out, err) = ket_qua(o);
    assert_eq!(ma, 0, "stderr: {err}");
    assert_eq!(out, "[a, b]\n");
    assert_eq!(err.lines().count(), 1, "stderr: {err:?}");
    dong_thoi_gian_cuoi(&err);

    // `--time` sau `--` là đối số thường, **không** bật đo và **không** bị từ chối.
    let o = Command::new(vis())
        .arg("run")
        .arg(&f)
        .arg("--")
        .args(["--time", "--vm"])
        .output()
        .expect("vis run <tệp> -- --time --vm");
    let (ma, out, err) = ket_qua(o);
    assert_eq!(ma, 0, "stderr: {err}");
    assert_eq!(out, "[--time, --vm]\n");
    assert_eq!(err, "", "không bật `--time`; stderr: {err:?}");
    let _ = std::fs::remove_dir_all(&d);
}

/// (11) Bằng chứng **đỏ trước**: cờ lạ vẫn bị từ chối đúng thông điệp cũ; `--time` không còn
/// rơi vào nhánh đó.
#[test]
fn co_la_van_bi_tu_choi_nhu_head() {
    let (ma, out, err) = chay_stdin(&["--bogus"], "print(1)\n");
    assert_eq!(ma, 2);
    assert_eq!(out, "");
    assert_eq!(err, "vis run: không hỗ trợ cờ `--bogus`\n");

    let (ma, out, err) = chay_stdin(&["--time", "--stdin"], "print(1)\n");
    assert_eq!(ma, 0, "stderr: {err}");
    assert_eq!(out, "1\n");
    assert!(
        !err.contains("không hỗ trợ cờ"),
        "`--time` phải được nhận: {err:?}"
    );
}

/// (12) Đo **đúng** khoảng chạy: chương trình tự đo bằng `time.monotonic_ms()` (mốc trong
/// lời gọi runtime) không thể **lớn hơn** thời gian CLI báo (mốc bao ngoài lời gọi đó) quá
/// sai số đồng hồ; ngược lại CLI phải báo **>=** thời gian tự đo (sai số 1 ms do làm tròn).
#[test]
fn thoi_gian_cli_bao_phu_thoi_gian_chuong_trinh_tu_do() {
    let d = tam("do");
    let f = ghi(
        &d,
        "do.vis",
        "use std.time\nfn main():\n    let t0 = time.monotonic_ms()\n    var s = 0\n    for i in 0..200000:\n        s += i % 7\n    print(time.monotonic_ms() - t0)\n",
    );
    let (ma, out, err) = chay_tep(&["--time"], &f);
    assert_eq!(ma, 0, "stderr: {err}");
    if let Ok(tu_do) = out.trim().parse::<u128>() {
        let cli = dong_thoi_gian_cuoi(&err);
        // Làm tròn xuống ms: sai số tối đa 1 ms (1000 µs).
        assert!(
            cli + 1000 >= tu_do * 1000,
            "CLI báo {cli} us nhưng chương trình tự đo {tu_do} ms"
        );
    }
    let _ = std::fs::remove_dir_all(&d);
}
