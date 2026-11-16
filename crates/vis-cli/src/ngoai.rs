// SPDX-License-Identifier: MIT OR Apache-2.0
//! **Host chương trình ngoài** (ST-27) — chỉ ở `vis-cli`; `vis-interp` chỉ thấy `trait Ngoai`.
//!
//! Chạy bằng [`std::process::Command`] (mảng đối số, **không** `sh -c`), env kế thừa, timeout (mặc
//! định 10 s), cap đồng thời = 1 (VS0/VS1 chưa có job). `exit != 0` ⇒ `VS0501` kèm **stderr** của
//! tiến trình (không trộn stdout). Mỗi lần chạy in một dòng `EXEC …` khi `--explain-perms`.

use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use vis_diag::{COMMAND_CODES, RUNTIME_CODES};
use vis_interp::builtin::{DocQuaHandle, DuongDanHandle};
use vis_interp::{Abort, Trap, Value};
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

/// **ST-123 (VD-130c)**: lệnh `ten` khớp grant `g` ⇔ **đường dẫn tuyệt đối đã canonicalize bằng
/// nhau**. `g` đã được phân giải **một lần lúc khởi động** (`quyen::tao`); `ten` phân giải theo
/// **cùng** luật tại thời điểm chạy. **Không** còn so **basename** — bản sao cùng tên ở thư mục khác
/// (**ST-123** red) bị chặn; tên trần (`echo`) khớp `/bin/echo` qua `PATH`, không qua tên.
fn khop_lenh(ten: &str, g: &str) -> bool {
    match crate::quyen::chuan_lenh(ten) {
        Some(d) => d.as_path() == std::path::Path::new(g),
        None => false,
    }
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
    ) -> Result<vis_interp::builtin::KetQua, Abort> {
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
        if let Some(ds) = &self.allow_proc {
            if !ds.iter().any(|g| khop_lenh(ten, g)) {
                return Err(loi(
                    &COMMAND_CODES[3],
                    format!(
                        "[quyền] lệnh `{ten}` không nằm trong allowlist của `cap[Proc]` (đã cấp: {})",
                        ds.join(", ")
                    ),
                    span,
                ));
            }
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
        let mut cmd = Command::new(ten);
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
        Ok(vis_interp::builtin::KetQua {
            stdout: String::from_utf8_lossy(&out.stdout).to_string(),
            stderr: String::from_utf8_lossy(&out.stderr).to_string(),
            code: out.status.code().unwrap_or(-1),
        })
    }
}

impl vis_interp::builtin::Ngoai for Runner {
    fn chay(
        &mut self,
        ten: &str,
        args: &[String],
        raw: bool,
        stdin: &str,
        span: Span,
    ) -> vis_interp::R<Value> {
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
    ) -> Result<vis_interp::builtin::KetQua, Abort> {
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
    Value::List(dong)
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

/// **Host hệ thống tệp** (ST-33) — `std::fs` **chỉ** ở đây; `vis-interp`/`vis-vm` chỉ thấy trait.
///
/// Chốt an toàn (Agent 0 duyệt): **cho phép** đường dẫn tuyệt đối và `..` (không sandbox — consent
/// thật/`cap[Dir]` là **VS2**); mỗi lần truy cập in `FS …` ra stderr khi `--explain-perms`.
impl vis_interp::builtin::FsHost for Runner {
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

    fn exists(&mut self, p: &str) -> bool {
        self.nhat_ky("exists", p);
        std::path::Path::new(p).exists()
    }

    fn list_entries(&mut self, p: &str) -> Result<Vec<vis_interp::builtin::MucFs>, String> {
        self.nhat_ky("list_entries", p);
        let rd = std::fs::read_dir(p).map_err(|e| format!("không đọc được thư mục `{p}`: {e}"))?;
        let mut muc: Vec<vis_interp::builtin::MucFs> = Vec::new();
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
            muc.push(vis_interp::builtin::MucFs {
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
