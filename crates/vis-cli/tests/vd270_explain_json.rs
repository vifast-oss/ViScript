// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270** — `vis explain --json`: đầu ra máy đọc cho một mã và cho `--list`.
//!
//! Hợp đồng khoá: `--json` in **một dòng** JSON nén trên stdout (không trộn văn bản người đọc);
//! mã lạ/`--json` thiếu đích ⇒ exit 2, stdout rỗng, lỗi ở stderr. Không có `--json` thì stdout
//! của chế độ người đọc **không đổi** (khoá bởi `explain.rs`/`explain_moi.rs` và các ca dưới).

use std::process::Command;

use vis_interp::value::Value;

fn vis(args: &[&str]) -> (i32, String, String) {
    let o = Command::new(env!("CARGO_BIN_EXE_vis"))
        .args(args)
        .output()
        .expect("chạy được binary vis");
    (
        o.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&o.stdout).to_string(),
        String::from_utf8_lossy(&o.stderr).to_string(),
    )
}

fn truong<'a>(m: &'a [(Value, Value)], k: &str) -> Option<&'a Value> {
    m.iter().find_map(|(kk, v)| match kk {
        Value::Str(s) if s == k => Some(v),
        _ => None,
    })
}

fn chuoi<'a>(m: &'a [(Value, Value)], k: &str) -> &'a str {
    match truong(m, k) {
        Some(Value::Str(s)) => s,
        khac => panic!("thiếu chuỗi `{k}`: {khac:?}"),
    }
}

fn muc_json(c: &vis_diag::Code) -> &'static str {
    match c.severity {
        vis_diag::Severity::Error => "error",
        vis_diag::Severity::Warning => "warning",
    }
}

/// Một mã: đúng **một dòng** JSON nén, đúng thứ tự khoá, exit 0, stderr rỗng.
#[test]
fn ma_json_dung_mot_dong_va_exit_0() {
    let (code, out, err) = vis(&["explain", "--json", "VS0412"]);
    assert_eq!(code, 0, "stderr: {err}");
    assert_eq!(
        out,
        "{\"code\":\"VS0412\",\"severity\":\"error\",\"title\":\"kiểu không khớp\",\"body\":\"VS0412 — lỗi: kiểu không khớp\"}\n"
    );
    assert_eq!(out.lines().count(), 1, "phải đúng một dòng: {out:?}");
    assert!(err.is_empty(), "stderr phải rỗng: {err:?}");
}

/// Mức **cảnh báo** phải là `"warning"` (không phải nhãn tiếng Việt/Nhật).
#[test]
fn ma_canh_bao_json_severity_warning() {
    let (code, out, err) = vis(&["explain", "VS0107", "--json"]);
    assert_eq!(code, 0, "stderr: {err}");
    assert_eq!(
        out,
        "{\"code\":\"VS0107\",\"severity\":\"warning\",\"title\":\"tab (cảnh báo-lỗi; tab = 4)\",\"body\":\"VS0107 — cảnh báo: tab (cảnh báo-lỗi; tab = 4)\"}\n"
    );
}

/// `--lang` đổi `title`/`body` nhưng `severity` **luôn** là `error`/`warning`.
#[test]
fn ma_json_du_ba_ngon_ngu() {
    let cases = [
        (
            "vi",
            "{\"code\":\"VS0412\",\"severity\":\"error\",\"title\":\"kiểu không khớp\",\"body\":\"VS0412 — lỗi: kiểu không khớp\"}\n",
        ),
        (
            "en",
            "{\"code\":\"VS0412\",\"severity\":\"error\",\"title\":\"type mismatch\",\"body\":\"VS0412 — error: type mismatch\"}\n",
        ),
        (
            "ja",
            "{\"code\":\"VS0412\",\"severity\":\"error\",\"title\":\"型が一致しません\",\"body\":\"VS0412 — エラー: 型が一致しません\"}\n",
        ),
    ];
    for (lang, muon) in cases {
        let (code, out, err) = vis(&["explain", "--json", "--lang", lang, "VS0412"]);
        assert_eq!(code, 0, "{lang}: {err}");
        assert_eq!(out, muon, "{lang}");
    }
    // `--lang` đứng sau mã cũng phải ăn (thứ tự cờ tự do như chế độ người đọc).
    let (code, out, _) = vis(&["explain", "VS0412", "--json", "--lang", "en"]);
    assert_eq!(code, 0);
    assert!(out.contains("\"title\":\"type mismatch\""), "{out}");
}

/// `--list --json`: **một** object `{"codes":[…]}`, đúng 53 phần tử, đúng thứ tự `all_codes()`;
/// mỗi phần tử chỉ có `code`/`severity`/`title`, **không** có `body`.
#[test]
fn list_json_dung_hinh_dang_va_du_53_ma() {
    let (code, out, err) = vis(&["explain", "--list", "--json"]);
    assert_eq!(code, 0, "stderr: {err}");
    assert!(err.is_empty(), "stderr phải rỗng: {err:?}");
    assert_eq!(out.lines().count(), 1, "phải đúng một dòng: {out:?}");
    assert!(out.starts_with("{\"codes\":["), "{out}");
    assert!(out.ends_with("]}\n"), "{out}");
    assert!(!out.contains("body"), "list không có body: {out}");

    let v = vis_interp::json::decode(out.trim_end()).expect("JSON hợp lệ");
    let Value::Map(kv) = v else {
        panic!("không phải object: {v:?}")
    };
    assert_eq!(kv.len(), 1, "chỉ khoá `codes`: {kv:?}");
    let Some(Value::List(ds)) = truong(&kv, "codes") else {
        panic!("thiếu mảng `codes`")
    };

    let muon: Vec<&vis_diag::Code> = vis_diag::all_codes().collect();
    assert_eq!(ds.len(), muon.len(), "độ dài mảng phải bằng sổ");
    assert_eq!(ds.len(), 53, "sổ hiện có 53 mã");
    for (i, (p, c)) in ds.iter().zip(muon.iter()).enumerate() {
        let Value::Map(m) = p else {
            panic!("phần tử {i} không phải object: {p:?}")
        };
        assert_eq!(m.len(), 3, "phần tử {i} đúng 3 khoá: {m:?}");
        assert_eq!(chuoi(m, "code"), c.short, "phần tử {i}");
        assert_eq!(chuoi(m, "severity"), muc_json(c), "phần tử {i}");
        assert_eq!(
            chuoi(m, "title"),
            c.message(vis_diag::Lang::Vi),
            "phần tử {i}"
        );
    }
    // Đếm thô: mỗi phần tử đúng một `"code"`, không dòng người đọc nào lọt vào.
    assert_eq!(out.matches("{\"code\":\"").count(), 53, "{out}");
}

/// `--list --json --lang en`: `title` theo ngôn ngữ, thứ tự không đổi.
#[test]
fn list_json_theo_ngon_ngu() {
    let (code, out, err) = vis(&["explain", "--list", "--json", "--lang", "en"]);
    assert_eq!(code, 0, "stderr: {err}");
    assert!(
        out.starts_with(
            "{\"codes\":[{\"code\":\"VS0101\",\"severity\":\"error\",\"title\":\"unterminated string\"},"
        ),
        "{out}"
    );
    assert_eq!(
        out.matches("{\"code\":\"").count(),
        53,
        "vẫn đủ 53 mã: {out}"
    );
}

/// Mã không có trong sổ: exit 2, **không** JSON trên stdout, stderr giữ nguyên câu cũ.
#[test]
fn ma_khong_co_json_exit_2_khong_json() {
    let (code, out, err) = vis(&["explain", "--json", "VS9999"]);
    assert_eq!(code, 2);
    assert!(out.is_empty(), "không được in JSON: {out:?}");
    assert_eq!(err, "vis: không có mã VS9999 trong sổ (SPEC-lex §7)\n");
    for lang in ["vi", "en", "ja"] {
        let (code, out, _) = vis(&["explain", "--json", "--lang", lang, "VS9999"]);
        assert_eq!(code, 2, "{lang}");
        assert!(out.is_empty(), "{lang}: {out:?}");
    }
}

/// Mã sai định dạng cũng là lỗi dùng: exit 2, stdout rỗng.
#[test]
fn ma_sai_dinh_dang_json_exit_2() {
    for ma in ["VS041", "VS04120", "VS04I2", "vs0412x", "nonsense", ""] {
        let (code, out, err) = vis(&["explain", "--json", ma]);
        assert_eq!(code, 2, "{ma:?}");
        assert!(out.is_empty(), "{ma:?}: {out:?}");
        assert!(
            err.contains(&format!("không có mã {ma}")),
            "{ma:?}: {err:?}"
        );
    }
}

/// `--json` mà không có mã lẫn `--list` ⇒ exit 2, usage ở stderr, stdout rỗng.
#[test]
fn json_thieu_dich_exit_2_usage() {
    for args in [
        vec!["explain", "--json"],
        vec!["explain", "--json", "--lang", "en"],
        vec!["explain", "--lang", "ja", "--json"],
    ] {
        let (code, out, err) = vis(&args);
        assert_eq!(code, 2, "args: {args:?}");
        assert!(out.is_empty(), "args: {args:?} — stdout: {out:?}");
        assert!(err.contains("--json"), "usage phải nêu --json: {err:?}");
        assert!(
            err.contains("dùng:") || err.contains("usage:") || err.contains("使い方:"),
            "args: {args:?} — stderr: {err:?}"
        );
    }
}

/// Không có `--json`: stdout của chế độ người đọc **không đổi** từng byte.
#[test]
fn khong_json_giu_nguyen_che_do_nguoi_doc() {
    let (code, out, err) = vis(&["explain", "VS0412"]);
    assert_eq!(code, 0);
    assert_eq!(out, "VS0412 — lỗi: kiểu không khớp\n", "{out:?}");
    assert!(err.is_empty(), "{err:?}");

    let (code, out, err) = vis(&["explain", "--list"]);
    assert_eq!(code, 0);
    assert!(err.is_empty(), "{err:?}");
    assert_eq!(out.lines().count(), 53, "{out}");
    assert!(!out.contains("{\"code\":"), "không lẫn JSON: {out}");
    let dau = out.lines().next().unwrap_or("");
    assert!(
        dau.starts_with("VS0101  lỗi"),
        "dòng đầu giữ định dạng người đọc: {dau:?}"
    );
    let cuoi = out.lines().last().unwrap_or("");
    let ma_cuoi = vis_diag::all_codes().last().map(|c| c.short).unwrap_or("");
    assert!(
        cuoi.starts_with(ma_cuoi),
        "dòng cuối là mã cuối sổ: {cuoi:?}"
    );
}
