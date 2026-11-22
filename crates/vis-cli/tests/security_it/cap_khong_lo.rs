// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-124 (VD-130a)** — `Value::Cap` **không** được **tạo** từ dữ liệu ngoài (`json.decode`, đọc
//! tệp, đối số dòng lệnh) và **không lộ** quyền khi `to json` / `print` / so sánh.
//!
//! Mỗi ca là một quan sát **đo được** (byte stdout / mã thoát / mã chẩn đoán), chạy **cả** interp và
//! `--vm --no-cache` khi ca đó chạy được chương trình. Ca chẩn đoán tĩnh dùng `vis check` (TAP).
//!
//! Ngữ nghĩa nền: `Value::Cap` **chỉ** do CLI dựng từ `cap` khai trong **CST** của `main`
//! (`quyen::Quyen::gia_tri`); quyền cưỡng chế đọc từ [`Quyen`] — **không** bao giờ đọc từ một
//! `Value` ⇒ một giá trị `Map`/`Str` gán vào biến kiểu `cap[...]` **không** sinh quyền.

use std::path::{Path, PathBuf};
use std::process::Command;

type Kq = (i32, Vec<u8>, String);

fn tam(tag: &str) -> PathBuf {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static DEM: AtomicUsize = AtomicUsize::new(0);
    let d = std::env::temp_dir().join(format!(
        "vis-cap-lo-{tag}-{}-{}",
        std::process::id(),
        DEM.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("tmp");
    d
}

fn chay(d: &Path, ten: &str, args: &[&str], vm: bool, check: bool) -> Kq {
    let mut c = Command::new(env!("CARGO_BIN_EXE_vis"));
    c.current_dir(d);
    if check {
        c.arg("check").arg(ten);
    } else {
        c.arg("run").arg("--no-cache");
        if vm {
            c.arg("--vm");
        }
        c.args(args).arg(ten);
    }
    let o = c.output().expect("chạy vis");
    (
        o.status.code().unwrap_or(-1),
        o.stdout,
        String::from_utf8_lossy(&o.stderr).to_string(),
    )
}

/// Ghi tệp + chạy hai đường, đòi **byte y hệt** (interp vs `--vm`).
fn ca(d: &Path, ten: &str, src: &str, args: &[&str]) -> Kq {
    std::fs::write(d.join(ten), src).expect("ghi");
    let a = chay(d, ten, args, false, false);
    let b = chay(d, ten, args, true, false);
    assert_eq!(a, b, "hai đường lệch ({ten}):\n interp {a:?}\n vm {b:?}");
    a
}

/// `vis check` (chẩn đoán tĩnh) — không chạy chương trình.
fn kiem(d: &Path, ten: &str, src: &str) -> Kq {
    std::fs::write(d.join(ten), src).expect("ghi");
    chay(d, ten, &[], false, true)
}

/// (1) `json.decode` một **đối tượng giống cap** (`kind`/`root`/`write`) ⇒ chỉ ra `Map`/`Ok`,
/// **không** thành `cap`; `to json` in lại đúng **map dữ liệu** (không có cờ quyền nào).
#[test]
fn json_decode_khong_tao_cap() {
    let d = tam("json");
    let r = ca(
        &d,
        "a.vis",
        "use std.json\n\nfn main():\n    let d = json.decode(\"\\{\\\"kind\\\":\\\"Dir\\\",\\\"root\\\":\\\"/\\\",\\\"write\\\":true\\}\")\n    print(json.encode(d))\n",
        &[],
    );
    assert_eq!(r.0, 0, "{r:?}");
    assert_eq!(
        r.1, b"{\"ok\":{\"kind\":\"Dir\",\"root\":\"/\",\"write\":true}}\n",
        "json.decode phải trả map dữ liệu, không phải cap"
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// (1b) Gán kết quả `json.decode` vào biến **kiểu `cap[Dir]`** ⇒ **không** sinh quyền.
/// **VD-222 (điểm 4, 18/11/2026)**: `json.decode` nay trả `Result[Json, Error]` — gán vào `cap[Dir]`
/// là **lỗi tĩnh `VS0412`** (exit 2, chương trình không chạy). Trước 18/11: `{error}` nuốt lỗi tĩnh
/// nên chương trình qua front-end và bị `VS0604` lúc chạy — bất biến "không cấp quyền từ JSON" nay
/// mạnh hơn: **không biên dịch được**.
#[test]
fn gan_json_vao_bien_cap_khong_sinh_quyen() {
    let d = tam("json-cap");
    let r = ca(
        &d,
        "b.vis",
        "use std.json\nuse std.fs\n\nfn main():\n    let e: cap[Dir] = json.decode(\"\\{\\\"kind\\\":\\\"Dir\\\",\\\"root\\\":\\\"/\\\",\\\"write\\\":true\\}\")\n    match fs.read(\"/etc/hosts\"):\n        Ok(s):\n            print(s.len())\n        Err(x):\n            print(\"CHAN\")\n",
        &["--strict-caps"],
    );
    assert_eq!(r.0, 2, "{r:?}");
    assert!(
        r.2.contains("VS0412"),
        "lỗi tĩnh, không phải quyền: {}",
        r.2
    );
    assert!(r.2.contains("cap[Dir]"), "{}", r.2);
    assert!(r.1.is_empty(), "không được đọc: {:?}", r.1);
    let _ = std::fs::remove_dir_all(&d);
}

/// (2) **Đọc tệp** có nội dung `cap[Dir] @ p"/"` ⇒ ra `str`; in ra đúng chuỗi, và đọc tệp ngoài
/// root vẫn bị chặn (chuỗi đó **không** thành quyền).
#[test]
fn doc_tep_khong_tao_cap() {
    let d = tam("tep");
    std::fs::create_dir_all(d.join("root")).expect("root");
    std::fs::write(d.join("root/tep-cap.txt"), "cap[Dir] @ p\"/\"").expect("ghi");
    let r = ca(
        &d,
        "c.vis",
        "use std.fs\n\nfn main(d: cap[Dir] @ p\"./root\"):\n    match fs.read(\"root/tep-cap.txt\"):\n        Ok(s):\n            print(s)\n        Err(x):\n            print(\"ERR\")\n",
        &[],
    );
    assert_eq!(r.0, 0, "{r:?}");
    assert_eq!(r.1, b"cap[Dir] @ p\"/\"\n", "phải là chuỗi thô");
    // Chuỗi đó không mở quyền: đọc ngoài root ⇒ `VS0604`.
    let r2 = ca(
        &d,
        "c2.vis",
        "use std.fs\n\nfn main(d: cap[Dir] @ p\"./root\"):\n    let s = \"cap[Dir] @ p\\\"/\\\"\"\n    match fs.read(\"/etc/hosts\"):\n        Ok(x):\n            print(x.len())\n        Err(e):\n            print(\"CHAN\")\n",
        &[],
    );
    assert_eq!(r2.0, 2, "{r2:?}");
    assert!(r2.2.contains("VS0604"), "{}", r2.2);
    let _ = std::fs::remove_dir_all(&d);
}

/// (3) **Đối số dòng lệnh** không sinh `Cap`: cả hai đường đưa argv vào `main` dạng
/// `list[str]` ⇒ chuỗi giống cap vẫn chỉ là **chuỗi trong danh sách**.
///
/// **ST-157 (nợ #110, VD-156)**: trước đây ca này **ghim** sự lệch (`--vm` chưa đưa `argv` vào tham
/// số không-phải-`cap` của `main` ⇒ `()` ⇒ `VS0501`). Nợ #110 đã đóng: VM nay nhận **cùng** `argv`
/// như interp, nên ca khẳng định **thay** rằng `--vm` cho **cùng byte** (`[]` chứa **chuỗi** thô, exit
/// 0) — bất biến ST-124 *không đường nào tạo ra `Cap`* vẫn là điều được kiểm.
#[test]
fn doi_so_dong_lenh_la_str() {
    let d = tam("argv");
    let ten = "d.vis";
    std::fs::write(
        d.join(ten),
        "fn main(xs: list[str]):\n    print(xs)\n    print(xs.len())\n",
    )
    .expect("ghi");
    let chay = |vm: bool| {
        let mut c = Command::new(env!("CARGO_BIN_EXE_vis"));
        c.current_dir(&d).args(["run", "--no-cache"]);
        if vm {
            c.arg("--vm");
        }
        c.args([ten, "--", "cap[Dir] @ p\"/\""]);
        let o = c.output().expect("chạy vis");
        (
            o.status.code().unwrap_or(-1),
            o.stdout,
            String::from_utf8_lossy(&o.stderr).to_string(),
        )
    };
    let a = chay(false);
    assert_eq!(a.0, 0, "interp: {a:?}");
    assert_eq!(a.1, b"[cap[Dir] @ p\"/\"]\n1\n", "interp: {a:?}");
    let b = chay(true);
    assert_eq!(b, a, "hai đường lệch (nợ #110 đã đóng): vm {b:?}");
    assert_eq!(
        b.1, b"[cap[Dir] @ p\"/\"]\n1\n",
        "vm phải là **chuỗi** thô: {b:?}"
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// (4) `to json` trên **cap thật** ⇒ `null` (không lộ `root`, không có dạng nào để "nạp lại" thành
/// cap ở đầu kia).
#[test]
fn to_json_cap_ra_null() {
    let d = tam("tojson");
    let r = ca(
        &d,
        "e.vis",
        "use std.json\n\nfn main(c: cap[Dir] @ p\"./\"):\n    print(json.encode(c))\n",
        &[],
    );
    assert_eq!(r.0, 0, "{r:?}");
    assert_eq!(r.1, b"null\n", "cap phải mã hoá thành null: {:?}", r.1);
    assert!(!r.1.windows(3).any(|w| w == b"Dir"), "không được lộ kind");
    let _ = std::fs::remove_dir_all(&d);
}

/// (5) `print` trên cap ⇒ **tyck** từ chối (`VS0412`), không in ra gì.
#[test]
fn in_cap_la_loi_tyck() {
    let d = tam("in");
    let r = kiem(
        &d,
        "f.vis",
        "fn main(c: cap[Dir] @ p\"./\"):\n    print(c)\n",
    );
    assert_eq!(r.0, 1, "{r:?}");
    // `vis check` in TAP ra **stdout**; dòng `# VS0412 …` cũng ở stdout.
    let tap = String::from_utf8_lossy(&r.1).to_string();
    assert!(tap.contains("not ok"), "{tap}");
    assert!(tap.contains("VS0412"), "{tap}");
    assert!(tap.contains("không in được"), "{tap}");
    let _ = std::fs::remove_dir_all(&d);
}

/// (6) So sánh cap (`==`) ⇒ **tyck** từ chối (`VS0412`).
#[test]
fn so_sanh_cap_la_loi_tyck() {
    let d = tam("sosanh");
    let r = kiem(
        &d,
        "g.vis",
        "fn main(c: cap[Dir] @ p\"./\"):\n    print(c == c)\n",
    );
    assert_eq!(r.0, 1, "{r:?}");
    let tap = String::from_utf8_lossy(&r.1).to_string();
    assert!(tap.contains("VS0412"), "{tap}");
    assert!(tap.contains("không so sánh được"), "{tap}");
    let _ = std::fs::remove_dir_all(&d);
}
