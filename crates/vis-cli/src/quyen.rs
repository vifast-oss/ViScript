// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-72b (VD-103/VD-106)** — cưỡng chế quyền `cap[...]` ở **biên host** + `--grant`.
//!
//! `main` khai `cap` (CST) ⇒ [`Quyen::bat`] bật: `HostFs`/`HostNet` chỉ cho thao tác trong **root**
//! đã khai; ngoài root ⇒ `Err` có dấu [`vis_interp::stdlib::DAU_QUYEN`] ⇒ tầng std đổi thành **trap
//! `VS0604`** (không bắt được), exit 2. **Không** khai `cap` ⇒ `bat=false` ⇒ **không hạn chế**
//! (tương thích 858 test cũ; "siết mặc định" là **nợ**).

use std::path::{Path, PathBuf};

use vis_interp::builtin::{FsHost, MucFs, NetHost};
use vis_interp::stdlib::DAU_QUYEN;
use vis_interp::value::Value;
use vis_syntax::{Kind, Mode, NodeKind};

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
            let ty = text
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
                            text.get(x.span.start..x.span.end)
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

/// **ST-97**: tệp thực thi (Unix) — dùng để bắt `--grant` Proc gán nhầm cho `cap[Dir]`.
#[cfg(unix)]
fn la_tep_thuc_thi(p: &Path) -> bool {
    la_tep_lenh(p)
}

#[cfg(not(unix))]
fn la_tep_thuc_thi(_p: &Path) -> bool {
    false
}

/// **ST-123 (VD-130c)**: tệp **chạy được** — Unix: tệp thường + bit `x`; nền khác: tệp thường.
fn la_tep_lenh(p: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::metadata(p)
            .map(|m| m.is_file() && (m.permissions().mode() & 0o111) != 0)
            .unwrap_or(false)
    }
    #[cfg(not(unix))]
    {
        std::fs::metadata(p).map(|m| m.is_file()).unwrap_or(false)
    }
}

/// **ST-123 (VD-130c)**: phân giải **giá trị `--grant` của `cap[Proc]`** thành đường dẫn tuyệt đối
/// **đã canonicalize**, **một lần lúc khởi động** (không phải mỗi lần chạy lệnh).
///
/// - Giá trị **không có `/`** ⇒ dò `PATH` (chỉ nhận [tệp chạy được](la_tep_lenh)), mục khớp đầu
///   tiên — **cùng** luật `execvp` sẽ dùng khi chạy.
/// - Giá trị **có `/`** ⇒ ghép `goc` nếu tương đối, rồi `canonicalize` (theo symlink).
/// - Không phân giải được ⇒ `Err` có chẩn đoán rõ (CLI in ra rồi exit 2), **không** im lặng nhận.
pub fn phan_giai_lenh(v: &str, goc: &Path) -> Result<PathBuf, String> {
    if !v.contains('/') {
        let path = std::env::var_os("PATH").unwrap_or_default();
        for thu_muc in std::env::split_paths(&path) {
            let ung = thu_muc.join(v);
            if let Ok(c) = std::fs::canonicalize(&ung) {
                if la_tep_lenh(&c) {
                    return Ok(c);
                }
            }
        }
        return Err(format!(
            "không thấy lệnh `{v}` trong `PATH` — cần đường dẫn tuyệt đối tới tệp thực thi có thật lúc khởi động"
        ));
    }
    let p = Path::new(v);
    let ung = if p.is_absolute() {
        p.to_path_buf()
    } else {
        goc.join(p)
    };
    let c = std::fs::canonicalize(&ung)
        .map_err(|e| format!("không phân giải được `{}`: {e}", ung.display()))?;
    if !la_tep_lenh(&c) {
        return Err(format!(
            "`{}` không phải tệp thực thi (cần tệp thường có quyền chạy)",
            c.display()
        ));
    }
    Ok(c)
}

/// **ST-123 (VD-130c)**: phân giải **lệnh sẽ chạy** (`ten` do chương trình gọi) theo **cùng luật**
/// với `--grant`, tại thời điểm chạy; `None` = không phân giải được ⇒ không thể khớp allowlist.
pub fn chuan_lenh(ten: &str) -> Option<PathBuf> {
    let goc = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    phan_giai_lenh(ten, &goc).ok()
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

    fn tuyet_doi(&self, p: &str) -> PathBuf {
        let p = Path::new(p);
        if p.is_absolute() {
            p.to_path_buf()
        } else {
            self.goc.join(p)
        }
    }

    fn chuan(p: &Path) -> PathBuf {
        if let Ok(c) = std::fs::canonicalize(p) {
            return c;
        }
        // Tệp **chưa tồn tại** (ghi): canonicalize thư mục cha rồi ghép tên.
        if let (Some(cha), Some(ten)) = (p.parent(), p.file_name()) {
            if let Ok(c) = std::fs::canonicalize(cha) {
                return c.join(ten);
            }
        }
        p.to_path_buf()
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
        let c = Self::chuan(&self.tuyet_doi(p));
        self.kiem_canon(&c, p, can_write)?;
        Ok(Some(c))
    }

    /// Quyết định quyền trên **đường dẫn đã canonicalize** `c` (dùng chung cho kiểm trước và sau
    /// khi mở — **ST-124**).
    fn kiem_canon(&self, c: &Path, p: &str, can_write: bool) -> Result<(), String> {
        let mut trong_root = false;
        for q in &self.moc {
            if q.kind != "Dir" && q.kind != "File" {
                continue;
            }
            let r = Self::chuan(Path::new(&q.root));
            if c.starts_with(&r) {
                trong_root = true;
                if !can_write || q.write {
                    return Ok(());
                }
            }
        }
        let co_fs = self.moc.iter().any(|q| q.kind == "Dir" || q.kind == "File");
        let msg = if self.siet && !co_fs {
            // **ST-81**: strict + **không** khai cap fs ⇒ nêu rõ **cần khai** gì.
            format!("[quyền] cần khai `cap[Dir]` cho `{p}`")
        } else if trong_root {
            format!("[quyền] thiếu `(write)` trên `cap[Dir]` cho `{p}`")
        } else {
            format!("[quyền] thiếu `cap[Dir]` cho `{p}`")
        };
        Err(format!("{DAU_QUYEN}{msg}"))
    }

    /// **ST-124 (VD-130b)**: xác nhận **sau khi đã mở** — đóng khe TOCTOU kiểm-rồi-mở.
    ///
    /// `truoc` = đường dẫn canonicalize đã qua kiểm **trước** khi mở; `danh_tinh` = `(dev, ino)` của
    /// **handle vừa mở** (host báo; `(0, 0)` = không báo).
    ///
    /// - Có danh tính ⇒ đọc lại đường dẫn **bây giờ** (`sau`) và đòi `metadata(sau)` **trùng**
    ///   `(dev, ino)` của handle: nếu trùng thì `sau` **đúng là** tệp đã mở ⇒ sau đó kiểm quyền trên
    ///   `sau` (chứ không phải trên đường dẫn cũ). Lệch ⇒ đường dẫn bị đổi trong lúc mở ⇒ **từ chối**.
    /// - Không có danh tính (mock/nền không inode) ⇒ so `sau != truoc` ⇒ **từ chối** (chặn được ca
    ///   "đổi sang đường dẫn khác rồi để nguyên", không chặn được ca "đổi rồi đổi lại" — ghi ở nợ).
    pub fn xac_nhan_sau_mo(
        &self,
        p: &str,
        truoc: &Path,
        danh_tinh: (u64, u64),
    ) -> Result<(), String> {
        let sau = Self::chuan(&self.tuyet_doi(p));
        #[cfg(unix)]
        if danh_tinh.0 != 0 {
            use std::os::unix::fs::MetadataExt;
            let loi = format!(
                "{DAU_QUYEN}[quyền] `{p}` bị thay đổi trong khi mở (chống kiểm-rồi-mở: không đọc lại được đường dẫn)"
            );
            let md = std::fs::metadata(&sau).map_err(|_| loi.clone())?;
            if (md.dev(), md.ino()) != danh_tinh {
                return Err(loi);
            }
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
        let host = dia_chi.rsplit_once(':').map(|(h, _)| h).unwrap_or(dia_chi);
        let co_net = self.moc.iter().any(|q| q.kind == "Net");
        let khop = self.moc.iter().any(|q| {
            q.kind == "Net" && (q.root == host || (q.root.is_empty() && host == "localhost"))
        });
        let loopback = host == "127.0.0.1" || host == "localhost";
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

/// Bọc `FsHost` để **kiểm quyền trước** mọi thao tác.
pub struct FsGuard<'a> {
    pub trong: &'a mut dyn FsHost,
    pub quyen: &'a Quyen,
}

impl FsHost for FsGuard<'_> {
    /// **ST-124 (VD-130b)**: **mở-rồi-kiểm** thay cho kiểm-rồi-mở.
    ///
    /// 1. Kiểm **trước** (giữ nguyên thông điệp lỗi cũ, và **không mở** tệp đã biết là ngoài quyền).
    /// 2. Mở tệp **một lần** qua host, lấy `(nội dung, dev, ino)` của chính handle.
    /// 3. Xác nhận **sau khi mở**: đường dẫn canonicalize hiện tại phải trỏ **đúng** tệp đã mở, và
    ///    quyền được kiểm lại trên **đường dẫn sau khi mở** ⇒ nội dung trả về luôn là tệp đã kiểm.
    fn read(&mut self, p: &str) -> Result<String, String> {
        let da_kiem = self.quyen.kiem_fs_canon(p, false)?;
        let (s, dev, ino) = self.trong.read_danh_tinh(p)?;
        if let Some(truoc) = da_kiem {
            self.quyen.xac_nhan_sau_mo(p, &truoc, (dev, ino))?;
        }
        Ok(s)
    }
    fn write(&mut self, p: &str, s: &str) -> Result<(), String> {
        self.quyen.kiem_fs(p, true)?;
        self.trong.write(p, s)
    }
    /// **ST-83**: ngoài quyền ⇒ **trap `VS0604`** (không còn `false` im lặng).
    fn exists(&mut self, p: &str) -> bool {
        self.exists_quyen(p).unwrap_or(false)
    }
    fn exists_quyen(&mut self, p: &str) -> Result<bool, String> {
        self.quyen.kiem_fs(p, false)?;
        Ok(self.trong.exists(p))
    }
    fn list(&mut self, p: &str) -> Result<Vec<String>, String> {
        self.quyen.kiem_fs(p, false)?;
        self.trong.list(p)
    }
    fn list_entries(&mut self, p: &str) -> Result<Vec<MucFs>, String> {
        self.quyen.kiem_fs(p, false)?;
        self.trong.list_entries(p)
    }
}

/// Bọc `NetHost` để kiểm `cap[Net]`.
pub struct NetGuard<'a> {
    pub trong: &'a mut dyn NetHost,
    pub quyen: &'a Quyen,
}

impl NetHost for NetGuard<'_> {
    fn connect(&mut self, dia_chi: &str, timeout_ms: u64) -> Result<String, String> {
        self.quyen.kiem_net(dia_chi)?;
        self.trong.connect(dia_chi, timeout_ms)
    }
    fn listen(&mut self, dia_chi: &str) -> Result<String, String> {
        self.quyen.kiem_net(dia_chi)?;
        self.trong.listen(dia_chi)
    }
    fn local_addr(&mut self, tay: &str) -> Result<String, String> {
        self.trong.local_addr(tay)
    }
    fn accept(&mut self, tay: &str, timeout_ms: u64) -> Result<String, String> {
        self.trong.accept(tay, timeout_ms)
    }
    fn read(&mut self, tay: &str, toi_da: usize, timeout_ms: u64) -> Result<Vec<u8>, String> {
        self.trong.read(tay, toi_da, timeout_ms)
    }
    fn write(&mut self, tay: &str, du_lieu: &[u8]) -> Result<(), String> {
        self.trong.write(tay, du_lieu)
    }
    fn close(&mut self, tay: &str) -> Result<(), String> {
        self.trong.close(tay)
    }
    fn ghi_nhat_ky(&mut self, dong: &str) {
        self.trong.ghi_nhat_ky(dong);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vis_interp::builtin::FsHost;

    fn tam(tag: &str) -> PathBuf {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static DEM: AtomicUsize = AtomicUsize::new(0);
        let d = std::env::temp_dir().join(format!(
            "vis-toctou-{tag}-{}-{}",
            std::process::id(),
            DEM.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).expect("tmp");
        d
    }

    /// Quyền `cap[Dir]` = `root`, **strict-caps** bật, không `cap[Proc]`.
    fn quyen_dir(root: &Path) -> Quyen {
        Quyen {
            moc: vec![QuyenCap {
                pos: 0,
                kind: "Dir".to_string(),
                root: root.display().to_string(),
                write: false,
                allow: Vec::new(),
            }],
            bat: true,
            siet: true,
            goc: root.to_path_buf(),
            allow_proc: None,
        }
    }

    /// Thay `root/tep` bằng **symlink trỏ ra ngoài** (`ngoai`) hoặc bằng **tệp trong root**
    /// (`TRONG`) — bằng `rename` nên **nguyên tử** (không có trạng thái nửa vời).
    #[cfg(unix)]
    fn dat_tep(root: &Path, tep: &Path, ngoai: &Path, toi_ngoai: bool) {
        let tam = root.join(".tam");
        let _ = std::fs::remove_file(&tam);
        if toi_ngoai {
            std::os::unix::fs::symlink(ngoai, &tam).expect("symlink");
        } else {
            std::fs::write(&tam, "TRONG").expect("ghi .tam");
        }
        std::fs::rename(&tam, tep).expect("rename");
    }

    /// Host giả **đổi symlink ngay bên trong `read`** — mô phỏng **tất định** khe thời gian giữa
    /// bước KIỂM và bước MỞ (không phụ thuộc thời gian thật, không `sleep`).
    #[cfg(unix)]
    struct HostDoi {
        root: PathBuf,
        tep: PathBuf,
        ngoai: PathBuf,
    }

    #[cfg(unix)]
    impl FsHost for HostDoi {
        fn read(&mut self, p: &str) -> Result<String, String> {
            dat_tep(&self.root, &self.tep, &self.ngoai, true);
            std::fs::read_to_string(p).map_err(|e| format!("không đọc được `{p}`: {e}"))
        }
        fn write(&mut self, _p: &str, _s: &str) -> Result<(), String> {
            Ok(())
        }
        fn exists(&mut self, _p: &str) -> bool {
            false
        }
        fn list(&mut self, _p: &str) -> Result<Vec<String>, String> {
            Ok(Vec::new())
        }
    }

    /// **Ca tất định**: đường dẫn đã KIỂM (trong root) bị đổi thành symlink ra ngoài **đúng lúc**
    /// mở ⇒ must be blocked; previously read outside content.
    #[cfg(unix)]
    #[test]
    fn doi_symlink_dung_luc_mo_bi_chan() {
        let d = tam("bien");
        let root = d.join("root");
        std::fs::create_dir_all(&root).expect("root");
        let ngoai = d.join("ngoai.txt");
        std::fs::write(&ngoai, "NGOAI").expect("ngoai");
        let tep = root.join("tep");
        std::fs::write(&tep, "TRONG").expect("tep");
        let q = quyen_dir(&root);
        let mut host = HostDoi {
            root: root.clone(),
            tep: tep.clone(),
            ngoai,
        };
        let mut g = FsGuard {
            trong: &mut host,
            quyen: &q,
        };
        let kq = g.read(tep.to_str().expect("utf8"));
        assert!(
            kq.is_err(),
            "phải CHẶN đọc tệp ngoài root, nhưng đọc được: {kq:?}"
        );
        assert!(
            !kq.unwrap_or_default().contains("NGOAI"),
            "nội dung ngoài root đã lọt ra"
        );
        let _ = std::fs::remove_dir_all(&d);
    }

    /// **Ca đua thật**: luồng phụ đổi symlink liên tục, luồng chính đọc `LAP` lần.
    /// Không phụ thuộc **thời gian tuyệt đối** (chỉ đếm vòng), **lặp có giới hạn**.
    /// Bất biến: **không lần nào** trả về nội dung của tệp ngoài root; và **phải có** lần đọc
    /// thành công (`TRONG`) để ca không "xanh vì chặn tất".
    #[cfg(unix)]
    #[test]
    fn dua_doi_symlink_lap_co_gioi_han() {
        const LAP: usize = 50_000;
        let d = tam("dua");
        let root = d.join("root");
        std::fs::create_dir_all(&root).expect("root");
        let ngoai = d.join("ngoai.txt");
        std::fs::write(&ngoai, "NGOAI").expect("ngoai");
        let tep = root.join("tep");
        std::fs::write(&tep, "TRONG").expect("tep");
        let q = quyen_dir(&root);
        let mut runner = crate::ngoai::Runner::default();
        let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let st = stop.clone();
        let (r2, t2, n2) = (root.clone(), tep.clone(), ngoai.clone());
        let phu = std::thread::spawn(move || {
            let mut vong = 0usize;
            while !st.load(std::sync::atomic::Ordering::Relaxed) {
                dat_tep(&r2, &t2, &n2, vong.is_multiple_of(2));
                vong += 1;
            }
            vong
        });
        let (mut ok_trong, mut ro_ri, mut chan) = (0usize, 0usize, 0usize);
        let p = tep.to_str().expect("utf8");
        {
            let mut g = FsGuard {
                trong: &mut runner,
                quyen: &q,
            };
            for _ in 0..LAP {
                match g.read(p) {
                    Ok(s) if s == "TRONG" => ok_trong += 1,
                    Ok(_) => ro_ri += 1,
                    Err(_) => chan += 1,
                }
            }
        }
        stop.store(true, std::sync::atomic::Ordering::Relaxed);
        let vong = phu.join().expect("luồng phụ");
        eprintln!(
            "TOCTOU dua: lap={LAP} ok_trong={ok_trong} ro_ri={ro_ri} chan={chan} vong_doi={vong}"
        );
        assert_eq!(ro_ri, 0, "RÒ RỈ: đọc được nội dung ngoài root {ro_ri} lần");
        assert!(
            ok_trong > 0,
            "ca vô nghĩa: không lần nào đọc được tệp trong root"
        );
        let _ = std::fs::remove_dir_all(&d);
    }
}
