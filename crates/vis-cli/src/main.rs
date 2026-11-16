// SPDX-License-Identifier: MIT OR Apache-2.0
//! `vis` — lệnh dòng ViScript. Vòng 1 (ST-02b) mới có `explain`.

use std::process::ExitCode;

#[cfg(test)]
mod audit_span;
mod cache;
mod cli;
mod fsmock;
mod json_esc;
mod lsp;
mod modules;
mod net;
mod ngoai;
mod pkg;
mod quyen;
mod vistest;

use vis_diag::{all_codes, lookup_str, severity_label, Lang};
use vis_syntax::Mode;

use cli::run::{in_sandbox_check, ma_thoat_khi_loi, run_dry};
/// **ST-63 (VD-105)**: host mạng của CLI (TCP thật, chỉ loopback) — tên cục bộ cho rõ điểm nối.
use net::Net as HostNet;

fn usage() -> ExitCode {
    eprintln!(
        "dùng: vis explain [--lang vi|en|ja] <VSxxxx> | vis explain --list [--lang vi|en|ja] | vis run [--tap] [--strict-caps] <tệp.vis> [-- args…] | vis check <tệp…> | vis fmt <tệp> | vis fmt --write <tệp…> | vis fmt --check <tệp…> | vis fmt --indent-report <tệp…> | vis lsp --stdio | vis pkg add|install|list"
    );
    ExitCode::from(2)
}

/// **ST-78/ST-83**: trợ giúp riêng của `vis explain` — đúng số khối/số mã (đếm động) và **theo
/// `--lang`** (vi/en/ja); không viết cứng số.
fn usage_explain(lang: Lang) -> ExitCode {
    let khoi = vis_diag::BLOCKS_RESERVED.len();
    let ma = vis_diag::all_codes().count();
    match lang {
        Lang::Vi => {
            eprintln!(
                "dùng: vis explain <VSxxxx> [--lang vi|en|ja] | vis explain --list [--lang vi|en|ja]"
            );
            eprintln!(
                "sổ mã: {khoi} khối, {ma} mã — `vis explain --list` in toàn bộ (mã + câu ngắn)"
            );
        }
        Lang::En => {
            eprintln!(
                "usage: vis explain <VSxxxx> [--lang vi|en|ja] | vis explain --list [--lang vi|en|ja]"
            );
            eprintln!(
                "code book: {khoi} blocks, {ma} codes — `vis explain --list` prints all (code + short text)"
            );
        }
        Lang::Ja => {
            eprintln!(
                "使い方: vis explain <VSxxxx> [--lang vi|en|ja] | vis explain --list [--lang vi|en|ja]"
            );
            eprintln!(
                "コード帳: {khoi} ブロック、{ma} コード — `vis explain --list` で全件表示（コード + 短い説明）"
            );
        }
    }
    ExitCode::from(2)
}

/// **FsHost** của CLI (ST-42): `--fs-mock` ⇒ mock tất định, ngược lại ⇒ hệ tệp thật.
enum HostFs {
    That(ngoai::Runner),
    Mock(fsmock::FsMock),
}

impl vis_interp::builtin::FsHost for HostFs {
    fn read(&mut self, p: &str) -> Result<String, String> {
        match self {
            HostFs::That(r) => r.read(p),
            HostFs::Mock(m) => m.read(p),
        }
    }
    /// **ST-124 (VD-130b)**: chỉ host **hệ tệp thật** báo được danh tính tệp đã mở; mock giữ
    /// `(0, 0)` (default của trait) ⇒ tầng quyền giữ hành vi cũ.
    fn read_danh_tinh(&mut self, p: &str) -> Result<(String, u64, u64), String> {
        match self {
            HostFs::That(r) => r.read_danh_tinh(p),
            HostFs::Mock(m) => m.read_danh_tinh(p),
        }
    }
    /// **ST-137 (VD-141)**: host **thật** báo được đường dẫn thật của handle (Linux `/proc`);
    /// mock không có không gian tên ⇒ default của trait (`KhongCo`) ⇒ tầng quyền giữ bước ST-124.
    fn read_theo_handle(&mut self, p: &str) -> Result<vis_interp::builtin::DocQuaHandle, String> {
        match self {
            HostFs::That(r) => r.read_theo_handle(p),
            HostFs::Mock(m) => m.read_theo_handle(p),
        }
    }
    fn write(&mut self, p: &str, s: &str) -> Result<(), String> {
        match self {
            HostFs::That(r) => r.write(p, s),
            HostFs::Mock(m) => m.write(p, s),
        }
    }
    fn exists(&mut self, p: &str) -> bool {
        match self {
            HostFs::That(r) => r.exists(p),
            HostFs::Mock(m) => m.exists(p),
        }
    }
    fn list(&mut self, p: &str) -> Result<Vec<String>, String> {
        match self {
            HostFs::That(r) => r.list(p),
            HostFs::Mock(m) => m.list(p),
        }
    }
    fn list_entries(&mut self, p: &str) -> Result<Vec<vis_interp::builtin::MucFs>, String> {
        match self {
            HostFs::That(r) => r.list_entries(p),
            HostFs::Mock(m) => m.list_entries(p),
        }
    }
}

impl HostFs {
    fn tao(giai_thich: bool, mock: Option<&fsmock::FsMock>) -> Self {
        match mock {
            Some(m) => HostFs::Mock(m.clone()),
            None => HostFs::That(ngoai::Runner {
                giai_thich,
                ..ngoai::Runner::default()
            }),
        }
    }
}

/// Ghi ra stdout **một lần**; `BrokenPipe` (vd `vis fmt … | head`) ⇒ im lặng thoát 0, không panic.
/// Lỗi I/O khác ⇒ báo stderr và trả `false` (caller thoát 2).
fn xuat_stdout(s: &str) -> bool {
    use std::io::Write;
    let stdout = std::io::stdout();
    let mut w = stdout.lock();
    match w.write_all(s.as_bytes()).and_then(|()| w.flush()) {
        Ok(()) => true,
        Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => std::process::exit(0),
        Err(e) => {
            eprintln!("# không ghi được stdout: {e}");
            false
        }
    }
}

/// Escape ký tự điều khiển để chẩn đoán nằm gọn trên **một** dòng (TAP/stderr).
fn dong_chan_doan(code: &str, line: u32, col: u32, message: &str) -> String {
    format!(
        "{code} {line}:{col} {}",
        message
            .replace('\n', "\\n")
            .replace('\r', "\\r")
            .replace('\t', "\\t")
    )
}

/// `vis run [--tap] <tệp.vis> [-- args…]` — parse → lower → check → interp (ST-14).
pub(crate) fn run_run(rest: &[String]) -> ExitCode {
    let mut tap = false;
    let mut dung_vm = false;
    let mut dump = false;
    let mut dung_cache = true;
    let mut dump_deps = false;
    // **ST-72 (VD-106 (2))**: in quyền khai báo của `main` rồi exit 0, **không** chạy.
    let mut dry = false;
    // **ST-73 (VD-107)**: probe Landlock (chỉ đọc) + mức cưỡng chế runtime; **không** chạy chương trình.
    let mut sandbox_check = false;
    // **ST-132 (VD-120)**: cưỡng chế **thật** bằng kernel (Landlock) khi chạy — opt-in, chỉ Linux.
    let mut sandbox = false;
    // **ST-72b (VD-103)**: `--grant <tên>=<path>` thay root của cap đã khai (không thêm quyền mới).
    let mut grants: Vec<(String, String)> = Vec::new();
    let mut explain_perms = false;
    // **ST-81 (VD-108)**: mọi truy cập fs/net phải nằm trong `cap` đã khai (mặc định: không siết).
    let mut strict_caps = false;
    // **ST-130 (VD-124)**: thoát **tường minh** khỏi chế độ siết mặc định khi chương trình dùng gói
    // (`use pkg.*`) — khi đó in **một** dòng cảnh báo ra stderr.
    let mut no_strict_caps = false;
    // `vis run --no-extern`: tắt chương trình ngoài (lệnh lạ ⇒ `VS0601`).
    let mut co_no_extern = false;
    let mut tep: Option<String> = None;
    let mut prog_args: Vec<String> = Vec::new();
    let mut mock_tep: Option<String> = None;
    let mut i = 0;
    while i < rest.len() {
        match rest[i].as_str() {
            "--tap" => tap = true,
            // ST-20: chạy đường **VM** thay cho interp (cùng ngữ nghĩa; `steps` đơn vị **lệnh**).
            "--vm" => dung_vm = true,
            "--dump-bytecode" => dump = true,
            // ST-23: cache bytecode bật **mặc định** cho đường VM; `--no-cache` để tắt.
            "--no-cache" => dung_cache = false,
            // ST-25: in cây phụ thuộc của module.
            "--dump-deps" => dump_deps = true,
            "--dry" => dry = true,
            "--strict-caps" => strict_caps = true,
            "--no-strict-caps" => no_strict_caps = true,
            "--sandbox-check" => sandbox_check = true,
            "--sandbox" => sandbox = true,
            "--grant" => {
                i += 1;
                let Some(v) = rest.get(i) else {
                    eprintln!("vis run: `--grant` cần giá trị dạng <tên>=<đường-dẫn>");
                    return ExitCode::from(2);
                };
                let Some((ten, duong)) = v.split_once('=') else {
                    eprintln!("vis run: `--grant {v}` phải dạng <tên>=<đường-dẫn>");
                    return ExitCode::from(2);
                };
                grants.push((ten.to_string(), duong.to_string()));
            }
            // ST-27: in nhật ký quyền trước mỗi lần chạy chương trình ngoài.
            "--explain-perms" => explain_perms = true,
            // ST-27: tắt chương trình ngoài ⇒ lệnh lạ `VS0601`.
            "--no-extern" => co_no_extern = true,
            // ST-42: `FsHost` **mock tất định** từ kịch bản JSON (test hermetic).
            "--fs-mock" => {
                if i + 1 >= rest.len() {
                    eprintln!("vis run: `--fs-mock` cần đường dẫn tệp kịch bản");
                    return ExitCode::from(2);
                }
                i += 1;
                mock_tep = Some(rest[i].clone());
            }
            "--" => {
                prog_args = rest[i + 1..].to_vec();
                break;
            }
            a if a.starts_with('-') => {
                eprintln!("vis run: không hỗ trợ cờ `{a}`");
                return ExitCode::from(2);
            }
            a => {
                if tep.is_some() {
                    eprintln!("vis run: chỉ nhận **một** tệp");
                    return ExitCode::from(2);
                }
                tep = Some(a.to_string());
            }
        }
        i += 1;
    }
    let Some(tep) = tep else {
        eprintln!(
            "dùng: vis run [--tap] [--vm] [--dump-bytecode] [--dump-deps] [--dry] [--sandbox] [--sandbox-check] [--strict-caps] [--no-strict-caps] [--grant tên=path] [--no-cache] <tệp.vis> [-- args…]\n  --strict-caps: mọi truy cập `std.fs`/`net` và tiến trình ngoài (`cap[Proc]`) phải nằm trong `cap` đã khai (mặc định: không siết, **trừ** khi chương trình dùng gói `use pkg.*` ⇒ siết — VD-124)\n  --no-strict-caps: thoát tường minh chế độ siết khi dùng gói; in **một** dòng cảnh báo ra stderr"
        );
        return ExitCode::from(2);
    };
    // **ST-130 (VD-124)**: hai cờ mâu thuẫn nhau ⇒ lỗi rõ, không đoán ý người dùng.
    if strict_caps && no_strict_caps {
        eprintln!("vis run: `--strict-caps` và `--no-strict-caps` mâu thuẫn nhau");
        return ExitCode::from(2);
    }
    // ST-18: `.vish` chạy được (parse → `cmd_lower` → tyck → interp), không còn `Bail out!`.
    let fs_mock: Option<fsmock::FsMock> = match &mock_tep {
        None => None,
        Some(p) => match std::fs::read_to_string(p) {
            Ok(s) => match fsmock::FsMock::tu_json(&s) {
                Ok(m) => Some(m),
                Err(e) => {
                    eprintln!("vis run: {e}");
                    return ExitCode::from(2);
                }
            },
            Err(e) => {
                eprintln!("vis run: không đọc được kịch bản `--fs-mock` `{p}`: {e}");
                return ExitCode::from(2);
            }
        },
    };
    let che_do = if tep.ends_with(".vish") {
        vis_syntax::Mode::Command
    } else {
        vis_syntax::Mode::Script
    };
    let text = match std::fs::read_to_string(&tep) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("vis run: không đọc được `{tep}`: {e}");
            return ExitCode::from(2);
        }
    };
    // **ST-73 (VD-107)**: `--sandbox-check` — probe Landlock **chỉ đọc** + mức cưỡng chế runtime;
    // in **stdout**, exit 0/3/4, **không** chạy chương trình.
    if sandbox_check {
        let caps = quyen::ds_cap(&text, che_do);
        return in_sandbox_check(&caps);
    }
    // **ST-72 (VD-106 (2))**: `--dry` — chỉ in quyền từ CST của `main`, exit 0, **không** chạy.
    if dry {
        return run_dry(&tep, &text, che_do);
    }
    // **ST-72b (VD-103/VD-106)**: quyền theo `cap` của `main` + `--grant`.
    // **ST-130 (VD-124)**: khối `quyen::tao` **dời xuống sau** `modules::load_with` — chế độ siết mặc
    // định phụ thuộc **đồ thị import** (có `pkg.*` hay không).
    let ds_cap = quyen::ds_cap(&text, che_do);
    let goc = std::env::current_dir().unwrap_or_default();
    if tap {
        eprintln!("TAP version 13");
        eprintln!("1..1");
    }
    // **ST-87b (VD-109)**: **BỎ fast-path cache một tệp** — nó đọc khoá v1 (`cache::khoa`, không có
    // `rel`/deps) trong khi mọi chỗ ghi dùng khoá v2 (`cache::khoa_multi`) ⇒ **hit = 0 lần**, chỉ là
    // nhánh chết. Đường duy nhất: nạp module → frontend → `khoa_multi` (cache hit bỏ codegen). Đo:
    // tệp nhỏ lần 2 **~12 ms** (frontend ~1 ms), không hồi quy.
    // 1–3) nạp **module** (ST-25): đa tệp, `use`, cấm vòng, chẩn đoán đúng tên tệp.
    let loaded = match modules::load_with(&tep, !co_no_extern) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("vis run: {e}");
            return ExitCode::from(2);
        }
    };
    if dump_deps {
        eprint!("{}", modules::dump_deps(&loaded));
    }
    if !loaded.diags.is_empty() {
        for (rel, d) in &loaded.diags {
            eprintln!(
                "{rel}:{}:{}: error {}: {}",
                d.line, d.col, d.code.short, d.message
            );
        }
        if tap {
            let ma = loaded
                .diags
                .first()
                .map(|(_, d)| d.code.short)
                .unwrap_or("VS0201");
            eprintln!("not ok 1 - {} ({ma})", tep);
        }
        return ExitCode::from(2);
    }
    // **ST-130 (VD-124)**: chương trình **dùng gói** (`use pkg.*`) ⇒ strict-caps **mặc định** bật;
    // thoát bằng `--no-strict-caps` (một dòng cảnh báo ra **stderr**, không ra stdout).
    let co_pkg = loaded.files.iter().any(|f| f.rel.starts_with("pkg/"));
    let siet = strict_caps || (co_pkg && !no_strict_caps);
    if co_pkg && no_strict_caps && !strict_caps {
        eprintln!(
            "vis run: cảnh báo: chương trình dùng gói nhưng chạy với `--no-strict-caps` — KHÔNG siết quyền (VD-124)"
        );
    }
    let quyen = match quyen::tao(&ds_cap, &grants, &goc, siet) {
        Ok(q) => q,
        Err(e) => {
            eprintln!("vis run: {e}");
            return ExitCode::from(2);
        }
    };
    let mut h = vis_types::HirOutput {
        hir: loaded.hir,
        diagnostics: Vec::new(),
        symbols: loaded.symbols,
        types: loaded.types,
        error: None,
    };
    // **ST-93 (VD-111)**: trong `.vish`, **pipeline cấp tệp** là chương trình; `fn main` (nếu có)
    // chỉ **khai `cap`** — không được "nuốt" pipeline. Chỉ bỏ `main` khi có **cả** câu lệnh cấp tệp;
    // `.vish` chỉ có `main` vẫn chạy `main` như cũ. Cả interp + VM nhận **cùng** HIR đã lọc.
    if che_do == vis_syntax::Mode::Command
        && !h.hir.stmts.is_empty()
        && h.hir
            .items
            .iter()
            .any(|it| matches!(it, vis_types::HItem::Fn(f) if f.name == "main"))
    {
        h.hir
            .items
            .retain(|it| !matches!(it, vis_types::HItem::Fn(f) if f.name == "main"));
    }
    // ST-54: chẩn đoán front end (parse/hạ/tyck) cho **cửa codegen** — defense-in-depth.
    let chan_doan: Vec<vis_syntax::Diagnostic> =
        loaded.diags.iter().map(|(_, d)| d.clone()).collect();
    // **ST-132 (VD-120)**: `--sandbox` — áp ruleset Landlock lên luồng này (luồng chạy kế tiếp thừa
    // hưởng) **sau** khi đã nạp xong chương trình/module, **trước** khi chạy. Tắt cache: trong
    // domain, ghi/đọc thư mục cache nằm ngoài `cap` ⇒ luôn trượt, bật chỉ tổ nhiễu.
    if sandbox {
        if let Err(ma) = cli::run::ap_sandbox(&quyen) {
            return ma;
        }
        dung_cache = false;
    }
    // 4) chạy — **trong luồng có stack lớn** (ST-15): đệ quy sâu (đến cap `MAX_CALL_DEPTH` = 10^4)
    //    không được làm tràn stack process; cap vượt ⇒ `VS0501` bình thường.
    // **ST-56b (VD-97 (5))**: phần tử thứ 4 = **lỗi codegen** (đường `--vm`) ⇒ exit ≠ 0.
    let (stdout, diags, steps, loi_codegen) = std::thread::scope(|scope| {
        std::thread::Builder::new()
            .name("vis-run".into())
            // 256 MiB: đo được ~8 KiB/khung ⇒ 10^4 khung ≈ 80 MiB, còn dư.
            .stack_size(256 * 1024 * 1024)
            .spawn_scoped(scope, || {
                let out = if dung_vm {
                    // ST-20: đường **VM** — cùng `Show`/builtin/bảng ngữ nghĩa với interp.
                    // ST-23: cache theo hash closure; **luôn** nạp qua `load_verified`.
                    let khoa = cache::khoa_multi(&loaded.files, che_do);
                    let tu_cache = if dung_cache { cache::nap(&khoa) } else { None };
                    let m = match tu_cache {
                        Some(m) => m,
                        None => {
                            match vis_bytecode::compile_kiem(&h.hir, &h.symbols, &text, &chan_doan)
                            {
                                Ok(m) => {
                                    if dung_cache {
                                        let _ = cache::luu(&khoa, &m);
                                    }
                                    m
                                }
                                Err(e) => {
                                    // **ST-56b (VD-97 (5))**: in chẩn đoán **và** báo hiệu
                                    // lỗi để CLI **exit ≠ 0** (trước: in rồi exit 0 — im lặng).
                                    eprintln!("vis run --vm: codegen lỗi: {e}");
                                    return (String::new(), Vec::new(), 0, true);
                                }
                            }
                        }
                    };
                    if dump {
                        eprintln!("{}", vis_vm::dump::dump(&m));
                    }
                    // ST-33/ST-63: `std.fs`/`std.net` đi qua host (chỉ CLI biết OS).
                    // **ST-89 (VD-110)**: `.vish` (trừ `--no-extern`) truyền **cùng** runner Ngoai mà
                    // interp dùng — VM chỉ gọi qua trait, không chạm OS.
                    let mut runner = ngoai::Runner {
                        giai_thich: explain_perms,
                        cho_phep_proc: quyen.cho_phep_proc(),
                        allow_proc: quyen.allow_proc(),
                        ..ngoai::Runner::default()
                    };
                    let co_ngoai = che_do == vis_syntax::Mode::Command && !co_no_extern;
                    let mut fs_tho = HostFs::tao(explain_perms, fs_mock.as_ref());
                    let mut net_tho = HostNet::moi(explain_perms);
                    let mut fs = quyen::FsGuard {
                        trong: &mut fs_tho,
                        quyen: &quyen,
                    };
                    let mut net = quyen::NetGuard {
                        trong: &mut net_tho,
                        quyen: &quyen,
                    };
                    let ngoai_host: Option<&mut dyn vis_interp::builtin::Ngoai> =
                        if co_ngoai { Some(&mut runner) } else { None };
                    vis_vm::run_with_full_hosts_caps(
                        &m,
                        &text,
                        &prog_args,
                        ngoai_host,
                        Some(&mut fs),
                        Some(&mut net),
                        &quyen.gia_tri(),
                    )
                } else if che_do == vis_syntax::Mode::Command && !co_no_extern {
                    // ST-27: `.vish` với runner chương trình ngoài (chỉ CLI biết OS).
                    let mut runner = ngoai::Runner {
                        giai_thich: explain_perms,
                        cho_phep_proc: quyen.cho_phep_proc(),
                        allow_proc: quyen.allow_proc(),
                        ..ngoai::Runner::default()
                    };
                    let mut fs_tho = HostFs::tao(explain_perms, fs_mock.as_ref());
                    let mut net_tho = HostNet::moi(explain_perms);
                    let mut fs = quyen::FsGuard {
                        trong: &mut fs_tho,
                        quyen: &quyen,
                    };
                    let mut net = quyen::NetGuard {
                        trong: &mut net_tho,
                        quyen: &quyen,
                    };
                    vis_interp::run_with_hosts_caps(
                        &text,
                        &h.hir,
                        &h.symbols,
                        &h.types,
                        &prog_args,
                        Some(&mut runner),
                        Some(&mut fs),
                        Some(&mut net),
                        &quyen.gia_tri(),
                    )
                } else {
                    // ST-33: `.vis` cũng có `std.fs` (không có chương trình ngoài).
                    let mut fs_tho = HostFs::tao(explain_perms, fs_mock.as_ref());
                    let mut net_tho = HostNet::moi(explain_perms);
                    let mut fs = quyen::FsGuard {
                        trong: &mut fs_tho,
                        quyen: &quyen,
                    };
                    let mut net = quyen::NetGuard {
                        trong: &mut net_tho,
                        quyen: &quyen,
                    };
                    vis_interp::run_with_hosts_caps(
                        &text,
                        &h.hir,
                        &h.symbols,
                        &h.types,
                        &prog_args,
                        None,
                        Some(&mut fs),
                        Some(&mut net),
                        &quyen.gia_tri(),
                    )
                };
                let d: Vec<(String, u32, u32, String)> = out
                    .diagnostics
                    .iter()
                    .map(|x| (x.code.short.to_string(), x.line, x.col, x.message.clone()))
                    .collect();
                (out.stdout, d, out.steps, false)
            })
            .expect("không tạo được luồng chạy")
            .join()
            .expect("luồng chạy lỗi")
    });
    if loi_codegen {
        if tap {
            eprintln!("not ok 1 - {tep} (codegen)");
        }
        return ExitCode::from(1);
    }
    if !xuat_stdout(&stdout) {
        return ExitCode::from(2);
    }
    if diags.is_empty() {
        if tap {
            eprintln!("ok 1 - {} (steps={})", tep, steps);
        }
        ExitCode::from(0)
    } else {
        for (ma, line, col, msg) in &diags {
            eprintln!("{tep}:{line}:{col}: error {ma}: {msg}");
        }
        if tap {
            eprintln!("not ok 1 - {} ({}, steps={})", tep, diags[0].0, steps);
        }
        ExitCode::from(ma_thoat_khi_loi(&diags))
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        // ST-23: quản lý cache bytecode.
        Some("cache") => match args.get(1).map(String::as_str) {
            Some("--clear") => {
                let n = cache::xoa();
                eprintln!("vis cache: đã xoá {n} tệp");
                ExitCode::from(0)
            }
            Some("--path") | None => {
                match cache::thu_muc() {
                    Some(p) => println!("{}", p.display()),
                    None => eprintln!("vis cache: không xác định được thư mục cache"),
                }
                ExitCode::from(0)
            }
            Some(khac) => {
                eprintln!("vis cache: cờ lạ `{khac}` (dùng --clear | --path)");
                ExitCode::from(2)
            }
        },
        Some("explain") => {
            let mut lang = Lang::Vi;
            let mut list = false;
            let mut ma: Option<String> = None;
            let mut i = 1;
            while i < args.len() {
                match args[i].as_str() {
                    "--list" => list = true,
                    "--lang" => match args.get(i + 1) {
                        Some(v) => match Lang::parse(v) {
                            Some(l) => {
                                lang = l;
                                i += 1; // bỏ qua giá trị ngôn ngữ
                            }
                            None => {
                                eprintln!("vis explain: ngôn ngữ không hỗ trợ `{v}` (vi|en|ja)");
                                return ExitCode::from(2);
                            }
                        },
                        None => {
                            eprintln!("vis explain: `--lang` cần giá trị vi|en|ja");
                            return ExitCode::from(2);
                        }
                    },
                    a if a.starts_with('-') => {
                        eprintln!("vis explain: không hỗ trợ cờ `{a}`");
                        return usage_explain(lang);
                    }
                    a => ma = Some(a.to_string()),
                }
                i += 1;
            }
            if list {
                // `explain --list` phủ **cả ba** sổ đã cấp (01xx + 02xx + 03xx, VD-44 #7).
                for c in all_codes() {
                    println!(
                        "{}  {:<8} {}",
                        c.short,
                        severity_label(c.severity, lang),
                        c.message(lang)
                    );
                }
                return ExitCode::SUCCESS;
            }
            match ma {
                None => usage_explain(lang),
                Some(code) => match lookup_str(&code) {
                    Some(c) => {
                        println!(
                            "{} — {}: {}",
                            c.short,
                            severity_label(c.severity, lang),
                            c.message(lang)
                        );
                        ExitCode::SUCCESS
                    }
                    None => {
                        // VD-20: exit 1 dành cho lỗi từ main; mã sai là **lỗi dùng** ⇒ 2.
                        eprintln!("vis: không có mã {code} trong sổ (SPEC-lex §7)");
                        ExitCode::from(2)
                    }
                },
            }
        }
        Some("run") => run_run(&args[1..]),
        Some("check") => cli::check::run_check(&args[1..]),
        // ST-30: chạy bộ ca `test/sem/**` in TAP.
        Some("test") => vistest::run(&args[1..]),
        // ST-67 (VD-102): server LSP tối thiểu trên stdio (khung + JSON-RPC).
        Some("lsp") => lsp::run(&args[1..]),
        // ST-70 (VD-101): registry **local** cho gói ViScript.
        Some("pkg") => pkg::run(&args[1..]),
        Some("fmt") => cli::fmt::run_fmt(&args[1..]),
        _ => usage(),
    }
}
