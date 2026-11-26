// SPDX-License-Identifier: MIT OR Apache-2.0
//! **Cache bytecode** theo hash closure (ST-23, VD-52 #3): `.../vifast/vis/<hash>.vbc`.
//!
//! Hash (FNV-1a 64 bit × 4, VD-54) phủ: **văn bản nguồn** + **phiên bản compiler** + **phiên bản
//! định dạng VBC1** + **bảng thẻ VBC1** (ST-126, VD-129) + **chế độ** (`.vis`/`.vish`) — tức
//! "closure" của nguồn và phụ thuộc biên dịch.
//!
//! **Luôn** nạp qua [`vis_bytecode::load_verified`]: bytes hỏng/lệch ⇒ biên dịch lại và ghi lại;
//! **không bao giờ** chạy bytecode chưa xác minh (VD-52 #3).

use std::path::PathBuf;

/// Phiên bản compiler dùng trong hash (đổi ⇒ cache cũ tự vô hiệu).
// **SỬA KÈM LÝ DO (ST-41)**: `Render` đổi **cách hiển thị** giá trị cuối pipeline ⇒ module đã cache
// bằng bản cũ phải **hết hiệu lực**; gắn nhãn logic vào khoá (không đổi `CARGO_PKG_VERSION`).
// **SỬA KÈM LÝ DO (ST-249/VD-129, 2026-11-20)**: ST-248 đổi logic codegen (nâng `Some` ở đích
// field/index) ⇒ bytecode cache sinh trước bản sửa vẫn có `VERSION=2` nhưng trả `5`/`7` thay vì
// `Some(5)`/`Some(7)`. `VERSION` không đổi (ST-246 đã đổi cho ngữ nghĩa bytecode) ⇒ phải đổi nhãn
// logic compiler để cache cũ tự vô hiệu — nợ #187 / `BLOCKED.md` §16.
// **SỬA KÈM LÝ DO (VD-245 §5, ST-250, 2026-11-20)**: runtime nay trap ô global chưa khởi tạo;
// cache sinh trước bản sửa vẫn có `VERSION=2` nhưng mang ngữ nghĩa VM cũ ⇒ đổi nhãn lần nữa để
// phiên bản phát hành gồm đầy đủ hàng rào global. Không đổi `VERSION`/opcode.
// **SỬA KÈM LÝ DO (VD-256 / ST-276, 2026-11-20)**: artifact nay có thêm **trailer bảng phí S**
// (`vis_bytecode::fee`, định dạng `VERSION=4`); cache sinh trước bản sửa không mang bảng phụ ⇒
// nhãn đổi để artifact cũ tự vô hiệu. `VERSION` VBC1 lõi vẫn 2 (hàng rào ST-246 giữ nguyên).
// **SỬA KÈM LÝ DO (VD-257 / stream B, 2026-11-20)**: bốn trục phiên bản nay được đưa **hết** vào
// khoá: module format (2) + instrumented module (3) + event-table (4) + weight-table (1). Cache
// sinh bởi tooling cũ (chưa có trailer / bảng trọng số khác) phải tự vô hiệu — kể cả khi nội dung
// source trùng. Đổi nhãn logic là có chủ ý (VD-129); không đụng opcode/ngữ nghĩa mặc định.
// **SỬA KÈM LÝ DO (VD-258 / stream A, 2026-11-20)**: checker/codegen đổi ngữ nghĩa
// `Result?`, `?.` lồng, record ctor/default, `None`; `VERSION` VBC1 không đổi ⇒ cache cũ
// cùng nguồn có thể chạy bytecode trước bản sửa. Đổi nhãn logic để cache cũ tự vô hiệu.
// **ST-289 (VD-272 điểm 3)**: `CARGO_PKG_VERSION` nay là **0.4.1** (workspace một nguồn) ⇒
// `PHIEN_BAN` đổi theo, cache bản 0.1.0 tự vô hiệu; **không** đổi `VERSION` VBC1/ABI/định dạng.
// **ST-290 (VD-273)**: ngân sách ngữ nghĩa bật **mặc định** 40M và artifact VM có trailer `VFEE`
// (bảng phí) ⇒ cache cũ không mang ngữ nghĩa ngân sách mới phải tự vô hiệu; `VERSION` VBC1 và
// ABI không đổi.
// **ST-305 (v0.5.0)**: bump `CARGO_PKG_VERSION` 0.4.5 → 0.5.0 + nhãn `v050-release` ⇒ cache 0.4.5
// tự vô hiệu; `VERSION` VBC1/ABI/định dạng không đổi.
// **ST-310f (v0.5.5)**: bump `CARGO_PKG_VERSION` 0.5.0 → 0.5.5 + nhãn `v055-release` (Pha B refactor
// kiến trúc 4 tầng) ⇒ cache 0.5.0 tự vô hiệu; `VERSION` VBC1/ABI/định dạng không đổi.
// **ST-319a (#206, 2026-11-27)**: codegen closure chụp **biến cấp tệp theo giá trị** tại điểm tạo
// (trước đây thân closure đọc ô global live) ⇒ cùng nguồn, bytecode đổi ngữ nghĩa nhưng `VERSION`
// VBC1 không đổi; nhãn buộc cache cũ tự vô hiệu (ca thật: cache pre-fix cho `0,1` thay vì `0,0`).
pub const PHIEN_BAN: &str = concat!(
    env!("CARGO_PKG_VERSION"),
    "+vd241-coercion-2026-11-19+st248-field-index-lift-2026-11-20+st250-global-init-trap-2026-11-20+vd256-fee-side-table-v4-2026-11-20+vd257-b-four-version-axes-2026-11-20+vd258-checker-option-record-none-2026-11-20+vd260-rest-parity-option-2026-11-21+vd263-letelse-generic-range-2026-11-21+vd265-266-methodret-genericres-intlit-2026-11-21+vd268-nestassign-2026-11-21+vd269-optmethod-2026-11-21+vd273-budget-default-40m-2026-11-22+v040-release-2026-11-23+v041-release-2026-11-23+v045-release-2026-11-23+st302-derives-json-async-impl-2026-11-23+v050-release-2026-11-23+v055-release-2026-11-23+v060-release-2026-11-24+st319a-closure-cap-tep-2026-11-27"
);

/// Thư mục cache: `$XDG_CACHE_HOME/vifast/vis` hoặc `~/.cache/vifast/vis`.
pub fn thu_muc() -> Option<PathBuf> {
    let goc = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache")))?;
    Some(goc.join("vifast").join("vis"))
}

/// Khoá cache cho **đơn vị biên dịch nhiều tệp** (ST-25): hash closure trên (phiên bản, mode, thứ tự
/// tất định các tệp `(rel, nội dung)`). Vì `files` ở **hậu duyệt DFS** theo thứ tự `use` trong nguồn,
/// kết quả **tất định**; sửa bất kỳ tệp phụ thuộc nào ⇒ khoá đổi.
///
/// **ST-126 (VD-129)**: khoá phủ thêm **bảng thẻ `VBC1`** ([`vis_bytecode::format::bang_the_hash`]) —
/// bytecode đã ghi bằng bảng thẻ khác **không** được dùng lại (verifier **không** biết ngữ nghĩa thẻ,
/// nên không thể là hàng rào cho việc đổi bảng). Đổi bảng thẻ ⇒ bytecache cũ thành **vô hiệu**.
pub fn khoa_multi(files: &[crate::modules::FileUnit], mode: vis_syntax::Mode) -> [u8; 32] {
    khoa_multi_voi(files, mode, &vis_bytecode::format::TEN_THE)
}

/// Như [`khoa_multi`] nhưng nhận **bảng thẻ** tường minh — **chỉ** để test đo "đổi bảng thẻ ⇒ đổi
/// khoá" (ST-126) mà không phải sửa code thật; đường chạy thật luôn truyền
/// [`vis_bytecode::format::TEN_THE`].
pub fn khoa_multi_voi(
    files: &[crate::modules::FileUnit],
    mode: vis_syntax::Mode,
    the: &[&str],
) -> [u8; 32] {
    khoa_multi_voi_version(files, mode, the, vis_bytecode::VERSION)
}

/// Như [`khoa_multi`] nhưng nhận **phiên bản bytecode** tường minh — **chỉ** để test đo "đổi
/// `VERSION` ⇒ đổi khoá" (ST-246/VD-244), tức chứng minh cache v1 không được tái dùng dưới ngữ
/// nghĩa v2. Đường chạy thật luôn truyền [`vis_bytecode::VERSION`] (xem [`khoa_multi`]).
pub fn khoa_multi_voi_version(
    files: &[crate::modules::FileUnit],
    mode: vis_syntax::Mode,
    the: &[&str],
    version: u16,
) -> [u8; 32] {
    let che_do = if mode == vis_syntax::Mode::Command {
        "vish"
    } else {
        "vis"
    };
    let mut s = String::with_capacity(
        64 + files
            .iter()
            .map(|f| f.rel.len() + f.text.len() + 2)
            .sum::<usize>(),
    );
    s.push_str("vifast-vis-cache-v3\n");
    s.push_str(PHIEN_BAN);
    s.push('\n');
    // **ST-246 (VD-244 §2)**: `VERSION` là một phần của khoá — cache v1 tự vô hiệu, source được
    // biên dịch lại thay vì tái dùng artifact mang ngữ nghĩa cũ.
    s.push_str(&version.to_string());
    s.push('\n');
    // **VD-257 (stream B)** — bốn trục phiên bản, **độc lập**, đều nằm trong khoá:
    //   (1) module format `VERSION` = 2, (2) instrumented module = 3 (dòng dưới),
    //   (3) event-table `fee::VERSION` = 4, (4) weight-table `WEIGHT_TABLE_VERSION` = 1.
    // Cache do tooling cũ xác minh (không có trailer / bảng trọng số khác) KHÔNG được tái dùng.
    s.push_str(&vis_bytecode::fee::VERSION.to_string());
    s.push('\n');
    s.push_str(&vis_bytecode::fee::WEIGHT_TABLE_VERSION.to_string());
    s.push('\n');
    s.push_str(&vis_bytecode::INSTRUMENTED_VERSION.to_string());
    s.push('\n');
    // **VD-129**: bảng thẻ là một phần của **định dạng** ⇒ phải nằm trong khoá.
    for b in vis_bytecode::format::bang_the_hash_voi(the) {
        s.push_str(&format!("{b:02x}"));
    }
    s.push('\n');
    s.push_str(che_do);
    s.push('\n');
    for f in files {
        s.push_str(&f.rel);
        s.push('\n');
        s.push_str(&f.text);
        s.push('\n');
    }
    vis_bytecode::format::source_hash(&s)
}

/// Đường dẫn tệp cache cho một khoá.
pub fn duong_dan(k: &[u8; 32]) -> Option<PathBuf> {
    let mut ten = String::with_capacity(64 + 4);
    for b in k {
        ten.push_str(&format!("{b:02x}"));
    }
    ten.push_str(".vbc");
    Some(thu_muc()?.join(ten))
}

/// **ST-290 (VD-273)** — nạp **cả** module lẫn bảng phí từ cache có trailer `VFEE` (artifact
/// được đóng dấu `INSTRUMENTED_VERSION`). Trả `None` nếu thiếu/hỏng ⇒ caller biên dịch lại.
///
/// Module được hạ dấu về `VERSION` sau khi tách bảng phí: bytecode **không đổi**, dấu `3` chỉ
/// bảo vệ đường nạp (VD-257); đường chạy thường (`Vm::run` → `verify`) nhận `VERSION`.
pub fn nap_co_phi(k: &[u8; 32]) -> Option<(vis_bytecode::Module, vis_bytecode::fee::FeeTable)> {
    let p = duong_dan(k)?;
    let bytes = std::fs::read(&p).ok()?;
    let (mut m, fee) = vis_bytecode::load_verified_with_fee(&bytes).ok()?;
    m.version = vis_bytecode::VERSION;
    Some((m, fee))
}

/// **ST-290 (VD-273)** — ghi module + bảng phí thành artifact có trailer (cache cho đường VM
/// bật ngân sách mặc định). Vẫn chỉ ghi thứ qua được verifier.
pub fn luu_co_phi(
    k: &[u8; 32],
    m: &vis_bytecode::Module,
    fee: &vis_bytecode::fee::FeeTable,
) -> bool {
    if vis_bytecode::verify(m).is_err() {
        return false;
    }
    let Some(p) = duong_dan(k) else {
        return false;
    };
    if let Some(dir) = p.parent() {
        if std::fs::create_dir_all(dir).is_err() {
            return false;
        }
    }
    let bytes = vis_bytecode::encode_with_fee(m, fee);
    let tmp = p.with_extension("vbc.tmp");
    if std::fs::write(&tmp, &bytes).is_err() {
        return false;
    }
    std::fs::rename(&tmp, &p).is_ok()
}

/// Xoá toàn bộ cache; trả số tệp đã xoá.
pub fn xoa() -> usize {
    let Some(dir) = thu_muc() else {
        return 0;
    };
    let Ok(ds) = std::fs::read_dir(&dir) else {
        return 0;
    };
    let mut n = 0;
    for e in ds.flatten() {
        let p = e.path();
        if p.extension()
            .map(|x| x == "vbc" || x == "tmp")
            .unwrap_or(false)
            && std::fs::remove_file(&p).is_ok()
        {
            n += 1;
        }
    }
    n
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::FileUnit;
    use std::path::PathBuf;

    fn hex(k: &[u8; 32]) -> String {
        let mut s = String::with_capacity(64);
        for b in k {
            s.push_str(&format!("{b:02x}"));
        }
        s
    }

    /// Đơn vị biên dịch **cố định** (2 tệp) — đầu vào của ca ghim khoá.
    fn don_vi() -> Vec<FileUnit> {
        vec![
            FileUnit {
                path: PathBuf::from("/tmp/vis/vis-module-a.vis"),
                rel: "a.vis".to_string(),
                text: "fn a() -> int:\n    1\n".to_string(),
            },
            FileUnit {
                path: PathBuf::from("/tmp/vis/vis-module-f.vis"),
                rel: "f.vis".to_string(),
                text: "use ./a\n\nprint(a())\n".to_string(),
            },
        ]
    }

    /// **ST-126 (VD-129)** — khoá cache của một đơn vị **cố định** bị **ghim** bằng hex.
    /// Ghim lại: **VD-241 §3/T6** + VD-241 Phase B (2026-11-19) — sửa compiler ⇒ PHIEN_BAN đổi (memory VD-129).
    /// Ghim lại lần nữa (2026-11-19, đợt coercion Option): compiler đổi `lift_opt`/`var`/gán lại/VM `Unwrap`
    /// ⇒ `PHIEN_BAN` đổi ⇒ khoá đổi (memory VD-129).
    /// Ghim lại lần nữa (2026-11-20, **ST-246/VD-244**): `vis_bytecode::VERSION` tăng `1 → 2` để đánh
    /// dấu ngữ nghĩa mới của ô `>= GLOBAL_SLOT_BASE` ⇒ khoá đổi là **có chủ ý**: cache v1 bị bỏ,
    /// source được biên dịch lại.
    /// Ghim lại lần nữa (2026-11-20, **ST-249/VD-129**, nợ #187): ST-248 đổi logic codegen (nâng
    /// `Some` ở đích field/index) nhưng `VERSION` vẫn 2 ⇒ `PHIEN_BAN` đổi để cache cũ tự vô hiệu.
    /// Ghim lại lần nữa (2026-11-20, **VD-245 §5 / ST-250**): runtime trap ô global chưa khởi tạo ⇒
    /// cache mang ngữ nghĩa VM cũ phải vô hiệu trước khi phát hành.
    /// Ghim lại lần nữa (2026-11-20, **VD-257 / stream B**): bốn trục phiên bản (module 2/3,
    /// event-table 4, weight 1) đều vào khoá; cache của tooling trước VD-257 tự vô hiệu.
    /// Ghim lại lần nữa (2026-11-20, **VD-258 / stream A**): checker/codegen đổi `Result?`,
    /// `?.` lồng, record ctor/default và `None`; `VERSION` VBC1 không đổi ⇒ nhãn logic phải đổi
    /// để cache cũ cùng nguồn không chạy bytecode trước bản sửa.
    /// Ghim lại lần nữa (2026-11-21, **VD-260**): rest-pattern, type-directed Option lift và
    /// branch join đổi codegen ⇒ cache cũ không được tái dùng.
    #[test]
    fn khoa_cache_ghim() {
        let k = hex(&khoa_multi(&don_vi(), vis_syntax::Mode::Script));
        // **ST-289 (VD-272 điểm 3)**: phiên bản workspace `0.1.0 → 0.4.1` (một nguồn
        // `version.workspace = true`). `PHIEN_BAN` nhúng `CARGO_PKG_VERSION` **có chủ ý**
        // (VD-129) ⇒ khoá đổi, cache của bản 0.1.0 tự vô hiệu; `VERSION` VBC1 và ABI **không đổi**.
        // **ST-290 (VD-273)**: ngân sách mặc định 40M + artifact VM mang bảng phí (trailer VFEE)
        // ⇒ cache cũ không mang ngữ nghĩa ngân sách mới bị bỏ; ghim lại lần nữa.
        // **v0.4.0 (ST-296, VD-129)**: bump `CARGO_PKG_VERSION` 0.3.6 → 0.4.0 + nhãn
        // `v040-release` ⇒ khoá đổi có chủ ý; cache 0.3.6 tự vô hiệu, `VERSION` VBC1/ABI không đổi.
        // **v0.4.1 (ST-296d, VD-129)**: bump 0.4.0 → 0.4.1 + nhãn `v041-release` ⇒ ghim lại;
        // cache 0.4.0 tự vô hiệu, `VERSION` VBC1/ABI/định dạng không đổi.
        // **v0.4.5 (ST-298, VD-129)**: bump 0.4.1 → 0.4.5 + nhãn `v045-release` ⇒ ghim lại;
        // cache 0.4.1 tự vô hiệu, `VERSION` VBC1/ABI/định dạng không đổi.
        // **ST-302 (VD-284)**: sinh method `derives Json` + `async impl/cmd` ⇒ bytecode cùng
        // nguồn đổi nghĩa; nhãn `st302-derives-json-async-impl` buộc cache 0.4.5 cũ vô hiệu.
        // **v0.5.0 (ST-305, VD-129)**: bump 0.4.5 → 0.5.0 + nhãn `v050-release` ⇒ ghim lại;
        // cache 0.4.5 tự vô hiệu, `VERSION` VBC1/ABI/định dạng không đổi.
        // **v0.5.5 (ST-310f, VD-129)**: bump 0.5.0 → 0.5.5 + nhãn `v055-release` (Pha B refactor
        // kiến trúc) ⇒ ghim lại; cache 0.5.0 tự vô hiệu, `VERSION` VBC1/ABI/định dạng không đổi.
        // **ST-319a (#206, 2026-11-27)**: codegen chụp biến cấp tệp theo **giá trị tại điểm tạo
        // closure** ⇒ nhãn `st319a-closure-cap-tep` buộc cache pre-fix tự vô hiệu (trước bump,
        // cache cũ vẫn trả `0,1` thay vì `0,0` dù binary đã sửa); `VERSION` VBC1 không đổi.
        assert_eq!(
            k, "de308705a335d3da380dc8ff57b57b50f2ffccbf281af4f1a3bc45eb2e117bba",
            "khoá cache đã đổi — VD-129: đổi khoá là thay đổi **có chủ ý**, phải ghim lại kèm VD"
        );
        assert_ne!(
            k, "c5c4d5ac27f3e67f0d0d7c6f5fb2afea1fa1e80ba4332594c949d49898755b66",
            "ST-319a: cache sinh trước closure-capture-by-value không được tái dùng"
        );
        assert_ne!(
            k, "903995a78ed8585f360005df18a81a0be3a970c5fdaf2a3f7401cc90ec2100d9",
            "v0.6.0: cache sinh trước bump 0.6.0 không được tái dùng"
        );
        assert_ne!(
            k, "88613f2f2aa91f08fb2825e0d6769085818d3551feb54eb529e34099d43314dc",
            "ST-302: cache sinh trước derives Json/async impl không được tái dùng"
        );
        assert_ne!(
            k, "d92f31c28ed5e97a07976b83be0fb6a3c4c3914120e4e8f98e9fc832685418ce",
            "v0.4.5: cache sinh trước bump 0.4.5 không được tái dùng"
        );
        assert_ne!(
            k, "42f5389364ae126f8b9dea4c3bf107c983c3b92d7b1743a5293d547f3d4d7c0f",
            "v0.4.1: cache sinh trước bump 0.4.1 không được tái dùng"
        );
        assert_ne!(
            k, "22572d0b3267c93ddecc223a7522cc69767b3a5c1f6b46f8bff04d95d3803a8b",
            "v0.4.1: cache sinh trước bump 0.4.1 không được tái dùng"
        );
        assert_ne!(
            k, "e5373caf155dfad15e8173df5f5218651847d17d0963e9f0784b42c8ccdf026d",
            "v0.4.0: cache sinh trước bump 0.4.0 không được tái dùng"
        );
        assert_ne!(
            k, "2c0382c5bf04f2657aeec1155ab00e5ed1f193e0838b048c9be9f24685ecbbbf",
            "ST-290: cache sinh trước ngân sách mặc định 40M không được tái dùng"
        );
        assert_ne!(
            k, "c7e09d0f0b535a3f8edb9d794c9d852f4e15c5fe07586b5d37fcdbe55f3fd393",
            "ST-289: cache sinh trước khi đồng bộ phiên bản (0.1.0) không được tái dùng"
        );
        assert_ne!(
            k, "204af1c294a5744ddf069f14472ff3b4a9f42230605439bab785c83e203aa159",
            "VD-269: cache sinh trước ?.method không được tái dùng"
        );
        assert_ne!(
            k, "1bafa9499ed38019c6b2a87727eef2156d8e207e8f5ee52756e8c4882d256631",
            "VD-268: cache sinh trước gán lồng không được tái dùng"
        );
        assert_ne!(
            k, "f20c2adeac140ca71aecdcb17d0e8997f61c5d9a1e666ef9b8ec6f9a5706c254",
            "VD-265/266: cache sinh trước methodret/genericres/intlit không được tái dùng"
        );
        assert_ne!(
            k, "dd5794e2e7aafb68df8a4e9780d29c40041a722209e989b1f8b2ddd612408d02",
            "VD-263: cache sinh trước let-else/generic/range không được tái dùng"
        );
        assert_ne!(
            k, "37da16859f16f004b16c4ce325686990f05ea60bc65d1432e59a195d702a7b27",
            "VD-260: cache sinh trước rest-pattern/parity không được tái dùng"
        );
        assert_ne!(
            k, "98a0b98342213b9b2fe3f8a8050ae0e12ba3e09ab276493cc2c657558b4650e8",
            "VD-258: cache sinh trước checker/record/None mới không được tái dùng"
        );
        assert_ne!(
            k, "cca587acccdba4f540520dc611ccb8b8128a972136e3821f01c38914607e0f20",
            "VD-257: cache sinh trước bốn-trục-phiên bản không được tái dùng"
        );
        // ST-249/#187: khoá của bản trước ST-248 (chỉ đổi PHIEN_BAN, VERSION vẫn 2) phải KHÔNG còn
        // trùng — nếu trùng thì cache cũ vẫn được nạp và trả kết quả sai của field/index lift.
        assert_ne!(
            k, "66c332a94f29959cdba27435f63415fa90a1bcd0550370ceeb174777c89d85c5",
            "ST-249: cache sinh trước field/index lift không được tái dùng"
        );
        // VD-245 §5: cache sinh trước global init trap (dù đã có field/index lift) cũng không được tái dùng.
        assert_ne!(
            k, "ad63ae9807aadc6c6208040197966169a0552ea07fe96d9340a5e84b7daf210e",
            "VD-245: cache sinh trước global init trap không được tái dùng"
        );
        assert!(
            PHIEN_BAN.contains("st248-field-index-lift"),
            "ST-249: PHIEN_BAN phải mang nhãn logic của ST-248"
        );
        // VD-256/ST-276 (D7): cache phải mang cả nhãn bảng phí + số phiên bản định dạng phí.
        assert!(PHIEN_BAN.contains("vd256-fee-side-table-v4"));
        assert_eq!(vis_bytecode::fee::VERSION, 4);
        // VD-257 (stream B): bốn trục độc lập, cache phải mang nhãn mới + weight-table version.
        assert!(PHIEN_BAN.contains("vd257-b-four-version-axes"));
        // VD-258 (stream A): đổi ngữ nghĩa checker/codegen (Result?, ?. lồng, record, None)
        // nhưng VERSION VBC1 không đổi ⇒ nhãn cache phải đổi.
        assert!(
            PHIEN_BAN.contains("vd258-checker-option-record-none"),
            "VD-258: cache phải mang nhãn checker/record/None mới"
        );
        assert!(
            PHIEN_BAN.contains("vd260-rest-parity-option"),
            "VD-260: cache phải mang nhãn rest-pattern/parity mới"
        );
        assert!(
            PHIEN_BAN.contains("vd263-letelse-generic-range"),
            "VD-263: cache phải mang nhãn let-else/generic/range mới"
        );
        assert!(
            PHIEN_BAN.contains("vd265-266-methodret-genericres-intlit"),
            "VD-265/266: cache phải mang nhãn methodret/genericres/intlit"
        );
        assert!(
            PHIEN_BAN.contains("vd268-nestassign"),
            "VD-268: cache phải mang nhãn gán lồng"
        );
        assert!(
            PHIEN_BAN.contains("vd269-optmethod"),
            "VD-269: cache phải mang nhãn gọi phương thức qua ?."
        );
        assert!(
            PHIEN_BAN.contains("vd273-budget-default-40m"),
            "ST-290: cache phải mang nhãn ngân sách mặc định 40M"
        );
        assert!(
            PHIEN_BAN.contains("v040-release-2026-11-23"),
            "v0.4.0: cache phải mang nhãn phát hành"
        );
        assert!(
            PHIEN_BAN.contains("v041-release-2026-11-23"),
            "v0.4.1: cache phải mang nhãn phát hành"
        );
        assert!(
            PHIEN_BAN.contains("v045-release-2026-11-23"),
            "v0.4.5: cache phải mang nhãn phát hành"
        );
        assert!(
            PHIEN_BAN.contains("v050-release-2026-11-23"),
            "v0.5.0: cache phải mang nhãn phát hành"
        );
        assert!(
            PHIEN_BAN.contains("v055-release-2026-11-23"),
            "v0.5.5: cache phải mang nhãn phát hành"
        );
        assert!(
            PHIEN_BAN.contains("v060-release-2026-11-24"),
            "v0.6.0: cache phải mang nhãn phát hành"
        );
        assert!(
            PHIEN_BAN.starts_with("0.6.0+"),
            "PHIEN_BAN phải bắt đầu 0.6.0"
        );
        assert_eq!(vis_bytecode::fee::WEIGHT_TABLE_VERSION, 1);
        assert_eq!(vis_bytecode::INSTRUMENTED_VERSION, 3);
        assert_ne!(
            k, "ad63ae9807aadc6c6208040197966169a0552ea07fe96d9340a5e84b7daf210e",
            "VD-256: cache sinh trước bảng phí không được tái dùng"
        );
    }

    /// **ST-126 (VD-129) — phần khó**: **đổi bảng thẻ ⇒ đổi khoá cache**, nên bytecode cũ **không**
    /// bị dùng nhầm. Giả lập "bảng thẻ đổi" bằng **tham số** của [`khoa_multi_voi`] (không sửa code
    /// thật): đổi **tên** một thẻ, rồi thêm/đổi **số** thẻ.
    #[test]
    fn khoa_cache_doi_khi_bang_the_doi() {
        let files = don_vi();
        let goc = khoa_multi(&files, vis_syntax::Mode::Script);
        let bang_that = &vis_bytecode::format::TEN_THE;

        // (a) Khoá **thật** đúng bằng khoá tính với `bang_the_hash()` của bảng thật ⇒ bảng thẻ
        //     thật sự là đầu vào của `khoa_multi` (không phải nhánh chết).
        let bang_that_hash = vis_bytecode::format::bang_the_hash();
        assert_eq!(
            goc,
            khoa_multi_voi(&files, vis_syntax::Mode::Script, bang_that),
            "khoa_multi phải dùng đúng bảng thẻ thật"
        );
        assert_eq!(
            hex(&bang_that_hash),
            hex(&vis_bytecode::format::bang_the_hash_voi(bang_that))
        );

        // (b) Đổi **tên** một thẻ.
        let mut doi_ten: Vec<&str> = bang_that.to_vec();
        doi_ten[5] = "PopDoiTen";
        assert_ne!(
            hex(&vis_bytecode::format::bang_the_hash_voi(&doi_ten)),
            hex(&bang_that_hash),
            "đổi tên thẻ phải đổi hash bảng thẻ"
        );
        assert_ne!(
            hex(&khoa_multi_voi(&files, vis_syntax::Mode::Script, &doi_ten)),
            hex(&goc),
            "đổi tên thẻ phải đổi khoá cache (nếu không, bytecode cũ bị dùng nhầm)"
        );

        // (c) Thêm **một thẻ mới** ở cuối (lệnh mới).
        let mut them: Vec<&str> = bang_that.to_vec();
        them.push("TheMoi");
        assert_ne!(
            hex(&khoa_multi_voi(&files, vis_syntax::Mode::Script, &them)),
            hex(&goc),
            "thêm thẻ phải đổi khoá cache"
        );

        // (d) Đổi **số** thẻ (hoán vị hai tên) — cùng số phần tử, khác ánh xạ.
        let mut hoan_vi: Vec<&str> = bang_that.to_vec();
        hoan_vi.swap(5, 6);
        assert_ne!(
            hex(&khoa_multi_voi(&files, vis_syntax::Mode::Script, &hoan_vi)),
            hex(&goc),
            "đổi số thẻ phải đổi khoá cache"
        );

        // (e) Đường dẫn tệp cache khác nhau ⇒ tệp bytecode cũ **không** được tìm thấy cho bảng mới.
        assert_ne!(
            duong_dan(&khoa_multi_voi(&files, vis_syntax::Mode::Script, &them)),
            duong_dan(&goc),
            "hai bảng thẻ khác nhau phải trỏ hai tệp cache khác nhau"
        );

        // (f) Bảng thẻ **không** làm mất tính chất cũ: đổi nguồn / đổi chế độ vẫn đổi khoá.
        let mut khac = files.clone();
        khac[1].text.push('\n');
        assert_ne!(khoa_multi(&khac, vis_syntax::Mode::Script), goc);
        assert_ne!(khoa_multi(&files, vis_syntax::Mode::Command), goc);
    }

    /// **ST-246 (VD-244 §2)** — `VERSION` là đầu vào của khoá: artifact v1 **không** được tái dùng
    /// dưới ngữ nghĩa v2. Mô phỏng "khoá cũ" bằng [`khoa_multi_voi_version`] với `version = 1`
    /// (giá trị trước ST-246), không sửa code thật.
    #[test]
    fn khoa_cache_v1_khong_duoc_tai_dung_v2() {
        let files = don_vi();
        let the = &vis_bytecode::format::TEN_THE;

        // (a) Khoá thật đúng bằng khoá tính với `VERSION` hiện tại (= 2) ⇒ không phải nhánh chết.
        let that = khoa_multi(&files, vis_syntax::Mode::Script);
        assert_eq!(
            that,
            khoa_multi_voi_version(&files, vis_syntax::Mode::Script, the, vis_bytecode::VERSION),
            "khoa_multi phải dùng đúng VERSION hiện tại"
        );
        assert_eq!(vis_bytecode::VERSION, 2, "VERSION phải là 2 sau ST-246");

        // (b) Khoá v1 khác khoá v2 ⇒ tệp cache v1 không bao giờ được `nap()` tìm thấy.
        let cu = khoa_multi_voi_version(&files, vis_syntax::Mode::Script, the, 1);
        assert_ne!(hex(&cu), hex(&that), "đổi VERSION phải đổi khoá cache");
        assert_ne!(
            duong_dan(&cu),
            duong_dan(&that),
            "cache v1 và v2 phải trỏ hai tệp khác nhau"
        );

        // (c) Mọi phiên bản khác (0, 3, …) cũng khác — khoá khoá theo VERSION, không phải hằng số.
        for v in [0u16, 3, 99] {
            assert_ne!(
                hex(&khoa_multi_voi_version(
                    &files,
                    vis_syntax::Mode::Script,
                    the,
                    v
                )),
                hex(&that),
                "VERSION {v} không được trùng khoá hiện tại"
            );
        }
    }
}
