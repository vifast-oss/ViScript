// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-127 (VD-125)** — semver tối giản cho registry cục bộ của `vis pkg`: [`Version`],
//! [`VersionReq`] và so khớp ràng buộc.
//!
//! # Ngữ nghĩa (chốt ở VD-125, **không** tự thêm)
//!
//! - Phiên bản là `MAJOR.MINOR.PATCH` **số nguyên**; **prerelease/build metadata bị từ chối** kèm
//!   chẩn đoán ([`LoiSemver`]) — VD-125.
//! - Toán tử: `^` `~` `=` `>` `>=` `<` `<=`; **dấu phẩy = AND**; **không có toán tử ⇒ hiểu là `^`**
//!   (giống Cargo, **kể cả** luật `^0.x`).
//! - Không có `||`, không `*`/`x`, không khoảng `a - b`, không `!=` — các dạng đó **từ chối**.
//! - Thiếu thành phần (`1`, `1.2`) **được phép trong ràng buộc** (điền 0 khi so sánh); với `^`/`~`
//!   luật chặn trên của Cargo phụ thuộc **số thành phần đã ghi** (`^0` = `<1.0.0`, `^0.0` = `<0.1.0`),
//!   nên [`Khoang`] giữ `n` = số thành phần.
//!
//! Không crate mới (tự viết parser), không `unsafe`, không I/O.

#![allow(dead_code)] // `VersionReq`/`Khoang` chưa nối vào đường chạy ở ST-127 (resolver dùng ở
                     // ST-128, lock ghi `version_req` ở ST-129); bỏ `allow` này khi đã nối (JOURNAL ST-127).

use std::fmt;

/// Lỗi cú pháp semver: giữ **nguyên văn** chuỗi gốc + lý do (tiếng Việt). Không panic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoiSemver {
    /// Chuỗi gốc đã nhận (nguyên văn, kể cả khoảng trắng) — để thông điệp chỉ đúng chỗ sai.
    pub goc: String,
    /// Lý do (tiếng Việt).
    pub ly_do: String,
}

impl LoiSemver {
    fn moi(goc: &str, ly_do: impl Into<String>) -> Self {
        Self {
            goc: goc.to_string(),
            ly_do: ly_do.into(),
        }
    }
}

impl fmt::Display for LoiSemver {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "`{}`: {}", self.goc, self.ly_do)
    }
}

impl std::error::Error for LoiSemver {}

/// Câu chẩn đoán chung cho prerelease/build metadata (VD-125).
pub const LY_DO_PRERELEASE: &str =
    "prerelease/build metadata bị từ chối — phiên bản VS2 chỉ có MAJOR.MINOR.PATCH số nguyên (VD-125)";

/// Tách `MAJOR[.MINOR[.PATCH]]` — trả `(major, minor, patch, số thành phần đã ghi)`.
///
/// `n` cần cho luật chặn trên của `^`/`~` (giống Cargo): `^0` khác `^0.0` khác `^0.0.0`.
fn tac_so(goc: &str, s: &str) -> Result<(u64, u64, u64, u8), LoiSemver> {
    if s.is_empty() {
        return Err(LoiSemver::moi(goc, "thiếu số phiên bản"));
    }
    if s.contains('-') || s.contains('+') {
        return Err(LoiSemver::moi(goc, LY_DO_PRERELEASE));
    }
    let phan: Vec<&str> = s.split('.').collect();
    if phan.len() > 3 {
        return Err(LoiSemver::moi(
            goc,
            format!("quá 3 thành phần (`{}`)", phan.len()),
        ));
    }
    let mut so = [0u64; 3];
    for (i, p) in phan.iter().enumerate() {
        if p.is_empty() {
            return Err(LoiSemver::moi(goc, "thành phần rỗng (`.`)"));
        }
        if !p.bytes().all(|b| b.is_ascii_digit()) {
            return Err(LoiSemver::moi(
                goc,
                format!("thành phần `{p}` không phải số nguyên"),
            ));
        }
        if p.len() > 1 && p.starts_with('0') {
            return Err(LoiSemver::moi(goc, format!("số `{p}` có 0 đứng đầu")));
        }
        so[i] = p
            .parse::<u64>()
            .map_err(|_| LoiSemver::moi(goc, format!("số `{p}` tràn u64")))?;
    }
    Ok((so[0], so[1], so[2], phan.len() as u8))
}

/// Phiên bản `MAJOR.MINOR.PATCH` — **đúng ba** thành phần, số nguyên, không prerelease.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Version {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
}

impl Version {
    /// Tạo từ ba số.
    pub fn moi(major: u64, minor: u64, patch: u64) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    /// Đọc `MAJOR.MINOR.PATCH` (bỏ khoảng trắng hai đầu); **đỏ** nếu thiếu/thừa thành phần, không
    /// phải số, có 0 đứng đầu, tràn `u64`, hoặc có prerelease/build metadata.
    pub fn parse(s: &str) -> Result<Self, LoiSemver> {
        let t = s.trim();
        let (major, minor, patch, n) = tac_so(s, t)?;
        if n != 3 {
            return Err(LoiSemver::moi(
                s,
                format!("phiên bản phải đủ MAJOR.MINOR.PATCH (nhận {n} thành phần)"),
            ));
        }
        Ok(Self {
            major,
            minor,
            patch,
        })
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// Toán tử so khớp — **đúng bảy** dạng của VD-125.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToanTu {
    Caret,
    Tilde,
    Bang,
    Lon,
    LonBang,
    Nho,
    NhoBang,
}

impl ToanTu {
    /// Dấu in ra (dạng chuẩn hoá trong lock).
    pub fn dau(self) -> &'static str {
        match self {
            ToanTu::Caret => "^",
            ToanTu::Tilde => "~",
            ToanTu::Bang => "=",
            ToanTu::Lon => ">",
            ToanTu::LonBang => ">=",
            ToanTu::Nho => "<",
            ToanTu::NhoBang => "<=",
        }
    }
}

/// Một ràng buộc đơn: `toán tử` + `MAJOR[.MINOR[.PATCH]]` + **số thành phần đã ghi** `n`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Khoang {
    pub tt: ToanTu,
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
    /// Số thành phần **đã ghi** (1..=3) — quyết định chặn trên của `^`/`~` và dạng in chuẩn hoá.
    pub n: u8,
}

impl Khoang {
    /// Cận dưới (đã điền 0 cho thành phần thiếu).
    fn san(&self) -> Version {
        Version::moi(self.major, self.minor, self.patch)
    }

    /// Chặn trên **không kể** — chỉ `^`/`~` có; luật Cargo, phụ thuộc `n`.
    fn tren(self) -> Option<Version> {
        match self.tt {
            ToanTu::Caret => Some(match self.n {
                // `^0` ⇒ <1.0.0
                1 => Version::moi(self.major + 1, 0, 0),
                // `^0.0` ⇒ <0.1.0; `^1.2` ⇒ <2.0.0
                2 if self.major == 0 => Version::moi(0, self.minor + 1, 0),
                2 => Version::moi(self.major + 1, 0, 0),
                // `^0.2.3` ⇒ <0.3.0; `^0.0.3` ⇒ <0.0.4; `^1.2.3` ⇒ <2.0.0
                _ if self.major > 0 => Version::moi(self.major + 1, 0, 0),
                _ if self.minor > 0 => Version::moi(0, self.minor + 1, 0),
                _ => Version::moi(0, 0, self.patch + 1),
            }),
            ToanTu::Tilde => Some(if self.n == 1 {
                // `~1` ⇒ <2.0.0
                Version::moi(self.major + 1, 0, 0)
            } else {
                // `~1.2` / `~1.2.3` / `~0.0.3` ⇒ <major.(minor+1).0
                Version::moi(self.major, self.minor + 1, 0)
            }),
            _ => None,
        }
    }

    /// Phiên bản `v` có thoả **một** ràng buộc này không.
    pub fn khop(&self, v: &Version) -> bool {
        let san = self.san();
        match self.tt {
            ToanTu::Bang => *v == san,
            ToanTu::Lon => *v > san,
            ToanTu::LonBang => *v >= san,
            ToanTu::Nho => *v < san,
            ToanTu::NhoBang => *v <= san,
            ToanTu::Caret | ToanTu::Tilde => {
                // `tren()` luôn `Some` cho hai toán tử này.
                *v >= san && self.tren().is_some_and(|t| *v < t)
            }
        }
    }
}

impl fmt::Display for Khoang {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.tt.dau())?;
        match self.n {
            1 => write!(f, "{}", self.major),
            2 => write!(f, "{}.{}", self.major, self.minor),
            _ => write!(f, "{}.{}.{}", self.major, self.minor, self.patch),
        }
    }
}

/// Tập ràng buộc **AND** (dấu phẩy) — `VersionReq::khop` = **mọi** khoang đều thoả.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionReq {
    ds: Vec<Khoang>,
}

impl VersionReq {
    /// Một ràng buộc **chặt** (không thoả mãn gì) — hữu ích cho ca xung đột; in ra `>…`? Không:
    /// dùng `parse("^0.0.0")` khi cần; hàm này chỉ tạo từ danh sách đã có.
    pub fn tu_ds(ds: Vec<Khoang>) -> Self {
        Self { ds }
    }

    /// Đọc chuỗi ràng buộc: `^1.2.3, <2.0.0`; **không** toán tử ⇒ `^`; **dấu phẩy = AND**.
    ///
    /// Từ chối: rỗng, `||`, khoảng `a - b`, `!=`, `*`/`x`, thành phần rỗng giữa hai dấu phẩy,
    /// prerelease/build metadata.
    pub fn parse(s: &str) -> Result<Self, LoiSemver> {
        let t = s.trim();
        if t.is_empty() {
            return Err(LoiSemver::moi(s, "ràng buộc rỗng"));
        }
        if t.contains("||") {
            return Err(LoiSemver::moi(
                s,
                "chỉ hỗ trợ AND (dấu phẩy); `||` không có trong VS2 (VD-125)",
            ));
        }
        if t.contains(" - ") {
            return Err(LoiSemver::moi(
                s,
                "khoảng `a - b` không hỗ trợ (VD-125: chỉ ^ ~ = > >= < <= và dấu phẩy = AND)",
            ));
        }
        let mut ds = Vec::new();
        for phan in t.split(',') {
            let p = phan.trim();
            if p.is_empty() {
                return Err(LoiSemver::moi(s, "thành phần rỗng giữa hai dấu phẩy"));
            }
            ds.push(doc_khoang(s, p)?);
        }
        Ok(Self { ds })
    }

    /// Danh sách khoang (giữ **thứ tự đã ghi** — in chuẩn hoá tất định).
    pub fn ds(&self) -> &[Khoang] {
        &self.ds
    }

    /// `v` có thoả **mọi** ràng buộc không (AND).
    pub fn khop(&self, v: &Version) -> bool {
        self.ds.iter().all(|k| k.khop(v))
    }

    /// Chuỗi **chuẩn hoá** (dùng cho `version_req` trong lock): `>=1.0.0, <2.0.0`.
    pub fn chuan_hoa(&self) -> String {
        self.to_string()
    }
}

impl fmt::Display for VersionReq {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, k) in self.ds.iter().enumerate() {
            if i > 0 {
                f.write_str(", ")?;
            }
            write!(f, "{k}")?;
        }
        Ok(())
    }
}

/// Đọc **một** khoang: tách toán tử (không có ⇒ `^`), rồi đọc số.
fn doc_khoang(goc: &str, p: &str) -> Result<Khoang, LoiSemver> {
    let (tt, con) = if let Some(r) = p.strip_prefix(">=") {
        (ToanTu::LonBang, r)
    } else if let Some(r) = p.strip_prefix("<=") {
        (ToanTu::NhoBang, r)
    } else if let Some(r) = p.strip_prefix('>') {
        (ToanTu::Lon, r)
    } else if let Some(r) = p.strip_prefix('<') {
        (ToanTu::Nho, r)
    } else if let Some(r) = p.strip_prefix('^') {
        (ToanTu::Caret, r)
    } else if let Some(r) = p.strip_prefix('~') {
        (ToanTu::Tilde, r)
    } else if let Some(r) = p.strip_prefix('=') {
        (ToanTu::Bang, r)
    } else {
        // VD-125: **không** toán tử ⇒ hiểu là `^` (giống Cargo).
        (ToanTu::Caret, p)
    };
    if con.starts_with('!') || p.starts_with('!') {
        return Err(LoiSemver::moi(goc, "`!=` không hỗ trợ (VD-125)"));
    }
    if con == "*" || con.eq_ignore_ascii_case("x") {
        return Err(LoiSemver::moi(
            goc,
            "`*`/`x` không hỗ trợ (VD-125: ghi ràng buộc tường minh)",
        ));
    }
    if con.is_empty() {
        return Err(LoiSemver::moi(
            goc,
            format!("thiếu số sau toán tử `{}`", tt.dau()),
        ));
    }
    let (major, minor, patch, n) = tac_so(goc, con)?;
    Ok(Khoang {
        tt,
        major,
        minor,
        patch,
        n,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(s: &str) -> Version {
        Version::parse(s).unwrap_or_else(|e| panic!("Version::parse({s:?}) đỏ: {e}"))
    }

    fn req(s: &str) -> VersionReq {
        VersionReq::parse(s).unwrap_or_else(|e| panic!("VersionReq::parse({s:?}) đỏ: {e}"))
    }

    fn khop(s: &str, phien_ban: &str) -> bool {
        req(s).khop(&v(phien_ban))
    }

    // ----- 1..16: Version::parse -----

    #[test]
    fn version_doc_binh_thuong() {
        assert_eq!(v("1.2.3"), Version::moi(1, 2, 3));
        assert_eq!(v("0.0.0"), Version::moi(0, 0, 0));
        assert_eq!(v("10.20.30"), Version::moi(10, 20, 30));
        assert_eq!(v(" 1.2.3 "), Version::moi(1, 2, 3), "bỏ khoảng trắng");
        assert_eq!(v("18446744073709551615.0.0").major, u64::MAX);
    }

    #[test]
    fn version_tu_choi_thieu_thua_thanh_phan() {
        for s in ["1.2", "1", "", "1.2.3.4", "1..3", "1.2."] {
            assert!(Version::parse(s).is_err(), "phải từ chối {s:?}");
        }
        let e = Version::parse("1.2").unwrap_err().to_string();
        assert!(e.contains("MAJOR.MINOR.PATCH"), "{e}");
        assert!(e.contains("1.2"), "phải nêu chuỗi gốc: {e}");
    }

    #[test]
    fn version_tu_choi_prerelease_va_build() {
        for s in ["1.2.3-beta", "1.2.3-beta.1", "1.2.3+build5", "1.0.0-rc.1+x"] {
            let e = Version::parse(s).unwrap_err();
            assert_eq!(e.ly_do, LY_DO_PRERELEASE, "ca {s:?}");
        }
        // Chẩn đoán phải nêu **đúng** chuỗi gốc + VD-125.
        let e = Version::parse("1.2.3-beta").unwrap_err().to_string();
        assert!(e.contains("1.2.3-beta") && e.contains("VD-125"), "{e}");
    }

    #[test]
    fn version_tu_choi_khong_phai_so_va_so_0_dau() {
        for s in ["1.2.x", "1.-2.3", "01.2.3", "1.02.3", "1.2.03", "a.b.c"] {
            assert!(Version::parse(s).is_err(), "phải từ chối {s:?}");
        }
        assert!(Version::parse("01.2.3")
            .unwrap_err()
            .ly_do
            .contains("0 đứng đầu"));
    }

    #[test]
    fn version_in_ra_va_thu_tu() {
        assert_eq!(v("1.2.3").to_string(), "1.2.3");
        assert!(v("1.2.3") < v("1.3.0"));
        assert!(v("1.10.0") > v("1.9.0"), "so theo **số**, không theo chuỗi");
        assert!(v("2.0.0") > v("1.99.99"));
    }

    // ----- 17..24: VersionReq::parse lỗi -----

    #[test]
    fn req_tu_choi_cu_phap_la() {
        for s in [
            "",
            "^",
            "^1.2.3 || ^2.0.0",
            "*",
            "x",
            "!=1.0.0",
            "1.2.3 - 2.0.0",
            ">=1.0.0,",
            ",>=1.0.0",
            ">=1.0.0,,<2.0.0",
            "^1.2.3-beta",
        ] {
            assert!(VersionReq::parse(s).is_err(), "phải từ chối {s:?}");
        }
    }

    #[test]
    fn req_loi_neu_dung_ly_do() {
        let e = VersionReq::parse("^1.0.0 || ^2.0.0").unwrap_err();
        assert!(
            e.ly_do.contains("||") && e.ly_do.contains("dấu phẩy"),
            "{e}"
        );
        let e = VersionReq::parse("*").unwrap_err();
        assert!(e.ly_do.contains("*"), "{e}");
        let e = VersionReq::parse(">=1.0.0,").unwrap_err();
        assert!(e.ly_do.contains("dấu phẩy"), "{e}");
        let e = VersionReq::parse("^1.2.3-beta").unwrap_err();
        assert_eq!(e.ly_do, LY_DO_PRERELEASE);
    }

    // ----- 25..37: từng toán tử -----

    #[test]
    fn caret_thuong() {
        assert!(khop("^1.2.3", "1.2.3"));
        assert!(khop("^1.2.3", "1.9.9"));
        assert!(khop("^1.2.3", "1.2.4"));
        assert!(!khop("^1.2.3", "1.2.2"));
        assert!(!khop("^1.2.3", "2.0.0"));
        assert!(!khop("^1.2.3", "0.9.9"));
    }

    #[test]
    fn caret_0x_theo_cargo() {
        // ^0.2.3 ⇒ >=0.2.3 <0.3.0
        assert!(khop("^0.2.3", "0.2.3"));
        assert!(khop("^0.2.3", "0.2.9"));
        assert!(!khop("^0.2.3", "0.3.0"));
        assert!(!khop("^0.2.3", "0.2.2"));
        // ^0.0.3 ⇒ >=0.0.3 <0.0.4
        assert!(khop("^0.0.3", "0.0.3"));
        assert!(!khop("^0.0.3", "0.0.4"));
        // ^0.0 ⇒ >=0.0.0 <0.1.0
        assert!(khop("^0.0", "0.0.9"));
        assert!(!khop("^0.0", "0.1.0"));
        // ^0 ⇒ >=0.0.0 <1.0.0
        assert!(khop("^0", "0.9.9"));
        assert!(!khop("^0", "1.0.0"));
        // ^0.0.0 ⇒ chỉ đúng 0.0.0
        assert!(khop("^0.0.0", "0.0.0"));
        assert!(!khop("^0.0.0", "0.0.1"));
    }

    #[test]
    fn tilde_theo_cargo() {
        // ~1.2.3 ⇒ >=1.2.3 <1.3.0
        assert!(khop("~1.2.3", "1.2.3"));
        assert!(khop("~1.2.3", "1.2.9"));
        assert!(!khop("~1.2.3", "1.3.0"));
        assert!(!khop("~1.2.3", "1.2.2"));
        // ~1.2 ⇒ >=1.2.0 <1.3.0
        assert!(khop("~1.2", "1.2.0"));
        assert!(!khop("~1.2", "1.3.0"));
        // ~1 ⇒ >=1.0.0 <2.0.0
        assert!(khop("~1", "1.9.0"));
        assert!(!khop("~1", "2.0.0"));
        // ~0.0.3 ⇒ >=0.0.3 <0.1.0
        assert!(khop("~0.0.3", "0.0.9"));
        assert!(!khop("~0.0.3", "0.1.0"));
        // ~0.0 ⇒ >=0.0.0 <0.1.0
        assert!(khop("~0.0", "0.0.0"));
        assert!(!khop("~0.0", "0.1.0"));
    }

    #[test]
    fn bang_dung_chinh_xac() {
        assert!(khop("=1.2.3", "1.2.3"));
        assert!(!khop("=1.2.3", "1.2.4"));
        assert!(!khop("=1.2.3", "1.2.2"));
        // `=` với phiên bản thiếu thành phần ⇒ điền 0.
        assert!(khop("=1.2", "1.2.0"));
        assert!(!khop("=1.2", "1.2.1"));
        assert!(khop("=1", "1.0.0"));
    }

    #[test]
    fn lon_va_lon_bang() {
        assert!(khop(">1.2.3", "1.2.4"));
        assert!(!khop(">1.2.3", "1.2.3"));
        assert!(!khop(">1.2.3", "1.2.2"));
        assert!(khop(">=1.2.3", "1.2.3"));
        assert!(khop(">=1.2.3", "9.0.0"));
        assert!(!khop(">=1.2.3", "1.2.2"));
        // `>1.2` = lớn hơn 1.2.0.
        assert!(khop(">1.2", "1.2.1"));
        assert!(!khop(">1.2", "1.2.0"));
    }

    #[test]
    fn nho_va_nho_bang() {
        assert!(khop("<1.2.3", "1.2.2"));
        assert!(!khop("<1.2.3", "1.2.3"));
        assert!(khop("<=1.2.3", "1.2.3"));
        assert!(!khop("<=1.2.3", "1.2.4"));
        assert!(khop("<2", "1.99.99"));
        assert!(!khop("<2", "2.0.0"));
    }

    #[test]
    fn khong_toan_tu_hieu_la_caret() {
        assert!(khop("1.2.3", "1.9.0"));
        assert!(!khop("1.2.3", "2.0.0"));
        assert!(!khop("1.2.3", "1.2.2"));
        // `1.0` (thiếu PATCH) cũng là `^1.0` ⇒ <2.0.0.
        assert!(khop("1.0", "1.5.0"));
        assert!(!khop("1.0", "2.0.0"));
        // Không toán tử ⇒ cùng ngữ nghĩa `^`.
        assert_eq!(req("1.2.3"), req("^1.2.3"));
    }

    // ----- 40..46: AND, chuẩn hoá, thứ tự -----

    #[test]
    fn dau_phay_la_and() {
        assert!(khop(">=1.2.0, <1.5.0", "1.4.9"));
        assert!(!khop(">=1.2.0, <1.5.0", "1.5.0"));
        assert!(!khop(">=1.2.0, <1.5.0", "1.1.9"));
        // Ba khoang.
        assert!(khop(">=1.0.0, <2.0.0, >=1.4.0", "1.4.0"));
        assert!(!khop(">=1.0.0, <2.0.0, >=1.4.0", "1.3.9"));
        // Trộn `^` với chặn trên.
        assert!(khop("^1, <1.4", "1.3.9"));
        assert!(!khop("^1, <1.4", "1.4.0"));
        // Khoảng rỗng ⇒ không phiên bản nào thoả (ca xung đột cổ điển).
        let r = req(">=2.0.0, <1.0.0");
        assert!(!r.khop(&v("1.5.0")));
        assert!(!r.khop(&v("2.0.0")));
    }

    #[test]
    fn chuan_hoa_in_ra() {
        assert_eq!(req("1.2.3").to_string(), "^1.2.3");
        assert_eq!(req(" 1.2.3 ").to_string(), "^1.2.3");
        assert_eq!(req(">=1.0.0,<2.0.0").to_string(), ">=1.0.0, <2.0.0");
        assert_eq!(req("^0").to_string(), "^0");
        assert_eq!(req("^0.0").to_string(), "^0.0");
        assert_eq!(req("~1.2").to_string(), "~1.2");
        assert_eq!(req("=1.2.3").to_string(), "=1.2.3");
        assert_eq!(req("<=2").to_string(), "<=2");
        assert_eq!(req("^1.2.3").chuan_hoa(), "^1.2.3");
        // Giữ **thứ tự** đã ghi (tất định).
        assert_eq!(req("<2.0.0, >=1.0.0").to_string(), "<2.0.0, >=1.0.0");
        assert_eq!(req("<2.0.0, >=1.0.0").ds().len(), 2);
    }

    #[test]
    fn binh_dang_ngu_nghia() {
        assert_eq!(req("^1.2"), req("^1.2"));
        assert_ne!(req("^1.2"), req("^1.2.0"), "`n` khác ⇒ chặn trên khác");
        assert_eq!(req("1.2.3"), req("^1.2.3"));
        assert_eq!(req(">=1.0.0,<2.0.0"), req(">=1.0.0, <2.0.0"));
        assert_ne!(req(">=1.0.0"), req(">1.0.0"));
    }

    #[test]
    fn toan_tu_in_dung_dau() {
        assert_eq!(ToanTu::Caret.dau(), "^");
        assert_eq!(ToanTu::Tilde.dau(), "~");
        assert_eq!(ToanTu::Bang.dau(), "=");
        assert_eq!(ToanTu::Lon.dau(), ">");
        assert_eq!(ToanTu::LonBang.dau(), ">=");
        assert_eq!(ToanTu::Nho.dau(), "<");
        assert_eq!(ToanTu::NhoBang.dau(), "<=");
    }

    #[test]
    fn tu_ds_va_mot_so_ca_bien() {
        // `tu_ds` giữ nguyên danh sách đã có.
        let k = Khoang {
            tt: ToanTu::Bang,
            major: 1,
            minor: 0,
            patch: 0,
            n: 3,
        };
        let r = VersionReq::tu_ds(vec![k]);
        assert!(r.khop(&v("1.0.0")));
        assert!(!r.khop(&v("1.0.1")));
        assert_eq!(r.to_string(), "=1.0.0");
        // Phiên bản rất lớn vẫn so đúng.
        assert!(khop(
            ">=18446744073709551615.0.0",
            "18446744073709551615.0.0"
        ));
        assert!(!khop(
            ">18446744073709551615.0.0",
            "18446744073709551615.0.0"
        ));
    }
}
