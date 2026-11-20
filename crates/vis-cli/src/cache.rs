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
pub const PHIEN_BAN: &str = concat!(
    env!("CARGO_PKG_VERSION"),
    "+vd241-coercion-2026-11-19+st248-field-index-lift-2026-11-20+st250-global-init-trap-2026-11-20+vd256-fee-side-table-v4-2026-11-20+vd257-b-four-version-axes-2026-11-20"
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

/// Nạp module từ cache — **chỉ** khi `load_verified` chấp nhận.
pub fn nap(k: &[u8; 32]) -> Option<vis_bytecode::Module> {
    let p = duong_dan(k)?;
    let bytes = std::fs::read(&p).ok()?;
    vis_bytecode::load_verified(&bytes).ok()
}

/// Ghi module đã xác minh xuống cache (lỗi I/O ⇒ bỏ qua, không làm hỏng lần chạy).
pub fn luu(k: &[u8; 32], m: &vis_bytecode::Module) -> bool {
    // Chỉ ghi thứ **qua được** verifier.
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
    let bytes = vis_bytecode::encode(m);
    // Ghi tạm rồi đổi tên: tránh tệp cụt khi bị ngắt.
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
    #[test]
    fn khoa_cache_ghim() {
        let k = hex(&khoa_multi(&don_vi(), vis_syntax::Mode::Script));
        assert_eq!(
            k, "98a0b98342213b9b2fe3f8a8050ae0e12ba3e09ab276493cc2c657558b4650e8",
            "khoá cache đã đổi — VD-129: đổi khoá là thay đổi **có chủ ý**, phải ghim lại kèm VD"
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
