// SPDX-License-Identifier: MIT OR Apache-2.0
//! **Host chương trình ngoài** (ST-27) — chỉ ở `vis-cli`; `vis-interp` chỉ thấy `trait Ngoai`.
//!
//! Chạy bằng [`std::process::Command`] (mảng đối số, **không** `sh -c`), env kế thừa, timeout (mặc
//! định 10 s), cap đồng thời = 1 (VS0/VS1 chưa có job). `exit != 0` ⇒ `VS0501` kèm **stderr** của
//! tiến trình (không trộn stdout). Mỗi lần chạy in một dòng `EXEC …` khi `--explain-perms`.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use vis_core::host::{DocQuaHandle, DuongDanHandle};
use vis_core::{Abort, Trap, Value};
use vis_diag::{COMMAND_CODES, RUNTIME_CODES};
use vis_syntax::Span;

/// Runner cho chương trình ngoài.
pub struct Runner {
    /// In `EXEC …` ra stderr trước mỗi lần chạy (nhật ký quyền).
    pub giai_thich: bool,
    /// Bao nhiêu mili-giây thì bỏ cuộc (mặc định 10 000).
    pub timeout_ms: u64,
    /// **ST-93 (VD-111)**: `--strict-caps` ⇒ chỉ chạy khi `main` khai `cap[Proc]`; `true` ngoài strict.
    pub cho_phep_proc: bool,
    /// **ST-97 (VD-112)**: `Some(ds)` ⇒ chỉ chạy lệnh khớp allowlist `--grant` của `cap[Proc]`.
    pub allow_proc: Option<Vec<String>>,
}

impl Default for Runner {
    fn default() -> Self {
        Self {
            giai_thich: false,
            timeout_ms: std::env::var("VIS_CMD_TIMEOUT_MS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(10_000),
            cho_phep_proc: true,
            allow_proc: None,
        }
    }
}

fn loi(ma: &'static vis_diag::Code, msg: impl Into<String>, span: Span) -> Abort {
    Abort::Trap(Trap::new(ma, msg, span))
}

/// Cap **stdin** mỗi stage ngoài (ST-34): 8 MiB. Vượt ⇒ `VS0501` **trước khi spawn**.
pub const MAX_STDIN: usize = 8 * 1024 * 1024;

impl Runner {
    /// Chạy một chương trình ngoài, ghi `stdin` (thread riêng, đóng khi xong), có timeout.
    ///
    /// **Không** deadlock: writer chạy song song, `wait_with_output()` đọc stdout/stderr.
    /// Timeout **10 s/stage** (VD-60); thông điệp nêu **stage nào** (ST-34).
    fn chay_raw(
        &mut self,
        ten: &str,
        args: &[String],
        stdin: &str,
        span: Span,
    ) -> Result<vis_core::host::KetQua, Abort> {
        // **ST-93 (VD-111)**: cưỡng chế **ở biên host**, dùng chung cho interp + VM
        // (`Ngoai::Runner` là host duy nhất); `cap[Proc]` là quyền chạy tiến trình, **không**
        // allowlist danh sách lệnh.
        if !self.cho_phep_proc {
            return Err(loi(
                &COMMAND_CODES[3],
                format!("[quyền] cần khai `cap[Proc]` cho `{ten}`"),
                span,
            ));
        }
        // **ST-97 (VD-112)**: allowlist chỉ áp khi người dùng đã `--grant` cho `cap[Proc]`; không grant
        // ⇒ không giới hạn (tương thích).
        // **ST-123 (VD-130c)**: khớp theo **đường dẫn tuyệt đối đã canonicalize** (hai phía), không
        // theo basename.
        // **ST-314a (W3 C2)**: phân giải `ten` qua PATH **một lần**, kiểm allowlist, rồi
        // **exec chính đường tuyệt đối đó**. Trước đây `Command::new(ten)` tra PATH lần hai ⇒
        // đổi symlink/PATH giữa hai lần tra chạy được binary ngoài allowlist (TOCTOU).
        let mut duong_chay: Option<std::path::PathBuf> = None;
        if let Some(ds) = &self.allow_proc {
            let Some(d) = crate::quyen::chuan_lenh(ten) else {
                return Err(loi(
                    &COMMAND_CODES[3],
                    format!("[quyền] không phân giải được lệnh `{ten}` để kiểm allowlist"),
                    span,
                ));
            };
            if !ds.iter().any(|g| d.as_path() == std::path::Path::new(g)) {
                return Err(loi(
                    &COMMAND_CODES[3],
                    format!(
                        "[quyền] lệnh `{ten}` không nằm trong allowlist của `cap[Proc]` (đã cấp: {})",
                        ds.join(", ")
                    ),
                    span,
                ));
            }
            duong_chay = Some(d);
        }
        if stdin.len() > MAX_STDIN {
            return Err(loi(
                &RUNTIME_CODES[0],
                format!(
                    "stdin của stage `{ten}` quá lớn: {} byte (giới hạn {MAX_STDIN} byte = 8 MiB)",
                    stdin.len()
                ),
                span,
            ));
        }
        if self.giai_thich {
            eprintln!("EXEC {ten} {args:?}");
            if !stdin.is_empty() {
                eprintln!("PIPE stdin {ten} ({} byte)", stdin.len());
            }
        }
        let mut cmd = match &duong_chay {
            Some(p) => Command::new(p),
            None => Command::new(ten),
        };
        cmd.args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut con = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => {
                return Err(loi(
                    &RUNTIME_CODES[0],
                    format!("không chạy được chương trình ngoài `{ten}`: {e}"),
                    span,
                ));
            }
        };
        // Ghi stdin ở **thread riêng** rồi **đóng** (drop) — nếu không `cat`/`grep` treo.
        if let Some(mut si) = con.stdin.take() {
            let du_lieu = stdin.as_bytes().to_vec();
            std::thread::spawn(move || {
                use std::io::Write;
                // Lỗi `EPIPE` sau khi tiến trình bị giết (timeout) — **bỏ qua**.
                let _ = si.write_all(&du_lieu);
                let _ = si.flush();
            });
        }
        let han = Instant::now() + Duration::from_millis(self.timeout_ms);
        loop {
            match con.try_wait() {
                Ok(Some(_)) => break,
                Ok(None) => {
                    if Instant::now() >= han {
                        let _ = con.kill();
                        let _ = con.wait();
                        return Err(loi(
                            &RUNTIME_CODES[0],
                            format!("stage `{ten}` quá thời gian ({} ms)", self.timeout_ms),
                            span,
                        ));
                    }
                    std::thread::sleep(Duration::from_millis(5));
                }
                Err(e) => {
                    return Err(loi(
                        &RUNTIME_CODES[0],
                        format!("lỗi chờ `{ten}`: {e}"),
                        span,
                    ))
                }
            }
        }
        let out = con.wait_with_output().map_err(|e| {
            Abort::Trap(Trap::new(
                &RUNTIME_CODES[0],
                format!("lỗi đọc kết quả `{ten}`: {e}"),
                span,
            ))
        })?;
        Ok(vis_core::host::KetQua {
            stdout: String::from_utf8_lossy(&out.stdout).to_string(),
            stderr: String::from_utf8_lossy(&out.stderr).to_string(),
            code: out.status.code().unwrap_or(-1),
        })
    }
}

impl vis_core::host::Ngoai for Runner {
    fn chay(
        &mut self,
        ten: &str,
        args: &[String],
        raw: bool,
        stdin: &str,
        span: Span,
    ) -> vis_core::R<Value> {
        let kq = self.chay_raw(ten, args, stdin, span)?;
        if kq.code != 0 {
            let gon = kq.stderr.trim();
            return Err(loi(
                &RUNTIME_CODES[0],
                if gon.is_empty() {
                    format!("chương trình ngoài `{ten}` thoát mã {}", kq.code)
                } else {
                    format!("chương trình ngoài `{ten}` thoát mã {}: {gon}", kq.code)
                },
                span,
            ));
        }
        if raw {
            return Ok(Value::Str(kq.stdout));
        }
        Ok(tach_dong(&kq.stdout))
    }

    fn chay_day_du(
        &mut self,
        ten: &str,
        args: &[String],
        stdin: &str,
        span: Span,
    ) -> Result<vis_core::host::KetQua, Abort> {
        self.chay_raw(ten, args, stdin, span)
    }
}

/// Tách `stdout` thành `list[str]` — **bỏ** newline cuối, không sinh dòng rỗng thừa.
fn tach_dong(s: &str) -> Value {
    let mut t = s;
    if let Some(x) = t.strip_suffix('\n') {
        t = x;
    }
    let mut dong: Vec<Value> = Vec::new();
    if !t.is_empty() {
        for d in t.split('\n') {
            dong.push(Value::Str(d.to_string()));
        }
    }
    Value::list(dong)
}

/// **ST-137 (VD-141)**: mở tệp **một lần**, lấy **đường dẫn thật của chính handle** rồi **đọc từ
/// handle** — trả nội dung + danh tính + [`DuongDanHandle`].
///
/// Thứ tự có chủ ý: lấy đường dẫn handle **trước** khi đọc nội dung ⇒ `/proc` lỗi/không mount thì
/// **từ chối ngay** (`KhongLayDuoc`), **không** kéo nội dung ngoài vùng cấp vào bộ nhớ và **không**
/// quay về cách cũ (kiểm theo tên).
///
/// `goc_proc` = `/proc/self/fd` ở production; **tham số hoá** để test được nhánh "không đọc được
/// `/proc`" mà không cần unmount procfs.
pub(crate) fn doc_theo_handle(p: &str, goc_proc: &Path) -> Result<DocQuaHandle, String> {
    use std::io::Read as _;
    let loi = |e: std::io::Error| format!("không đọc được `{p}`: {e}");
    let mut f = std::fs::File::open(p).map_err(loi)?;
    #[cfg(unix)]
    let danh_tinh = {
        use std::os::unix::fs::MetadataExt;
        let md = f.metadata().map_err(loi)?;
        (md.dev(), md.ino())
    };
    #[cfg(not(unix))]
    let danh_tinh = {
        f.metadata().map_err(loi)?;
        (0u64, 0u64)
    };
    let duong_dan = duong_dan_cua_handle(&f, goc_proc);
    let mut noi_dung = String::new();
    f.read_to_string(&mut noi_dung).map_err(loi)?;
    Ok(DocQuaHandle {
        noi_dung,
        danh_tinh,
        duong_dan,
    })
}

/// Linux: `/proc/self/fd/<fd>` là đường dẫn **thật** của vật đã mở (kernel giải hết symlink).
#[cfg(target_os = "linux")]
fn duong_dan_cua_handle(f: &std::fs::File, goc_proc: &Path) -> DuongDanHandle {
    use std::os::fd::AsRawFd;
    let lien_ket = goc_proc.join(f.as_raw_fd().to_string());
    match std::fs::read_link(&lien_ket) {
        Ok(p) => DuongDanHandle::Co(p),
        Err(e) => {
            DuongDanHandle::KhongLayDuoc(format!("không đọc được `{}`: {e}", lien_ket.display()))
        }
    }
}

/// Nền **không có** `/proc` (macOS): std an toàn **không** có primitive fd→path —
/// `read_link("/dev/fd/N")` trả `EINVAL`, `canonicalize("/dev/fd/N")` trả đường dẫn giả của
/// `fdesc`, `fcntl(F_GETPATH)` cần `libc`/`unsafe` (ngoài VS2) ⇒ `KhongCo`: tầng quyền giữ bước
/// xác nhận của ST-124 (đo **0/50 000**), ghi thành nợ ở ST-137.
#[cfg(all(unix, not(target_os = "linux")))]
fn duong_dan_cua_handle(_f: &std::fs::File, _goc_proc: &Path) -> DuongDanHandle {
    DuongDanHandle::KhongCo
}

#[cfg(not(unix))]
fn duong_dan_cua_handle(_f: &std::fs::File, _goc_proc: &Path) -> DuongDanHandle {
    DuongDanHandle::KhongCo
}

/// **VD-268**: một cấp của `walk` — đọc tên + loại mục (**không** theo symlink), sắp theo byte,
/// rồi in mục và đệ quy vào thư mục. `tuong_doi` là tiền tố tương đối (rỗng ở root).
fn duyet_de_quy(goc: &Path, tuong_doi: &str, ra: &mut Vec<String>) -> Result<(), String> {
    let hien = goc.display();
    let rd = std::fs::read_dir(goc).map_err(|e| format!("không đọc được thư mục `{hien}`: {e}"))?;
    let mut muc: Vec<(String, bool, bool)> = Vec::new();
    for e in rd {
        let e = e.map_err(|x| format!("lỗi đọc mục trong `{hien}`: {x}"))?;
        let ft = e
            .file_type()
            .map_err(|x| format!("lỗi đọc loại mục trong `{hien}`: {x}"))?;
        muc.push((
            e.file_name().to_string_lossy().to_string(),
            ft.is_dir(),
            ft.is_symlink(),
        ));
    }
    muc.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));
    for (ten, la_thu_muc, la_symlink) in muc {
        // **VD-268**: symlink không được in và không được đệ quy ⇒ không thể đi ra ngoài root
        // hoặc lặp vô hạn.
        if la_symlink {
            continue;
        }
        let rel = if tuong_doi.is_empty() {
            ten.clone()
        } else {
            format!("{tuong_doi}/{ten}")
        };
        if la_thu_muc {
            ra.push(format!("{rel}/"));
            duyet_de_quy(&goc.join(&ten), &rel, ra)?;
        } else {
            // Tệp thường **và** mục đặc biệt (fifo/socket/…) — chỉ in tên, không đi xuống.
            ra.push(rel);
        }
    }
    Ok(())
}

/// **VD-269**: `symlink_metadata` của một đường dẫn phải là **tệp thường** và **không** phải
/// symlink (không đi theo link). `vai` chỉ dùng để dựng thông điệp ("chép"/"đổi tên").
fn metadata_tep_thuong(p: &str, vai: &str) -> Result<std::fs::Metadata, String> {
    let md = std::fs::symlink_metadata(p).map_err(|e| format!("không {vai} được `{p}`: {e}"))?;
    if md.file_type().is_symlink() {
        return Err(format!(
            "không {vai} được `{p}`: là symlink (không đi theo symlink)"
        ));
    }
    if !md.is_file() {
        return Err(format!("không {vai} được `{p}`: không phải tệp thường"));
    }
    Ok(md)
}

/// **VD-269**: đích phải **chưa tồn tại** — mọi thứ đã có (tệp/thư mục/symlink) đều bị từ chối.
/// Chỉ `NotFound` mới là "chưa có"; lỗi khác (vd thiếu quyền đọc thư mục cha) là `Err`, **không**
/// được coi là trống.
fn kiem_dich_chua_co(to: &str, vai: &str) -> Result<(), String> {
    match std::fs::symlink_metadata(to) {
        Ok(_) => Err(format!("không {vai} được `{to}`: đích đã tồn tại")),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(format!("không {vai} được `{to}`: {e}")),
    }
}

/// **VD-269**: chép **từng byte** `from` → `to` (không UTF-8), **chỉ tệp thường**, không theo
/// symlink ở **cả hai** đầu, **không** ghi đè đích.
///
/// Đích được tạo bằng `create_new` (**O_CREAT|O_EXCL**) ⇒ kể cả khi một symlink/tệp xuất hiện ở
/// đích **sau** bước kiểm, lời gọi vẫn thất bại thay vì ghi đè. Nguồn được `lstat` rồi mở; trên
/// Unix còn so `(dev, ino)` của **handle** với `lstat` để thu hẹp khe đổi tệp giữa hai syscall.
/// Khe đua **còn lại** của rename-style (đổi tên tệp nguồn nhiều lần) là giới hạn chung của std an
/// toàn, không có `unsafe`/`libc` — ghi ở báo cáo.
///
/// Nếu ghi dở rồi lỗi, tệp đích **vừa tạo** được xoá (best effort) để không để lại tệp cụt.
fn chep_tep(from: &str, to: &str) -> Result<(), String> {
    use std::io::Write as _;
    let md = metadata_tep_thuong(from, "chép")?;
    let mut nguon =
        std::fs::File::open(from).map_err(|e| format!("không chép được `{from}`: {e}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let md_handle = nguon
            .metadata()
            .map_err(|e| format!("không chép được `{from}`: {e}"))?;
        if (md_handle.dev(), md_handle.ino()) != (md.dev(), md.ino()) || !md_handle.is_file() {
            return Err(format!(
                "không chép được `{from}`: tệp nguồn bị thay đổi trong khi mở"
            ));
        }
    }
    #[cfg(not(unix))]
    {
        let _ = md;
        if !nguon
            .metadata()
            .map_err(|e| format!("không chép được `{from}`: {e}"))?
            .is_file()
        {
            return Err(format!("không chép được `{from}`: không phải tệp thường"));
        }
    }
    kiem_dich_chua_co(to, "chép")?;
    let mut dich = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(to)
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::AlreadyExists {
                format!("không chép được `{to}`: đích đã tồn tại")
            } else {
                format!("không chép được `{to}`: {e}")
            }
        })?;
    if let Err(e) = std::io::copy(&mut nguon, &mut dich).and_then(|_| dich.flush()) {
        drop(dich);
        let _ = std::fs::remove_file(to);
        return Err(format!("không chép được `{from}` → `{to}`: {e}"));
    }
    Ok(())
}

/// **VD-269**: đổi tên **cùng hệ tệp** `from` → `to`, chỉ tệp thường, không theo symlink, **từ
/// chối đích đã tồn tại** để không phụ thuộc ngữ nghĩa ghi đè của Unix.
///
/// Kiểm-rồi-`rename` có khe đua hẹp: đích xuất hiện **sau** bước kiểm thì `rename` của OS có thể
/// ghi đè. `renameat2(RENAME_NOREPLACE)` cần `libc`/`unsafe` (ngoài VS2) nên chưa dùng — ghi nợ ở
/// báo cáo; trên Linux, tệp nguồn vẫn được `lstat` trước khi đổi tên.
fn doi_ten_tep(from: &str, to: &str) -> Result<(), String> {
    metadata_tep_thuong(from, "đổi tên")?;
    kiem_dich_chua_co(to, "đổi tên")?;
    std::fs::rename(from, to).map_err(|e| format!("không đổi tên được `{from}` → `{to}`: {e}"))
}

/// **VD-270 (mkdir-all)**: tạo **đệ quy** mọi thành phần còn thiếu của `p`, **không đi theo
/// symlink** ở **bất kỳ** thành phần nào (kể cả đích cuối). Mỗi tiền tố được kiểm bằng
/// `symlink_metadata`:
///
/// - chưa có ⇒ `create_dir` (một cấp; đua `AlreadyExists` được kiểm lại);
/// - đã là thư mục thật ⇒ đi tiếp (đích cuối đã là thư mục ⇒ `Ok`, idempotent);
/// - là symlink hoặc tệp/loại khác ⇒ `Err` **dữ liệu** (không tạo gì thêm).
///
/// Khác `std::fs::create_dir_all` ở đúng điểm **symlink**: bản std đi theo symlink tới thư mục;
/// bản này **từ chối** để không ghi ra ngoài đường dẫn người dùng đã nêu. Đường dẫn rỗng ⇒ `Err`.
/// Không tự kiểm `cap[Dir]` — việc đó do `FsGuard` làm **trước** khi gọi host này.
fn tao_thu_muc_de_quy(p: &str) -> Result<(), String> {
    if p.is_empty() {
        return Err("đường dẫn rỗng".to_string());
    }
    let mut hien_tai = PathBuf::new();
    for thanh in Path::new(p).components() {
        hien_tai.push(thanh);
        match std::fs::symlink_metadata(&hien_tai) {
            Ok(md) => {
                if md.file_type().is_symlink() {
                    return Err(format!(
                        "`{}` là symlink (không đi theo)",
                        hien_tai.display()
                    ));
                }
                if !md.is_dir() {
                    return Err(format!(
                        "`{}` đã tồn tại và không phải thư mục",
                        hien_tai.display()
                    ));
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                match std::fs::create_dir(&hien_tai) {
                    Ok(()) => {}
                    // Đua với tiến trình khác: mục vừa xuất hiện vẫn phải là thư mục thật.
                    Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                        let md = std::fs::symlink_metadata(&hien_tai)
                            .map_err(|e| format!("`{}`: {e}", hien_tai.display()))?;
                        if md.file_type().is_symlink() || !md.is_dir() {
                            return Err(format!(
                                "`{}` đã tồn tại và không phải thư mục",
                                hien_tai.display()
                            ));
                        }
                    }
                    Err(e) => return Err(format!("`{}`: {e}", hien_tai.display())),
                }
            }
            Err(e) => return Err(format!("`{}`: {e}", hien_tai.display())),
        }
    }
    Ok(())
}

/// **Host hệ thống tệp** (ST-33) — `std::fs` **chỉ** ở đây; `vis-interp`/`vis-vm` chỉ thấy trait.
///
/// Chốt an toàn (Agent 0 duyệt): **cho phép** đường dẫn tuyệt đối và `..` (không sandbox — consent
/// thật/`cap[Dir]` là **VS2**); mỗi lần truy cập in `FS …` ra stderr khi `--explain-perms`.
impl vis_core::host::FsHost for Runner {
    fn read(&mut self, p: &str) -> Result<String, String> {
        self.nhat_ky("read", p);
        std::fs::read_to_string(p).map_err(|e| format!("không đọc được `{p}`: {e}"))
    }

    /// **ST-124 (VD-130b)**: mở **một lần** rồi trả `(nội dung, dev, ino)` của **chính handle** đó —
    /// tầng quyền dùng `(dev, ino)` để xác nhận "tệp đã mở" đúng là tệp mà đường dẫn canonicalize
    /// trỏ tới (chống TOCTOU kiểm-rồi-mở) mà **không** cần `unsafe`.
    #[cfg(unix)]
    fn read_danh_tinh(&mut self, p: &str) -> Result<(String, u64, u64), String> {
        use std::io::Read as _;
        use std::os::unix::fs::MetadataExt;
        self.nhat_ky("read", p);
        let loi = |e: std::io::Error| format!("không đọc được `{p}`: {e}");
        let mut f = std::fs::File::open(p).map_err(loi)?;
        let md = f.metadata().map_err(loi)?;
        let mut s = String::new();
        f.read_to_string(&mut s).map_err(loi)?;
        Ok((s, md.dev(), md.ino()))
    }

    /// **ST-137 (VD-141)**: đọc **từ handle** + báo **đường dẫn thật của handle** để tầng quyền
    /// quyết định trên vật đã mở (Linux: `/proc/self/fd`).
    fn read_theo_handle(&mut self, p: &str) -> Result<DocQuaHandle, String> {
        self.nhat_ky("read", p);
        doc_theo_handle(p, Path::new("/proc/self/fd"))
    }

    fn write(&mut self, p: &str, s: &str) -> Result<(), String> {
        self.nhat_ky("write", p);
        std::fs::write(p, s).map_err(|e| format!("không ghi được `{p}`: {e}"))
    }

    /// **VD-266**: thêm **cuối** tệp (tạo nếu thiếu), **không** cắt — `append(true)` + `create(true)`.
    fn append(&mut self, p: &str, s: &str) -> Result<(), String> {
        use std::io::Write as _;
        self.nhat_ky("append", p);
        let loi = |e: std::io::Error| format!("không ghi được `{p}`: {e}");
        let mut f = std::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(p)
            .map_err(loi)?;
        f.write_all(s.as_bytes()).map_err(loi)
    }

    /// **VD-266**: đọc tệp rồi tách dòng bằng [`chia_dong`](vis_core::host::chia_dong) —
    /// **cùng** ngữ nghĩa với mock.
    fn read_lines(&mut self, p: &str) -> Result<Vec<String>, String> {
        self.nhat_ky("read_lines", p);
        std::fs::read_to_string(p)
            .map(|s| vis_core::host::chia_dong(&s))
            .map_err(|e| format!("không đọc được `{p}`: {e}"))
    }

    /// **VD-266**: tạo **một** thư mục (`create_dir`, không phải `mkdir -p`).
    fn mkdir(&mut self, p: &str) -> Result<(), String> {
        self.nhat_ky("mkdir", p);
        std::fs::create_dir(p).map_err(|e| format!("không tạo được thư mục `{p}`: {e}"))
    }

    /// **VD-270 (mkdir-all)**: tạo **đệ quy** mọi thư mục thiếu — xem [`tao_thu_muc_de_quy`]
    /// (không đi theo symlink; đích đã là thư mục ⇒ `Ok`; tệp/symlink ⇒ `Err` dữ liệu).
    fn mkdir_all(&mut self, p: &str) -> Result<(), String> {
        self.nhat_ky("mkdir_all", p);
        tao_thu_muc_de_quy(p).map_err(|e| format!("không tạo được thư mục `{p}`: {e}"))
    }

    /// **VD-266**: xoá **một tệp**, **từ chối** thư mục (không đệ quy). `symlink_metadata` để không
    /// đi theo symlink cuối: symlink trỏ tới thư mục vẫn bị xoá **chính link**, không đụng đích.
    fn remove(&mut self, p: &str) -> Result<(), String> {
        self.nhat_ky("remove", p);
        match std::fs::symlink_metadata(p) {
            Ok(md) if md.is_dir() => Err(format!("không xoá được `{p}`: là thư mục (chỉ xoá tệp)")),
            _ => std::fs::remove_file(p).map_err(|e| format!("không xoá được `{p}`: {e}")),
        }
    }

    /// **VD-270 (fs-current-dir)**: `std::env::current_dir` trả đường dẫn **tuyệt đối**;
    /// `OsString::into_string` **từ chối** UTF-8 không hợp lệ (không cắt cụt bằng lossy). Không
    /// có đường dẫn người dùng nên **không** kiểm `cap` — tầng quyền không tham gia.
    fn current_dir(&mut self) -> Result<String, String> {
        self.nhat_ky("current_dir", ".");
        std::env::current_dir()
            .map_err(|e| format!("không lấy được thư mục làm việc hiện tại: {e}"))?
            .into_os_string()
            .into_string()
            .map_err(|_| "thư mục làm việc hiện tại không phải UTF-8".to_string())
    }

    /// **VD-269**: chép tệp — xem [`chep_tep`] (chỉ tệp thường, không symlink, không ghi đè đích).
    fn copy(&mut self, from: &str, to: &str) -> Result<(), String> {
        self.nhat_ky("copy", from);
        self.nhat_ky("copy →", to);
        chep_tep(from, to)
    }

    /// **VD-269**: đổi tên tệp cùng hệ tệp — xem [`doi_ten_tep`] (từ chối đích đã tồn tại).
    fn rename(&mut self, from: &str, to: &str) -> Result<(), String> {
        self.nhat_ky("rename", from);
        self.nhat_ky("rename →", to);
        doi_ten_tep(from, to)
    }

    /// **VD-270**: số **byte** của **tệp thường**. `symlink_metadata` ⇒ **không** đi theo symlink
    /// (symlink ⇒ `Err`); thư mục/thiếu/loại khác ⇒ `Err` dữ liệu. `int` là 64-bit; tệp lớn hơn
    /// `i64::MAX` bị từ chối thay vì tràn.
    fn size(&mut self, p: &str) -> Result<i64, String> {
        self.nhat_ky("size", p);
        let loi = |e: std::io::Error| format!("không lấy được kích thước `{p}`: {e}");
        let md = std::fs::symlink_metadata(p).map_err(loi)?;
        if md.file_type().is_symlink() {
            return Err(format!(
                "không lấy được kích thước `{p}`: là symlink (không đi theo)"
            ));
        }
        if !md.is_file() {
            return Err(format!(
                "không lấy được kích thước `{p}`: không phải tệp thường"
            ));
        }
        i64::try_from(md.len()).map_err(|_| format!("không lấy được kích thước `{p}`: quá lớn"))
    }

    /// **VD-270**: `true` ⇔ **tệp thường**, theo `symlink_metadata` (**không** đi theo symlink nên
    /// symlink ⇒ `false`). Thiếu `p` ⇒ `Ok(false)`; lỗi khác (vd không đủ quyền OS) ⇒ `Err`.
    fn is_file(&mut self, p: &str) -> Result<bool, String> {
        self.nhat_ky("is_file", p);
        match std::fs::symlink_metadata(p) {
            Ok(md) => Ok(md.is_file()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(e) => Err(format!("không xem được loại `{p}`: {e}")),
        }
    }

    /// **VD-270**: `true` ⇔ **thư mục**, theo `symlink_metadata` (symlink ⇒ `false`). Thiếu `p` ⇒
    /// `Ok(false)`; lỗi khác (vd không đủ quyền OS) ⇒ `Err`.
    fn is_dir(&mut self, p: &str) -> Result<bool, String> {
        self.nhat_ky("is_dir", p);
        match std::fs::symlink_metadata(p) {
            Ok(md) => Ok(md.is_dir()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(e) => Err(format!("không xem được loại `{p}`: {e}")),
        }
    }

    /// **VD-270 (fs-is-symlink)**: `true` ⇔ **chính** `p` là symlink, theo `symlink_metadata`
    /// (**không** đi theo link) — nên **dangling symlink** vẫn `true` (khác `exists`). Tệp/thư mục
    /// thường và `p` thiếu ⇒ `Ok(false)`; lỗi khác (vd `ENOTDIR`, không đủ quyền OS) ⇒ `Err`.
    fn is_symlink(&mut self, p: &str) -> Result<bool, String> {
        self.nhat_ky("is_symlink", p);
        match std::fs::symlink_metadata(p) {
            Ok(md) => Ok(md.file_type().is_symlink()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(e) => Err(format!("không xem được loại `{p}`: {e}")),
        }
    }

    /// **VD-270 (fs-read-link)**: đọc **đích** của symlink tại `p` bằng `std::fs::read_link` —
    /// **không** đi theo link, nên **dangling** vẫn trả đích. Đích trả **nguyên văn** (tương đối
    /// hay tuyệt đối đúng như lúc tạo). Tệp/thư mục thường, `p` thiếu, `ENOTDIR`, không đủ quyền
    /// OS ⇒ `Err` dữ liệu; đích không phải UTF-8 cũng ⇒ `Err` (không cắt cụt thay bằng lossy).
    fn read_link(&mut self, p: &str) -> Result<String, String> {
        self.nhat_ky("read_link", p);
        let dich =
            std::fs::read_link(p).map_err(|e| format!("không đọc được liên kết `{p}`: {e}"))?;
        dich.into_os_string()
            .into_string()
            .map_err(|_| format!("không đọc được liên kết `{p}`: đích không phải UTF-8 hợp lệ"))
    }

    /// **VD-270 (fs-canonicalize)**: đường dẫn **tuyệt đối đã phân giải** của `p` bằng
    /// `std::fs::canonicalize` — **chạm đĩa** và đọc `cwd` cho đường dẫn tương đối; symlink và
    /// `..` được phân giải theo trạng thái **hiện tại** của hệ tệp (khác `path.normalize` thuần
    /// từ vựng). `p` phải **tồn tại**: rỗng/thiếu/`ENOENT` (gồm cả **dangling**), lỗi quyền OS,
    /// hoặc kết quả không phải UTF-8 ⇒ `Err` dữ liệu (không cắt cụt thay bằng lossy).
    fn canonicalize(&mut self, p: &str) -> Result<String, String> {
        self.nhat_ky("canonicalize", p);
        let c = std::fs::canonicalize(p)
            .map_err(|e| format!("không phân giải được đường dẫn `{p}`: {e}"))?;
        c.into_os_string().into_string().map_err(|_| {
            format!("không phân giải được đường dẫn `{p}`: kết quả không phải UTF-8 hợp lệ")
        })
    }

    /// **VD-270 (fsbytes)**: đọc **toàn bộ** tệp thành byte thô (`std::fs::read`); thiếu
    /// tệp/thư mục/lỗi đọc ⇒ `Err` **dữ liệu**.
    fn read_bytes(&mut self, p: &str) -> Result<Vec<u8>, String> {
        self.nhat_ky("read_bytes", p);
        std::fs::read(p).map_err(|e| format!("không đọc được `{p}`: {e}"))
    }

    /// **VD-270 (fsbytes)**: ghi **toàn bộ** `data` (`std::fs::write`: tạo mới hoặc cắt rồi ghi);
    /// lỗi OS ⇒ `Err` **dữ liệu**.
    fn write_bytes(&mut self, p: &str, data: &[u8]) -> Result<(), String> {
        self.nhat_ky("write_bytes", p);
        std::fs::write(p, data).map_err(|e| format!("không ghi được `{p}`: {e}"))
    }

    /// **VD-268**: duyệt **đệ quy** dưới `p` bằng `read_dir` + `DirEntry::file_type()` (không đi
    /// theo symlink). Trả đường dẫn **tương đối** so với `p`, phân cách `/`; thư mục kèm `/`; DFS,
    /// mỗi cấp sắp theo byte của tên; mục ẩn vẫn gồm. Symlink bị **bỏ hoàn toàn** ⇒ không có vòng.
    ///
    /// `p` được theo nếu chính nó là symlink (quyền đã kiểm trên đích canonicalize); chỉ các mục
    /// **bên trong** mới bị bỏ. Lỗi đọc bất kỳ (thiếu/không phải thư mục/không đọc được thư mục
    /// con) ⇒ `Err` dữ liệu.
    fn walk(&mut self, p: &str) -> Result<Vec<String>, String> {
        self.nhat_ky("walk", p);
        let md =
            std::fs::metadata(p).map_err(|e| format!("không duyệt được thư mục `{p}`: {e}"))?;
        if !md.is_dir() {
            return Err(format!("không duyệt được `{p}`: không phải thư mục"));
        }
        let mut ra = Vec::new();
        duyet_de_quy(Path::new(p), "", &mut ra)?;
        Ok(ra)
    }

    fn exists(&mut self, p: &str) -> bool {
        self.nhat_ky("exists", p);
        std::path::Path::new(p).exists()
    }

    fn list_entries(&mut self, p: &str) -> Result<Vec<vis_core::host::MucFs>, String> {
        self.nhat_ky("list_entries", p);
        let rd = std::fs::read_dir(p).map_err(|e| format!("không đọc được thư mục `{p}`: {e}"))?;
        let mut muc: Vec<vis_core::host::MucFs> = Vec::new();
        for e in rd {
            let e = e.map_err(|x| format!("lỗi đọc mục trong `{p}`: {x}"))?;
            let md = e
                .metadata()
                .map_err(|x| format!("lỗi đọc metadata của mục trong `{p}`: {x}"))?;
            let modified_ns = md
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_nanos() as i64)
                .unwrap_or(0);
            let mode = {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    md.permissions().mode()
                }
                #[cfg(not(unix))]
                {
                    0
                }
            };
            muc.push(vis_core::host::MucFs {
                name: e.file_name().to_string_lossy().to_string(),
                size: md.len(),
                modified_ns,
                is_file: md.is_file(),
                mode,
            });
        }
        muc.sort_by(|a, b| a.name.as_bytes().cmp(b.name.as_bytes()));
        Ok(muc)
    }

    fn list(&mut self, p: &str) -> Result<Vec<String>, String> {
        self.nhat_ky("list", p);
        let rd = std::fs::read_dir(p).map_err(|e| format!("không đọc được thư mục `{p}`: {e}"))?;
        let mut ten: Vec<String> = Vec::new();
        for e in rd {
            let e = e.map_err(|x| format!("lỗi đọc mục trong `{p}`: {x}"))?;
            ten.push(e.file_name().to_string_lossy().to_string());
        }
        // **Sắp xếp** theo byte — ổn định giữa các hệ tệp; **không** đệ quy.
        ten.sort();
        Ok(ten)
    }
}

impl Runner {
    fn nhat_ky(&self, thao_tac: &str, p: &str) {
        if self.giai_thich {
            eprintln!("FS {thao_tac} {p}");
        }
    }
}
