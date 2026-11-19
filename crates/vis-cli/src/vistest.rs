// SPDX-License-Identifier: MIT OR Apache-2.0
//! **`vis test`** (ST-30) — chạy bộ `test/sem/**` (và mọi thư mục được trỏ) in **TAP**.
//!
//! Quy ước bộ ca (giống harness `crates/vis-cli/tests/sem.rs`):
//! - `<tên>.vis`/`.vish` **không** có `<tên>.err` ⇒ phải chạy **exit 0** và `stdout` **byte-khớp** `.out`
//!   (bỏ dòng SPDX đầu của `.out`).
//! - có `<tên>.err` ⇒ phải **đỏ** đúng mã trong `.err` (`VS05xx` ⇒ exit **101**, còn lại ⇒ exit 2).
//!
//! `--vm` chạy **đường VM** (`--no-cache`); không có `--vm` thì chạy interp (mặc định `vis run`, VD-56).

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use vis_syntax::{Kind, Mode, NodeKind, Span};

fn usage() -> ExitCode {
    eprintln!("dùng: vis test [--vm] [--strict-caps] [--timeout <ms>] [--json] [thư mục|tệp]");
    ExitCode::from(2)
}

/// **ST-101b (tạm)**: escape JSON hiện tại — tách để đo đỏ (sau thay bằng `json_esc`).
fn esc_json(s: &str) -> String {
    crate::json_esc::esc(s)
}

/// **ST-101**: thông điệp JSON **một dòng**, cắt ≤200 **ký tự** (char, không byte).
fn mot_dong(m: &str) -> String {
    let s = m.replace(['\n', '\r', '\t'], " ");
    // ≤200 **kể cả** dấu “…” khi cắt.
    if s.chars().count() <= 200 {
        return s;
    }
    let mut o: String = s.chars().take(199).collect();
    o.push('…');
    o
}

/// **ST-101**: ca cho `--json`.
struct CaJson {
    ten: String,
    ok: bool,
    msg: String,
}

/// Bỏ dòng SPDX đầu của tệp kỳ vọng (QD §9.3 — fixture `.out` có header).
fn bo_spdx(s: &str) -> &str {
    match s.split_once('\n') {
        Some((d, rest)) if d.trim_start().starts_with('#') && d.contains("SPDX") => rest,
        _ => s,
    }
}

/// Tìm `test/sem` từ cwd leo lên (chạy được cả trong `crates/vis-cli` và ở gốc repo).
fn tim_mac_dinh() -> Option<PathBuf> {
    let mut d = std::env::current_dir().ok()?;
    loop {
        for ten in ["test/sem", "viscript/test/sem"] {
            let p = d.join(ten);
            if p.is_dir() {
                return Some(p);
            }
        }
        if !d.pop() {
            return None;
        }
    }
}

fn gom(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    let mut ds: Vec<PathBuf> = rd.filter_map(|e| e.ok()).map(|e| e.path()).collect();
    ds.sort();
    for p in ds {
        if p.is_dir() {
            gom(&p, out);
        } else if matches!(
            p.extension().and_then(|x| x.to_str()),
            Some("vis") | Some("vish")
        ) {
            out.push(p);
        }
    }
}

/// **ST-79 (VD-108)**: tên + span của mọi `test fn` trong tệp, theo thứ tự khai báo (CST).
fn ds_test(text: &str) -> Vec<(String, Span, Span)> {
    let out = vis_syntax::parse(text, Mode::Script);
    let cst = &out.cst;
    let mut ds = Vec::new();
    for n in cst.root.children() {
        if n.kind != NodeKind::TestDecl {
            continue;
        }
        let toks: Vec<&vis_syntax::Token> = n.tokens(&cst.tokens).collect();
        let mut idents = toks.iter().filter(|t| t.kind == Kind::Ident);
        let Some(test_tok) = idents.next() else {
            continue;
        };
        let Some(name_tok) = idents.next() else {
            continue;
        };
        ds.push((name_tok.text.clone(), test_tok.span, name_tok.span));
    }
    ds
}

/// Đổi `test fn <tên>` thành `fn main` (chỉ tên đang chọn); giữ nguyên phần còn lại.
fn thanh_main(text: &str, test_span: Span, name_span: Span) -> String {
    // **VD-241 § fix(Windows/CRLF #psvn-A1)**: span trỏ vào văn bản chuẩn hoá CRLF→LF, slice theo đó.
    let norm = vis_syntax::lex::normalize_source(text);
    let mut s = String::with_capacity(norm.len());
    s.push_str(&norm[..test_span.start]);
    s.push_str("fn main");
    s.push_str(&norm[name_span.end..]);
    s
}

/// **ST-230 (nối VD-winport)**: PATH cho tiến trình con của `vis test`, có **thư mục của binary
/// `vis`** ở đầu. Fixture `.vish` gọi trợ giúp `test-child` đặt **cạnh** `vis` (cùng cách hai
/// harness Rust `tests/sem.rs` + `tests/diag_cmp.rs` làm bằng `CARGO_BIN_EXE_test-child`); không có
/// mục này thì `./target/debug/vis test test/sem` báo `VS0501 không chạy được chương trình ngoài
/// test-child` cho 4 fixture `vish/ngoai-*` dù `cargo test` xanh. Chỉ đổi PATH của **tiến trình
/// con**; PATH của chính `vis` giữ nguyên.
fn path_voi_muc_dau(dir: &Path) -> Option<std::ffi::OsString> {
    if dir.as_os_str().is_empty() {
        return None;
    }
    let cu = std::env::var_os("PATH").unwrap_or_default();
    let mut muc: Vec<PathBuf> = vec![dir.to_path_buf()];
    muc.extend(std::env::split_paths(&cu));
    std::env::join_paths(muc).ok()
}

/// Đặt PATH cho `c`: `dir(binary vis)` + PATH hiện tại; không làm gì nếu không xác định được.
fn dat_path_tro_giup(c: &mut Command) {
    let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("vis"));
    if let Some(dir) = exe.parent() {
        if let Some(p) = path_voi_muc_dau(dir) {
            c.env("PATH", p);
        }
    }
}

fn chay(ca: &Path, vm: bool, strict: bool) -> (i32, String, String) {
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
    match c.output() {
        Ok(o) => (
            o.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&o.stdout).to_string(),
            String::from_utf8_lossy(&o.stderr).to_string(),
        ),
        Err(e) => (-1, String::new(), format!("không chạy được: {e}")),
    }
}

/// Mã chẩn đoán đầu trong stderr: `(mã, dòng, cột, thông điệp)` — dòng stderr có dạng
/// `tệp:dòng:cột: error VSxxxx: thông điệp`.
fn chan_doan(stderr: &str) -> Option<(String, u32, u32, String)> {
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
fn chay_co_han(c: &mut Command, ms: u64) -> (i32, String, String, bool) {
    c.stdout(std::process::Stdio::piped())
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

pub fn run(args: &[String]) -> ExitCode {
    let mut vm = false;
    let mut strict = false;
    // **ST-101**: `--json` — đầu ra máy đọc; không cờ ⇒ TAP y hệt.
    let mut json_ra = false;
    // **ST-105**: `--timeout <ms>` (≥50) chỉ áp cho **`test fn`**; mặc định **không** giới hạn.
    let mut timeout_han: Option<u64> = None;
    let mut duong: Option<PathBuf> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--vm" => vm = true,
            "--strict-caps" => strict = true,
            "--json" => json_ra = true,
            "--timeout" => {
                let Some(v) = args.get(i + 1) else {
                    eprintln!("vis test: `--timeout` cần số nguyên ≥ 50 (ms)");
                    return ExitCode::from(2);
                };
                let Ok(ms) = v.parse::<u64>() else {
                    eprintln!("vis test: `--timeout` cần số nguyên ≥ 50 (ms), được `{v}`");
                    return ExitCode::from(2);
                };
                if ms < 50 {
                    eprintln!("vis test: `--timeout` phải ≥ 50 ms, được {ms}");
                    return ExitCode::from(2);
                }
                timeout_han = Some(ms);
                i += 1;
            }
            "-h" | "--help" => return usage(),
            s if s.starts_with('-') => {
                eprintln!("vis test: không hỗ trợ cờ `{s}`");
                return usage();
            }
            s => duong = Some(PathBuf::from(s)),
        }
        i += 1;
    }
    let goc = match duong {
        Some(p) => {
            if !p.exists() {
                eprintln!("vis test: không có đường dẫn `{}`", p.display());
                return ExitCode::from(2);
            }
            p
        }
        None => {
            match tim_mac_dinh() {
                Some(p) => p,
                None => {
                    eprintln!("vis test: không tìm thấy `test/sem` (chạy ở gốc repo hoặc truyền đường dẫn)");
                    return ExitCode::from(2);
                }
            }
        }
    };
    let mut files: Vec<PathBuf> = Vec::new();
    if goc.is_dir() {
        gom(&goc, &mut files);
    } else {
        files.push(goc.clone());
    }
    // **ST-79 (VD-108)**: tệp có `test fn` ⇒ **mỗi test một ca TAP** (chạy bằng bản sao đổi
    // `test fn <tên>` → `fn main`); tệp thường giữ nguyên hành vi fixture `.out`/`.err`.
    let mut ca: Vec<(PathBuf, Option<(String, String)>)> = Vec::new();
    for p in files {
        let text = std::fs::read_to_string(&p).unwrap_or_default();
        let tests = ds_test(&text);
        if tests.is_empty() {
            ca.push((p, None));
        } else {
            for (ten, test_span, name_span) in tests {
                let src = thanh_main(&text, test_span, name_span);
                ca.push((p.clone(), Some((ten, src))));
            }
        }
    }
    let mut tap = String::new();
    tap.push_str("TAP version 13\n");
    tap.push_str(&format!("1..{}\n", ca.len()));
    let mut hong = 0usize;
    let mut cases: Vec<CaJson> = Vec::new();
    for (i, (p, bai_test)) in ca.iter().enumerate() {
        let n = i + 1;
        let ten = p
            .strip_prefix(&goc)
            .unwrap_or(p)
            .to_string_lossy()
            .to_string();
        if let Some((ten_test, src)) = bai_test {
            // Ghi bản sao tạm, chạy `vis run` (interp hoặc VM), TAP theo **tên test**.
            let d = std::env::temp_dir().join(format!("vis-test-{}-{}", std::process::id(), i));
            let _ = std::fs::remove_dir_all(&d);
            let _ = std::fs::create_dir_all(&d);
            let tep = d.join("t.vis");
            if let Err(e) = std::fs::write(&tep, src) {
                hong += 1;
                let m = format!("không ghi được bản sao: {e}");
                tap.push_str(&format!("not ok {n} - {ten_test}\n# {m}\n"));
                cases.push(CaJson {
                    ten: ten_test.clone(),
                    ok: false,
                    msg: mot_dong(&m),
                });
                continue;
            }
            let mut c =
                Command::new(std::env::current_exe().unwrap_or_else(|_| PathBuf::from("vis")));
            dat_path_tro_giup(&mut c);
            c.arg("run");
            if strict {
                c.arg("--strict-caps");
            }
            if vm {
                c.arg("--vm").arg("--no-cache");
            }
            c.arg(&tep);
            // **ST-105**: chỉ nhánh `test fn` mới áp `--timeout`; fixture `.out/.err` giữ nguyên.
            let (code, _out, stderr, qua_han) = match timeout_han {
                Some(ms) => chay_co_han(&mut c, ms),
                None => match c.output() {
                    Ok(o) => (
                        o.status.code().unwrap_or(-1),
                        String::from_utf8_lossy(&o.stdout).to_string(),
                        String::from_utf8_lossy(&o.stderr).to_string(),
                        false,
                    ),
                    Err(e) => (-1, String::new(), format!("không chạy được: {e}"), false),
                },
            };
            let _ = std::fs::remove_dir_all(&d);
            if qua_han {
                hong += 1;
                let ms = timeout_han.unwrap_or(0);
                let m = format!("quá hạn {ms} ms");
                tap.push_str(&format!("not ok {n} - {ten_test}\n# {ten_test}: {m}\n"));
                cases.push(CaJson {
                    ten: ten_test.clone(),
                    ok: false,
                    msg: mot_dong(&m),
                });
                continue;
            }
            if code == 0 {
                tap.push_str(&format!("ok {n} - {ten_test}\n"));
                cases.push(CaJson {
                    ten: ten_test.clone(),
                    ok: true,
                    msg: String::new(),
                });
            } else {
                hong += 1;
                tap.push_str(&format!("not ok {n} - {ten_test}\n"));
                let (ma, dong, cot, msg) =
                    chan_doan(&stderr).unwrap_or(("VS?".into(), 1, 1, String::new()));
                tap.push_str(&format!("# {ten_test}:{dong}:{cot}: {ma} {msg}\n"));
                cases.push(CaJson {
                    ten: ten_test.clone(),
                    ok: false,
                    msg: mot_dong(&format!("{ma} {msg}")),
                });
            }
            continue;
        }
        let err = p.with_extension("err");
        let (code, out, stderr) = chay(p, vm, strict);
        if err.exists() {
            let mong = std::fs::read_to_string(&err).unwrap_or_default();
            // `.err` có thể có header SPDX: lấy **dòng đầu không phải chú thích**.
            let mong = mong
                .lines()
                .map(str::trim)
                .find(|l| !l.is_empty() && !l.starts_with('#'))
                .unwrap_or("");
            let muon = if mong.starts_with("VS05") { 101 } else { 2 };
            // ST-30b: hai đường **đã đồng bộ** chẩn đoán ⇒ `--vm` khoá **đúng mã** như interp.
            let ok = code == muon && stderr.contains(mong);
            if ok {
                tap.push_str(&format!("ok {n} - {ten}\n"));
                cases.push(CaJson {
                    ten: ten.clone(),
                    ok: true,
                    msg: String::new(),
                });
            } else {
                hong += 1;
                tap.push_str(&format!("not ok {n} - {ten}\n"));
                let (ma, dong, cot, msg) =
                    chan_doan(&stderr).unwrap_or((mong.to_string(), 1, 1, String::new()));
                tap.push_str(&format!("# {ten}:{dong}:{cot}: {ma} {msg}\n"));
                cases.push(CaJson {
                    ten: ten.clone(),
                    ok: false,
                    msg: mot_dong(&format!("{ma} {msg}")),
                });
            }
            continue;
        }
        let want_path = p.with_extension("out");
        let want = std::fs::read_to_string(&want_path).unwrap_or_default();
        let want = bo_spdx(&want);
        if code == 0 && out == want {
            tap.push_str(&format!("ok {n} - {ten}\n"));
            cases.push(CaJson {
                ten: ten.clone(),
                ok: true,
                msg: String::new(),
            });
        } else {
            hong += 1;
            tap.push_str(&format!("not ok {n} - {ten}\n"));
            if code != 0 {
                let (ma, dong, cot, msg) =
                    chan_doan(&stderr).unwrap_or(("VS?".into(), 1, 1, String::new()));
                tap.push_str(&format!("# {ten}:{dong}:{cot}: {ma} {msg}\n"));
                cases.push(CaJson {
                    ten: ten.clone(),
                    ok: false,
                    msg: mot_dong(&format!("{ma} {msg}")),
                });
            } else {
                tap.push_str(&format!("# {ten}:1:1: VS0000 stdout khác kỳ vọng\n"));
                cases.push(CaJson {
                    ten: ten.clone(),
                    ok: false,
                    msg: "VS0000 stdout khác kỳ vọng".to_string(),
                });
            }
        }
    }
    if json_ra {
        let mut ds = String::new();
        for (i, c) in cases.iter().enumerate() {
            if i > 0 {
                ds.push(',');
            }
            ds.push_str(&format!(
                "{{\"name\":\"{}\",\"ok\":{},\"message\":\"{}\"}}",
                esc_json(&c.ten),
                c.ok,
                esc_json(&c.msg)
            ));
        }
        let body = format!(
            "{{\"total\":{},\"ok\":{},\"failed\":{},\"cases\":[{ds}]}}\n",
            ca.len(),
            ca.len().saturating_sub(hong),
            hong
        );
        print!("{body}");
    } else {
        print!("{tap}");
    }
    if hong > 0 {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

/// `vis test` chỉ dùng ở chế độ script/lệnh — giữ tham chiếu để `Mode` không bị bỏ (tài liệu hoá).
#[allow(dead_code)]
fn _mode(_: Mode) {}

#[cfg(test)]
mod tests {
    use super::*;
    use vis_interp::json;
    use vis_interp::value::Value;

    /// Đo đỏ ST-101b: newline/tab/control trong `message` làm JSON **không parse được**.
    #[test]
    fn esc_json_newline_tab_control_parse_duoc() {
        let goc = "a\"b\\c\n\t\r\u{1}";
        let body = format!("{{\"m\":\"{}\"}}", esc_json(goc));
        let v = json::decode(&body).expect("JSON phải hợp lệ");
        let Value::Map(kv) = v else {
            panic!("không phải map")
        };
        let m = kv.iter().find_map(|(k, v)| match (k, v) {
            (Value::Str(k), Value::Str(v)) if k == "m" => Some(v.clone()),
            _ => None,
        });
        assert_eq!(m.as_deref(), Some(goc));
    }

    /// `sh -c <script>` — `test fn` ViScript không in được ra stderr, cũng không chạy được lệnh
    /// ngoài, nên hai ca dưới gọi thẳng `chay_co_han` với một con `sh`.
    ///
    /// **ST-139 (Agent 0)**: chỉ Unix — `/bin/sh` không có trên Windows. `test_timeout.rs` cũng đã
    /// dùng `ps` (Unix) từ ST-105, nhưng ca **unit** này chạy trong `cargo test --workspace` của
    /// cổng trên mọi nền, nên phải tự chặn để `gate --fast` không đỏ trên Windows (ST-136).
    #[cfg(unix)]
    fn sh(script: &str) -> Command {
        let mut c = Command::new("/bin/sh");
        c.arg("-c").arg(script);
        c
    }

    /// Nợ #89 (issue #23): con in ~200 KB ra **stderr** (> buffer pipe ~64 KiB) rồi thoát ngay ⇒
    /// không được báo quá hạn oan, stderr phải về **đủ**.
    #[cfg(unix)]
    #[test]
    fn stderr_nhieu_khong_bi_qua_han_oan() {
        let t0 = std::time::Instant::now();
        let (ma, _out, err, qua_han) = chay_co_han(
            &mut sh("i=0; while [ $i -lt 20000 ]; do echo 0123456789 >&2; i=$((i+1)); done"),
            3_000,
        );
        assert!(!qua_han, "báo quá hạn oan sau {:?}", t0.elapsed());
        assert_eq!(ma, 0);
        assert_eq!(err.len(), 20_000 * 11, "stderr phải về đủ");
        assert!(t0.elapsed().as_millis() < 3_000);
    }

    /// Quá hạn **không bao giờ treo**: con bị kill nhưng **cháu** (`sleep 5`, nợ #88 chưa kill
    /// cây cháu) còn giữ đầu ghi stdout/stderr ⇒ vẫn phải trả `quá hạn` trong < 3 s.
    #[cfg(unix)]
    #[test]
    fn chau_giu_ong_khong_lam_treo() {
        let t0 = std::time::Instant::now();
        let (_ma, _out, _err, qua_han) = chay_co_han(&mut sh("sleep 5 & sleep 10"), 300);
        let dt = t0.elapsed();
        assert!(qua_han);
        assert!(
            dt.as_millis() < 3_000,
            "{dt:?} ≥ 3 s ⇒ đang chờ cháu đóng ống"
        );
    }

    /// **ST-230**: PATH cho tiến trình con có thư mục binary ở **đầu**, phần PATH cũ giữ nguyên
    /// thứ tự phía sau; thư mục rỗng ⇒ không đổi PATH (nhánh `current_exe()` fallback `"vis"`).
    #[test]
    fn path_voi_muc_dau_them_dau_va_giu_phan_con_lai() {
        let p = super::path_voi_muc_dau(Path::new("/tmp/muc-vis"))
            .expect("PATH phải join được (không chứa ký tự cấm)");
        let ds: Vec<PathBuf> = std::env::split_paths(&p).collect();
        assert_eq!(ds.first(), Some(&PathBuf::from("/tmp/muc-vis")));
        let cu: Vec<PathBuf> =
            std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default()).collect();
        assert_eq!(&ds[1..], &cu[..], "PATH cũ phải giữ nguyên thứ tự");
        assert!(
            super::path_voi_muc_dau(Path::new("")).is_none(),
            "thư mục rỗng ⇒ không đổi PATH"
        );
    }
}
