// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270** — `vis test --slowest <N>`: báo cáo N **đơn vị** chạy chậm nhất ra **stderr**.
//!
//! Khoá hành vi:
//! - `N` nguyên ≥ 1; thiếu/`0`/âm/không nguyên/lặp cờ ⇒ exit **2** + thông điệp/usage nêu
//!   `--slowest`; `--slowest` + `--list` ⇒ exit **2** (không có thời gian chạy).
//! - Đo **thời gian tường** từng **đơn vị đã chạy** (ca × lần lặp `--repeat`; `--fail-fast` chỉ
//!   đơn vị đã chạy). Sau TAP/JSON (stdout **không đổi**), in tối đa `N` dòng ra stderr, **giảm
//!   dần**; hoà giữ **thứ tự gốc**. Dạng: `# slowest: <ms> ms - <tên>`, dưới 1 ms dùng
//!   `# slowest: <us> us - <tên>`.
//! - `--jobs`/`--repeat`/`--shard`/`--filter`/`--shuffle` compose; `--slowest` áp **sau** khi đã
//!   có danh sách đơn vị.
//!
//! **RED-BEFORE**: chạy tệp này trên HEAD `e229c63` (trước khi thêm cờ) cho
//! **0 xanh / 7 đỏ** — mọi lệnh `vis test --slowest …` in `vis test: không hỗ trợ cờ
//! '--slowest'` (kèm usage cũ) và exit **2**; `--slowest` + `--list` cũng exit 2 nhưng vì cờ
//! lạ, còn ca usage đỏ vì usage chưa nêu `--slowest`. Ca chậm dùng busy loop
//! `time.monotonic_ms()` ~1000 ms; ca nhanh chỉ `assert true` ⇒ chênh lệch lớn (~1000 ms vs
//! ~1–5 ms cả thời gian spawn), không phụ thuộc tốc độ CPU.
//!
//! **ST-296/VD-282(3) — chịu tải, không `#[ignore]`.** Đo thật dưới load **36** (soak 22/11):
//! `vis test` spawn ca nhanh mất tối đa **74 ms**; ca chậm 50 ms cũ có thể bị vượt ⇒ flake.
//! Nay: ca chậm **1000 ms** (≥ 13× fast đo được), ngưỡng **800 ms** (80% nominal, vẫn ≥ 10× fast);
//! **đảo thứ tự nguồn** (`a_nhanh` trước `b_cham`) để bắt regression “giữ thứ tự nguồn thay vì
//! sắp theo thời gian”; **warmup** một lượt không cờ; ca `--repeat` lấy **trung vị 3 lần** và đòi
//! ≥ 2/3 lần đúng thứ tự (nhiễu lẻ không che regression). Vẫn kiểm đủ: đơn vị `[k/N]`, stdout
//! không đổi, giảm dần, `--filter`/`--shard`/`--slowest` compose.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

fn tam(tag: &str) -> PathBuf {
    static DEM: AtomicUsize = AtomicUsize::new(0);
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!(
        "vis-vd270-slowest-{tag}-{}-{n}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("thư mục tạm");
    d
}

fn ghi(d: &Path, ten: &str, src: &str) {
    std::fs::write(d.join(ten), src).expect("ghi nguồn");
}

fn vis(args: &[&str]) -> (i32, String, String) {
    let o = Command::new(env!("CARGO_BIN_EXE_vis"))
        .args(args)
        .output()
        .expect("chạy vis");
    (
        o.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&o.stdout).to_string(),
        String::from_utf8_lossy(&o.stderr).to_string(),
    )
}

/// Ca chậm **tất định tương đối**: busy loop tới khi `monotonic_ms` đi qua ~1000 ms. Không dùng
/// `sleep` (stdlib chưa có) và không phụ thuộc tốc độ CPU — chỉ cần đồng hồ thật.
const CHAM: &str = r#"use std.time
test fn cham():
    let t0 = time.monotonic_ms()
    var x = 0
    while time.monotonic_ms() - t0 < 1000:
        x += 1
    assert x > 0
"#;

/// Ca nhanh: một `assert true`, chắc chắn nhanh hơn hẳn ca chậm.
const NHANH: &str = r#"test fn nhanh():
    assert true
"#;

/// Bộ 2 ca **nhanh trước, chậm sau** (`a_nhanh.vis`, `b_cham.vis` — `gom` sắp theo đường dẫn).
/// Thứ tự nguồn **ngược** thứ tự thời gian ⇒ nếu `--slowest` không thực sự sắp theo thời gian,
/// test đỏ (trước đây đặt `cham` trước nên không phân biệt được “sắp” với “giữ nguyên nguồn”).
/// Chạy **warmup** một lượt không cờ (bỏ kết quả) để nạp trang/binary trước khi đo.
fn bo_hai_ca(d: &Path) -> String {
    ghi(d, "a_nhanh.vis", NHANH);
    ghi(d, "b_cham.vis", CHAM);
    let ds = d.to_str().unwrap().to_string();
    let _ = vis(&["test", &ds]);
    ds
}

/// Mọi dòng slowest trong stderr (giữ nguyên chuỗi).
fn dong_slowest(stderr: &str) -> Vec<String> {
    stderr
        .lines()
        .filter(|l| l.starts_with("# slowest: "))
        .map(str::to_string)
        .collect()
}

/// Tách `# slowest: <số> <ms|us> - <tên>` ⇒ `(micro giây, tên)`.
fn tach_dong(dong: &str) -> (u128, &str) {
    let rest = dong
        .strip_prefix("# slowest: ")
        .unwrap_or_else(|| panic!("dòng không đúng dạng slowest: {dong:?}"));
    let (tg, ten) = rest
        .split_once(" - ")
        .unwrap_or_else(|| panic!("thiếu ` - <tên>`: {dong:?}"));
    let (so, don_vi) = tg
        .split_once(' ')
        .unwrap_or_else(|| panic!("thiếu đơn vị thời gian: {dong:?}"));
    let n: u128 = so
        .parse()
        .unwrap_or_else(|_| panic!("số thời gian không nguyên: {dong:?}"));
    let us = match don_vi {
        "ms" => n * 1000,
        "us" => n,
        khac => panic!("đơn vị lạ `{khac}`: {dong:?}"),
    };
    (us, ten)
}

/// Trung vị (sắp tại chỗ); vector không rỗng theo mọi lời gọi.
fn trung_vi(v: &mut [u128]) -> u128 {
    v.sort_unstable();
    v[v.len() / 2]
}

/// (1) `--slowest 1`: đúng **một** dòng, đúng ca chậm; stdout y hệt không cờ; dạng thời gian
/// hợp lệ; không cờ ⇒ stderr không có dòng slowest.
#[test]
fn slowest_1_chi_ca_cham() {
    let d = tam("mot");
    let ds = bo_hai_ca(&d);
    let ds = ds.as_str();

    let (c0, o0, e0) = vis(&["test", ds]);
    assert_eq!(c0, 0, "{o0} {e0}");
    assert!(
        dong_slowest(&e0).is_empty(),
        "không cờ ⇒ không có dòng slowest: {e0}"
    );

    let (c1, o1, e1) = vis(&["test", "--slowest", "1", ds]);
    assert_eq!(c1, 0, "--slowest 1: {o1} {e1}");
    assert_eq!(o1, o0, "--slowest không được đổi stdout (TAP)");

    let ds_dong = dong_slowest(&e1);
    assert_eq!(ds_dong.len(), 1, "đúng 1 dòng slowest: {e1}");
    let (us, ten) = tach_dong(&ds_dong[0]);
    assert_eq!(ten, "cham", "ca chậm nhất phải là `cham`: {e1}");
    assert!(us >= 800_000, "ca chậm phải ≥ 800 ms, được {us} us: {e1}");

    // Dạng `--slowest=N` tương đương.
    let (cb, ob, eb) = vis(&["test", "--slowest=1", ds]);
    assert_eq!(cb, 0, "--slowest=1: {ob} {eb}");
    assert_eq!(ob, o0);
    assert_eq!(dong_slowest(&eb).len(), 1, "{eb}");

    let _ = std::fs::remove_dir_all(&d);
}

/// (2) `--slowest 2`: hai dòng, **giảm dần** theo thời gian, `cham` trước `nhanh`.
#[test]
fn slowest_2_giam_dan() {
    let d = tam("hai");
    let ds = bo_hai_ca(&d);
    let ds = ds.as_str();

    let (c, o, e) = vis(&["test", "--slowest", "2", ds]);
    assert_eq!(c, 0, "{o} {e}");
    let ds_dong = dong_slowest(&e);
    assert_eq!(ds_dong.len(), 2, "đúng 2 dòng slowest: {e}");
    let (us0, ten0) = tach_dong(&ds_dong[0]);
    let (us1, ten1) = tach_dong(&ds_dong[1]);
    assert_eq!(ten0, "cham", "dòng đầu phải là ca chậm: {e}");
    assert_eq!(ten1, "nhanh", "dòng sau phải là ca nhanh: {e}");
    assert!(us0 >= us1, "phải sắp giảm dần: {us0} us < {us1} us: {e}");
    assert!(us0 >= 800_000, "ca chậm ≥ 800 ms: {e}");
    assert!(us1 < us0, "ca nhanh phải nhanh hơn hẳn: {e}");

    let _ = std::fs::remove_dir_all(&d);
}

/// (3) `N` lớn hơn số ca ⇒ in **hết** số đơn vị đã chạy, không lỗi.
#[test]
fn slowest_lon_hon_so_ca_in_het() {
    let d = tam("het");
    let ds = bo_hai_ca(&d);
    let ds = ds.as_str();

    let (c, o, e) = vis(&["test", "--slowest", "100", ds]);
    assert_eq!(c, 0, "{o} {e}");
    let ds_dong = dong_slowest(&e);
    assert_eq!(ds_dong.len(), 2, "chỉ có 2 ca nên in 2 dòng: {e}");
    let mut tens: Vec<&str> = ds_dong.iter().map(|x| tach_dong(x).1).collect();
    tens.sort();
    assert_eq!(tens, ["cham", "nhanh"], "{e}");

    let _ = std::fs::remove_dir_all(&d);
}

/// (4) `--slowest` + `--list` ⇒ exit 2; N sai (thiếu/0/âm/không nguyên/lặp) ⇒ exit 2; stderr
/// nêu `--slowest`; usage (test + gốc) cũng nêu cờ.
#[test]
fn slowest_list_va_sai_exit_2() {
    let d = tam("sai");
    let ds = bo_hai_ca(&d);
    let ds = ds.as_str();

    let (cl, ol, el) = vis(&["test", "--list", "--slowest", "1", ds]);
    assert_eq!(cl, 2, "--slowest + --list phải exit 2: {ol} {el}");
    assert!(el.contains("--slowest"), "stderr phải nêu --slowest: {el}");
    assert!(ol.is_empty(), "không được in danh sách: {ol}");

    let thieu = vec!["test", "--slowest"];
    let khong = vec!["test", "--slowest", "0", ds];
    let am = vec!["test", "--slowest", "-2", ds];
    let khong_nguyen = vec!["test", "--slowest", "1.5", ds];
    let chu = vec!["test", "--slowest", "abc", ds];
    let lap = vec!["test", "--slowest", "1", "--slowest", "2", ds];
    let lap_bang = vec!["test", "--slowest=1", "--slowest=2", ds];
    for args in [thieu, khong, am, khong_nguyen, chu, lap, lap_bang] {
        let (c, _o, e) = vis(&args);
        assert_eq!(c, 2, "{args:?}: {e}");
        assert!(e.contains("--slowest"), "{args:?}: {e}");
    }

    let (ch, _oh, eh) = vis(&["test", "--help"]);
    assert_eq!(ch, 2, "--help là lỗi dùng");
    assert!(
        eh.contains("--slowest"),
        "usage test phải nêu --slowest: {eh}"
    );
    let (cr, _or, er) = vis(&[]);
    assert_eq!(cr, 2);
    assert!(
        er.contains("--slowest"),
        "usage gốc phải nêu --slowest: {er}"
    );

    let _ = std::fs::remove_dir_all(&d);
}

/// (5) `--repeat 2`: mỗi lần lặp là **một đơn vị** riêng, tên `[k/N]`; in đủ 4 dòng, hai ca chậm
/// đứng trước hai ca nhanh; stdout y hệt không cờ.
#[test]
fn slowest_repeat_2_ten_k_tren_n() {
    let d = tam("repeat");
    let ds = bo_hai_ca(&d);
    let ds = ds.as_str();

    let (c0, o0, e0) = vis(&["test", "--repeat", "2", ds]);
    assert_eq!(c0, 0, "{o0} {e0}");

    // **Trung vị 3 lần**: mỗi lần phải đủ 4 đơn vị `[k/N]`, stdout y hệt; đòi **≥ 2/3** lần
    // đúng thứ tự (một nhiễu lẻ không che regression), và trung vị `cham` > trung vị `nhanh`
    // ít nhất 300 ms. Ca chậm 1000 ms nên khoảng cách này rộng gấp >13× fast đo được (74 ms).
    let mut dem_dung = 0usize;
    let mut cham_us: Vec<u128> = Vec::new();
    let mut nhanh_us: Vec<u128> = Vec::new();
    for lan in 0..3 {
        let (c, o, e) = vis(&["test", "--repeat", "2", "--slowest", "4", ds]);
        assert_eq!(c, 0, "lần {lan}: {o} {e}");
        assert_eq!(o, o0, "lần {lan}: --slowest không được đổi stdout (TAP)");
        let ds_dong = dong_slowest(&e);
        assert_eq!(ds_dong.len(), 4, "lần {lan}: 4 đơn vị (2 ca × 2 lần): {e}");
        let mut tens: Vec<&str> = ds_dong.iter().map(|x| tach_dong(x).1).collect();
        tens.sort();
        assert_eq!(
            tens,
            ["cham [1/2]", "cham [2/2]", "nhanh [1/2]", "nhanh [2/2]"],
            "lần {lan}: mỗi lần lặp phải là đơn vị riêng có hậu tố `[k/N]`: {e}"
        );
        if ds_dong[..2]
            .iter()
            .all(|x| tach_dong(x).1.starts_with("cham ["))
        {
            dem_dung += 1;
        }
        for dong in &ds_dong {
            let (us, ten) = tach_dong(dong);
            if ten.starts_with("cham [") {
                assert!(us >= 800_000, "lần {lan}: ca chậm ≥ 800 ms: {dong}");
                cham_us.push(us);
            } else if ten.starts_with("nhanh [") {
                assert!(us < 800_000, "lần {lan}: ca nhanh phải « 800 ms: {dong}");
                nhanh_us.push(us);
            } else {
                panic!("lần {lan}: tên lạ: {dong}");
            }
        }
    }
    assert!(
        dem_dung >= 2,
        "≥ 2/3 lần hai dòng đầu phải là ca chậm (thực tế {dem_dung}/3)"
    );
    let mc = trung_vi(&mut cham_us);
    let mn = trung_vi(&mut nhanh_us);
    assert!(mc >= 800_000, "trung vị ca chậm ≥ 800 ms: {mc} us");
    assert!(
        mc > mn + 300_000,
        "trung vị ca chậm phải > ca nhanh +300 ms: {mc} vs {mn} us"
    );
    println!("ST296-SLOWEST dem_dung={dem_dung}/3 trung_vi_cham={mc}us trung_vi_nhanh={mn}us");

    let _ = std::fs::remove_dir_all(&d);
}

/// (6) stdout JSON y hệt không cờ; slowest chỉ ra stderr.
#[test]
fn slowest_json_stdout_khong_doi() {
    let d = tam("json");
    let ds = bo_hai_ca(&d);
    let ds = ds.as_str();

    let (c0, o0, _e0) = vis(&["test", "--json", ds]);
    assert_eq!(c0, 0, "{o0}");
    let (c1, o1, e1) = vis(&["test", "--json", "--slowest", "2", ds]);
    assert_eq!(c1, 0, "{o1} {e1}");
    assert_eq!(o1, o0, "--slowest không được đổi JSON stdout");
    assert!(o1.starts_with('{') && o1.ends_with("}\n"), "{o1}");
    assert_eq!(dong_slowest(&e1).len(), 2, "{e1}");

    let _ = std::fs::remove_dir_all(&d);
}

/// (7) Compose: `--filter`/`--shard` quyết định danh sách đơn vị trước; `--shuffle`/`--jobs`
/// không đổi tập stdout; `--slowest` áp **sau** (chỉ đơn vị thực sự đã chạy).
#[test]
fn slowest_compose_filter_shard_shuffle_jobs() {
    let d = tam("compose");
    let ds = bo_hai_ca(&d);
    let ds = ds.as_str();

    // `--filter nhanh` ⇒ chỉ còn 1 đơn vị; dù N lớn cũng chỉ 1 dòng.
    let (cf, of, ef) = vis(&["test", "--filter", "nhanh", "--slowest", "9", ds]);
    assert_eq!(cf, 0, "{of} {ef}");
    let df = dong_slowest(&ef);
    assert_eq!(df.len(), 1, "filter phải áp trước slowest: {ef}");
    assert_eq!(tach_dong(&df[0]).1, "nhanh", "{ef}");

    // `--shard 2/2` chọn ca thứ 2 (`b_cham`, vì `a_nhanh` đứng trước) ⇒ slowest chỉ ca đó.
    let (cs, os, es) = vis(&["test", "--shard", "2/2", "--slowest", "5", ds]);
    assert_eq!(cs, 0, "{os} {es}");
    let dss = dong_slowest(&es);
    assert_eq!(dss.len(), 1, "shard phải áp trước slowest: {es}");
    assert_eq!(tach_dong(&dss[0]).1, "cham", "{es}");

    // `--shuffle --seed` + `--jobs 2`: stdout giống hệt lần chạy không slowest; stderr vẫn đủ
    // 2 dòng giảm dần (slowest áp sau khi đã có đơn vị).
    let tham = ["test", "--shuffle", "--seed", "42", "--jobs", "2", ds];
    let (c0, o0, e0) = vis(&tham);
    assert_eq!(c0, 0, "{o0} {e0}");
    let mut tham_slow = tham.to_vec();
    tham_slow.push("--slowest");
    tham_slow.push("2");
    let (c1, o1, e1) = vis(&tham_slow);
    assert_eq!(c1, 0, "{o1} {e1}");
    assert_eq!(o1, o0, "shuffle+jobs+slowest không được đổi stdout");
    let d1 = dong_slowest(&e1);
    assert_eq!(d1.len(), 2, "{e1}");
    let (u0, t0) = tach_dong(&d1[0]);
    let (u1, t1) = tach_dong(&d1[1]);
    assert_eq!((t0, t1), ("cham", "nhanh"), "{e1}");
    assert!(u0 >= u1, "{e1}");

    let _ = std::fs::remove_dir_all(&d);
}
