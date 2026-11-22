// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270** — `vis fmt --check --json` (đường tệp/thư mục và `--stdin --check`).
//!
//! Đỏ trước (HEAD `226245b`, trước task này): `--json` **chưa** phải cờ của `vis fmt`:
//! - `vis fmt --json` ⇒ `vis fmt: không hỗ trợ cờ --json` + usage, exit 2;
//! - `vis fmt --check --json a.vis` coi `--json` là **tệp** ⇒ `# --json: không đọc được`, exit 2,
//!   stdout rỗng. Vì vậy mọi ca JSON dưới đây đỏ trước.
//!
//! Hợp đồng: **một** object JSON nén/dòng trên stdout, khoá + thứ tự cố định:
//! `{"ok":bool,"files":[{"path":…,"formatted":bool,"error":null|{"code":"VSxxxx","line":n,"col":n,"message":…}}]}`.
//! Đường tệp giữ **thứ tự đối số**, chỉ sắp trong từng thư mục; `line`/`col` **1-based**;
//! exit 0 (mọi tệp đã đúng) / 1 (cần sửa) / 2 (lỗi đọc/cú pháp hoặc dùng sai).
//! `--json` không đi với `--write`/`--indent-report` hay `--stdin` trần (thiếu `--check`).

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn tam(tag: &str) -> PathBuf {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static DEM: AtomicUsize = AtomicUsize::new(0);
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-fmt-json-{tag}-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("tạo thư mục tạm");
    d
}

fn ghi(d: &Path, ten: &str, src: &str) -> PathBuf {
    let p = d.join(ten);
    std::fs::write(&p, src).expect("ghi tệp");
    p
}

fn s(p: &Path) -> String {
    p.display().to_string()
}

/// Escape tối thiểu cho **kỳ vọng** JSON trong test (đường dẫn tạm; `\` cũng là escape JSON).
fn duong_json(p: &Path) -> String {
    s(p).replace('\\', "\\\\").replace('"', "\\\"")
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
    // CLI có thể từ chối tham số **trước khi** đọc stdin (vd `--stdin --json` thiếu `--check`)
    // ⇒ ống đóng sớm; BrokenPipe là hợp lệ, không phải lỗi test.
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

/// Tệp đã đúng chuẩn ⇒ `ok:true`, `formatted:true`, `error:null`, exit 0, stderr rỗng.
#[test]
fn tep_tot_json_chinh_xac_exit_0() {
    let d = tam("tot");
    let p = ghi(&d, "a.vis", "fn f():\n    1\n");
    let f = s(&p);
    let fj = duong_json(&p);
    let (ma, out, err) = vis(&["fmt", "--check", "--json", &f]);
    assert_eq!(ma, 0, "exit 0; stderr: {err}");
    assert_eq!(err, "", "chế độ JSON không in stderr");
    assert_eq!(
        out,
        format!(
            "{{\"ok\":true,\"files\":[{{\"path\":\"{fj}\",\"formatted\":true,\"error\":null}}]}}\n"
        )
    );

    // `--json` đứng trước `--check` cũng nhận, cùng JSON.
    let (ma, out2, err2) = vis(&["fmt", "--json", "--check", &f]);
    assert_eq!(ma, 0, "stderr: {err2}");
    assert_eq!(out2, out);
    let _ = std::fs::remove_dir_all(&d);
}

/// Tệp cần sửa thụt lề ⇒ `ok:false`, `formatted:false`, `error:null`, exit 1.
#[test]
fn tep_sai_json_chinh_xac_exit_1() {
    let d = tam("sai");
    let p = ghi(&d, "a.vis", "fn f():\n  1\n");
    let f = s(&p);
    let fj = duong_json(&p);
    let (ma, out, err) = vis(&["fmt", "--check", "--json", &f]);
    assert_eq!(ma, 1, "exit 1; stderr: {err}");
    assert_eq!(err, "");
    assert_eq!(
        out,
        format!(
            "{{\"ok\":false,\"files\":[{{\"path\":\"{fj}\",\"formatted\":false,\"error\":null}}]}}\n"
        )
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// Hỗn hợp tốt/sai, **thứ tự đối số** (không sắp toàn cục) ⇒ entry đúng thứ tự, exit 1.
#[test]
fn hon_hop_giu_thu_tu_doi_so_exit_1() {
    let d = tam("hon-hop");
    // Cố ý đặt `z` (tốt) trước `a` (sai): sắp toàn cục sẽ đảo, hợp đồng giữ thứ tự đối số.
    let tot = ghi(&d, "z_tot.vis", "fn f():\n    1\n");
    let sai = ghi(&d, "a_sai.vis", "fn f():\n  1\n");
    let (f_tot, f_sai) = (s(&tot), s(&sai));
    let (fj_tot, fj_sai) = (duong_json(&tot), duong_json(&sai));
    let (ma, out, err) = vis(&["fmt", "--check", "--json", &f_tot, &f_sai]);
    assert_eq!(ma, 1, "stderr: {err}");
    assert_eq!(err, "");
    assert_eq!(
        out,
        format!(
            "{{\"ok\":false,\"files\":[{{\"path\":\"{fj_tot}\",\"formatted\":true,\"error\":null}},{{\"path\":\"{fj_sai}\",\"formatted\":false,\"error\":null}}]}}\n"
        )
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// Lỗi cú pháp ⇒ entry có `error` đúng mã + `line`/`col` 1-based + message JSON-escape, exit 2.
#[test]
fn loi_cu_phap_json_chinh_xac_exit_2() {
    let d = tam("loi");
    let p = ghi(&d, "c_loi.vis", "let x =\n");
    let f = s(&p);
    let fj = duong_json(&p);
    let (ma, out, err) = vis(&["fmt", "--check", "--json", &f]);
    assert_eq!(ma, 2, "exit 2; stderr: {err}");
    assert_eq!(err, "", "lỗi cú pháp vẫn nằm trong JSON, không stderr");
    assert_eq!(
        out,
        format!(
            "{{\"ok\":false,\"files\":[{{\"path\":\"{fj}\",\"formatted\":false,\"error\":{{\"code\":\"VS0201\",\"line\":1,\"col\":8,\"message\":\"mong biểu thức, thật `\\n` (NEWLINE)\"}}}}]}}\n"
        )
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// Thư mục ⇒ duyệt đệ quy **đã sắp** (a trước z), mỗi tệp một entry, exit 1.
#[test]
fn thu_muc_duyet_sap_xep() {
    let d = tam("thu-muc");
    let sub = d.join("sub");
    std::fs::create_dir_all(&sub).expect("mkdir");
    let tot = ghi(&sub, "a_tot.vis", "fn f():\n    1\n");
    let sai = ghi(&d, "z_sai.vis", "fn f():\n  1\n");
    let (fj_tot, fj_sai) = (duong_json(&tot), duong_json(&sai));
    let (ma, out, err) = vis(&["fmt", "--check", "--json", &s(&d)]);
    assert_eq!(ma, 1, "stderr: {err}");
    assert_eq!(err, "");
    assert_eq!(
        out,
        format!(
            "{{\"ok\":false,\"files\":[{{\"path\":\"{fj_tot}\",\"formatted\":true,\"error\":null}},{{\"path\":\"{fj_sai}\",\"formatted\":false,\"error\":null}}]}}\n"
        )
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// `.vish` cảnh báo thụt lề ở chế độ văn bản nhưng **không** lọt stderr khi `--json`.
#[test]
fn json_khong_in_canh_bao_vish() {
    let d = tam("vish");
    let p = ghi(&d, "a.vish", "cmd f():\n  echo hi  \n");
    let f = s(&p);
    let fj = duong_json(&p);
    let (ma, out, err) = vis(&["fmt", "--check", "--json", &f]);
    assert_eq!(ma, 1, "exit 1; stderr: {err}");
    assert_eq!(err, "", "JSON không được lẫn cảnh báo văn bản");
    assert_eq!(
        out,
        format!(
            "{{\"ok\":false,\"files\":[{{\"path\":\"{fj}\",\"formatted\":false,\"error\":null}}]}}\n"
        )
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// `--stdin --check --json`: 0 đã đúng / 1 cần sửa / 2 lỗi cú pháp, mỗi ca **một** entry `-`.
#[test]
fn stdin_check_json_0_1_2_chinh_xac() {
    let (ma, out, err) = vis_stdin(
        &["fmt", "--stdin", "--check", "--json"],
        b"fn f():\n    1\n",
    );
    assert_eq!(ma, 0, "stderr: {err}");
    assert_eq!(err, "");
    assert_eq!(
        out,
        "{\"ok\":true,\"files\":[{\"path\":\"-\",\"formatted\":true,\"error\":null}]}\n"
    );

    let (ma, out, err) = vis_stdin(&["fmt", "--stdin", "--check", "--json"], b"fn f():\n  1\n");
    assert_eq!(ma, 1, "stderr: {err}");
    assert_eq!(err, "");
    assert_eq!(
        out,
        "{\"ok\":false,\"files\":[{\"path\":\"-\",\"formatted\":false,\"error\":null}]}\n"
    );

    let (ma, out, err) = vis_stdin(&["fmt", "--stdin", "--check", "--json"], b"let x =\n");
    assert_eq!(ma, 2, "stderr: {err}");
    assert_eq!(err, "", "lỗi cú pháp nằm trong JSON");
    assert_eq!(
        out,
        "{\"ok\":false,\"files\":[{\"path\":\"-\",\"formatted\":false,\"error\":{\"code\":\"VS0201\",\"line\":1,\"col\":8,\"message\":\"mong biểu thức, thật `\\n` (NEWLINE)\"}}]}\n"
    );

    // Đầu vào rỗng là điểm bất động ⇒ exit 0, một entry `-`.
    let (ma, out, err) = vis_stdin(&["fmt", "--stdin", "--check", "--json"], b"");
    assert_eq!(ma, 0, "stderr: {err}");
    assert_eq!(err, "");
    assert_eq!(
        out,
        "{\"ok\":true,\"files\":[{\"path\":\"-\",\"formatted\":true,\"error\":null}]}\n"
    );
}

/// `--json` nhận ở cả hai thứ tự với `--stdin --check`; `--mode command` vẫn có hiệu lực.
#[test]
fn stdin_json_thu_tu_co_va_mode() {
    let (ma, out, err) = vis_stdin(
        &["fmt", "--check", "--json", "--stdin"],
        b"fn f():\n    1\n",
    );
    assert_eq!(ma, 0, "stderr: {err}");
    assert_eq!(
        out,
        "{\"ok\":true,\"files\":[{\"path\":\"-\",\"formatted\":true,\"error\":null}]}\n"
    );

    // Chế độ lệnh: thụt lề 2 đã đúng; chế độ script (mặc định) sẽ đòi 4 ⇒ exit 1.
    let vish = b"fn f():\n  let x = cmd { echo hi }\n  x\n";
    let (ma, out, err) = vis_stdin(
        &["fmt", "--stdin", "--check", "--json", "--mode", "command"],
        vish,
    );
    assert_eq!(ma, 0, "stderr: {err}");
    assert_eq!(
        out,
        "{\"ok\":true,\"files\":[{\"path\":\"-\",\"formatted\":true,\"error\":null}]}\n"
    );
    let (ma, out, err) = vis_stdin(&["fmt", "--stdin", "--check", "--json"], vish);
    assert_eq!(ma, 1, "stderr: {err}");
    assert_eq!(
        out,
        "{\"ok\":false,\"files\":[{\"path\":\"-\",\"formatted\":false,\"error\":null}]}\n"
    );
}

/// Stdin không phải UTF-8 ⇒ exit 2, vẫn **một** object JSON với lỗi `VS0201` (đường đọc).
#[test]
fn stdin_khong_utf8_json_exit_2() {
    let (ma, out, err) = vis_stdin(&["fmt", "--stdin", "--check", "--json"], &[0xff, 0xfe]);
    assert_eq!(ma, 2, "stderr: {err}");
    assert_eq!(err, "", "chế độ JSON không in stderr");
    assert_eq!(
        out,
        "{\"ok\":false,\"files\":[{\"path\":\"-\",\"formatted\":false,\"error\":{\"code\":\"VS0201\",\"line\":1,\"col\":1,\"message\":\"không đọc được stdin (cần UTF-8): stream did not contain valid UTF-8\"}}]}\n"
    );
}

/// `--json` không `--check` (và mọi tổ hợp cấm) ⇒ usage + exit 2, stdout rỗng.
#[test]
fn to_hop_sai_exit_2_usage() {
    let d = tam("sai-to-hop");
    let p = ghi(&d, "a.vis", "fn f():\n    1\n");
    let f = s(&p);
    let cases: &[&[&str]] = &[
        &["fmt", "--json"],
        &["fmt", "--json", "--check"],
        &["fmt", "--json", "--write", &f],
        &["fmt", "--write", "--json", &f],
        &["fmt", "--json", "--indent-report", &f],
        &["fmt", "--indent-report", "--json", &f],
        &["fmt", "--check", "--json", "--mode", "script", &f],
        &["fmt", "--json", &f, "--check"], // --json sau tệp: không rơi vào chế độ stdout
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
            err.contains("--json"),
            "stderr phải nêu `--json` (args {args:?}): {err}"
        );
    }
    // Không tổ hợp sai nào được ghi tệp.
    assert_eq!(
        std::fs::read_to_string(&p).expect("đọc lại"),
        "fn f():\n    1\n"
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// Tổ hợp cấm với `--stdin` ⇒ usage + exit 2 **trước khi** đọc stdin.
#[test]
fn stdin_to_hop_sai_exit_2_usage() {
    let cases: &[&[&str]] = &[
        &["fmt", "--stdin", "--json"],
        &["fmt", "--json", "--stdin"],
        &["fmt", "--stdin", "--check", "--json", "--write"],
        &["fmt", "--stdin", "--check", "--json", "--indent-report"],
        &["fmt", "--stdin", "--check", "--json", "a.vis"],
    ];
    for args in cases {
        let (ma, out, err) = vis_stdin(args, b"fn f():\n    1\n");
        assert_eq!(ma, 2, "args {args:?} phải exit 2; stderr: {err}");
        assert_eq!(out, "", "usage không in stdout (args {args:?})");
        assert!(
            err.contains("dùng: vis fmt"),
            "phải in usage (args {args:?}): {err}"
        );
    }
}

/// Cờ lạ vẫn bị từ chối như trước (bằng chứng `--json` từng là cờ lạ: `--jsonx` giữ nguyên
/// đường usage/exit 2, không bị nhận nhầm thành tệp).
#[test]
fn co_la_van_bi_tu_choi() {
    let (ma, out, err) = vis(&["fmt", "--jsonx", "a.vis"]);
    assert_eq!(ma, 2, "stderr: {err}");
    assert_eq!(out, "");
    assert!(err.contains("không hỗ trợ cờ"), "stderr: {err}");
    assert!(err.contains("dùng: vis fmt"), "stderr: {err}");
}

/// Tệp không đọc được ⇒ entry lỗi `VS0201` line/col 1, exit 2 (không panic, không bỏ tệp).
#[test]
fn tep_khong_doc_duoc_json_exit_2() {
    let d = tam("khong-doc");
    let p = d.join("khong_ton_tai.vis");
    let f = s(&p);
    let fj = duong_json(&p);
    let (ma, out, err) = vis(&["fmt", "--check", "--json", &f]);
    assert_eq!(ma, 2, "stderr: {err}");
    assert_eq!(err, "");
    assert!(out.starts_with("{\"ok\":false,\"files\":["), "{out}");
    assert!(
        out.contains(&format!(
            "{{\"path\":\"{fj}\",\"formatted\":false,\"error\":{{\"code\":\"VS0201\",\"line\":1,\"col\":1,\"message\":\"không đọc được:"
        )),
        "{out}"
    );
    assert!(out.ends_with("]}\n"), "{out}");
    let _ = std::fs::remove_dir_all(&d);
}
