// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-122 (VD-127)** — span **tổng hợp** phải mang `file` của tệp sinh ra nó: ca **quan sát được**
//! qua ba đường của sản phẩm (`check --json`, `check` TAP, `run` + `run --vm`).
//!
//! Mỗi ca là **một cây đa tệp** dưới `test/modules/span/<ca>/`: `main.vis` (tệp chính, `file_id = 0`)
//! `use ./mod`, còn `mod.vish` (module, `file_id = 1`) chứa cấu trúc mà **lower phải sinh node**:
//! thân `cmd`, nhóm `( … )`, closure `x => …`, vị từ ngầm `it`, item `.vish`. Trước ST-122 cả năm ca
//! đều bị báo **sang tệp chính** (`main.vis:4:1`); nay phải đúng `mod.vish:<dòng>:<cột>`.
//!
//! Vì sao **không** đặt fixture trong `test/sem/…`: ba harness `sem.rs`/`diag_cmp.rs`/`vs_interp.rs`
//! chỉ quét **một tầng** của `test/sem/<nhóm>/` và khoá cứng **447** ca, còn `vistest.rs::gom` thì
//! **đệ quy** ⇒ tệp module trong `test/sem/**` sẽ bị `vis test` chạy như một ca độc lập (đỏ). Fixture
//! đa tệp ở `test/modules/` là chỗ duy nhất không làm lệch `sem`/`vis test`/parity.
//!
//! Audit **toàn bộ** HIR (không hardcode danh sách tệp) nằm ở unit test của bin crate:
//! `crates/vis-cli/src/audit_span.rs` (cần `modules::load`, mà integration test không thấy được).

use std::path::PathBuf;
use std::process::Command;

/// `(tên ca, mã, dòng, cột)` — vị trí **đúng** trong `mod.vish`. Số này đối chiếu độc lập với chạy
/// **một tệp** cùng nội dung (`mod.vish` chép ra tệp tạm) — xem `da_tep_khop_tep_don`.
const CA: [(&str, &str, u32, u32); 5] = [
    ("cmd-than", "VS0412", 3, 5),
    ("where-closure", "VS0412", 3, 26),
    ("where-it", "VS0412", 3, 67),
    ("nhom", "VS0412", 3, 7),
    ("item-khai", "VS0412", 3, 5),
];

fn goc() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../")
}

fn tep(ca: &str, ten: &str) -> String {
    goc()
        .join("test/modules/span")
        .join(ca)
        .join(ten)
        .to_string_lossy()
        .to_string()
}

struct Kq {
    code: i32,
    out: String,
    err: String,
}

fn chay(args: &[&str]) -> Kq {
    let o = Command::new(env!("CARGO_BIN_EXE_vis"))
        .args(args)
        .output()
        .expect("chạy được `vis`");
    Kq {
        code: o.status.code().unwrap_or(-1),
        out: String::from_utf8_lossy(&o.stdout).to_string(),
        err: String::from_utf8_lossy(&o.stderr).to_string(),
    }
}

// ----- 1. `check --json`: đúng `file` + `line` + `col` -----

#[test]
fn json_bao_dung_tep_va_dong_cot_moi_ca() {
    let mut sai: Vec<String> = Vec::new();
    for (ca, ma, dong, cot) in CA {
        let main = tep(ca, "main.vis");
        let k = chay(&["check", "--json", &main]);
        // Chẩn đoán phải nằm ở **mod.vish** (module, `file_id = 1`) với `line`/`col` thật; trước ST-122
        // nó nằm ở `main.vis` với dòng:cột vô nghĩa (`4:1`).
        let mong = format!(
            "{{\"file\":\"mod.vish\",\"diagnostics\":[{{\"code\":\"{ma}\",\"line\":{dong},\"col\":{cot},"
        );
        if !k.out.contains(&mong) {
            sai.push(format!(
                "{ca}: JSON thiếu `{mong}` — thật: {}",
                k.out.trim()
            ));
        }
        // Tệp chính **không** được có chẩn đoán (đây chính là lỗi cũ: node module bị quy sang tệp chính).
        let chinh = format!("{{\"file\":\"{main}\",\"diagnostics\":[]}}");
        if !k.out.contains(&chinh) {
            sai.push(format!(
                "{ca}: JSON phải có `{chinh}` — thật: {}",
                k.out.trim()
            ));
        }
        if k.code != 1 {
            sai.push(format!("{ca}: `check --json` phải exit 1, thật {}", k.code));
        }
    }
    assert!(sai.is_empty(), "{} ca sai:\n{}", sai.len(), sai.join("\n"));
}

// ----- 2. TAP: đúng `tệp:dòng:cột` ở cả dòng `not ok` và dòng chẩn đoán -----

#[test]
fn tap_bao_dung_tep_va_dong_cot_moi_ca() {
    let mut sai: Vec<String> = Vec::new();
    for (ca, ma, dong, cot) in CA {
        let main = tep(ca, "main.vis");
        let k = chay(&["check", &main]);
        if k.code != 1 {
            sai.push(format!("{ca}: `check` phải exit 1, thật {}", k.code));
        }
        let not_ok = format!("not ok 1 - {main} (mod.vish:{dong}:{cot})");
        if !k.out.contains(&not_ok) {
            sai.push(format!(
                "{ca}: TAP phải có `{not_ok}` — thật: {}",
                k.out.trim()
            ));
        }
        let dong_ma = format!("# {ma} {dong}:{cot} ");
        if !k.out.contains(&dong_ma) {
            sai.push(format!(
                "{ca}: TAP phải có `{dong_ma}` — thật: {}",
                k.out.trim()
            ));
        }
        // Không được báo sang tệp chính (lỗi cũ).
        if k.out.contains(&format!("({main}:")) || k.out.contains("(main.vis:") {
            sai.push(format!("{ca}: TAP báo **sai tệp**: {}", k.out.trim()));
        }
    }
    assert!(sai.is_empty(), "{} ca sai:\n{}", sai.len(), sai.join("\n"));
}

// ----- 3. `run` (interp) và `run --vm`: cùng mã, cùng tệp:dòng:cột, cùng exit -----

#[test]
fn run_interp_va_vm_bao_dung_tep_dong_cot() {
    let mut sai: Vec<String> = Vec::new();
    for (ca, ma, dong, cot) in CA {
        let main = tep(ca, "main.vis");
        let a = chay(&["run", &main]);
        let b = chay(&["run", "--vm", "--no-cache", &main]);
        let mong = format!("mod.vish:{dong}:{cot}: error {ma}:");
        for (nhan, k) in [("run", &a), ("run --vm", &b)] {
            if k.code != 2 {
                sai.push(format!("{ca}/{nhan}: phải exit 2, thật {}", k.code));
            }
            if !k.err.contains(&mong) {
                sai.push(format!(
                    "{ca}/{nhan}: stderr phải có `{mong}` — thật: {}",
                    k.err.trim()
                ));
            }
            if k.err.contains("main.vis:") {
                sai.push(format!(
                    "{ca}/{nhan}: stderr báo **sai tệp**: {}",
                    k.err.trim()
                ));
            }
        }
        // Hai đường phải **giống nhau** (interp ↔ VM), đúng phân hoạch của `diag_cmp`.
        if a.err != b.err {
            sai.push(format!("{ca}: run ≠ run --vm: {:?} vs {:?}", a.err, b.err));
        }
    }
    assert!(sai.is_empty(), "{} ca sai:\n{}", sai.len(), sai.join("\n"));
}

// ----- 4. Đa tệp **không** đổi `dòng:cột`: chép `mod.vish` ra tệp đơn phải ra **cùng** vị trí -----

#[test]
fn da_tep_khop_tep_don() {
    let tmp = std::env::temp_dir().join(format!("vis-span-tep-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).expect("thư mục tạm");
    let mut sai: Vec<String> = Vec::new();
    for (ca, ma, dong, cot) in CA {
        let noi_dung = std::fs::read_to_string(tep(ca, "mod.vish")).expect("đọc mod.vish");
        let don = tmp.join(format!("{ca}.vish"));
        std::fs::write(&don, &noi_dung).expect("ghi");
        let k = chay(&["check", "--json", &don.to_string_lossy()]);
        let mong = format!("\"code\":\"{ma}\",\"line\":{dong},\"col\":{cot},");
        if !k.out.contains(&mong) {
            sai.push(format!(
                "{ca}: chạy **một tệp** phải ra cùng `{mong}` — thật: {}",
                k.out.trim()
            ));
        }
    }
    let _ = std::fs::remove_dir_all(&tmp);
    assert!(sai.is_empty(), "{} ca sai:\n{}", sai.len(), sai.join("\n"));
}

// ----- 5. Fixture **audit-only** (không sinh chẩn đoán) vẫn phải nạp sạch -----

#[test]
fn fixture_audit_hop_le() {
    let mut sai: Vec<String> = Vec::new();
    for ca in ["ngoai", "get-select"] {
        let main = tep(ca, "main.vis");
        let k = chay(&["check", &main]);
        if k.code != 0 || !k.out.contains("ok 1") {
            sai.push(format!("{ca}: phải `check` xanh — thật {:?}", k.out.trim()));
        }
        let r = chay(&["run", &main]);
        if r.code != 0 || r.out != "0\n" {
            sai.push(format!(
                "{ca}: `run` phải exit 0 + stdout `0` — thật exit {} + {:?}",
                r.code, r.out
            ));
        }
    }
    assert!(sai.is_empty(), "{} ca sai:\n{}", sai.len(), sai.join("\n"));
}
