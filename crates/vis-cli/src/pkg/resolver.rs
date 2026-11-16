// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-128 (VD-125)** — resolver **tất định** cho registry cục bộ.
//!
//! Quy tắc (VD-125):
//!
//! - Mỗi gói trong đồ thị có **đúng một** phiên bản (VS2 — không có nhiều bản song song).
//! - Chọn **phiên bản cao nhất thoả mọi ràng buộc**; **quay lui** khi nhánh hiện tại dẫn tới ngõ cụt.
//! - Không thoả được ⇒ [`LoiXungDot`] nêu **chuỗi ràng buộc** gây xung đột (ai đòi gì), kèm các
//!   phiên bản registry **có**; `pkg` đổi lỗi này thành mã **`VS0704`** (khối `07xx`, cấp mới).
//! - **Tất định**: mọi tập hợp duyệt theo `BTreeMap`/`BTreeSet` (thứ tự tên byte), ứng viên thử theo
//!   thứ tự **giảm dần** ⇒ cùng registry + cùng ràng buộc ⇒ cùng lời giải, cùng thứ tự.
//!
//! Không mạng, không crate mới.

use std::collections::BTreeMap;
use std::path::Path;
#[cfg(test)]
use std::path::PathBuf;

use super::semver::{Version, VersionReq};

/// Lỗi không giải được: đủ dữ kiện để in **chuỗi ràng buộc** (VD-125).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoiXungDot {
    /// Gói không chọn được phiên bản.
    pub ten: String,
    /// `(nguồn, ràng buộc)` — nguồn `"gốc"` hoặc `"<tên>@<phiên bản>"`; đã sắp xếp ổn định.
    pub rang_buoc: Vec<(String, VersionReq)>,
    /// Các phiên bản **có** trong registry (tăng dần) — rỗng nếu không có bản nào.
    pub co: Vec<Version>,
    /// Lỗi **đọc manifest** của chính gói này (nếu có) — ưu tiên báo lỗi cấu hình của registry
    /// (ví dụ thiếu `entry`) thay vì chỉ nói "không có phiên bản thoả".
    pub loi_manh: Vec<String>,
}

impl LoiXungDot {
    /// Câu mô tả **chuỗi ràng buộc** (dùng cho cả `Display` và thông điệp `VS0704`).
    pub fn chuoi_rang_buoc(&self) -> String {
        let mut ds: Vec<String> = self
            .rang_buoc
            .iter()
            .map(|(nguon, req)| format!("`{req}` (từ {nguon})"))
            .collect();
        ds.sort();
        ds.dedup();
        ds.join(" + ")
    }
}

impl std::fmt::Display for LoiXungDot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "không phiên bản `{}` nào thoả đồng thời {}",
            self.ten,
            self.chuoi_rang_buoc()
        )?;
        if self.co.is_empty() {
            write!(f, "; registry không có phiên bản `{}` nào", self.ten)?;
        } else {
            let ds: Vec<String> = self.co.iter().map(ToString::to_string).collect();
            write!(f, "; registry có: {}", ds.join(", "))?;
        }
        if let Some(dau) = self.loi_manh.first() {
            write!(f, "; manifest lỗi: {dau}")?;
        }
        Ok(())
    }
}

impl std::error::Error for LoiXungDot {}

/// Một phiên bản trong chỉ mục, kèm **deps đã đọc** từ manifest.
#[derive(Debug, Clone)]
struct Muc {
    version: Version,
    /// `(tên gói, ràng buộc)` — giữ **thứ tự** trong manifest rồi sắp theo tên để tất định.
    deps: Vec<(String, VersionReq)>,
}

/// Resolver quanh một thư mục registry: `<registry>/<tên>/<phiên bản>/manifest.json`.
pub struct Resolver<'a> {
    reg: &'a Path,
    chi_muc: BTreeMap<String, Vec<Muc>>,
    /// Xung đột "nhiều ràng buộc nhất" gặp trong lần duyệt — chọn theo tiêu chí **tất định**.
    xung_dot: Option<LoiXungDot>,
    /// `--entry` của **gói gốc** (chỉ áp cho gói này, như `pkg add --entry` từ ST-90).
    entry_goc: Option<(String, String)>,
    /// Lỗi đọc manifest theo từng gói (để thông điệp cuối nêu lỗi cấu hình thay vì "không có bản").
    loi_manh: BTreeMap<String, Vec<String>>,
}

impl<'a> Resolver<'a> {
    /// Tạo resolver trên thư mục registry (chưa đọc gì — đọc lười từng gói).
    pub fn moi(reg: &'a Path) -> Self {
        Self {
            reg,
            chi_muc: BTreeMap::new(),
            xung_dot: None,
            entry_goc: None,
            loi_manh: BTreeMap::new(),
        }
    }

    /// Áp `--entry` cho **gói gốc** (chỉ khi manifest của nó thiếu `entry` — như `doc_manifest`).
    pub fn voi_entry_goc(mut self, ten: &str, entry: &str) -> Self {
        self.entry_goc = Some((ten.to_string(), entry.to_string()));
        self
    }

    /// Đọc + sắp xếp **giảm dần** mọi phiên bản hợp lệ của một gói (lười, có cache).
    ///
    /// Phiên bản **không hợp lệ theo VD-125** (thiếu thành phần, prerelease…) bị **bỏ qua**; gói
    /// không có thư mục/không có bản hợp lệ ⇒ `Err` với danh sách `co` rỗng.
    fn nap(&mut self, ten: &str) -> Result<(), LoiXungDot> {
        if self.chi_muc.contains_key(ten) {
            return Ok(());
        }
        let dir = self.reg.join(ten);
        let mut ds: Vec<Muc> = Vec::new();
        if let Ok(rd) = std::fs::read_dir(&dir) {
            for it in rd.flatten() {
                if !it.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                    continue;
                }
                let Some(ten_ban) = it.file_name().to_str().map(str::to_string) else {
                    continue;
                };
                let Ok(version) = Version::parse(&ten_ban) else {
                    continue; // prerelease/thiếu thành phần ⇒ không phải phiên bản VS2
                };
                let mpath = dir.join(&ten_ban).join("manifest.json");
                if !mpath.is_file() {
                    continue;
                }
                let eo = match &self.entry_goc {
                    Some((t, e)) if t == ten => Some(e.as_str()),
                    _ => None,
                };
                let m = match super::doc_manifest(&mpath, eo) {
                    Ok(m) => m,
                    Err(err) => {
                        // Không chọn bản hỏng, nhưng **ghi nhớ** lý do để thông điệp cuối nêu
                        // đúng lỗi cấu hình (ví dụ thiếu `entry`) thay vì "không có phiên bản".
                        self.loi_manh
                            .entry(ten.to_string())
                            .or_default()
                            .push(format!("`{}@{ten_ban}`: {err}", ten));
                        continue;
                    }
                };
                let mut deps = Vec::new();
                for (dn, dreq) in &m.deps {
                    match VersionReq::parse(dreq) {
                        Ok(r) => deps.push((dn.clone(), r)),
                        Err(_) => {
                            deps.clear();
                            break;
                        }
                    }
                }
                deps.sort_by(|a, b| a.0.cmp(&b.0));
                ds.push(Muc { version, deps });
            }
        }
        // Giảm dần: ứng viên **cao nhất** được thử trước ⇒ "chọn cao nhất thoả" (VD-125).
        ds.sort_by_key(|m| std::cmp::Reverse(m.version));
        let co: Vec<Version> = ds.iter().map(|m| m.version).rev().collect();
        self.chi_muc.insert(ten.to_string(), ds);
        if co.is_empty() {
            return Err(LoiXungDot {
                ten: ten.to_string(),
                rang_buoc: Vec::new(),
                co,
                loi_manh: self.loi_manh.get(ten).cloned().unwrap_or_default(),
            });
        }
        Ok(())
    }

    /// Các phiên bản **có** của gói (tăng dần).
    pub fn cac_ban(&mut self, ten: &str) -> Vec<Version> {
        let _ = self.nap(ten);
        self.chi_muc
            .get(ten)
            .map(|ds| ds.iter().map(|m| m.version).rev().collect())
            .unwrap_or_default()
    }

    /// Giải một tập ràng buộc **gốc** ⇒ `tên → phiên bản` (mỗi gói **đúng một** bản).
    pub fn giai(
        &mut self,
        goc: &[(String, VersionReq)],
    ) -> Result<BTreeMap<String, Version>, LoiXungDot> {
        self.xung_dot = None;
        let mut rang: BTreeMap<String, Vec<(String, VersionReq)>> = BTreeMap::new();
        for (ten, req) in goc {
            rang.entry(ten.clone())
                .or_default()
                .push(("gốc".to_string(), req.clone()));
        }
        let mut chon: BTreeMap<String, Version> = BTreeMap::new();
        if self.tim(&mut chon, &mut rang)? {
            Ok(chon)
        } else {
            Err(self.xung_dot.take().unwrap_or_else(|| LoiXungDot {
                ten: goc.first().map(|(t, _)| t.clone()).unwrap_or_default(),
                rang_buoc: Vec::new(),
                co: Vec::new(),
                loi_manh: Vec::new(),
            }))
        }
    }

    /// DFS có **quay lui**: chọn gói **tên nhỏ nhất** chưa chọn, thử phiên bản **giảm dần**.
    fn tim(
        &mut self,
        chon: &mut BTreeMap<String, Version>,
        rang: &mut BTreeMap<String, Vec<(String, VersionReq)>>,
    ) -> Result<bool, LoiXungDot> {
        let Some(ten) = rang
            .keys()
            .find(|k| !chon.contains_key(k.as_str()))
            .cloned()
        else {
            return Ok(true); // mọi gói có ràng buộc đã được chọn
        };
        let ds_req = rang.get(&ten).cloned().unwrap_or_default();
        if self.nap(&ten).is_err() {
            // Gói không tồn tại / không có bản hợp lệ: ghi nhận rồi thất bại nhánh này.
            let loi_manh = self.loi_manh.get(&ten).cloned().unwrap_or_default();
            self.ghi_nho(LoiXungDot {
                ten: ten.clone(),
                rang_buoc: ds_req,
                co: Vec::new(),
                loi_manh,
            });
            return Ok(false);
        }
        let ung_vien: Vec<Muc> = self
            .chi_muc
            .get(&ten)
            .map(|ds| {
                ds.iter()
                    .filter(|m| ds_req.iter().all(|(_, r)| r.khop(&m.version)))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();
        if ung_vien.is_empty() {
            let co: Vec<Version> = self
                .chi_muc
                .get(&ten)
                .map(|ds| ds.iter().map(|m| m.version).rev().collect())
                .unwrap_or_default();
            self.ghi_nho(LoiXungDot {
                ten: ten.clone(),
                rang_buoc: ds_req,
                co,
                loi_manh: Vec::new(),
            });
            return Ok(false);
        }
        for m in ung_vien {
            // Ràng buộc mới từ deps của `m`: (tên dep, ràng buộc, nguồn).
            let nguon = format!("{}@{}", ten, m.version);
            let mut them: Vec<(String, VersionReq)> = Vec::new();
            let mut ok = true;
            for (dn, dreq) in &m.deps {
                if let Some(vd) = chon.get(dn) {
                    if !dreq.khop(vd) {
                        // Ràng buộc mới mâu thuẫn với gói **đã chọn** ⇒ nhánh này chết.
                        let mut ds = rang.get(dn).cloned().unwrap_or_default();
                        ds.push((nguon.clone(), dreq.clone()));
                        let co = self.cac_ban(dn);
                        self.ghi_nho(LoiXungDot {
                            ten: dn.clone(),
                            rang_buoc: ds,
                            co,
                            loi_manh: Vec::new(),
                        });
                        ok = false;
                        break;
                    }
                }
                them.push((dn.clone(), dreq.clone()));
            }
            if ok {
                chon.insert(ten.clone(), m.version);
                for (dn, dreq) in &them {
                    rang.entry(dn.clone())
                        .or_default()
                        .push((nguon.clone(), dreq.clone()));
                }
                if self.tim(chon, rang)? {
                    return Ok(true);
                }
                // Quay lui: bỏ ràng buộc đã thêm (đúng thứ tự ngược) + bỏ lựa chọn.
                for (dn, _) in them.iter().rev() {
                    if let Some(v) = rang.get_mut(dn) {
                        v.pop();
                        if v.is_empty() {
                            rang.remove(dn);
                        }
                    }
                }
                chon.remove(&ten);
            }
        }
        Ok(false)
    }

    /// Giữ xung đột **nhiều ràng buộc nhất** (tie: tên nhỏ hơn) ⇒ thông điệp ổn định giữa các lần chạy.
    fn ghi_nho(&mut self, xd: LoiXungDot) {
        let thay = match &self.xung_dot {
            None => true,
            Some(cu) => {
                xd.rang_buoc.len() > cu.rang_buoc.len()
                    || (xd.rang_buoc.len() == cu.rang_buoc.len() && xd.ten < cu.ten)
            }
        };
        if thay {
            self.xung_dot = Some(xd);
        }
    }
}

/// Phiên bản **cao nhất** của **một** gói thoả `req` (không xét deps) — dùng cho đường cũ
/// `pkg update` (ST-128 giữ hành vi, ST-129 mới giải lại toàn đồ thị).
pub fn cao_nhat_thoa(reg: &Path, ten: &str, req: &VersionReq) -> Result<Version, LoiXungDot> {
    let mut r = Resolver::moi(reg);
    r.nap(ten)?;
    let ds = r.chi_muc.get(ten).cloned().unwrap_or_default();
    let co: Vec<Version> = ds.iter().map(|m| m.version).rev().collect();
    ds.iter()
        .find(|m| req.khop(&m.version))
        .map(|m| m.version)
        .ok_or_else(|| LoiXungDot {
            ten: ten.to_string(),
            rang_buoc: vec![("gốc".to_string(), req.clone())],
            co,
            loi_manh: r.loi_manh.get(ten).cloned().unwrap_or_default(),
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn tam(tag: &str) -> PathBuf {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static DEM: AtomicUsize = AtomicUsize::new(0);
        let n = DEM.fetch_add(1, Ordering::Relaxed);
        let d =
            std::env::temp_dir().join(format!("vis-resolver-{tag}-{}-{}", std::process::id(), n));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).expect("tmp");
        d
    }

    /// Ghi một gói vào registry: `registry/<tên>/<ver>/manifest.json` + `src/main.vis`.
    fn goi(reg: &Path, ten: &str, ver: &str, deps: &[(&str, &str)]) {
        let dir = reg.join(ten).join(ver);
        fs::create_dir_all(dir.join("src")).expect("mkdir");
        let noi_dung =
            "# SPDX-License-Identifier: MIT OR Apache-2.0\n\npub fn f() -> int:\n    1\n"
                .to_string();
        fs::write(dir.join("src/main.vis"), &noi_dung).expect("ghi src");
        let sha = super::super::bam(&noi_dung);
        let dep_json = deps
            .iter()
            .map(|(n, r)| format!("\"{n}\":\"{r}\""))
            .collect::<Vec<_>>()
            .join(",");
        let m = format!(
            "{{\"deps\":{{{dep_json}}},\"entry\":\"src/main.vis\",\"files\":[{{\"path\":\"src/main.vis\",\"sha256\":\"{sha}\"}}],\"name\":\"{ten}\",\"version\":\"{ver}\"}}"
        );
        fs::write(dir.join("manifest.json"), m).expect("ghi manifest");
    }

    fn req(s: &str) -> VersionReq {
        VersionReq::parse(s).unwrap_or_else(|e| panic!("req {s:?}: {e}"))
    }

    fn v(s: &str) -> Version {
        Version::parse(s).unwrap_or_else(|e| panic!("version {s:?}: {e}"))
    }

    /// In lời giải thành chuỗi `tên@ver` sắp theo tên — để so **byte** giữa hai lần chạy.
    fn chuoi(giai: &BTreeMap<String, Version>) -> String {
        let mut s = String::new();
        for (k, ver) in giai {
            s.push_str(k);
            s.push('@');
            s.push_str(&ver.to_string());
            s.push('\n');
        }
        s
    }

    // ----- 1..3: chọn cao nhất -----

    #[test]
    fn chon_cao_nhat_khi_moi_ban_deu_thoa() {
        let d = tam("cao-nhat");
        let reg = d.join("registry");
        goi(&reg, "a", "1.0.0", &[]);
        goi(&reg, "a", "1.9.0", &[]);
        goi(&reg, "a", "1.10.0", &[]);
        let g = Resolver::moi(&reg)
            .giai(&[("a".into(), req(">=0.0.0"))])
            .unwrap();
        assert_eq!(g["a"], v("1.10.0"), "1.10.0 > 1.9.0 theo **số**");
        assert_eq!(g.len(), 1, "mỗi gói **đúng một** phiên bản");
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn rang_buoc_thu_hep_khoang() {
        let d = tam("khoang");
        let reg = d.join("registry");
        goi(&reg, "a", "1.0.0", &[]);
        goi(&reg, "a", "1.5.0", &[]);
        goi(&reg, "a", "2.0.0", &[]);
        let g = Resolver::moi(&reg)
            .giai(&[("a".into(), req("^1.2"))])
            .unwrap();
        assert_eq!(g["a"], v("1.5.0"));
        let g = Resolver::moi(&reg)
            .giai(&[("a".into(), req("<1.5"))])
            .unwrap();
        assert_eq!(g["a"], v("1.0.0"));
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn caret_0x_trong_deps() {
        let d = tam("caret0");
        let reg = d.join("registry");
        goi(&reg, "a", "1.0.0", &[("b", "^0.1")]);
        goi(&reg, "b", "0.1.9", &[]);
        goi(&reg, "b", "0.2.0", &[]);
        let g = Resolver::moi(&reg)
            .giai(&[("a".into(), req(">=0.0.0"))])
            .unwrap();
        assert_eq!(g["a"], v("1.0.0"));
        assert_eq!(g["b"], v("0.1.9"), "^0.1 ⇒ <0.2.0 (luật ^0.x)");
        let _ = fs::remove_dir_all(&d);
    }

    // ----- 4..6: phụ thuộc bắc cầu + một phiên bản mỗi gói -----

    #[test]
    fn phu_thuoc_bac_cau() {
        let d = tam("bac-cau");
        let reg = d.join("registry");
        goi(&reg, "a", "1.0.0", &[("b", "^1")]);
        goi(&reg, "b", "1.2.0", &[("c", "^2")]);
        goi(&reg, "c", "2.3.0", &[]);
        let g = Resolver::moi(&reg)
            .giai(&[("a".into(), req(">=0.0.0"))])
            .unwrap();
        assert_eq!(g.len(), 3);
        assert_eq!(g["b"], v("1.2.0"));
        assert_eq!(g["c"], v("2.3.0"));
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn hai_goc_dung_chung_mot_dep_chi_mot_ban() {
        let d = tam("chung");
        let reg = d.join("registry");
        goi(&reg, "a", "1.0.0", &[("d", "^1")]);
        goi(&reg, "b", "1.0.0", &[("d", ">=1.1")]);
        goi(&reg, "d", "1.1.0", &[]);
        goi(&reg, "d", "1.9.0", &[]);
        let g = Resolver::moi(&reg)
            .giai(&[("a".into(), req("^1")), ("b".into(), req("^1"))])
            .unwrap();
        assert_eq!(g["d"], v("1.9.0"), "cao nhất thoả **cả hai** ràng buộc");
        assert_eq!(
            g.keys().filter(|k| k.as_str() == "d").count(),
            1,
            "đúng một phiên bản `d`"
        );
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn rang_buoc_and_tu_ba_goc() {
        let d = tam("and3");
        let reg = d.join("registry");
        for (ten, deps) in [
            ("a", vec![("d", ">=1.2")]),
            ("b", vec![("d", "<1.5")]),
            ("c", vec![("d", "!=x")]),
        ] {
            if ten == "c" {
                // `!=` bị từ chối ⇒ manifest hỏng ⇒ resolver bỏ qua `c` (không chọn bản hỏng).
                let dir = reg.join("c").join("1.0.0");
                fs::create_dir_all(dir.join("src")).unwrap();
                fs::write(dir.join("src/main.vis"), "x\n").unwrap();
                fs::write(
                    dir.join("manifest.json"),
                    "{\"deps\":{\"d\":\"!=1.0.0\"},\"entry\":\"src/main.vis\",\"files\":[{\"path\":\"src/main.vis\",\"sha256\":\"00\"}],\"name\":\"c\",\"version\":\"1.0.0\"}",
                )
                .unwrap();
                continue;
            }
            goi(&reg, ten, "1.0.0", &deps);
        }
        goi(&reg, "d", "1.2.0", &[]);
        goi(&reg, "d", "1.4.9", &[]);
        goi(&reg, "d", "1.5.0", &[]);
        let g = Resolver::moi(&reg)
            .giai(&[("a".into(), req("^1")), ("b".into(), req("^1"))])
            .unwrap();
        assert_eq!(g["d"], v("1.4.9"), "AND: >=1.2 và <1.5 ⇒ 1.4.9");
        let _ = fs::remove_dir_all(&d);
    }

    // ----- 7..8: quay lui -----

    #[test]
    fn quay_lui_khi_ban_cao_nhat_dan_toi_ngo_cut() {
        let d = tam("quay-lui");
        let reg = d.join("registry");
        // a phụ thuộc b ^1 (cho phép b 1.0.0 và 2.x? Không: ^1 ⇒ <2.0.0) ⇒ dùng ràng buộc rộng hơn.
        goi(&reg, "a", "1.0.0", &[("b", ">=1.0.0")]);
        // b 2.0.0 cần c ^9 (không có) ⇒ nhánh chết; b 1.0.0 cần c ^1 ⇒ sống.
        goi(&reg, "b", "2.0.0", &[("c", "^9")]);
        goi(&reg, "b", "1.0.0", &[("c", "^1")]);
        goi(&reg, "c", "1.2.0", &[]);
        let g = Resolver::moi(&reg)
            .giai(&[("a".into(), req("^1"))])
            .unwrap();
        assert_eq!(g["b"], v("1.0.0"), "phải **quay lui** khỏi b 2.0.0");
        assert_eq!(g["c"], v("1.2.0"));
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn quay_lui_hai_tang() {
        let d = tam("quay-lui-2");
        let reg = d.join("registry");
        goi(&reg, "a", "1.0.0", &[("b", ">=1")]);
        goi(&reg, "b", "2.0.0", &[("c", "^9")]);
        goi(&reg, "b", "1.5.0", &[("c", ">=2"), ("d", "^1")]);
        goi(&reg, "c", "2.0.0", &[]);
        goi(&reg, "c", "1.0.0", &[]);
        goi(&reg, "d", "1.0.0", &[]);
        let g = Resolver::moi(&reg)
            .giai(&[("a".into(), req("^1"))])
            .unwrap();
        assert_eq!(g["b"], v("1.5.0"));
        assert_eq!(g["c"], v("2.0.0"));
        assert_eq!(g["d"], v("1.0.0"));
        let _ = fs::remove_dir_all(&d);
    }

    // ----- 9..11: xung đột + chuỗi ràng buộc -----

    #[test]
    fn xung_dot_neu_chuoi_rang_buoc() {
        let d = tam("xung-dot");
        let reg = d.join("registry");
        goi(&reg, "a", "1.0.0", &[("d", "^2")]);
        goi(&reg, "b", "1.0.0", &[("d", "<1.5")]);
        goi(&reg, "d", "1.4.0", &[]);
        goi(&reg, "d", "2.0.0", &[]);
        let e = Resolver::moi(&reg)
            .giai(&[("a".into(), req("^1")), ("b".into(), req("^1"))])
            .unwrap_err();
        assert_eq!(e.ten, "d");
        let chuoi = e.chuoi_rang_buoc();
        assert!(chuoi.contains("^2") && chuoi.contains("<1.5"), "{chuoi}");
        assert!(
            chuoi.contains("a@1.0.0") && chuoi.contains("b@1.0.0"),
            "phải nêu **nguồn** hai ràng buộc: {chuoi}"
        );
        let thong_diep = e.to_string();
        assert!(
            thong_diep.contains("1.4.0") && thong_diep.contains("2.0.0"),
            "{thong_diep}"
        );
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn xung_dot_voi_goc() {
        let d = tam("xung-dot-goc");
        let reg = d.join("registry");
        goi(&reg, "a", "1.0.0", &[("d", "^2")]);
        goi(&reg, "d", "1.0.0", &[]);
        goi(&reg, "d", "2.0.0", &[]);
        let e = Resolver::moi(&reg)
            .giai(&[("a".into(), req("^1")), ("d".into(), req("<1.5"))])
            .unwrap_err();
        assert_eq!(e.ten, "d");
        let chuoi = e.chuoi_rang_buoc();
        assert!(chuoi.contains("gốc") && chuoi.contains("^2"), "{chuoi}");
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn goi_khong_co_trong_registry() {
        let d = tam("khong-co");
        let reg = d.join("registry");
        fs::create_dir_all(&reg).unwrap();
        let e = Resolver::moi(&reg)
            .giai(&[("vang".into(), req(">=0.0.0"))])
            .unwrap_err();
        assert_eq!(e.ten, "vang");
        assert!(e.co.is_empty());
        assert!(e.to_string().contains("không có phiên bản"), "{e}");
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn khong_co_ban_nao_thoa_chi_ke_cac_ban_co() {
        let d = tam("khong-thoa");
        let reg = d.join("registry");
        goi(&reg, "a", "1.0.0", &[]);
        goi(&reg, "a", "1.2.0", &[]);
        let e = Resolver::moi(&reg)
            .giai(&[("a".into(), req("^2"))])
            .unwrap_err();
        assert_eq!(e.ten, "a");
        assert_eq!(e.co, vec![v("1.0.0"), v("1.2.0")], "liệt kê bản **có**");
        assert!(e.chuoi_rang_buoc().contains("^2"));
        let _ = fs::remove_dir_all(&d);
    }

    // ----- 12..14: tất định + bỏ qua bản không hợp lệ + vòng -----

    #[test]
    fn tat_dinh_hai_lan_cung_byte() {
        let d = tam("tat-dinh");
        let reg = d.join("registry");
        goi(&reg, "a", "1.0.0", &[("b", "^1"), ("c", ">=1.0")]);
        goi(&reg, "b", "1.0.0", &[("c", "^1")]);
        goi(&reg, "b", "1.1.0", &[]);
        goi(&reg, "c", "1.0.0", &[]);
        goi(&reg, "c", "1.1.0", &[]);
        let goc = [("a".to_string(), req(">=0.0.0"))];
        let l1 = Resolver::moi(&reg).giai(&goc).unwrap();
        let l2 = Resolver::moi(&reg).giai(&goc).unwrap();
        assert_eq!(chuoi(&l1), chuoi(&l2), "hai lần giải ⇒ **cùng byte**");
        assert_eq!(l1, l2);
        assert_eq!(
            chuoi(&l1),
            "a@1.0.0\nb@1.1.0\nc@1.1.0\n",
            "lời giải tất định, sắp theo tên"
        );
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn bo_qua_ban_khong_hop_le_vd125() {
        let d = tam("khong-hop-le");
        let reg = d.join("registry");
        goi(&reg, "a", "1.0.0", &[]);
        // prerelease + thiếu thành phần: **không** phải phiên bản VS2 ⇒ không được chọn.
        for ten_ban in ["1.1.0-beta", "1.2", "2"] {
            let dir = reg.join("a").join(ten_ban);
            fs::create_dir_all(dir.join("src")).unwrap();
            fs::write(dir.join("src/main.vis"), "x\n").unwrap();
            fs::write(
                dir.join("manifest.json"),
                format!("{{\"deps\":{{}},\"entry\":\"src/main.vis\",\"files\":[{{\"path\":\"src/main.vis\",\"sha256\":\"00\"}}],\"name\":\"a\",\"version\":\"{ten_ban}\"}}"),
            )
            .unwrap();
        }
        let g = Resolver::moi(&reg)
            .giai(&[("a".into(), req(">=0.0.0"))])
            .unwrap();
        assert_eq!(g["a"], v("1.0.0"), "chỉ bản hợp lệ mới được chọn");
        let co = Resolver::moi(&reg).cac_ban("a");
        assert_eq!(co, vec![v("1.0.0")]);
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn vong_phu_thuoc_xu_ly_duoc() {
        let d = tam("vong");
        let reg = d.join("registry");
        goi(&reg, "a", "1.0.0", &[("a", "^1"), ("b", "^1")]);
        goi(&reg, "b", "1.0.0", &[("a", "^1")]);
        let g = Resolver::moi(&reg)
            .giai(&[("a".into(), req("^1"))])
            .unwrap();
        assert_eq!(g.len(), 2);
        assert_eq!(g["a"], v("1.0.0"));
        assert_eq!(g["b"], v("1.0.0"));
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn khong_phu_thuoc_chi_tra_ve_goc() {
        let d = tam("khong-dep");
        let reg = d.join("registry");
        goi(&reg, "a", "1.0.0", &[]);
        let g = Resolver::moi(&reg)
            .giai(&[("a".into(), req("=1.0.0"))])
            .unwrap();
        assert_eq!(chuoi(&g), "a@1.0.0\n");
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn cao_nhat_thoa_dung_cho_mot_goi() {
        let d = tam("cao-nhat-thoa");
        let reg = d.join("registry");
        goi(&reg, "x", "1.0.0", &[]);
        goi(&reg, "x", "1.4.0", &[]);
        goi(&reg, "x", "2.0.0", &[]);
        assert_eq!(cao_nhat_thoa(&reg, "x", &req("^1")).unwrap(), v("1.4.0"));
        assert!(cao_nhat_thoa(&reg, "x", &req("^3")).is_err());
        let _ = fs::remove_dir_all(&d);
    }
}
