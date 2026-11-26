// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-315d (VD-291 §3.4)** — parity `interp ↔ VM` cho **22 hàm `fs`** (+ symlink POSIX).
//!
//! **ST-316d (VD-292 §4.1)** — tách từ `misc_api_it/host_parity.rs` (cận trần R1); nội dung và
//! assertion giữ nguyên 100 %.

use std::path::Path;

use super::{duong, hai_duong_chuan_hoa, hai_duong_voi, tam};
fn dat_fixture(work: &Path) {
    let _ = std::fs::remove_dir_all(work);
    std::fs::create_dir_all(work.join("sub")).expect("mkdir sub");
    std::fs::write(work.join("a.txt"), "hi\n").expect("a.txt");
    std::fs::write(work.join("sub/b.txt"), "b").expect("b.txt");
    std::fs::write(work.join("bytes.bin"), [65u8, 66, 67]).expect("bytes.bin");
    #[cfg(unix)]
    std::os::unix::fs::symlink("a.txt", work.join("link.txt")).expect("symlink");
}

/// 20 hàm `fs` **đường thành công**: mỗi hàm in **giá trị thật** (bool/số/nội dung so sánh),
/// không còn nhãn `E`/`W` chung — F3 review.
pub(super) const SRC_FS: &str = r#"use std.fs
use std.path

fn main():
    let d = "__W__"
    print("exists-a=" + str(fs.exists(d + "/a.txt")))
    print("exists-x=" + str(fs.exists(d + "/missing")))
    match fs.is_file(d + "/a.txt"):
        Ok(b): print("is_file-a=" + str(b))
        Err(e): print("is_file-a loi=" + e.message)
    match fs.is_file(d + "/sub"):
        Ok(b): print("is_file-sub=" + str(b))
        Err(e): print("is_file-sub loi=" + e.message)
    match fs.is_dir(d + "/sub"):
        Ok(b): print("is_dir-sub=" + str(b))
        Err(e): print("is_dir-sub loi=" + e.message)
    match fs.is_dir(d + "/a.txt"):
        Ok(b): print("is_dir-a=" + str(b))
        Err(e): print("is_dir-a loi=" + e.message)
    match fs.size(d + "/a.txt"):
        Ok(n): print("size-a=" + str(n))
        Err(e): print("size-a loi=" + e.message)
    match fs.read(d + "/a.txt"):
        Ok(s): print("read-a=" + str(s == "hi\n"))
        Err(e): print("read-a loi=" + e.message)
    match fs.read_lines(d + "/a.txt"):
        Ok(xs): print("lines=" + str(xs == ["hi"]))
        Err(e): print("lines loi=" + e.message)
    match fs.read_bytes(d + "/bytes.bin"):
        Ok(xs): print("bytes=" + str(xs == [65, 66, 67]))
        Err(e): print("bytes loi=" + e.message)
    match fs.write(d + "/w.txt", "w\n"):
        Ok(_):
            print("write ok")
            match fs.read(d + "/w.txt"):
                Ok(s): print("write-noi-dung=" + str(s == "w\n"))
                Err(e): print("write-doc loi=" + e.message)
        Err(e): print("write loi=" + e.message)
    match fs.write_bytes(d + "/w.bin", [68, 69]):
        Ok(_):
            print("write_bytes ok")
            match fs.read_bytes(d + "/w.bin"):
                Ok(xs): print("write_bytes-noi-dung=" + str(xs == [68, 69]))
                Err(e): print("write_bytes-doc loi=" + e.message)
        Err(e): print("write_bytes loi=" + e.message)
    match fs.append(d + "/a.txt", "more\n"):
        Ok(_): print("append ok")
        Err(e): print("append loi=" + e.message)
    match fs.read(d + "/a.txt"):
        Ok(s): print("append-noi-dung=" + str(s == "hi\nmore\n"))
        Err(e): print("append-doc loi=" + e.message)
    match fs.size(d + "/a.txt"):
        Ok(n): print("size-sau-append=" + str(n))
        Err(e): print("size-sau-append loi=" + e.message)
    match fs.copy(d + "/a.txt", d + "/cp.txt"):
        Ok(_): print("copy ok")
        Err(e): print("copy loi=" + e.message)
    match fs.read(d + "/cp.txt"):
        Ok(s): print("copy-noi-dung=" + str(s == "hi\nmore\n"))
        Err(e): print("copy-doc loi=" + e.message)
    match fs.rename(d + "/cp.txt", d + "/mv.txt"):
        Ok(_): print("rename ok")
        Err(e): print("rename loi=" + e.message)
    print("exists-mv=" + str(fs.exists(d + "/mv.txt")))
    match fs.mkdir(d + "/newdir"):
        Ok(_): print("mkdir ok")
        Err(e): print("mkdir loi=" + e.message)
    match fs.mkdir_all(d + "/deep/x"):
        Ok(_): print("mkdir_all ok")
        Err(e): print("mkdir_all loi=" + e.message)
    match fs.is_dir(d + "/newdir"):
        Ok(b): print("is_dir-newdir=" + str(b))
        Err(e): print("is_dir-newdir loi=" + e.message)
    match fs.is_dir(d + "/deep/x"):
        Ok(b): print("is_dir-deep=" + str(b))
        Err(e): print("is_dir-deep loi=" + e.message)
    match fs.list(d + "/sub"):
        Ok(xs): print("list-sub=" + str(xs))
        Err(e): print("list-sub loi=" + e.message)
    match fs.walk(d):
        Ok(xs):
            print("walk-n=" + str(xs.len()))
            print("walk-co-mv=" + str(xs.any(x => path.basename(x) == "mv.txt")))
            print("walk-co-b=" + str(xs.any(x => path.basename(x) == "b.txt")))
        Err(e): print("walk loi=" + e.message)
    match fs.glob(d, "*.txt"):
        Ok(xs):
            print("glob-n=" + str(xs.len()))
            print("glob-co-a=" + str(xs.any(x => path.basename(x) == "a.txt")))
            print("glob-co-mv=" + str(xs.any(x => path.basename(x) == "mv.txt")))
            print("glob-co-link=" + str(xs.any(x => path.basename(x) == "link.txt")))
            print("glob-co-b=" + str(xs.any(x => path.basename(x) == "b.txt")))
        Err(e): print("glob loi=" + e.message)
    match fs.canonicalize(d + "/a.txt"):
        Ok(p): print("canon-a=" + path.basename(p))
        Err(e): print("canon-a loi=" + e.message)
    match fs.canonicalize(d + "/sub/../a.txt"):
        Ok(p): print("canon-dotdot=" + path.basename(p))
        Err(e): print("canon-dotdot loi=" + e.message)
    match fs.current_dir():
        Ok(p):
            print("cwd-khong-rong=" + str(p.len() > 0))
            match fs.canonicalize(p):
                Ok(cp):
                    match fs.canonicalize("."):
                        Ok(q): print("cwd-khop=" + str(cp == q))
                        Err(e): print("cwd-khop loi=" + e.message)
                Err(e): print("cwd-canon loi=" + e.message)
        Err(e): print("cwd loi=" + e.message)
    match fs.remove(d + "/mv.txt"):
        Ok(_): print("remove ok")
        Err(e): print("remove loi=" + e.message)
    print("exists-sau-remove=" + str(fs.exists(d + "/mv.txt")))
"#;

/// 17 hàm `fs` **nhánh lỗi** trên tệp/thư mục thiếu hoặc sai loại; in **nguyên văn**
/// `e.message` (F1 review). Đường dẫn tempdir được chuẩn hoá thành `<W>` trước khi so.
pub(super) const SRC_FS_LOI: &str = r#"use std.fs

fn main():
    let d = "__W__"
    match fs.read(d + "/khong-co.txt"):
        Ok(_): print("read=KHONG-DUOC-PHEP")
        Err(e): print("read=" + e.message)
    match fs.size(d + "/khong-co.txt"):
        Ok(_): print("size=KHONG-DUOC-PHEP")
        Err(e): print("size=" + e.message)
    match fs.remove(d + "/khong-co.txt"):
        Ok(_): print("remove=KHONG-DUOC-PHEP")
        Err(e): print("remove=" + e.message)
    match fs.mkdir(d + "/sub"):
        Ok(_): print("mkdir=KHONG-DUOC-PHEP")
        Err(e): print("mkdir=" + e.message)
    match fs.mkdir_all(d + "/a.txt/x"):
        Ok(_): print("mkdir_all=KHONG-DUOC-PHEP")
        Err(e): print("mkdir_all=" + e.message)
    match fs.copy(d + "/khong-co.txt", d + "/x"):
        Ok(_): print("copy=KHONG-DUOC-PHEP")
        Err(e): print("copy=" + e.message)
    match fs.rename(d + "/khong-co.txt", d + "/x"):
        Ok(_): print("rename=KHONG-DUOC-PHEP")
        Err(e): print("rename=" + e.message)
    match fs.read_link(d + "/a.txt"):
        Ok(_): print("read_link=KHONG-DUOC-PHEP")
        Err(e): print("read_link=" + e.message)
    match fs.canonicalize(d + "/khong-co.txt"):
        Ok(_): print("canonicalize=KHONG-DUOC-PHEP")
        Err(e): print("canonicalize=" + e.message)
    match fs.list(d + "/a.txt"):
        Ok(_): print("list=KHONG-DUOC-PHEP")
        Err(e): print("list=" + e.message)
    match fs.walk(d + "/a.txt"):
        Ok(_): print("walk=KHONG-DUOC-PHEP")
        Err(e): print("walk=" + e.message)
    match fs.glob(d + "/khong-co", "*.txt"):
        Ok(_): print("glob=KHONG-DUOC-PHEP")
        Err(e): print("glob=" + e.message)
    match fs.append(d + "/sub", "x"):
        Ok(_): print("append=KHONG-DUOC-PHEP")
        Err(e): print("append=" + e.message)
    match fs.write(d + "/sub", "x"):
        Ok(_): print("write=KHONG-DUOC-PHEP")
        Err(e): print("write=" + e.message)
    match fs.read_bytes(d + "/khong-co.bin"):
        Ok(_): print("read_bytes=KHONG-DUOC-PHEP")
        Err(e): print("read_bytes=" + e.message)
    match fs.read_lines(d + "/khong-co.txt"):
        Ok(_): print("read_lines=KHONG-DUOC-PHEP")
        Err(e): print("read_lines=" + e.message)
    match fs.write_bytes(d + "/sub", [1, 2]):
        Ok(_): print("write_bytes=KHONG-DUOC-PHEP")
        Err(e): print("write_bytes=" + e.message)
"#;

/// **ST-315d** — 20 hàm `fs` tất định trên tempdir, **assert giá trị chính xác** cho cả 20 hàm
/// (kể cả nội dung sau `write`/`write_bytes`/`append`/`copy`) — F3 review.
#[test]
fn fs_20_ham_parity_tren_tempdir() {
    let d = tam("fs");
    let work = d.join("work");
    let src = SRC_FS.replace("__W__", &duong(&work));
    let (out, err) = hai_duong_voi(&d, "fs", &src, || dat_fixture(&work));
    // Không có `link.txt` trên Windows ⇒ `walk` ít hơn đúng 1 mục.
    // `fs.walk` bỏ symlink trên **mọi** nền tảng ⇒ `link.txt` không được đếm; fixture còn
    // đúng 10 mục ở cả Unix lẫn Windows (R-Win2 phát hiện kỳ vọng 9 cũ sai).
    let walk_n = 10;
    let mong = format!(
        "exists-a=true\n\
         exists-x=false\n\
         is_file-a=true\n\
         is_file-sub=false\n\
         is_dir-sub=true\n\
         is_dir-a=false\n\
         size-a=3\n\
         read-a=true\n\
         lines=true\n\
         bytes=true\n\
         write ok\n\
         write-noi-dung=true\n\
         write_bytes ok\n\
         write_bytes-noi-dung=true\n\
         append ok\n\
         append-noi-dung=true\n\
         size-sau-append=8\n\
         copy ok\n\
         copy-noi-dung=true\n\
         rename ok\n\
         exists-mv=true\n\
         mkdir ok\n\
         mkdir_all ok\n\
         is_dir-newdir=true\n\
         is_dir-deep=true\n\
         list-sub=[b.txt]\n\
         walk-n={walk_n}\n\
         walk-co-mv=true\n\
         walk-co-b=true\n\
         glob-n=4\n\
         glob-co-a=true\n\
         glob-co-mv=true\n\
         glob-co-link=false\n\
         glob-co-b=true\n\
         canon-a=a.txt\n\
         canon-dotdot=a.txt\n\
         cwd-khong-rong=true\n\
         cwd-khop=true\n\
         remove ok\n\
         exists-sau-remove=false\n"
    );
    assert_eq!(out, mong, "fs đường thành công: {out}\nstderr: {err}");
}

/// **ST-315d** — **17 ca lỗi `fs`** (F1): in nguyên văn `e.message`; Unix ghim **toàn bộ**
/// thông điệp (đã chuẩn hoá `<W>`), nền tảng khác chỉ đòi 17 nhãn + thông điệp chứa `<W>`.
#[test]
fn fs_loi_moi_ham_parity() {
    let d = tam("fs-loi");
    let work = d.join("work");
    let w = duong(&work);
    let src = SRC_FS_LOI.replace("__W__", &w);
    let (out, err) = hai_duong_chuan_hoa(
        &d,
        "fs-loi",
        &src,
        || dat_fixture(&work),
        |s: &str| s.replace(&w, "<W>"),
    );
    let nhan: Vec<&str> = out
        .lines()
        .map(|l| l.split_once('=').map(|(a, _)| a).unwrap_or(""))
        .collect();
    assert_eq!(
        nhan,
        [
            "read",
            "size",
            "remove",
            "mkdir",
            "mkdir_all",
            "copy",
            "rename",
            "read_link",
            "canonicalize",
            "list",
            "walk",
            "glob",
            "append",
            "write",
            "read_bytes",
            "read_lines",
            "write_bytes",
        ],
        "thiếu ca lỗi fs: {out}\nstderr: {err}"
    );
    assert!(
        !out.contains("KHONG-DUOC-PHEP"),
        "có nhánh lỗi trả Ok: {out}"
    );
    #[cfg(unix)]
    assert_eq!(
        out,
        "read=không đọc được `<W>/khong-co.txt`: No such file or directory (os error 2)\n\
         size=không lấy được kích thước `<W>/khong-co.txt`: No such file or directory (os error 2)\n\
         remove=không xoá được `<W>/khong-co.txt`: No such file or directory (os error 2)\n\
         mkdir=không tạo được thư mục `<W>/sub`: File exists (os error 17)\n\
         mkdir_all=không tạo được thư mục `<W>/a.txt/x`: `<W>/a.txt` đã tồn tại và không phải thư mục\n\
         copy=không chép được `<W>/khong-co.txt`: No such file or directory (os error 2)\n\
         rename=không đổi tên được `<W>/khong-co.txt`: No such file or directory (os error 2)\n\
         read_link=không đọc được liên kết `<W>/a.txt`: Invalid argument (os error 22)\n\
         canonicalize=không phân giải được đường dẫn `<W>/khong-co.txt`: No such file or directory (os error 2)\n\
         list=không đọc được thư mục `<W>/a.txt`: Not a directory (os error 20)\n\
         walk=không duyệt được `<W>/a.txt`: không phải thư mục\n\
         glob=không duyệt được thư mục `<W>/khong-co`: No such file or directory (os error 2)\n\
         append=không ghi được `<W>/sub`: Is a directory (os error 21)\n\
         write=không ghi được `<W>/sub`: Is a directory (os error 21)\n\
         read_bytes=không đọc được `<W>/khong-co.bin`: No such file or directory (os error 2)\n\
         read_lines=không đọc được `<W>/khong-co.txt`: No such file or directory (os error 2)\n\
         write_bytes=không ghi được `<W>/sub`: Is a directory (os error 21)\n",
        "thông điệp lỗi fs (Unix): {out}\nstderr: {err}"
    );
    #[cfg(not(unix))]
    for dong in out.lines() {
        assert!(
            dong.contains("<W>"),
            "thông điệp lỗi phải nêu đường dẫn thật: {dong}"
        );
    }
}

/// `fs.is_symlink` + `fs.read_link` (POSIX; Windows không tạo symlink không đặc quyền).
pub(super) const SRC_FS_LINK: &str = r#"use std.fs
use std.path

fn main():
    let d = "__W__"
    match fs.is_symlink(d + "/link.txt"):
        Ok(b): print("is_symlink-link=" + str(b))
        Err(e): print("is_symlink-link loi=" + e.message)
    match fs.is_symlink(d + "/a.txt"):
        Ok(b): print("is_symlink-a=" + str(b))
        Err(e): print("is_symlink-a loi=" + e.message)
    match fs.read_link(d + "/link.txt"):
        Ok(p): print("read_link=" + path.basename(p))
        Err(e): print("read_link loi=" + e.message)
    match fs.read(d + "/link.txt"):
        Ok(s): print("read-qua-link=" + str(s == "hi\n"))
        Err(e): print("read-qua-link loi=" + e.message)
    match fs.canonicalize(d + "/link.txt"):
        Ok(p): print("canon-link=" + path.basename(p))
        Err(e): print("canon-link loi=" + e.message)
"#;

/// **ST-315d** — `fs.is_symlink` + `fs.read_link` (POSIX; Windows không tạo symlink không đặc quyền).
#[cfg(unix)]
#[test]
fn fs_symlink_parity() {
    let d = tam("fs-link");
    let work = d.join("work");
    let src = SRC_FS_LINK.replace("__W__", &duong(&work));
    let (out, err) = hai_duong_voi(&d, "fs-link", &src, || dat_fixture(&work));
    assert_eq!(
        out,
        "is_symlink-link=true\n\
         is_symlink-a=false\n\
         read_link=a.txt\n\
         read-qua-link=true\n\
         canon-link=a.txt\n",
        "symlink: {out}\nstderr: {err}"
    );
}
