// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-318e (VD-297)** — kiểu kết quả và bộ chạy ca của `vis test`: `test fn`/fixture, tuần tự
//! và song song. Tách nguyên văn khỏi `vistest.rs` (0 đổi hành vi/TAP/JSON).

use std::path::Path;
use std::process::Command;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use super::*;

/// Kết quả **một lần chạy** một ca — tương ứng một dòng TAP và một phần tử `cases` JSON.
pub(super) struct Kq {
    pub(super) ok: bool,
    /// Dòng chi tiết ngay sau `not ok` (không có `# `, không có newline); rỗng khi `ok`.
    pub(super) chi_tiet: String,
    /// `message` cho `--json` (đã làm một dòng + cắt ≤200 ký tự).
    pub(super) msg: String,
}

impl Kq {
    fn ok() -> Self {
        Kq {
            ok: true,
            chi_tiet: String::new(),
            msg: String::new(),
        }
    }

    fn hong(chi_tiet: String, msg: impl Into<String>) -> Self {
        Kq {
            ok: false,
            chi_tiet,
            msg: mot_dong(&msg.into()),
        }
    }
}

/// **VD-270**: chạy **một lần** ca `p` — `test fn` (bản sao `thanh_main`) hoặc fixture `.out/.err`.
///
/// Tách khỏi vòng chọn ca để `--repeat` gọi lại nguyên xi từng lần; `--timeout` áp **mỗi lần**
/// chạy của **cả hai** nhánh — `test fn` **và** fixture `.out/.err` (VD-270 mở rộng ST-105).
///
/// `don_vi` là chỉ số **đơn vị** (ca × lần lặp, 0-based) — duy nhất kể cả khi `--jobs > 1`, nên
/// thư mục tạm `vis-test-{pid}-{don_vi}` không đụng nhau giữa các worker.
fn chay_mot_lan(
    don_vi: usize,
    p: &Path,
    bai_test: Option<&(String, String)>,
    ten: &str,
    vm: bool,
    strict: bool,
    timeout_han: Option<u64>,
) -> Kq {
    if let Some((_, src)) = bai_test {
        // Ghi bản sao tạm, chạy `vis run` (interp hoặc VM), TAP theo **tên test**.
        let d = std::env::temp_dir().join(format!("vis-test-{}-{}", std::process::id(), don_vi));
        let _ = std::fs::remove_dir_all(&d);
        let _ = std::fs::create_dir_all(&d);
        let tep = d.join("t.vis");
        if let Err(e) = std::fs::write(&tep, src) {
            let m = format!("không ghi được bản sao: {e}");
            return Kq::hong(m.clone(), m);
        }
        let mut c = Command::new(std::env::current_exe().unwrap_or_else(|_| PathBuf::from("vis")));
        dat_path_tro_giup(&mut c);
        c.arg("run");
        if strict {
            c.arg("--strict-caps");
        }
        if vm {
            c.arg("--vm").arg("--no-cache");
        }
        c.arg(&tep);
        let (code, _out, stderr, qua_han) = chay_theo_han(&mut c, timeout_han);
        let _ = std::fs::remove_dir_all(&d);
        if qua_han {
            let ms = timeout_han.unwrap_or(0);
            let m = format!("quá hạn {ms} ms");
            return Kq::hong(format!("{ten}: {m}"), m);
        }
        if code == 0 {
            return Kq::ok();
        }
        let (ma, dong, cot, msg) =
            chan_doan(&stderr).unwrap_or(("VS?".into(), 1, 1, String::new()));
        return Kq::hong(
            format!("{ten}:{dong}:{cot}: {ma} {msg}"),
            format!("{ma} {msg}"),
        );
    }
    let err = p.with_extension("err");
    let trap = p.with_extension("trap");
    let prefix = p.with_extension("out_prefix");
    let (code, out, stderr, qua_han) = chay(p, vm, strict, timeout_han);
    // **VD-270**: hạn thắng mọi kỳ vọng `.err`/`.out`; cùng câu với nhánh `test fn`.
    if qua_han {
        let ms = timeout_han.unwrap_or(0);
        let m = format!("quá hạn {ms} ms");
        return Kq::hong(format!("{ten}: {m}"), m);
    }

    // Bổ sung: helper đọc fixture `.exit` / dòng kỳ vọng đầu (SPDX-safe). Nested fn — không đụng module khác.
    fn doc_exit(p: &Path) -> Option<i32> {
        let s = std::fs::read_to_string(p.with_extension("exit")).ok()?;
        s.lines()
            .map(str::trim)
            .find(|l| !l.is_empty() && !l.starts_with('#'))?
            .parse::<i32>()
            .ok()
    }
    fn dong_dau(p: &Path, ext: &str) -> String {
        let s = std::fs::read_to_string(p.with_extension(ext)).unwrap_or_default();
        s.lines()
            .map(str::trim)
            .find(|l| !l.is_empty() && !l.starts_with('#'))
            .unwrap_or("")
            .to_string()
    }

    // `.trap` (28 ca): kỳ vọng **đúng mã** VSxxxx trong stderr và exit ≠ 0 (`.exit` override khi có).
    if trap.exists() {
        let mong = dong_dau(p, "trap");
        let ma_thuc = chan_doan(&stderr).map(|(m, _, _, _)| m).unwrap_or_default();
        let exit_ok = doc_exit(p).map_or(code != 0, |e| code == e);
        if exit_ok && !mong.is_empty() && ma_thuc == mong {
            return Kq::ok();
        }
        let (ma, dong, cot, msg) = chan_doan(&stderr).unwrap_or((
            if mong.is_empty() { "VS?".into() } else { mong },
            1,
            1,
            String::new(),
        ));
        return Kq::hong(
            format!("{ten}:{dong}:{cot}: {ma} {msg}"),
            format!("{ma} {msg}"),
        );
    }

    // `.out_prefix` (2 ca): stdout **bắt đầu bằng** prefix, sau `bo_spdx`; exit mặc định 0.
    if prefix.exists() {
        let prefix_str = std::fs::read_to_string(&prefix).unwrap_or_default();
        let mong = bo_spdx(&prefix_str);
        let muon = doc_exit(p).unwrap_or(0);
        if code == muon && bo_spdx(&out).starts_with(mong) {
            return Kq::ok();
        }
        return Kq::hong(
            format!("{ten}:1:1: VS0000 stdout khác kỳ vọng"),
            "VS0000 stdout khác kỳ vọng",
        );
    }

    if err.exists() {
        let mong = dong_dau(p, "err");
        let muon = doc_exit(p).unwrap_or_else(|| if mong.starts_with("VS05") { 101 } else { 2 });
        // ST-30b: hai đường **đã đồng bộ** chẩn đoán ⇒ `--vm` khoá **đúng mã** như interp.
        if code == muon && stderr.contains(&mong) {
            return Kq::ok();
        }
        let (ma, dong, cot, msg) =
            chan_doan(&stderr).unwrap_or((mong.clone(), 1, 1, String::new()));
        return Kq::hong(
            format!("{ten}:{dong}:{cot}: {ma} {msg}"),
            format!("{ma} {msg}"),
        );
    }
    let want_path = p.with_extension("out");
    let want = std::fs::read_to_string(&want_path).unwrap_or_default();
    let want = bo_spdx(&want);
    if code == doc_exit(p).unwrap_or(0) && out == want {
        return Kq::ok();
    }
    if code != 0 {
        let (ma, dong, cot, msg) =
            chan_doan(&stderr).unwrap_or(("VS?".into(), 1, 1, String::new()));
        return Kq::hong(
            format!("{ten}:{dong}:{cot}: {ma} {msg}"),
            format!("{ma} {msg}"),
        );
    }
    Kq::hong(
        format!("{ten}:1:1: VS0000 stdout khác kỳ vọng"),
        "VS0000 stdout khác kỳ vọng",
    )
}

/// **VD-270**: cấu hình chạy chung cho đường tuần tự và song song (gom tham số, tránh lặp).
#[derive(Clone, Copy)]
pub(super) struct CauHinhChay<'a> {
    pub(super) goc: &'a Path,
    pub(super) vm: bool,
    pub(super) strict: bool,
    pub(super) timeout_han: Option<u64>,
    pub(super) repeat: usize,
    pub(super) jobs: usize,
    pub(super) fail_fast: bool,
}

/// **VD-270**: chạy **tuần tự** mọi đơn vị (hành vi cũ, `--jobs 1`) — `--fail-fast` dừng **trước**
/// đơn vị kế tiếp ngay sau lần đỏ đầu; đuôi chưa chạy để `None`.
pub(super) fn chay_tuan_tu(
    ca: &[(PathBuf, Option<(String, String)>)],
    cfg: &CauHinhChay<'_>,
) -> KetQuaDonVi {
    let so_don_vi = ca.len().saturating_mul(cfg.repeat);
    let mut ra: KetQuaDonVi = (0..so_don_vi).map(|_| None).collect();
    let mut hong = 0usize;
    let mut don_vi = 0usize;
    'ca: for (p, bai_test) in ca {
        let ten_goc = ten_goc_cua(cfg.goc, p, bai_test.as_ref());
        for k in 1..=cfg.repeat {
            // **VD-267/VD-270**: `--fail-fast` dừng **trước** lần lặp kế tiếp ngay sau lần đỏ đầu.
            if cfg.fail_fast && hong > 0 {
                break 'ca;
            }
            let ten = ten_don_vi(&ten_goc, k, cfg.repeat);
            // **VD-270** (`--slowest`): chỉ đo **lần chạy** của đơn vị, không tính lúc in.
            let bat = std::time::Instant::now();
            let kq = chay_mot_lan(
                don_vi,
                p,
                bai_test.as_ref(),
                &ten_goc,
                cfg.vm,
                cfg.strict,
                cfg.timeout_han,
            );
            let thoi_gian = bat.elapsed();
            if !kq.ok {
                hong += 1;
            }
            ra[don_vi] = Some((ten, kq, thoi_gian));
            don_vi += 1;
        }
    }
    ra
}

/// **VD-270**: chạy **song song** `jobs` đơn vị bằng `std::thread::scope` + atomic + kênh; kết quả
/// ghi vào **đúng ô** của nó nên in ra vẫn theo thứ tự gốc.
///
/// `--fail-fast`: worker kiểm cờ `dung` **trước khi nhận** đơn vị mới; đơn vị đã nhận vẫn hoàn tất
/// rồi mới gửi kết quả. Vì chỉ số phát bằng `fetch_add` nên tập đã nhận luôn là **tiền tố** của
/// thứ tự gốc; đơn vị sau ca đỏ có thể đã kịp chạy (khác bản tuần tự), plan đếm đúng số đã chạy.
pub(super) fn chay_song_song(
    ca: &[(PathBuf, Option<(String, String)>)],
    cfg: &CauHinhChay<'_>,
) -> KetQuaDonVi {
    let so_don_vi = ca.len().saturating_mul(cfg.repeat);
    let mut ra: KetQuaDonVi = (0..so_don_vi).map(|_| None).collect();
    if so_don_vi == 0 {
        return ra;
    }
    let tiep = AtomicUsize::new(0);
    let dung = AtomicBool::new(false);
    let (tx, rx) = std::sync::mpsc::channel::<(usize, String, Kq, std::time::Duration)>();
    std::thread::scope(|s| {
        for _ in 0..cfg.jobs.min(so_don_vi) {
            let tx = tx.clone();
            let tiep = &tiep;
            let dung = &dung;
            let cfg = *cfg;
            s.spawn(move || loop {
                if cfg.fail_fast && dung.load(Ordering::Acquire) {
                    break;
                }
                let don_vi = tiep.fetch_add(1, Ordering::AcqRel);
                if don_vi >= so_don_vi {
                    break;
                }
                let i_ca = don_vi / cfg.repeat;
                let k = don_vi % cfg.repeat + 1;
                let (p, bai_test) = &ca[i_ca];
                let ten_goc = ten_goc_cua(cfg.goc, p, bai_test.as_ref());
                let ten = ten_don_vi(&ten_goc, k, cfg.repeat);
                // **VD-270** (`--slowest`): thời gian tường của **đơn vị** (chỉ lúc chạy).
                let bat = std::time::Instant::now();
                let kq = chay_mot_lan(
                    don_vi,
                    p,
                    bai_test.as_ref(),
                    &ten_goc,
                    cfg.vm,
                    cfg.strict,
                    cfg.timeout_han,
                );
                let thoi_gian = bat.elapsed();
                if cfg.fail_fast && !kq.ok {
                    dung.store(true, Ordering::Release);
                }
                if tx.send((don_vi, ten, kq, thoi_gian)).is_err() {
                    break;
                }
            });
        }
        // Đóng đầu gửi gốc: vòng nhận dưới đây kết thúc khi **mọi** worker xong.
        drop(tx);
        for (don_vi, ten, kq, thoi_gian) in rx {
            ra[don_vi] = Some((ten, kq, thoi_gian));
        }
    });
    ra
}
