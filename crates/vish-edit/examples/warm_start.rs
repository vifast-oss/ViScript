// SPDX-License-Identifier: MIT OR Apache-2.0
//! Đo khởi động ấm API `vish-edit` (release): tạo Editor + render CST + history rỗng.
//! Chạy: `cargo run --release -p vish-edit --example warm_start`.

use std::time::Instant;

fn main() {
    let t = Instant::now();
    let mut ed = vish_edit::Editor::new(vish_edit::History::default());
    for c in "fn main():".chars() {
        ed.feed(vish_edit::Key::Char(c));
    }
    ed.feed(vish_edit::Key::Newline);
    let _ = ed.render();
    let _ = ed.feed(vish_edit::Key::CtrlC);
    println!("{}", t.elapsed().as_micros());
}
