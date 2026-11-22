// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-276 / VD-256** — định nghĩa sự kiện ngân sách **dùng chung** + biểu diễn **S (bảng phụ theo PC)**.
//!
//! Hai nghĩa vụ được tách rõ (VD-256 §4):
//!
//! - **(A) Parity nguồn**: compiler và interpreter cùng dùng [`FeeKind`] ở đây. Bảng phụ **không**
//!   tự nhận là bằng chứng instrumentation nguồn đầy đủ — bằng chứng là **test so dãy sự kiện có
//!   thứ tự** (`crates/vis-cli/tests/lat256_budget.rs`).
//! - **(B) An toàn bytecode không đáng tin**: [`verify_fee`] kiểm cấu trúc (kind/site/PC, giới hạn
//!   kích thước), ràng buộc suy độc lập từ opcode và CFG (mọi chu trình phải có ít nhất một điểm
//!   phí; cạnh vào thân vòng không được bỏ điểm phí guard). **Không** tái dựng HIR từ bytecode.
//!
//! Đây là đường **MUST-NOT-BE-DEFAULT** (VD-256 §5): bảng phụ là phần **bổ sung** (trailer) của
//! artifact, `decode` bỏ qua trailer, VM dùng hệ ngân sách cũ trừ khi test-hook bật. Không đổi
//! `MAGIC`, bảng opcode hay ngữ nghĩa thực thi mặc định.
//!
//! **VD-257 (stream B) — bốn trục phiên bản, độc lập:**
//! 1. **Module format**: `format::VERSION = 2` (cơ sở) / `format::INSTRUMENTED_VERSION = 3`
//!    (artifact có trailer `VFEE`, **bắt buộc** ngữ nghĩa budget mới).
//! 2. **Event-table** ([`EVENT_TABLE_VERSION`] = [`VERSION`] = 4): cấu trúc `pc/kind/site` trong `VFEE`.
//! 3. **Weight-table** ([`WEIGHT_TABLE_VERSION`] = 1): bảng trọng số v1 (thử nghiệm, chưa duyệt).
//! 4. **Cache key**: `vis-cli::cache::PHIEN_BAN` + cả ba số trên + bảng thẻ + mode + nguồn.
//!
//! Hàng rào tương thích nằm ở **trục 1**: artifact instrumented mang module version `3` ⇒ runtime
//! cũ (so `2`) **từ chối** ngay ở `decode`, không lờ trailer. Cửa nạp có budget
//! ([`load_verified_with_fee`]) **chỉ** nhận `3` + trailer hợp lệ ⇒ artifact cũ (`2`, không trailer)
//! bị từ chối rõ ràng, không bị coi là "đã instrumented/charged". Đổi cache key **không đủ** để bảo
//! vệ đường *direct load* — chính là lý do phải có bước này.

use crate::format::{Instr, Module};

// MỘT nguồn định nghĩa sự kiện: include chung với vis-interp/src/frame/fee.rs.
include!("fee_shared.rs");

/// Bảng phụ S: một danh sách **có thứ tự** sự kiện cho mỗi hàm (theo chỉ số hàm trong `Module`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FeeTable {
    pub funcs: Vec<Vec<FeeEntry>>,
}

impl FeeTable {
    /// Bảng rỗng với `n` hàm.
    pub fn new(n: usize) -> Self {
        Self {
            funcs: vec![Vec::new(); n],
        }
    }
    pub fn push(&mut self, func: usize, e: FeeEntry) {
        if func >= self.funcs.len() {
            self.funcs.resize(func + 1, Vec::new());
        }
        self.funcs[func].push(e);
    }
    pub fn is_empty(&self) -> bool {
        self.funcs.iter().all(|v| v.is_empty())
    }
    pub fn total(&self) -> usize {
        self.funcs.iter().map(|v| v.len()).sum()
    }
    /// Dãy sự kiện **có thứ tự** trên toàn module (theo hàm rồi theo thứ tự ghi).
    pub fn trace(&self) -> Vec<FeeEntry> {
        let mut v = Vec::with_capacity(self.total());
        for f in &self.funcs {
            v.extend_from_slice(f);
        }
        v
    }
    /// Có ít nhất một sự kiện tại `pc` của hàm `func` không?
    pub fn has_pc(funcs: &[Vec<FeeEntry>], func: usize, pc: u32) -> bool {
        funcs
            .get(func)
            .map(|v| v.iter().any(|e| e.pc == pc))
            .unwrap_or(false)
    }
}

// ---------------------------------------------------------------------------
// Serialization: trailer `VFEE` nối sau byte VBC1. `decode` bỏ qua trailer;
// `decode_fee` đọc riêng ⇒ không đổi cấu trúc `Module`/`Func`, mọi test cũ vẫn biên dịch.
// ---------------------------------------------------------------------------

/// Marker của trailer bảng phí (không đụng `MAGIC = VBC1`).
pub const FEE_MAGIC: [u8; 4] = *b"VFEE";
/// Giới hạn kích thước bảng (VD-256 §4 "giới hạn kích thước bảng"): tổng số entry.
pub const MAX_FEE_ENTRIES: usize = 1_000_000;
/// Tối đa số sự kiện tại **một** PC (chặn artifact khổng lồ tại một điểm).
pub const MAX_PER_PC: usize = 16;

/// Nối trailer bảng phí vào byte module (kết quả tất định).
///
/// **VD-257**: artifact được đóng dấu [`INSTRUMENTED_VERSION`] (3) ngay trong **trường version của
/// lõi VBC1** — đây là hàng rào mà runtime cũ (so `VERSION = 2`) cũng thực hiện, nên nó từ chối
/// artifact mới **mà không cần đọc trailer**. `MAGIC`/opcode/bố cục không đổi; chỉ giá trị version.
pub fn encode_with_fee(m: &Module, fee: &FeeTable) -> Vec<u8> {
    let mut stamped = m.clone();
    stamped.version = crate::format::INSTRUMENTED_VERSION;
    let mut out = crate::encode::encode(&stamped);
    out.extend_from_slice(&FEE_MAGIC);
    out.extend_from_slice(&VERSION.to_le_bytes());
    out.extend_from_slice(&WEIGHT_TABLE_VERSION.to_le_bytes());
    out.extend_from_slice(&(fee.funcs.len() as u32).to_le_bytes());
    for v in &fee.funcs {
        out.extend_from_slice(&(v.len() as u32).to_le_bytes());
        for e in v {
            out.extend_from_slice(&e.pc.to_le_bytes());
            out.push(e.kind.tag());
            out.extend_from_slice(&e.site.to_le_bytes());
        }
    }
    out
}

/// Lỗi giải mã trailer phí.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeeDecodeError {
    /// Không có trailer `VFEE` (artifact cũ hoặc không có instrumentation).
    Absent,
    /// Phiên bản định dạng phí lạ.
    BadVersion(u16),
    /// Phiên bản bảng trọng số lạ.
    BadWeightVersion(u16),
    /// Dữ liệu cụt / độ dài khai báo vượt quá.
    Truncated,
    /// Tag sự kiện lạ.
    BadKind(u8),
    /// Số entry vượt [`MAX_FEE_ENTRIES`].
    TooMany(usize),
    /// Nhiều hơn [`MAX_PER_PC`] sự kiện tại một PC.
    TooManyAtPc(u32),
}

impl core::fmt::Display for FeeDecodeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            FeeDecodeError::Absent => write!(f, "không có trailer bảng phí VFEE"),
            FeeDecodeError::BadVersion(v) => write!(f, "phiên bản bảng phí {v} ≠ {VERSION}"),
            FeeDecodeError::BadWeightVersion(v) => {
                write!(f, "phiên bản bảng trọng số {v} ≠ {WEIGHT_TABLE_VERSION}")
            }
            FeeDecodeError::Truncated => write!(f, "trailer bảng phí bị cụt"),
            FeeDecodeError::BadKind(t) => write!(f, "tag sự kiện phí lạ: {t}"),
            FeeDecodeError::TooMany(n) => write!(f, "bảng phí có {n} entry (> {MAX_FEE_ENTRIES})"),
            FeeDecodeError::TooManyAtPc(pc) => write!(f, "pc {pc} có quá {MAX_PER_PC} sự kiện"),
        }
    }
}

/// Đọc trailer bảng phí; **không** đọc phần module (đã do `decode` lo).
pub fn decode_fee(bytes: &[u8]) -> Result<FeeTable, FeeDecodeError> {
    let at = bytes
        .windows(4)
        .rposition(|w| w == FEE_MAGIC)
        .ok_or(FeeDecodeError::Absent)?;
    let mut p = at + 4;
    fn u16v(bytes: &[u8], p: &mut usize) -> Result<u16, FeeDecodeError> {
        if *p + 2 > bytes.len() {
            return Err(FeeDecodeError::Truncated);
        }
        let v = u16::from_le_bytes([bytes[*p], bytes[*p + 1]]);
        *p += 2;
        Ok(v)
    }
    fn u32v(bytes: &[u8], p: &mut usize) -> Result<u32, FeeDecodeError> {
        if *p + 4 > bytes.len() {
            return Err(FeeDecodeError::Truncated);
        }
        let v = u32::from_le_bytes([bytes[*p], bytes[*p + 1], bytes[*p + 2], bytes[*p + 3]]);
        *p += 4;
        Ok(v)
    }
    let v = u16v(bytes, &mut p)?;
    if v != VERSION {
        return Err(FeeDecodeError::BadVersion(v));
    }
    let wv = u16v(bytes, &mut p)?;
    if wv != WEIGHT_TABLE_VERSION {
        return Err(FeeDecodeError::BadWeightVersion(wv));
    }
    let nf = u32v(bytes, &mut p)? as usize;
    if nf > MAX_FEE_ENTRIES {
        return Err(FeeDecodeError::TooMany(nf));
    }
    let mut funcs = Vec::with_capacity(nf.min(1024));
    let mut total = 0usize;
    for _ in 0..nf {
        let n = u32v(bytes, &mut p)? as usize;
        total = total.saturating_add(n);
        if total > MAX_FEE_ENTRIES {
            return Err(FeeDecodeError::TooMany(total));
        }
        let mut v = Vec::with_capacity(n.min(1024));
        let mut per_pc: std::collections::HashMap<u32, usize> = std::collections::HashMap::new();
        for _ in 0..n {
            let pc = u32v(bytes, &mut p)?;
            if p + 1 > bytes.len() {
                return Err(FeeDecodeError::Truncated);
            }
            let kind = FeeKind::from_tag(bytes[p]).ok_or(FeeDecodeError::BadKind(bytes[p]))?;
            p += 1;
            let site = u32v(bytes, &mut p)?;
            let k = per_pc.entry(pc).or_insert(0);
            *k += 1;
            if *k > MAX_PER_PC {
                return Err(FeeDecodeError::TooManyAtPc(pc));
            }
            v.push(FeeEntry { pc, kind, site });
        }
        funcs.push(v);
    }
    Ok(FeeTable { funcs })
}

/// Giải mã **cả module lẫn bảng phí** rồi xác minh cả hai — cửa nạp cho test-hook ngân sách.
///
/// **VD-257 — chính sách rõ ràng cho artifact cũ:** cửa này **chỉ** nhận module version 3 (có
/// trailer). Artifact cũ (version 2, không trailer) bị từ chối với mã `Decode::BadVersion(2)` —
/// **không** được coi là "đã instrumented/charged" và cũng không rơi vào đường mặc định im lặng.
/// Trailer thiếu (`FeeDecodeError::Absent`) cũng bị từ chối: module version 3 mà không có bảng phí
/// là artifact hỏng, không phải "chương trình zero-work".
pub fn load_verified_with_fee(
    bytes: &[u8],
) -> Result<(Module, FeeTable), crate::verify::LoadError> {
    let m = crate::decode::decode_for_version(bytes, crate::format::INSTRUMENTED_VERSION)
        .map_err(crate::verify::LoadError::Decode)?;
    crate::verify::verify_for_version(&m, crate::format::INSTRUMENTED_VERSION)
        .map_err(crate::verify::LoadError::Verify)?;
    let fee = decode_fee(bytes)
        .map_err(|e| crate::verify::LoadError::Verify(VerifyError::FeeDecode(e.to_string())))?;
    verify_fee(&m, &fee)
        .map_err(|e| crate::verify::LoadError::Verify(VerifyError::FeeDecode(format!("{e:?}"))))?;
    Ok((m, fee))
}

// ---------------------------------------------------------------------------
// Verifier (D4 / nghĩa vụ B) — kiểm cấu trúc + CFG. Không tái dựng HIR.
// ---------------------------------------------------------------------------

/// Lỗi xác minh bảng phí (độc lập với [`crate::verify::VerifyError`] của VBC1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FeeVerifyError {
    /// Số hàm trong bảng khác số hàm của module.
    FuncCount { got: usize, want: usize },
    /// Tổng số entry vượt giới hạn.
    TooMany(usize),
    /// Entry có `pc` ngoài code của hàm.
    BadPc { func: usize, pc: u32, len: usize },
    /// `pc` hợp lệ nhưng hàm không có span tương ứng (module hỏng; `verify` thường đã chặn
    /// `SpanMismatch`, nhưng `verify_fee` phải chịu được lời gọi trực tiếp).
    SpanMissing { func: usize, pc: u32 },
    /// `site` nằm ngoài **phạm vi văn bản nguồn** suy từ span của mọi lệnh (ràng buộc suy từ
    /// cấu trúc, không tin artifact). Không buộc `site` nằm trong span của **đúng** lệnh tại
    /// `pc`: sự kiện `E_STMT` mang span **cả câu lệnh**, còn span lệnh đầu tiên là span hẹp của
    /// biểu thức con — ràng buộc cũ từ chối chính artifact do codegen sinh (ST-290).
    BadSite {
        func: usize,
        pc: u32,
        site: u32,
        start: u32,
        end: u32,
    },
    /// Bảng rỗng nhưng module có công việc ⇒ **không** được coi là "chương trình không tốn công".
    EmptyTable,
    /// Một chu trình CFG (trong hàm) không có điểm phí nào ⇒ có thể chạy mãi không bị tính.
    CycleUncharged { func: usize, pc: usize },
    /// Cạnh nhảy lùi vào đầu vòng mà đầu vòng không có điểm phí guard.
    LoopGuardUncharged { func: usize, pc: usize },
    /// Chu trình đệ quy qua lời gọi mà không có điểm phí nào trong chu trình.
    CallCycleUncharged { func: usize },
}

/// Nối vào `VerifyError` để giữ một cửa nạp (`load_verified_with_fee`).
pub use crate::verify::VerifyError;

/// Kiểm **cấu trúc + CFG** của bảng phí. Không đọc/tin trọng số (không có trong artifact).
pub fn verify_fee(m: &Module, fee: &FeeTable) -> Result<(), FeeVerifyError> {
    if fee.funcs.len() != m.funcs.len() {
        return Err(FeeVerifyError::FuncCount {
            got: fee.funcs.len(),
            want: m.funcs.len(),
        });
    }
    let total = fee.total();
    if total > MAX_FEE_ENTRIES {
        return Err(FeeVerifyError::TooMany(total));
    }
    // **ST-290** — biên trên của văn bản nguồn suy từ **mọi** span lệnh. `site` hợp lệ chỉ cần
    // nằm trong biên này (xem `BadSite`), không cần nằm trong span hẹp của đúng lệnh tại `pc`.
    let max_end: u32 = m
        .funcs
        .iter()
        .flat_map(|f| f.spans.iter())
        .map(|s| s.end)
        .max()
        .unwrap_or(0);
    for (fi, f) in m.funcs.iter().enumerate() {
        let mut per_pc: std::collections::HashMap<u32, usize> = std::collections::HashMap::new();
        for e in &fee.funcs[fi] {
            if e.pc as usize >= f.code.len() {
                return Err(FeeVerifyError::BadPc {
                    func: fi,
                    pc: e.pc,
                    len: f.code.len(),
                });
            }
            let k = per_pc.entry(e.pc).or_insert(0);
            *k += 1;
            if *k > MAX_PER_PC {
                return Err(FeeVerifyError::TooMany(total));
            }
            // `site` do artifact tự khai ⇒ chỉ tin khi nó nằm trong phạm vi văn bản nguồn.
            // Không tin bất biến `code.len() == spans.len()` của `verify`: `verify_fee` là API
            // công khai, có thể nhận `Module` dựng tay (test/verifier khác) ⇒ không được panic.
            f.spans
                .get(e.pc as usize)
                .ok_or(FeeVerifyError::SpanMissing { func: fi, pc: e.pc })?;
            if e.site != 0 && e.site > max_end {
                return Err(FeeVerifyError::BadSite {
                    func: fi,
                    pc: e.pc,
                    site: e.site,
                    start: 0,
                    end: max_end,
                });
            }
        }
        check_cfg(fi, f, &fee.funcs[fi])?;
    }
    check_call_cycles(m, fee)?;
    // Đặt CUỐI: lỗi cấu trúc/CFG cụ thể phải hiện trước thông điệp "bảng rỗng".
    if total == 0 && m.funcs.iter().any(|f| !f.code.is_empty()) {
        return Err(FeeVerifyError::EmptyTable);
    }
    Ok(())
}

/// Các successor **hợp lệ** của lệnh tại `pc` (cùng quy ước với `verify::need_stack`).
///
/// `verify` cho phép đích nhảy bằng `code.len()` (cuối code) vì nó có thể **không bao giờ chạy**
/// (lệnh không tới được). Với verifier phí, `code.len()` là **terminal**, không phải một PC để
/// tra `charged[n]`/`idx[n]` — làm vậy là panic trên module crafted.
fn succs(code: &[Instr], pc: usize) -> Vec<usize> {
    let n = code.len();
    let off = |o: i32| -> Option<usize> {
        let t = pc as i64 + o as i64;
        if t < 0 {
            // Giữ quy ước cũ (kẹp về 0) cho module dựng tay sai; đây chỉ là đồ thị **suy**,
            // không phải đường thi hành. Không bao giờ để chỉ số âm lọt vào mảng.
            Some(0)
        } else if t >= n as i64 {
            // `code.len()` là terminal: `verify` cho phép đích này cho nhánh không tới được.
            None
        } else {
            Some(t as usize)
        }
    };
    let sau = if pc + 1 < n { Some(pc + 1) } else { None };
    match code[pc] {
        Instr::Return | Instr::Halt => Vec::new(),
        Instr::Jump(o) => off(o).into_iter().collect(),
        Instr::JumpIfFalse(o)
        | Instr::JumpIfNotTrue(o)
        | Instr::JumpIfNone(o)
        | Instr::JumpIfSome(o)
        | Instr::IterNext(o) => off(o).into_iter().chain(sau).collect(),
        _ => sau.into_iter().collect(),
    }
}

/// (1) Mọi chu trình trong CFG hàm phải có điểm phí; (2) cạnh lùi vào đầu vòng phải có guard.
///
/// Dùng **Kahn** (lặp, không đệ quy) trên đồ thị con các PC **không có phí**: bỏ các đỉnh có phí
/// thì chu trình còn lại chính là chu trình không được tính. Cách này tránh cả `idx[n]` lẫn tràn
/// stack khi code rất dài (module không đáng tin).
fn check_cfg(func: usize, f: &crate::format::Func, fee: &[FeeEntry]) -> Result<(), FeeVerifyError> {
    let n = f.code.len();
    if n == 0 {
        return Ok(());
    }
    let charged: Vec<bool> = (0..n)
        .map(|pc| fee.iter().any(|e| e.pc as usize == pc))
        .collect();
    // Cạnh lùi: đích <= nguồn ⇒ đầu vòng phải có điểm phí (guard charge, VD-256 §3).
    for pc in 0..n {
        for s in succs(&f.code, pc) {
            if s <= pc && !charged[s] {
                return Err(FeeVerifyError::LoopGuardUncharged { func, pc: s });
            }
        }
    }
    let so_khong_phi = charged.iter().filter(|b| !**b).count();
    if so_khong_phi == 0 {
        return Ok(());
    }
    // Đồ thị con chỉ gồm PC không có phí (đỉnh có phí bị bỏ ⇒ cạnh đi qua nó cũng bỏ).
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n];
    let mut indeg = vec![0usize; n];
    for pc in 0..n {
        if charged[pc] {
            continue;
        }
        for s in succs(&f.code, pc) {
            if !charged[s] {
                adj[pc].push(s);
                indeg[s] += 1;
            }
        }
    }
    let mut hang: std::collections::VecDeque<usize> = (0..n)
        .filter(|&pc| !charged[pc] && indeg[pc] == 0)
        .collect();
    let mut da_xet = 0usize;
    while let Some(u) = hang.pop_front() {
        da_xet += 1;
        for &v in &adj[u] {
            indeg[v] -= 1;
            if indeg[v] == 0 {
                hang.push_back(v);
            }
        }
    }
    if da_xet != so_khong_phi {
        let pc = (0..n)
            .find(|&pc| !charged[pc] && indeg[pc] > 0)
            .unwrap_or(0);
        return Err(FeeVerifyError::CycleUncharged { func, pc });
    }
    Ok(())
}

/// Đệ quy qua lời gọi: mỗi SCC của đồ thị gọi có chu trình phải có ít nhất một điểm phí.
fn check_call_cycles(m: &Module, fee: &FeeTable) -> Result<(), FeeVerifyError> {
    let nf = m.funcs.len();
    let mut edge: Vec<Vec<usize>> = vec![Vec::new(); nf];
    for (fi, f) in m.funcs.iter().enumerate() {
        for ins in &f.code {
            let t = match *ins {
                Instr::Call(t, _) | Instr::MakeClosure(t, _) => Some(t as usize),
                _ => None,
            };
            if let Some(t) = t {
                if t < nf {
                    edge[fi].push(t);
                }
            }
        }
    }
    let has_fee: Vec<bool> = fee.funcs.iter().map(|v| !v.is_empty()).collect();
    let so_khong_phi = has_fee.iter().filter(|b| !**b).count();
    if so_khong_phi == 0 {
        return Ok(());
    }
    // Đồ thị con các hàm **không có** điểm phí; Kahn lặp (không đệ quy) như `check_cfg`.
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); nf];
    let mut indeg = vec![0usize; nf];
    for fi in 0..nf {
        if has_fee[fi] {
            continue;
        }
        for &v in &edge[fi] {
            if !has_fee[v] {
                adj[fi].push(v);
                indeg[v] += 1;
            }
        }
    }
    let mut hang: std::collections::VecDeque<usize> = (0..nf)
        .filter(|&fi| !has_fee[fi] && indeg[fi] == 0)
        .collect();
    let mut da_xet = 0usize;
    while let Some(u) = hang.pop_front() {
        da_xet += 1;
        for &v in &adj[u] {
            indeg[v] -= 1;
            if indeg[v] == 0 {
                hang.push_back(v);
            }
        }
    }
    if da_xet != so_khong_phi {
        let func = (0..nf)
            .find(|&fi| !has_fee[fi] && indeg[fi] > 0)
            .unwrap_or(0);
        return Err(FeeVerifyError::CallCycleUncharged { func });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_khong_phan_voi_1_2_va_module_version() {
        const {
            assert!(VERSION != 1 && VERSION != 2 && VERSION != 3);
            // Bốn trục độc lập: module 2/3, event-table 4, weight 1.
            assert!(crate::format::VERSION == 2);
            assert!(crate::format::INSTRUMENTED_VERSION == 3);
            assert!(EVENT_TABLE_VERSION == 4);
            assert!(WEIGHT_TABLE_VERSION == 1);
        }
        for k in FeeKind::ALL {
            assert_eq!(FeeKind::from_tag(k.tag()), Some(k));
            assert!(!k.name().is_empty());
        }
        assert_eq!(FeeKind::from_tag(0), None);
        assert_eq!(FeeKind::from_tag(99), None);
    }

    #[test]
    fn nhieu_su_kien_cung_pc_giu_thu_tu() {
        let mut t = FeeTable::new(1);
        t.push(0, FeeEntry::new(2, FeeKind::Call, 10));
        t.push(0, FeeEntry::new(2, FeeKind::Expr, 11));
        t.push(0, FeeEntry::new(2, FeeKind::Assign, 12));
        let tr: Vec<_> = t.trace().iter().map(|e| e.kind).collect();
        assert_eq!(tr, vec![FeeKind::Call, FeeKind::Expr, FeeKind::Assign]);
    }
}
