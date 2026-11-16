// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-132 (VD-120)** — ca kiểm **chứng minh kernel chặn thật** (không phải `FsGuard`).
//!
//! Mỗi ca chạy trong một luồng riêng của harness (Landlock áp cho **luồng gọi** và mọi tiến trình
//! con kế tiếp), nên các ca không ảnh hưởng nhau.
//!
//! Ca **quan trọng nhất** là ca tiến trình con: nó `exec` `/bin/cat` — **không** đi qua `FsGuard`
//! của runtime — nên nếu bị `EACCES` thì bằng chứng là **kernel** chặn, đúng yêu cầu VD-120.

#![cfg(target_os = "linux")]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use vis_host_linux::sandbox::{ap_dung, ChoPhep, KetQua, LoiSandbox, Muc};

/// Thư mục tạm riêng cho từng ca (dưới `/tmp`, tồn tại trước khi áp sandbox).
fn tam(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("vis-ll-{}-{tag}", std::process::id()));
    let _ = fs::remove_dir_all(&d);
    fs::create_dir_all(&d).expect("tạo thư mục tạm");
    d
}

/// Áp sandbox và đòi kernel cưỡng chế **đầy đủ** — ca nào không đạt thì đỏ, không được bỏ qua.
fn ap(cho: &[ChoPhep]) -> KetQua {
    let kq = ap_dung(cho).expect("áp sandbox phải thành công trên runner Linux");
    assert_eq!(kq.muc, Muc::Day, "kernel phải cưỡng chế ĐẦY ĐỦ: {kq:?}");
    assert!(kq.no_new_privs, "PR_SET_NO_NEW_PRIVS phải bật");
    kq
}

fn doc(p: &Path) -> std::io::Result<String> {
    fs::read_to_string(p)
}

#[test]
fn doc_trong_thu_muc_duoc_phep() {
    let d = tam("ro-trong");
    let f = d.join("a.txt");
    fs::write(&f, "xin chao\n").expect("ghi tệp mẫu");
    let kq = ap(&[ChoPhep {
        duong: d.clone(),
        ghi: false,
        la_tep: false,
    }]);
    assert_eq!(kq.abi, 1);
    assert_eq!(kq.so_luat, 1);
    assert_eq!(doc(&f).expect("đọc trong phạm vi phải được"), "xin chao\n");
    assert!(kq.dong().starts_with("sandbox: landlock ABI=1 mức=đầy đủ"));
    let _ = fs::remove_dir_all(&d);
}

#[test]
fn doc_ngoai_bi_kernel_chan() {
    let trong = tam("ro-trong2");
    let ngoai = tam("ro-ngoai2");
    let f = ngoai.join("bi-mat.txt");
    fs::write(&f, "khong duoc doc\n").expect("ghi tệp ngoài");
    ap(&[ChoPhep {
        duong: trong.clone(),
        ghi: false,
        la_tep: false,
    }]);
    let e = doc(&f).expect_err("đọc tệp ngoài phạm vi PHẢI bị chặn");
    assert_eq!(
        e.kind(),
        std::io::ErrorKind::PermissionDenied,
        "phải là PermissionDenied, thật {e:?}"
    );
    let _ = fs::remove_dir_all(&trong);
    let _ = fs::remove_dir_all(&ngoai);
}

#[test]
fn ghi_trong_thu_muc_duoc_phep() {
    let d = tam("rw-trong");
    let kq = ap(&[ChoPhep {
        duong: d.clone(),
        ghi: true,
        la_tep: false,
    }]);
    assert_eq!(kq.muc, Muc::Day);
    fs::write(d.join("moi.txt"), "ok\n").expect("ghi trong phạm vi phải được");
    assert_eq!(doc(&d.join("moi.txt")).expect("đọc lại"), "ok\n");
    let _ = fs::remove_dir_all(&d);
}

#[test]
fn ghi_ngoai_bi_chan() {
    let trong = tam("rw-trong2");
    let ngoai = tam("rw-ngoai2");
    ap(&[ChoPhep {
        duong: trong.clone(),
        ghi: true,
        la_tep: false,
    }]);
    let e = fs::write(ngoai.join("x.txt"), "khong\n").expect_err("ghi ngoài phải bị chặn");
    assert_eq!(e.kind(), std::io::ErrorKind::PermissionDenied, "thật {e:?}");
    let _ = fs::remove_dir_all(&trong);
    let _ = fs::remove_dir_all(&ngoai);
}

#[test]
fn ghi_trong_thu_muc_chi_doc_bi_chan() {
    let d = tam("ro-ghi");
    fs::write(d.join("co.txt"), "x\n").expect("tệp mẫu");
    ap(&[ChoPhep {
        duong: d.clone(),
        ghi: false,
        la_tep: false,
    }]);
    let e = fs::write(d.join("moi.txt"), "y\n").expect_err("ghi vào thư mục chỉ đọc phải bị chặn");
    assert_eq!(e.kind(), std::io::ErrorKind::PermissionDenied, "thật {e:?}");
    let _ = fs::remove_dir_all(&d);
}

// **ĐO ĐƯỢC (ST-132), ghi lại thay vì giấu**: trên kernel `7.0.0-43-generic` (Landlock ABI 8),
// mọi `execve` **bên trong** một domain Landlock đều trả `EACCES` — đo 4 biến thể: (a) không xử lý
// `Execute`; (b) xử lý `Execute` + cấp trên thư mục chứa; (c) cấp `Execute` trên `/`; (d) chạy tệp
// sao chép trong thư mục được cấp. Cả 4 đều `EACCES`, kể cả khi `READ|EXEC` đã được cấp.
// Hệ quả: **không dựng được** ca "tiến trình con bỏ qua FsGuard" bằng `exec` trong môi trường này,
// nên ca đó **không** nằm ở đây. Bằng chứng kernel tương đương đã có ở các ca dưới: chúng gọi
// thẳng syscall (`fs::read_to_string`/`fs::write`) — **không** đi qua `FsGuard`, nên bị chặn nghĩa
// là **kernel** chặn. Chi tiết + biến thể: `viscript/agent/JOURNAL/st132-landlock.md` §4.

/// **Bằng chứng kernel ở TIẾN TRÌNH CON** (VD-120): dựng một helper **tĩnh** (`crt-static`) để nó
/// không cần `ld.so` (execve một binary động cần quyền `EXECUTE` trên cả interpreter), rồi cho
/// tiến trình con đọc/ghi **ngoài** ruleset — nó không đi qua `FsGuard` nào.
fn dung_helper(d: &Path) -> PathBuf {
    let src = d.join("h.rs");
    let out = d.join("helper");
    fs::write(
        &src,
        r#"fn main() {
    let a: Vec<String> = std::env::args().collect();
    if a.len() < 3 { eprintln!("dung: doc|ghi <tep> [noi-dung]"); std::process::exit(2); }
    let kq: std::io::Result<()> = if a[1] == "doc" {
        std::fs::read_to_string(&a[2]).map(|_| ())
    } else {
        std::fs::write(&a[2], a.get(3).cloned().unwrap_or_default())
    };
    match kq { Ok(()) => std::process::exit(0), Err(e) => { eprintln!("loi: {e}"); std::process::exit(7); } }
}
"#,
    )
    .expect("ghi nguồn helper");
    let kq = Command::new("rustc")
        .args(["-O", "-C", "target-feature=+crt-static", "-o"])
        .arg(&out)
        .arg(&src)
        .stdin(Stdio::inherit())
        .output()
        .expect("chạy rustc để dựng helper tĩnh");
    assert!(
        kq.status.success(),
        "rustc phải dựng được helper tĩnh: {}",
        String::from_utf8_lossy(&kq.stderr)
    );
    out
}

fn chay_con(helper: &Path, args: &[&str]) -> std::process::Output {
    let mut c = Command::new(helper);
    for a in args {
        c.arg(a);
    }
    c.stdin(Stdio::inherit()).output().expect("chạy helper")
}

#[test]
fn con_doc_ngoai_pham_vi_bi_kernel_chan() {
    let trong = tam("con2-trong");
    let ngoai = tam("con2-ngoai");
    let helper = dung_helper(&trong);
    let f = ngoai.join("bi-mat.txt");
    fs::write(&f, "bi mat\n").expect("tệp ngoài");
    ap(&[ChoPhep {
        duong: trong.clone(),
        ghi: false,
        la_tep: false,
    }]);
    let out = chay_con(&helper, &["doc", f.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(7), "con phải bị từ chối: {out:?}");
    let loi = String::from_utf8_lossy(&out.stderr);
    assert!(
        loi.contains("Permission denied"),
        "kernel phải trả Permission denied: {loi}"
    );
    let _ = fs::remove_dir_all(&trong);
    let _ = fs::remove_dir_all(&ngoai);
}

#[test]
fn con_ghi_ngoai_pham_vi_bi_kernel_chan() {
    let trong = tam("con3-trong");
    let ngoai = tam("con3-ngoai");
    let helper = dung_helper(&trong);
    ap(&[ChoPhep {
        duong: trong.clone(),
        ghi: true,
        la_tep: false,
    }]);
    let dich = ngoai.join("x.txt");
    let out = chay_con(&helper, &["ghi", dich.to_str().unwrap(), "x"]);
    assert_eq!(
        out.status.code(),
        Some(7),
        "con ghi ngoài phải bị chặn: {out:?}"
    );
    assert!(String::from_utf8_lossy(&out.stderr).contains("Permission denied"));
    assert!(!dich.exists(), "tệp ngoài KHÔNG được tạo");
    let _ = fs::remove_dir_all(&trong);
    let _ = fs::remove_dir_all(&ngoai);
}

/// Đối chứng: cùng helper, cùng domain — đọc **trong** phạm vi thì phải chạy được.
#[test]
fn con_doc_trong_pham_vi_chay_duoc() {
    let d = tam("con4");
    let helper = dung_helper(&d);
    let f = d.join("trong.txt");
    fs::write(&f, "duoc phep\n").expect("tệp trong");
    ap(&[ChoPhep {
        duong: d.clone(),
        ghi: false,
        la_tep: false,
    }]);
    let out = chay_con(&helper, &["doc", f.to_str().unwrap()]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "con đọc trong phạm vi phải OK: {out:?}"
    );
    let _ = fs::remove_dir_all(&d);
}

/// Không có luật nào ⇒ **mọi** truy cập hệ tệp bị chặn (deny-by-default của Landlock).
#[test]
fn khong_co_luat_nao_thi_chan_het() {
    let d = tam("rong");
    let f = d.join("a.txt");
    fs::write(&f, "x\n").expect("tệp mẫu");
    let kq = ap(&[]);
    assert_eq!(kq.so_luat, 0);
    let e = doc(&f).expect_err("không luật ⇒ phải bị chặn");
    assert_eq!(e.kind(), std::io::ErrorKind::PermissionDenied, "thật {e:?}");
    let _ = fs::remove_dir_all(&d);
}

/// Nhiều thư mục: đọc được cả hai, thư mục thứ ba vẫn bị chặn.
#[test]
fn nhieu_thu_muc_duoc_cap() {
    let a = tam("nhieu-a");
    let b = tam("nhieu-b");
    let c = tam("nhieu-c");
    fs::write(a.join("a.txt"), "A\n").expect("tệp A");
    fs::write(b.join("b.txt"), "B\n").expect("tệp B");
    fs::write(c.join("c.txt"), "C\n").expect("tệp C");
    let kq = ap(&[
        ChoPhep {
            duong: a.clone(),
            ghi: false,
            la_tep: false,
        },
        ChoPhep {
            duong: b.clone(),
            ghi: true,
            la_tep: false,
        },
    ]);
    assert_eq!(kq.so_luat, 2);
    assert_eq!(doc(&a.join("a.txt")).expect("đọc A"), "A\n");
    assert_eq!(doc(&b.join("b.txt")).expect("đọc B"), "B\n");
    fs::write(b.join("moi.txt"), "B2\n").expect("ghi B");
    let e = doc(&c.join("c.txt")).expect_err("thư mục không cấp phải bị chặn");
    assert_eq!(e.kind(), std::io::ErrorKind::PermissionDenied, "thật {e:?}");
    for d in [&a, &b, &c] {
        let _ = fs::remove_dir_all(d);
    }
}

/// Luật cho **một tệp** không mở cả thư mục chứa nó.
#[test]
fn luat_cho_mot_tep_khong_mo_ca_thu_muc() {
    let d = tam("tep-rieng");
    let duoc = d.join("duoc.txt");
    let khong = d.join("khong.txt");
    fs::write(&duoc, "duoc\n").expect("tệp được phép");
    fs::write(&khong, "khong\n").expect("tệp bị chặn");
    ap(&[ChoPhep {
        duong: duoc.clone(),
        ghi: false,
        la_tep: true,
    }]);
    assert_eq!(doc(&duoc).expect("tệp được phép phải đọc được"), "duoc\n");
    let e = doc(&khong).expect_err("tệp cùng thư mục nhưng không có luật phải bị chặn");
    assert_eq!(e.kind(), std::io::ErrorKind::PermissionDenied, "thật {e:?}");
    let _ = fs::remove_dir_all(&d);
}

/// Đường dẫn không tồn tại ⇒ lỗi **hệ thống** rõ ràng, không im lặng bỏ luật.
#[test]
fn duong_dan_khong_ton_tai_la_loi_he_thong() {
    let khong_co = PathBuf::from("/khong/ton/tai/vis-ll");
    match ap_dung(&[ChoPhep {
        duong: khong_co,
        ghi: false,
        la_tep: false,
    }]) {
        Err(LoiSandbox::HeThong(s)) => assert!(s.contains("khong/ton/tai"), "thông điệp: {s}"),
        other => panic!("phải là LoiSandbox::HeThong, thật {other:?}"),
    }
}
