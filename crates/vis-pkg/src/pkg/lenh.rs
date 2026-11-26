// SPDX-License-Identifier: MIT OR Apache-2.0
//! Các lệnh `vis pkg` (`run`/`add`/`install`/`update`/`list`/`remove`/`migrate`) — tách khỏi
//! `pkg.rs` ở `ST-317b`.

use std::collections::{BTreeMap, BTreeSet};
use std::process::ExitCode;

use super::semver::{Version, VersionReq};

use super::goi::{ghi_cay, GoiCai, TepCai};
use super::khoa::{ghi_lock, lock_con_thoa, rang_buoc_hieu_luc, roots_tu_lock, tao_khoa};
use super::nguon::{
    cwd, khoa_cong_tu_chon, nap_goi, registry_tu_chon, req_moi_phien_ban, tach_ten, tuy_chon,
    TuyChon,
};
use super::{bam, doc_lock, gc, nang_cap_lock, GoiKhoa, LOCK_PHIEN_BAN};

/// Chạy `vis pkg …`; lỗi ⇒ `vis pkg: …` + exit **2** (mẫu `vis cache`, không mã mới).
pub fn run(args: &[String], net: &mut dyn vis_core::host::NetHost) -> ExitCode {
    match chay(args, net) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("vis pkg: {e}");
            ExitCode::from(2)
        }
    }
}

pub(crate) fn dung() -> &'static str {
    "dùng: vis pkg add <tên>[@<ver>] [--entry <path>] [--registry <dir|http://host:port|https://host[:port][/path]>] [--registry-key <64-hex>]... | vis pkg install | vis pkg update [<tên>] [--registry <dir|http://host:port|https://host[:port][/path]>] [--registry-key <64-hex>]... | vis pkg list [--json] | vis pkg remove <tên> | vis pkg gc [--dry-run] [--lock <path>]... [--root <dir>]... | vis pkg migrate"
}

pub(crate) fn chay(args: &[String], net: &mut dyn vis_core::host::NetHost) -> Result<(), String> {
    let Some(lenh) = args.first() else {
        return Err(dung().to_string());
    };
    match lenh.as_str() {
        "add" => lenh_add(&args[1..], net),
        "install" => lenh_install(&args[1..]),
        "update" => lenh_update(&args[1..], net),
        "list" => lenh_list(&args[1..]),
        "remove" => lenh_remove(&args[1..]),
        "gc" => gc::lenh(&args[1..]),
        "migrate" => lenh_migrate(&args[1..]),
        khac => Err(format!("lệnh lạ `{khac}` — {}", dung())),
    }
}
/// `vis pkg add <tên>[@<ràng buộc>] [--registry <dir>]`.
///
/// **ST-128 (VD-125)**: giải đồ thị bằng [`resolver::Resolver`] — mỗi gói **đúng một** phiên bản,
/// chọn **cao nhất thoả mọi ràng buộc**, quay lui khi xung đột; không giải được ⇒ `VS0704`.
///
/// **ST-129**: ghi `vis.lock` **định dạng 2** — `roots` (ràng buộc **gốc**) + `version_req` của
/// từng gói (ràng buộc **hiệu lực** = AND của mọi ràng buộc trỏ tới nó). Không ghi `@` ⇒ root là
/// `^<phiên bản đã chọn>` (giống "lấy bản mới nhất, ở lại major đó").
pub(crate) fn lenh_add(
    args: &[String],
    net: &mut dyn vis_core::host::NetHost,
) -> Result<(), String> {
    let TuyChon {
        ten,
        reg,
        entry: entry_override,
        khoa,
    } = tuy_chon(args, true)?;
    let Some(spec) = ten else {
        return Err(format!("thiếu tên gói — {}", dung()));
    };
    let (ten, req_nguoi_dung) = tach_ten(&spec)?;
    let reg = registry_tu_chon(reg)?;
    let khoa_cong = khoa_cong_tu_chon(&khoa, &reg)?;
    let cwd = cwd()?;
    // Đọc lock cũ **trước** khi ghi gì (lock hỏng ⇒ dừng, không nửa vời).
    let khoa_cu = doc_lock(&cwd.join("vis.lock"), false)?;
    let mut roots = roots_tu_lock(&khoa_cu)?;
    // Ràng buộc **tạm** để giải: không ghi `@` ⇒ "mọi phiên bản" rồi sẽ thu về `^<đã chọn>`.
    let req_tam = req_nguoi_dung.clone().unwrap_or_else(req_moi_phien_ban);
    roots.insert(ten.clone(), req_tam);
    // Giải **trước** khi ghi bất cứ thứ gì: xung đột ⇒ không tạo `.vis/`, không sửa `vis.lock`.
    let (giai, ds) = nap_goi(
        &reg,
        &roots,
        entry_override.as_deref().map(|e| (ten.as_str(), e)),
        &khoa_cong,
        &BTreeMap::new(),
        net,
    )?;
    // Root **ghi vào lock**: `@req` ⇒ đúng req đó; không `@` ⇒ `^<phiên bản đã chọn>`.
    if req_nguoi_dung.is_none() {
        let v = giai
            .get(&ten)
            .ok_or_else(|| format!("lời giải thiếu gói gốc `{ten}`"))?;
        let r = VersionReq::parse(&format!("^{v}"))
            .map_err(|e| format!("không tạo được ràng buộc `^{v}`: {e}"))?;
        roots.insert(ten.clone(), r);
    }
    // 1) blob nội dung + 2) cây cài đặt (sau khi **mọi** manifest hợp lệ).
    ghi_cay(&cwd, &ds)?;
    // 3) lock định dạng 2: `roots` + `version_req` hiệu lực của từng gói.
    let hieu_luc = rang_buoc_hieu_luc(&roots, &ds);
    let khoa = tao_khoa(&roots, &ds, &hieu_luc);
    ghi_lock(&cwd.join("vis.lock"), &khoa)
}

/// `vis pkg install` — **chỉ** đọc `vis.lock` + `.vis/store`; không mở registry.
///
/// **ST-129 (VD-125)**: lock hiện có **còn thoả** ⇒ **giữ nguyên byte** (không ghi lại). Lock **định
/// dạng cũ** ⇒ đọc được, **ghi lại định dạng mới** (migration, `version_req` = `=<version>` từng gói).
/// Lock định dạng mới nhưng **không** còn thoả ⇒ lỗi rõ, chỉ đường `vis pkg update` (install không
/// có registry nên **không** tự giải lại).
pub(crate) fn lenh_install(args: &[String]) -> Result<(), String> {
    let tc = tuy_chon(args, false)?;
    if tc.ten.is_some() || tc.entry.is_some() || !tc.khoa.is_empty() {
        return Err(format!("`install` không nhận đối số — {}", dung()));
    }
    if tc.reg.is_some() {
        return Err("`install` không nhận `--registry`".to_string());
    }
    let cwd = cwd()?;
    let lock_path = cwd.join("vis.lock");
    let mut khoa = doc_lock(&lock_path, true)?;
    // Migration: lock định dạng cũ ⇒ sinh `version_req` = `=<version>` cho từng gói rồi ghi lại
    // **một lần**; lock định dạng 2 còn thoả ⇒ **không** chạm tệp (byte y hệt).
    let can_ghi = nang_cap_lock(&mut khoa);
    lock_con_thoa(&khoa)?;
    // Kiểm **toàn bộ** blob trước khi chạm `.vis/pkg` (lock hỏng/thiếu blob ⇒ cây giữ nguyên).
    struct GoiLai {
        name: String,
        version: String,
        files: Vec<(String, String)>,
    }
    let mut noi_dung: Vec<GoiLai> = Vec::new();
    for g in &khoa.goi {
        let mut fs = Vec::new();
        for f in &g.files {
            let blob = cwd.join(".vis/store").join(&f.sha256);
            let bytes = std::fs::read(&blob).map_err(|_| {
                format!(
                    "thiếu blob `{}` cho `{}@{}` — chạy `vis pkg add`",
                    f.sha256, g.name, g.version
                )
            })?;
            let text = std::str::from_utf8(&bytes)
                .map_err(|e| format!("blob `{}` không phải UTF-8: {e}", f.sha256))?;
            if bam(text) != f.sha256 {
                return Err(format!("blob `{}` hỏng (băm lệch)", f.sha256));
            }
            fs.push((f.path.clone(), text.to_string()));
        }
        noi_dung.push(GoiLai {
            name: g.name.clone(),
            version: g.version.clone(),
            files: fs,
        });
    }
    for g in noi_dung {
        let dich = cwd.join(".vis/pkg").join(&g.name).join(&g.version);
        let _ = std::fs::remove_dir_all(&dich);
        for (path, text) in g.files {
            let p = dich.join(&path);
            if let Some(d) = p.parent() {
                std::fs::create_dir_all(d)
                    .map_err(|e| format!("không tạo được `{}`: {e}", d.display()))?;
            }
            std::fs::write(&p, text.as_bytes())
                .map_err(|e| format!("lỗi ghi `{}`: {e}", p.display()))?;
        }
    }
    if can_ghi {
        ghi_lock(&lock_path, &khoa)?;
    }
    Ok(())
}

/// `vis pkg update [<tên>] [--registry <dir>]`.
///
/// **ST-129 (VD-125)**: **giải lại** từ `roots` của lock (lock cũ ⇒ mỗi gói thành root `=<version>`);
/// gói được chọn lại theo ràng buộc, nội dung đổi ⇒ cài lại + cập nhật sha; ghi lại lock **định dạng
/// mới**.
///
/// **ST-288 (#142)**: có `<tên>` ⇒ **soft pin** các gói khác ở phiên bản đang có — resolver *thử
/// trước* bản đó để gói không liên quan giữ nguyên, nhưng **không** ghim cứng `=<version>`: nếu
/// manifest của gói được update đòi nâng một dep chung thì dep đó được nâng trong phạm vi ràng buộc
/// (thay vì báo `VS0704`/bỏ qua bản mới). Gói không liên quan vẫn giữ phiên bản.
pub(crate) fn lenh_update(
    args: &[String],
    net: &mut dyn vis_core::host::NetHost,
) -> Result<(), String> {
    let TuyChon {
        ten,
        reg,
        entry,
        khoa,
    } = tuy_chon(args, true)?;
    if entry.is_some() {
        return Err("`update` không nhận `--entry`".to_string());
    }
    let cwd = cwd()?;
    let lock_path = cwd.join("vis.lock");
    let khoa_cu = doc_lock(&lock_path, true)?;
    if let Some(t) = &ten {
        if !khoa_cu.goi.iter().any(|g| &g.name == t) {
            return Err(format!("chưa cài gói `{t}` — không có trong `vis.lock`"));
        }
    }
    let reg = registry_tu_chon(reg)?;
    let khoa_cong = khoa_cong_tu_chon(&khoa, &reg)?;
    let roots = roots_tu_lock(&khoa_cu)?;
    // **ST-288 (#142)**: soft pin (không hard pin) — xem doc hàm. Gói được update **không** ưu tiên.
    let mut uu_tien: BTreeMap<String, Version> = BTreeMap::new();
    if let Some(t) = &ten {
        for g in &khoa_cu.goi {
            if &g.name != t {
                if let Ok(v) = Version::parse(&g.version) {
                    uu_tien.insert(g.name.clone(), v);
                }
            }
        }
    }
    // Giải **lại** toàn bộ (tất định) rồi so với lock cũ để biết gói nào đổi.
    let (giai, ds) = nap_goi(&reg, &roots, None, &khoa_cong, &uu_tien, net)?;
    let cu: BTreeMap<&str, &GoiKhoa> = khoa_cu.goi.iter().map(|g| (g.name.as_str(), g)).collect();
    let mut doi = khoa_cu.can_migrate() || khoa_cu.roots.is_empty();
    let mut can_cai: Vec<GoiCai> = Vec::new();
    for g in &ds {
        let v = giai.get(&g.name).map(|v| v.to_string()).unwrap_or_default();
        match cu.get(g.name.as_str()) {
            Some(c) if c.version == g.version && c.sha256 == g.sha256 => {
                println!("không đổi {} {}", g.name, g.version);
            }
            _ => {
                println!("đã cập nhật {} {}", g.name, g.version);
                can_cai.push(GoiCai {
                    name: g.name.clone(),
                    version: g.version.clone(),
                    entry: g.entry.clone(),
                    sha256: g.sha256.clone(),
                    files: g
                        .files
                        .iter()
                        .map(|f| TepCai {
                            path: f.path.clone(),
                            sha256: f.sha256.clone(),
                            noi_dung: f.noi_dung.clone(),
                        })
                        .collect(),
                    deps: g.deps.clone(),
                });
                let _ = v;
                doi = true;
            }
        }
    }
    if !can_cai.is_empty() {
        ghi_cay(&cwd, &can_cai)?;
    }
    let hieu_luc = rang_buoc_hieu_luc(&roots, &ds);
    let khoa = tao_khoa(&roots, &ds, &hieu_luc);
    if doi {
        ghi_lock(&lock_path, &khoa)?;
    }
    Ok(())
}

/// `vis pkg list [--json]` — không cờ: `tên ver sha256[:12]` (giữ nguyên); `--json`:
/// `{"packages":[{name,version,entry,sha256}]}` sắp theo **tên byte-order**.
pub(crate) fn lenh_list(args: &[String]) -> Result<(), String> {
    let json_ra = args.iter().any(|a| a == "--json");
    let args: Vec<String> = args
        .iter()
        .filter(|a| a.as_str() != "--json")
        .cloned()
        .collect();
    let tc = tuy_chon(&args, false)?;
    if tc.ten.is_some() || tc.reg.is_some() || tc.entry.is_some() || !tc.khoa.is_empty() {
        return Err(format!("`list` không nhận đối số — {}", dung()));
    }
    let cwd = cwd()?;
    let khoa = doc_lock(&cwd.join("vis.lock"), true)?.goi;
    if json_ra {
        let mut out = String::from("{\"packages\":[");
        for (i, p) in khoa.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            // **ST-129 (VD-125)**: thêm `version_req`; lock cũ chưa có ⇒ in `=<version>` (giá trị
            // hiệu lực khi migrate) để đầu ra **không phụ thuộc** việc lock đã migrate hay chưa.
            let req = p
                .version_req
                .clone()
                .unwrap_or_else(|| format!("={}", p.version));
            out.push_str(&format!(
                "{{\"name\":\"{}\",\"version\":\"{}\",\"version_req\":\"{}\",\"entry\":\"{}\",\"sha256\":\"{}\"}}",
                crate::json_esc::esc(&p.name),
                crate::json_esc::esc(&p.version),
                crate::json_esc::esc(&req),
                crate::json_esc::esc(p.entry.as_deref().unwrap_or("")),
                crate::json_esc::esc(&p.sha256)
            ));
        }
        out.push_str("]}");
        println!("{out}");
        return Ok(());
    }
    for p in &khoa {
        println!(
            "{} {} {}",
            p.name,
            p.version,
            &p.sha256[..12.min(p.sha256.len())]
        );
    }
    Ok(())
}

/// `vis pkg remove <tên>` — xoá mục trong `vis.lock` + cây `.vis/pkg/<tên>/<ver>`; blob trong
/// `.vis/store` chỉ xoá khi **không** gói còn lại nào dùng. **Không** sửa `vis.toml`, không mạng.
pub(crate) fn lenh_remove(args: &[String]) -> Result<(), String> {
    if args.len() != 1 {
        return Err(format!("`remove` cần đúng **một** tên gói — {}", dung()));
    }
    let ten = &args[0];
    if ten.starts_with('-') {
        return Err(format!("cờ lạ `{ten}` — {}", dung()));
    }
    let cwd = cwd()?;
    let khoa_path = cwd.join("vis.lock");
    let mut khoa = doc_lock(&khoa_path, true)?;
    let Some(pos) = khoa.goi.iter().position(|g| g.name == *ten) else {
        return Err(format!("chưa cài gói `{ten}` — không có trong `vis.lock`"));
    };
    let goi = khoa.goi.remove(pos);
    // **ST-129**: bỏ cả ràng buộc gốc (nếu là gói người dùng đòi) — lock vẫn nhất quán.
    khoa.roots.remove(ten);
    // Blob còn dùng bởi gói khác (so **sha**, không theo tên).
    let con_dung: BTreeSet<String> = khoa
        .goi
        .iter()
        .flat_map(|g| g.files.iter().map(|f| f.sha256.clone()))
        .collect();
    for f in &goi.files {
        if !con_dung.contains(&f.sha256) {
            let _ = std::fs::remove_file(cwd.join(".vis/store").join(&f.sha256));
        }
    }
    let thu_muc = cwd.join(".vis/pkg").join(&goi.name);
    let _ = std::fs::remove_dir_all(thu_muc.join(&goi.version));
    let _ = std::fs::remove_dir(thu_muc);
    ghi_lock(&khoa_path, &khoa)
}

/// `vis pkg migrate` — nâng `vis.lock` cũ lên định dạng hiện tại **một cách tường minh** (cùng logic
/// `install`/`update`: ghim đúng phiên bản đang có, không nới ràng buộc).
///
/// **VS3-5b (VD-271)**: không mạng, không đụng `.vis/pkg`/store; lock đã hiện tại ⇒ **không ghi lại**
/// (byte y hệt); thiếu lock ⇒ lỗi rõ.
pub(crate) fn lenh_migrate(args: &[String]) -> Result<(), String> {
    if !args.is_empty() {
        return Err(format!("`migrate` không nhận đối số — {}", dung()));
    }
    let cwd = cwd()?;
    let lock_path = cwd.join("vis.lock");
    if !lock_path.is_file() {
        return Err("chưa có `vis.lock` — không có gì để migrate".to_string());
    }
    let mut khoa = doc_lock(&lock_path, true)?;
    let can_ghi = nang_cap_lock(&mut khoa);
    // Kể cả lock đã mới: vẫn kiểm nhất quán trước khi báo “không đổi”.
    lock_con_thoa(&khoa)?;
    if !can_ghi {
        println!("vis.lock đã ở định dạng {LOCK_PHIEN_BAN} — không đổi");
        return Ok(());
    }
    ghi_lock(&lock_path, &khoa)?;
    println!("đã nâng cấp vis.lock lên định dạng {LOCK_PHIEN_BAN}");
    Ok(())
}
