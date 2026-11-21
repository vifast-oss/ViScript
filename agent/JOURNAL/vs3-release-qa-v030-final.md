<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3 QA — FINAL v0.3.0 candidate `0d26527` / code `6f0ea95`

> **Người chạy**: W3 (QA/release), 2026-11-21 16:54–17:00 JST.
> **Phạm vi**: chỉ đo; **không sửa code/docs chung**, không tag/push/merge.
> **Local only — không dùng GitHub Actions.**

## 0. Mốc

| Mục | Giá trị |
|---|---|
| `main` | **`0d26527`** (docs v0.3.0 final; feature freeze) |
| Code | **`6f0ea95`** — HTTP/2 default-on + h2 idle/queue, W1f async cap≥1, W5 worker-copy |
| Branch QA | `vs3/w3-registry`, merge `main` → merge commit **`c95bbf8`**; cây sản phẩm = `6f0ea95` |
| Tag | `v0.1.0` = `50675dd`; `v0.2.0` = `42b700c` (không đổi) |
| Gate kỳ vọng | **3496 passed / 3 skipped** |
| Disk | đầu **7,1G**; thấp nhất **5,5G** (> 5G); fresh-clone xong xoá, còn **6,8G** |
| Toolchain | `~/.cargo/bin/cargo` 1.100.0 |

## 1. Kết quả local

| # | Lệnh | Kết quả | Thời gian | Log |
|---|---|---|---|---|
| 1 | `./vis-dev gate --fast` | **PASS — 3496 passed / 3 skipped** | 163,03 s (nextest 35,11 s) | `/tmp/opencode/qa-w3-v030-final-gate.log` |
| 2 | `./vis-dev nightly --skip-gate --fuzz-minutes 2` | **PASS** — fuzz **18.090.482 ca / 0 vi phạm**; fmt 475 tệp bất động | 120,73 s | `/tmp/opencode/qa-w3-v030-final-nightly.log` |
| 3 | `VIS_DIFF_N=100000 VIS_DIFF_SEED=202611211657 ./bin/vis-diff` | **PASS — 100.000 ca, lệch 0** | 55,74 s | `/tmp/opencode/qa-w3-v030-final-diff.log` |
| 4 | `./bin/vis-host-suite` | **PASS — 54/54** + `host_posix_real` PASS | 1,53 s | `/tmp/opencode/qa-w3-v030-final-host.log` |
| 5 | `./bin/vis-targets` | **PASS** — core **18/18**; sqlite **16 PASS + 2 EXPECTED_RED** (#156) | 6,56 s | `/tmp/opencode/qa-w3-v030-final-targets.log` |
| 6 | Fresh-clone `0d26527` (xem journal riêng) | **PASS — 3496/3 + host 54/54** | 86,52 s + 2,83 s | `/tmp/opencode/qa-w3-freshclone-v030-final.log` |

- Gate có **8 ca `vs3_http2` default-on** pass, gồm `h2_idle_timeout_dong_phien`,
  `h2_tai_su_dung_ket_noi`, `registry_https_thuong_luong_h2`.
- Gate có `vs3_default`/`vs3_chan_timeout`/`vs3_worker` pass; sync vẫn byte-identical.
- Diff prelude 2.000 ca seed cố định `20261115` lệch 0; 100k seed mới `202611211657` lệch 0.
- Targets: core 18/18; sqlite 16 PASS + 2 `EXPECTED_RED` đúng nợ #156 (C toolchain msvc/wasm).
- **Không có test đỏ** trên Linux local.

## 2. Checklist v0.3.0

### 2.1 HTTP/2 default-on

| # | Hạng mục | Bằng chứng | Trạng thái |
|---|---|---|---|
| 4b2.1 | `h2` review license/unsafe/transitive | W2 có báo cáo `BAO-CAO-VS3-4B2-H2-IMPLEMENT`/`H2-FLIP`/`H2-IDLE`; QA này không lặp review độc lập | **Chấp nhận theo W2** |
| 4b2.2 | Default-on | Gate mặc định chạy 8 ca `vs3_http2` (không cần `--features`) | **PASS local** |
| 4b2.3 | Test h2 local + fallback | 8/8 pass, gồm idle timeout/reuse/auth/redirect/fallback HTTP/1.1 | **PASS** |
| 4b2.4 | Không hồi quy HTTP/1.1/TLS/registry | gate 3496/3; TLS/keepalive/registry trong gate | **PASS** |
| 4b2.5 | Windows h2 | W4 đã verify `a26ec4b` (3450/0); **chưa** verify `6f0ea95` | **CHƯA — blocker tag** |
| 4b2.6 | Fallback nếu h2 lỗi | Giữ HTTP/1.1; HTTP/2 → `v0.3.1` theo VD-271 nếu cần | Sẵn sàng |

### 2.2 Windows verify (local/psvn, không GitHub Actions)

- [ ] W4 chạy lại `cargo test --workspace` + `vis-dev check` tại **`6f0ea95`** (code final) trên `psvn`;
  kỳ vọng 0 product failure, h2/worker có mặt trong suite.
- [x] W4 đã verify `a26ec4b` (3450/0) — **không đủ** thay cho code final.
- [ ] `vis-targets` psvn core 18/18; sqlite 17/1 (theo W4), không thêm ô đỏ ngoài #156.
- [ ] Không tuyên bố Windows Tier 1; macOS ngoài phạm vi.

### 2.3 Net/package + worker + async

- [x] TLS, keep-alive, cookie/auth, registry HTTPS, Ed25519/GC đều nằm trong gate 3496/3.
- [x] Worker opt-in/copy payload (W5g) có test trong gate; default vẫn single-thread.
- [x] Async default (1d) + timeout kênh (2b) không hồi quy.
- [ ] Tàn dư đã ghi từ v0.2.0: async net production, worker evaluator, `select` VS3-2c — **ngoài
  phạm vi tag v0.3.0**, không chặn nếu release notes ghi rõ.

### 2.4 Điều kiện tag v0.3.0

- [x] Gate local **3496/3**, nightly PASS, diff 100k lệch 0, host 54/54, core targets 18/18.
- [x] Fresh-clone `0d26527` PASS (gate 3496/3 + host 54/54).
- [x] HTTP/2 default-on local pass (8 ca), feature freeze giữ.
- [ ] **Windows psvn `6f0ea95`** (W4) — blocker duy nhất còn lại.
- [ ] A0 tạo/push tag; W3 không tự tag.

## 3. Đề xuất

**Đề xuất A0 tag `v0.3.0` tại code `6f0ea95` (docs `0d26527`) NGAY SAU KHI W4 xác nhận psvn
`6f0ea95` xanh** (0 product failure). Toàn bộ bằng chứng Linux local + fresh-clone đã xanh; không
còn thay đổi code sau feature freeze. Nếu W4 chưa kịp, **không** tag sớm và ghi rõ Windows chưa
verify.

## 4. Blocker / rủi ro

1. **Windows `6f0ea95` chưa verify** — blocker duy nhất cho tag v0.3.0.
2. **Đĩa thấp**: low-water **5,5G** (sát ngưỡng 5G). Đã xoá clone/fresh target; còn 6,8G. Nếu chạy
   thêm full Windows/fresh-clone lần nữa cần dọn target cũ.
3. **2 ô EXPECTED_RED** msvc/wasm do #156 C toolchain — không đổi nhãn PASS.
4. **Không GitHub Actions**: mọi bằng chứng local Linux; Windows do W4 psvn thủ công.
5. Tàn dư sản phẩm đã biết (async net production, worker evaluator, `select` VS3-2c) — ghi trong
   release notes, không chặn tag theo phạm vi đã chốt.

## 5. Lệnh tái lập

```bash
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_TARGET_DIR=/home/ps/ViScript/target

./vis-dev gate --fast
./vis-dev nightly --skip-gate --fuzz-minutes 2
VIS_DIFF_N=100000 VIS_DIFF_SEED=202611211657 ./bin/vis-diff
./bin/vis-host-suite
./bin/vis-targets
```

**Kết luận**: code final `6f0ea95` / docs `0d26527` **xanh local Linux** (gate **3496/3**, nightly
PASS, diff 100k lệch 0, host 54/54, targets 18/18, fresh-clone PASS). Đề xuất tag `v0.3.0` sau khi
W4 xác nhận Windows `6f0ea95`.
