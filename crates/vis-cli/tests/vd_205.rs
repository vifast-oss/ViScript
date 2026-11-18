// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-205 / R4a (VD-187, VD-222 uỷ quyền A0-proxy 18/11/2026)** — `Error` bản ghi dựng sẵn +
//! `Json` enum dựng sẵn + trait `Error: Show` (ba phương thức) + `json.decode → Result[Json, Error]`.
//!
//! Mọi ca chạy **cả hai đường** (interp + VM) — byte stdout/stderr và exit code khớp nhau.

use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

static DEM: AtomicU32 = AtomicU32::new(0);

fn tam(tag: &str) -> PathBuf {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-vd205-{tag}-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("tạo thư mục tạm");
    d
}

fn chay_ca_hai(tag: &str, src: &str) -> [(u32, String, String); 2] {
    let d = tam(tag);
    let p = d.join(format!("{tag}.vis"));
    std::fs::write(
        &p,
        format!("# SPDX-License-Identifier: MIT OR Apache-2.0\n{src}"),
    )
    .expect("ghi tệp tạm");
    let mo = |extra: &[&str]| {
        let mut c = Command::new(env!("CARGO_BIN_EXE_vis"));
        c.arg("run").arg("--no-cache");
        c.args(extra);
        c.arg(&p);
        let o = c.output().expect("chạy vis run");
        (
            o.status.code().unwrap_or(-1) as u32,
            String::from_utf8_lossy(&o.stdout).to_string(),
            String::from_utf8_lossy(&o.stderr).to_string(),
        )
    };
    let a = mo(&[]);
    let b = mo(&["--vm"]);
    let _ = std::fs::remove_dir_all(&d);
    [a, b]
}

/// (1) `json.decode` trả `Result[Json, Error]`; `match` trên `Arr` in đúng.
#[test]
fn decode_ok_match_json_arr() {
    let [a, b] = chay_ca_hai(
        "arr",
        "use std.json\nfn main() -> ():\n    match json.decode(\"[1, \\\"x\\\"]\"):\n        Ok(j):\n            match j:\n                Arr(_): print(\"arr\")\n                _: print(\"other\")\n        Err(_):\n            print(\"E\")\n",
    );
    assert_eq!(a.0, 0, "interp: {}", a.2);
    assert_eq!(a.1, "arr\n");
    assert_eq!(b.0, 0, "vm: {}", b.2);
    assert_eq!(b.1, "arr\n");
    assert_eq!(a.1, b.1, "hai đường phải cùng byte");
}

/// (2) `Null`/`Bool`/`Num` phân biệt trên `match`, cả hai đường.
#[test]
fn decode_null_bool_num() {
    for (json, mong) in [("null", "null"), ("true", "bool"), ("1.5", "num")] {
        let src = format!(
            "use std.json\nfn main() -> ():\n    match json.decode(\"{json}\"):\n        Ok(j):\n            match j:\n                Null: print(\"null\")\n                Bool(_): print(\"bool\")\n                Num(_): print(\"num\")\n                Str(_): print(\"str\")\n                Arr(_): print(\"arr\")\n                Obj(_): print(\"obj\")\n        Err(e):\n            print(e.message)\n"
        );
        let [a, b] = chay_ca_hai("scalar", &src);
        assert_eq!(a.1, format!("{mong}\n"), "interp json={json}: {a:?}");
        assert_eq!(b.1, a.1, "vm khác byte: {b:?}");
    }
}

/// (3) `json.decode` lỗi: `e.message` chứa byte offset; `print(e)` chỉ in `{message}`;
/// `e.source`/`e.span` là `None`.
#[test]
fn decode_err_ban_ghi_error() {
    let [a, b] = chay_ca_hai(
        "err",
        "use std.json\nfn main() -> ():\n    match json.decode(\"nul\"):\n        Ok(_):\n            print(\"o\")\n        Err(e):\n            print(e)\n            print(e.message)\n            match e.source:\n                Some(_): print(\"s\")\n                None: print(\"none-source\")\n            match e.span:\n                Some(_): print(\"sp\")\n                None: print(\"none-span\")\n",
    );
    for r in [&a, &b] {
        assert_eq!(r.0, 0, "{r:?}");
        assert!(r.1.contains("JSON không hợp lệ ở byte 0"), "{r:?}");
        assert!(r.1.contains("none-source"), "{r:?}");
        assert!(r.1.contains("none-span"), "{r:?}");
        // `print(e)` in **thanh gọn** = chỉ message — không bọc `Error(...)` ở dòng đó.
        assert!(!r.1.contains("Error("), "bản ghi in gọn: {r:?}");
    }
    assert_eq!(a.1, b.1, "hai đường phải cùng byte");
}

/// (4) Bản ghi `Error` dựng sẵn có **đủ 3 trường** (`message`, `source: Error?`, `span: str?`).
#[test]
fn error_record_co_du_truong() {
    let [a, b] = chay_ca_hai(
        "fields",
        "use std.json\nfn main() -> ():\n    match json.decode(\"nul\"):\n        Ok(_): print(\"o\")\n        Err(e):\n            let m: str = e.message\n            let _s: Error? = e.source\n            let _p: str? = e.span\n            print(\"fields-ok\")\n",
    );
    for r in [&a, &b] {
        assert_eq!(r.0, 0, "{r:?}");
        assert_eq!(r.1, "fields-ok\n");
    }
}

/// (5) `Json`, `Error`, `AnyError` là **tên dự trữ** — người dùng không khai lại được (`VS0302`).
#[test]
fn ten_dun_san_du_bi_vs0302() {
    for src in [
        "type Json:\n    x: int\nfn main() -> ():\n    print(1)\n",
        "type Error:\n    x: int\nfn main() -> ():\n    print(1)\n",
        "enum Json:\n    A\nfn main() -> ():\n    print(1)\n",
        "type AnyError:\n    x: int\nfn main() -> ():\n    print(1)\n",
    ] {
        let d = tam("reserved");
        let p = d.join("reserved.vis");
        std::fs::write(
            &p,
            format!("# SPDX-License-Identifier: MIT OR Apache-2.0\n{src}"),
        )
        .expect("ghi");
        let o = Command::new(env!("CARGO_BIN_EXE_vis"))
            .arg("check")
            .arg(&p)
            .output()
            .expect("vis check");
        let tho = String::from_utf8_lossy(&o.stdout).to_string();
        assert_eq!(o.status.code().unwrap_or(-1), 1);
        assert!(tho.contains("VS0302"), "không có VS0302: {tho}");
        let _ = std::fs::remove_dir_all(&d);
    }
}

/// (6) `AnyError` đã là **đồng nghĩa `{error}`** — `fn f() -> Result[int, AnyError]` compile sạch.
#[test]
fn anyerror_la_dong_nghia_poison() {
    let [a, b] = chay_ca_hai(
        "anyerr",
        "fn f() -> Result[int, AnyError]:\n    Ok(7)\nfn main() -> ():\n    match f():\n        Ok(n): print(n)\n        Err(_): print(\"e\")\n",
    );
    assert_eq!(a.0, 0, "interp: {}\n{}", a.1, a.2);
    assert_eq!(b.0, 0, "vm: {}\n{}", b.1, b.2);
    assert_eq!(a.1, "7\n");
}

/// (7) `impl Error` thiếu phương thức ⇒ `VS0403`; đủ ba + `impl Show` ⇒ chạy được.
#[test]
fn impl_error_thieu_phuong_thuc_bao_vs0403() {
    let src = "type Hop:\n    x: int\n\nimpl Show for Hop:\n    fn show(self) -> str:\n        \"hop\"\n\nimpl Error for Hop:\n    fn message(self) -> str:\n        \"m\"\n\nfn main() -> ():\n    print(1)\n";
    let d = tam("thieu");
    let p = d.join("thieu.vis");
    std::fs::write(
        &p,
        format!("# SPDX-License-Identifier: MIT OR Apache-2.0\n{src}"),
    )
    .expect("ghi");
    let o = Command::new(env!("CARGO_BIN_EXE_vis"))
        .arg("check")
        .arg(&p)
        .output()
        .expect("vis check");
    let tho = String::from_utf8_lossy(&o.stdout).to_string();
    assert_eq!(o.status.code().unwrap_or(-1), 1);
    assert!(tho.contains("VS0403"), "{tho}");
    assert!(tho.contains("source") || tho.contains("span"), "{tho}");
    let _ = std::fs::remove_dir_all(&d);
}

/// (8) `impl Show` người dùng + `impl Error` đủ ba ⇒ dispatch **tĩnh** cả hai đường.
#[test]
fn impl_error_du_ba_phuong_thuc_chay() {
    let [a, b] = chay_ca_hai(
        "du",
        "type Hop:\n    x: int\n\nimpl Show for Hop:\n    fn show(self) -> str:\n        \"hop\"\n\nimpl Error for Hop:\n    fn message(self) -> str:\n        \"m\"\n    fn source(self) -> Error?:\n        None\n    fn span(self) -> str?:\n        None\n\nfn main() -> ():\n    let h: Hop = {x: 9}\n    print(h.show())\n    print(h.message())\n",
    );
    assert_eq!(a.0, 0, "interp: {}", a.2);
    assert_eq!(a.1, "hop\nm\n");
    assert_eq!(b.0, 0, "vm: {}", b.2);
    assert_eq!(b.1, "hop\nm\n");
}

/// (9) `match` thiếu nhánh ⇒ VS0402 `NOT_EXHAUSTIVE` — enum `Json` bắt buột như enum thường.
#[test]
fn match_json_khong_du_nhanh_bao_vs0402() {
    let src = "use std.json\nfn main() -> ():\n    match json.decode(\"1\"):\n        Ok(j):\n            match j:\n                Num(_): print(\"n\")\n        Err(_): print(\"E\")\n";
    let d = tam("khongdu");
    let p = d.join("khongdu.vis");
    std::fs::write(
        &p,
        format!("# SPDX-License-Identifier: MIT OR Apache-2.0\n{src}"),
    )
    .expect("ghi");
    let o = Command::new(env!("CARGO_BIN_EXE_vis"))
        .arg("check")
        .arg(&p)
        .output()
        .expect("vis check");
    let tho = String::from_utf8_lossy(&o.stdout).to_string();
    assert_eq!(o.status.code().unwrap_or(-1), 1);
    assert!(tho.contains("VS0402"), "{tho}");
    let _ = std::fs::remove_dir_all(&d);
}

/// (10) `json.encode` round-trip trên cây `Json` mới — byte-đối-byte.
#[test]
fn encode_decode_round_trip() {
    let src = "use std.json\nfn main() -> ():\n    let s = \"[1, true, null, 3.5]\"\n    match json.decode(s):\n        Ok(j):\n            print(json.encode(j))\n        Err(_):\n            print(\"E\")\n";
    let [a, b] = chay_ca_hai("roundtrip", src);
    for r in [&a, &b] {
        assert_eq!(r.0, 0, "{r:?}");
        assert_eq!(r.1, "[1,true,null,3.5]\n");
    }
}
