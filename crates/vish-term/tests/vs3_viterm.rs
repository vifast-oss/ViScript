// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-308** — snapshot & hợp đồng `vish-term`: `Terminal`, `AnsiPosix`, `ViTermFramed`,
//! render bảng/bản ghi/chẩn đoán. `WinConsole` kiểm bằng `cfg(windows)` (Linux không biên dịch).

use std::io;

use vish_term::{
    Align, AnsiPosix, Color, ColorMode, Column, Diagnostic, Record, Severity, Style, Table,
    Terminal, ViTermFramed,
};

fn ansi(color: ColorMode) -> AnsiPosix<Vec<u8>> {
    AnsiPosix::with_writer(Vec::new()).with_color(color)
}

fn text(term: AnsiPosix<Vec<u8>>) -> String {
    String::from_utf8(term.into_inner()).expect("UTF-8")
}

fn bang() -> Table {
    let mut table = Table::new(vec![Column::new("name"), Column::right("size")]);
    table.push_row(vec!["a".to_string(), "6mb".to_string()]);
    table.push_row(vec!["bb".to_string(), "1mb".to_string()]);
    table
}

fn ban_ghi() -> Record {
    let mut record = Record::new();
    record.push("name", "a");
    record.push("size", "6mb");
    record
}

fn chan_doan() -> Diagnostic {
    Diagnostic::new(Severity::Error, "kiểu không khớp")
        .code("VS0412")
        .at("app.vis", 12, 18)
        .note("gợi ý: dùng int.parse(...)")
}

#[test]
fn bang_ansi16_snapshot() {
    let mut term = ansi(ColorMode::Ansi16);
    term.table(&bang()).expect("ghi bảng");
    assert_eq!(
        text(term),
        concat!(
            "\x1b[1mname\x1b[0m  \x1b[1msize\x1b[0m\n",
            "a      6mb\n",
            "bb     1mb\n",
        )
    );
}

#[test]
fn bang_khong_mau_snapshot() {
    let mut term = ansi(ColorMode::None);
    term.table(&bang()).expect("ghi bảng");
    assert_eq!(text(term), "name  size\na      6mb\nbb     1mb\n");
}

#[test]
fn bang_co_ke_snapshot() {
    let mut table = bang();
    table.rule = true;
    let mut term = ansi(ColorMode::None);
    term.table(&table).expect("ghi bảng");
    assert_eq!(
        text(term),
        "name  size\n----  ----\na      6mb\nbb     1mb\n"
    );
}

#[test]
fn ban_ghi_snapshot() {
    let mut term = ansi(ColorMode::Ansi16);
    term.record(&ban_ghi()).expect("ghi bản ghi");
    assert_eq!(
        text(term),
        concat!("\x1b[2mname\x1b[0m  a\n", "\x1b[2msize\x1b[0m  6mb\n",)
    );
}

#[test]
fn chan_doan_snapshot() {
    let mut term = ansi(ColorMode::Ansi16);
    term.diagnostic(&chan_doan()).expect("ghi chẩn đoán");
    assert_eq!(
        text(term),
        concat!(
            "\x1b[31merror\x1b[0m[\x1b[1mVS0412\x1b[0m]: kiểu không khớp\n",
            "  \x1b[2m-->\x1b[0m app.vis:12:18\n",
            "  = gợi ý: dùng int.parse(...)\n",
        )
    );
}

#[test]
fn chan_doan_khong_ma_va_ghi_chu() {
    let diag = Diagnostic::new(Severity::Warning, "cẩn thận");
    let mut term = ansi(ColorMode::None);
    term.diagnostic(&diag).expect("ghi chẩn đoán");
    assert_eq!(text(term), "warning: cẩn thận\n");
}

#[test]
fn do_rong_nfc_va_nfd_cung_cot() {
    let nfd = "e\u{0302}\u{0301}";
    let nfc = "\u{1ebf}";
    for s in [nfd, nfc] {
        let mut table = Table::new(vec![Column::new("ten"), Column::new("x")]);
        table.push_row(vec![s.to_string(), "y".to_string()]);
        let mut term = ansi(ColorMode::None);
        term.table(&table).expect("ghi bảng");
        assert_eq!(
            text(term),
            format!("ten  x\n{s}    y\n"),
            "NFC và NFD phải cho cùng độ rộng cột"
        );
    }
}

#[test]
fn bang_o_thieu_khong_de_khoang_trang_thua() {
    let mut table = Table::new(vec![Column::new("a"), Column::new("b")]);
    table.push_row(vec!["x".to_string()]);
    table.push_row(vec!["y".to_string(), "z".to_string(), "thừa".to_string()]);
    let mut term = ansi(ColorMode::None);
    term.table(&table).expect("ghi bảng");
    assert_eq!(text(term), "a  b\nx\ny  z\n");
}

#[test]
fn bang_rong_va_ban_ghi_rong_khong_ghi_gi() {
    let mut term = ansi(ColorMode::None);
    term.table(&Table::default()).expect("bảng rỗng");
    term.record(&Record::new()).expect("bản ghi rỗng");
    assert_eq!(text(term), "");
}

#[test]
fn sgr_16_256_truecolor() {
    let mut term = ansi(ColorMode::Ansi16);
    term.write_styled(
        Style::new().fg(Color::BrightRed).bg(Color::Blue).bold(),
        "X",
    )
    .expect("ghi");
    assert_eq!(text(term), "\x1b[1;91;44mX\x1b[0m");

    let mut term = ansi(ColorMode::Ansi256);
    term.write_styled(Style::new().fg(Color::Ansi256(196)), "X")
        .expect("ghi");
    assert_eq!(text(term), "\x1b[38;5;196mX\x1b[0m");

    let mut term = ansi(ColorMode::TrueColor);
    term.write_styled(Style::new().bg(Color::Rgb(1, 2, 3)), "X")
        .expect("ghi");
    assert_eq!(text(term), "\x1b[48;2;1;2;3mX\x1b[0m");
}

#[test]
fn sgr_ha_cap_ve_16_va_khong_mau() {
    let mut term = ansi(ColorMode::Ansi16);
    term.write_styled(Style::new().fg(Color::Rgb(255, 0, 0)), "X")
        .expect("ghi");
    assert_eq!(text(term), "\x1b[31mX\x1b[0m");

    let mut term = ansi(ColorMode::Ansi256);
    term.write_styled(Style::new().fg(Color::Rgb(255, 0, 0)), "X")
        .expect("ghi");
    assert_eq!(text(term), "\x1b[38;5;196mX\x1b[0m");

    let mut term = ansi(ColorMode::None);
    term.write_styled(Style::new().fg(Color::Red).bold(), "X")
        .expect("ghi");
    assert_eq!(text(term), "X");
}

#[test]
fn con_tro_ansi_va_tat_con_tro() {
    let mut term = ansi(ColorMode::None);
    term.clear_line().expect("xoá dòng");
    term.move_up(2).expect("lên");
    term.show_cursor(false).expect("ẩn");
    term.show_cursor(true).expect("hiện");
    assert_eq!(text(term), "\r\x1b[2K\x1b[2A\x1b[?25l\x1b[?25h");

    let mut term = ansi(ColorMode::None).with_cursor(false);
    term.clear_line().expect("xoá dòng");
    term.move_up(2).expect("lên");
    term.show_cursor(false).expect("ẩn");
    assert_eq!(text(term), "");
}

#[test]
fn viterm_khung_snapshot() {
    let mut term = ViTermFramed::new(Vec::new());
    term.send_hello().expect("hello");
    term.send_text("xin chào").expect("text");
    term.send_styled(Style::new().fg(Color::Red).bold().underline(), "quan trọng")
        .expect("styled");
    term.send_table(&bang()).expect("table");
    term.send_record(&ban_ghi()).expect("record");
    term.send_diagnostic(&chan_doan()).expect("diag");
    term.send_clear_line().expect("clear");
    term.send_move_up(2).expect("cursor");
    term.send_show_cursor(true).expect("cursor");
    term.send_flush().expect("flush");
    term.send_bye().expect("bye");
    assert_eq!(
        String::from_utf8(term.into_inner()).expect("UTF-8"),
        concat!(
            r#"{"v":1,"t":"hello","proto":"vish-term/1","caps":{"color":"truecolor","unicode":true,"cursor":true,"structured":true}}"#,
            "\n",
            r#"{"v":1,"t":"text","s":"xin chào"}"#,
            "\n",
            r#"{"v":1,"t":"styled","style":{"fg":"red","bg":null,"attrs":["bold","underline"]},"s":"quan trọng"}"#,
            "\n",
            r#"{"v":1,"t":"table","cols":[{"title":"name","align":"left"},{"title":"size","align":"right"}],"rows":[["a","6mb"],["bb","1mb"]]}"#,
            "\n",
            r#"{"v":1,"t":"record","fields":[["name","a"],["size","6mb"]]}"#,
            "\n",
            r#"{"v":1,"t":"diag","severity":"error","code":"VS0412","message":"kiểu không khớp","location":{"path":"app.vis","line":12,"col":18},"notes":["gợi ý: dùng int.parse(...)"]}"#,
            "\n",
            r#"{"v":1,"t":"clear","what":"line"}"#,
            "\n",
            r#"{"v":1,"t":"cursor","op":"up","n":2}"#,
            "\n",
            r#"{"v":1,"t":"cursor","op":"show"}"#,
            "\n",
            r#"{"v":1,"t":"flush"}"#,
            "\n",
            r#"{"v":1,"t":"bye"}"#,
            "\n",
        )
    );
}

#[test]
fn viterm_hook_cau_truc_di_nguyen_khung() {
    let mut term = ViTermFramed::new(Vec::new());
    {
        let dyn_term: &mut dyn Terminal = &mut term;
        dyn_term.table(&bang()).expect("table");
        dyn_term.record(&ban_ghi()).expect("record");
        dyn_term.diagnostic(&chan_doan()).expect("diag");
    }
    let out = String::from_utf8(term.into_inner()).expect("UTF-8");
    assert!(out.contains("\"t\":\"table\""));
    assert!(out.contains("\"t\":\"record\""));
    assert!(out.contains("\"t\":\"diag\""));
    assert!(!out.contains("\"t\":\"text\""));
}

#[test]
fn viterm_escaping_json() {
    let mut term = ViTermFramed::new(Vec::new());
    term.send_text("a\"b\\c\nd\te\u{1}").expect("text");
    assert_eq!(
        String::from_utf8(term.into_inner()).expect("UTF-8"),
        concat!(r#"{"v":1,"t":"text","s":"a\"b\\c\nd\te\u0001"}"#, "\n")
    );
}

#[test]
fn viterm_write_line_giu_xuong_dong() {
    let mut term = ViTermFramed::new(Vec::new());
    term.write_line("a\nb").expect("dòng");
    assert_eq!(
        String::from_utf8(term.into_inner()).expect("UTF-8"),
        concat!(r#"{"v":1,"t":"text","s":"a\nb\n"}"#, "\n")
    );
}

#[test]
fn terminal_object_safe_va_flush() {
    fn ghi(term: &mut dyn Terminal) -> io::Result<()> {
        term.write("x")?;
        term.flush()
    }
    let mut term = ansi(ColorMode::None);
    ghi(&mut term).expect("ghi qua dyn");
    assert_eq!(text(term), "x");
}

#[cfg(windows)]
#[test]
fn winconsole_van_ban_thuan_crlf() {
    use vish_term::WinConsole;
    let mut term = WinConsole::with_writer(Vec::new());
    term.write_line("a").expect("dòng");
    term.write_styled(Style::new().fg(Color::Red), "b")
        .expect("có kiểu");
    term.flush().expect("flush");
    assert_eq!(
        String::from_utf8(term.into_inner()).expect("UTF-8"),
        "a\r\nb"
    );
}

#[cfg(windows)]
#[test]
fn winconsole_bat_ansi() {
    use vish_term::WinConsole;
    let mut term = WinConsole::with_writer(Vec::new()).with_ansi(true);
    term.write_styled(Style::new().fg(Color::Red), "b")
        .expect("có kiểu");
    assert_eq!(
        String::from_utf8(term.into_inner()).expect("UTF-8"),
        "\x1b[31mb\x1b[0m"
    );
}

#[test]
fn display_width_nfc_va_scalar() {
    assert_eq!(vish_term::display_width("Việt"), 4);
    assert_eq!(vish_term::display_width("e\u{0302}\u{0301}"), 1);
    assert_eq!(vish_term::display_width("😀"), 1);
}

#[test]
fn align_va_column_api() {
    let left = Column::new("a");
    let right = Column::right("b");
    assert_eq!(left.align, Align::Left);
    assert_eq!(right.align, Align::Right);
    let table = Table::new(vec![left, right]).with_rule(true);
    assert!(table.rule);
    assert_eq!(table.len(), 0);
    assert!(!table.is_empty());
}
