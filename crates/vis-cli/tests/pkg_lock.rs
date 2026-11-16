// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-129 (VD-125)** — `vis.lock` **định dạng 2** (`roots` + `version_req`), migration từ định dạng
//! cũ, `install` **giữ nguyên byte** khi lock còn thoả, `update` **giải lại**, `list --json` có
//! `version_req`.
//!
//! Định dạng **cũ** (đo bằng binary ở commit ST-128, xem JOURNAL §3):
//! `{"lock":1,"packages":[{"name","version","entry","sha256","files":[{"path","sha256"}]}]}`.
//! Định dạng **mới**: thêm `"lock":2`, `"roots":{tên:ràng buộc}`, `"version_req"` trong từng gói.

use std::path::{Path, PathBuf};
use std::process::Command;

fn tmp_dir(tag: &str) -> PathBuf {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static DEM: AtomicUsize = AtomicUsize::new(0);
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-lock-{tag}-{}-{}", std::process::id(), n));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("tmp");
    d
}

fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../test/pkg/registry")
}

fn vis(cwd: &Path, args: &[&str]) -> (i32, String, String) {
    let o = Command::new(env!("CARGO_BIN_EXE_vis"))
        .current_dir(cwd)
        .args(args)
        .output()
        .expect("chạy `vis`");
    (
        o.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&o.stdout).to_string(),
        String::from_utf8_lossy(&o.stderr).to_string(),
    )
}

fn doc(p: &Path) -> String {
    std::fs::read_to_string(p).unwrap_or_else(|e| panic!("đọc {}: {e}", p.display()))
}

fn ghi(p: &Path, s: &str) {
    if let Some(d) = p.parent() {
        std::fs::create_dir_all(d).expect("mkdir");
    }
    std::fs::write(p, s).expect("ghi");
}

/// Sao registry mẫu ra thư mục tạm (để thêm phiên bản mới trong ca "update nâng cấp").
fn sao_registry(dst: &Path) {
    fn di(src: &Path, dst: &Path) {
        std::fs::create_dir_all(dst).expect("mkdir");
        for e in std::fs::read_dir(src).expect("read_dir") {
            let e = e.expect("entry");
            let p = e.path();
            let q = dst.join(e.file_name());
            if p.is_dir() {
                di(&p, &q);
            } else {
                std::fs::copy(&p, &q).expect("copy");
            }
        }
    }
    di(&fixture(), dst);
}

/// Thêm một phiên bản gói vào registry tạm (manifest băm đúng nội dung).
fn them_ban(reg: &Path, ten: &str, ver: &str, than: &str) {
    let dir = reg.join(ten).join(ver);
    std::fs::create_dir_all(dir.join("src")).expect("mkdir");
    let noi_dung = format!("# SPDX-License-Identifier: MIT OR Apache-2.0\n\n{than}");
    std::fs::write(dir.join("src/main.vis"), &noi_dung).expect("ghi nguồn");
    let h = vis_bytecode::source_hash(&noi_dung);
    let sha: String = h.iter().map(|b| format!("{b:02x}")).collect();
    let m = format!(
        "{{\"deps\":{{}},\"entry\":\"src/main.vis\",\"files\":[{{\"path\":\"src/main.vis\",\"sha256\":\"{sha}\"}}],\"name\":\"{ten}\",\"version\":\"{ver}\"}}"
    );
    std::fs::write(dir.join("manifest.json"), m).expect("ghi manifest");
}

/// Đổi lock định dạng 2 thành định dạng **1** (bỏ `roots` + `version_req`, `lock` = 1) — đúng định
/// dạng cũ đã **đo** ở ST-128.
fn ve_dinh_dang_cu(lock: &str) -> String {
    let mut s = lock.to_string();
    assert!(s.contains("\"lock\":2"), "lock phải là định dạng 2: {s}");
    s = s.replace("\"lock\":2", "\"lock\":1");
    s = s.replace("\"roots\":{},", "");
    // bỏ mọi `"version_req":"…",`
    while let Some(i) = s.find("\"version_req\":\"") {
        let j = s[i..].find("\",\"").map(|k| i + k + 1).expect("kết thúc");
        s.replace_range(i..j + 1, "");
        // dọn dấu phẩy thừa nếu có (không xảy ra vì `version_req` luôn ở giữa)
    }
    s
}

/// Ca 1 — `install` khi lock định dạng 2 **còn thoả** ⇒ **không** đổi byte.
#[test]
fn install_giu_nguyen_lock_khi_con_thoa() {
    let d = tmp_dir("giu-nguyen");
    let (c, _o, e) = vis(
        &d,
        &[
            "pkg",
            "add",
            "http@^1",
            "--registry",
            fixture().to_str().unwrap(),
        ],
    );
    assert_eq!(c, 0, "stderr: {e}");
    let truoc = doc(&d.join("vis.lock"));
    assert!(truoc.contains("\"lock\":2"), "lock mới phải là 2: {truoc}");
    assert!(truoc.contains("\"version_req\":\"^1\""), "{truoc}");
    assert!(
        truoc.contains("\"roots\":{\"http\":\"^1\"}"),
        "roots phải có ràng buộc gốc: {truoc}"
    );
    let (c2, _o2, e2) = vis(&d, &["pkg", "install"]);
    assert_eq!(c2, 0, "stderr: {e2}");
    assert_eq!(
        doc(&d.join("vis.lock")),
        truoc,
        "install không được đổi byte"
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// Ca 2 — lock **định dạng cũ** ⇒ `install` đọc được, **ghi lại định dạng mới**; ghi lại lần nữa
/// (install lần 2) cho **cùng byte**.
#[test]
fn lock_cu_migrate_va_ghi_hai_lan_cung_byte() {
    let d = tmp_dir("migrate");
    let (c, _o, e) = vis(
        &d,
        &[
            "pkg",
            "add",
            "http@1.0.0",
            "--registry",
            fixture().to_str().unwrap(),
        ],
    );
    assert_eq!(c, 0, "stderr: {e}");
    let cu = ve_dinh_dang_cu(&doc(&d.join("vis.lock")));
    assert!(
        cu.contains("\"lock\":1") && !cu.contains("version_req"),
        "{cu}"
    );
    ghi(&d.join("vis.lock"), &cu);
    let (c1, _o1, e1) = vis(&d, &["pkg", "install"]);
    assert_eq!(c1, 0, "stderr: {e1}");
    let moi = doc(&d.join("vis.lock"));
    assert!(moi.contains("\"lock\":2"), "phải migrate lên 2: {moi}");
    assert!(
        moi.contains("\"version_req\":\"=1.0.0\""),
        "lock cũ ⇒ ghim đúng: {moi}"
    );
    let (c2, _o2, e2) = vis(&d, &["pkg", "install"]);
    assert_eq!(c2, 0, "stderr: {e2}");
    assert_eq!(doc(&d.join("vis.lock")), moi, "ghi lại hai lần cùng byte");
    let _ = std::fs::remove_dir_all(&d);
}

/// Ca 3 — lock định dạng mới nhưng **không còn thoả** ⇒ exit 2, chỉ đường `vis pkg update`.
#[test]
fn install_lock_khong_con_thoa_chi_duong_update() {
    let d = tmp_dir("khong-thoa");
    let (c, _o, e) = vis(
        &d,
        &[
            "pkg",
            "add",
            "http@1.0.0",
            "--registry",
            fixture().to_str().unwrap(),
        ],
    );
    assert_eq!(c, 0, "stderr: {e}");
    let lock =
        doc(&d.join("vis.lock")).replace("\"version_req\":\"=1.0.0\"", "\"version_req\":\"^2\"");
    ghi(&d.join("vis.lock"), &lock);
    let (c2, _o2, e2) = vis(&d, &["pkg", "install"]);
    assert_eq!(c2, 2, "stderr: {e2}");
    assert!(e2.contains("không còn thoả"), "{e2}");
    assert!(e2.contains("pkg update"), "phải chỉ đường: {e2}");
    let _ = std::fs::remove_dir_all(&d);
}

/// Ca 4 — `update` **giải lại** theo `roots`: thêm phiên bản mới trong khoảng ⇒ nâng cấp,
/// `version_req` giữ nguyên ràng buộc gốc.
#[test]
fn update_nang_trong_khoang() {
    let d = tmp_dir("nang-cap");
    let reg = d.join("reg");
    sao_registry(&reg);
    let (c, _o, e) = vis(
        &d,
        &[
            "pkg",
            "add",
            "http@^1.0",
            "--registry",
            reg.to_str().unwrap(),
        ],
    );
    assert_eq!(c, 0, "stderr: {e}");
    assert!(
        doc(&d.join("vis.lock")).contains("\"version\":\"1.10.0\""),
        "ban đầu 1.10.0"
    );
    // Bản mới xuất hiện **sau** khi add ⇒ chỉ `update` mới thấy.
    them_ban(
        &reg,
        "http",
        "1.11.0",
        "pub fn http_version() -> int:\n    111\n",
    );
    let (c2, o2, e2) = vis(&d, &["pkg", "update", "--registry", reg.to_str().unwrap()]);
    assert_eq!(c2, 0, "stdout: {o2} stderr: {e2}");
    assert!(o2.contains("đã cập nhật http 1.11.0"), "stdout: {o2}");
    let lock = doc(&d.join("vis.lock"));
    assert!(lock.contains("\"version\":\"1.11.0\""), "phải nâng: {lock}");
    assert!(
        lock.contains("\"version_req\":\"^1.0\""),
        "giữ ràng buộc gốc: {lock}"
    );
    assert!(
        d.join(".vis/pkg/http/1.11.0/src/main.vis").is_file(),
        "cây cài phải có bản mới"
    );
    // Update lần nữa: không còn gì đổi ⇒ **cùng byte**.
    let (c3, _o3, e3) = vis(&d, &["pkg", "update", "--registry", reg.to_str().unwrap()]);
    assert_eq!(c3, 0, "stderr: {e3}");
    assert_eq!(
        doc(&d.join("vis.lock")),
        lock,
        "update lần 2 không đổi byte"
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// Ca 5 — `list --json` có `version_req` (đúng ràng buộc **hiệu lực** của từng gói).
#[test]
fn list_json_co_version_req() {
    let d = tmp_dir("list-req");
    let (c, _o, e) = vis(
        &d,
        &[
            "pkg",
            "add",
            "http@^1.0",
            "--registry",
            fixture().to_str().unwrap(),
        ],
    );
    assert_eq!(c, 0, "stderr: {e}");
    let (c2, o, e2) = vis(&d, &["pkg", "list", "--json"]);
    assert_eq!(c2, 0, "stderr: {e2}");
    assert_eq!(o.lines().count(), 1, "một dòng JSON: {o}");
    let v = vis_interp::json::decode(&o).expect("JSON hợp lệ");
    let vis_interp::value::Value::Map(kv) = v else {
        panic!("không phải object")
    };
    let Some(vis_interp::value::Value::List(ps)) = kv.iter().find_map(|(k, v)| {
        matches!(k, vis_interp::value::Value::Str(s) if s == "packages").then_some(v)
    }) else {
        panic!("thiếu packages")
    };
    let mut thay = 0;
    for p in ps {
        let vis_interp::value::Value::Map(m) = p else {
            panic!("package không phải object")
        };
        let lay = |k: &str| {
            m.iter().find_map(|(kk, vv)| match (kk, vv) {
                (vis_interp::value::Value::Str(x), vis_interp::value::Value::Str(y)) if x == k => {
                    Some(y.clone())
                }
                _ => None,
            })
        };
        let req = lay("version_req").unwrap_or_else(|| panic!("thiếu `version_req`: {m:?}"));
        assert!(!req.is_empty());
        let ten = lay("name").unwrap();
        let muon = if ten == "http" { "^1.0" } else { "^0.1.0" };
        assert_eq!(req, muon, "ràng buộc hiệu lực của `{ten}`");
        thay += 1;
    }
    assert_eq!(thay, 2, "http + util");
    let _ = std::fs::remove_dir_all(&d);
}

/// Ca 6 — `add` nhiều gói ⇒ `roots` giữ **cả hai**, `version_req` của dep là ràng buộc **của cha**.
#[test]
fn add_ghi_roots_va_rang_buoc_dep() {
    let d = tmp_dir("roots");
    let reg = d.join("reg");
    sao_registry(&reg);
    for spec in ["http@^1.0", "util@^0.1"] {
        let (c, _o, e) = vis(
            &d,
            &["pkg", "add", spec, "--registry", reg.to_str().unwrap()],
        );
        assert_eq!(c, 0, "{spec}: {e}");
    }
    let lock = doc(&d.join("vis.lock"));
    assert!(
        lock.contains("\"roots\":{\"http\":\"^1.0\",\"util\":\"^0.1\"}"),
        "roots phải có cả hai, sắp theo tên: {lock}"
    );
    // `util` vừa là root (`^0.1`) vừa là dep của `http` (`^0.1.0`) ⇒ AND của hai ràng buộc.
    assert!(
        lock.contains("\"name\":\"util\",\"version\":\"0.1.0\",\"version_req\":\"^0.1, ^0.1.0\""),
        "version_req của util: {lock}"
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// Ca 7 — `remove` bỏ cả `roots`; lock vẫn hợp lệ và `install` chạy lại được.
#[test]
fn remove_bo_ca_roots() {
    let d = tmp_dir("remove-roots");
    let (c, _o, e) = vis(
        &d,
        &[
            "pkg",
            "add",
            "http@^1",
            "--registry",
            fixture().to_str().unwrap(),
        ],
    );
    assert_eq!(c, 0, "stderr: {e}");
    let (c2, _o2, e2) = vis(&d, &["pkg", "remove", "http"]);
    assert_eq!(c2, 0, "stderr: {e2}");
    let lock = doc(&d.join("vis.lock"));
    assert!(!lock.contains("\"http\""), "phải bỏ cả roots: {lock}");
    assert!(lock.contains("\"roots\":{}"), "{lock}");
    let (c3, _o3, e3) = vis(&d, &["pkg", "install"]);
    assert_eq!(c3, 0, "install sau remove: {e3}");
    let _ = std::fs::remove_dir_all(&d);
}

/// Ca 8 — lock cũ (định dạng 1) + `update` ⇒ ghi định dạng 2 **một lần**, lần sau cùng byte.
#[test]
fn update_lock_cu_ghi_dinh_dang_moi_mot_lan() {
    let d = tmp_dir("update-cu");
    let reg = d.join("reg");
    sao_registry(&reg);
    let (c, _o, e) = vis(
        &d,
        &[
            "pkg",
            "add",
            "http@1.0.0",
            "--registry",
            reg.to_str().unwrap(),
        ],
    );
    assert_eq!(c, 0, "stderr: {e}");
    ghi(
        &d.join("vis.lock"),
        &ve_dinh_dang_cu(&doc(&d.join("vis.lock"))),
    );
    let (c1, _o1, e1) = vis(&d, &["pkg", "update", "--registry", reg.to_str().unwrap()]);
    assert_eq!(c1, 0, "stderr: {e1}");
    let moi = doc(&d.join("vis.lock"));
    assert!(moi.contains("\"lock\":2"), "{moi}");
    assert!(
        moi.contains("\"roots\":{\"http\":\"=1.0.0\""),
        "roots từ lock cũ: {moi}"
    );
    let (c2, _o2, e2) = vis(&d, &["pkg", "update", "--registry", reg.to_str().unwrap()]);
    assert_eq!(c2, 0, "stderr: {e2}");
    assert_eq!(doc(&d.join("vis.lock")), moi, "update lần 2 cùng byte");
    let _ = std::fs::remove_dir_all(&d);
}
