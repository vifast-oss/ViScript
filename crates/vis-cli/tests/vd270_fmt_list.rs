// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270** — `vis fmt --check --list` (đường tệp/thư mục và `--stdin --check`).
//!
//! Đỏ trước (HEAD `3d30be1`, trước task này): `--list` **chưa** phải cờ của `vis fmt`:
//! - `vis fmt --list a.vis` ⇒ `vis fmt: không hỗ trợ cờ `--list`` + usage, exit 2;
//! - `vis fmt --check --list a.vis` coi `--list` là **tệp** ⇒ stderr `# --list: không đọc được…`,
//!   stdout vẫn in `a.vis` nếu lệch, exit 2.
//!
//! Vì vậy mọi ca dưới đây đỏ trước.
//!
//! Hợp đồng: đúng **một đường dẫn mỗi dòng** cho tệp cần sửa, **không** in gì khác ra stdout;
//! stderr chỉ cho dùng sai/lỗi đọc/lỗi cú pháp (luôn kèm tên tệp). Thứ tự: đối số theo thứ tự
//! người dùng, trong mỗi thư mục duyệt đệ quy **đã sắp**. Exit 0 (không tệp nào cần sửa) /
//! 1 (có tệp cần sửa) / 2 (lỗi đọc/cú pháp hoặc dùng sai). `--list` chỉ đi với `--check`, không
//! đi với `--json`/`--write`/`--indent-report`; `--stdin --check --list` in `-` khi cần sửa.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn tam(tag: &str) -> PathBuf {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static DEM: AtomicUsize = AtomicUsize::new(0);
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-fmt-list-{tag}-{}-{n}", std::process::id()));
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

/// Hỗn hợp tốt/sai: chỉ in tệp cần sửa, đúng **thứ tự đối số**, không in gì khác, exit 1.
#[test]
fn hon_hop_chi_in_tep_can_sua_dung_thu_tu_doi_so() {
    let d = tam("hon-hop");
    let tot = ghi(&d, "a_tot.vis", TOT);
    let sai1 = ghi(&d, "b_sai.vis", SAI);
    let sai2 = ghi(&d, "c_sai.vis", "fn g():\n  let x = 1\n  x\n");
    let (ma, out, err) = vis(&["fmt", "--check", "--list", &s(&sai2), &s(&tot), &s(&sai1)]);
    assert_eq!(ma, 1, "exit 1; stderr: {err}");
    assert_eq!(err, "", "chế độ list không in cảnh báo/usage");
    assert_eq!(
        out,
        format!("{}\n{}\n", s(&sai2), s(&sai1)),
        "chỉ đường dẫn tệp cần sửa, theo thứ tự đối số"
    );
    assert_eq!(
        std::fs::read_to_string(&sai1).unwrap(),
        SAI,
        "không ghi đĩa"
    );
    assert_eq!(
        std::fs::read_to_string(&sai2).unwrap(),
        "fn g():\n  let x = 1\n  x\n"
    );

    // `--list` đứng trước `--check` cũng nhận, cùng kết quả.
    let (ma, out2, err2) = vis(&["fmt", "--list", "--check", &s(&sai2), &s(&tot), &s(&sai1)]);
    assert_eq!(ma, 1, "stderr: {err2}");
    assert_eq!(out2, out);
    let _ = std::fs::remove_dir_all(&d);
}

/// Thư mục: đệ quy, sắp trong **từng** đối số; đối số sau giữ nguyên vị trí; bỏ đuôi lạ.
#[test]
fn thu_muc_duyet_de_quy_sap_trong_tung_doi_so() {
    let d = tam("thu-muc");
    let scan = d.join("scan");
    let sub = scan.join("sub");
    let tot = ghi(&scan, "a_tot.vis", TOT);
    let sai_root = ghi(&scan, "z_sai.vis", SAI);
    let sai_sub_a = ghi(&sub, "a_sai.vis", SAI);
    let sai_sub_b = ghi(&sub, "b_sai.vis", SAI);
    ghi(&scan, "bo_qua.txt", "không phải .vis/.vish\n");
    let extra = ghi(&d, "extra_sai.vis", SAI);

    // Thứ tự trong thư mục: a_tot.vis, sub/a_sai.vis, sub/b_sai.vis, z_sai.vis (byte).
    let (ma, out, err) = vis(&["fmt", "--check", "--list", &s(&scan), &s(&extra)]);
    assert_eq!(ma, 1, "stderr: {err}");
    assert_eq!(err, "");
    assert_eq!(
        out,
        format!(
            "{}\n{}\n{}\n{}\n",
            s(&sai_sub_a),
            s(&sai_sub_b),
            s(&sai_root),
            s(&extra)
        ),
        "đối số `scan` đứng trước `extra`; trong `scan` đã sắp đệ quy; bỏ `a_tot.vis`/`bo_qua.txt`"
    );
    assert!(!out.contains(&s(&tot)), "không in tệp đã đúng: {out}");

    // Đảo thứ tự đối số ⇒ `extra` lên trước, phần thư mục vẫn sắp.
    let (ma, out2, err2) = vis(&["fmt", "--check", "--list", &s(&extra), &s(&scan)]);
    assert_eq!(ma, 1, "stderr: {err2}");
    assert_eq!(
        out2,
        format!(
            "{}\n{}\n{}\n{}\n",
            s(&extra),
            s(&sai_sub_a),
            s(&sai_sub_b),
            s(&sai_root)
        )
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// Sạch: không stdout, không stderr, exit 0 — kể cả thư mục có `.vish` đã đúng chuẩn.
#[test]
fn sach_im_lang_exit_0() {
    let d = tam("sach");
    ghi(&d, "a_tot.vis", TOT);
    ghi(&d, "b_tot.vish", "cmd f() -> int:\n    1\n");
    ghi(&d, "rong.vis", "\n");
    ghi(&d, "bo_qua.txt", SAI);
    let (ma, out, err) = vis(&["fmt", "--check", "--list", &s(&d)]);
    assert_eq!(ma, 0, "stderr: {err}");
    assert_eq!(out, "", "không tệp nào cần sửa ⇒ stdout rỗng");
    assert_eq!(err, "", "không cảnh báo, không usage");
    let _ = std::fs::remove_dir_all(&d);
}

/// `.vish` lệch thụt lề nhưng **không** lệch định dạng: `--check` văn bản có cảnh báo, `--list`
/// phải im lặng hoàn toàn (stderr chỉ dành cho lỗi) và exit 0.
#[test]
fn vish_canh_bao_thut_le_khong_lot_stderr() {
    let d = tam("vish-canh-bao");
    // Cùng nguồn với `fmt_indent.rs`: thân `cmd` 2 dấu cách — `--check` cảnh báo, không đỏ.
    let p = ghi(&d, "a.vish", "cmd f():\n  echo hi\n");
    let (ma, out, err) = vis(&["fmt", "--check", "--list", &s(&p)]);
    assert_eq!(ma, 0, "cảnh báo thụt lề không làm đỏ; stderr: {err}");
    assert_eq!(out, "");
    assert_eq!(err, "", "list không được lẫn cảnh báo văn bản");
    let _ = std::fs::remove_dir_all(&d);
}

/// Lỗi cú pháp: exit 2 (thắng 1), stderr đúng chẩn đoán **kèm tên tệp**, stdout chỉ tệp lệch còn lại.
#[test]
fn loi_cu_phap_exit_2_stderr_kem_ten_tep() {
    let d = tam("loi-cu-phap");
    let sai = ghi(&d, "b_sai.vis", SAI);
    let loi = ghi(&d, "c_loi.vis", "let x =\n");
    let (ma, out, err) = vis(&["fmt", "--check", "--list", &s(&sai), &s(&loi)]);
    assert_eq!(ma, 2, "exit 2; stdout: {out}");
    assert_eq!(
        out,
        format!("{}\n", s(&sai)),
        "vẫn in tệp lệch định dạng; tệp lỗi cú pháp không vào stdout"
    );
    assert_eq!(
        err,
        format!(
            "# {}: VS0201 1:8 mong biểu thức, thật `\\n` (NEWLINE)\n",
            s(&loi)
        )
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// Tệp không đọc được: exit 2, stdout rỗng, stderr nêu đúng đường dẫn.
#[test]
fn tep_khong_doc_duoc_exit_2() {
    let d = tam("khong-doc");
    let khong_co = d.join("khong_ton_tai.vis");
    let (ma, out, err) = vis(&["fmt", "--check", "--list", &s(&khong_co)]);
    assert_eq!(ma, 2, "stderr: {err}");
    assert_eq!(out, "", "không in đường dẫn khi không đọc/định dạng được");
    // Không so nguyên văn thông điệp OS (Windows: "The system cannot find the file specified");
    // chỉ đòi đúng đường dẫn + mã lỗi OS 2 + xuống dòng.
    let dau = format!("# {}: không đọc được: ", s(&khong_co));
    assert!(err.starts_with(&dau), "stderr: {err}");
    assert!(err.contains("os error 2"), "stderr: {err}");
    assert!(err.ends_with('\n'), "stderr: {err}");
    let _ = std::fs::remove_dir_all(&d);
}

/// `--stdin --check --list`: 0 đã đúng (im lặng) / 1 cần sửa (`-`) / 2 lỗi cú pháp (stderr), `--mode` có hiệu lực.
#[test]
fn stdin_check_list_0_1_2_chinh_xac() {
    let (ma, out, err) = vis_stdin(&["fmt", "--stdin", "--check", "--list"], TOT.as_bytes());
    assert_eq!(ma, 0, "stderr: {err}");
    assert_eq!(out, "");
    assert_eq!(err, "");

    let (ma, out, err) = vis_stdin(&["fmt", "--stdin", "--check", "--list"], SAI.as_bytes());
    assert_eq!(ma, 1, "stderr: {err}");
    assert_eq!(out, "-\n", "chỉ một dấu `-` khi stdin cần sửa");
    assert_eq!(err, "");

    let (ma, out, err) = vis_stdin(
        &["fmt", "--list", "--check", "--stdin"],
        "let x =\n".as_bytes(),
    );
    assert_eq!(ma, 2, "exit 2; stdout: {out}");
    assert_eq!(out, "", "lỗi cú pháp không in đường dẫn");
    assert_eq!(err, "# VS0201 1:8 mong biểu thức, thật `\\n` (NEWLINE)\n");

    // Đầu vào rỗng là điểm bất động ⇒ exit 0, không in gì.
    let (ma, out, err) = vis_stdin(&["fmt", "--stdin", "--check", "--list"], b"");
    assert_eq!(ma, 0, "stderr: {err}");
    assert_eq!(out, "");
    assert_eq!(err, "");

    // `--mode command` chọn đúng chế độ: 2 dấu cách đã đúng; mặc định script đòi 4 ⇒ `-`.
    let vish = b"fn f():\n  let x = cmd { echo hi }\n  x\n";
    let (ma, out, err) = vis_stdin(
        &["fmt", "--stdin", "--check", "--list", "--mode", "command"],
        vish,
    );
    assert_eq!(ma, 0, "stderr: {err}");
    assert_eq!(out, "");
    let (ma, out, err) = vis_stdin(&["fmt", "--stdin", "--check", "--list"], vish);
    assert_eq!(ma, 1, "stderr: {err}");
    assert_eq!(out, "-\n");
}

/// Stdin không phải UTF-8: exit 2, stdout rỗng, stderr nêu lỗi đọc.
#[test]
fn stdin_khong_utf8_exit_2() {
    let (ma, out, err) = vis_stdin(&["fmt", "--stdin", "--check", "--list"], &[0xff, 0xfe]);
    assert_eq!(ma, 2, "stderr: {err}");
    assert_eq!(out, "");
    assert_eq!(
        err,
        "vis fmt: không đọc được stdin (cần UTF-8): stream did not contain valid UTF-8\n"
    );
}

/// Tổ hợp cấm ở đường tệp ⇒ usage + exit 2, stdout rỗng, **không** ghi/sửa tệp.
#[test]
fn to_hop_sai_duong_tep_exit_2_usage() {
    let d = tam("sai-to-hop");
    let p = ghi(&d, "a.vis", SAI);
    let f = s(&p);
    let cases: &[(&[&str], &str)] = &[
        (
            &["fmt", "--list", &f],
            "vis fmt: --list chỉ dùng với --check\n",
        ),
        (&["fmt", "--list"], "vis fmt: --list chỉ dùng với --check\n"),
        (
            &["fmt", "--write", "--list", &f],
            "vis fmt: --list chỉ dùng với --check (không đi với --write/--indent-report)\n",
        ),
        (
            &["fmt", "--list", "--write", &f],
            "vis fmt: --list chỉ dùng với --check (không đi với --write/--indent-report)\n",
        ),
        (
            &["fmt", "--indent-report", "--list", &f],
            "vis fmt: --list chỉ dùng với --check (không đi với --write/--indent-report)\n",
        ),
        (
            &["fmt", "--list", "--indent-report", &f],
            "vis fmt: --list chỉ dùng với --check (không đi với --write/--indent-report)\n",
        ),
        (
            &["fmt", "--check", "--list", "--json", &f],
            "vis fmt: --list không đi với --json (JSON đã là đầu ra máy)\n",
        ),
        (
            &["fmt", "--json", "--list", "--check", &f],
            "vis fmt: --list không đi với --json (JSON đã là đầu ra máy)\n",
        ),
    ];
    for (args, dong_dau) in cases {
        let (ma, out, err) = vis(args);
        assert_eq!(ma, 2, "args {args:?} phải exit 2; stderr: {err}");
        assert_eq!(out, "", "usage không in stdout (args {args:?})");
        assert!(
            err.starts_with(dong_dau),
            "args {args:?} phải bắt đầu bằng {dong_dau:?}: {err}"
        );
        assert!(
            err.contains("dùng: vis fmt"),
            "args {args:?} phải in usage: {err}"
        );
    }
    assert_eq!(std::fs::read_to_string(&p).unwrap(), SAI, "không ghi đĩa");

    // `--check --list` thiếu tệp ⇒ usage (không phải lỗi đọc).
    let (ma, out, err) = vis(&["fmt", "--check", "--list"]);
    assert_eq!(ma, 2);
    assert_eq!(out, "");
    assert!(err.contains("dùng: vis fmt"), "stderr: {err}");
    let _ = std::fs::remove_dir_all(&d);
}

/// Tổ hợp cấm với `--stdin` ⇒ usage + exit 2 **trước khi** đọc stdin.
#[test]
fn to_hop_sai_stdin_exit_2_usage() {
    let cases: &[&[&str]] = &[
        &["fmt", "--stdin", "--list"],
        &["fmt", "--list", "--stdin"],
        &["fmt", "--stdin", "--check", "--list", "--json"],
        &["fmt", "--json", "--stdin", "--list", "--check"],
        &["fmt", "--stdin", "--check", "--list", "--write"],
        &["fmt", "--stdin", "--check", "--list", "--indent-report"],
        &["fmt", "--stdin", "--check", "--list", "a.vis"],
    ];
    for args in cases {
        let (ma, out, err) = vis_stdin(args, SAI.as_bytes());
        assert_eq!(ma, 2, "args {args:?} phải exit 2; stderr: {err}");
        assert_eq!(out, "", "usage không in stdout (args {args:?})");
        assert!(
            err.contains("dùng: vis fmt"),
            "args {args:?} phải in usage: {err}"
        );
    }
}

/// Cờ lạ gần `--list` vẫn bị từ chối như trước — nhận cờ theo **tên chính xác**, không tiền tố.
/// (Đỏ trước: chính `--list` từng rơi vào nhánh này với `không hỗ trợ cờ`.)
#[test]
fn co_la_van_bi_tu_choi() {
    for co in ["--listx", "--lis", "--list=1"] {
        // Cờ ở vị trí đầu (chế độ) ⇒ usage rõ ràng.
        let (ma, out, err) = vis(&["fmt", co, "a.vis"]);
        assert_eq!(ma, 2, "cờ {co} phải exit 2; stderr: {err}");
        assert_eq!(out, "");
        assert!(
            err.starts_with(&format!("vis fmt: không hỗ trợ cờ `{co}`\n")),
            "stderr: {err}"
        );
        assert!(err.contains("dùng: vis fmt"), "stderr: {err}");
    }
}
