// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-313b (VD-289) / ST-317c** — test song ánh 1–1 giữa bảng registry `std.*` (nguồn duy nhất)
//! và `StdId`/dispatcher của `vis-runtime`.

use std::collections::BTreeSet;

use vis_hir::stdlib::{StdId, FUNCS};

#[test]
fn std_id_phu_dung_funcs_va_khong_trung() {
    assert_eq!(
        StdId::COUNT,
        FUNCS.len(),
        "StdId::COUNT phải bằng FUNCS.len()"
    );
    let mut da_co = BTreeSet::new();
    for f in FUNCS.iter() {
        let id = StdId::from_name(f.name)
            .unwrap_or_else(|| panic!("FUNCS có `{}` nhưng StdId thiếu", f.name));
        assert_eq!(id.as_str(), f.name, "as_str phải trả đúng tên");
        assert_eq!(id.index(), id as usize);
        assert_eq!(
            f.name.split_once('.').map(|(m, _)| m),
            Some(id.module()),
            "metadata module phải khớp tên đầy đủ"
        );
        assert!(da_co.insert(id), "`{}` trùng id {:?}", f.name, id);
    }
    assert_eq!(
        da_co.len(),
        StdId::COUNT,
        "phải phủ đúng COUNT id, không thừa/thiếu"
    );
}

#[test]
fn std_id_all_dung_thu_tu_khai_va_duy_nhat() {
    assert_eq!(StdId::ALL.len(), StdId::COUNT);
    for (i, id) in StdId::ALL.iter().enumerate() {
        assert_eq!(id.index(), i, "ALL[{i}] phải có index {i}");
        assert_eq!(StdId::from_name(id.as_str()), Some(*id));
    }
    let ten: BTreeSet<&'static str> = StdId::ALL.iter().map(|i| i.as_str()).collect();
    let funcs: BTreeSet<&'static str> = FUNCS.iter().map(|f| f.name).collect();
    assert_eq!(ten, funcs, "tập tên StdId và FUNCS phải trùng khít");
}

/// Quét khối `match id` của `call_by_id` trong mã nguồn: trả `(số nhánh, cờ phủ theo `ten`)`.
///
/// Lỗi khi gặp nhánh bắt-tất-cả `_ =>` ở cấp 1 (cấm — xem [`std_id_dem_that_va_dispatcher_khong_co_nhanh_bat_tat_ca`]).
fn quet_dispatcher(src: &str, ten: &[&str]) -> Result<(usize, Vec<bool>), String> {
    let mut trong_fn = false;
    let mut khop = vec![false; ten.len()];
    let mut so_nhanh = 0usize;
    for line in src.lines() {
        if line.starts_with("pub fn call_by_id") {
            trong_fn = true;
            continue;
        }
        if !trong_fn {
            continue;
        }
        // Hết hàm: `}` cột 0 (rustfmt; thân hàm thụt ≥ 4).
        if line == "}" {
            break;
        }
        let t = line.trim_start();
        let thut = line.len() - t.len();
        if thut == 8 && t.starts_with('_') && t.contains("=>") {
            return Err("dispatcher có nhánh bắt-tất-cả `_ =>`".to_string());
        }
        // Nhánh `match` cấp 1 thụt đúng 8 spaces; nhánh lồng sâu hơn (16+) không tính.
        // Dạng đã gặp: `StdId::X =>`, `StdId::A | StdId::B =>`, `id @ (StdId::A`, `| StdId::B`.
        if thut == 8 && t.contains("StdId::") {
            if t.contains("=>") {
                so_nhanh += 1;
            }
            let mut rest = t;
            while let Some(pos) = rest.find("StdId::") {
                let after = &rest[pos + "StdId::".len()..];
                let mot: String = after
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '_')
                    .collect();
                if let Some(i) = ten.iter().position(|x| *x == mot) {
                    khop[i] = true;
                }
                rest = &after[mot.len()..];
            }
        }
    }
    Ok((so_nhanh, khop))
}

/// **ST-317c** — đếm **số thật** và chốt dispatcher **không** có nhánh bắt-tất-cả (`_ =>`) che
/// thiếu case. Enum `StdId` sinh từ bảng registry nên `match id` vét cạn là **compile-time**;
/// test này bắt việc thêm `_ =>` (khi đó một biến thể mới sẽ lọt mà rustc không báo) và bắt
/// việc xoá nhánh dispatch của một id.
#[test]
fn std_id_dem_that_va_dispatcher_khong_co_nhanh_bat_tat_ca() {
    // Số thật tại ST-317c: **167**. Đổi số này là **đổi API std** (`VD-129`) — phải có VD nêu
    // đích danh tên thêm/xoá và regen `agent/API-STD.txt`; không sửa lặng lẽ.
    assert_eq!(StdId::COUNT, 167, "StdId::COUNT (số hàm std.* tại ST-317c)");
    assert_eq!(FUNCS.len(), 167, "FUNCS.len()");
    assert_eq!(StdId::ALL.len(), 167, "StdId::ALL.len()");

    let ten: Vec<String> = StdId::ALL.iter().map(|id| format!("{id:?}")).collect();
    let ten_ref: Vec<&str> = ten.iter().map(String::as_str).collect();
    let (so_nhanh, khop) = quet_dispatcher(include_str!("../stdlib.rs"), &ten_ref)
        .expect("dispatcher không được có `_ =>`");
    assert!(
        so_nhanh > 0,
        "không thấy nhánh dispatch nào trong call_by_id"
    );
    let thieu: Vec<&str> = StdId::ALL
        .iter()
        .zip(&khop)
        .filter(|(_, ok)| !**ok)
        .map(|(id, _)| id.as_str())
        .collect();
    assert!(
        thieu.is_empty(),
        "dispatcher thiếu nhánh cho: {thieu:?} (thêm hàm std.* = 1 hàng registry + 1 nhánh)"
    );
}

/// Ca âm của [`quet_dispatcher`]: `_ =>` bị từ chối; thiếu nhánh bị phát hiện qua cờ phủ.
#[test]
fn quet_dispatcher_ca_am() {
    let co_bat_tat_ca = "pub fn call_by_id(h: &mut dyn Host, id: StdId) -> R<Value> {\n    match id {\n        StdId::A => {}\n        _ => {}\n    }\n}\n";
    let e = quet_dispatcher(co_bat_tat_ca, &["A"]).expect_err("phải từ chối `_ =>`");
    assert!(e.contains("_ =>"), "thông điệp: {e}");

    let thieu = "pub fn call_by_id(h: &mut dyn Host, id: StdId) -> R<Value> {\n    match id {\n        StdId::B => {}\n    }\n}\n";
    let (n, khop) = quet_dispatcher(thieu, &["A", "B"]).expect("không có `_ =>`");
    assert_eq!(n, 1);
    assert_eq!(khop, vec![false, true], "A thiếu, B có");
}
