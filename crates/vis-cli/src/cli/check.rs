// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-116b**: `vis check` (TAP + `--json`) — tách nguyên khối từ `main.rs`, không đổi hành vi.
//! **VD-269**: thêm `--stdin` — kiểm bộ đệm UTF-8 từ stdin như một tệp script tên `-`.

use crate::*;
use std::process::ExitCode;

fn usage_check() -> ExitCode {
    eprintln!(
        "dùng: vis check [--syntax] [--no-extern] [--json] <tệp…> | vis check --stdin [--syntax] [--no-extern] [--json]"
    );
    eprintln!(
        "  --stdin: đọc UTF-8 từ stdin, kiểm như một tệp script tên `-`; không nhận tệp; thoát 0 (sạch) / 1 (có chẩn đoán) / 2 (dùng sai, không đọc được hoặc không phải UTF-8)"
    );
    ExitCode::from(2)
}

/// Kết quả kiểm **một** tệp: `Ok(())` sạch, `Err(<mã> <dòng>:<cột> <thông điệp>)` hoặc
/// `Err(không đọc được: …)`. Chỉ trả **một** chẩn đoán (lọc tầng như [`chon_chan_doan`]).
/// Chẩn đoán của `vis check` cho một tệp (ST-25: có thể đến từ tệp **phụ**).
struct ChanDoanCheck {
    ma: &'static str,
    line: u32,
    col: u32,
    msg: String,
    /// Tệp phụ (rỗng nếu chẩn đoán ở tệp chính).
    phu: String,
    /// **ST-110**: `"error"` hoặc `"warning"` (lấy từ `vis_diag::Code.severity`).
    severity: &'static str,
}

/// Tệp trong `check --json`: `rel` để khớp chẩn đoán, `hien` để in (tệp chính = đường dẫn đối số).
struct TepJson {
    rel: String,
    hien: String,
}

/// Kết quả `check_all`: **mọi tệp đã nạp** (chính trước, module theo thứ tự `use`) + chẩn đoán.
struct KetQuaCheck {
    files: Vec<TepJson>,
    diags: Vec<ChanDoanCheck>,
}

/// DFS **tiền tự** theo `deps`: chính trước, rồi module theo thứ tự `use` gặp.
fn thu_tu_nap(
    rel: &str,
    deps: &[(String, Vec<String>)],
    seen: &mut std::collections::BTreeSet<String>,
    out: &mut Vec<String>,
) {
    if !seen.insert(rel.to_string()) {
        return;
    }
    out.push(rel.to_string());
    if let Some((_, con)) = deps.iter().find(|(r, _)| r == rel) {
        for c in con {
            thu_tu_nap(c, deps, seen, out);
        }
    }
}

#[allow(dead_code)] // giữ API cũ (mặc định kiểm kiểu) cho người đọc/tests.
fn check_one_co(path: &str, cho_ngoai: bool) -> Result<(), ChanDoanCheck> {
    check_mode(path, cho_ngoai, true)
}

/// **ST-153 (VD-152)**: tên gốc của `path`, tách theo **cả** `/` (Unix) và `\` (Windows).
/// Dùng để so với `rel` của chẩn đoán: sai ở đây thì trên Windows `phu` luôn khác rỗng và bộ lọc
/// `--syntax` (QD §7.4) **giữ** chẩn đoán kiểu. Không đổi hành vi Unix.
fn ten_goc(path: &str) -> &str {
    path.rsplit(['/', '\\']).next().unwrap_or(path)
}

/// `kiem_kieu = false` ⇒ `--syntax`: chỉ cú pháp/hạ (bỏ `tyck`).
fn check_mode(path: &str, cho_ngoai: bool, kiem_kieu: bool) -> Result<(), ChanDoanCheck> {
    if let Err(e) = std::fs::read_to_string(path) {
        return Err(ChanDoanCheck {
            ma: "VS0201",
            line: 1,
            col: 1,
            msg: format!("không đọc được: {e}"),
            phu: String::new(),
            severity: "error",
        });
    }
    let loaded = modules::load_full(path, cho_ngoai, kiem_kieu).map_err(loi_nap)?;
    check_loaded(path, loaded, kiem_kieu)
}

/// **VD-269**: như [`check_mode`] nhưng tệp chính là **bộ đệm stdin** ảo `-` (cùng đường
/// [`modules::load_full_stdin`], không đọc/ghi đĩa).
fn check_stdin(text: &str, cho_ngoai: bool, kiem_kieu: bool) -> Result<(), ChanDoanCheck> {
    let loaded = modules::load_full_stdin(text, cho_ngoai, kiem_kieu).map_err(loi_nap)?;
    check_loaded("-", loaded, kiem_kieu)
}

/// Lỗi **cấp API** khi nạp (không phải chẩn đoán ngôn ngữ) ⇒ một dòng `VS0201`.
fn loi_nap(e: modules::LoadError) -> ChanDoanCheck {
    ChanDoanCheck {
        ma: "VS0201",
        line: 1,
        col: 1,
        msg: e.to_string(),
        phu: String::new(),
        severity: "error",
    }
}

/// Phần **sau nạp** dùng chung cho tệp và stdin: in warning mềm, lọc tầng (QD §7.4), trả
/// **một** chẩn đoán đầu tiên. `path` chỉ dùng để tách tệp chính/phụ (`-` cho stdin).
fn check_loaded(path: &str, loaded: modules::Loaded, kiem_kieu: bool) -> Result<(), ChanDoanCheck> {
    let la_vish = path.ends_with(".vish");
    // **VD-216 (VD-212, 2026-11-18)**: `VS0111` chết — bỏ bộ lọc chết. Mọi `Severity::Warning`
    // khác (`VS0208` danh sách làm khoá, …) **in** ra stderr nhưng **không** làm đỏ;
    // `VS0107` (tab) vẫn đỏ như cũ (VD-12).
    const MEM: vis_diag::Severity = vis_diag::Severity::Warning;
    let mem = |d: &(String, vis_syntax::Diagnostic)| {
        d.1.code.severity == MEM && d.1.code.short != "VS0107"
    };
    for (r, w) in &loaded.diags {
        if w.code.severity == MEM && w.code.short != "VS0107" {
            eprintln!(
                "{r}:{}:{}: warning {}: {}",
                w.line, w.col, w.code.short, w.message
            );
        }
    }
    let Some((rel, d)) = loaded.diags.iter().find(|x| !mem(x)) else {
        return Ok(());
    };
    let ma = d.code.short;
    let chinh = ten_goc(path);
    let phu = if rel == chinh {
        String::new()
    } else {
        rel.clone()
    };
    // **SỬA KÈM LÝ DO (ST-28)**: `vis check` **mặc định** báo **đúng chẩn đoán đầu** như `vis run`
    // (bỏ bộ lọc `01xx/02xx/07xx` của ST-25 — nó làm hai đường **lệch kết luận**: check xanh trong
    // khi run `VS0412`/`VS0301`). Bộ lọc chỉ còn ở chế độ **`--syntax`** (chỉ parse + cấu trúc
    // module), dùng cho cổng "khối mã trong tài liệu parse được" (QD §7.4).
    if !kiem_kieu {
        let giu = la_vish
            || !phu.is_empty()
            || ma.starts_with("VS01")
            || ma.starts_with("VS02")
            || ma.starts_with("VS06")
            || ma.starts_with("VS07");
        if !giu {
            return Ok(());
        }
    }
    Err(ChanDoanCheck {
        ma,
        line: d.line,
        col: d.col,
        msg: d.message.clone(),
        phu,
        severity: severity_cua(d.code.severity),
    })
}

/// **ST-110**: ánh xạ **mức** của mã (`vis_diag::Severity`) — không thêm/chia lại sổ mã.
fn severity_cua(s: vis_diag::Severity) -> &'static str {
    match s {
        vis_diag::Severity::Warning => "warning",
        vis_diag::Severity::Error => "error",
    }
}

/// **ST-101**: như [`check_mode`] nhưng trả **mọi** chẩn đoán (giữ thứ tự phát hiện) — cho `--json`.
/// `vis check` TAP vẫn chỉ báo chẩn đoán **đầu** như trước; JSON cần đủ.
fn check_all(path: &str, cho_ngoai: bool, kiem_kieu: bool) -> KetQuaCheck {
    if let Err(e) = std::fs::read_to_string(path) {
        return ket_qua_loi(path, format!("không đọc được: {e}"));
    }
    match modules::load_full(path, cho_ngoai, kiem_kieu) {
        Ok(l) => check_all_loaded(path, l, kiem_kieu),
        Err(e) => ket_qua_loi(path, e.to_string()),
    }
}

/// **VD-269**: như [`check_all`] nhưng tệp chính là **bộ đệm stdin** ảo `-`.
fn check_all_stdin(text: &str, cho_ngoai: bool, kiem_kieu: bool) -> KetQuaCheck {
    match modules::load_full_stdin(text, cho_ngoai, kiem_kieu) {
        Ok(l) => check_all_loaded("-", l, kiem_kieu),
        Err(e) => ket_qua_loi("-", e.to_string()),
    }
}

/// Kết quả lỗi **cấp API** (không đọc được/nạp được) ⇒ đúng một `VS0201`, khớp đường TAP.
fn ket_qua_loi(path: &str, msg: String) -> KetQuaCheck {
    KetQuaCheck {
        files: vec![TepJson {
            rel: path.to_string(),
            hien: path.to_string(),
        }],
        diags: vec![ChanDoanCheck {
            ma: "VS0201",
            line: 1,
            col: 1,
            msg,
            phu: String::new(),
            severity: "error",
        }],
    }
}

/// Phần **sau nạp** dùng chung cho tệp và stdin: dựng danh sách tệp theo thứ tự nạp + lọc
/// chẩn đoán như [`check_loaded`]. `path` chỉ dùng để hiện tên tệp chính (`-` cho stdin).
fn check_all_loaded(path: &str, loaded: modules::Loaded, kiem_kieu: bool) -> KetQuaCheck {
    // `.vish` có thể có `FileUnit.rel` rỗng ⇒ fallback đúng đường dẫn đối số (không mất chẩn đoán).
    let root = loaded
        .files
        .last()
        .map(|f| f.rel.clone())
        .filter(|r| !r.is_empty())
        // **ST-211 (VD-184)**: cùng luật tách như `ten_goc` (ST-153) — `--syntax` giữ trọn chẩn đoán.
        .unwrap_or_else(|| ten_goc(path).to_string());
    let mut thu_tu = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    thu_tu_nap(&root, &loaded.deps, &mut seen, &mut thu_tu);
    for f in &loaded.files {
        if !seen.contains(&f.rel) {
            thu_tu.push(f.rel.clone());
        }
    }
    let la_vish = path.ends_with(".vish");
    let mut diags = Vec::new();
    for (rel, d) in &loaded.diags {
        let ma = d.code.short;
        let phu = if rel == &root {
            String::new()
        } else {
            rel.clone()
        };
        if !kiem_kieu {
            let giu = la_vish
                || !phu.is_empty()
                || ma.starts_with("VS01")
                || ma.starts_with("VS02")
                || ma.starts_with("VS06")
                || ma.starts_with("VS07");
            if !giu {
                continue;
            }
        }
        diags.push(ChanDoanCheck {
            ma,
            line: d.line,
            col: d.col,
            msg: d.message.clone(),
            phu,
            severity: severity_cua(d.code.severity),
        });
    }
    let files = thu_tu
        .into_iter()
        .map(|rel| TepJson {
            hien: if rel == root {
                path.to_string()
            } else {
                rel.clone()
            },
            rel,
        })
        .collect();
    KetQuaCheck { files, diags }
}

/// **ST-101**: `vis check --json` — **một** object nén trên stdout; `line`/`col` **1-based**; thứ tự
/// `files` = thứ tự đối số, `diagnostics` = thứ tự phát hiện; exit y hệt không cờ.
pub(crate) fn run_check_json(args: &[&String], cho_ngoai: bool, kiem_kieu: bool) -> ExitCode {
    let kqs: Vec<KetQuaCheck> = args
        .iter()
        .map(|path| check_all(path.as_str(), cho_ngoai, kiem_kieu))
        .collect();
    xuat_check_json(&kqs)
}

/// **VD-269**: phần in JSON dùng chung cho đường tệp và `--stdin` (một kết quả ⇒ một tệp `-`).
fn xuat_check_json(kqs: &[KetQuaCheck]) -> ExitCode {
    let mut files = String::new();
    let mut ok = true;
    for (i, kq) in kqs.iter().enumerate() {
        // **ST-240 / VD-242 §4 (nợ #180)**: khớp TAP — warning mềm vẫn được phát ra nhưng **không**
        // làm đỏ. Ngoại lệ `VS0107` (tab) vẫn là lỗi như TAP/`run --tap`.
        let co_loi = kq
            .diags
            .iter()
            .any(|d| !(d.severity == "warning" && d.ma != "VS0107"));
        if co_loi {
            ok = false;
        }
        if i > 0 {
            files.push(',');
        }
        for (j, tep) in kq.files.iter().enumerate() {
            if j > 0 {
                files.push(',');
            }
            files.push_str(&format!(
                "{{\"file\":\"{}\",\"diagnostics\":[",
                json_esc::esc(&tep.hien)
            ));
            let cua = kq
                .diags
                .iter()
                .filter(|x| (tep.rel == kq.files[0].rel && x.phu.is_empty()) || x.phu == tep.rel);
            for (k, x) in cua.enumerate() {
                if k > 0 {
                    files.push(',');
                }
                files.push_str(&format!(
                    "{{\"code\":\"{}\",\"line\":{},\"col\":{},\"severity\":\"{}\",\"message\":\"{}\"}}",
                    json_esc::esc(x.ma),
                    x.line,
                    x.col,
                    x.severity,
                    json_esc::esc(&x.msg)
                ));
            }
            files.push_str("]}");
        }
    }
    let body = format!("{{\"ok\":{},\"files\":[{files}]}}\n", ok);
    if !xuat_stdout(&body) {
        return ExitCode::from(2);
    }
    if ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

/// Dòng TAP `not ok` + **đúng một** dòng chẩn đoán — dùng chung cho đường tệp và `--stdin`
/// (nhờ vậy `-` in y hệt một tệp, chỉ khác tên).
fn dong_not_ok(n: usize, path: &str, d: &ChanDoanCheck) -> String {
    // ST-25: chẩn đoán ở tệp phụ ⇒ dòng `not ok` nêu `tệp:dòng:cột` (TAP đúng tệp).
    let them = if d.phu.is_empty() {
        String::new()
    } else {
        format!(" ({}:{}:{})", d.phu, d.line, d.col)
    };
    // Lỗi **đọc tệp** giữ nguyên định dạng cũ: `# không đọc được: …` (không mã).
    let dong = if d.msg.starts_with("không đọc được") {
        d.msg.clone()
    } else {
        dong_chan_doan(d.ma, d.line, d.col, &d.msg)
    };
    format!("not ok {n} - {path}{them}\n# {dong}\n")
}

/// **VD-269**: `vis check --stdin` — đọc **toàn bộ** UTF-8 từ stdin, kiểm như một tệp script
/// tên hiển thị `-` qua cùng đường [`check_stdin`]/[`check_all_stdin`]. Không nhận tệp/đối số;
/// `--syntax`, `--no-extern`, `--json` vẫn được tôn trọng. Thoát 0 sạch / 1 có chẩn đoán /
/// 2 dùng sai, lỗi đọc hoặc stdin **không phải UTF-8**.
fn run_check_stdin(
    con_lai: &[&String],
    cho_ngoai: bool,
    kiem_kieu: bool,
    json_ra: bool,
) -> ExitCode {
    if let Some(bad) = con_lai.iter().find(|a| a.starts_with('-')) {
        eprintln!("vis check: không hỗ trợ cờ `{bad}` khi dùng --stdin");
        return usage_check();
    }
    if let Some(a) = con_lai.first() {
        eprintln!("vis check: --stdin không nhận đối số `{a}` (không nhận tệp)");
        return usage_check();
    }
    use std::io::Read;
    let mut raw = Vec::new();
    if let Err(e) = std::io::stdin().read_to_end(&mut raw) {
        eprintln!("vis check: không đọc được stdin: {e}");
        return ExitCode::from(2);
    }
    let src = match String::from_utf8(raw) {
        Ok(s) => s,
        Err(_) => {
            eprintln!("vis check: stdin không phải UTF-8 hợp lệ");
            return ExitCode::from(2);
        }
    };
    if json_ra {
        let kq = check_all_stdin(&src, cho_ngoai, kiem_kieu);
        return xuat_check_json(&[kq]);
    }
    let mut tap = String::from("TAP version 13\n1..1\n");
    let (dong, failed) = match check_stdin(&src, cho_ngoai, kiem_kieu) {
        Ok(()) => ("ok 1 - -\n".to_string(), false),
        Err(d) => (dong_not_ok(1, "-", &d), true),
    };
    tap.push_str(&dong);
    if !xuat_stdout(&tap) {
        return ExitCode::from(2);
    }
    if failed {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

/// `vis check <tệp…>` — in TAP 13. Thoát 0 (sạch) / 1 (có tệp hỏng) / 2 (dùng sai, `.vish`).
pub(crate) fn run_check(args: &[String]) -> ExitCode {
    if args.is_empty() {
        return usage_check();
    }
    // ST-27: `--no-extern` — lệnh lạ báo `VS0601` (không cho chương trình ngoài).
    let cho_ngoai = !args.iter().any(|a| a == "--no-extern");
    // ST-28: `--syntax` — chỉ cú pháp/hạ (bỏ `tyck`), dùng cho cổng "khối mã trong tài liệu
    // **parse** được" (QD §7.4).
    let kiem_kieu = !args.iter().any(|a| a == "--syntax");
    // **ST-101**: `--json` — đầu ra máy đọc, một dòng, stderr rỗng.
    let json_ra = args.iter().any(|a| a == "--json");
    // **VD-269**: `--stdin` là chế độ riêng — không lẫn với đường tệp.
    let stdin_ra = args.iter().any(|a| a == "--stdin");
    let args: Vec<&String> = args
        .iter()
        .filter(|a| {
            !matches!(
                a.as_str(),
                "--no-extern" | "--syntax" | "--json" | "--stdin"
            )
        })
        .collect();
    if stdin_ra {
        return run_check_stdin(&args, cho_ngoai, kiem_kieu, json_ra);
    }
    if json_ra {
        if args.is_empty() {
            return usage_check();
        }
        // **VD-270 (check-json, 2026-11-21)**: cờ lạ phải là **lỗi dùng** (exit 2, stdout rỗng)
        // như đường TAP/`--stdin`; trước đây `--json --la` coi `--la` là tệp ⇒ `VS0201` exit 1.
        if let Some(bad) = args.iter().find(|a| a.starts_with('-')) {
            eprintln!("vis check: không hỗ trợ cờ `{bad}`");
            return usage_check();
        }
        return run_check_json(&args, cho_ngoai, kiem_kieu);
    }
    if let Some(bad) = args.iter().find(|a| a.starts_with('-')) {
        eprintln!("vis check: không hỗ trợ cờ `{bad}`");
        return usage_check();
    }
    let mut tap = String::new();
    tap.push_str("TAP version 13\n");
    tap.push_str(&format!("1..{}\n", args.len()));
    let mut failed = false;
    for (i, path) in args.iter().enumerate() {
        let n = i + 1;
        match check_mode(path, cho_ngoai, kiem_kieu) {
            Ok(()) => tap.push_str(&format!("ok {n} - {path}\n")),
            Err(d) => {
                failed = true;
                tap.push_str(&dong_not_ok(n, path.as_str(), &d));
            }
        }
    }
    if !xuat_stdout(&tap) {
        return ExitCode::from(2);
    }
    if failed {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

#[cfg(test)]
mod tests {
    /// **ST-153 (VD-152)**: đường dẫn Windows dùng `\` ⇒ tên gốc **không** còn dấu phân cách
    /// (trước ST-153 hàm này trả nguyên cả đường dẫn).
    #[test]
    fn ten_goc_windows() {
        assert_eq!(super::ten_goc(r"C:\x\a.vis"), "a.vis");
    }

    /// **ST-153 (VD-152)**: đường dẫn Unix giữ **nguyên** hành vi cũ (`rsplit('/')`).
    #[test]
    fn ten_goc_unix() {
        assert_eq!(super::ten_goc("d/a.vis"), "a.vis");
    }

    /// **ST-153 (VD-152)**: tên **không** có dấu phân cách ⇒ nguyên vẹn.
    #[test]
    fn ten_goc_khong_co_dau() {
        assert_eq!(super::ten_goc("a.vis"), "a.vis");
    }

    /// **ST-211 (VD-184)**: bộ ca khoá đầy đủ cho tên gốc đường dẫn — tương đối, tuyệt đối `/…`,
    /// dấu Windows `C:\` + `C:/…`, tên chứa khoảng trắng, và Unix với dấu chết `\` (thái Long
    /// lưu ý: tách theo **cả** `\` và `/` — ST-153).
    #[test]
    fn ten_goc_du() {
        assert_eq!(super::ten_goc("./a.vis"), "a.vis"); // tương đối
        assert_eq!(super::ten_goc("/etc/vis/a.vis"), "a.vis"); // tuyệt đối Unix
        assert_eq!(super::ten_goc(r"C:\dir\a.vis"), "a.vis"); // Windows C:\
        assert_eq!(super::ten_goc("C:/dir/a.vis"), "a.vis"); // Windows C:/
        assert_eq!(super::ten_goc(r"C:/dir\a.vis"), "a.vis"); // hỗn hợp
        assert_eq!(super::ten_goc("my dir/a.vis"), "a.vis"); // khoảng trắng
        assert_eq!(super::ten_goc(""), ""); // rỗng (không panic)
        assert_eq!(super::ten_goc("dir/"), ""); // đuôi cắt
        assert_eq!(super::ten_goc("d\\a.vis"), "a.vis"); // \ trên cả Windows & Unix (ST-153)
    }
}
