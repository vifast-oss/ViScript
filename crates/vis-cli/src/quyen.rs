// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-72b (VD-103/VD-106)** — cưỡng chế quyền `cap[...]` ở **biên host** + `--grant`.
//!
//! `main` khai `cap` (CST) ⇒ [`Quyen::bat`] bật: `HostFs`/`HostNet` chỉ cho thao tác trong **root**
//! đã khai; ngoài root ⇒ `Err` có dấu [`vis_interp::stdlib::DAU_QUYEN`] ⇒ tầng std đổi thành **trap
//! `VS0604`** (không bắt được), exit 2. **Không** khai `cap` ⇒ `bat=false` ⇒ **không hạn chế**
//! (tương thích 858 test cũ; "siết mặc định" là **nợ**).

use std::path::{Path, PathBuf};

use vis_interp::builtin::{DocQuaHandle, DuongDanHandle, FsHost, MucFs, NetHost};
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
            let r = Self::chuan(Path::new(&q.root));
            if c.starts_with(&r) {
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
        let sau = Self::chuan(&self.tuyet_doi(p));
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

/// **ST-163**: mô tả **trạng thái quan sát được ngay sau** một lần đọc **thành công** — để phân
/// biệt "nội dung tệp ngoài root lọt ra" với "đọc nhầm một tệp trong root". Chỉ dùng cho chẩn đoán
/// (`cfg(test)`), không ảnh hưởng quyết định quyền.
#[cfg(all(test, unix))]
fn mo_ta_lan_doc(p: &str, d: &DocQuaHandle) -> String {
    use std::os::unix::fs::MetadataExt;
    let duong_dan = match &d.duong_dan {
        DuongDanHandle::Co(dd) => format!("Co({})", dd.display()),
        DuongDanHandle::KhongLayDuoc(e) => format!("KhongLayDuoc({e})"),
        DuongDanHandle::KhongCo => "KhongCo".to_string(),
    };
    let symlink = match std::fs::symlink_metadata(p) {
        Ok(m) => format!(
            "symlink={},({},{})",
            m.file_type().is_symlink(),
            m.dev(),
            m.ino()
        ),
        Err(e) => format!("symlink=LỖI({e})"),
    };
    let theo_ten = match std::fs::metadata(p) {
        Ok(m) => format!("({},{})", m.dev(), m.ino()),
        Err(e) => format!("LỖI({e})"),
    };
    format!(
        "doc(dai={}, danh_tinh_handle=({},{}), duong_dan={duong_dan}) sau_doc: symlink_metadata={symlink}, metadata={theo_ten}, chuan(p)={}",
        d.noi_dung.len(),
        d.danh_tinh.0,
        d.danh_tinh.1,
        Quyen::chuan(std::path::Path::new(p)).display()
    )
}

/// Bọc `FsHost` để **kiểm quyền trước** mọi thao tác.
pub struct FsGuard<'a> {
    pub trong: &'a mut dyn FsHost,
    pub quyen: &'a Quyen,
}

impl FsHost for FsGuard<'_> {
    /// **ST-124 (VD-130b) → ST-137 (VD-141)**: **mở-rồi-kiểm**, nhưng quyết định quyền trên
    /// **đường dẫn thật của handle** (khi host báo được — Linux `/proc/self/fd`), không trên tên.
    ///
    /// 1. Kiểm **trước** (giữ nguyên thông điệp lỗi cũ, và **không mở** tệp đã biết là ngoài quyền).
    /// 2. Mở tệp **một lần** qua host, lấy `(nội dung đọc TỪ handle, (dev,ino), đường dẫn thật)`.
    /// 3. Xác nhận **sau khi mở** trên đường dẫn của **chính handle** ⇒ tên bị đổi giữa các bước
    ///    không thể làm nội dung trả về khác vật mà quyết định quyền đã áp. Host **thật** mà
    ///    không lấy được đường dẫn handle ⇒ **từ chối** (`VS0604`).
    fn read(&mut self, p: &str) -> Result<String, String> {
        #[cfg(all(test, unix))]
        vet::bat_dau();
        let da_kiem = self.quyen.kiem_fs_canon(p, false)?;
        let d = self.trong.read_theo_handle(p)?;
        if let Some(truoc) = da_kiem {
            self.quyen.xac_nhan_handle(p, &truoc, &d)?;
        }
        #[cfg(all(test, unix))]
        if vet::dang_bat() {
            vet::them(mo_ta_lan_doc(p, &d));
        }
        Ok(d.noi_dung)
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

    /// **Ca đua thật**: luồng phụ đổi symlink liên tục, luồng chính đọc `VIS_TOCTOU_LAP` lần
    /// (mặc định **50 000**).
    /// Không phụ thuộc **thời gian tuyệt đối** (chỉ đếm vòng), **lặp có giới hạn**.
    /// Bất biến: **không lần nào** trả về nội dung của tệp ngoài root; và **phải có** lần đọc
    /// thành công (`TRONG`) để ca không "xanh vì chặn tất".
    ///
    /// **ST-163 — núm chẩn đoán, KHÔNG đổi kỳ vọng**: số vòng lấy từ biến môi trường
    /// `VIS_TOCTOU_LAP` khi có (mặc định **50 000** như ca gốc) — chỉ để chạy **dài** hơn nhằm bắt
    /// ca đua hiếm (`1/50 000` trên macOS, nợ `#146`). Biến này **không** đổi ngưỡng, **không** đổi
    /// `assert_eq!(ro_ri, 0, …)`, **không** `#[ignore]`/cfg-bỏ. Khi có bất thường, ca in ra **vòng
    /// thứ mấy**, **nội dung thật đọc được** (che 40 ký tự), **độ dài** ký tự/byte và mẫu `err`;
    /// đặt `VIS_TOCTOU_TRACE=1` để in thêm **vết** đường đi của chính lần đọc đó.
    #[cfg(unix)]
    #[test]
    fn dua_doi_symlink_lap_co_gioi_han() {
        let lap: usize = std::env::var("VIS_TOCTOU_LAP")
            .ok()
            .and_then(|s| s.trim().parse::<usize>().ok())
            .filter(|&n| n > 0)
            .unwrap_or(50_000);
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
        let mut ca_ro_ri: Vec<String> = Vec::new();
        let mut mau_loi: Vec<String> = Vec::new();
        let p = tep.to_str().expect("utf8");
        let dong_ho = std::time::Instant::now();
        {
            let mut g = FsGuard {
                trong: &mut runner,
                quyen: &q,
            };
            for vong in 0..lap {
                match g.read(p) {
                    Ok(s) if s == "TRONG" => ok_trong += 1,
                    Ok(s) => {
                        ro_ri += 1;
                        let vet = vet::lay();
                        if ca_ro_ri.len() < 8 {
                            let che: String = s.chars().take(40).collect();
                            ca_ro_ri.push(format!(
                                "RO_RI vong={vong} dai_byte={} dai_ky_tu={} noi_dung={:?} vet={}",
                                s.len(),
                                s.chars().count(),
                                che,
                                vet.as_deref()
                                    .unwrap_or("(tắt — đặt VIS_TOCTOU_TRACE=1 để in vết)")
                            ));
                        }
                    }
                    Err(e) => {
                        chan += 1;
                        let gon: String = e.chars().take(90).collect();
                        if mau_loi.len() < 4 && !mau_loi.contains(&gon) {
                            mau_loi.push(gon);
                        }
                    }
                }
            }
        }
        let giay = dong_ho.elapsed().as_secs_f64();
        stop.store(true, std::sync::atomic::Ordering::Relaxed);
        let vong = phu.join().expect("luồng phụ");
        eprintln!(
            "TOCTOU dua: lap={lap} ok_trong={ok_trong} ro_ri={ro_ri} chan={chan} vong_doi={vong} giay={giay:.1}"
        );
        for dong in &ca_ro_ri {
            eprintln!("{dong}");
        }
        for gon in &mau_loi {
            eprintln!("TOCTOU dua err: {gon}");
        }
        assert_eq!(ro_ri, 0, "RÒ RỈ: đọc được nội dung ngoài root {ro_ri} lần");
        assert!(
            ok_trong > 0,
            "ca vô nghĩa: không lần nào đọc được tệp trong root"
        );
        let _ = std::fs::remove_dir_all(&d);
    }

    /// **ST-137 (VD-141)** — host mô phỏng **kẻ tấn công thắng ở bước MỞ**: đổi `root/tep` thành
    /// **symlink ra ngoài** ngay trước `File::open` ⇒ handle = tệp NGOÀI root. Đường dẫn thật của
    /// handle (`/proc/self/fd`) phải chỉ ra ngoài root ⇒ tầng quyền **từ chối**; không được trả
    /// nội dung `NGOAI`. Đây là ca **tất định** cho đúng trật tự đã lọt trước ST-137.
    #[cfg(target_os = "linux")]
    struct HostThangOMo {
        root: PathBuf,
        tep: PathBuf,
        ngoai: PathBuf,
    }

    #[cfg(target_os = "linux")]
    impl FsHost for HostThangOMo {
        fn read(&mut self, p: &str) -> Result<String, String> {
            std::fs::read_to_string(p).map_err(|e| format!("không đọc được `{p}`: {e}"))
        }
        fn read_theo_handle(&mut self, p: &str) -> Result<DocQuaHandle, String> {
            dat_tep(&self.root, &self.tep, &self.ngoai, true);
            crate::ngoai::doc_theo_handle(p, std::path::Path::new("/proc/self/fd"))
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

    #[cfg(target_os = "linux")]
    #[test]
    fn handle_ngoai_root_bi_chan_tat_dinh() {
        let d = tam("st137-thang");
        let root = d.join("root");
        std::fs::create_dir_all(&root).expect("root");
        let ngoai = d.join("ngoai.txt");
        std::fs::write(&ngoai, "NGOAI").expect("ngoai");
        let tep = root.join("tep");
        std::fs::write(&tep, "TRONG").expect("tep");
        let q = quyen_dir(&root);
        let mut host = HostThangOMo {
            root: root.clone(),
            tep: tep.clone(),
            ngoai,
        };
        let mut g = FsGuard {
            trong: &mut host,
            quyen: &q,
        };
        let kq = g.read(tep.to_str().expect("utf8"));
        match &kq {
            Err(e) => {
                assert!(e.contains("VS0604"), "phải là lỗi quyền VS0604: {e}");
                assert!(
                    e.contains("ĐÃ MỞ") || e.contains("đã mở"),
                    "thông điệp phải nêu tệp ĐÃ MỞ: {e}"
                );
            }
            Ok(s) => panic!("đọc được tệp ngoài root (handle ra ngoài): {s:?}"),
        }
        assert!(
            !kq.unwrap_or_default().contains("NGOAI"),
            "nội dung ngoài root đã lọt ra"
        );
        let _ = std::fs::remove_dir_all(&d);
    }

    /// **ST-137 (VD-141)** — host đổi tên **sau khi đã mở+đọc**: nội dung phải là nội dung **của
    /// handle** (`TRONG`), quyết định quyền theo **đường dẫn thật của handle**, không theo tên đã
    /// đổi (tên lúc này trỏ ra ngoài). Trước ST-137 ca này trả `VS0604` oan (hoặc lọt `NGOAI`).
    #[cfg(target_os = "linux")]
    struct HostDoiSauKhiMo {
        root: PathBuf,
        tep: PathBuf,
        ngoai: PathBuf,
    }

    #[cfg(target_os = "linux")]
    impl FsHost for HostDoiSauKhiMo {
        fn read(&mut self, p: &str) -> Result<String, String> {
            std::fs::read_to_string(p).map_err(|e| format!("không đọc được `{p}`: {e}"))
        }
        fn read_theo_handle(&mut self, p: &str) -> Result<DocQuaHandle, String> {
            let d = crate::ngoai::doc_theo_handle(p, std::path::Path::new("/proc/self/fd"))?;
            dat_tep(&self.root, &self.tep, &self.ngoai, true);
            Ok(d)
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

    #[cfg(target_os = "linux")]
    #[test]
    fn doc_lay_tu_handle_khi_ten_da_bi_doi() {
        let d = tam("st137-sau");
        let root = d.join("root");
        std::fs::create_dir_all(&root).expect("root");
        let ngoai = d.join("ngoai.txt");
        std::fs::write(&ngoai, "NGOAI").expect("ngoai");
        let tep = root.join("tep");
        std::fs::write(&tep, "TRONG").expect("tep");
        let q = quyen_dir(&root);
        let mut host = HostDoiSauKhiMo {
            root: root.clone(),
            tep: tep.clone(),
            ngoai,
        };
        let mut g = FsGuard {
            trong: &mut host,
            quyen: &q,
        };
        let kq = g.read(tep.to_str().expect("utf8"));
        assert_eq!(
            kq.as_deref(),
            Ok("TRONG"),
            "phải trả nội dung CỦA HANDLE (TRONG), không theo tên đã đổi"
        );
        let _ = std::fs::remove_dir_all(&d);
    }

    /// **ST-137 (VD-141)** — host **thật** nhưng không lấy được đường dẫn thật của handle
    /// (`/proc` lỗi/không mount) ⇒ tầng quyền **từ chối** `VS0604`, **không** quay về kiểm theo tên.
    /// Host giả ở đây trả đúng thứ tự: trước ST-137 nó đã đọc được `NGOAI` rồi; nay phải bị chặn.
    struct HostKhongProc;

    impl FsHost for HostKhongProc {
        fn read(&mut self, _p: &str) -> Result<String, String> {
            Ok("NGOAI".to_string())
        }
        fn read_theo_handle(&mut self, _p: &str) -> Result<DocQuaHandle, String> {
            Ok(DocQuaHandle {
                noi_dung: "NGOAI".to_string(),
                danh_tinh: (7, 7),
                duong_dan: DuongDanHandle::KhongLayDuoc(
                    "không đọc được `/proc/self/fd/3`: No such file or directory".to_string(),
                ),
            })
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

    #[test]
    fn proc_khong_doc_duoc_thi_tu_choi() {
        let d = tam("st137-proc");
        let root = d.join("root");
        std::fs::create_dir_all(&root).expect("root");
        let tep = root.join("tep");
        std::fs::write(&tep, "TRONG").expect("tep");
        let q = quyen_dir(&root);
        let mut host = HostKhongProc;
        let mut g = FsGuard {
            trong: &mut host,
            quyen: &q,
        };
        let kq = g.read(tep.to_str().expect("utf8"));
        let e = kq.expect_err("phải TỪ CHỐI khi không đọc được /proc/self/fd");
        assert!(e.contains("VS0604"), "phải là VS0604: {e}");
        assert!(e.contains("/proc"), "thông điệp phải nêu /proc: {e}");
        let _ = std::fs::remove_dir_all(&d);
    }

    /// **ST-137** — kiểm **thẳng** host thật: `/proc` không có (tham số `goc_proc` trỏ thư mục
    /// không tồn tại) ⇒ `KhongLayDuoc`; `/proc` thật ⇒ `Co(đường dẫn trong root)`.
    #[cfg(target_os = "linux")]
    #[test]
    fn doc_theo_handle_bao_dung_duong_dan_cua_proc() {
        let d = tam("st137-doc");
        let root = d.join("root");
        std::fs::create_dir_all(&root).expect("root");
        let tep = root.join("tep");
        std::fs::write(&tep, "TRONG").expect("tep");
        let root_c = std::fs::canonicalize(&root).expect("canon root");
        let p = tep.to_str().expect("utf8");
        // /proc "không đọc được": trỏ vào một thư mục không tồn tại (không cần unmount procfs).
        let khong_proc = d.join("khong-co-proc");
        let d1 = crate::ngoai::doc_theo_handle(p, &khong_proc).expect("mở được tệp");
        assert!(
            matches!(&d1.duong_dan, DuongDanHandle::KhongLayDuoc(m) if m.contains("khong-co-proc")),
            "phải báo KhongLayDuoc khi không đọc được /proc: {:?}",
            d1.duong_dan
        );
        // /proc thật: đường dẫn handle phải nằm trong root và là tệp đã mở.
        let d2 = crate::ngoai::doc_theo_handle(p, std::path::Path::new("/proc/self/fd"))
            .expect("mở được tệp");
        match &d2.duong_dan {
            DuongDanHandle::Co(dd) => {
                assert!(
                    dd.starts_with(&root_c),
                    "đường dẫn handle phải trong root: {}",
                    dd.display()
                );
            }
            khac => panic!("phải là Co(đường dẫn handle), gặp {khac:?}"),
        }
        assert_eq!(d2.noi_dung, "TRONG");
        assert!(d2.danh_tinh.0 != 0, "phải báo danh tính (dev, ino)");
        let _ = std::fs::remove_dir_all(&d);
    }

    /// **ST-165 (nợ #138)** — từ chối vì tên đã cho **phân giải ra đích khác** (vd
    /// `/proc/self/fd/0` ⇒ `/dev/null`) thì thông điệp phải nêu **đích thật**: nếu chỉ nói
    /// "thiếu `cap[Dir]` cho `/proc/self/fd/0`", người khai `cap[Dir]` cho `/proc` tưởng cap của
    /// mình sai. Đỏ-trước: thông điệp cũ không có đích đã phân giải.
    #[cfg(unix)]
    #[test]
    fn thong_diep_tu_choi_neu_dich_that_khac_ten() {
        let d = tam("st165-dich");
        let root = d.join("root");
        std::fs::create_dir_all(&root).expect("root");
        let ngoai = d.join("ngoai.txt");
        std::fs::write(&ngoai, "NGOAI").expect("ngoai");
        let link = root.join("link");
        std::os::unix::fs::symlink(&ngoai, &link).expect("symlink");
        let q = quyen_dir(&root);
        let e = q
            .kiem_fs(link.to_str().expect("utf8"), false)
            .expect_err("phải TỪ CHỐI đích ngoài root");
        assert!(e.contains("VS0604"), "phải là VS0604: {e}");
        assert!(
            e.contains("phân giải tới"),
            "thông điệp phải nói tên đã phân giải: {e}"
        );
        let dich = std::fs::canonicalize(&ngoai).expect("canon đích");
        assert!(
            e.contains(&dich.display().to_string()),
            "thông điệp phải nêu đích thật {}: {e}",
            dich.display()
        );
        let _ = std::fs::remove_dir_all(&d);
    }

    /// **ST-165 (nợ #138)** — `/proc/self/fd/1` trỏ tới `pipe:[…]`: vật đã mở **không có đường
    /// dẫn** ⇒ phải nói rõ "không phải đường dẫn", **không** báo nhầm "thiếu `cap[Dir]`".
    /// (Trước ST-165: rơi vào nhánh "ngoài root" ⇒ thông điệp sai chẩn đoán.)
    #[test]
    fn doi_tuong_khong_phai_duong_dan_bao_ro_ly_do() {
        let d = tam("st165-pipe");
        let root = d.join("root");
        std::fs::create_dir_all(&root).expect("root");
        let q = quyen_dir(&root);
        let e = q
            .kiem_canon(Path::new("pipe:[123]"), "/proc/self/fd/1", false)
            .expect_err("phải TỪ CHỐI đối tượng không phải đường dẫn");
        assert!(e.contains("VS0604"), "phải là VS0604: {e}");
        assert!(
            e.contains("không phải đường dẫn"),
            "phải nói rõ vật không có đường dẫn: {e}"
        );
        assert!(e.contains("pipe:[123]"), "phải nêu đích: {e}");
        assert!(
            !e.contains("thiếu `cap[Dir]`"),
            "không được báo nhầm thiếu cap[Dir]: {e}"
        );
        let _ = std::fs::remove_dir_all(&d);
    }

    /// **ST-165 (nợ #138)** — `/proc` **không đọc được** (host thật) thì người vận hành phải biết
    /// **cần procfs** (không chỉ thấy "từ chối"): thông điệp phải nêu `procfs` + `/proc/self/fd`.
    #[test]
    fn thong_diep_khong_doc_duoc_proc_co_huong_dan_van_hanh() {
        let d = tam("st165-khong-proc");
        let root = d.join("root");
        std::fs::create_dir_all(&root).expect("root");
        let tep = root.join("tep");
        std::fs::write(&tep, "TRONG").expect("tep");
        let q = quyen_dir(&root);
        let mut host = HostKhongProc;
        let mut g = FsGuard {
            trong: &mut host,
            quyen: &q,
        };
        let e = g
            .read(tep.to_str().expect("utf8"))
            .expect_err("phải TỪ CHỐI khi không đọc được /proc/self/fd");
        assert!(e.contains("VS0604"), "phải là VS0604: {e}");
        assert!(
            e.contains("procfs") && e.contains("/proc/self/fd"),
            "phải hướng dẫn kiểm/mount procfs: {e}"
        );
        let _ = std::fs::remove_dir_all(&d);
    }

    /// **ST-165 (nợ #138)** — khi `/proc` **đọc được** (Linux thật), đọc `/proc/self/status` dưới
    /// `cap[Dir] /proc` **không** bị `VS0604`; bước kiểm theo tên + quyết định trên handle
    /// (`/proc/<pid>/status`) đều nằm trong root. Đây là ca "xanh-sau" cho đúng yêu cầu của nợ:
    /// `/proc` đọc được ⇒ đọc được thật.
    #[cfg(target_os = "linux")]
    #[test]
    fn doc_proc_status_khi_proc_doc_duoc_khong_bi_vs0604() {
        let q = quyen_dir(Path::new("/proc"));
        let c = q
            .kiem_fs_canon("/proc/self/status", false)
            .expect("phải CHO đọc /proc/self/status khi cap[Dir] /proc");
        let c = c.expect("quyền bật ⇒ phải trả đường dẫn đã canonicalize");
        assert!(
            c.starts_with("/proc"),
            "đường dẫn canonicalize phải trong /proc: {}",
            c.display()
        );
    }
}
