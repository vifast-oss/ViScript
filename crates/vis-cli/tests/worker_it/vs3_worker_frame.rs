// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-292 (P4b)** — CLI `--workers`/`--max-budget`: exit/VS0510/VS0506.
//!
//! Phần API thư viện (10 test) đã sang `vis-conformance/tests/worker_api_it/vs3_worker_frame_lib.rs`
//! (ST-313c/VD-289).

fn cli_run(args: &[&str], src: &str, tag: &str) -> std::process::Output {
    let p = std::env::temp_dir().join(format!("st292-{}-{}.vis", std::process::id(), tag));
    std::fs::write(&p, src).expect("ghi tạm");
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_vis"))
        .arg("run")
        .args(args)
        .arg(&p)
        .output()
        .expect("chạy vis");
    let _ = std::fs::remove_file(&p);
    out
}

#[test]
fn p4b_cli_workers_0_exit_2() {
    let o = cli_run(&["--workers", "0"], "fn main():\n    print(1)\n", "w0");
    assert_eq!(o.status.code(), Some(2), "stderr: {:?}", o.stderr);
}

#[test]
fn p4b_cli_budget_workers_4_giu_vs0510() {
    let src = r#"async fn dem(n: int) -> int:
    var i = 0
    var s = 0
    while i < n:
        s = s + i
        i = i + 1
    s

fn main():
    let a = spawn dem(5000)
    let _ = await a
    print("xong")
"#;
    let w1 = cli_run(&["--max-budget", "5000"], src, "b1");
    let w4 = cli_run(&["--workers", "4", "--max-budget", "5000"], src, "b4");
    assert_eq!(w1.status.code(), Some(101), "W1 stderr: {:?}", w1.stderr);
    assert_eq!(w4.status.code(), Some(101), "W4 stderr: {:?}", w4.stderr);
    let s1 = String::from_utf8_lossy(&w1.stderr);
    let s4 = String::from_utf8_lossy(&w4.stderr);
    assert!(s1.contains("VS0510"), "W1: {s1}");
    assert!(s4.contains("VS0510"), "W4: {s4}");
    assert!(
        s1.contains("VS0506") && s4.contains("VS0506"),
        "hợp đồng task-error"
    );
}
