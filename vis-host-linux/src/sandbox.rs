// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-132 (VD-120)** — cưỡng chế quyền bằng **kernel** Linux (Landlock).
//!
//! Khác [`crate::probe`]: probe chỉ **đọc** `/sys/kernel/security/lsm`; mô-đun này **tạo ruleset**
//! và `restrict_self()` để kernel chặn thật. Sau khi áp, mọi truy cập hệ tệp **ngoài** tập được
//! phép bị kernel trả `EACCES`/`EPERM` — kể cả **tiến trình con** thừa hưởng ruleset, nên đây là
//! bằng chứng "sandbox con" ở mức hệ tệp (không phải `FsGuard` của runtime).
//!
//! Ghi chú thiết kế:
//! - Chỉ **hệ tệp** (Dir/File). `cap[Net]` vẫn cưỡng chế ở **runtime** (`NetGuard`) như VD-120.
//! - Dùng **`ABI::V3`** (Linux ≥ 5.19) để **xử lý cả `Truncate`** — W3 C1: ruleset V1 không
//!   handle `Truncate` nên tiến trình con vẫn `truncate(2)` được tệp trong `cap[Dir]` chỉ-đọc.
//!   Kernel cũ hơn (ABI < 3) sẽ **từ chối tạo ruleset** (fail-closed: không chạy sandbox yếu).
//! - Quyền của **tệp** và của **thư mục** khác nhau: `ReadDir` chỉ hợp lệ trên thư mục, nên tệp
//!   chỉ xin `ReadFile` (+`WriteFile`+`Truncate` khi có quyền ghi). Xin sai ⇒ `add_rule` trả `EINVAL`.
//! - `restrict_self()` trả [`RestrictionStatus`]: `NotEnforced` ⇒ nền **không** cưỡng chế được
//!   (kernel cũ / LSM tắt) — người gọi phải coi là "chưa có sandbox", không được coi là xong.

use std::path::PathBuf;

use landlock::{
    Access, AccessFs, PathBeneath, PathFd, Ruleset, RulesetAttr, RulesetCreatedAttr, RulesetStatus,
    ABI,
};

/// Một mục được phép truy cập sau khi áp sandbox.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChoPhep {
    /// Đường dẫn **đã canonicalize** (đường dẫn không tồn tại ⇒ `add_rule` lỗi).
    pub duong: PathBuf,
    /// `true` ⇒ cho ghi trong phạm vi mục này.
    pub ghi: bool,
    /// `true` ⇒ mục là **tệp** (không phải thư mục) — quyết định tập quyền xin.
    pub la_tep: bool,
}

/// Mức cưỡng chế kernel đạt được.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Muc {
    /// Kernel cưỡng chế **đầy đủ** ruleset đã yêu cầu.
    Day,
    /// Kernel chỉ cưỡng chế **một phần** (ABI cũ hơn tập quyền yêu cầu).
    MotPhan,
    /// Kernel **không** cưỡng chế (Landlock tắt/không có).
    Khong,
}

/// Kết quả áp sandbox.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KetQua {
    /// ABI Landlock đã dùng (3: có `Truncate`/`Refer`).
    pub abi: u8,
    /// Mức cưỡng chế.
    pub muc: Muc,
    /// Số luật đã nạp vào ruleset.
    pub so_luat: usize,
    /// `PR_SET_NO_NEW_PRIVS` đã bật (Landlock đòi).
    pub no_new_privs: bool,
}

impl KetQua {
    /// Dòng chẩn đoán **ổn định** (in ra stderr khi `--sandbox`).
    pub fn dong(&self) -> String {
        let muc = match self.muc {
            Muc::Day => "đầy đủ",
            Muc::MotPhan => "một phần",
            Muc::Khong => "không",
        };
        format!(
            "sandbox: landlock ABI={} mức={} luật={} nnp={}",
            self.abi, muc, self.so_luat, self.no_new_privs
        )
    }
}

/// Lỗi khi áp sandbox.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoiSandbox {
    /// Kernel/LSM không hỗ trợ Landlock ⇒ người gọi trả exit 3 (như `--sandbox-check`).
    KhongHoTro(String),
    /// Lỗi hệ thống khi dựng/nạp luật (đường dẫn không tồn tại, EINVAL, …).
    HeThong(String),
}

impl std::fmt::Display for LoiSandbox {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoiSandbox::KhongHoTro(s) => write!(f, "landlock không hỗ trợ: {s}"),
            LoiSandbox::HeThong(s) => write!(f, "landlock lỗi hệ thống: {s}"),
        }
    }
}

/// Toàn bộ quyền hệ tệp của ABI, **gồm `Execute`**.
///
/// **ST-295 (VD-276 §3)** — chỉ khi `--sandbox` có `cap[Proc]` **và** `--grant`, ruleset mới
/// **xử lý** `Execute` (để chương trình ngoài trong allowlist chạy được). Không grant ⇒ giữ
/// nguyên tập quyền cũ (không đụng `Execute`), tránh đổi hành vi “không giới hạn lệnh”.
fn toan_bo(abi: ABI, execute: bool) -> landlock::BitFlags<AccessFs> {
    let mut a = AccessFs::from_all(abi);
    if !execute {
        a.remove(AccessFs::Execute);
    }
    a
}

/// Tập quyền **đọc** của ABI (kèm `Execute` khi bật `--grant` cho `cap[Proc]`).
fn chi_doc(abi: ABI, execute: bool) -> landlock::BitFlags<AccessFs> {
    let mut a = AccessFs::from_read(abi);
    if !execute {
        a.remove(AccessFs::Execute);
    }
    a
}

/// Tập quyền cho một mục: tệp chỉ xin quyền hợp lệ trên tệp; thư mục xin cả cây.
fn quyen(c: &ChoPhep, abi: ABI, execute: bool) -> landlock::BitFlags<AccessFs> {
    let mut q = match (c.la_tep, c.ghi) {
        // Tệp: chỉ ReadFile/WriteFile/Truncate hợp lệ (ReadDir trên tệp ⇒ EINVAL).
        // **ST-315c (#197)**: ABI::V3 xử lý `Truncate`; thiếu quyền này ⇒ `fs::write`/`O_TRUNC`/
        // `truncate(2)` trên tệp đơn được cấp `(write)` bị kernel trả `EACCES`.
        (true, false) => AccessFs::ReadFile.into(),
        (true, true) => AccessFs::ReadFile | AccessFs::WriteFile | AccessFs::Truncate,
        // Thư mục: đọc = ReadFile|ReadDir; ghi = toàn bộ tập quyền.
        (false, false) => chi_doc(abi, execute),
        (false, true) => toan_bo(abi, execute),
    };
    if execute {
        q |= AccessFs::Execute;
    }
    q
}

/// Áp **một** ruleset Landlock cho **luồng hiện tại** (và mọi tiến trình con kế tiếp).
///
/// Gọi **sau** khi đã đọc xong mọi tệp cần đọc (nạp chương trình, module) và **trước** khi chạy
/// chương trình: Landlock chỉ chặn được truy cập phát sinh **sau** lời gọi này.
pub fn ap_dung(cho: &[ChoPhep]) -> Result<KetQua, LoiSandbox> {
    ap_dung_voi_execute(cho, false)
}

/// **ST-295 (VD-276 §3)** — như [`ap_dung`] nhưng có/không **xử lý `Execute`**. `vis run --sandbox`
/// bật `true` **chỉ khi** có `cap[Proc]` + `--grant`; đường cũ/test giữ `false`.
pub fn ap_dung_voi_execute(cho: &[ChoPhep], execute: bool) -> Result<KetQua, LoiSandbox> {
    // **ST-314a (W3 C1)**: V3 xử lý `Truncate` (+`Refer`); V1 bỏ sót ⇒ lỗ hổng ghi tệp chỉ-đọc.
    let abi = ABI::V3;
    let mut ruleset = Ruleset::default();
    // Bắt buộc: chỉ định rõ tập quyền **xử lý**; quyền không xử lý vẫn theo quyền cũ của tiến trình.
    ruleset = ruleset
        .handle_access(toan_bo(abi, execute))
        .map_err(|e| LoiSandbox::KhongHoTro(e.to_string()))?;
    let mut da_tao = ruleset
        .create()
        .map_err(|e| LoiSandbox::KhongHoTro(e.to_string()))?;
    let mut so_luat = 0usize;
    for c in cho {
        let fd = PathFd::new(&c.duong)
            .map_err(|e| LoiSandbox::HeThong(format!("{}: {e}", c.duong.display())))?;
        let luat: PathBeneath<PathFd> = PathBeneath::new(fd, quyen(c, abi, execute));
        da_tao = da_tao
            .add_rule(luat)
            .map_err(|e| LoiSandbox::HeThong(format!("thêm luật {}: {e}", c.duong.display())))?;
        so_luat += 1;
    }
    let tt = da_tao
        .restrict_self()
        .map_err(|e| LoiSandbox::KhongHoTro(e.to_string()))?;
    let muc = match tt.ruleset {
        RulesetStatus::FullyEnforced => Muc::Day,
        RulesetStatus::PartiallyEnforced => Muc::MotPhan,
        RulesetStatus::NotEnforced => Muc::Khong,
    };
    Ok(KetQua {
        abi: abi as u8,
        muc,
        so_luat,
        no_new_privs: tt.no_new_privs,
    })
}
