// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-72b (VD-103/VD-106)** — cưỡng chế quyền `cap[...]` ở **biên host** + `--grant`.
//!
//! `main` khai `cap` (CST) ⇒ [`Quyen::bat`] bật: `HostFs`/`HostNet` chỉ cho thao tác trong **root**
//! đã khai; ngoài root ⇒ `Err` có dấu [`vis_core::host::DAU_QUYEN`] ⇒ tầng std đổi thành **trap
//! `VS0604`** (không bắt được), exit 2. **Không** khai `cap` ⇒ `bat=false` ⇒ **không hạn chế**
//! (tương thích 858 test cũ; "siết mặc định" là **nợ**).

use std::net::IpAddr;
use std::path::{Path, PathBuf};

use vis_core::host::DAU_QUYEN;
use vis_core::host::{DocQuaHandle, DuongDanHandle};
use vis_core::value::Value;
use vis_syntax::{Kind, Mode, NodeKind};

use crate::quyen_chuan::{chuan, chuan_de_quy};

/// **ST-286 (R0c)** — guard `FsHost` + helper đường dẫn `--grant` tách khỏi tệp này để giữ R1
/// (≤ 1500 dòng/tệp); tái xuất giữ nguyên mọi đường dùng cũ (`quyen::FsGuard`,
/// `quyen::chuan_lenh`, …). **0 đổi hành vi**.
#[path = "guard.rs"]
mod guard_impl;
#[path = "path.rs"]
mod path_impl;

pub use guard_impl::FsGuard;
use path_impl::la_tep_thuc_thi;
pub use path_impl::{chuan_lenh, phan_giai_lenh};

/// **ST-310c (VD-285)** — primitive mở/đọc/ghi **một nguồn** dưới fd gốc `cap`, tái xuất cho
/// `vis-plugin-host` dùng (plugin tĩnh). Không crate nào khác được chép `openat2`.
#[cfg(unix)]
pub use crate::quyen_mo::{la_loi_symlink, mo_goc, mo_tep_doc, mo_tep_ghi};

/// Một `cap` khai trên `main` (đọc từ **CST**).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapKhai {
    pub name: String,
    pub kind: String,
    /// Đường dẫn/host **đã bỏ cú pháp literal** (`p"…"`/`"…"`); rỗng nếu không có `@`.
    pub path: String,
    pub write: bool,
    /// Chỉ số tham số trong `main` (0-based).
    pub pos: u32,
}

/// Một quyền **đã giải** (root canonicalize) — nguồn cho `Value::Cap` + kiểm tra.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuyenCap {
    pub pos: u32,
    pub kind: String,
    pub root: String,
    pub write: bool,
    /// **ST-97 (VD-112)**: allowlist lệnh cho `cap[Proc]` (rỗng = không giới hạn).
    pub allow: Vec<String>,
}

/// Tập quyền của lần chạy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Quyen {
    pub moc: Vec<QuyenCap>,
    pub bat: bool,
    /// **ST-81 (VD-108)**: `--strict-caps` — mọi truy cập fs/net phải nằm trong `cap` đã khai;
    /// **không** khai `cap` nào mà chạm fs/net ⇒ `VS0604`. Mặc định `false` (tương thích).
    pub siet: bool,
    pub goc: PathBuf,
    /// **ST-97**: `Some(ds)` = có ≥1 `--grant` cho cap `Proc` ⇒ chỉ chạy lệnh khớp `ds`.
    pub allow_proc: Option<Vec<String>>,
}

/// Bỏ `p"…"`/`"…"` để lấy đường dẫn/host **thuần**.
pub fn bo_lit(s: &str) -> String {
    let s = s.trim();
    let inner = if let Some(r) = s.strip_prefix("p\"") {
        r
    } else if let Some(r) = s.strip_prefix('"') {
        r
    } else {
        return s.to_string();
    };
    inner.strip_suffix('"').unwrap_or(inner).to_string()
}

/// **ST-93 (VD-111)**: tách **chữ ký `fn main`** trong `.vish` (khai báo ở cột 1) — CST chế độ
/// Command không có `FnDecl`, nên `--dry`/quyền trước đây **không thấy** `main(p: cap[Proc])`.
fn chu_ky_main_vish(text: &str) -> Option<String> {
    let mut bat = false;
    let mut sig = String::new();
    let mut sau = 0i32;
    for line in text.lines() {
        if !bat {
            if line.starts_with("fn main") {
                bat = true;
                sig.push_str(line);
                sig.push('\n');
            } else {
                continue;
            }
        } else {
            sig.push_str(line);
            sig.push('\n');
        }
        for c in line.chars() {
            match c {
                '(' | '[' => sau += 1,
                ')' | ']' => sau -= 1,
                _ => {}
            }
        }
        if sau == 0 && sig.contains(':') {
            return Some(sig.trim_end().to_string());
        }
    }
    None
}

/// Đọc danh sách `cap` của `main` từ **CST** (dùng cho cả `--dry` và cưỡng chế).
/// **ST-93**: `.vish` khai báo `fn main` ở cột 1 nằm **ngoài** CST Command ⇒ dựng lại một chương
/// trình Script tí hon từ chữ ký để dùng **cùng** bộ đọc `cap`.
pub fn ds_cap(text: &str, mode: Mode) -> Vec<CapKhai> {
    if mode == Mode::Command {
        if let Some(sig) = chu_ky_main_vish(text) {
            let synth = format!("{sig}\n    let __cap = 1\n");
            return ds_cap(&synth, Mode::Script);
        }
        return Vec::new();
    }
    let parsed = vis_syntax::parse(text, mode);
    let cst = &parsed.cst;
    let mut out = Vec::new();
    for n in cst.root.children() {
        if n.kind != NodeKind::FnDecl {
            continue;
        }
        let ten = n
            .tokens(&cst.tokens)
            .find(|t| t.kind == Kind::Ident)
            .map(|t| t.text.clone());
        if ten.as_deref() != Some("main") {
            continue;
        }
        let Some(ps) = n.child(NodeKind::Params) else {
            continue;
        };
        let params: Vec<&vis_syntax::Node> = ps
            .children()
            .filter(|x| x.kind == NodeKind::Param)
            .collect();
        for (pos, p) in params.iter().enumerate() {
            let Some(tyn) = p.child(NodeKind::Type) else {
                continue;
            };
            // **VD-241 § fix(Windows/CRLF #psvn-A1)**: span trỏ vào văn bản đã chuẩn hoá CRLF→LF.
            let ty = vis_syntax::lex::normalize_source(text)
                .get(tyn.span.start..tyn.span.end)
                .unwrap_or("")
                .trim()
                .to_string();
            let Some(kind) = ty
                .strip_prefix("cap[")
                .and_then(|r| r.strip_suffix(']'))
                .map(str::trim)
                .filter(|k| !k.is_empty())
            else {
                continue;
            };
            let toks: Vec<&vis_syntax::Token> = p.tokens(&cst.tokens).collect();
            let mut name = String::new();
            for t in &toks {
                if t.kind == Kind::Ident {
                    name = t.text.clone();
                }
                if t.is_punct(":") {
                    break;
                }
            }
            let path = toks
                .iter()
                .find(|t| matches!(t.kind, Kind::Path | Kind::Str))
                .map(|t| bo_lit(&t.text))
                .unwrap_or_default();
            let write = p
                .child(NodeKind::GrantList)
                .map(|g| {
                    g.children()
                        .filter(|x| x.kind == NodeKind::DottedPath)
                        .any(|x| {
                            // **VD-241 § fix(Windows/CRLF #psvn-A1)**: span trỏ vào văn bản chuẩn hoá.
                            vis_syntax::lex::normalize_source(text)
                                .get(x.span.start..x.span.end)
                                .map(|s| s.trim() == "write")
                                .unwrap_or(false)
                        })
                })
                .unwrap_or(false);
            out.push(CapKhai {
                name,
                kind: kind.to_string(),
                path,
                write,
                pos: pos as u32,
            });
        }
        break; // chỉ `main` cấp tệp đầu tiên
    }
    out
}

/// Tính `Quyen` từ `cap` khai + `--grant`; lỗi (grant lạ) ⇒ `Err` cho CLI exit 2.
pub fn tao(
    ds: &[CapKhai],
    grants: &[(String, String)],
    goc: &Path,
    siet: bool,
) -> Result<Quyen, String> {
    for (ten, _) in grants {
        if !ds.iter().any(|c| &c.name == ten) {
            return Err(format!(
                "`--grant {ten}=…`: `main` không khai `cap` tên `{ten}` (không thêm quyền mới)"
            ));
        }
    }
    let mut moc = Vec::new();
    let mut allow_proc: Vec<String> = Vec::new();
    for c in ds {
        // **ST-97 (VD-112)**: `cap[Proc]` nhận **nhiều** `--grant` cùng tên = allowlist lệnh.
        if c.kind == "Proc" {
            let ds_grant: Vec<String> = grants
                .iter()
                .filter(|(n, _)| n == &c.name)
                .map(|(_, v)| v.clone())
                .collect();
            // **ST-123 (VD-130c)**: phân giải **một lần lúc khởi động** thành đường dẫn tuyệt đối
            // đã canonicalize; allowlist so **bằng** đường dẫn đó, không so basename.
            let mut allow: Vec<String> = Vec::new();
            for v in &ds_grant {
                match phan_giai_lenh(v, goc) {
                    Ok(p) => allow.push(p.to_string_lossy().to_string()),
                    Err(e) => return Err(format!("`--grant {}={v}`: {e}", c.name)),
                }
            }
            let root = if ds_grant.is_empty() {
                c.path.clone()
            } else {
                ds_grant.join(":")
            };
            allow_proc.extend(allow.iter().cloned());
            moc.push(QuyenCap {
                pos: c.pos,
                kind: c.kind.clone(),
                root,
                write: false,
                allow,
            });
            continue;
        }
        let duong = grants
            .iter()
            .find(|(n, _)| n == &c.name)
            .map(|(_, v)| v.clone())
            .unwrap_or_else(|| c.path.clone());
        let root = match c.kind.as_str() {
            "Dir" | "File" => {
                let p = if Path::new(&duong).is_absolute() {
                    PathBuf::from(&duong)
                } else {
                    goc.join(&duong)
                };
                let canon = std::fs::canonicalize(&p).unwrap_or(p.clone());
                // **ST-97**: `--grant` cho `cap[Dir]` mà trỏ **tệp thực thi** ⇒ nhiều khả năng nhầm với
                // `cap[Proc]`; từ chối rõ (exit 2), không âm thầm nhận làm thư mục.
                if c.kind == "Dir" && la_tep_thuc_thi(&canon) {
                    return Err(format!(
                        "`--grant {}={duong}`: **kiểu không khớp** — `cap[Dir]` nhận **thư mục**, không nhận tệp thực thi; `cap[Proc]` mới nhận lệnh",
                        c.name
                    ));
                }
                canon.to_string_lossy().to_string()
            }
            _ => duong,
        };
        moc.push(QuyenCap {
            pos: c.pos,
            kind: c.kind.clone(),
            root,
            write: c.write && matches!(c.kind.as_str(), "Dir" | "File"),
            allow: Vec::new(),
        });
    }
    Ok(Quyen {
        moc,
        bat: siet || !ds.is_empty(),
        siet,
        goc: goc.to_path_buf(),
        allow_proc: if allow_proc.is_empty() {
            None
        } else {
            Some(allow_proc)
        },
    })
}

impl Quyen {
    /// **ST-97**: allowlist `cap[Proc]` (nếu người dùng đã `--grant`).
    pub fn allow_proc(&self) -> Option<Vec<String>> {
        self.allow_proc.clone()
    }

    /// Giá trị `Value::Cap` cho từng tham số (đưa vào interp/VM).
    pub fn gia_tri(&self) -> Vec<(u32, Value)> {
        self.moc
            .iter()
            .map(|c| {
                (
                    c.pos,
                    Value::Cap {
                        kind: c.kind.clone(),
                        root: c.root.clone(),
                        write: c.write,
                    },
                )
            })
            .collect()
    }

    /// **ST-295 (VD-276)**: ghép đường dẫn người dùng với `goc`; `quyen_mo` dùng lại để chuẩn hoá
    /// từ vựng trước khi mở bằng `openat` (không phụ thuộc `canonicalize`).
    pub(crate) fn tuyet_doi(&self, p: &str) -> PathBuf {
        let p = Path::new(p);
        if p.is_absolute() {
            p.to_path_buf()
        } else {
            self.goc.join(p)
        }
    }

    /// Kiểm thao tác hệ tệp; `can_write=false` ⇒ chỉ cần root, `true` ⇒ root phải có `(write)`.
    pub fn kiem_fs(&self, p: &str, can_write: bool) -> Result<(), String> {
        self.kiem_fs_canon(p, can_write).map(|_| ())
    }

    /// **ST-124 (VD-130b)**: như [`kiem_fs`](Self::kiem_fs) nhưng trả **đường dẫn canonicalize đã
    /// kiểm** (`None` = chưa bật quyền) để bước sau xác nhận "tệp đã mở" vẫn đúng đường dẫn đó.
    pub fn kiem_fs_canon(&self, p: &str, can_write: bool) -> Result<Option<PathBuf>, String> {
        if !self.bat {
            return Ok(None);
        }
        let c = chuan(&self.tuyet_doi(p));
        self.kiem_canon(&c, p, can_write)?;
        Ok(Some(c))
    }

    /// **VD-270 (mkdir-all)**: như [`kiem_fs`](Self::kiem_fs) nhưng dùng
    /// [`chuan_de_quy`](crate::quyen_chuan::chuan_de_quy) — canonicalize **tổ tiên gần nhất đã tồn tại** rồi ghép
    /// đuôi **từ vựng**; `root/../ngoai/...` không lọt chỉ vì đuôi chưa tồn tại. `(write)` như `mkdir`.
    fn kiem_fs_de_quy(&self, p: &str, can_write: bool) -> Result<(), String> {
        if !self.bat {
            return Ok(());
        }
        let c = chuan_de_quy(&self.tuyet_doi(p));
        self.kiem_canon(&c, p, can_write)
    }

    /// Quyết định quyền trên **đường dẫn đã canonicalize** `c` (dùng chung cho kiểm trước và sau
    /// khi mở — **ST-124**).
    ///
    /// **ST-165 (nợ #138) — thông điệp phải cho người vận hành biết VÌ SAO**, vì khi đọc `/proc`
    /// có hai ca từ chối đúng nhưng thông điệp cũ gây hiểu nhầm là "khai thiếu `cap`":
    /// - `c` **không phải đường dẫn tuyệt đối** (vd `/proc/self/fd/1` trỏ tới `pipe:[…]`): vật đã mở
    ///   không có đường dẫn ⇒ không kiểm được theo vật ⇒ nói rõ, **không** báo "thiếu `cap[Dir]`".
    /// - `p` tuyệt đối nhưng phân giải ra `c` khác (vd `/proc/self/fd/0` ⇒ `/dev/null`): nêu **đích
    ///   thật**; nếu không, người khai `cap[Dir]` cho `/proc` tưởng cap của mình sai.
    fn kiem_canon(&self, c: &Path, p: &str, can_write: bool) -> Result<(), String> {
        if !c.is_absolute() {
            return Err(format!(
                "{DAU_QUYEN}[quyền] `{p}` trỏ tới đối tượng không phải đường dẫn (`{}`) ⇒ \
                 không xác định được vật để kiểm quyền ⇒ từ chối (ST-165 · nợ #138)",
                c.display()
            ));
        }
        let mut trong_root = false;
        for q in &self.moc {
            if q.kind != "Dir" && q.kind != "File" {
                continue;
            }
            // **ST-314a (W3 H2)**: `q.root` đã canonicalize **một lần lúc khởi động**; KHÔNG
            // canonicalize lại mỗi lần kiểm — nếu gốc cap bị đổi thành symlink, cả `c` lẫn `r`
            // sẽ "di chuyển" ra ngoài và `starts_with` vẫn đúng (thoát vùng cấp).
            let r = Path::new(&q.root);
            if c.starts_with(r) {
                trong_root = true;
                if !can_write || q.write {
                    return Ok(());
                }
            }
        }
        let co_fs = self.moc.iter().any(|q| q.kind == "Dir" || q.kind == "File");
        if self.siet && !co_fs {
            // **ST-81**: strict + **không** khai cap fs ⇒ nêu rõ **cần khai** gì.
            return Err(format!("{DAU_QUYEN}[quyền] cần khai `cap[Dir]` cho `{p}`"));
        }
        if trong_root {
            return Err(format!(
                "{DAU_QUYEN}[quyền] thiếu `(write)` trên `cap[Dir]` cho `{p}`"
            ));
        }
        // **ST-165**: nêu đích thật khi tên đã cho phân giải ra đường dẫn khác (chỉ khi `p` tuyệt
        // đối — đường dẫn tương đối đã được ghép với `goc`, hiển thị `c` sẽ khó hiểu).
        let them = if Path::new(p).is_absolute() && Path::new(p) != c {
            format!(
                " (`{p}` phân giải tới `{}` — ngoài mọi `cap[Dir]`/`cap[File]` đã khai)",
                c.display()
            )
        } else {
            String::new()
        };
        Err(format!(
            "{DAU_QUYEN}[quyền] thiếu `cap[Dir]` cho `{p}`{them}"
        ))
    }

    /// **ST-124 (VD-130b)**: xác nhận **sau khi đã mở** trên **tên** — nay chỉ dùng làm **dự phòng**
    /// cho host/nền **không** báo được đường dẫn thật của handle (`KhongCo`: `--fs-mock`, macOS —
    /// ST-137 §4). Trên Linux, `xac_nhan_handle` quyết định trên đường dẫn **của handle** và **không**
    /// đi qua hàm này.
    ///
    /// `truoc` = đường dẫn canonicalize đã qua kiểm **trước** khi mở; `danh_tinh` = `(dev, ino)` của
    /// **handle vừa mở** (host báo; `(0, 0)` = không báo).
    ///
    /// - Có danh tính ⇒ đọc lại đường dẫn **bây giờ** (`sau`) và đòi `sau` **không phải symlink** +
    ///   `(dev, ino)` của **chính mục tên đó** (`symlink_metadata`) **trùng** `(dev, ino)` của handle:
    ///   nếu trùng thì `sau` **đúng là** tệp đã mở ⇒ sau đó kiểm quyền trên `sau` (chứ không phải trên
    ///   đường dẫn cũ). Lệch ⇒ đường dẫn bị đổi trong lúc mở ⇒ **từ chối**.
    /// - Không có danh tính (mock/nền không inode) ⇒ so `sau != truoc` ⇒ **từ chối** (chặn được ca
    ///   "đổi sang đường dẫn khác rồi để nguyên", không chặn được ca "đổi rồi đổi lại" — ghi ở nợ).
    ///
    /// **ST-174 (VD-176 §1) — nợ `#146` đã vá**: bước kiểm dùng **`symlink_metadata`** (không đi theo
    /// symlink) thay cho `metadata` (ST-137/ST-163). Trước bản vá, `chuan` (syscall 1, trả **chuỗi**)
    /// và `metadata(sau)` (syscall 2, **đi theo symlink**) có thể thấy hai trạng thái khác nhau: kẻ tấn
    /// công đổi `root/tep` thành symlink trỏ ra ngoài **đúng khe giữa hai syscall** thì `sau` vẫn là
    /// **tên trong root** nhưng `metadata(sau)` trả **inode tệp ngoài** (trùng handle đã mở) ⇒
    /// `kiem_canon` xanh ⇒ **nội dung ngoài lọt** (đo 4/8,1 triệu vòng trên macOS ở ST-163). Nay mục
    /// tên là symlink ⇒ `(dev, ino)` của **symlink** ≠ `(dev, ino)` của handle ⇒ **từ chối**, tất định.
    /// Bằng chứng tất định (không dựa số vòng): `quyen_st174.rs` — ca A/B + ca âm điểm chèn.
    /// **Giới hạn còn lại** (chính sách, không phải lỗi đua): **hardlink** tệp ngoài vào trong root được
    /// coi là **nằm trong root** — xem `agent/HOST-BAO-DAM.md`.
    pub fn xac_nhan_sau_mo(
        &self,
        p: &str,
        truoc: &Path,
        danh_tinh: (u64, u64),
    ) -> Result<(), String> {
        let sau = chuan(&self.tuyet_doi(p));
        // **ST-174 (VD-176 §2)** — điểm chèn **CHỈ TEST** giữa `canonicalize` (dòng trên) và bước kiểm
        // (dòng dưới): ca tất định hoán đổi `root/tep` **đúng khe giữa hai syscall**. `cfg(test)` ⇒
        // **không** có trong build thường (`cargo build`); chưa đăng ký hook ⇒ no-op.
        #[cfg(all(test, unix))]
        chen::chay(&sau, p);
        #[cfg(unix)]
        if danh_tinh.0 != 0 {
            use std::os::unix::fs::MetadataExt;
            let loi = format!(
                "{DAU_QUYEN}[quyền] `{p}` bị thay đổi trong khi mở (chống kiểm-rồi-mở: không đọc lại được đường dẫn)"
            );
            // **ST-174 (VD-176 §1)**: `symlink_metadata` — **không** đi theo symlink (nợ `#146`).
            let md = std::fs::symlink_metadata(&sau).map_err(|_| loi.clone())?;
            if (md.dev(), md.ino()) != danh_tinh {
                return Err(loi);
            }
            #[cfg(all(test, unix))]
            vet::them(format!(
                "xac_nhan_sau_mo(truoc={}, sau={}, danh_tinh_handle=({},{}), metadata(sau)=({},{}))",
                truoc.display(),
                sau.display(),
                danh_tinh.0,
                danh_tinh.1,
                md.dev(),
                md.ino()
            ));
            return self.kiem_canon(&sau, p, false);
        }
        let _ = danh_tinh;
        if sau != truoc {
            return Err(format!(
                "{DAU_QUYEN}[quyền] `{p}` bị thay đổi trong khi mở (chống kiểm-rồi-mở)"
            ));
        }
        Ok(())
    }

    /// **ST-137 (VD-141)**: quyết định quyền trên **handle đã mở** — nguồn: **đường dẫn thật của
    /// chính handle** do host báo.
    ///
    /// - `Co(dd)` (Linux `/proc/self/fd`): quyết định trên `dd` — **vật** đã mở, không trên tên.
    ///   Đường dẫn của handle là đường dẫn **sau cùng** của vật đó (kernel giải symlink), nên tên
    ///   bị đổi trong lúc mở **không** làm sai quyết định.
    /// - `KhongLayDuoc` (host thật, `/proc` lỗi/không mount): **từ chối** (`VS0604`) — không quay
    ///   về kiểm theo tên (nếu quay về thì cửa sổ TOCTOU mở lại).
    /// - `KhongCo` (mock, hoặc nền không có primitive fd→path như macOS): giữ bước xác nhận của
    ///   ST-124 ([`xac_nhan_sau_mo`](Self::xac_nhan_sau_mo)).
    pub fn xac_nhan_handle(&self, p: &str, truoc: &Path, d: &DocQuaHandle) -> Result<(), String> {
        match &d.duong_dan {
            DuongDanHandle::Co(dd) => self.kiem_canon(dd, p, false).map_err(|e| {
                format!(
                    "{e} (tệp ĐÃ MỞ nằm ở `{}` — chống kiểm-rồi-mở theo handle, ST-137)",
                    dd.display()
                )
            }),
            DuongDanHandle::KhongLayDuoc(ly_do) => Err(format!(
                "{DAU_QUYEN}[quyền] không xác định được đường dẫn thật của tệp đã mở cho `{p}` \
                 ({ly_do}) ⇒ từ chối (không quay về kiểm theo tên; ST-137). Trên Linux cần \
                 procfs đọc được tại `/proc/self/fd`: thiếu procfs thì **mọi** lần đọc đều bị \
                 từ chối — hãy kiểm/mount procfs (ST-165 · nợ #138)"
            )),
            DuongDanHandle::KhongCo => self.xac_nhan_sau_mo(p, truoc, d.danh_tinh),
        }
    }

    /// **ST-93 (VD-111)**: `--strict-caps` + **không** khai `cap[Proc]` ⇒ **không** cho chạy tiến
    /// trình ngoài. Mặc định (`siet=false`) vẫn cho (tương thích).
    pub fn cho_phep_proc(&self) -> bool {
        !self.siet || self.moc.iter().any(|c| c.kind == "Proc")
    }

    /// Kiểm thao tác mạng: host khớp `cap[Net]` đã khai **hoặc** loopback khi có `cap[Net]`.
    pub fn kiem_net(&self, dia_chi: &str) -> Result<(), String> {
        if !self.bat {
            return Ok(());
        }
        let host = crate::net_quyen::host_cua(dia_chi);
        let co_net = self.moc.iter().any(|q| q.kind == "Net");
        let khop = self.moc.iter().any(|q| {
            let root = crate::net_quyen::host_cua(&q.root);
            q.kind == "Net" && (root == host || (root.is_empty() && host == "localhost"))
        });
        let loopback = host == "localhost"
            || host
                .parse::<IpAddr>()
                .map(|ip| ip.is_loopback())
                .unwrap_or(false);
        if khop || (loopback && co_net) {
            Ok(())
        } else if self.siet && !co_net {
            // **ST-81**: strict + không khai `cap[Net]` ⇒ nêu rõ cần khai.
            Err(format!(
                "{DAU_QUYEN}[quyền] cần khai `cap[Net]` cho `{dia_chi}`"
            ))
        } else {
            Err(format!(
                "{DAU_QUYEN}[quyền] thiếu `cap[Net]` cho `{dia_chi}` (không khai host này)"
            ))
        }
    }
}

/// **ST-174 (VD-176 §2) — ĐIỂM CHÈN chỉ test**, nằm **giữa** `canonicalize` và bước kiểm
/// (`symlink_metadata`) của [`Quyen::xac_nhan_sau_mo`] — chỗ mà `#146` lọt nội dung ngoài root.
///
/// Vì sao cần: ca đua thật của ST-163 chỉ tái hiện được **4/8,1 triệu vòng** trên macOS (phụ thuộc
/// thời gian), nên **không** chứng minh được bản vá. Điểm chèn cho phép ca test dựng lại **đúng khe
/// giữa hai syscall** ⇒ kết quả **100/100 tất định**, không dựa số vòng.
///
/// Bảo đảm:
/// - **Chỉ `cfg(all(test, unix))`** ⇒ **không** tồn tại trong build thường (`cargo build`) — không có
///   cờ sản phẩm, không đổi hành vi phát hành.
/// - Chưa đăng ký hook ⇒ [`chay`] là **no-op** ⇒ đường mã chạy y như không có điểm chèn.
/// - Hook **thread-local** (mỗi ca test một luồng) ⇒ ca chạy song song không nhiễu nhau.
/// - **Ca âm** (ST-174 §6): [`so_lan`] đếm số lần điểm chèn **thực sự chạy**; nếu điểm chèn bị bỏ thì
///   `so_lan == 0` ⇒ ca tất định **báo “không chạy được điểm chèn”**, không thể xanh im lặng.
#[cfg(all(test, unix))]
pub(crate) mod chen {
    use std::cell::{Cell, RefCell};
    use std::path::Path;

    /// Hook của điểm chèn: `(sau, p)` — hai tham số của `xac_nhan_sau_mo`.
    type Hook = Box<dyn FnMut(&Path, &str)>;

    thread_local! {
        static HOOK: RefCell<Option<Hook>> = const { RefCell::new(None) };
        static DEM: Cell<usize> = const { Cell::new(0) };
    }

    /// Đăng ký hook cho **luồng hiện tại** (đặt lại bộ đếm).
    pub(crate) fn dat(h: Hook) {
        DEM.with(|d| d.set(0));
        HOOK.with(|c| *c.borrow_mut() = Some(h));
    }

    /// Gỡ hook + đặt lại bộ đếm (ca test gọi ở cuối để không ảnh hưởng ca khác cùng luồng).
    pub(crate) fn go() {
        HOOK.with(|c| *c.borrow_mut() = None);
        DEM.with(|d| d.set(0));
    }

    /// Số lần điểm chèn **đã chạy** trên luồng hiện tại; `0` = **không chạy được điểm chèn**.
    pub(crate) fn so_lan() -> usize {
        DEM.with(Cell::get)
    }

    /// **Điểm chèn**: gọi hook (nếu có) với `(sau, p)`. No-op khi chưa đăng ký ⇒ **không** đổi hành vi.
    pub(crate) fn chay(sau: &Path, p: &str) {
        if HOOK.with(|c| c.borrow().is_none()) {
            return;
        }
        DEM.with(|d| d.set(d.get() + 1));
        // `take` rồi trả lại để hook có thể `FnMut` (giữ trạng thái) mà không mượn `RefCell` khi chạy.
        if let Some(mut h) = HOOK.with(|c| c.borrow_mut().take()) {
            h(sau, p);
            HOOK.with(|c| *c.borrow_mut() = Some(h));
        }
    }
}

/// **ST-163 — vết chẩn đoán ca đua, CHỈ tồn tại khi biên dịch test (`cfg(all(test, unix))`)**:
/// ghi lại *đường đi* của **lần đọc vừa xong** (kiểm trước → mở qua handle → xác nhận sau khi mở)
/// để ca đua [`tests::dua_doi_symlink_lap_co_gioi_han`] in ra **vì sao** một lần đọc lọt nội dung
/// ngoài root (nợ `#146`).
///
/// Bật bằng biến môi trường **`VIS_TOCTOU_TRACE=1`**; **mặc định tắt** ⇒ không syscall thêm, không
/// đổi hành vi, **không** đổi kỳ vọng/ngưỡng của ca. Đây là **núm chẩn đoán** của ST-163, **không**
/// phải cờ sản phẩm: khối này không tồn tại trong bản phát hành.
#[cfg(all(test, unix))]
mod vet {
    use std::cell::RefCell;
    use std::sync::OnceLock;

    thread_local! {
        static VET: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    }

    fn bat() -> bool {
        static BAT: OnceLock<bool> = OnceLock::new();
        *BAT.get_or_init(|| std::env::var_os("VIS_TOCTOU_TRACE").is_some())
    }

    /// Đang bật chẩn đoán? (đọc biến môi trường **một lần**).
    pub(super) fn dang_bat() -> bool {
        bat()
    }

    /// Xoá vết của lần đọc trước (chỉ khi bật).
    pub(super) fn bat_dau() {
        if bat() {
            VET.with(|v| v.borrow_mut().clear());
        }
    }

    /// Thêm một dòng vết (chỉ khi bật).
    pub(super) fn them(dong: String) {
        if bat() {
            VET.with(|v| v.borrow_mut().push(dong));
        }
    }

    /// Lấy (và xoá) vết của lần đọc gần nhất; `None` = chưa bật chẩn đoán.
    pub(super) fn lay() -> Option<String> {
        if !bat() {
            return None;
        }
        VET.with(|v| {
            let d = std::mem::take(&mut *v.borrow_mut());
            if d.is_empty() {
                None
            } else {
                Some(d.join(" | "))
            }
        })
    }
}

/// **VS3-4b**: cầu `cap[Net]` + parser host:port nằm ở [`crate::net_quyen`] (R1); tái xuất để
/// `quyen::NetGuard` giữ nguyên đường dùng cũ.
pub use crate::net_quyen::NetGuard;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
