// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-14 — harness ngữ nghĩa (TAP) chạy qua binary `vis run`.**
//!
//! Mỗi ca: `test/sem/<nhóm>/<ca>.vis` + **hoặc** `<ca>.out` (stdout kỳ vọng, so **từng byte**)
//! **hoặc** `<ca>.err` (mã lỗi kỳ vọng: `VS05xx` ⇒ exit **101** (C1/R9), `0xxx` ⇒ exit 2).
//!
//! Chống xanh-giả: (i) so **byte** stdout, không cắt/normalize; (ii) đọc `steps=N` từ TAP
//! (`vis run --tap`) và **đòi `N > 0`** cho ca chạy được — ca không chạy bước nào mà báo `ok` là
//! **SAI**; (iii) khoá **đúng số ca tìm thấy** (100) + mỗi nhóm ≥ 6 ca, để không âm thầm bỏ ca.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Bỏ **dòng SPDX đầu** của tệp kỳ vọng (QD §9.3 yêu cầu SPDX cho mọi tệp văn bản); phần còn lại
/// vẫn được so **từng byte** với stdout.
///
/// PATH có thêm thư mục chứa `test-child` — cho fixture `.vish` gọi `test-child` bare.
fn path_voi_tc() -> std::ffi::OsString {
    let tc = std::path::Path::new(env!("CARGO_BIN_EXE_test-child"));
    let mut muc: Vec<std::path::PathBuf> =
        vec![tc.parent().expect("test-child parent").to_path_buf()];
    let cur = std::env::var_os("PATH").unwrap_or_default();
    for p in std::env::split_paths(&cur) {
        muc.push(p);
    }
    std::env::join_paths(muc).expect("join PATH")
}

fn boc_spdx(t: &str) -> String {
    match t.strip_prefix("# SPDX-License-Identifier") {
        Some(rest) => match rest.find('\n') {
            Some(i) => rest[i + 1..].to_string(),
            None => String::new(),
        },
        None => t.to_string(),
    }
}

fn vis() -> &'static str {
    env!("CARGO_BIN_EXE_vis")
}

fn sem_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("test")
        .join("sem")
}

struct Ca {
    ten: String,
    nhom: String,
    vis: PathBuf,
    out: Option<String>,
    err: Option<String>,
}

fn gom_ca() -> Vec<Ca> {
    let goc = sem_dir();
    let mut ds = Vec::new();
    let mut nhom_dir: Vec<PathBuf> = std::fs::read_dir(&goc)
        .unwrap_or_else(|e| panic!("không đọc được {}: {e}", goc.display()))
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_dir())
        .collect();
    nhom_dir.sort();
    for d in nhom_dir {
        let nhom = d.file_name().unwrap().to_string_lossy().to_string();
        let mut ca: Vec<PathBuf> = std::fs::read_dir(&d)
            .unwrap_or_else(|e| panic!("không đọc được {}: {e}", d.display()))
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| {
                p.extension()
                    .map(|x| x == "vis" || x == "vish")
                    .unwrap_or(false)
            })
            .collect();
        ca.sort();
        for f in ca {
            let stem = f.with_extension("");
            let out = std::fs::read_to_string(stem.with_extension("out"))
                .ok()
                .map(|t| boc_spdx(&t));
            let err = std::fs::read_to_string(stem.with_extension("err"))
                .ok()
                .map(|t| boc_spdx(&t));
            assert!(
                out.is_some() ^ err.is_some(),
                "{}: phải có **đúng một** trong `.out`/`.err`",
                f.display()
            );
            ds.push(Ca {
                ten: f.file_stem().unwrap().to_string_lossy().to_string(),
                nhom: nhom.clone(),
                vis: f,
                out,
                err,
            });
        }
    }
    ds
}

/// Bóc `steps=N` từ stderr TAP.
fn steps_cua(stderr: &str) -> usize {
    for line in stderr.lines() {
        if let Some(i) = line.find("steps=") {
            let rest = &line[i + "steps=".len()..];
            let num: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
            if let Ok(n) = num.parse() {
                return n;
            }
        }
    }
    0
}

#[test]
fn sem_323_ca_qua_binary() {
    let ds = gom_ca();
    let mut theo_nhom: BTreeMap<String, usize> = BTreeMap::new();
    for c in &ds {
        *theo_nhom.entry(c.nhom.clone()).or_default() += 1;
    }
    // ST-15: khoá **ca / nhóm**, mỗi nhóm ≥ 6 — thiếu/thừa/thiếu nhóm là ĐỎ.
    // ST-18: 309 `.vis` + 14 `.vish` (nhóm `vish`, chế độ lệnh) ⇒ **323 ca / 17 nhóm**.
    // **SỬA KÈM LÝ DO (ST-35)**: +8 ca `generic/*` (instantiation thật: `Hop[int]` ≠ `Hop[str]`,
    // `Or[int, str]`, hàm liên kết) ⇒ **346 ca / 18 nhóm** (ST-28 trước: 338).
    // **SỬA KÈM LÝ DO (ST-37)**: +3 ca `vish/khe-*.vish` (khe ống hợp nhất: `( [1,2,3] ) | take 2`,
    // …) ⇒ **349 ca / 18 nhóm** — xếp vào nhóm `vish` để **không** tạo nhóm < 6 ca.
    // **SỬA KÈM LÝ DO (ST-38)**: +6 ca `vish/khe-*` (closure-arg: `where`/`each`/`first`/`count`) ⇒ **355 ca / 18 nhóm**.
    // **SỬA KÈM LÝ DO (ST-56)**: +6 ca (`ham/ref-*`, `ban-ghi/self-amp*`, `generic/hop-long-*`)
    // ⇒ **376 ca / 18 nhóm** (333 `.vis` + 43 `.vish`, **55** `.err`).
    // **SỬA KÈM LÝ DO (ST-56b)**: +6 ca phép gán (`ban-ghi/self-gan-truong`, `var-self-gan`,
    // `var-truong-gan`; `list/var-chi-so-gan`, `list/let-chi-so-gan`; `map/var-khoa-gan`)
    // ⇒ **382 ca / 18 nhóm** (339 `.vis` + 43 `.vish`, **57** `.err`).
    // **SỬA KÈM LÝ DO (ST-57)**: +6 ca nhóm **`trait/`** (siêu trait ×2, `derives` ×4) ⇒ **388 ca /
    // **19 nhóm** (345 `.vis` + 43 `.vish`, **59** `.err`).
    assert_eq!(
        ds.len(),
        // **SỬA KÈM LÝ DO (ST-60)**: +**22** ca nhóm **mới `async/`** ⇒ **416 ca / 20 nhóm** (367
        // `.vis` + 49 `.vish`, **69** `.err`): await cơ bản/thứ tự/cache, task in, spawn + join cuối,
        // task trả record/list/str, đệ quy async, async gọi async, và 7 ca **âm** (VS0412 ×4, VS0201,
        // VS0506 ×2).
        // **SỬA KÈM LÝ DO (ST-60b)**: +**3** ca (2 `.err` trong `loi/`: gán lồng gốc `var`, gán lồng
        // gốc `let`; 1 xanh trong `vish/`: thân `cmd` khai báo dùng **biến** vừa khai) ⇒ **419 ca /
        // 20 nhóm** (367 `.vis` + 50 `.vish`, **71** `.err`). (`std.* --tên` khoá ở Rust
        // `gan_hai_duong::std_khong_nhan_tham_so_ten` — HIR `{error}` không sinh bytecode hỏng.)
        // **SỬA KÈM LÝ DO (ST-61)**: +**15** ca nhóm **mới** `kenh/` (tạo/send/recv/FIFO, deadlock
        // `VS0505` không treo, gửi sau `close` `VS0504`, sai kiểu `VS0412`, spawn có đệm, `chan` làm
        // tên người dùng) ⇒ **441 ca / 21 nhóm** (367 `.vis` + 50 `.vish`, **80** `.err`).
        // **SỬA KÈM LÝ DO (ST-60c)**: +**7** ca nhóm `trait/` (impl sinh từ `derives` vào
        // `impl_pairs`; `derives Show` cho enum **sinh mã thật**; `Eq` chặn **sâu** list/map/opt) ⇒
        // **441 ca / 21 nhóm** (367 `.vis` + 50 `.vish`, **80** `.err`).
        // **SỬA KÈM LÝ DO (ST-72)**: +**6** ca nhóm **mới `cap/`** (cap[Dir]/cap[Net]/`@`/`(write)`
        // hợp lệ ×3; sai `@` trên int, `(write)` trên Net, so sánh/in cap ×3) ⇒ **447 ca / 22 nhóm**
        // (397 `.vis` + 50 `.vish`, **83** `.err`).
        // **SỬA KÈM LÝ DO (R9, VD-178)**: +**1** ca `loi/range-khong-int` (`VS0416`) ⇒ **448 ca**
        // (398 `.vis` + 50 `.vish`, **84** `.err`).
        // **SỬA KÈM LÝ DO (R6, VD-178)**: +**4** ca `chuoi/` (`cong-noi`, `in-nhieu`, `spec`,
        // `p-khong-escape`) ⇒ **452 ca** (402 `.vis` + 50 `.vish`, **84** `.err`).
        // **SỬA KÈM LÝ DO (R17, VD-178)**: +**1** ca `dieu-khien/is-empty` ⇒ **453 ca**
        // (403 `.vis` + 50 `.vish`, **84** `.err`).
        // **SỬA KÈM LÝ DO (R3b, VD-178)**: +**1** ca `result/let-else` ⇒ **454 ca**
        // (404 `.vis` + 50 `.vish`, **84** `.err`).
        // **SỬA KÈM LÝ DO (R3a, VD-178)**: +**1** ca `result/if-let-pattern` ⇒ **455 ca**
        // (405 `.vis` + 50 `.vish`, **84** `.err`).
        // **SỬA KÈM LÝ DO (R16, VD-178)**: +**1** ca `pham-vi/che-ten-let` ⇒ **456 ca**
        // (406 `.vis` + 50 `.vish`, **84** `.err`).
        // **SỬA KÈM LÝ DO (R12a, VD-178)**: +**1** ca `kieu/type-alias-newtype` ⇒ **457 ca**
        // (407 `.vis` + 50 `.vish`, **84** `.err`).
        // **SỬA KÈM LÝ DO (R11, VD-178)**: +**1** ca `enum/vi-tri-va-dung-ten` ⇒ **458 ca**
        // (408 `.vis` + 50 `.vish`, **84** `.err`).
        // **SỬA KÈM LÝ DO (R7, VD-178)**: +**1** ca `so-hoc/chia-nguyen` ⇒ **459 ca**
        // (409 `.vis` + 50 `.vish`, **84** `.err`).
        // **SỬA KÈM LÝ DO (R8, VD-178)**: +**1** ca `list/chi-so-an-toan` ⇒ **460 ca**
        // (410 `.vis` + 50 `.vish`, **84** `.err`).
        // **SỬA KÈM LÝ DO (R5a, VD-178)**: +**1** ca `ham/goi-ten` ⇒ **461 ca**
        // (411 `.vis` + 50 `.vish`, **84** `.err`).
        // **SỬA KÈM LÝ DO (R5c, VD-178)**: +**1** ca `chuoi-phuong-thuc/std-method` ⇒ **462 ca**
        // (412 `.vis` + 50 `.vish`, **84** `.err`).
        // **SỬA KÈM LÝ DO (R20a, VD-178)**: +**1** ca `loi/assert-thong-diep` ⇒ **463 ca**
        // (413 `.vis` + 50 `.vish`, **84** `.err`).
        // **SỬA KÈM LÝ DO (R20b, VD-178)**: +**1** ca `list/duyet-nang-cao` ⇒ **464 ca**
        // (414 `.vis` + 50 `.vish`, **84** `.err`).
        // **SỬA KÈM LÝ DO (R19b, VD-178)**: +**1** ca `loi/doi-dau-tran` (`VS0507`) ⇒ **465 ca**
        // (415 `.vis` + 50 `.vish`, **85** `.err`).
        // **SỬA KÈM LÝ DO (R2, VD-178)**: +**3** ca `optional/` (int-qhap-gan, map-get-lot,
        // t-qhap-generic) + **1** ca `loi/none-bo` (`VS0203`) ⇒ **469 ca**
        // (419 `.vis` + 50 `.vish`, **86** `.err`).
        // **SỬA KÈM LÝ DO (VD-240-b, 2026-11-19)**: +**4** ca nhóm `async/` (**grid VD-240**:
        // `grid-await-nghich-thu-tu`, `grid-chan-hai-task-kenh`, `grid-await-if`, `grid-kenh-list`)
        // ⇒ **473 ca** (423 `.vis` + 50 `.vish`, **86** `.err`). Grid pin thứ tự thực thi
        // run-to-completion một luồng: await quyết định thứ tự chạy; kênh FIFO; byte-identical hai engine.
        // **SỬA KÈM LÝ DO (VD-241 §3, 2026-11-19)**: +**10** ca `optional/long-*` + `ban-ghi/field-optional`
        //   (9 Option lồng xanh + 1 `long-no-pseudo` err) ⇒ **483 ca**
        //   (**433 `.vis`** + 50 `.vish`, **89 `.err`**).
        // **SỬA KÈM LÝ DO (VD-241 §7, 2026-11-19)**: +**4** ca xanh `optional/` (`long-reject-e` đổi thành
        //   `long-none-e` **xanh** — `None` hợp lệ khi lớp **ngoài cùng** là `Option`, VD-238 sửa theo lệnh A0;
        //   +`none-three-states`, `long-none-param`, `long-none-return`) ⇒ **486 ca**
        //   (**436 `.vis`** + 50 `.vish`, **88 `.err`**).
        // **SỬA KÈM LÝ DO (VD-241 đợt coercion, 2026-11-19)**: +1 xanh `optional/long-option-matrix.vis`
        //   (ma trận Option lồng) ⇒ **487 ca** (**437 `.vis`** + 50 `.vish`, **88 `.err`**).
        // **SỬA KÈM LÝ DO (VD-241 Gate 1, 2026-11-19)**: +**27** ca nhóm mới **`sqlite/`** (19 xanh,
        //   8 `.err` VS0501) ⇒ **514 ca / 23 nhóm** (**464 `.vis`** + 50 `.vish`, **96 `.err`**).
        // **SỬA KÈM LÝ DO (VD-241 pre-merge, 2026-11-19)**: +**11** ca `sqlite/` (`execute-tra-hang`,
        //   `bind-inf`, ma trận authorizer 9 ca); mọi lỗi dữ liệu nay là `Result.Err` ⇒ 8 `.err` cũ đổi thành `.out`;
        //   nhóm `sqlite/` không còn `.err` ⇒ **525 ca / 23 nhóm** (**475 `.vis`** + 50 `.vish`, **88 `.err`**).
        525,
        "phải đúng 525 ca (475 `.vis` + 50 `.vish`), thật {} — nhóm: {theo_nhom:?}",
        ds.len()
    );
    // **SỬA KÈM LÝ DO (ST-57)**: nhóm mới **`trait/`** ⇒ **19 nhóm**.
    // **SỬA KÈM LÝ DO (ST-60)**: nhóm mới **`async/`** ⇒ **20 nhóm**.
    // **SỬA KÈM LÝ DO (ST-61)**: nhóm mới **`kenh/`** ⇒ **21 nhóm**.
    // **SỬA KÈM LÝ DO (ST-72)**: nhóm mới **`cap/`** ⇒ **22 nhóm**.
    assert_eq!(
        theo_nhom.len(),
        23,
        "phải đúng 23 nhóm (ST-28 `generic`; ST-57 `trait`; ST-60 `async`; ST-61 `kenh`; ST-72 `cap`; VD-241 Gate 1 `sqlite`), thật {theo_nhom:?}"
    );
    // **Chống bỏ ca**: số tệp `.vis` trên đĩa phải **bằng** số ca đã gom (không ca nào bị lọc mất).
    let tren_dia: usize = std::fs::read_dir(sem_dir())
        .expect("đọc test/sem")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_dir())
        .map(|d| {
            std::fs::read_dir(d)
                .expect("đọc nhóm")
                .filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| {
                    p.extension()
                        .map(|x| x == "vis" || x == "vish")
                        .unwrap_or(false)
                })
                .count()
        })
        .sum();
    assert_eq!(
        tren_dia,
        ds.len(),
        "số tệp `.vis`/`.vish` ({tren_dia}) phải bằng số ca chạy ({})",
        ds.len()
    );
    for (nhom, n) in &theo_nhom {
        assert!(*n >= 6, "nhóm `{nhom}` chỉ có {n} ca (cần ≥ 6)");
    }

    let mut sai = Vec::new();
    for c in &ds {
        let res = Command::new(vis())
            .env("PATH", path_voi_tc())
            .arg("run")
            .arg("--tap")
            .arg(&c.vis)
            .output()
            .unwrap_or_else(|e| panic!("không chạy được `vis run`: {e}"));
        let stdout = String::from_utf8_lossy(&res.stdout).to_string();
        let stderr = String::from_utf8_lossy(&res.stderr).to_string();
        let code = res.status.code().unwrap_or(-1);
        let mo_ta = format!("{}/{}", c.nhom, c.ten);
        match (&c.out, &c.err) {
            (Some(want), None) => {
                if code != 0 || stdout != *want {
                    sai.push(format!(
                        "{mo_ta}: muốn exit 0 + stdout {want:?}, được exit {code} + stdout {stdout:?}\n  stderr: {stderr}"
                    ));
                    continue;
                }
                if steps_cua(&stderr) == 0 {
                    sai.push(format!(
                        "{mo_ta}: **chưa chạy bước nào** (steps=0) mà báo ok"
                    ));
                }
            }
            (None, Some(ma)) => {
                let ma = ma.trim();
                let muon_exit = if ma.starts_with("VS05") { 101 } else { 2 };
                if code != muon_exit || !stderr.contains(ma) {
                    sai.push(format!(
                        "{mo_ta}: muốn exit {muon_exit} + stderr chứa `{ma}`, được exit {code}\n  stderr: {stderr}"
                    ));
                    continue;
                }
                // Lỗi runtime thì chương trình **đã chạy** ít nhất một bước.
                if ma.starts_with("VS05") && steps_cua(&stderr) == 0 {
                    sai.push(format!("{mo_ta}: lỗi runtime mà steps=0 (chưa chạy gì)"));
                }
            }
            _ => unreachable!(),
        }
    }
    assert!(
        sai.is_empty(),
        "{}/{} ca sai:\n{}",
        sai.len(),
        ds.len(),
        sai.join("\n")
    );
}
