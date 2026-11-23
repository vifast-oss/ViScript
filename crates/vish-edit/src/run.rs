// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-304** — vòng đọc một dòng thật (raw mode + render); API cho ViShell/`vis`.

use std::io::{self, Write};

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
    let _raw = RawMode::enable()?;
    let stdin = io::stdin();
    let mut input = stdin.lock();
    let mut ed = Editor::new(history);
    loop {
        eprint!("\r\x1b[2K{prompt}{}", ed.render());
        io::stderr().flush()?;
        let Some(key) = read_key(&mut input)? else {
            return Ok(Outcome::Eof);
        };
        match ed.feed(key) {
            Action::Continue => {}
            Action::Cancel => {
                eprintln!();
                return Ok(Outcome::Interrupted);
            }
            Action::Submit(line) => {
                ed.history_mut().add(&line)?;
                ed.clear();
                eprintln!();
                return Ok(Outcome::Line(line));
            }
        }
    }
}
