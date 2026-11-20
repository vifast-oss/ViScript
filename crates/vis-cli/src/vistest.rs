// SPDX-License-Identifier: MIT OR Apache-2.0
//! **`vis test`** (ST-30) — chạy bộ `test/sem/**` (và mọi thư mục được trỏ) in **TAP**.
//!
//! Quy ước bộ ca (giống harness `crates/vis-cli/tests/sem.rs`):
//! - `<tên>.vis`/`.vish` **không** có `<tên>.err` ⇒ phải chạy **exit 0** và `stdout` **byte-khớp** `.out`
//!   (bỏ dòng SPDX đầu của `.out`).
//! - có `<tên>.err` ⇒ phải **đỏ** đúng mã trong `.err` (`VS05xx` ⇒ exit **101**, còn lại ⇒ exit 2).
//!
//! `--vm` chạy **đường VM** (`--no-cache`); không có `--vm` thì chạy interp (mặc định `vis run`, VD-56).
//!
//! **VD-267**:
//! - `--filter <substr>` — chỉ chọn ca có **tên `test fn`** hoặc **đường dẫn hiển thị** của fixture
//!   chứa `<substr>` (phân biệt hoa/thường). Lọc **trước khi chạy** nên ca bị loại không có tác dụng
//!   phụ. Chuỗi rỗng hoặc không ca nào khớp ⇒ TAP hợp lệ `1..0`, exit **0**.
//! - `--fail-fast` — dừng ngay sau ca `not ok` **đầu tiên**. Kế hoạch TAP đếm **ca đã chạy**
//!   (`1..<đã chạy>`), không phải số ca đã chọn; `--json` cũng đếm ca đã chạy. Exit vẫn **1**.
//!
//! **VD-270**:
//! - `--repeat <N>` — chạy mỗi ca **đã chọn** N lần (`N ≥ 1`, mặc định **1**), thứ tự
//!   ca-trước-lặp-sau. `N = 1` giữ **nguyên byte** TAP/JSON cũ; `N > 1` ⇒ mỗi lần lặp là một ca
//!   riêng, tên `"<tên> [k/N]"` (k 1-based), đánh số TAP liên tục, plan `1..<số lần đã chạy>`.
//! - Đỏ ở **bất kỳ** lần lặp nào ⇒ exit **1**; không `--fail-fast` thì giữ **mọi** kết quả lặp.
//!   `--fail-fast` dừng ngay sau lần lặp đỏ đầu tiên (`--json` đếm cùng tập đã chạy).
//! - `--list` bỏ qua `--repeat` (liệt kê ca đã chọn **một lần**) nhưng vẫn kiểm N; `--timeout`
//!   áp **mỗi lần lặp** của **mọi ca** — `test fn` **và** fixture `.out/.err` (mở rộng ST-105:
//!   fixture quá hạn ⇒ kill, `not ok` + `# <tên>: quá hạn <ms> ms`, đếm là đỏ).
//! - `--shuffle [--seed <u64>]` — xáo thứ tự ca **đã chọn**, **sau `--filter`** và **trước**
//!   `--repeat` (vẫn ca-trước-lặp-sau) lẫn `--list` (liệt kê theo thứ tự đã xáo). Cùng seed +
//!   cùng tập chọn ⇒ cùng thứ tự; thiếu `--seed` ⇒ seed **0**; `--seed` không kèm `--shuffle`
//!   ⇒ exit **2**. PRNG **SplitMix64** + Fisher–Yates, không thêm phụ thuộc; không cờ ⇒
//!   nguyên byte thứ tự/TAP/JSON cũ.
//! - `--shard <I>/<N>` — chia ca **đã chọn** (sau `--filter`, sau `--shuffle`) thành `N` phần
//!   rời nhau theo chỉ số 0-based: phần `I` giữ các ca có `chỉ_số % N == I-1` (1 ≤ I ≤ N).
//!   Hợp `N` phần = đúng tập đã chọn, không trùng; `--list`/`--repeat`/`--json` áp **sau** shard.
//!   Cú pháp sai (`0/2`, `3/2`, `1/0`, thiếu `/`, thừa `/`, không nguyên, thiếu giá trị, lặp cờ)
//!   ⇒ exit **2** + usage nêu `--shard`.

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use vis_syntax::{Kind, Mode, NodeKind, Span};

fn usage() -> ExitCode {
    eprintln!("dùng: vis test [--vm] [--strict-caps] [--timeout <ms>] [--json] [--json-out <path>] [--filter <substr>] [--fail-fast] [--list] [--repeat <N>] [--shuffle [--seed <u64>]] [--shard <I>/<N>] [thư mục|tệp]");
    eprintln!("  --filter <substr>: chỉ chạy ca có tên `test fn` hoặc đường dẫn hiển thị chứa <substr> (phân biệt hoa/thường); chuỗi rỗng hoặc không khớp ⇒ `1..0`, exit 0
  --list: chỉ in tên các ca đã chọn (mỗi dòng), không chạy; --list --json in {{\"tests\":[…]}}.");
    eprintln!("  --timeout <ms>: giới hạn mỗi ca (test fn **và** fixture .out/.err) ≤ ms (≥ 50); quá hạn ⇒ kill + `not ok` với `quá hạn <ms> ms`; mặc định **không** giới hạn");
    eprintln!("  --fail-fast: dừng sau ca `not ok` đầu tiên; kế hoạch TAP = số ca **đã chạy**");
    eprintln!("  --repeat <N>: chạy mỗi ca đã chọn N lần (số nguyên ≥ 1, mặc định 1); N > 1 ⇒ mỗi lần lặp là một ca TAP/JSON tên `<tên> [k/N]`; --list bỏ qua --repeat (liệt kê một lần)");
    eprintln!("  --shuffle [--seed <u64>]: xáo thứ tự ca đã chọn trước khi chạy/liệt kê; cùng seed ⇒ cùng thứ tự, mặc định seed 0; --seed chỉ dùng cùng --shuffle (thiếu/sai ⇒ exit 2)");
    eprintln!("  --shard <I>/<N>: chỉ chạy phần I/N của ca đã chọn (1 ≤ I ≤ N); chia theo chỉ số sau --filter/--shuffle, trước --list/--repeat; N phần rời nhau, hợp = đủ tập");
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
            // **VD-258** — cấu hình core không có host `std.sqlite`; bỏ nhóm này thay vì
            // chạy 38 ca chắc chắn đỏ. Cấu hình mặc định (`sqlite`) không đổi.
            #[cfg(not(feature = "sqlite"))]
            if p.file_name().is_some_and(|n| n == "sqlite") {
                continue;
            }
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

/// **VD-270**: chạy tiến trình con với hạn tuỳ chọn — `Some(ms)` ⇒ [`chay_co_han`] (kill khi
/// quá hạn); `None` ⇒ [`Command::output`] **không** giới hạn, giữ nguyên hành vi cũ.
fn chay_theo_han(c: &mut Command, timeout_han: Option<u64>) -> (i32, String, String, bool) {
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
fn chay(
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

/// Kết quả **một lần chạy** một ca — tương ứng một dòng TAP và một phần tử `cases` JSON.
struct Kq {
    ok: bool,
    /// Dòng chi tiết ngay sau `not ok` (không có `# `, không có newline); rỗng khi `ok`.
    chi_tiet: String,
    /// `message` cho `--json` (đã làm một dòng + cắt ≤200 ký tự).
    msg: String,
}

impl Kq {
    fn ok() -> Self {
        Kq {
            ok: true,
            chi_tiet: String::new(),
            msg: String::new(),
        }
    }

    fn hong(chi_tiet: String, msg: impl Into<String>) -> Self {
        Kq {
            ok: false,
            chi_tiet,
            msg: mot_dong(&msg.into()),
        }
    }
}

/// **VD-270**: chạy **một lần** ca `p` — `test fn` (bản sao `thanh_main`) hoặc fixture `.out/.err`.
///
/// Tách khỏi vòng chọn ca để `--repeat` gọi lại nguyên xi từng lần; `--timeout` áp **mỗi lần**
/// chạy của **cả hai** nhánh — `test fn` **và** fixture `.out/.err` (VD-270 mở rộng ST-105).
fn chay_mot_lan(
    chi_so: usize,
    p: &Path,
    bai_test: Option<&(String, String)>,
    ten: &str,
    vm: bool,
    strict: bool,
    timeout_han: Option<u64>,
) -> Kq {
    if let Some((_, src)) = bai_test {
        // Ghi bản sao tạm, chạy `vis run` (interp hoặc VM), TAP theo **tên test**.
        let d = std::env::temp_dir().join(format!("vis-test-{}-{}", std::process::id(), chi_so));
        let _ = std::fs::remove_dir_all(&d);
        let _ = std::fs::create_dir_all(&d);
        let tep = d.join("t.vis");
        if let Err(e) = std::fs::write(&tep, src) {
            let m = format!("không ghi được bản sao: {e}");
            return Kq::hong(m.clone(), m);
        }
        let mut c = Command::new(std::env::current_exe().unwrap_or_else(|_| PathBuf::from("vis")));
        dat_path_tro_giup(&mut c);
        c.arg("run");
        if strict {
            c.arg("--strict-caps");
        }
        if vm {
            c.arg("--vm").arg("--no-cache");
        }
        c.arg(&tep);
        let (code, _out, stderr, qua_han) = chay_theo_han(&mut c, timeout_han);
        let _ = std::fs::remove_dir_all(&d);
        if qua_han {
            let ms = timeout_han.unwrap_or(0);
            let m = format!("quá hạn {ms} ms");
            return Kq::hong(format!("{ten}: {m}"), m);
        }
        if code == 0 {
            return Kq::ok();
        }
        let (ma, dong, cot, msg) =
            chan_doan(&stderr).unwrap_or(("VS?".into(), 1, 1, String::new()));
        return Kq::hong(
            format!("{ten}:{dong}:{cot}: {ma} {msg}"),
            format!("{ma} {msg}"),
        );
    }
    let err = p.with_extension("err");
    let (code, out, stderr, qua_han) = chay(p, vm, strict, timeout_han);
    // **VD-270**: hạn thắng mọi kỳ vọng `.err`/`.out`; cùng câu với nhánh `test fn`.
    if qua_han {
        let ms = timeout_han.unwrap_or(0);
        let m = format!("quá hạn {ms} ms");
        return Kq::hong(format!("{ten}: {m}"), m);
    }
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
        if code == muon && stderr.contains(mong) {
            return Kq::ok();
        }
        let (ma, dong, cot, msg) =
            chan_doan(&stderr).unwrap_or((mong.to_string(), 1, 1, String::new()));
        return Kq::hong(
            format!("{ten}:{dong}:{cot}: {ma} {msg}"),
            format!("{ma} {msg}"),
        );
    }
    let want_path = p.with_extension("out");
    let want = std::fs::read_to_string(&want_path).unwrap_or_default();
    let want = bo_spdx(&want);
    if code == 0 && out == want {
        return Kq::ok();
    }
    if code != 0 {
        let (ma, dong, cot, msg) =
            chan_doan(&stderr).unwrap_or(("VS?".into(), 1, 1, String::new()));
        return Kq::hong(
            format!("{ten}:{dong}:{cot}: {ma} {msg}"),
            format!("{ma} {msg}"),
        );
    }
    Kq::hong(
        format!("{ten}:1:1: VS0000 stdout khác kỳ vọng"),
        "VS0000 stdout khác kỳ vọng",
    )
}

/// **VD-270**: SplitMix64 — PRNG nhỏ, tất định, không phụ thuộc (trạng thái chỉ là `u64`).
fn splitmix64(s: &mut u64) -> u64 {
    *s = s.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *s;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// **VD-270**: Fisher–Yates từ cuối, nguồn ngẫu nhiên là `splitmix64(seed)` ⇒ cùng seed cho
/// **cùng hoán vị** trên mọi máy; không dùng crate `rand` nên không thêm phụ thuộc.
fn xao_tron<T>(v: &mut [T], seed: u64) {
    let mut s = seed;
    for i in (1..v.len()).rev() {
        let j = (splitmix64(&mut s) % (i as u64 + 1)) as usize;
        v.swap(i, j);
    }
}

/// **VD-270**: đọc `--shard <I>/<N>` — đúng một `/`, hai số nguyên dương, `1 ≤ I ≤ N`.
fn parse_shard(s: &str) -> Option<(usize, usize)> {
    let (a, b) = s.split_once('/')?;
    if b.contains('/') {
        return None;
    }
    let i = a.parse::<usize>().ok()?;
    let n = b.parse::<usize>().ok()?;
    if n == 0 || i == 0 || i > n {
        return None;
    }
    Some((i, n))
}

/// **VD-270**: object JSON của `--json`/`--json-out` (một dòng + `\n`).
fn json_body(cases: &[CaJson], hong: usize) -> String {
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
    format!(
        "{{\"total\":{},\"ok\":{},\"failed\":{},\"cases\":[{ds}]}}\n",
        cases.len(),
        cases.len().saturating_sub(hong),
        hong
    )
}

pub fn run(args: &[String]) -> ExitCode {
    let mut vm = false;
    let mut strict = false;
    // **ST-101**: `--json` — đầu ra máy đọc; không cờ ⇒ TAP y hệt.
    let mut json_ra = false;
    // **VD-270**: `--json-out <path>` — ghi cùng object JSON ra tệp, stdout vẫn TAP nếu không `--json`.
    let mut json_out: Option<String> = None;
    // **ST-105**: `--timeout <ms>` (≥50) chỉ áp cho **`test fn`**; mặc định **không** giới hạn.
    let mut timeout_han: Option<u64> = None;
    // **VD-267**: `--filter <substr>` — lọc **trước khi chạy**; `None` = chạy tất cả.
    let mut loc: Option<String> = None;
    // **VD-267**: `--fail-fast` — dừng sau ca `not ok` đầu tiên.
    let mut fail_fast = false;
    // **ST-281**: `--list` — chỉ in tên ca đã chọn, không chạy.
    let mut list = false;
    // **VD-270**: `--repeat <N>` — chạy mỗi ca đã chọn N lần (mặc định 1).
    let mut repeat: usize = 1;
    // **VD-270**: `--shuffle [--seed <u64>]` — xáo thứ tự ca đã chọn; seed mặc định 0.
    let mut shuffle = false;
    let mut seed: Option<u64> = None;
    // **VD-270**: `--shard <I>/<N>` — chỉ chạy phần `I` của `N` phần rời nhau (1-based).
    let mut shard: Option<(usize, usize)> = None;
    let mut duong: Option<PathBuf> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--vm" => vm = true,
            "--strict-caps" => strict = true,
            "--json" => json_ra = true,
            "--json-out" => {
                let Some(v) = args.get(i + 1) else {
                    eprintln!("vis test: `--json-out` cần đường dẫn tệp");
                    return ExitCode::from(2);
                };
                if v.is_empty() {
                    eprintln!("vis test: `--json-out` cần đường dẫn tệp (rỗng)");
                    return ExitCode::from(2);
                }
                if json_out.is_some() {
                    eprintln!("vis test: `--json-out` lặp lại");
                    return ExitCode::from(2);
                }
                json_out = Some(v.clone());
                i += 1;
            }
            s if s.starts_with("--json-out=") => {
                let v = &s["--json-out=".len()..];
                if v.is_empty() {
                    eprintln!("vis test: `--json-out` cần đường dẫn tệp (rỗng)");
                    return ExitCode::from(2);
                }
                if json_out.is_some() {
                    eprintln!("vis test: `--json-out` lặp lại");
                    return ExitCode::from(2);
                }
                json_out = Some(v.to_string());
            }
            "--fail-fast" => fail_fast = true,
            "--list" => list = true,
            // **VD-270**: `--shuffle` không tham số; `--seed` là u64 (thiếu/sai ⇒ exit 2).
            "--shuffle" => shuffle = true,
            "--seed" => {
                let Some(v) = args.get(i + 1) else {
                    eprintln!("vis test: `--seed` cần số nguyên u64 (0..=2^64-1)");
                    return ExitCode::from(2);
                };
                let Ok(n) = v.parse::<u64>() else {
                    eprintln!("vis test: `--seed` cần số nguyên u64 (0..=2^64-1), được `{v}`");
                    return ExitCode::from(2);
                };
                seed = Some(n);
                i += 1;
            }
            s if s.starts_with("--seed=") => {
                let v = &s["--seed=".len()..];
                let Ok(n) = v.parse::<u64>() else {
                    eprintln!("vis test: `--seed` cần số nguyên u64 (0..=2^64-1), được `{v}`");
                    return ExitCode::from(2);
                };
                seed = Some(n);
            }
            // **VD-270**: N sai (0, âm, không nguyên, thiếu) ⇒ exit 2; usage nêu `--repeat`.
            "--repeat" => {
                let Some(v) = args.get(i + 1) else {
                    eprintln!("vis test: `--repeat` cần số nguyên ≥ 1");
                    return ExitCode::from(2);
                };
                let Ok(n) = v.parse::<usize>() else {
                    eprintln!("vis test: `--repeat` cần số nguyên ≥ 1, được `{v}`");
                    return ExitCode::from(2);
                };
                if n < 1 {
                    eprintln!("vis test: `--repeat` phải ≥ 1, được {n}");
                    return ExitCode::from(2);
                }
                repeat = n;
                i += 1;
            }
            // **VD-270**: `--shard <I>/<N>`; sai cú pháp/miền/lặp ⇒ exit 2, usage nêu `--shard`.
            "--shard" => {
                let Some(v) = args.get(i + 1) else {
                    eprintln!("vis test: `--shard` cần `<I>/<N>` (1 ≤ I ≤ N)");
                    return ExitCode::from(2);
                };
                let Some((i_shard, n_shard)) = parse_shard(v) else {
                    eprintln!("vis test: `--shard` cần `<I>/<N>` (1 ≤ I ≤ N), được `{v}`");
                    return ExitCode::from(2);
                };
                if shard.is_some() {
                    eprintln!("vis test: `--shard` lặp lại");
                    return ExitCode::from(2);
                }
                shard = Some((i_shard, n_shard));
                i += 1;
            }
            s if s.starts_with("--shard=") => {
                let v = &s["--shard=".len()..];
                let Some((i_shard, n_shard)) = parse_shard(v) else {
                    eprintln!("vis test: `--shard` cần `<I>/<N>` (1 ≤ I ≤ N), được `{v}`");
                    return ExitCode::from(2);
                };
                if shard.is_some() {
                    eprintln!("vis test: `--shard` lặp lại");
                    return ExitCode::from(2);
                }
                shard = Some((i_shard, n_shard));
            }
            "--filter" => {
                let Some(v) = args.get(i + 1) else {
                    eprintln!("vis test: `--filter` cần chuỗi con");
                    return ExitCode::from(2);
                };
                if v.starts_with('-') {
                    eprintln!(
                        "vis test: `--filter` cần chuỗi con (gặp `{v}`; dùng --filter=<substr> nếu chuỗi bắt đầu bằng `-`)"
                    );
                    return ExitCode::from(2);
                }
                loc = Some(v.clone());
                i += 1;
            }
            s if s.starts_with("--filter=") => {
                loc = Some(s["--filter=".len()..].to_string());
            }
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
    // **VD-270**: `--json-out` không đi với `--list` (hai đầu ra máy khác khuôn).
    if json_out.is_some() && list {
        eprintln!("vis test: `--json-out` không đi với `--list`");
        return ExitCode::from(2);
    }
    // **VD-270**: `--seed` chỉ có nghĩa khi xáo; thiếu `--shuffle` ⇒ lỗi dùng (exit 2).
    if seed.is_some() && !shuffle {
        eprintln!("vis test: `--seed` chỉ dùng cùng `--shuffle`");
        return ExitCode::from(2);
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
    // **VD-267**: áp `--filter` **tại đây**, trước mọi lần chạy ⇒ ca bị loại không có tác dụng phụ.
    let loc = loc.as_deref();
    let chon = |ten: &str| match loc {
        None => true,
        // Quy ước VD-267: chuỗi rỗng ⇒ chọn 0 ca (tài liệu ở `usage`).
        Some(f) => !f.is_empty() && ten.contains(f),
    };
    let mut ca: Vec<(PathBuf, Option<(String, String)>)> = Vec::new();
    for p in files {
        let text = std::fs::read_to_string(&p).unwrap_or_default();
        let tests = ds_test(&text);
        if tests.is_empty() {
            let rel = p.strip_prefix(&goc).unwrap_or(&p);
            // Tên hiển thị của **một tệp** (gốc = chính tệp) là rỗng; khi lọc dùng đường dẫn đã
            // truyền để `--filter <tên-tệp>` vẫn khớp. Tên in ra TAP giữ nguyên hành vi cũ.
            let ten_loc = if rel.as_os_str().is_empty() {
                p.to_string_lossy().to_string()
            } else {
                rel.to_string_lossy().to_string()
            };
            if chon(&ten_loc) {
                ca.push((p, None));
            }
        } else {
            for (ten, test_span, name_span) in tests {
                if !chon(&ten) {
                    continue;
                }
                let src = thanh_main(&text, test_span, name_span);
                ca.push((p.clone(), Some((ten, src))));
            }
        }
    }
    // **VD-270**: xáo **ca đã chọn** (sau `--filter`), trước `--list` và trước vòng lặp
    // ca-trước-lặp-sau; không cờ ⇒ thứ tự cũ nguyên byte.
    if shuffle {
        xao_tron(&mut ca, seed.unwrap_or(0));
    }
    // **VD-270**: `--shard I/N` — sau `--filter`/`--shuffle`, trước `--list`/`--repeat`; các
    // phần rời nhau và hợp đúng tập đã chọn.
    if let Some((i_shard, n_shard)) = shard {
        ca = ca
            .into_iter()
            .enumerate()
            .filter(|(idx, _)| idx % n_shard == i_shard - 1)
            .map(|(_, c)| c)
            .collect();
    }
    if list {
        let mut tens: Vec<String> = Vec::new();
        for (p, bai) in &ca {
            let ten = match bai {
                Some((ten, _)) => ten.clone(),
                None => {
                    let rel = p.strip_prefix(&goc).unwrap_or(p);
                    if rel.as_os_str().is_empty() {
                        p.to_string_lossy().to_string()
                    } else {
                        rel.to_string_lossy().to_string()
                    }
                }
            };
            tens.push(ten);
        }
        if json_ra {
            let ds: Vec<String> = tens
                .iter()
                .map(|t| format!("\"{}\"", esc_json(t)))
                .collect();
            println!("{{\"tests\":[{}]}}", ds.join(","));
        } else {
            for ten in &tens {
                println!("{ten}");
            }
        }
        return ExitCode::SUCCESS;
    }
    let mut than = String::new();
    let mut hong = 0usize;
    let mut cases: Vec<CaJson> = Vec::new();
    // **VD-270**: số **lần lặp** đã chạy — cũng là số dòng TAP / độ dài `cases`/plan.
    let mut da_chay = 0usize;
    'ca: for (i, (p, bai_test)) in ca.iter().enumerate() {
        // **VD-270**: tên gốc của ca (không hậu tố) — `test fn` dùng **tên test**, fixture dùng
        // đường dẫn hiển thị như trước; hậu tố `[k/N]` chỉ thêm khi N > 1.
        let ten_goc = match bai_test {
            Some((ten_test, _)) => ten_test.clone(),
            None => p
                .strip_prefix(&goc)
                .unwrap_or(p)
                .to_string_lossy()
                .to_string(),
        };
        for k in 1..=repeat {
            // **VD-267/VD-270**: `--fail-fast` dừng **trước** lần lặp kế tiếp ngay sau lần đỏ đầu.
            if fail_fast && hong > 0 {
                break 'ca;
            }
            da_chay += 1;
            let ten = if repeat > 1 {
                format!("{ten_goc} [{k}/{repeat}]")
            } else {
                ten_goc.clone()
            };
            let kq = chay_mot_lan(i, p, bai_test.as_ref(), &ten_goc, vm, strict, timeout_han);
            if kq.ok {
                than.push_str(&format!("ok {da_chay} - {ten}\n"));
            } else {
                hong += 1;
                than.push_str(&format!("not ok {da_chay} - {ten}\n# {}\n", kq.chi_tiet));
            }
            cases.push(CaJson {
                ten,
                ok: kq.ok,
                msg: kq.msg,
            });
        }
    }
    let mut tap = String::new();
    tap.push_str("TAP version 13\n");
    // **VD-267/VD-270**: kế hoạch đếm **lần lặp đã chạy** (với `--fail-fast` dừng giữa chừng,
    // TAP vẫn hợp lệ); không có `--fail-fast` thì số đã chạy = số ca đã chọn × N.
    tap.push_str(&format!("1..{}\n", cases.len()));
    tap.push_str(&than);
    let body = if json_ra || json_out.is_some() {
        Some(json_body(&cases, hong))
    } else {
        None
    };
    if let Some(path) = json_out.as_deref() {
        if let Err(e) = std::fs::write(path, body.as_deref().unwrap_or_default()) {
            eprintln!("vis test: không ghi được `{path}`: {e}");
            return ExitCode::from(2);
        }
    }
    if json_ra {
        print!("{}", body.as_deref().unwrap_or_default());
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
