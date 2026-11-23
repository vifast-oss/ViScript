// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-304** — state machine soạn dòng: đa dòng/thụt lề, history, tìm mờ, Ctrl-C.

use crate::history::History;
use crate::key::Key;
use crate::render::render_cst;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Continue,
    /// Enter: gửi nguyên buffer cho caller.
    Submit(String),
    /// Ctrl-C: caller thoát 130.
    Cancel,
}

#[derive(Debug, Clone)]
pub struct Editor {
    buffer: String,
    cursor: usize,
    history: History,
    /// `None` = đang ở draft; `Some(i)` = đang xem entry i.
    hist_index: Option<usize>,
    draft: String,
    search: Option<String>,
}

impl Editor {
    pub fn new(history: History) -> Self {
        Self {
            buffer: String::new(),
            cursor: 0,
            history,
            hist_index: None,
            draft: String::new(),
            search: None,
        }
    }

    pub fn text(&self) -> &str {
        &self.buffer
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn history(&self) -> &History {
        &self.history
    }

    pub fn history_mut(&mut self) -> &mut History {
        &mut self.history
    }

    pub fn clear(&mut self) {
        self.buffer.clear();
        self.cursor = 0;
        self.hist_index = None;
        self.draft.clear();
        self.search = None;
    }

    pub fn render(&self) -> String {
        let body = render_cst(&self.buffer);
        match &self.search {
            Some(q) => format!("\x1b[2m[tìm: {q}]\x1b[0m {body}"),
            None => body,
        }
    }

    pub fn feed(&mut self, key: Key) -> Action {
        if let Some(q) = self.search.as_mut() {
            match key {
                Key::CtrlC | Key::Esc => self.search = None,
                Key::Backspace => {
                    q.pop();
                }
                Key::Enter => {
                    if let Some(first) = self.history.fuzzy(q).first() {
                        self.buffer = (*first).to_string();
                        self.cursor = self.buffer.len();
                    }
                    self.search = None;
                }
                Key::Char(c) => q.push(c),
                _ => {}
            }
            return Action::Continue;
        }
        match key {
            Key::CtrlC => return Action::Cancel,
            Key::CtrlR => {
                self.search = Some(String::new());
                return Action::Continue;
            }
            Key::Char(c) => {
                self.chen(&c.to_string());
                self.hist_index = None;
            }
            Key::Tab => {
                self.chen("    ");
                self.hist_index = None;
            }
            Key::Enter => return Action::Submit(self.buffer.clone()),
            Key::Newline => {
                let line_start = self.buffer[..self.cursor]
                    .rfind('\n')
                    .map(|i| i + 1)
                    .unwrap_or(0);
                let line = &self.buffer[line_start..self.cursor];
                let indent: String = line
                    .chars()
                    .take_while(|c| *c == ' ' || *c == '\t')
                    .collect();
                let extra = if line.trim_end().ends_with(':') {
                    "    "
                } else {
                    ""
                };
                self.chen(&format!("\n{indent}{extra}"));
                self.hist_index = None;
            }
            Key::Backspace => {
                if self.cursor > 0 {
                    let p = self.prev();
                    self.buffer.replace_range(p..self.cursor, "");
                    self.cursor = p;
                    self.hist_index = None;
                }
            }
            Key::Delete => {
                if self.cursor < self.buffer.len() {
                    let n = self.next();
                    self.buffer.replace_range(self.cursor..n, "");
                    self.hist_index = None;
                }
            }
            Key::Left => self.cursor = self.prev(),
            Key::Right => self.cursor = self.next(),
            Key::Home => {
                self.cursor = self.buffer[..self.cursor]
                    .rfind('\n')
                    .map(|i| i + 1)
                    .unwrap_or(0);
            }
            Key::End => {
                self.cursor = self.buffer[self.cursor..]
                    .find('\n')
                    .map(|i| self.cursor + i)
                    .unwrap_or(self.buffer.len());
            }
            Key::WordLeft => self.cursor = self.word_left(),
            Key::WordRight => self.cursor = self.word_right(),
            Key::Up => self.history_up(),
            Key::Down => self.history_down(),
            Key::Esc => {}
        }
        Action::Continue
    }

    fn chen(&mut self, s: &str) {
        self.buffer.insert_str(self.cursor, s);
        self.cursor += s.len();
    }

    fn prev(&self) -> usize {
        self.buffer[..self.cursor]
            .chars()
            .next_back()
            .map(|c| self.cursor - c.len_utf8())
            .unwrap_or(0)
    }

    fn next(&self) -> usize {
        self.buffer[self.cursor..]
            .chars()
            .next()
            .map(|c| self.cursor + c.len_utf8())
            .unwrap_or(self.buffer.len())
    }

    fn word_left(&self) -> usize {
        let mut i = self.cursor;
        while i > 0 {
            let p = self.buffer[..i].chars().next_back().unwrap();
            if p.is_whitespace() {
                i -= p.len_utf8();
            } else {
                break;
            }
        }
        while i > 0 {
            let p = self.buffer[..i].chars().next_back().unwrap();
            if p.is_alphanumeric() || p == '_' {
                i -= p.len_utf8();
            } else {
                break;
            }
        }
        i
    }

    fn word_right(&self) -> usize {
        let mut i = self.cursor;
        while i < self.buffer.len() {
            let c = self.buffer[i..].chars().next().unwrap();
            if c.is_alphanumeric() || c == '_' {
                i += c.len_utf8();
            } else {
                break;
            }
        }
        while i < self.buffer.len() {
            let c = self.buffer[i..].chars().next().unwrap();
            if c.is_whitespace() {
                i += c.len_utf8();
            } else {
                break;
            }
        }
        i
    }

    fn history_up(&mut self) {
        if self.history.entries().is_empty() {
            return;
        }
        let i = match self.hist_index {
            None => {
                self.draft = self.buffer.clone();
                self.history.entries().len() - 1
            }
            Some(0) => 0,
            Some(i) => i - 1,
        };
        self.hist_index = Some(i);
        self.buffer = self.history.entries()[i].clone();
        self.cursor = self.buffer.len();
    }

    fn history_down(&mut self) {
        let Some(i) = self.hist_index else {
            return;
        };
        if i + 1 < self.history.entries().len() {
            self.hist_index = Some(i + 1);
            self.buffer = self.history.entries()[i + 1].clone();
        } else {
            self.hist_index = None;
            self.buffer = self.draft.clone();
        }
        self.cursor = self.buffer.len();
    }
}
