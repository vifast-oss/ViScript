// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-70 (VD-101)** — `vis pkg`: registry **local** (không mạng), cài vào `.vis/pkg/<tên>/<ver>/`.
//!
//! Mô hình: `manifest.json` của mỗi gói khai **tường minh** `files[{path,sha256}]` (không glob,
//! không `read_dir` khi cài); nội dung được **chuẩn hoá** (bỏ BOM, CRLF→LF, đúng một LF cuối) **khi
//! ingest** ⇒ băm ổn định. `vis.lock` ghi **tất cả** gói đã cài (khoá sắp theo tên, không timestamp,
//! không đường dẫn tuyệt đối). `install` **chỉ** đọc `vis.lock` + kho blob `.vis/store/<sha256>` ⇒
//! không cần registry.
//!
//! Băm: `file_sha = SHA256(nội dung chuẩn hoá)`;
//! `tree_sha = SHA256("vispkg1\n" + Σ sorted(path+"\n"+file_sha+"\n"))` — dùng
//! [`vis_bytecode::source_hash`] (SHA-256 thật, không thêm crate).
//!
//! **ST-317b**: tách `pkg.rs` (1.151 dòng) thành `mod.rs` (chỉ khai báo + re-export) và các tệp
//! chuyên trách: [`manifest`] (manifest + `tree_sha`), [`goi`] (đọc gói/giải đồ thị/ghi cây),
//! [`khoa`] (lock định dạng 2), [`nguon`] (đối số + registry + khoá ký), [`lenh`] (các lệnh CLI).
//! API công khai và hành vi không đổi.

mod goi;
mod khoa;
mod lenh;
mod manifest;
mod nguon;

/// **VS3-5b/5c (VD-271)**: dọn blob/thư mục gói không còn được **bất kỳ** `vis.lock` nào tham chiếu
/// (một lock mặc định, `--lock` lặp lại, hoặc `--root` quét có kiểm soát).
mod gc;
/// **VS3-5a (VD-271)**: registry **HTTP/HTTPS** (index + tarball, SHA-256 trước khi cài) — dựng trên
/// HTTP client hiện có của `vis-interp`; `https://` dùng TLS của `vis-cli::net` (VS3-4a).
mod registry_http;
/// **ST-128 (VD-125)**: resolver **tất định** trên registry nhiều phiên bản (chọn cao nhất thoả,
/// quay lui khi xung đột; không giải được ⇒ `VS0704` kèm chuỗi ràng buộc).
pub mod resolver;
/// **ST-127 (VD-125)**: semver tối giản (`Version`/`VersionReq`) — resolver dùng ở ST-128, lock ghi
/// `version_req` ở ST-129. Không crate mới.
pub use vis_loader::semver;
/// **VS3-5a**: base64 + tar ustar tối giản — cùng lát, tách tệp cho R1.
mod tar_b64;

// **ST-296b (VD-281/VD-282)**: metadata lock + băm/chuẩn hoá đã chuyển sang `vis-loader`;
// re-export để các module con (`gc`, `registry_http`, `resolver`) và test giữ nguyên đường gọi.
pub(crate) use vis_loader::lock_meta::{
    bam, chuan_hoa, chuoi, doc_lock, kiem_path, la_hex64, lay, lay_chuoi, nang_cap_lock,
    so_phien_ban, GoiKhoa, Khoa, TepKhoa, LOCK_PHIEN_BAN,
};

/// Chạy `vis pkg …`; lỗi ⇒ `vis pkg: …` + exit **2** (mẫu `vis cache`, không mã mới).
pub use lenh::run;

// Tên giữ nguyên đường `super::…` cho `gc`/`registry_http`/`resolver`/`tar_b64` và test.
pub(crate) use goi::{doc_tat_ca, giai_roots, GoiCai};
pub(crate) use lenh::dung;
pub(crate) use manifest::{doc_manifest, manifest_tu_gia_tri, Manifest};
pub(crate) use nguon::cwd;
