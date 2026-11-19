// SPDX-License-Identifier: MIT OR Apache-2.0
//! ST-30 — **`vis test`**: chạy bộ `test/sem/**` in **TAP**, đếm khớp harness (346), `--vm` khớp interp.
//!
//! Khoá: `1..N` đúng số ca; ca đỏ ⇒ exit 1 + `not ok` + dòng `# <tệp>:<dòng>:<cột>: <mã>`; `--vm`
//! chạy đường VM — **ST-89**: VM có host chương trình ngoài, **không còn SKIP**.

use std::path::PathBuf;
use std::process::Command;

/// Số ca mà harness `sem.rs` đang khoá — `vis test` **phải** khớp, không tự bịa.
/// **SỬA KÈM LÝ DO (ST-56)**: 370 → **376** (+6 ca nợ lõi A).
/// **SỬA KÈM LÝ DO (ST-56b)**: 376 → **382** (+6 ca phép gán).
/// **SỬA KÈM LÝ DO (ST-57)**: 382 → **388** (+6 ca `trait/`: siêu trait ×2, `derives` ×4).
/// **SỬA KÈM LÝ DO (ST-59)**: 388 → **394** (+6 ca `vish/`: shell `let`/biến stage đầu/thân `cmd`).
/// **SỬA KÈM LÝ DO (ST-60)**: 394 → **416** (+22 ca `async/` — chốt VD-90/VD-95).
/// **SỬA KÈM LÝ DO (ST-60b)**: 416 → **420** (+4 ca hotfix closure/đích lồng/`std --tên`).
/// **SỬA KÈM LÝ DO (ST-60c)**: 419 → **426** (+7 ca `trait/`: derives↔siêu trait, enum Show, Eq sâu).
/// **SỬA KÈM LÝ DO (ST-61)**: 426 → **441** (+15 ca nhóm `kenh/`).
/// **SỬA KÈM LÝ DO (ST-72)**: 441 → **447** (+6 ca nhóm `cap/`).
/// **SỬA KÈM LÝ DO (R9, VD-178)**: 447 → **448** (+`loi/range-khong-int` VS0416).
/// **SỬA KÈM LÝ DO (R6, VD-178)**: 448 → **452** (+4 ca `chuoi/`).
/// **SỬA KÈM LÝ DO (R17, VD-178)**: 452 → **453** (+`dieu-khien/is-empty`).
/// **SỬA KÈM LÝ DO (R3b, VD-178)**: 453 → **454** (+`result/let-else`).
/// **SỬA KÈM LÝ DO (R3a, VD-178)**: 454 → **455** (+`result/if-let-pattern`).
/// **SỬA KÈM LÝ DO (R16, VD-178)**: 455 → **456** (+`pham-vi/che-ten-let`).
/// **SỬA KÈM LÝ DO (R12a, VD-178)**: 456 → **457** (+`kieu/type-alias-newtype`).
/// **SỬA KÈM LÝ DO (R11, VD-178)**: 457 → **458** (+`enum/vi-tri-va-dung-ten`).
/// **SỬA KÈM LÝ DO (R7, VD-178)**: 458 → **459** (+`so-hoc/chia-nguyen`).
/// **SỬA KÈM LÝ DO (R8, VD-178)**: 459 → **460** (+`list/chi-so-an-toan`).
/// **SỬA KÈM LÝ DO (R5a, VD-178)**: 460 → **461** (+`ham/goi-ten`).
/// **SỬA KÈM LÝ DO (R5c, VD-178)**: 461 → **462** (+`chuoi-phuong-thuc/std-method`).
/// **SỬA KÈM LÝ DO (R20a, VD-178)**: 462 → **463** (+`assert/thong-diep`).
/// **SỬA KÈM LÝ DO (R20b, VD-178)**: 463 → **464** (+`list/duyet-nang-cao`).
/// **SỬA KÈM LÝ DO (R19b, VD-178)**: 464 → **465** (+`loi/doi-dau-tran` VS0507).
/// **SỬA KÈM LÝ DO (R2, VD-178)**: 465 → **469** (+`optional/{int-qhap-gan, map-get-lot, t-qhap-generic}` + `loi/none-bo`).
/// **SỬA KÈM LÝ DO (VD-240-b, 2026-11-19)**: 469 → **473** (+4 ca grid `async/grid-*`, 423 `.vis` + 50 `.vish`).
/// **SỬA KÈM LÝ DO (VD-241 §3, 2026-11-19)**: 473 → **483** (+10 ca Option lồng thật; 433 `.vis` + 50 `.vish`).
/// **SỬA KÈM LÝ DO (VD-241 §7, 2026-11-19)**: 483 → **486** (+3 ca `optional/` — `none-three-states`,
///   `long-none-param`, `long-none-return`; đổi `long-reject-e` (`.err`) thành `long-none-e` hợp lệ); 436 `.vis` + 50 `.vish`, **88 `.err`**.
/// **SỬA KÈM LÝ DO (VD-241 đợt coercion, 2026-11-19)**: 486 → **487** (+1 ca `optional/long-option-matrix.vis` —
///   ma trận sugar/spelled-out/alias × biên let/gán-lại/đối số/thân hàm/return × bảng giá trị A0; sửa runtime coercion); 437 `.vis` + 50 `.vish`, **88 `.err`**.
/// **SỬA KÈM LÝ DO (VD-241 Gate 1, 2026-11-19)**: 487 → **514** (+27 ca `sqlite/`: 19 xanh + 8 lỗi lúc chạy VS0501); 464 `.vis` + 50 `.vish`, **96 `.err`**.
/// **SỬA KÈM LÝ DO (VD-241 pre-merge, 2026-11-19)**: 514 → **525** (+11 ca `sqlite/`: `execute-tra-hang`,
///   `bind-inf`, ma trận authorizer 9 ca — mọi lỗi dữ liệu nay là `Result.Err` ⇒ 8 `.err` đổi thành `.out`);
///   475 `.vis` + 50 `.vish`, **88 `.err`**.
const SO_CA: usize = 525;

/// PATH có thêm thư mục chứa `test-child` — cho fixture `.vish` gọi `test-child` bare.
fn path_voi_tc() -> std::ffi::OsString {
    let tc = std::path::Path::new(env!("CARGO_BIN_EXE_test-child"));
    let mut muc: Vec<std::path::PathBuf> =
        vec![tc.parent().expect("test-child parent").to_path_buf()];
    let cur = std::env::var_os("PATH").unwrap_or_default();
    for p in std::env::split_paths(&cur) {
        muc.push(p);
    }
    std::env::join_paths(muc).expect("join PATH")
}

fn vis(args: &[&str]) -> (i32, String, String) {
    let mut c = Command::new(env!("CARGO_BIN_EXE_vis"));
    c.env("PATH", path_voi_tc());
    let o = c.args(args).output().expect("chạy `vis test`");
    (
        o.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&o.stdout).to_string(),
        String::from_utf8_lossy(&o.stderr).to_string(),
    )
}

fn dong(s: &str) -> Vec<&str> {
    s.lines().collect()
}

fn tam(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("vis-test-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("thư mục tạm");
    d
}

#[test]
fn tap_hop_le_va_du_so_ca() {
    let (c, o, e) = vis(&["test"]);
    let ls = dong(&o);
    assert_eq!(c, 0, "exit phải 0 khi toàn xanh: {e}");
    assert_eq!(ls[0], "TAP version 13");
    assert_eq!(ls[1], format!("1..{SO_CA}"), "kế hoạch phải khớp harness");
    let ok = ls.iter().filter(|l| l.starts_with("ok ")).count();
    assert_eq!(ok, SO_CA, "phải {SO_CA} dòng `ok`");
    assert_eq!(ls.iter().filter(|l| l.starts_with("not ok")).count(), 0);
}

#[test]
fn cac_dong_ok_dung_thu_tu_khong_lap() {
    let (_, o, _) = vis(&["test"]);
    let ls = dong(&o);
    for (i, l) in ls.iter().filter(|l| l.starts_with("ok ")).enumerate() {
        assert!(
            l.starts_with(&format!("ok {} - ", i + 1)),
            "dòng thứ {} sai thứ tự: {l}",
            i + 1
        );
    }
}

/// **SỬA KÈM LÝ DO (ST-89)**: trước đây `--vm` SKIP 7 ca `.vish` dùng chương trình ngoài (VM không
/// có host `Ngoai`); nay VM nhận **cùng** host ⇒ **0 SKIP**, mọi ca phải `ok` như interp.
#[test]
fn vm_khop_interp_va_khong_con_skip() {
    let (c, o, e) = vis(&["test", "--vm"]);
    let ls = dong(&o);
    assert_eq!(c, 0, "VM: exit phải 0 (ca cần host thì SKIP): {e}");
    assert_eq!(ls[1], format!("1..{SO_CA}"));
    assert_eq!(
        ls.iter().filter(|l| l.starts_with("not ok")).count(),
        0,
        "--vm không được có ca đỏ"
    );
    let skip = ls.iter().filter(|l| l.contains("# SKIP")).count();
    assert_eq!(
        skip, 0,
        "VM có host Ngoai ⇒ không được còn SKIP, thật {skip}"
    );
}

#[test]
fn ca_do_thoat_1_va_co_dong_chi_vi_tri() {
    let d = tam("do");
    std::fs::write(d.join("sai.vis"), "print(1)\n").expect("ghi");
    std::fs::write(
        d.join("sai.out"),
        "# SPDX-License-Identifier: MIT OR Apache-2.0\n2\n",
    )
    .expect("ghi");
    let (c, o, e) = vis(&["test", d.to_str().unwrap()]);
    assert_eq!(c, 1, "phải exit 1 khi có ca đỏ: {e}");
    let ls = dong(&o);
    assert_eq!(ls[1], "1..1");
    assert!(ls[2].starts_with("not ok 1"), "TAP: {o}");
    assert!(
        ls[3].starts_with("# sai.vis:1:1:"),
        "dòng chẩn đoán phải `<tệp>:<dòng>:<cột>:` — thật {:?}",
        ls.get(3)
    );
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn ca_loi_dung_ma_thi_ok() {
    let d = tam("err");
    std::fs::write(d.join("loi.vis"), "let x: int = \"a\"\n").expect("ghi");
    std::fs::write(d.join("loi.err"), "VS0412\n").expect("ghi");
    let (c, o, e) = vis(&["test", d.to_str().unwrap()]);
    assert_eq!(c, 0, "ca lỗi đúng mã phải ok: {o} {e}");
    assert!(dong(&o)[2].starts_with("ok 1"), "TAP: {o}");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn mot_tep_chay_duoc() {
    let d = tam("mot");
    let p = d.join("ok.vis");
    std::fs::write(&p, "print(1 + 1)\n").expect("ghi");
    std::fs::write(
        d.join("ok.out"),
        "# SPDX-License-Identifier: MIT OR Apache-2.0\n2\n",
    )
    .expect("ghi");
    let (c, o, _) = vis(&["test", p.to_str().unwrap()]);
    assert_eq!(c, 0);
    assert_eq!(dong(&o)[1], "1..1");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn co_la_la_dung_sai_thoat_2() {
    let (c, _, e) = vis(&["test", "--bogus"]);
    assert_eq!(c, 2);
    assert!(e.contains("dùng: vis test"), "stderr: {e}");
}

#[test]
fn duong_dan_khong_ton_tai_thoat_2() {
    let (c, _, e) = vis(&["test", "/khong/co/that"]);
    assert_eq!(c, 2, "stderr: {e}");
}

#[test]
fn tap_moi_not_ok_deu_co_dong_chan_doan() {
    let d = tam("nhieu");
    std::fs::write(d.join("a.vis"), "print(1)\n").expect("ghi");
    std::fs::write(
        d.join("a.out"),
        "# SPDX-License-Identifier: MIT OR Apache-2.0\n9\n",
    )
    .expect("ghi");
    std::fs::write(d.join("b.vis"), "print(\"x\")\n").expect("ghi");
    std::fs::write(
        d.join("b.out"),
        "# SPDX-License-Identifier: MIT OR Apache-2.0\nx\n",
    )
    .expect("ghi");
    let (c, o, _) = vis(&["test", d.to_str().unwrap()]);
    assert_eq!(c, 1);
    let ls = dong(&o);
    assert_eq!(ls[1], "1..2");
    for (i, l) in ls.iter().enumerate() {
        if l.starts_with("not ok") {
            assert!(
                ls.get(i + 1).map(|x| x.starts_with("# ")).unwrap_or(false),
                "mỗi `not ok` phải kèm dòng `# …`: {:?}",
                ls.get(i + 1)
            );
        }
    }
    let _ = std::fs::remove_dir_all(&d);
}

/// **ST-32**: ca `.err` lỗi **biên dịch** phải ra **cùng mã** ở `--vm` (không SKIP, không khác mã) —
/// trước ST-30b VM in `VS0501` cho lỗi chỉ số, nay đồng bộ; ca này khoá nhánh **biên dịch**.
#[test]
fn err_bien_dich_ra_cung_ma_o_vm() {
    let d = tam("biendich");
    std::fs::write(d.join("sai.vis"), "let x: int = \"chuoi\"\n").expect("ghi");
    std::fs::write(d.join("sai.err"), "VS0412\n").expect("ghi");
    let (ci, oi, ei) = vis(&["test", d.to_str().unwrap()]);
    let (cv, ov, ev) = vis(&["test", "--vm", d.to_str().unwrap()]);
    assert_eq!(ci, 0, "interp phải ok: {oi} {ei}");
    assert_eq!(cv, 0, "--vm phải ok (cùng mã VS0412): {ov} {ev}");
    assert!(oi.contains("ok 1"), "TAP interp: {oi}");
    assert!(ov.contains("ok 1"), "TAP vm: {ov}");
    assert!(
        !ov.contains("# SKIP"),
        "ca lỗi biên dịch **không** được SKIP ở --vm: {ov}"
    );
    let _ = std::fs::remove_dir_all(&d);
}
