// SPDX-License-Identifier: MIT OR Apache-2.0
//! **Host hệ thống tệp** (ST-33, VD-266) — `vis-interp`/`vis-vm` **không** biết `std::fs`;
//! `vis-cli` cài bằng `std::fs`. Tách khỏi `builtin.rs` để giữ luật **R1** (≤ 1.500 dòng/tệp).
//!
//! Thiếu host ⇒ `VS0604` "chưa hỗ trợ ở môi trường này" (cùng mẫu `Ngoai`, VD-60 (2)). Mọi helper
//! mới của VD-266 có **default** trả `Err("chưa hỗ trợ")` ⇒ host cũ/mock vẫn biên dịch.

/// Một mục thư mục **kèm metadata** (ST-42) — cho `ls`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MucFs {
    pub name: String,
    pub size: u64,
    pub modified_ns: i64,
    pub is_file: bool,
    /// ST-46: bit quyền kiểu POSIX (0 = chưa biết ⇒ in `-` trong `ls --long`).
    pub mode: u32,
}

/// **ST-137 (VD-141)**: đường dẫn **thật của chính handle** đã mở — để tầng quyền quyết định trên
/// **vật** đã mở thay vì trên **tên** (đóng cửa sổ TOCTOU của đường đọc).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DuongDanHandle {
    /// Host **không có** không gian tên thật (`--fs-mock`) **hoặc** nền **không có** primitive
    /// fd→path trong std an toàn (macOS: không có `/proc`). Tầng quyền giữ bước xác nhận của ST-124.
    KhongCo,
    /// Đường dẫn thật của handle (Linux: `/proc/self/fd/<fd>`), kernel đã giải symlink.
    Co(std::path::PathBuf),
    /// Host **thật** nhưng **không lấy được** đường dẫn của handle (`/proc` lỗi/không mount)
    /// ⇒ khi bật quyền phải **từ chối** (`VS0604`), **không** quay về cách cũ.
    KhongLayDuoc(String),
}

/// **ST-137 (VD-141)**: kết quả **một lần mở** — nội dung **đọc từ handle** + danh tính + đường dẫn
/// thật của chính handle đó.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocQuaHandle {
    pub noi_dung: String,
    /// `(dev, ino)` của handle; `(0, 0)` = host không báo (mock).
    pub danh_tinh: (u64, u64),
    pub duong_dan: DuongDanHandle,
}

/// **VD-266**: tách dòng dùng chung cho `std.fs.read_lines` — cùng ngữ nghĩa `lines()`:
/// tách theo `\n`, bỏ `\r` ở **cuối mỗi dòng** (CRLF), và bỏ phần tử rỗng **cuối** do newline
/// kết thúc tệp. Dòng rỗng ở giữa vẫn giữ.
pub fn chia_dong(s: &str) -> Vec<String> {
    let mut ds: Vec<String> = s
        .split('\n')
        .map(|x| x.strip_suffix('\r').unwrap_or(x).to_string())
        .collect();
    if ds.last().is_some_and(|x| x.is_empty()) {
        ds.pop();
    }
    ds
}

pub trait FsHost {
    /// Đọc cả tệp thành chuỗi (UTF-8).
    fn read(&mut self, p: &str) -> Result<String, String>;
    /// Ghi đè cả tệp.
    fn write(&mut self, p: &str, s: &str) -> Result<(), String>;
    /// Tệp/thư mục có tồn tại.
    fn exists(&mut self, p: &str) -> bool;

    /// **ST-83 (VD-109)**: như [`exists`](Self::exists) nhưng **có thể báo vượt quyền** (chuỗi bắt
    /// đầu bằng [`DAU_QUYEN`](crate::stdlib::DAU_QUYEN)) ⇒ `VS0604`. Default: gọi `exists` (host cũ
    /// không đổi chữ ký).
    fn exists_quyen(&mut self, p: &str) -> Result<bool, String> {
        Ok(self.exists(p))
    }
    /// **ST-124 (VD-130b)**: như [`read`](Self::read) nhưng trả **kèm danh tính tệp ĐÃ MỞ**
    /// `(dev, ino)` để tầng quyền (`vis-cli::quyen::FsGuard`) xác nhận **mở-rồi-kiểm** chống TOCTOU
    /// kiểm-rồi-mở. `(0, 0)` = host **không báo** danh tính (mock, hoặc nền không có inode) ⇒ tầng
    /// quyền giữ hành vi cũ. Default: gọi `read` (impl cũ không đổi chữ ký).
    fn read_danh_tinh(&mut self, p: &str) -> Result<(String, u64, u64), String> {
        Ok((self.read(p)?, 0, 0))
    }
    /// **ST-137 (VD-141)**: như [`read_danh_tinh`](Self::read_danh_tinh) nhưng trả kèm **đường dẫn
    /// thật của chính handle đã mở** (Linux: `/proc/self/fd/<fd>`) + nội dung **đọc từ handle**.
    /// Tầng quyền quyết định trên **vật đã mở**, không trên tên ⇒ đóng cửa sổ TOCTOU.
    /// Default: `read_danh_tinh` + [`DuongDanHandle::KhongCo`] (host cũ không đổi chữ ký).
    fn read_theo_handle(&mut self, p: &str) -> Result<DocQuaHandle, String> {
        let (noi_dung, dev, ino) = self.read_danh_tinh(p)?;
        Ok(DocQuaHandle {
            noi_dung,
            danh_tinh: (dev, ino),
            duong_dan: DuongDanHandle::KhongCo,
        })
    }
    /// Tên các mục trong thư mục, **đã sắp xếp**, **không** đệ quy.
    fn list(&mut self, p: &str) -> Result<Vec<String>, String>;

    /// ST-42: mục thư mục **kèm metadata** (`ls`). Default: **chưa hỗ trợ** ⇒ impl cũ vẫn biên dịch.
    fn list_entries(&mut self, _p: &str) -> Result<Vec<MucFs>, String> {
        Err("môi trường này không cung cấp metadata thư mục".to_string())
    }

    /// **VD-266**: đọc tệp thành **danh sách dòng** — tách `\n`, bỏ `\r` cuối dòng, không sinh
    /// dòng rỗng cuối (xem [`chia_dong`]). Default: **chưa hỗ trợ** ⇒ host cũ/mock vẫn biên dịch.
    fn read_lines(&mut self, _p: &str) -> Result<Vec<String>, String> {
        Err("chưa hỗ trợ".to_string())
    }

    /// **VD-266**: thêm vào **cuối** tệp (tạo nếu thiếu), **không** cắt nội dung cũ.
    /// Default: **chưa hỗ trợ** ⇒ host cũ/mock vẫn biên dịch.
    fn append(&mut self, _p: &str, _s: &str) -> Result<(), String> {
        Err("chưa hỗ trợ".to_string())
    }

    /// **VD-266**: tạo **một** thư mục (như `mkdir`, **không** phải `mkdir -p`); lỗi nếu đã tồn tại
    /// hoặc thiếu thư mục cha. Default: **chưa hỗ trợ** ⇒ host cũ/mock vẫn biên dịch.
    fn mkdir(&mut self, _p: &str) -> Result<(), String> {
        Err("chưa hỗ trợ".to_string())
    }

    /// **VD-266**: xoá **một tệp** (không đệ quy); **từ chối** thư mục.
    /// Default: **chưa hỗ trợ** ⇒ host cũ/mock vẫn biên dịch.
    fn remove(&mut self, _p: &str) -> Result<(), String> {
        Err("chưa hỗ trợ".to_string())
    }

    /// **VD-268**: liệt kê **đệ quy** dưới `p` — mỗi mục là đường dẫn **tương đối** so với `p`,
    /// phân cách `/`; **thư mục** kèm `/` ở cuối; **DFS**, tại mỗi cấp sắp theo **byte của tên**;
    /// mục ẩn vẫn gồm; **symlink bị bỏ hoàn toàn** (không in, không đệ quy — tránh vòng).
    /// Default: **chưa hỗ trợ** ⇒ host cũ/mock vẫn biên dịch.
    fn walk(&mut self, _p: &str) -> Result<Vec<String>, String> {
        Err("chưa hỗ trợ".to_string())
    }
}

/// **VD-268** — trần độ dài **mẫu** glob (byte): mỗi mục tốn `O(|mẫu| · |đường|)`, nên mẫu sinh
/// tự động khổng lồ bị từ chối **trước** khi nhân chi phí với số mục của `walk`.
pub const GLOB_MAU_TOI_DA: usize = 4096;

/// **VD-268** — một mảnh của mẫu glob sau khi phân tích.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ManhGlob {
    /// Ký tự thường — gồm cả `[`, `]`, `{`, `}`, `\` (**không** có lớp ký tự/escape).
    Chu(char),
    /// `?` — đúng **một** ký tự, trừ `/`.
    Mot,
    /// `*` — không hoặc nhiều ký tự, trừ `/`.
    Sao,
    /// `**` ở **cuối** mẫu — không hoặc nhiều ký tự, **gồm** `/`.
    Sau,
    /// `**/` — không hoặc nhiều **thành phần** đầy đủ (mỗi thành phần kết thúc bằng `/`).
    SauThuMuc,
}

/// **VD-268** — phân tích mẫu glob. Chỉ ba ký tự đặc biệt `*`, `?`, `**`:
///
/// - `*` = không hoặc nhiều ký tự, **trừ** `/`;
/// - `?` = đúng **một ký tự** (Unicode), **trừ** `/`;
/// - `**` = không hoặc nhiều ký tự, **gồm** `/`, nhưng **chỉ** hợp lệ khi là **một thành phần
///   đầy đủ** của mẫu (đầu/cuối mẫu hoặc giữa hai `/`). `**/` khớp **không hoặc nhiều** thư mục
///   (nên `**/*.txt` khớp cả `a.txt` ở gốc); `**` ở cuối khớp **mọi thứ còn lại** (kể cả rỗng).
///
/// `[`, `]`, `{`, `}`, `\` là **ký tự thường**. Mẫu rỗng, mẫu tuyệt đối (bắt đầu `/`), mẫu quá dài
/// ([`GLOB_MAU_TOI_DA`]), và `**` không phải thành phần đầy đủ (`a**b`, `***`) ⇒ `Err`.
fn phan_tich_glob(mau: &str) -> Result<Vec<ManhGlob>, String> {
    if mau.is_empty() {
        return Err("mẫu glob rỗng".to_string());
    }
    if mau.len() > GLOB_MAU_TOI_DA {
        return Err(format!(
            "mẫu glob quá dài ({} byte > {GLOB_MAU_TOI_DA})",
            mau.len()
        ));
    }
    if mau.starts_with('/') {
        return Err("mẫu glob phải là đường dẫn tương đối (không bắt đầu bằng `/`)".to_string());
    }
    let cs: Vec<char> = mau.chars().collect();
    let mut ra = Vec::with_capacity(cs.len());
    let mut i = 0;
    while i < cs.len() {
        match cs[i] {
            '*' if cs.get(i + 1) == Some(&'*') => {
                let dau = i == 0 || cs[i - 1] == '/';
                let cuoi = i + 2 == cs.len() || cs[i + 2] == '/';
                if !dau || !cuoi {
                    return Err(format!(
                        "mẫu glob không hợp lệ: `**` phải là một thành phần đầy đủ (đầu/cuối \
                         mẫu hoặc giữa hai `/`), tại ký tự {i}"
                    ));
                }
                if i + 2 < cs.len() {
                    ra.push(ManhGlob::SauThuMuc);
                    i += 3; // `**` + `/`
                } else {
                    ra.push(ManhGlob::Sau);
                    i += 2;
                }
            }
            '*' => {
                ra.push(ManhGlob::Sao);
                i += 1;
            }
            '?' => {
                ra.push(ManhGlob::Mot);
                i += 1;
            }
            c => {
                ra.push(ManhGlob::Chu(c));
                i += 1;
            }
        }
    }
    Ok(ra)
}

/// **VD-268** — khớp **một** chuỗi với các mảnh đã phân tích bằng DP theo vị trí (không đệ quy,
/// `O(|mảnh| · |s|)`); `true` ⇔ khớp **toàn bộ** `s`.
fn khop_manh(manh: &[ManhGlob], s: &[char]) -> bool {
    let n = s.len();
    let mut hien = vec![false; n + 1];
    hien[0] = true;
    for m in manh {
        let mut sau = vec![false; n + 1];
        match *m {
            ManhGlob::Chu(c) => {
                for (j, x) in s.iter().enumerate() {
                    if hien[j] && *x == c {
                        sau[j + 1] = true;
                    }
                }
            }
            ManhGlob::Mot => {
                for (j, x) in s.iter().enumerate() {
                    if hien[j] && *x != '/' {
                        sau[j + 1] = true;
                    }
                }
            }
            ManhGlob::Sao => {
                // `*`: cửa sổ khớp chỉ được nới qua ký tự khác `/`.
                let mut mo = false;
                for j in 0..=n {
                    if hien[j] {
                        mo = true;
                    }
                    sau[j] = mo;
                    if j < n && s[j] == '/' {
                        mo = false;
                    }
                }
            }
            ManhGlob::Sau => {
                // `**` (cuối): tiền tố bất kỳ, gồm `/`.
                let mut mo = false;
                for j in 0..=n {
                    if hien[j] {
                        mo = true;
                    }
                    sau[j] = mo;
                }
            }
            ManhGlob::SauThuMuc => {
                // `**/`: khớp rỗng, hoặc dừng ngay **sau** một `/` (một thành phần đầy đủ).
                let mut truoc = false;
                for j in 0..=n {
                    sau[j] = hien[j] || (j > 0 && s[j - 1] == '/' && truoc);
                    if hien[j] {
                        truoc = true;
                    }
                }
            }
        }
        if !sau.iter().any(|x| *x) {
            return false;
        }
        hien = sau;
    }
    hien[n]
}

/// **VD-268** — tên **cuối** của một mục `walk` (bỏ `/` cuối của thư mục): dùng khi mẫu **không**
/// có `/` ⇒ khớp theo **basename** (mọi tầng), không theo cả đường dẫn.
fn ten_cuoi(muc: &str) -> &str {
    let m = muc.strip_suffix('/').unwrap_or(muc);
    m.rsplit('/').next().unwrap_or(m)
}

/// **VD-268** — lọc kết quả [`FsHost::walk`] bằng **mẫu glob thuần** (không thêm method host ⇒
/// host thật, `--fs-mock`, interp và VM dùng **cùng** một ngữ nghĩa):
///
/// - mục trả về giữ **nguyên dạng** của `walk` (tương đối so với root, thư mục kèm `/`);
/// - mẫu **không có** `/` ⇒ khớp **basename** (nên `*.txt` khớp cả `b/c.txt`); mục ẩn vẫn khớp
///   (`*` khớp cả tên bắt đầu bằng `.`);
/// - mẫu **có** `/` ⇒ khớp **toàn bộ** đường dẫn tương đối. Mỗi mục được thử ở dạng **không** có
///   `/` cuối; **thư mục** còn được thử thêm dạng **có** `/` cuối **chỉ khi** mẫu kết thúc bằng `/`
///   (chọn thư mục, vd `*/`) hoặc bằng `/**` (để `**` cuối khớp **rỗng** và gồm chính thư mục đó,
///   vd `b/**` ⇒ có `b/`). Nhờ vậy `b/*` chọn **con** của `b` chứ không chọn chính `b/`; kết quả
///   vẫn in thư mục kèm `/`;
/// - kết quả **sắp theo byte** (không theo thứ tự DFS của `walk`), **không** trùng;
/// - mẫu không hợp lệ ⇒ `Err` (lỗi **dữ liệu**, không trap) — xem [`phan_tich_glob`].
///
/// Hàm **thuần**: không chạm hệ thống tệp, không theo symlink (việc đó đã do `walk` quyết định).
pub fn loc_glob(muc: &[String], mau: &str) -> Result<Vec<String>, String> {
    let manh = phan_tich_glob(mau)?;
    let theo_ten = !mau.contains('/');
    // `/` cuối mẫu = chỉ chọn thư mục; `/**` cuối = `**` được khớp **rỗng** (gồm chính thư mục).
    let giu_slash = mau.ends_with('/');
    let sau_cuoi = mau == "**" || mau.ends_with("/**");
    let mut ra: Vec<String> = Vec::new();
    for m in muc {
        let khop = if theo_ten {
            let ten: Vec<char> = ten_cuoi(m).chars().collect();
            khop_manh(&manh, &ten)
        } else {
            let la_thu_muc = m.ends_with('/');
            // Dạng có `/` cuối chỉ dùng khi mẫu cần nó (tránh `b/*` khớp chính `b/`).
            let mut khop = if !la_thu_muc || giu_slash || sau_cuoi {
                let duong: Vec<char> = m.chars().collect();
                khop_manh(&manh, &duong)
            } else {
                false
            };
            if !khop {
                if let Some(khong_slash) = m.strip_suffix('/') {
                    let duong: Vec<char> = khong_slash.chars().collect();
                    khop = khop_manh(&manh, &duong);
                }
            }
            khop
        };
        if khop {
            ra.push(m.clone());
        }
    }
    ra.sort_by(|a, b| a.as_bytes().cmp(b.as_bytes()));
    Ok(ra)
}
