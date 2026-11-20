// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270** — `vis fmt --check --stat`: một dòng tổng hợp cho CI.
//!
//! Đỏ trước (HEAD `1b51c62`, trước task này): `--stat` **chưa** phải cờ của `vis fmt`:
//! - `vis fmt --check --stat a.vis` ⇒ `vis fmt: không hỗ trợ cờ `--stat`` + usage, exit 2,
//!   stdout rỗng (không có dòng `fmt-check: …`);
//! - `vis fmt --check a.vis --stat` coi `--stat` là **tệp** ⇒ stderr `# --stat: không đọc
//!   được…` và stdout vẫn in đường dẫn tệp lệch, exit 2.
//!
//! Vì vậy mọi ca dưới đây đỏ trước.
//!
//! Hợp đồng: đúng **một** dòng stdout `fmt-check: <N> tệp, <M> cần sửa, <K> lỗi`; không in gì
//! khác ra stdout; lỗi đọc/cú pháp chi tiết ra stderr. Exit 0 (M=K=0) / 1 (M>0, K=0) / 2 (K>0
//! hoặc dùng sai). `--stat` chỉ đi với `--check`, không đi với `--json`/`--list`/`--diff`/
//! `--write`/`--indent-report`. `--stdin --check --stat`: N=1, stdin rỗng vẫn tính 1 tệp 0 cần
//! sửa; lỗi cú pháp ⇒ 1 lỗi, exit 2.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn tam(tag: &str) -> PathBuf {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static DEM: AtomicUsize = AtomicUsize::new(0);
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-fmt-stat-{tag}-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("tạo thư mục tạm");
    d
}

fn ghi(d: &Path, ten: &str, src: &str) -> PathBuf {
    let p = d.join(ten);
    if let Some(cha) = p.parent() {
        std::fs::create_dir_all(cha).expect("tạo thư mục cha");
    }
    std::fs::write(&p, src).expect("ghi tệp");
    p
}

fn s(p: &Path) -> String {
    p.display().to_string()
}

fn vis(args: &[&str]) -> (i32, String, String) {
    let o = Command::new(env!("CARGO_BIN_EXE_vis"))
        .args(args)
        .output()
        .expect("chạy `vis`");
    (
        o.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&o.stdout).into_owned(),
        String::from_utf8_lossy(&o.stderr).into_owned(),
    )
}

fn vis_stdin(args: &[&str], stdin: &[u8]) -> (i32, String, String) {
    let mut con = Command::new(env!("CARGO_BIN_EXE_vis"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("chạy `vis`");
    {
        let mut vao = con.stdin.take().expect("stdin pipe");
        if let Err(e) = vao.write_all(stdin) {
            if e.kind() != std::io::ErrorKind::BrokenPipe {
                panic!("ghi stdin: {e}");
            }
        }
    }
    let o = con.wait_with_output().expect("chờ `vis`");
    (
        o.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&o.stdout).into_owned(),
        String::from_utf8_lossy(&o.stderr).into_owned(),
    )
}

const TOT: &str = "fn f():\n    1\n";
const SAI: &str = "fn f():\n  1\n";

/// Hỗn hợp tệp: N = số tệp đã kiểm, M = số tệp cần sửa, K = 0; exit 1.
/// Chỉ **một** dòng stdout, không in đường dẫn; không ghi đĩa. Nhận cả thứ tự `--stat --check`.
#[test]
fn hon_hop_tep_mot_dong_exit_1() {
    let d = tam("hon-hop");
    let tot = ghi(&d, "a_tot.vis", TOT);
    let sai1 = ghi(&d, "b_sai.vis", SAI);
    let sai2 = ghi(&d, "c_sai.vis", "fn g():\n  let x = 1\n  x\n");
    let (ma, out, err) = vis(&["fmt", "--check", "--stat", &s(&tot), &s(&sai1), &s(&sai2)]);
    assert_eq!(ma, 1, "exit 1; stderr: {err}");
    assert_eq!(out, "fmt-check: 3 tệp, 2 cần sửa, 0 lỗi\n");
    assert_eq!(out.lines().count(), 1, "đúng một dòng stdout: {out:?}");
    assert_eq!(err, "", "không cảnh báo/usage");
    assert_eq!(
        std::fs::read_to_string(&sai1).unwrap(),
        SAI,
        "không ghi đĩa"
    );
    assert_eq!(
        std::fs::read_to_string(&sai2).unwrap(),
        "fn g():\n  let x = 1\n  x\n",
        "không ghi đĩa"
    );

    // `--stat` đứng trước `--check` cũng nhận, cùng kết quả.
    let (ma2, out2, err2) = vis(&["fmt", "--stat", "--check", &s(&tot), &s(&sai1), &s(&sai2)]);
    assert_eq!(ma2, 1, "stderr: {err2}");
    assert_eq!(out2, out);
    let _ = std::fs::remove_dir_all(&d);
}

/// Thư mục sạch: đếm đúng tệp `.vis`/`.vish` (bỏ đuôi lạ), M=K=0, exit 0, stderr rỗng.
#[test]
fn thu_muc_sach_exit_0() {
    let d = tam("sach");
    ghi(&d, "a_tot.vis", TOT);
    ghi(&d, "b_tot.vish", "cmd f() -> int:\n    1\n");
    ghi(&d, "rong.vis", "\n");
    ghi(&d, "bo_qua.txt", SAI);
    let (ma, out, err) = vis(&["fmt", "--check", "--stat", &s(&d)]);
    assert_eq!(ma, 0, "stderr: {err}");
    assert_eq!(out, "fmt-check: 3 tệp, 0 cần sửa, 0 lỗi\n");
    assert_eq!(err, "", "không cảnh báo, không usage");
    let _ = std::fs::remove_dir_all(&d);
}

/// Lỗi cú pháp: vẫn đếm tệp lỗi vào N và K, tệp lệch còn lại vào M; exit 2 thắng 1.
/// Chi tiết chẩn đoán ra stderr (kèm tên tệp), stdout vẫn đúng **một** dòng tổng hợp.
#[test]
fn loi_cu_phap_exit_2_stderr_kem_ten_tep() {
    let d = tam("loi-cu-phap");
    let tot = ghi(&d, "a_tot.vis", TOT);
    let sai = ghi(&d, "b_sai.vis", SAI);
    let loi = ghi(&d, "c_loi.vis", "let x =\n");
    let (ma, out, err) = vis(&["fmt", "--check", "--stat", &s(&tot), &s(&sai), &s(&loi)]);
    assert_eq!(ma, 2, "exit 2; stdout: {out}");
    assert_eq!(out, "fmt-check: 3 tệp, 1 cần sửa, 1 lỗi\n");
    assert_eq!(
        err,
        format!(
            "# {}: VS0201 1:8 mong biểu thức, thật `\\n` (NEWLINE)\n",
            s(&loi)
        )
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// Tệp không đọc được: K=1, exit 2; stderr nêu đúng đường dẫn; không panic.
#[test]
fn tep_khong_doc_duoc_exit_2() {
    let d = tam("khong-doc");
    let tot = ghi(&d, "a_tot.vis", TOT);
    let khong_co = d.join("khong_ton_tai.vis");
    let (ma, out, err) = vis(&["fmt", "--check", "--stat", &s(&tot), &s(&khong_co)]);
    assert_eq!(ma, 2, "stderr: {err}");
    assert_eq!(out, "fmt-check: 2 tệp, 0 cần sửa, 1 lỗi\n");
    assert_eq!(
        err,
        format!(
            "# {}: không đọc được: No such file or directory (os error 2)\n",
            s(&khong_co)
        )
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// `--stdin --check --stat`: 0 đã đúng / 1 cần sửa / 2 lỗi cú pháp; N luôn = 1.
/// Stdin rỗng vẫn tính 1 tệp, 0 cần sửa. `--mode command` có hiệu lực.
#[test]
fn stdin_check_stat_0_1_2_chinh_xac() {
    let (ma, out, err) = vis_stdin(&["fmt", "--stdin", "--check", "--stat"], TOT.as_bytes());
    assert_eq!(ma, 0, "stderr: {err}");
    assert_eq!(out, "fmt-check: 1 tệp, 0 cần sửa, 0 lỗi\n");
    assert_eq!(err, "");

    let (ma, out, err) = vis_stdin(&["fmt", "--stdin", "--check", "--stat"], SAI.as_bytes());
    assert_eq!(ma, 1, "stderr: {err}");
    assert_eq!(out, "fmt-check: 1 tệp, 1 cần sửa, 0 lỗi\n");
    assert_eq!(err, "");

    let (ma, out, err) = vis_stdin(&["fmt", "--stat", "--check", "--stdin"], SAI.as_bytes());
    assert_eq!(ma, 1, "thứ tự cờ ngược vẫn nhận; stderr: {err}");
    assert_eq!(out, "fmt-check: 1 tệp, 1 cần sửa, 0 lỗi\n");

    // Đầu vào rỗng: 1 tệp, 0 cần sửa.
    let (ma, out, err) = vis_stdin(&["fmt", "--stdin", "--check", "--stat"], b"");
    assert_eq!(ma, 0, "stderr: {err}");
    assert_eq!(out, "fmt-check: 1 tệp, 0 cần sửa, 0 lỗi\n");
    assert_eq!(err, "");

    // Lỗi cú pháp: 1 lỗi, 0 cần sửa, exit 2, chi tiết ra stderr.
    let (ma, out, err) = vis_stdin(
        &["fmt", "--stdin", "--check", "--stat"],
        "let x =\n".as_bytes(),
    );
    assert_eq!(ma, 2, "exit 2; stdout: {out}");
    assert_eq!(out, "fmt-check: 1 tệp, 0 cần sửa, 1 lỗi\n");
    assert_eq!(err, "# VS0201 1:8 mong biểu thức, thật `\\n` (NEWLINE)\n");

    // `--mode command`: 2 dấu cách đã đúng; mặc định script đòi 4.
    let vish = b"fn f():\n  let x = cmd { echo hi }\n  x\n";
    let (ma, out, err) = vis_stdin(
        &["fmt", "--stdin", "--check", "--stat", "--mode", "command"],
        vish,
    );
    assert_eq!(ma, 0, "stderr: {err}");
    assert_eq!(out, "fmt-check: 1 tệp, 0 cần sửa, 0 lỗi\n");
    let (ma, out, err) = vis_stdin(&["fmt", "--stdin", "--check", "--stat"], vish);
    assert_eq!(ma, 1, "stderr: {err}");
    assert_eq!(out, "fmt-check: 1 tệp, 1 cần sửa, 0 lỗi\n");
}

/// Stdin không phải UTF-8: 1 tệp, 0 cần sửa, 1 lỗi; exit 2; lỗi đọc ra stderr.
#[test]
fn stdin_khong_utf8_exit_2() {
    let (ma, out, err) = vis_stdin(&["fmt", "--stdin", "--check", "--stat"], &[0xff, 0xfe]);
    assert_eq!(ma, 2, "stderr: {err}");
    assert_eq!(out, "fmt-check: 1 tệp, 0 cần sửa, 1 lỗi\n");
    assert_eq!(
        err,
        "vis fmt: không đọc được stdin (cần UTF-8): stream did not contain valid UTF-8\n"
    );
}

/// `--stat` chỉ đi với `--check`; không đi với `--json`/`--list`/`--diff`/`--write`/
/// `--indent-report`; đứng sau tệp cũng bị từ chối. Mọi ca: usage + exit 2, stdout rỗng,
/// không ghi tệp.
#[test]
fn to_hop_sai_exit_2_usage() {
    let d = tam("sai-to-hop");
    let p = ghi(&d, "a.vis", TOT);
    let f = s(&p);
    let cases: &[&[&str]] = &[
        &["fmt", "--stat"],
        &["fmt", "--stat", "--check"],
        &["fmt", "--stat", &f],
        &["fmt", "--stat", "--write", &f],
        &["fmt", "--write", "--stat", &f],
        &["fmt", "--check", "--stat", "--json", &f],
        &["fmt", "--check", "--stat", "--list", &f],
        &["fmt", "--check", "--stat", "--diff", &f],
        &["fmt", "--check", "--stat", "--indent-report", &f],
        &["fmt", "--indent-report", "--stat", &f],
        &["fmt", &f, "--check", "--stat"],
        &["fmt", "--stat", "--json", &f],
    ];
    for args in cases {
        let (ma, out, err) = vis(args);
        assert_eq!(ma, 2, "args {args:?} phải exit 2; stderr: {err}");
        assert_eq!(out, "", "usage không in stdout (args {args:?})");
        assert!(
            err.contains("dùng: vis fmt"),
            "phải in usage (args {args:?}): {err}"
        );
        assert!(
            err.contains("--stat"),
            "usage phải nêu `--stat` (args {args:?}): {err}"
        );
    }
    assert_eq!(
        std::fs::read_to_string(&p).expect("đọc lại"),
        TOT,
        "không tổ hợp sai nào được ghi tệp"
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// Tổ hợp cấm với `--stdin` ⇒ usage + exit 2 **trước khi** đọc stdin, stdout rỗng.
#[test]
fn stdin_to_hop_sai_exit_2_usage() {
    let cases: &[&[&str]] = &[
        &["fmt", "--stdin", "--stat"],
        &["fmt", "--stat", "--stdin"],
        &["fmt", "--stdin", "--check", "--stat", "--json"],
        &["fmt", "--stdin", "--check", "--stat", "--list"],
        &["fmt", "--stdin", "--check", "--stat", "--diff"],
        &["fmt", "--stdin", "--check", "--stat", "--write"],
        &["fmt", "--stdin", "--check", "--stat", "--indent-report"],
        &["fmt", "--stdin", "--check", "--stat", "a.vis"],
    ];
    for args in cases {
        let (ma, out, err) = vis_stdin(args, b"fn f():\n    1\n");
        assert_eq!(ma, 2, "args {args:?} phải exit 2; stderr: {err}");
        assert_eq!(out, "", "usage không in stdout (args {args:?})");
        assert!(
            err.contains("dùng: vis fmt"),
            "phải in usage (args {args:?}): {err}"
        );
        assert!(
            err.contains("--stat"),
            "usage phải nêu `--stat` (args {args:?}): {err}"
        );
    }
}

/// Cờ lạ vẫn bị từ chối như trước (`--statx` không bị nhận nhầm thành tệp).
#[test]
fn co_la_van_bi_tu_choi() {
    let (ma, out, err) = vis(&["fmt", "--statx", "a.vis"]);
    assert_eq!(ma, 2, "stderr: {err}");
    assert_eq!(out, "");
    assert!(err.contains("không hỗ trợ cờ"), "stderr: {err}");
    assert!(err.contains("dùng: vis fmt"), "stderr: {err}");
}
