// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-304b** — completion theo kiểu + job control; PTY giả Linux/macOS.

use std::sync::atomic::{AtomicU32, Ordering};

use vish_edit::completion::{CandidateKind, CompletionContext};
use vish_edit::job::{parse_command, JobCommand, JobControl};

static DEM: AtomicU32 = AtomicU32::new(0);

fn tam(tag: &str) -> std::path::PathBuf {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vish-304b-{tag}-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn completion_truong_record_theo_bien() {
    let mut c = CompletionContext::new();
    c.add_record("User", &["name", "age"]);
    c.add_var("u", "User");
    let got = c.complete("u.na", 4);
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].text, "name");
    assert_eq!(got[0].kind, CandidateKind::RecordField);
    assert!(c.complete("u.zz", 4).is_empty());
}

#[test]
fn completion_bien_the_enum() {
    let mut c = CompletionContext::new();
    c.add_enum("Color", &["Red", "Green"]);
    c.add_var("mau", "Color");
    let got = c.complete("mau.Gr", 6);
    assert_eq!(got[0].text, "Green");
    assert_eq!(got[0].kind, CandidateKind::EnumVariant);
    assert_eq!(c.complete("Color.Re", 8)[0].text, "Red");
}

#[test]
fn completion_co_cmd_va_async_cmd() {
    let src = "cmd build(--release: bool, --target: str):\n    pass\n\nasync cmd test(--filter: str, --repeat: int):\n    pass\n";
    let c = CompletionContext::from_source(src);
    let got = c.complete("build --re", 10);
    assert_eq!(got[0].text, "--release");
    assert_eq!(got[0].kind, CandidateKind::CmdFlag);
    let got = c.complete("test --fi", 9);
    assert_eq!(got[0].text, "--filter");
}

#[test]
fn completion_path() {
    let d = tam("path");
    std::fs::write(d.join("a.vis"), "").unwrap();
    std::fs::create_dir(d.join("sub")).unwrap();
    let c = CompletionContext::new().with_cwd(&d);
    let got = c.complete("./", 2);
    assert!(got
        .iter()
        .any(|x| x.text == "./a.vis" && x.kind == CandidateKind::Path));
    assert!(got.iter().any(|x| x.text == "./sub/"));
    let got = c.complete("./a", 3);
    assert_eq!(got[0].text, "./a.vis");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn completion_from_source_record_enum_cmd() {
    let src = "type User:\n    name: str\n    age: int\n\nenum Color:\n    Red\n    Green\n\ncmd build(--release: bool):\n    pass\n";
    let mut c = CompletionContext::from_source(src);
    c.add_var("u", "User");
    assert_eq!(c.complete("u.na", 4)[0].text, "name");
    assert_eq!(c.complete("Color.Gr", 8)[0].text, "Green");
    assert_eq!(c.complete("build --re", 10)[0].text, "--release");
}

#[test]
fn completion_bien_tu_param_ham() {
    let src = "type User:\n    name: str\n\nfn f(u: User):\n    pass\n";
    let c = CompletionContext::from_source(src);
    assert_eq!(c.complete("u.na", 4)[0].text, "name");
}

#[test]
fn parse_job_commands() {
    assert_eq!(
        parse_command("sleep 1 &"),
        Some(JobCommand::Spawn("sleep 1".to_string()))
    );
    assert_eq!(parse_command("jobs"), Some(JobCommand::List));
    assert_eq!(parse_command("fg 2"), Some(JobCommand::Fg(Some(2))));
    assert_eq!(parse_command("bg"), Some(JobCommand::Bg(None)));
    assert_eq!(
        parse_command("kill -TERM 3"),
        Some(JobCommand::Kill(Some(3)))
    );
    assert_eq!(parse_command("echo x"), None);
}

#[cfg(unix)]
#[test]
fn job_control_spawn_jobs_kill() {
    let mut j = JobControl::new();
    j.handle("exec sleep 5 &").expect("spawn");
    assert_eq!(j.jobs().len(), 1);
    assert_eq!(j.jobs()[0].cmd, "exec sleep 5");
    j.handle("kill 1").expect("kill");
    assert!(j.jobs().is_empty());
}

#[cfg(unix)]
#[test]
fn job_control_bg_va_fg() {
    let mut j = JobControl::new();
    j.handle("exec sleep 0.05 &").expect("spawn");
    assert!(j.handle("bg 1").expect("bg")[0].contains("(bg)"));
    assert!(j.handle("fg 1").expect("fg")[0].contains("(fg)"));
    assert!(j.jobs().is_empty());
}

#[cfg(windows)]
#[test]
fn windows_parse_job_va_completion_cfg() {
    assert_eq!(parse_command("jobs"), Some(JobCommand::List));
    let c = CompletionContext::new();
    assert!(c.complete("x", 1).is_empty());
}
