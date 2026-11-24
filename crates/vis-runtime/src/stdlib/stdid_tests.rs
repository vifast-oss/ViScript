// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-313b (VD-289)** — test song ánh 1–1 giữa `FUNCS` (nguồn chữ ký) và `StdId` dispatch.

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
