// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-258 (review follow-up)** — giữ bằng chứng POSIX THẬT trên Unix.
//!
//! Bộ `test/host/suite.txt` và 4 fixture `test/sem/vish/ngoai-*` đã được đổi sang
//! `test-child` để chạy được trên Windows. Điều đó cần cho portability nhưng làm mất
//! bằng chứng tương thích với `/bin/echo`, `/usr/bin/grep`, `/bin/sh` thật trên Unix.
//! Test này chạy một tập nhỏ **lệnh thật** trên Unix, cả interp lẫn VM, để không có
//! "xanh nhờ mock" duy nhất.
//!
//! **VD-259** — bổ sung `/bin/sh -c "exit 3"` (thẳng + qua `complete`) và `/usr/bin/printf`
//! (có/không newline cuối): đúng các dạng suite host đang phủ bằng `test-child`, nay có
//! bằng chứng POSIX thật tương ứng. Test nằm trong `cargo test --workspace` (gate --fast) và
//! `bin/vis-host-suite` gọi tường minh trên Unix.
//!
//! Chỉ Unix: Windows dùng `test-child` (xem `host_conformance` + `test/host/suite.txt`).

#![cfg(unix)]

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn thu_muc_tam() -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("đồng hồ trước epoch")
        .as_nanos();
    let d = std::env::temp_dir().join(format!("vis-posix-real-{}-{stamp}", std::process::id()));
    std::fs::create_dir_all(&d).expect("tạo thư mục tạm");
    d
}

fn chay(tep: &Path, vm: bool) -> (i32, String, String) {
    let mut c = Command::new(env!("CARGO_BIN_EXE_vis"));
    c.arg("run").arg("--no-cache");
    if vm {
        c.arg("--vm");
    }
    c.arg(tep);
    let o = c.output().expect("chạy vis");
    (
        o.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&o.stdout).to_string(),
        String::from_utf8_lossy(&o.stderr).to_string(),
    )
}

/// Mỗi ca chạy cả interp và VM; không dựa vào `test-child`.
#[test]
fn lenh_posix_that_tren_unix() {
    let d = thu_muc_tam();

    let echo = d.join("echo.vish");
    std::fs::write(&echo, "/bin/echo hi\n").expect("ghi echo");
    let grep = d.join("grep.vish");
    std::fs::write(&grep, "/bin/echo x | /usr/bin/grep z\n").expect("ghi grep");
    let sh = d.join("sh.vish");
    std::fs::write(&sh, "/bin/sh -c \"exit 3\"\n").expect("ghi sh");
    let sh_complete = d.join("sh-complete.vish");
    std::fs::write(&sh_complete, "/bin/sh -c \"exit 3\" | complete\n").expect("ghi sh complete");
    let printf_abc = d.join("printf-abc.vish");
    std::fs::write(&printf_abc, "/usr/bin/printf abc\n").expect("ghi printf abc");
    let printf_lines = d.join("printf-lines.vish");
    std::fs::write(&printf_lines, "/usr/bin/printf \"a\\nb\\n\"\n").expect("ghi printf lines");

    for vm in [false, true] {
        let (ma, out, err) = chay(&echo, vm);
        assert_eq!(ma, 0, "echo ({vm:?}) exit: {err}");
        assert_eq!(out, "[hi]\n", "echo ({vm:?}) stdout");
        assert!(err.is_empty(), "echo ({vm:?}) stderr: {err}");

        let (ma, out, err) = chay(&grep, vm);
        assert_eq!(
            ma, 101,
            "grep ({vm:?}) exit; stdout={out:?}, stderr={err:?}"
        );
        assert!(
            err.contains("VS0501"),
            "grep ({vm:?}) phải có chẩn đoán VS0501; stderr={err:?}"
        );

        // `/bin/sh -c "exit 3"` thẳng: lệnh ngoài thất bại ⇒ VS0501 + exit 101.
        let (ma, out, err) = chay(&sh, vm);
        assert_eq!(ma, 101, "sh ({vm:?}) exit; stdout={out:?}, stderr={err:?}");
        assert!(out.is_empty(), "sh ({vm:?}) stdout: {out:?}");
        assert!(
            err.contains("VS0501") && err.contains("thoát mã 3"),
            "sh ({vm:?}) phải nêu VS0501 + mã 3; stderr={err:?}"
        );

        // Qua `complete`: mã thoát thật (3) được giữ, không biến thành lỗi VS0501.
        let (ma, out, err) = chay(&sh_complete, vm);
        assert_eq!(
            ma, 0,
            "sh complete ({vm:?}) exit; stdout={out:?}, stderr={err:?}"
        );
        assert_eq!(
            out, "stdout  []\nstderr  []\ncode    3\n",
            "sh complete ({vm:?}) stdout"
        );
        assert!(err.is_empty(), "sh complete ({vm:?}) stderr: {err}");

        // `/usr/bin/printf abc` (không newline cuối) ⇒ vẫn một dòng `[abc]`.
        let (ma, out, err) = chay(&printf_abc, vm);
        assert_eq!(
            ma, 0,
            "printf abc ({vm:?}) exit; stdout={out:?}, stderr={err:?}"
        );
        assert_eq!(out, "[abc]\n", "printf abc ({vm:?}) stdout");
        assert!(err.is_empty(), "printf abc ({vm:?}) stderr: {err}");

        // `printf "a\nb\n"` ⇒ hai dòng, in dạng danh sách như suite host.
        let (ma, out, err) = chay(&printf_lines, vm);
        assert_eq!(
            ma, 0,
            "printf lines ({vm:?}) exit; stdout={out:?}, stderr={err:?}"
        );
        assert_eq!(out, "[a, b]\n", "printf lines ({vm:?}) stdout");
        assert!(err.is_empty(), "printf lines ({vm:?}) stderr: {err}");
    }

    let _ = std::fs::remove_dir_all(&d);
}
