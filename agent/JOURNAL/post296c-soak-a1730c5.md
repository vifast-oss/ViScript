<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# Post-ST-296c soak — `a1730c5` → `5d7791c` (v0.4.1)

> **Người chạy**: W3, 2026-11-23 · nhánh **`plan/w3-qa-296c`** (tạo từ `a1730c5`, fast-forward
> `5d7791c`). Không push/merge/tag; không GitHub Actions; không sửa code sản phẩm.
> `VIS_SOAK_TMPDIR=/home/ps/tmp`, `CARGO_BUILD_JOBS=3`, `TMPDIR=/home/ps/tmp`.

## 0. Kết luận

- Lần soak đầu trên `a1730c5` **DỪNG gate-1** vì **T-INFRA do tải**: 3 ca LSP hết hạn 5 s khi
  load **35–48** (W1 đang chạy 30 `yes` + các suite khác), process bị kill (`code -1`), stderr rỗng.
- Tái kiểm riêng 3 ca khi load **3,79**: **15/15 xanh** (5 lần × 3 ca) → xác nhận **T-INFRA**.
- Sau khi W1 dừng tải: fast-forward `5d7791c` (v0.4.1) và chạy **1× soak 10/10 sạch**:
  **PASS — 10/10**, 530 s; 3× gate **3656/4**, 3× diff 100k **lech=0**, host **54/54**, cache **5/5**.
- `vis --version` = **`vis 0.4.1`**; không blocker. **Đủ bằng chứng soak cho tag `v0.4.1`**.

## 1. Lần đỏ đầu trên `a1730c5` (T-INFRA)

- Log: `/home/ps/vislogs/post296c-soak-a1730c5.log`
  sha256 `b3ec822fedb12d21887fee300213545636a40a2eae715633a46a5542ac1a995b`.
- Gate-1 exit 1 sau 531 s; load lúc dừng **35,85 / 34,73 / 21,54** (cao nhất ghi nhận ~48).
- 3 ca đỏ, tất cả `left: -1 right: 0` sau ~5,05 s (deadline harness 5 s), stderr rỗng:
  - `lsp_it lsp_codelens::lsp_codelens_toi_da_200`
  - `lsp_it lsp_implementation::implementation_cap_50`
  - `lsp_it lsp_inlay_hint::inlay_cap_200`
- **Phân loại: T-INFRA do tải**, không retry mù. Không có assertion sản phẩm sai.

## 2. Tái kiểm 3 ca khi load thấp

- Log: `/home/ps/vislogs/post296c-lsp-15.log`
  sha256 `685d44e1e34a0bf9dae0407b87d9a475161b22cf7cde61c106b4eb54312a3dbf`.
- Chạy riêng từng ca, `--exact`, load **3,79**; mỗi ca 5 lần:
  **15/15 PASS**, mỗi lần ~0,02–0,05 s. Kết luận **T-INFRA do tải**, không phải sản phẩm.

## 3. Soak chính thức trên `5d7791c` (v0.4.1)

- `git merge --ff-only 5d7791c` trên `plan/w3-qa-296c` (không commit code).
- Log: `/home/ps/vislogs/post296c-soak-5d7791c.log`
  sha256 `c40a22b4c6b523ceb9c6703d492e4e8e3e2428b9593413887f406baaeb15eabb`.
- **10/10 vòng OK**, wall **530 s**; load đầu 6,74 → cuối 3,51.

| Vòng | Gate | Diff 100k | Host | Cache |
|---|---|---|---|---|
| 1 | **3656/4** | `lech=0` (seed `20261115`) | **54/54** | **5/5** |
| 2 | **3656/4** | `lech=0` (seed `20261115`) | **54/54** | **5/5** |
| 3 | **3656/4** | `lech=0` (seed `20261115`) | — | — |

`vis --version` = **`vis 0.4.1`**.

## 4. Targeted trên `5d7791c`

| Lệnh | Kết quả |
|---|---|
| `cargo test -p vis-interp --lib` | **303/303** |
| `cargo test -p vis-cli --bin vis db::sqlite` | **87/87** |
| `cargo test -p vis-loader` | **29/29** |
| `cargo test -p vis-lsp` | **27/27** |
| `bin/vis-targets` | core **18/18**; sqlite **16 PASS + 2 EXPECTED_RED** |

- Log targeted `5d7791c`: `/home/ps/vislogs/post296c-targeted-5d7791c.log`
  sha256 `950e1c46ab2225b185aa722483c43f87ac0e1d345e2ac514c863a461c14b3e04`.
- `bin/vis-targets`: `/home/ps/vislogs/post296c-targets.log`
  sha256 `9e96d79117f425ebeb05b139a5d7acf0b88bd78def27b463364162f64624ffba`.
- (Targeted `a1730c5` trước đó cũng xanh: 303/87/29/27; `vis-targets` chỉ chạy lại trên `5d7791c`.)

## 5. Blocker

Không. Đĩa còn **22 G** (> 10 G); không commit log lớn (R7). Commit này chỉ thêm journal.
