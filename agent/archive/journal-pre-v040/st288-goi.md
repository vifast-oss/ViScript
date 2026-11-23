# ST-288 · Khép G3 Gói — `VS0708`, `pkg gc --root`, `#142`

Agent 2 (W3) · **2026-11-22** · task **ST-288** · nguồn: [`VD-273`](../DECISIONS.md) (mục 2),
hàng ST-288 trong [`QUEUE.md`](../QUEUE.md).
Worktree `/home/ps/.local/share/opencode/worktree/cd45e5/vs3-w3-registry`, nhánh `plan/w3-platform`,
baseline `1c58ba3` (đã merge `main`). **Không** push/merge/tag.

## 1. Mục tiêu (nguyên văn + cách đo)

1. Cấp **`VS0708`** (Error) cho lỗi xác minh chữ ký/khoá tin cậy registry gói — đủ `vi/en/ja` trong
   `crates/vis-diag/src/lib.rs` + `agent/API-CODES.txt` (quy trình VD-129/VD-273).
2. Khoá **semantics + ca biên `pkg gc --root`**.
3. Vá **#142** — `pkg update <tên>` không ghim cứng các gói khác khi manifest của gói được update
   cho phép/đòi nâng dep chung.
4. Test **đỏ-trước**, `vs3_pkg_gc_multi`, `vs3_registry_sig`, `pkg update` xanh; `./vis-dev gate --fast`;
   fmt/clippy; **R1** (tệp nguồn ≤ 1.500 dòng).

## 2. Đỏ-trước (đo trên mã `1c58ba3` trước khi sửa)

```
cargo test -p vis-cli --features pkg-sig --test vs3_registry_sig
  → 6 failed / 4 passed
    ky_sai_thi_tu_choi            "phải nêu mã VS0708: … chữ ký Ed25519 không khớp …"
    sua_manifest_sau_ky_bi_tu_choi
    thieu_chu_ky_khi_co_khoa      "… thiếu `signature_ed25519` …"
    chu_ky_sai_dinh_dang          "… `signature_ed25519`: cần 128 chữ số hex …"
    khoa_sai_hex                  "… cần 64 chữ số hex (32 byte) …"
    key_rotation_hai_khoa_va_tu_choi_sai

cargo test -p vis-cli --test pkg update_mot_ten_nang_duoc_dep_chung
  → FAILED: "không đổi app 1.0.0 / không đổi other 1.0.0 / không đổi util 0.1.0"
    (app 1.1.0 cần util ^0.2 nhưng resolver bỏ qua bản mới vì util bị ghim cứng =0.1.0)
```

Ca GC mới (`gc_lap_root_long_nhau_khong_dem_trung_va_khong_tao_vis`) **xanh ngay** trên mã cũ: đây là
**ca khoá semantics** (không phải ca sửa lỗi) — hành vi `--root` hiện có đã đúng, ST-288 chỉ đóng
đinh nghĩa + ca biên.

## 3. `VS0708` (VD-273 mục 2)

- `crates/vis-diag/src/lib.rs`: thêm `Code { id: 708, short: "VS0708", … }` vào `MODULE_CODES`
  (sổ `07xx`, **8** mã module; `all_codes()` **54**; `next_block()` vẫn **801**).
  - vi: `lỗi xác minh chữ ký số hoặc khoá tin cậy của registry gói`
  - en: `package registry signature or trusted key verification failed`
  - ja: `パッケージレジストリの署名または信頼キーの検証に失敗しました`
- `agent/API-CODES.txt`: thêm dòng `VS0708\terror: …` (khoá tương thích VD-129).
- Dùng mã ở **mọi** lỗi thuộc miền tin cậy của registry (không chỉ thông điệp):
  - `pkg/registry_http.rs::kiem_chu_ky`: thiếu `signature_ed25519`, hex sai, chữ ký không khớp
    bất kỳ khoá nào (kể cả key rotation);
  - `pkg.rs::khoa_cong_tu_chon`: khoá công khai sai hex/rỗng (cấu hình khoá tin cậy).
- **Không** gán `VS0708` cho lỗi dùng `--registry-key` với registry cục bộ (đó là lỗi cú pháp CLI,
  vẫn exit 2, giữ nguyên câu cũ) và **không** đụng `VS0707` (hash nội dung lệch).
- Test khoá: `crates/vis-diag/tests/messages.rs` (snapshot 12 ca `07xx` × 3 ngôn ngữ, đếm 8 mã module
  / 54 mã), `crates/vis-cli/tests/vs3_registry_sig.rs` (6 ca âm + key rotation đều đòi `VS0708`),
  `api_dong_bang` (API-CODES ↔ code), `explain_moi`/`vd270_explain_json`/`st83_no_nho` (đếm 54).

## 4. `#142` — soft pin thay ghim cứng

**Chính sách đã chốt** (khớp VD-178 QĐ-6 / VD-180: “`pkg update X` chỉ đổi X và phần phụ thuộc bắt
buộc của X, mọi gói khác giữ nguyên”):

- `pkg update` **không tên**: giữ nguyên — giải lại toàn bộ từ `roots`, chọn cao nhất thoả.
- `pkg update <tên>`: **không** thêm root `=<version>` cho mọi gói khác nữa. Thay vào đó resolver
  nhận **soft pin** `tên → phiên bản đang có` (`resolver::Resolver::voi_uu_tien`): bản ưu tiên được
  **thử trước** nếu thoả ràng buộc; không thoả thì resolver vẫn chọn bản cao nhất thoả.
- Hệ quả: gói không liên quan **giữ phiên bản** (kể cả khi registry có bản mới); dep chung **được
  nâng** khi manifest của gói được update đòi (`^0.2` không còn thoả bản 0.1.0); ràng buộc gốc của
  người dùng (root `^1`) vẫn là trần — xung đột thật vẫn `VS0704`, không nới ràng buộc.
- Không đổi định dạng lock (`lock: 2`); lock không đổi ⇒ **không ghi lại** (byte-identical, ca lần 2).
- Lock cũ (`roots` rỗng): `roots_tu_lock` vẫn ghim `=<version>` như trước ⇒ hành vi không đổi (đường
  migrate là `pkg update` không tên / `pkg migrate`).

Ca khoá:

- `crates/vis-cli/tests/pkg.rs::update_mot_ten_nang_duoc_dep_chung` — app 1.1.0 (`util ^0.2`) +
  other 1.1.0 có trong registry; `pkg update app` ⇒ app 1.1.0, **util 0.2.0**, **other 1.0.0**;
  cài đúng cây; lần hai `không đổi` và lock byte-identical.
- `crates/vis-cli/src/pkg/resolver.rs::tests::soft_pin_giu_ban_cu_nhung_khong_chan_nang` — unit:
  không ưu tiên ⇒ cao nhất; ưu tiên cả ba ⇒ bản cũ; bỏ ưu tiên `a` ⇒ `a` lên 1.1.0, **kéo `b`**,
  `c` (không liên quan) giữ 1.0.0.

## 5. `pkg gc --root` — semantics đã khoá

Viết thành hợp đồng ở doc `pkg/gc.rs::lenh`:

1. `--root` **lặp lại được**; mỗi root quét đệ quy `vis.lock`, **không** theo symlink; bỏ
   `.git`/`.vis`/`target`/`node_modules`; giới hạn độ sâu 16 / 10 000 thư mục / 512 lock — vượt ⇒
   **lỗi**, không bỏ qua im lặng (bỏ sót lock có thể xoá blob đang dùng).
2. **Workspace được dọn** = chính các root (canonical) **+** thư mục chứa từng lock; trùng nhau
   (root lồng nhau / hai root cùng cây) chỉ tính **một lần**.
3. Hợp tham chiếu của **mọi** lock trước khi xoá bất cứ thứ gì; `kiem_truoc` kiểm symlink **toàn bộ**
   workspace trước khi xoá mục đầu tiên (không xoá một phần).
4. Root không có `.vis`/store là **no-op**, **không tạo** thư mục; không có lock nào ⇒ từ chối.

Ca mới `crates/vis-cli/tests/vs3_pkg_gc_multi.rs::gc_lap_root_long_nhau_khong_dem_trung_va_khong_tao_vis`:
`--root a --root b --root .` ⇒ in `2 lock, 3 workspace` (dedupe), blob chia sẻ còn, blob mồ côi bị
xoá, `root/.vis` **không** được tạo; lần hai 0/0. Ba ca cũ (chia sẻ nhiều lock, thiếu lock, symlink)
giữ nguyên.

## 6. Kết quả máy

```
./vis-dev gate --fast                      → PASS (fmt + clippy + R1..R6 + self-test + doc-examples + L0)
cargo clippy -p vis-cli -p vis-diag --all-targets --features pkg-sig -- -D warnings → OK
cargo fmt --all -- --check                 → OK
```

Test đích (features `pkg-sig` khi cần):

| Suite | Kết quả |
|---|---|
| `vis-diag` (messages) | 12/12 |
| `pkg` | **25/25** (24 cũ + ca #142) |
| `pkg_lock` | 8/8 |
| `pkg_resolver` | 7/7 |
| `vs3_registry` | 11/11 |
| `vs3_registry_sig` | **10/10** |
| `vs3_pkg_gc` | 5/5 |
| `vs3_pkg_gc_multi` | **4/4** (3 cũ + ca `--root` lặp/lồng) |
| `api_dong_bang` | 3/3 |
| `explain_moi` / `vd270_explain_json` / `st83_no_nho` | 6/6 · 9/9 · 6/6 |
| `cargo test -p vis-cli --bin vis soft_pin` | 1/1 |

R1: `pkg.rs` **1.407** dòng (< 1.500); `resolver.rs` 779; `gc.rs` 446; `vis-diag/src/lib.rs` 959.

## 7. Tệp đổi

- `crates/vis-diag/src/lib.rs`, `crates/vis-diag/tests/messages.rs`
- `agent/API-CODES.txt`
- `crates/vis-cli/src/pkg.rs` (thread `uu_tien` + `VS0708` cho khoá tin cậy)
- `crates/vis-cli/src/pkg/resolver.rs` (soft pin + unit test)
- `crates/vis-cli/src/pkg/registry_http.rs` (`VS0708`)
- `crates/vis-cli/src/pkg/gc.rs` (hợp đồng `--root`)
- `crates/vis-cli/src/pkg_sig.rs` (doc mã lỗi)
- `crates/vis-cli/tests/{pkg.rs,vs3_registry_sig.rs,vs3_pkg_gc_multi.rs,explain_moi.rs,
  vd270_explain_json.rs,st83_no_nho.rs}`
- `agent/{SO-NO.md,QUEUE.md,VS3-TIEN-DO.md}` (đóng `#142`, ST-288 → REVIEW)

## 8. Ghi chú vận hành / blocker

- **Đĩa đầy giữa chừng**: lúc build, `/` còn **54 MB** (các worktree khác đang build: W1 9,6 G,
  W2 7,2 G, W4 6,7 G). Đã xoá **hai cây gate tạm cũ từ 20/11** `/home/ps/.tmp-clone-gate/{st231,verify}`
  (2,9 G + 2,9 G) ⇒ sau đó còn **13 G**. Không đụng target của worktree/agent khác, không dùng target
  `main`; target riêng của worktree này: `target/` (~9,5 G).
- **Không** push/merge/tag; **không** GitHub Actions.
- Không có blocker kỹ thuật còn lại. Ghi chú cho Gatekeeper: `vs3_registry_sig` cần
  `--features pkg-sig` (mặc định tệp rỗng theo `cfg`), đúng như các lần VS3-5b/5c trước.
