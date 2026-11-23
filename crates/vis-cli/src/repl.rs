// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-303/ST-310b** — `vis repl`: phiên nhỏ dùng `vis-loader` + `vis-interp`.
//!
//! Mỗi dòng nhập được thử trên **bản nháp** gồm các dòng đã commit; chỉ commit khi nạp +
//! kiểm kiểu + chạy sạch. Lượt lỗi in chẩn đoán ra stderr và **không** làm mất trạng thái trước.
//! Lệnh phiên: `:help`, `:type <expr>`, `:reset`, `:quit`.
//!
//! **ST-310b (VD-285)**:
//! - TTY thật ⇒ mặc định dùng `vish-edit` (`read_line_from`) để soạn dòng; Ctrl-C thoát **130**.
//! - Non-TTY ⇒ đọc dòng từ ống, **không** prompt/không ANSI — pipe sạch cho script.
//! - `--format viterm` ⇒ mọi đầu ra trên stdout là khung JSON Lines `vish-term/1`
//!   (`vish_term::ViTermFramed`), kể cả chẩn đoán (`diag`), mở bằng `hello` và đóng bằng `bye`.
//! - Mọi ghi văn bản đi qua `vish-term`; module này không tự phát SGR/CSI.

use std::io::{self, BufRead, IsTerminal, Write};
use std::process::ExitCode;

use vis_loader::modules;
use vis_syntax::Diagnostic;
use vis_types::{check, Type};
use vish_edit::{read_line_from, History, Outcome};
use vish_term::{AnsiPosix, ColorMode, Severity, Terminal, ViTermFramed};

/// Định dạng đầu ra của `vis repl`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Format {
    /// Văn bản thuần: stdout = kết quả chương trình, stderr = chẩn đoán/prompt.
    #[default]
    Text,
    /// Khung JSON Lines `vish-term/1` trên stdout.
    ViTerm,
}

impl Format {
    /// Phân tích `--format text|viterm`.
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "text" => Some(Self::Text),
            "viterm" => Some(Self::ViTerm),
            _ => None,
        }
    }
}

/// Một chẩn đoán đã tách khỏi nguồn (`vis-syntax`) để render theo `Format`.
#[derive(Debug, Clone)]
struct ChanDoan {
    path: Option<String>,
    line: u32,
    col: u32,
    code: String,
    message: String,
}

/// Việc cần ghi ra đầu ra, do phiên sinh ra (không tự ghi) để test được bằng ống nhớ.
#[derive(Debug, Clone)]
enum SuKien {
    /// Ghi nguyên văn stdout.
    Ra(String),
    /// Ghi một dòng stdout.
    RaDong(String),
    /// Chẩn đoán (text ⇒ stderr; viterm ⇒ khung `diag` stdout).
    ChanDoan(ChanDoan),
    /// Lỗi dùng/phiên — luôn ra stderr, prefix `vis repl:`.
    Loi(String),
}

#[derive(Default)]
struct Phien {
    committed: String,
    committed_out: String,
}

fn help() -> Vec<&'static str> {
    vec![
        ":help          trợ giúp",
        ":type <expr>   in kiểu tĩnh của biểu thức",
        ":reset         xoá trạng thái phiên",
        ":quit          thoát",
    ]
}

/// Nạp + kiểm kiểu bản nháp; lỗi trả về dưới dạng sự kiện (không tự in).
fn nap_va_kiem(src: &str) -> Result<(modules::Loaded, vis_types::CheckOutput), Vec<SuKien>> {
    let loaded = match modules::load_full_stdin(src, true, false) {
        Ok(l) => l,
        Err(e) => return Err(vec![SuKien::Loi(format!("vis repl: {e}"))]),
    };
    if !loaded.diags.is_empty() {
        return Err(loaded
            .diags
            .iter()
            .map(|(f, d)| {
                SuKien::ChanDoan(ChanDoan {
                    path: Some(f.clone()),
                    line: d.line,
                    col: d.col,
                    code: d.code.short.to_string(),
                    message: d.message.clone(),
                })
            })
            .collect());
    }
    let checked = check(&loaded.hir, &loaded.symbols, &loaded.types);
    if !checked.is_clean() {
        return Err(checked
            .diags
            .iter()
            .map(|d| {
                SuKien::ChanDoan(ChanDoan {
                    path: None,
                    line: d.line,
                    col: d.col,
                    code: d.code.short.to_string(),
                    message: d.message.clone(),
                })
            })
            .collect());
    }
    Ok((loaded, checked))
}

fn su_kien_loi_runtime(d: &Diagnostic) -> SuKien {
    SuKien::ChanDoan(ChanDoan {
        path: None,
        line: d.line,
        col: d.col,
        code: d.code.short.to_string(),
        message: d.message.clone(),
    })
}

impl Phien {
    /// Xử lý một dòng. Trả `(sự kiện, còn tiếp?)`.
    fn xu_ly(&mut self, line: &str) -> (Vec<SuKien>, bool) {
        if let Some(cmd) = line.trim().strip_prefix(':') {
            let (ten, rest) = cmd.split_once(char::is_whitespace).unwrap_or((cmd, ""));
            match ten {
                "quit" | "q" => return (Vec::new(), false),
                "help" | "h" => {
                    return (
                        help()
                            .into_iter()
                            .map(|s| SuKien::RaDong(s.to_string()))
                            .collect(),
                        true,
                    )
                }
                "reset" => {
                    self.committed.clear();
                    self.committed_out.clear();
                    return (
                        vec![SuKien::RaDong("(đã xoá trạng thái)".to_string())],
                        true,
                    );
                }
                "type" | "t" => {
                    let expr = rest.trim();
                    if expr.is_empty() {
                        return (
                            vec![SuKien::Loi("vis repl: :type cần biểu thức".to_string())],
                            true,
                        );
                    }
                    let src = format!("{}let __repl_type = {expr}\n", self.committed);
                    return match nap_va_kiem(&src) {
                        Ok((_l, c)) => {
                            let ty = c.types_of_expr.last().cloned().unwrap_or(Type::Unit);
                            (vec![SuKien::RaDong(ty.to_string())], true)
                        }
                        Err(ev) => (ev, true),
                    };
                }
                _ => {
                    return (
                        vec![SuKien::Loi(format!(
                            "vis repl: lệnh lạ `:{ten}` (dùng :help)"
                        ))],
                        true,
                    )
                }
            }
        }
        if line.trim().is_empty() {
            return (Vec::new(), true);
        }
        let candidate = format!("{}{line}\n", self.committed);
        let (loaded, _checked) = match nap_va_kiem(&candidate) {
            Ok(x) => x,
            Err(ev) => return (ev, true),
        };
        let out = vis_interp::run(&candidate, &loaded.hir, &loaded.symbols, &loaded.types, &[]);
        if !out.is_clean() || out.exit != 0 {
            if out.is_clean() {
                return (
                    vec![SuKien::Loi(format!(
                        "vis repl: chương trình thoát {}",
                        out.exit
                    ))],
                    true,
                );
            }
            return (
                out.diagnostics.iter().map(su_kien_loi_runtime).collect(),
                true,
            );
        }
        let extra = out
            .stdout
            .strip_prefix(self.committed_out.as_str())
            .unwrap_or(out.stdout.as_str());
        let mut ev = Vec::new();
        if !extra.is_empty() {
            ev.push(SuKien::Ra(extra.to_string()));
        }
        self.committed = candidate;
        self.committed_out = out.stdout;
        (ev, true)
    }
}

/// Đầu ra có thể chuyển giữa văn bản và khung ViTerm mà **không** rải nhánh ở nơi gọi.
enum Ra<W: Write> {
    Text(AnsiPosix<W>),
    ViTerm(ViTermFramed<W>),
}

impl<W: Write> Ra<W> {
    fn khoi_tao(out: W, format: Format) -> io::Result<Self> {
        match format {
            Format::Text => Ok(Self::Text(
                AnsiPosix::with_writer(out).with_color(ColorMode::None),
            )),
            Format::ViTerm => {
                let mut term = ViTermFramed::new(out);
                term.send_hello()?;
                Ok(Self::ViTerm(term))
            }
        }
    }

    fn ghi(&mut self, s: &str) -> io::Result<()> {
        match self {
            Self::Text(term) => term.write(s),
            Self::ViTerm(term) => term.send_text(s),
        }
    }

    fn ghi_dong(&mut self, s: &str) -> io::Result<()> {
        match self {
            Self::Text(term) => term.write_line(s),
            Self::ViTerm(term) => term.send_text(&format!("{s}\n")),
        }
    }

    fn chan_doan(&mut self, d: &vish_term::Diagnostic) -> io::Result<()> {
        match self {
            Self::Text(term) => term.diagnostic(d),
            Self::ViTerm(term) => term.send_diagnostic(d),
        }
    }

    fn ket_thuc(&mut self) -> io::Result<()> {
        match self {
            Self::Text(term) => term.flush(),
            Self::ViTerm(term) => term.send_bye(),
        }
    }
}

/// Ghi chẩn đoán: text giữ **đúng khuôn cũ** (`mã: dòng: thông điệp`) trên stderr; ViTerm gửi
/// khung `diag` trên stdout.
fn phat_chan_doan<W: Write, E: Write>(
    ra: &mut Ra<W>,
    err: &mut E,
    format: Format,
    d: ChanDoan,
) -> io::Result<()> {
    match format {
        Format::Text => match d.path {
            Some(p) => writeln!(err, "{}:{}:{}: {}: {}", p, d.line, d.col, d.code, d.message),
            None => writeln!(err, "{}: {}: {}", d.code, d.line, d.message),
        },
        Format::ViTerm => {
            let mut td = vish_term::Diagnostic::new(Severity::Error, d.message).code(d.code);
            if let Some(p) = d.path {
                td = td.at(p, d.line, d.col);
            }
            ra.chan_doan(&td)
        }
    }
}

fn ghi_su_kien<W: Write, E: Write>(
    ra: &mut Ra<W>,
    err: &mut E,
    format: Format,
    ev: SuKien,
) -> io::Result<()> {
    match ev {
        SuKien::Ra(s) => ra.ghi(&s),
        SuKien::RaDong(s) => ra.ghi_dong(&s),
        SuKien::ChanDoan(d) => phat_chan_doan(ra, err, format, d),
        SuKien::Loi(s) => writeln!(err, "{s}"),
    }
}

/// Chạy phiên trên reader/writer cho trước (test PTY giả + `run`).
///
/// `interactive = true` dùng `vish-edit` (prompt ra `err`); `false` đọc dòng từ ống. Trả mã thoát
/// (`0`, `130` khi Ctrl-C). Lỗi I/O trả `Err` để caller in và thoát 2.
pub fn run_with<R: BufRead, W: Write, E: Write>(
    input: &mut R,
    out: &mut W,
    err: &mut E,
    interactive: bool,
    format: Format,
    mut history: History,
) -> io::Result<i32> {
    let mut ra = Ra::khoi_tao(&mut *out, format)?;
    if interactive {
        writeln!(err, "vis repl — gõ :help để xem lệnh")?;
    }
    let mut phien = Phien::default();
    let mut ma = 0;
    loop {
        let line = if interactive {
            match read_line_from(input, err, "vis> ", &mut history)? {
                Outcome::Line(l) => l,
                Outcome::Interrupted => {
                    ma = 130;
                    break;
                }
                Outcome::Eof => break,
            }
        } else {
            let mut buf = String::new();
            if input.read_line(&mut buf)? == 0 {
                break;
            }
            buf.trim_end_matches(['\r', '\n']).to_string()
        };
        let (su_kien, tiep) = phien.xu_ly(&line);
        for ev in su_kien {
            ghi_su_kien(&mut ra, err, format, ev)?;
        }
        if !tiep {
            break;
        }
    }
    ra.ket_thuc()?;
    Ok(ma)
}

fn usage() -> ExitCode {
    eprintln!("dùng: vis repl [--format text|viterm]");
    ExitCode::from(2)
}

pub fn run(args: &[String]) -> ExitCode {
    let mut format = Format::Text;
    let mut i = 0;
    while i < args.len() {
        let arg = args[i].as_str();
        if arg == "--format" {
            match args.get(i + 1) {
                Some(v) => match Format::parse(v) {
                    Some(f) => {
                        format = f;
                        i += 1;
                    }
                    None => {
                        eprintln!("vis repl: --format không hỗ trợ `{v}` (text|viterm)");
                        return ExitCode::from(2);
                    }
                },
                None => {
                    eprintln!("vis repl: --format cần giá trị text|viterm");
                    return ExitCode::from(2);
                }
            }
        } else if let Some(v) = arg.strip_prefix("--format=") {
            match Format::parse(v) {
                Some(f) => format = f,
                None => {
                    eprintln!("vis repl: --format không hỗ trợ `{v}` (text|viterm)");
                    return ExitCode::from(2);
                }
            }
        } else {
            eprintln!("vis repl: không hỗ trợ đối số `{arg}`");
            return usage();
        }
        i += 1;
    }

    let stdin = io::stdin();
    let interactive = stdin.is_terminal();
    let mut input = stdin.lock();
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let stderr = io::stderr();
    let mut err = stderr.lock();
    match run_with(
        &mut input,
        &mut out,
        &mut err,
        interactive,
        format,
        History::default(),
    ) {
        Ok(ma) => ExitCode::from(ma as u8),
        Err(e) => {
            let _ = writeln!(err, "vis repl: {e}");
            ExitCode::from(2)
        }
    }
}
