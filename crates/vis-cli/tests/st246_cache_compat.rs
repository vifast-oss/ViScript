// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-246 (VD-244 §2)** — bằng chứng cache v1 không được tái dùng dưới ngữ nghĩa v2.
//!
//! Khoá cache (`cache.rs::khoa_multi`) đưa `vis_bytecode::VERSION` vào; khi VERSION tăng `1 → 2`,
//! mọi artifact v1 bị bỏ và **source được biên dịch lại**, chạy đúng trên cả **interp** và **VM**.
//! Ngoài ra, loader dùng `load_verified` ⇒ artifact v1 (dù nằm đúng đường dẫn cache hiện tại) bị từ
//! chối, không bao giờ chạy sai.

use std::path::{Path, PathBuf};
use std::process::Command;

use vis_bytecode::{encode, Const, Func, Instr, Module, Span, HASH_LEN, VERSION};

fn goc() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn thu_muc_cache(tag: &str) -> PathBuf {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static DEM: AtomicUsize = AtomicUsize::new(0);
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-st246-{tag}-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("tmp cache");
    d
}

fn chay(cache: &Path, f: &Path, args: &[&str]) -> (i32, Vec<u8>, Vec<u8>) {
    let o = Command::new(env!("CARGO_BIN_EXE_vis"))
        .current_dir(goc())
        .env("XDG_CACHE_HOME", cache)
        .arg("run")
        .args(args)
        .arg(f)
        .output()
        .expect("chạy vis");
    (o.status.code().unwrap_or(-1), o.stdout, o.stderr)
}

fn tim_vbc(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(ds) = std::fs::read_dir(dir) else {
        return;
    };
    for e in ds.flatten() {
        let p = e.path();
        if p.is_dir() {
            tim_vbc(&p, out);
        } else if p.extension().and_then(|x| x.to_str()) == Some("vbc") {
            out.push(p);
        }
    }
}

fn mot_vbc(cache: &Path) -> PathBuf {
    let mut v = Vec::new();
    tim_vbc(cache, &mut v);
    assert_eq!(v.len(), 1, "cần đúng 1 tệp cache, có {:?}", v);
    v.remove(0)
}

/// Artifact **v1 giả** in `999` (sai) — nếu runtime v2 tái dùng nó, output sẽ là `999\n` thay vì
/// `42\n` của source. Dùng dạng module đã được verifier chấp nhận (`verify.rs::hop_le`), rồi đóng
/// dấu cố định `version = 1` vào byte (bypass `encode`'s `m.version.max(VERSION)`).
fn artifact_v1_sai() -> Vec<u8> {
    let mut m = Module {
        version: VERSION,
        source_hash: [7u8; HASH_LEN],
        consts: vec![
            Const {
                kind: 0,
                text: "999".into(),
            },
            Const {
                kind: 5,
                text: "print".into(),
            },
        ],
        funcs: vec![Func {
            name: "main".into(),
            arity: 0,
            nparams: 0,
            nlocals: 1,
            max_stack: 2,
            code: vec![Instr::Const(0), Instr::CallBuiltin(1, 1), Instr::Return],
            spans: vec![Span { start: 0, end: 0 }; 3],
        }],
        entry: 0,
    };
    assert_eq!(
        vis_bytecode::verify(&m),
        Ok(()),
        "dạng module mẫu phải hợp lệ"
    );
    let mut b = encode(&m);
    assert_eq!(&b[..4], vis_bytecode::MAGIC);
    b[4..6].copy_from_slice(&1u16.to_le_bytes()); // artifact v1
    m.version = 1;
    assert_eq!(
        vis_bytecode::verify(&m),
        Err(vis_bytecode::VerifyError::BadVersion(1))
    );
    b
}

/// **Bằng chứng (VD-244 §2)**: cache v1 bị bỏ; source được biên dịch lại và chạy đúng trên
/// **VM** lẫn **interp**.
#[test]
fn cache_v1_bi_bo_source_bien_dich_lai_chay_dung_hai_engine() {
    assert_eq!(VERSION, 2, "giả định VERSION = 2 sau ST-246");

    let cache = thu_muc_cache("e2e");
    let src = cache.join("prog.vis");
    std::fs::write(&src, "print(42)\n").expect("ghi source");

    // (1) Lần đầu: biên dịch từ source, ghi artifact v2 vào cache, VM in đúng 42.
    let (c1, o1, e1) = chay(&cache, &src, &["--vm"]);
    assert_eq!(c1, 0, "lần đầu: {e1:?}");
    assert_eq!(o1, b"42\n", "lần đầu VM");
    let p = mot_vbc(&cache);
    let b1 = std::fs::read(&p).expect("doc vbc");
    assert_eq!(
        u16::from_le_bytes([b1[4], b1[5]]),
        2,
        "artifact vừa ghi phải là v2"
    );

    // (2) Đặt artifact v1 **in sai 999** đúng vào đường dẫn cache hiện tại. Nếu loader không kiểm
    //     VERSION, lần chạy sau sẽ in 999.
    std::fs::write(&p, artifact_v1_sai()).expect("ghi artifact v1 vào cache");
    let b1 = std::fs::read(&p).expect("doc");
    assert_eq!(u16::from_le_bytes([b1[4], b1[5]]), 1, "đã cài artifact v1");

    // (3) Lần hai: v1 bị từ chối ⇒ source được biên dịch lại ⇒ vẫn in 42, cache được ghi đè v2.
    let (c2, o2, e2) = chay(&cache, &src, &["--vm"]);
    assert_eq!(c2, 0, "lần hai: {e2:?}");
    assert_eq!(
        o2, b"42\n",
        "cache v1 KHÔNG được tái dùng (nếu có sẽ ra 999)"
    );
    assert_ne!(o2, b"999\n", "phát hiện tái dùng artifact v1");
    let p2 = mot_vbc(&cache);
    assert_eq!(p2, p, "vẫn một đường dẫn cache");
    let b2 = std::fs::read(&p2).expect("doc vbc sau recompile");
    assert_eq!(
        u16::from_le_bytes([b2[4], b2[5]]),
        2,
        "cache phải được ghi lại dưới VERSION = 2"
    );

    // (4) Cùng source chạy đúng trên **interp** và **VM --no-cache** (hai engine, không cache).
    let (ci, oi, ei) = chay(&cache, &src, &[]);
    assert_eq!(ci, 0, "interp: {ei:?}");
    assert_eq!(oi, b"42\n", "interp phải in đúng 42");

    let (cv, ov, ev) = chay(&cache, &src, &["--vm", "--no-cache"]);
    assert_eq!(cv, 0, "vm no-cache: {ev:?}");
    assert_eq!(ov, b"42\n", "VM no-cache phải in đúng 42");
}

/// Khoá cache thực sự đưa `VERSION` vào (kiểm chứng cơ chế ở mức nguồn, bổ sung cho unit test
/// `cache.rs::khoa_cache_v1_khong_duoc_tai_dung_v2`).
#[test]
fn khoa_cache_co_version_trong_nguon() {
    let s = std::fs::read_to_string(goc().join("crates/vis-cli/src/cache.rs")).expect("cache.rs");
    assert!(
        s.contains("vis_bytecode::VERSION"),
        "khoa_multi phải lấy VERSION từ vis_bytecode"
    );
    assert!(
        s.contains("khoa_multi_voi_version"),
        "phải có đường tính khoá theo VERSION tường minh để test"
    );
}
