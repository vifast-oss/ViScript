// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-318e (VD-297)** — chạy tiến trình con cho `vis test`: PATH trợ giúp, hạn `--timeout`,
//! gom stdout/stderr nền. Tách nguyên văn khỏi `vistest.rs` (0 đổi hành vi).

use std::path::{Path, PathBuf};
use std::process::Command;

/// **ST-230 (nối VD-winport)**: PATH cho tiến trình con của `vis test`, có **thư mục của binary
/// `vis`** ở đầu. Fixture `.vish` gọi trợ giúp `test-child` đặt **cạnh** `vis` (cùng cách hai
/// harness Rust `tests/sem.rs` + `tests/diag_cmp.rs` làm bằng `CARGO_BIN_EXE_test-child`); không có
/// mục này thì `./target/debug/vis test test/sem` báo `VS0501 không chạy được chương trình ngoài
/// test-child` cho 4 fixture `vish/ngoai-*` dù `cargo test` xanh. Chỉ đổi PATH của **tiến trình
/// con**; PATH của chính `vis` giữ nguyên.
pub(super) fn path_voi_muc_dau(dir: &Path) -> Option<std::ffi::OsString> {
    if dir.as_os_str().is_empty() {
        return None;
    }
    let cu = std::env::var_os("PATH").unwrap_or_default();
    let mut muc: Vec<PathBuf> = vec![dir.to_path_buf()];
    muc.extend(std::env::split_paths(&cu));
    std::env::join_paths(muc).ok()
}

/// Đặt PATH cho `c`: `dir(binary vis)` + PATH hiện tại; không làm gì nếu không xác định được.
pub(super) fn dat_path_tro_giup(c: &mut Command) {
    let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("vis"));
    if let Some(dir) = exe.parent() {
        if let Some(p) = path_voi_muc_dau(dir) {
            c.env("PATH", p);
        }
    }
}

/// **VD-270**: chạy tiến trình con với hạn tuỳ chọn — `Some(ms)` ⇒ [`chay_co_han`] (kill khi
/// quá hạn); `None` ⇒ [`Command::output`] **không** giới hạn, giữ nguyên hành vi cũ.
pub(super) fn chay_theo_han(
    c: &mut Command,
    timeout_han: Option<u64>,
) -> (i32, String, String, bool) {
    match timeout_han {
        Some(ms) => chay_co_han(c, ms),
        None => match c.output() {
            Ok(o) => (
                o.status.code().unwrap_or(-1),
                String::from_utf8_lossy(&o.stdout).to_string(),
                String::from_utf8_lossy(&o.stderr).to_string(),
                false,
            ),
            Err(e) => (-1, String::new(), format!("không chạy được: {e}"), false),
        },
    }
}

/// **VD-270**: chạy **fixture** `.out/.err` với hạn tuỳ chọn; trả thêm cờ `quá_hạn` để ca gọi
/// báo `not ok` + `quá hạn <ms> ms` **trước** khi so `.err`/`.out`.
pub(super) fn chay(
    ca: &Path,
    vm: bool,
    strict: bool,
    timeout_han: Option<u64>,
) -> (i32, String, String, bool) {
    let mut c = Command::new(std::env::current_exe().unwrap_or_else(|_| PathBuf::from("vis")));
    dat_path_tro_giup(&mut c);
    c.arg("run").arg("--tap");
    if strict {
        c.arg("--strict-caps");
    }
    if vm {
        c.arg("--vm").arg("--no-cache");
    }
    c.arg(ca);
    chay_theo_han(&mut c, timeout_han)
}

/// Mã chẩn đoán đầu trong stderr: `(mã, dòng, cột, thông điệp)` — dòng stderr có dạng
/// `tệp:dòng:cột: error VSxxxx: thông điệp`.
pub(super) fn chan_doan(stderr: &str) -> Option<(String, u32, u32, String)> {
    for l in stderr.lines() {
        if let Some(i) = l.find("VS") {
            let ma: String = l[i..].chars().take(6).collect();
            // Vị trí: hai trường số ngay trước `: error`.
            let truoc = l[..i]
                .trim_end_matches("error")
                .trim_end_matches(':')
                .trim_end();
            let mut so: Vec<u32> = truoc
                .rsplit(':')
                .take(2)
                .filter_map(|x| x.trim().parse::<u32>().ok())
                .collect();
            so.reverse();
            let (dong, cot) = match so.as_slice() {
                [d, c] => (*d, *c),
                [d] => (*d, 1),
                _ => (1, 1),
            };
            let msg = l[i..]
                .split_once(':')
                .map(|(_, m)| m.trim().to_string())
                .unwrap_or_default();
            return Some((ma, dong, cot, msg));
        }
    }
    None
}

/// Sau khi con đã thoát/bị kill, chờ hai luồng đọc ống **tối đa** chừng này. Cháu (nợ #88) có thể
/// còn giữ đầu ghi ⇒ không bao giờ có EOF; hết hạn thì bỏ phần output chưa về, **không** chờ.
const HAN_GOM_OUTPUT: std::time::Duration = std::time::Duration::from_millis(500);

/// Một luồng đọc `ong` tới EOF rồi gửi `(la_stderr, byte)` về kênh. Luồng chạy **song song** với
/// vòng poll ⇒ con in quá buffer pipe (~64 KiB) không bị chặn ở `write` (nợ #89, issue #23).
fn doc_nen(
    ong: Option<impl std::io::Read + Send + 'static>,
    la_stderr: bool,
    tx: std::sync::mpsc::Sender<(bool, Vec<u8>)>,
) {
    let Some(mut ong) = ong else { return };
    std::thread::spawn(move || {
        let mut b = Vec::new();
        let _ = ong.read_to_end(&mut b);
        let _ = tx.send((la_stderr, b));
    });
}

/// **ST-105**: chạy tiến trình con với hạn `ms`; quá hạn ⇒ `kill` + `wait`, trả `(mã, stdout, stderr,
/// quá_hạn)`. Poll `try_wait` mỗi 10 ms (không crate mới, không `unsafe`).
///
/// stdout **và** stderr được đọc bởi hai luồng nền **trong lúc** poll (chỉ một ống đầy là đủ
/// deadlock); gom kết quả bằng `recv_timeout` có hạn [`HAN_GOM_OUTPUT`] ⇒ không bao giờ treo.
pub(super) fn chay_co_han(c: &mut Command, ms: u64) -> (i32, String, String, bool) {
    // Cùng hợp đồng stdin với `Command::output` (đóng ngay) — thêm `--timeout` không đổi
    // cách fixture nhìn stdin.
    c.stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    let mut child = match c.spawn() {
        Ok(x) => x,
        Err(e) => return (-1, String::new(), format!("không chạy được: {e}"), false),
    };
    let (tx, rx) = std::sync::mpsc::channel();
    doc_nen(child.stdout.take(), false, tx.clone());
    doc_nen(child.stderr.take(), true, tx);
    let bat = std::time::Instant::now();
    let (status, qua_han) = loop {
        match child.try_wait() {
            Ok(Some(s)) => break (s, false),
            Ok(None) if bat.elapsed() >= std::time::Duration::from_millis(ms) => {
                let _ = child.kill();
                match child.wait() {
                    Ok(s) => break (s, true),
                    Err(e) => return (-1, String::new(), format!("không chờ được con: {e}"), true),
                }
            }
            Ok(None) => std::thread::sleep(std::time::Duration::from_millis(10)),
            Err(e) => return (-1, String::new(), format!("không chờ được con: {e}"), false),
        }
    };
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let cuoi = std::time::Instant::now() + HAN_GOM_OUTPUT;
    for _ in 0..2 {
        match rx.recv_timeout(cuoi.saturating_duration_since(std::time::Instant::now())) {
            Ok((false, b)) => out = b,
            Ok((true, b)) => err = b,
            Err(_) => break,
        }
    }
    (
        status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out).to_string(),
        String::from_utf8_lossy(&err).to_string(),
        qua_han,
    )
}
