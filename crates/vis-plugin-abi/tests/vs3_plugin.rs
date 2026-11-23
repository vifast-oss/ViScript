// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-307 (VD-278/VD-281/VD-283)** — plugin động WASI/WASM: ABI, sandbox `cap[...]`,
//! tắt cứng khi `--strict-caps`/`--sandbox`, parity interp↔VM qua runtime thật.
//!
//! Plugin mẫu `test/plugin-wasi/mau.rs` được dựng bằng `rustc --target wasm32-wasip1` (không
//! thêm crate, không tải gì). Runtime dò `wasmtime` → Python `wasmtime` → Node `node:wasi`;
//! thiếu cả ba ⇒ in `BLOCKED-ENV` (cùng quy ước ST-299a/b), không giả vờ đã chạy.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

use vis_plugin_abi::{Cap, ChinhSach, PluginValue, PluginWasi};

fn goc_repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn thu_muc_tam(ten: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("vis-plugin-wasi-{ten}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("tmp");
    d
}

/// Tệp chạy được: nhận đúng đường dẫn, hoặc thêm `.exe` trên Windows (`rustc` → `rustc.exe`).
fn tep_rustc(p: &Path) -> Option<PathBuf> {
    if p.is_file() {
        return Some(p.to_path_buf());
    }
    #[cfg(windows)]
    {
        let exe = p.with_extension("exe");
        if exe.is_file() {
            return Some(exe);
        }
    }
    None
}

/// Dò `rustc`: **ưu tiên `VIS_RUSTC`**, rồi `CARGO_HOME/bin`, `$HOME/.cargo/bin`,
/// `%USERPROFILE%\.cargo\bin`, cuối cùng `PATH` — chấp nhận cả `rustc` lẫn `rustc.exe`
/// (Windows quick ST-307 chỉ có `rustc.exe` nên bản cũ trả `None`).
fn tim_rustc() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os("VIS_RUSTC") {
        let p = PathBuf::from(p);
        if let Some(t) = tep_rustc(&p) {
            return Some(t);
        }
        eprintln!(
            "BLOCKED-ENV: VIS_RUSTC=`{}` không phải tệp chạy được — dò tiếp",
            p.display()
        );
    }
    for bien in ["CARGO_HOME", "HOME", "USERPROFILE"] {
        if let Some(goc) = std::env::var_os(bien) {
            let goc = PathBuf::from(goc);
            let bin = if bien == "CARGO_HOME" {
                goc.join("bin")
            } else {
                goc.join(".cargo").join("bin")
            };
            for ten in ["rustc", "rustc.exe"] {
                if let Some(t) = tep_rustc(&bin.join(ten)) {
                    return Some(t);
                }
            }
        }
    }
    let path = std::env::var_os("PATH")?;
    for d in std::env::split_paths(&path) {
        for ten in ["rustc", "rustc.exe"] {
            if let Some(t) = tep_rustc(&d.join(ten)) {
                return Some(t);
            }
        }
    }
    None
}

/// `true` nếu `rustc` gọi được qua `PATH` (nguồn thứ hai, độc lập với [`tim_rustc`]).
fn goi_duoc_rustc() -> bool {
    Command::new("rustc")
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Windows/macOS có `rustc` ⇒ máy đủ toolchain, **phải chạy WASI thật**; không được "xanh giả"
/// bằng cách bỏ qua. Linux vẫn cho phép `BLOCKED-ENV` khi container thiếu runtime/target.
fn bat_buoc_chay_that() -> bool {
    cfg!(any(windows, target_os = "macos")) && goi_duoc_rustc()
}

/// Thiếu môi trường: in `BLOCKED-ENV`; trên Windows/macOS có `rustc` thì **panic** thay vì
/// trả về âm thầm (bài học Windows quick ST-307: 4 ca bị bỏ qua mà vẫn báo pass).
fn thieu_moi_truong(ly_do: &str) {
    if bat_buoc_chay_that() {
        panic!(
            "BLOCKED-ENV: {ly_do} — {} có rustc nên test phải chạy WASI thật, không được bỏ qua",
            std::env::consts::OS
        );
    }
    eprintln!("BLOCKED-ENV: {ly_do}");
}

/// Dựng (một lần) plugin mẫu; `None` ⇒ môi trường thiếu toolchain wasm ⇒ `BLOCKED-ENV`.
fn plugin_mau() -> Option<&'static Path> {
    static MOT_LAN: OnceLock<Option<PathBuf>> = OnceLock::new();
    MOT_LAN
        .get_or_init(|| {
            let goc = goc_repo();
            let src = goc.join("test/plugin-wasi/mau.rs");
            let out = goc.join("target/plugin-wasi/mau.wasm");
            let moi_hon = || -> bool {
                let (Ok(a), Ok(b)) = (std::fs::metadata(&out), std::fs::metadata(&src)) else {
                    return false;
                };
                match (a.modified(), b.modified()) {
                    (Ok(a), Ok(b)) => a >= b,
                    _ => false,
                }
            };
            if !moi_hon() {
                std::fs::create_dir_all(out.parent()?).ok()?;
                let rustc = match tim_rustc() {
                    Some(r) => r,
                    None => {
                        thieu_moi_truong(
                            "không tìm thấy rustc (đã dò VIS_RUSTC, CARGO_HOME/bin, \
                             ~/.cargo/bin, USERPROFILE/.cargo/bin, PATH kể cả rustc.exe)",
                        );
                        return None;
                    }
                };
                let st = match Command::new(rustc)
                    .args(["--target", "wasm32-wasip1", "-O", "--edition", "2021"])
                    .arg(&src)
                    .arg("-o")
                    .arg(&out)
                    .current_dir(&goc)
                    .status()
                {
                    Ok(st) => st,
                    Err(e) => {
                        thieu_moi_truong(&format!("không chạy được rustc: {e}"));
                        return None;
                    }
                };
                if !st.success() || !out.is_file() {
                    thieu_moi_truong(
                        "rustc không dựng được wasm32-wasip1 cho plugin mẫu \
                         (cài `rustup target add wasm32-wasip1`)",
                    );
                    return None;
                }
            }
            Some(out)
        })
        .as_deref()
}

fn co_runtime() -> bool {
    vis_plugin_abi::wasi::tim_runtime().is_ok()
}

fn nap_mau(caps: Vec<Cap>) -> PluginWasi {
    let wasm = plugin_mau().expect("có plugin mẫu");
    PluginWasi::nap(wasm, caps, ChinhSach::moi()).expect("nạp plugin WASI")
}

#[test]
fn nap_plugin_va_goi_cong() {
    if !co_runtime() {
        thieu_moi_truong("không có runtime WASI (wasmtime/python-wasmtime/node)");
        return;
    }
    let Some(_) = plugin_mau() else {
        thieu_moi_truong("không dựng được wasm32-wasip1 cho plugin mẫu");
        return;
    };
    let p = nap_mau(Vec::new());
    assert_eq!(p.manifest().module, "mau");
    assert_eq!(p.manifest().abi, 1);
    assert_eq!(p.manifest().tim("cong").map(|h| h.params.len()), Some(2));
    assert!(
        matches!(
            p.runtime(),
            vis_plugin_abi::Runtime::Wasmtime(_)
                | vis_plugin_abi::Runtime::Python(_)
                | vis_plugin_abi::Runtime::Node(_)
        ),
        "runtime: {:?}",
        p.runtime()
    );
    let kq = p
        .goi("cong", &[PluginValue::Int(2), PluginValue::Int(3)])
        .expect("gọi cong");
    assert_eq!(kq, PluginValue::Int(5));
}

#[test]
fn parity_interp_vm_qua_wasi() {
    if !co_runtime() || plugin_mau().is_none() {
        thieu_moi_truong("thiếu runtime WASI hoặc toolchain wasm32-wasip1");
        return;
    }
    let src = "fn cong(a: int, b: int) -> int:\n    a + b\n\nfn main() -> int:\n    cong(2, 3)\n";
    let p = vis_syntax::parse(src, vis_syntax::Mode::Script);
    assert!(p.diagnostics.is_empty(), "parse: {:?}", p.diagnostics);
    let l = vis_syntax::lex(src, vis_syntax::Mode::Script);
    let h = vis_types::lower_with(
        &p.cst,
        &l,
        vis_types::LowerOptions {
            std_mods: vec!["fs".into(), "str".into()],
            ..Default::default()
        },
    );
    assert!(h.is_clean(), "lower: {:?}", h.diagnostics);
    let t = vis_types::check(&h.hir, &h.symbols, &h.types);
    assert!(t.is_clean(), "tyck: {:?}", t.diags);
    let i = vis_interp::run(src, &h.hir, &h.symbols, &h.types, &[]);
    let m = vis_bytecode::compile(&h.hir, &h.symbols, src).expect("compile");
    let v = vis_vm::run(&m, src, &[]);
    assert!(i.is_clean(), "interp: {:?}", i.diagnostics);
    assert!(v.is_clean(), "vm: {:?}", v.diagnostics);
    let iv = PluginValue::try_from(i.value.clone().expect("interp value")).expect("interp ABI");
    let vv = PluginValue::try_from(v.value.clone().expect("vm value")).expect("vm ABI");
    assert_eq!(iv, vv, "interp và VM phải cùng giá trị");

    let plugin = nap_mau(Vec::new());
    let kq = plugin
        .goi("cong", &[PluginValue::Int(2), PluginValue::Int(3)])
        .expect("gọi cong");
    assert_eq!(kq, iv);
    assert_eq!(kq, vv);
    assert_eq!(kq, PluginValue::Int(5));
}

#[test]
fn cap_dir_la_sandbox_tu_nhien() {
    if !co_runtime() || plugin_mau().is_none() {
        thieu_moi_truong("thiếu runtime WASI hoặc toolchain wasm32-wasip1");
        return;
    }
    let d = thu_muc_tam("cap");
    let f = d.join("in.txt");
    std::fs::write(&f, "xin chào wasi").expect("fixture");
    let guest = "/cap0/in.txt".to_string();

    // Không cap ⇒ plugin không thấy preopen nào.
    let khong = nap_mau(Vec::new());
    let e = khong
        .goi("doc", &[PluginValue::Str(guest.clone())])
        .expect_err("thiếu cap phải bị từ chối");
    assert_eq!(e.code, "VS0604", "{e}");

    // cap chỉ đọc ⇒ đọc được, ghi bị từ chối.
    let doc = nap_mau(vec![Cap::dir(&d, false)]);
    let kq = doc
        .goi("doc", &[PluginValue::Str(guest.clone())])
        .expect("đọc trong cap");
    assert_eq!(kq, PluginValue::Str("xin chào wasi".into()));
    let e = doc
        .goi(
            "ghi",
            &[
                PluginValue::Str(guest.clone()),
                PluginValue::Str("mới".into()),
            ],
        )
        .expect_err("cap chỉ đọc không được ghi");
    assert_eq!(e.code, "VS0604", "{e}");
    assert_eq!(
        std::fs::read_to_string(&f).expect("fixture còn"),
        "xin chào wasi"
    );

    // cap ghi ⇒ ghi được.
    let ghi = nap_mau(vec![Cap::dir(&d, true)]);
    let kq = ghi
        .goi(
            "ghi",
            &[
                PluginValue::Str(guest.clone()),
                PluginValue::Str("đã ghi".into()),
            ],
        )
        .expect("ghi trong cap");
    assert_eq!(kq, PluginValue::Unit);
    assert_eq!(std::fs::read_to_string(&f).expect("đọc lại"), "đã ghi");

    // Ngoài preopen: dùng tệp **THẬT** nằm ngoài cap. Không lấy `/etc/passwd` làm fixture
    // chính vì Windows không có tệp đó ⇒ ENOENT không chứng minh được sandbox.
    let f_ngoai =
        std::env::temp_dir().join(format!("vis-plugin-wasi-ngoai-{}.txt", std::process::id()));
    std::fs::write(&f_ngoai, "ngoài cap").expect("fixture ngoài cap");
    let e = ghi
        .goi(
            "doc",
            &[PluginValue::Str(f_ngoai.to_string_lossy().into_owned())],
        )
        .expect_err("tệp thật ngoài cap phải bị từ chối");
    assert_eq!(e.code, "VS0604", "{e}");
    assert!(e.message.contains("không đọc được"), "{e}");
    assert!(e.message.contains("os error"), "{e}");
    assert_eq!(
        std::fs::read_to_string(&f_ngoai).expect("host vẫn đọc được tệp ngoài cap"),
        "ngoài cap",
        "tệp ngoài cap phải tồn tại — bị từ chối là do sandbox, không phải ENOENT"
    );

    // Thoát cap bằng `..` tới đúng tệp thật đó cũng phải bị WASI từ chối.
    let ten_ngoai = f_ngoai
        .file_name()
        .expect("tên tệp ngoài cap")
        .to_string_lossy()
        .into_owned();
    let e = ghi
        .goi("doc", &[PluginValue::Str(format!("/cap0/../{ten_ngoai}"))])
        .expect_err("`..` ra ngoài cap phải bị từ chối");
    assert_eq!(e.code, "VS0604", "{e}");
    assert!(e.message.contains("os error"), "{e}");

    // Linux/macOS: `/etc/passwd` tồn tại thật nhưng ngoài preopen ⇒ vẫn bị từ chối.
    #[cfg(unix)]
    {
        assert!(Path::new("/etc/passwd").is_file());
        let e = ghi
            .goi("doc", &[PluginValue::Str("/etc/passwd".into())])
            .expect_err("/etc/passwd ngoài cap phải bị từ chối");
        assert_eq!(e.code, "VS0604", "{e}");
    }
}

#[test]
fn tat_cung_khi_strict_caps_hoac_sandbox() {
    let khong_ton_tai = std::env::temp_dir().join("vis-plugin-khong-ton-tai-307.wasm");
    for (cs, tu) in [
        (ChinhSach::tu_co(true, false), "strict-caps"),
        (ChinhSach::tu_co(false, true), "sandbox"),
        (ChinhSach::tu_co(true, true), "strict-caps"),
    ] {
        let e = PluginWasi::nap(&khong_ton_tai, Vec::new(), cs)
            .expect_err("chế độ hạn chế phải tắt plugin động");
        assert_eq!(e.code, "VS0604", "{e}");
        assert!(e.message.contains(tu), "thông điệp phải nêu `{tu}`: {e}");
        assert!(
            !e.message.contains("không đọc được"),
            "phải chặn TRƯỚC khi đọc tệp: {e}"
        );
    }
    assert!(!ChinhSach::moi().tat_plugin_dong());
    assert!(ChinhSach::tu_co(true, false).tat_plugin_dong());
    assert!(ChinhSach::tu_co(false, true).tat_plugin_dong());
}

#[test]
fn tu_choi_abi_lech_va_plugin_la() {
    if !co_runtime() || plugin_mau().is_none() {
        thieu_moi_truong("thiếu runtime WASI hoặc toolchain wasm32-wasip1");
        return;
    }
    let d = thu_muc_tam("abi");
    let goc = std::fs::read(plugin_mau().expect("plugin mẫu")).expect("đọc wasm");

    // Đổi marker `vis_plugin_abi_v1` → `v2` (cùng độ dài, section không hỏng).
    let mut sai = goc.clone();
    let marker = b"vis_plugin_abi_v1";
    let pos = sai
        .windows(marker.len())
        .position(|w| w == marker)
        .expect("wasm mẫu có marker");
    let cuoi = pos + marker.len() - 1;
    assert_eq!(sai[cuoi], b'1');
    sai[cuoi] = b'2';
    let p_sai = d.join("abi2.wasm");
    std::fs::write(&p_sai, &sai).expect("ghi wasm lệch ABI");
    let e = PluginWasi::nap(&p_sai, Vec::new(), ChinhSach::moi())
        .expect_err("ABI lệch phải bị từ chối");
    assert_eq!(e.code, "VS0501", "{e}");
    assert!(e.message.contains("ABI") && e.message.contains('2'), "{e}");

    // Tệp không phải wasm.
    let p_la = d.join("la.bin");
    std::fs::write(&p_la, b"khong-phai-wasm").expect("ghi tệp lạ");
    let e =
        PluginWasi::nap(&p_la, Vec::new(), ChinhSach::moi()).expect_err("tệp lạ phải bị từ chối");
    assert_eq!(e.code, "VS0501", "{e}");
    assert!(e.message.contains("wasm"), "{e}");

    // Wasm hợp lệ nhưng không phải WASI command (mất `_start`).
    let mut khong_start = goc.clone();
    let start = b"_start";
    let pos = khong_start
        .windows(start.len())
        .position(|w| w == start)
        .expect("wasm mẫu có _start");
    khong_start[pos + start.len() - 1] = b'x';
    let p_start = d.join("khong-start.wasm");
    std::fs::write(&p_start, &khong_start).expect("ghi wasm thiếu _start");
    let e = PluginWasi::nap(&p_start, Vec::new(), ChinhSach::moi())
        .expect_err("thiếu _start phải bị từ chối");
    assert_eq!(e.code, "VS0501", "{e}");
    assert!(e.message.contains("_start"), "{e}");
}

#[test]
fn codec_gia_tri_tron_vong_va_sai_thi_loi() {
    let ds = vec![
        PluginValue::Unit,
        PluginValue::Bool(true),
        PluginValue::Int(-7),
        PluginValue::Float(1.5),
        PluginValue::Str("xin chào\nwasi".into()),
        PluginValue::Bytes(vec![0, 255, 16]),
        PluginValue::List(vec![
            PluginValue::Int(1),
            PluginValue::Str("a".into()),
            PluginValue::Bool(false),
        ]),
        PluginValue::Map(vec![(PluginValue::Str("k".into()), PluginValue::Int(9))]),
        PluginValue::Option(Some(Box::new(PluginValue::Int(3)))),
        PluginValue::Option(None),
        PluginValue::Result(Ok(Box::new(PluginValue::Str("ok".into())))),
        PluginValue::Cap {
            kind: "Dir".into(),
            root: "/tmp/x".into(),
            write: true,
        },
    ];
    for v in ds {
        let s = vis_plugin_abi::wasi::ma_hoa(&v);
        let lai = vis_plugin_abi::wasi::giai_ma(&s).expect("giải mã");
        assert_eq!(lai, v, "codec: {s}");
    }
    for sai in ["", "x:", "i:abc:", "s:2:61", "l:2:i:1:", "c:1:44"] {
        assert!(
            vis_plugin_abi::wasi::giai_ma(sai).is_err(),
            "phải lỗi: `{sai}`"
        );
    }
}
