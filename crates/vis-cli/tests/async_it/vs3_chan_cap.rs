// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VS3-2a** — CLI `async` + `chan(1)` tự chọn frame, interp ↔ `--vm`.
//!
//! Phần API thư viện (5 test) đã sang
//! `vis-conformance/tests/async_api_it/vs3_chan_cap_lib.rs` (ST-313c/VD-289).

use std::process::Command;

#[test]
fn cli_async_cap1_chay_frame_hai_duong() {
    let d = std::env::temp_dir().join(format!("vis-chan-cap-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("tmp");
    let p = d.join("cap1.vis");
    std::fs::write(
        &p,
        "# SPDX-License-Identifier: MIT OR Apache-2.0\n".to_string()
            + "async fn san(c: Chan[int]) -> int:\n    c.send(1)\n    0\n\nfn main():\n    let c = chan[int](1)\n    let t = spawn san(c)\n    print(c.recv())\n",
    )
    .expect("ghi");
    let mut kq = Vec::new();
    for vm in [false, true] {
        let mut c = Command::new(env!("CARGO_BIN_EXE_vis"));
        c.current_dir(&d).arg("run");
        if vm {
            c.arg("--vm").arg("--no-cache");
        }
        let o = c.arg(p.to_str().unwrap()).output().expect("vis run");
        kq.push((
            o.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&o.stdout).to_string(),
            String::from_utf8_lossy(&o.stderr).to_string(),
        ));
    }
    assert_eq!(kq[0].0, 0, "interp: {}", kq[0].2);
    assert_eq!(kq[0].1, "Some(1)\n");
    assert_eq!(kq[0], kq[1], "interp↔VM CLI lệch");
    let _ = std::fs::remove_dir_all(&d);
}
