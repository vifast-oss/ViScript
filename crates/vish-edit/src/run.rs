// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-304/ST-310b** — vòng đọc một dòng thật (raw mode + render); API cho ViShell/`vis`.
//!
//! **ST-310b**: không tự phát CSI; mọi điều khiển con trỏ + SGR đi qua [`vish_term::Terminal`].
//! `read_line_from` nhận reader/writer bất kỳ để test bằng **PTY giả** (chuỗi byte) trên 3 OS.

use std::io::{self, Read, Write};

use vish_term::{AnsiPosix, ColorMode, Terminal};

use crate::editor::{Action, Editor};
use crate::history::History;
use crate::key::read_key;
use crate::term::RawMode;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Line(String),
    /// Ctrl-C — caller nên thoát 130.
    Interrupted,
    /// EOF stdin.
    Eof,
}

/// Đọc một dòng (có thể nhiều dòng bằng Ctrl-J) từ terminal. `prompt` ghi ra stderr để stdout
/// của chương trình không bị lẫn; trả `Line` khi Enter.
pub fn read_line(prompt: &str, history: History) -> io::Result<Outcome> {
    let stdin = io::stdin();
    let mut input = stdin.lock();
    let stderr = io::stderr();
    let mut err = stderr.lock();
    let mut history = history;
    read_line_from(&mut input, &mut err, prompt, &mut history)
}

/// Như [`read_line`] nhưng đọc/ghi trên reader/writer cho trước và giữ **lịch sử qua nhiều dòng**
/// (`&mut History`). Dùng cho `vis repl` (TTY thật) và test PTY giả.
///
/// Raw mode vẫn gắn với stdin thật (`RawMode::enable` no-op khi stdin không phải terminal).
pub fn read_line_from<R: Read, W: Write>(
    input: &mut R,
    err: &mut W,
    prompt: &str,
    history: &mut History,
) -> io::Result<Outcome> {
    let _raw = RawMode::enable()?;
    // Terminal trên stderr: `vis` không được trộn prompt/echo vào stdout.
    let mut term = AnsiPosix::with_writer(err).with_color(ColorMode::Ansi16);
    let mut ed = Editor::new(history.clone());
    loop {
        term.clear_line()?;
        term.write(prompt)?;
        term.write(&ed.render())?;
        term.flush()?;
        let Some(key) = read_key(input)? else {
            return Ok(Outcome::Eof);
        };
        match ed.feed(key) {
            Action::Continue => {}
            Action::Cancel => {
                term.write_line("")?;
                term.flush()?;
                return Ok(Outcome::Interrupted);
            }
            Action::Submit(line) => {
                history.add(&line)?;
                ed.clear();
                term.write_line("")?;
                term.flush()?;
                return Ok(Outcome::Line(line));
            }
        }
    }
}
