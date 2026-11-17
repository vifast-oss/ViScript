# ST-171 — Tách ViScript thành repo riêng "ViScript" (báo cáo)

**Ngày**: 2026-11-17 · **Người làm**: Agent 2 dòng ViScript · **Ngân sách**: 3 h (dừng mềm 00:00 JST)
**Nguồn quyết định**: VD-165 (tách repo) · VD-167(1) (chuyển cả `docs/viscript/`) · VD-179 §4 (phạm vi)
**Trạng thái**: **DONE phần tách + kiểm** · **2 mục BLOCKED-CHO-A0** (§8) · **chưa commit** (Agent 1 commit)

| | |
|---|---|
| Repo cũ | `/Users/mps/dvchd/ViFastOs` — HEAD lúc tách `210f9d67` (nay `91215d07`, 2 commit OS-only không chạm ViScript) |
| Repo mới | **`/Users/mps/dvchd/ViScript`** — HEAD `9b1466b` (bản tách) + **50 tệp sửa / 15 tệp mới chưa commit** |
| Lịch sử | **459 commit** giữ lại (lọc từ **1791** commit của `ViFastOs`), trong đó **454** commit `[ST-…]` |
| Tệp theo dõi | **2191** = 2180 `viscript/` + 3 `docs/viscript/` + 8 `vis-host-*` |
| Cổng mac | **PASS 10/10**, L0 **1184/0** (131 suite, 251 s) — log `JOURNAL/st171-gate-macos-lan7.log` |
| Cổng ps | **PASS 10/10**, L0 **1188/0** (131 suite, 39 s) — log `JOURNAL/st171-gate-ps-lan2.log` |
| `check_report` | **PASS (29 báo cáo, 197 hàng QUEUE)** — log `JOURNAL/st171-check-report.log` |

---

## 1. Phạm vi tách (VD-179 §4)

Xác định bằng **`cargo metadata --manifest-path viscript/Cargo.toml --no-deps`** (không viết tay) + đọc
`Cargo.toml` từng crate:

```
workspace_root: /Users/mps/dvchd/ViFastOs/viscript
8 crate: vis-diag · vis-syntax · vis-cli · vis-bytecode · vis-types · vis-fmt · vis-interp · vis-vm
```

| Nhóm | Nội dung | Tệp tracked |
|---|---|---|
| `viscript/` | `crates/` (220) · `test/` (1208) · `agent/` (733) · `bin/` (9) · `infra/` (6) · `Cargo.toml` · `Cargo.lock` · `AGENTS.md` · `.gitignore` | **2180** |
| `docs/viscript/` | `SPEC-lex.md` · `ViScript-thiet-ke.md` · `doc-examples-allow.txt` | **3** |
| `vis-host-linux/` | workspace riêng, **path dep** của `vis-cli` (Linux-only) | **5** |
| `vis-host-wasm/` | workspace riêng, chỉ dùng cho `bin/vis-targets` | **3** |
| | **Tổng** | **2191** |

### 1.1 Path dep trỏ ra ngoài `viscript/` — **BLOCKED-CHO-A0**

`crates/vis-cli/Cargo.toml:24`:

```toml
[target.'cfg(target_os = "linux")'.dependencies]
vis-host-linux = { path = "../../../vis-host-linux" }
```

Đây **đúng** là trường hợp luật ST-171 nói phải **báo BLOCKED-CHO-A0, không tự quyết**. Đã ghi
`agent/BLOCKED.md` §7.2. **Cách xử lý tạm (để repo mới build được, không cần tệp nào từ repo cũ)**:
đưa `vis-host-linux/` + `vis-host-wasm/` vào repo mới (2 workspace riêng, `exclude` ở `Cargo.toml` gốc)
và đổi path dep `../../../vis-host-linux` → `../../vis-host-linux`. **A0 cần chốt** (a) giữ trong repo
`ViScript` (đang làm) hay (b) tách repo `vis-host` riêng; (c) repo `ViFastOs` có xoá 2 thư mục này không.

### 1.2 Kiểm "không cần tệp nào từ repo cũ"

`cargo build --workspace` + `./vis-dev gate --fast` chạy **trong** `/Users/mps/dvchd/ViScript` với
`PATH` chỉ có toolchain — **không** đọc gì từ `/Users/mps/dvchd/ViFastOs`. Bằng chứng: cổng mac PASS
10/10 (§4) và cổng ps PASS 10/10 trên **bản copy độc lập** `/root/viscript-moi` (§5).

---

## 2. Cách tách

```bash
git clone --no-hardlinks /Users/mps/dvchd/ViFastOs /tmp/st171-split
cd /tmp/st171-split
git filter-repo --force \
  --path viscript/ --path docs/viscript/ \
  --path vis-host-linux/ --path vis-host-wasm/ \
  --path-rename viscript/:
# → 459 commit, 0,63 s; repack 3,02 s
cp -a /tmp/st171-split /Users/mps/dvchd/ViScript
```

`git-filter-repo` có sẵn (`/opt/homebrew/bin/git-filter-repo`), nên **không** phải dùng
`git subtree split`. Lịch sử **giữ đủ**: `git log --oneline | wc -l` = **459** > 0 và có **454** commit
`[ST-…]` cũ (commit đầu `a09768c [KT-00] vòng vá sau review + duyệt Agent 0`, commit cuối `9b1466b [ST-170]`).

### 2.1 Bố cục repo mới — **chọn `docs/viscript/` (giữ tên thư mục)**

`viscript/` thành **gốc** repo; `docs/viscript/` **giữ nguyên** tên (không đổi thành `docs/`) — lựa chọn
này **ít sửa đường dẫn hơn** (mọi `docs/viscript/...` trong mã + tài liệu + `doc_examples.py` giữ nguyên),
đúng như luật ST-171 cho phép ("hoặc giữ `docs/viscript/` nếu đơn giản hơn — ghi rõ lựa chọn").

```
/Users/mps/dvchd/ViScript/
├── AGENTS.md          (từ viscript/AGENTS.md, sửa liên kết ../ → nội bộ)
├── Cargo.toml         (workspace 8 crate + exclude 2 host crate)
├── Cargo.lock
├── LICENSE-MIT · LICENSE-APACHE · TRADEMARKS.md   (chép từ repo cũ)
├── .gitignore         (target/ .vis/ __pycache__/ *.pyc .DS_Store)
├── vis-dev · vis      (shim gốc → bin/vis-dev, bin/vis)
├── crates/            (8 crate)
├── bin/               (vis, vis-dev, vis-diff, vis-host-suite, vis-linux, vis-lock, vis-remote, vis-soak, vis-targets)
├── test/              (check cmd doc-examples expr fixtures fmt host modules pkg report sem std)
├── agent/             (QUEUE STATE BLOCKED DECISIONS JOURNAL reports SO-NO ROADMAP HOST-BAO-DAM API-*)
├── infra/             (linux/ windows/)
├── docs/viscript/     (SPEC-lex.md · ViScript-thiet-ke.md · doc-examples-allow.txt)
├── vis-host-linux/    (workspace riêng)
└── vis-host-wasm/     (workspace riêng)
```

---

## 3. Cập nhật đường dẫn trong repo mới

Nguyên tắc: **đường dẫn thực thi** (script, Python, Rust) + `AGENTS.md` sửa hết; **tài liệu lịch sử**
(`agent/JOURNAL/`, `agent/reports/*.md`, `agent/QUEUE.md`, `agent/DECISIONS.md`) **giữ nguyên văn** vì
chúng là **bản ghi** của cây cũ — sửa sẽ làm sai bằng chứng. Ghi rõ ở đây để A0/Agent 1 biết.

| Tệp | Sửa |
|---|---|
| `crates/vis-cli/Cargo.toml` | `path = "../../../vis-host-linux"` → `"../../vis-host-linux"` |
| `Cargo.toml` | thêm `exclude = ["vis-host-linux", "vis-host-wasm"]` (2 workspace riêng nằm trong cây) |
| `crates/vis-cli/tests/check.rs` · `crates/vis-fmt/tests/fmt.rs` · `crates/vis-syntax/tests/doc_cmd.rs` · `crates/vis-syntax/tests/doc_fixtures.rs` · `crates/vis-types/tests/hir.rs` · `crates/vis-diag/src/lib.rs` | `../../../docs/viscript` → `../../docs/viscript` (đường dẫn theo `CARGO_MANIFEST_DIR`/CWD). **`include!("../../../test/doc_excluded.rs")` GIỮ NGUYÊN** — `include!` tính theo **tệp nguồn**, bỏ một cấp `viscript/` không đổi |
| `crates/vis-cli/tests/sync_list.rs` | `goc_repo()` `../../..` → `../..`; `script()` `viscript/bin/vis-linux` → `bin/vis-linux`; `DIRS` 5 → **9**, `FILES` 3 → **5**, `TAT_CA` 8 → **14**; kỳ vọng `1/8` → `1/14` |
| `test/report/check_report.py` | `ROOT` `parents[3]` → `parents[2]`; `REPORTS`/`QUEUE`/`BLOCKED` bỏ tiền tố `viscript/`; `git show <head>:viscript/agent/QUEUE.md` → `:agent/QUEUE.md` |
| `test/report/run_fixtures.sh` | `cd ../../..` → `cd ../..`; `viscript/test/report/...` → `test/report/...` |
| `test/doc-examples/doc_examples.py` | `REPO` `parents[3]` → `parents[2]`; `FIXTURES` bỏ `viscript/`; `REPO/viscript/target/debug/vis` → `REPO/target/debug/vis`; `REPO/viscript/crates` → `REPO/crates`; `REPO/viscript/Cargo.toml` → `REPO/Cargo.toml`; `root/viscript/test/doc-examples` → `root/test/doc-examples` |
| `test/fixtures/run_fixtures.sh` | `mk_qd` + ca 9/11: `$tmp/N/viscript/test/doc-examples` → `$tmp/N/test/doc-examples`; `$VIS_ROOT/../docs/viscript` → `$VIS_ROOT/docs/viscript` |
| `bin/vis-dev` | `REPO_ROOT` = `VIS_ROOT` (gốc repo **là** cây ViScript); `viscript/test/report/...` → `test/report/...` |
| `bin/vis-remote` | `REPO_ROOT` = `VIS_ROOT`; **`ps` → `/root/viscript-moi`**, **`psvn` → `/c/Users/ps/viscript-moi`** (VD-165(f)) |
| `bin/vis-linux` | `REPO_ROOT` = `VIS_ROOT`; `SYNC_DIRS` = `crates bin test agent infra docs vis-host-linux vis-host-wasm .git` (9); `SYNC_FILES` = `AGENTS.md Cargo.toml Cargo.lock vis-dev vis` (5); `bash viscript/bin/vis-dev` → `bash bin/vis-dev`; `bash viscript/infra/linux/setup.sh` → `bash infra/linux/setup.sh` |
| `bin/vis-targets` | `ROOT` `../..` → `..`; `MANIFEST` `$ROOT/viscript/Cargo.toml` → `$ROOT/Cargo.toml` |
| `AGENTS.md` | viết lại phần đầu cho repo tự chứa (`../AGENTS.md`, `../docs/QUYET-DINH-sau-review.md` → ghi rõ **nằm ở repo `ViFastOs`**; `docs/viscript/...` → đường dẫn nội bộ; `viscript/agent/...` → `agent/...`); thêm §4 **Lệnh** |
| **mới** `vis-dev`, `vis` | shim gốc repo → `bin/vis-dev`, `bin/vis` (AGENTS §6 gọi `./vis-dev`) |
| **mới** `docs/viscript/doc-examples-allow.txt.license` | R3 đòi sidecar `.license` cho `*.txt`; ở repo cũ tệp này **ngoài** `viscript/` nên R3 không quét, nay `docs/` nằm trong cây ⇒ phải có |
| **mới** `.gitignore` · `LICENSE-MIT` · `LICENSE-APACHE` · `TRADEMARKS.md` | repo tự chứa |

### 3.1 SHA trong báo cáo `R1..R29` — **ánh xạ lại** (bắt buộc)

`git filter-repo` **viết lại SHA** (459/1791 commit giữ lại). `check_report.py` kiểm `head` và
`tasks[].commits` phải **tồn tại trong git** + `head` là **tổ tiên HEAD** ⇒ phải ánh xạ lại.

- Dùng `.git/filter-repo/commit-map`; **3136** tham chiếu SHA trong 29 báo cáo JSON đã ánh xạ.
- **7** tham chiếu trỏ commit **bị prune** (commit chỉ chạm `v2/`/`crates/` dòng OS, không chạm cây
  ViScript) ⇒ dùng **tổ tiên còn sống gần nhất**, ghi rõ từng dòng trong
  **`agent/JOURNAL/st171-commit-map.txt`** (kèm `.license`).
  - `R24.head` = `3079dede` (`[KT-00] D2-105 …`, chỉ sửa `v2/agent/DECISIONS.md`).
  - `ST-119` trong R23..R29 ghi `324d1da9` — thực chất là commit `[KT-66b]` của **dòng OS** (lỗi dữ
    liệu có từ trước, không phải do tách repo); ánh xạ sang tổ tiên còn sống.
- **Không** sửa `agent/reports/*.md` (bản ghi lịch sử); SHA trong phần văn xuôi của `.md` là SHA **cũ**
  — tra bảng ánh xạ khi cần.

---

## 4. Cổng trên macOS (repo mới)

| Lần | Kết quả | L0 | Suite | Giây | Log |
|---|---|---|---|---|---|
| 1 | **đỏ** — clippy: `include!("../../test/doc_excluded.rs")` sai (tôi đổi nhầm; `include!` tính theo tệp nguồn) | — | — | 10 | `st171-gate-macos.log` |
| 2 | **đỏ** — R3: thiếu sidecar `.license` cho `docs/viscript/doc-examples-allow.txt` + `st171-commit-map.txt` | — | — | 19 | `st171-gate-macos-lan2.log` |
| 3 | **đỏ** — self-test: cây tạm còn tiền tố `viscript/` (`$tmp/9/viscript/test/doc-examples`) | — | — | 22 | `st171-gate-macos-lan3.log` |
| 4 | **đỏ** — `sync_list`: `goc_repo()` còn `../../..` + kỳ vọng `1/8` | — | — | 315 | `st171-gate-macos-lan4.log` |
| 5 | **PASS 10/10** | **1184/0** | 131 | 290 | `st171-gate-macos-lan5.log` |
| 6 | **PASS 10/10** | **1184/0** | 131 | 254 | `st171-gate-macos-lan6.log` |
| **7 (cây cuối cùng)** | **PASS 10/10** | **1184/0** | **131** | **251** | **`st171-gate-macos-lan7.log`** |

**So với R29 (mac)**: R29 = **PASS 10/10, L0 1184/0, 131 suite, 326,2 s** ⇒ **trùng từng con số**
(1184/0, 131 suite). Chênh **thời gian** 254 s vs 326,2 s là do **tải máy** (R29 ghi `load_truoc`
`{1.79 3.31 3.82}`), **không** phải chênh số ca — **không suite nào lệch**.

10 bước cổng đều OK: `fmt` · `clippy` · luật R1..R4 · self-test case âm **12/12** · `vis explain VS0107` ·
`check_report --all` · fixture âm của máy kiểm báo cáo **5/5** · `build` · `doc-examples` (24 khối: 18 xanh
+ 6 ngoại lệ đỏ đúng; 12 fixture QD) · L0 test.

---

## 5. Cổng trên `ps` (bản copy `/root/viscript-moi`)

| Lần | Kết quả | L0 | Suite | Giây | Log |
|---|---|---|---|---|---|
| 1 | **PASS 10/10** | **1188/0** | 131 | 59 | `st171-gate-ps.log` |
| **2 (cây cuối)** | **PASS 10/10** | **1188/0** | **131** | **39** | **`st171-gate-ps-lan2.log`** |

- **Đường dẫn bản copy**: **`/root/viscript-moi`** trên `ps` (Linux x86_64, Ubuntu 26.04.1, 12 core).
  Tạo bằng `rsync -a --delete --exclude target/ --exclude .git/filter-repo/` từ
  `/Users/mps/dvchd/ViScript` (luật ST-171 cho phép "clone mới **hoặc copy**"; **không** clone được vì
  repo mới **chưa có remote** — §8.1).
- **`vis-remote` trỏ sang** (VD-165(f)): `bin/vis-remote` sửa mặc định
  `ps) echo "${VIS_REMOTE_PS_REPO:-/root/viscript-moi}"` (cũ `/root/vifast-linux`).
  Kiểm: `./bin/vis-remote --hosts` → `ps repo=/root/viscript-moi`; `./bin/vis-remote ps --check` → **OK**
  (`repo: /root/viscript-moi`, `head_full: 9b1466b9…`, `.exit` đọc từ tệp đúng).
- **So với R29 (ps)**: R29 = **PASS 10/10, L0 1188/0, 131 suite, 39,2 s** ⇒ **trùng từng con số**
  (1188/0, 131 suite). Lần 2 (cây cuối) **39 s** ≈ R29 **39,2 s**; lần 1 **59 s** do build **nguội** (R29 dùng target đã làm nóng
  từ ST-176) — **không suite nào lệch**.
- **Lưu ý**: cây `/root/viscript-moi` đang **bẩn 61 mục** (bản tách **chưa commit** — Agent 1 commit), nên
  `vis-remote ps` sẽ **từ chối** (exit 6) cho tới khi commit; muốn đo ngay thì `--allow-dirty` (chỉ chẩn đoán).
  Cây cũ `/root/vifast-linux-BAN-CU-KHONG-DUNG` **giữ nguyên**, không xoá/khôi phục.

---

## 6. `psvn` — quy trình `git pull` không bị chặn sau một lần chạy cổng

**Kiểm được** (psvn truy cập được). Cơ chế: `bin/vis-remote` ghi log + `.exit` vào
**`<home máy đích>/vislogs/`** (`psvn`: `/c/Users/ps/vislogs/`), **ngoài repo** ⇒ không sinh tệp
untracked ⇒ `git pull --ff-only` không bị chặn.

Bằng chứng đo trên psvn (repo cũ `/c/Users/ps/vifast-windows/repo`, HEAD `39041b3f`):

| Bước | Lệnh | Kết quả |
|---|---|---|
| 1 | `git status --porcelain \| wc -l` (trước) | **0** |
| 2 | `./bin/vis-remote psvn --tag st171-psvn-pull -- bash -lc 'cd <repo> && git rev-parse --short HEAD && git status --porcelain \| wc -l'` | `39041b3f`, **0**; log máy đích `/c/Users/ps/vislogs/st171-psvn-pull-20261117T120659Z.log` — **ngoài repo** |
| 3 | `git pull --ff-only` | **`PULL_EXIT=0`** (fast-forward tới `39041b3f`, có `R29-viscript.*`); `git status --porcelain` sau = **0** |

**Quy trình chuẩn cho repo `ViScript` mới** (khi A0 cấp remote — §8.1):

```bash
# trên psvn, một lần: clone repo mới
git clone <URL> /c/Users/ps/viscript-moi
# mỗi lần đo: log LUÔN ra ngoài repo
./bin/vis-remote psvn --log /c/Users/ps/vislogs/<task>-gate-psvn.log -- bash -lc 'cd /c/Users/ps/viscript-moi && ./vis-dev gate --fast'
# sau khi đo: cây vẫn sạch ⇒ pull không bị chặn
cd /c/Users/ps/viscript-moi && git status --porcelain && git pull --ff-only
```

**Chưa làm được**: **chưa** clone repo `ViScript` lên psvn vì **chưa có URL remote** (§8.1). `bin/vis-remote`
đã trỏ mặc định `psvn → /c/Users/ps/viscript-moi`; hiện đường đó **chưa tồn tại** nên `vis-remote psvn`
sẽ trả **exit 3 (BLOCKED-ENV)** cho tới khi clone xong. **Không** chạy cổng psvn cho repo mới (ST-171
không yêu cầu; psvn chỉ đo — VD-146).

---

## 7. Repo cũ — chuẩn bị xoá (CHƯA commit)

Đã **stage** (không commit) trong `/Users/mps/dvchd/ViFastOs`:

| Việc | Số tệp | Chi tiết |
|---|---|---|
| Xoá `viscript/` | **2180** | `test/` 1208 · `agent/` 733 · `crates/` 220 · `bin/` 9 · `infra/` 6 · `Cargo.toml` · `Cargo.lock` · `AGENTS.md` · `.gitignore` |
| Xoá `docs/viscript/` | **3** | `SPEC-lex.md` · `ViScript-thiet-ke.md` · `doc-examples-allow.txt` |
| Xoá shim gốc (hệ quả bắt buộc — trỏ `viscript/bin/…` nay không còn) | **2** | `vis-dev` · `vis` |
| **Tổng xoá** | **2185** | `git status --porcelain \| grep -c '^D '` = **2185** |
| Thêm | **1** | **`VISCRIPT.md`** ở gốc — bảng "trước → sau" + đường dẫn repo mới + lịch sử + BLOCKED remote |

`viscript/target/` (7,2 GB, untracked) đã `rm -rf` cùng thư mục; **không** còn `viscript/` hay
`docs/viscript/` trên đĩa ⇒ **không** còn hai bản song song.

### 7.1 Cần Agent 1 / A0 quyết trước khi commit

1. **`vis-host-linux/` + `vis-host-wasm/`** (8 tệp) **vẫn còn** trong repo cũ — nay **trùng** với repo
   mới. Luật ST-171 chỉ nói xoá `viscript/` + `docs/viscript/`, nên tôi **không tự xoá**. Đề nghị:
   xoá kèm (chúng là crate host của ViScript) — hoặc giữ nếu A0 chọn phương án (b) ở §8.2.
2. **`AGENTS.md` gốc** còn nhiều dòng trỏ `viscript/agent/...` (bảng §2/§3) — **Agent 2 không được sửa**
   `AGENTS.md` (AGENTS §3); cần người điều phối / Agent 0 cập nhật.
3. `v2/agent/*` và dòng OS **không** bị ảnh hưởng: `./v2/bin/v2` còn nguyên, `crates/` (OS) còn nguyên.

---

## 8. Còn BLOCKED (BLOCKED-CHO-A0)

Đã ghi `agent/BLOCKED.md` §7.

### 8.1 Remote của repo `ViScript` — **CHƯA CÓ** (chặn bước push + clone psvn)

- **Chặn bởi**: không có URL remote, không có quyền tạo repo `vifast-oss/ViScript`.
- **Đã làm**: repo local đầy đủ, **459 commit**, **không** push (đúng luật "không tự commit/push").
- **Cần**: A0 cấp **URL** + **quyền tạo** `vifast-oss/ViScript`; sau đó
  `git remote add origin <url> && git push -u origin main`.
- **Hệ quả**: `psvn` chưa clone được repo mới (§6); `ps` dùng **bản copy** thay clone (§5).

### 8.2 `vis-host-linux` là path dep **ngoài** `viscript/` — cần A0 chốt vị trí

Xem §1.1. Tạm xử lý: đưa 2 host crate vào repo mới + đổi path dep; **không** cần tệp nào từ repo cũ.

---

## 9. Danh sách tệp để Agent 1 commit

### 9.1 Repo mới `/Users/mps/dvchd/ViScript` — **50 sửa + 15 mới**

**Sửa (50)** — 29 báo cáo + 21 tệp:

```
.gitignore · AGENTS.md · Cargo.toml
agent/BLOCKED.md · agent/STATE.md
agent/reports/R1..R29-viscript.json            (29 tệp — chỉ đổi SHA head/commits)
bin/vis-dev · bin/vis-linux · bin/vis-remote · bin/vis-targets
crates/vis-cli/Cargo.toml
crates/vis-cli/tests/check.rs · crates/vis-cli/tests/sync_list.rs
crates/vis-diag/src/lib.rs
crates/vis-fmt/tests/fmt.rs
crates/vis-syntax/tests/doc_cmd.rs · crates/vis-syntax/tests/doc_fixtures.rs
crates/vis-types/tests/hir.rs
test/doc-examples/doc_examples.py
test/fixtures/run_fixtures.sh
test/report/check_report.py · test/report/run_fixtures.sh
```

**Mới (15)**:

```
LICENSE-MIT · LICENSE-APACHE · TRADEMARKS.md · vis-dev · vis
docs/viscript/doc-examples-allow.txt.license
agent/JOURNAL/st171-commit-map.txt (+ .license)
agent/JOURNAL/st171-gate-macos.log · -lan2 · -lan3 · -lan4 · -lan5 · -lan6 · -lan7
agent/JOURNAL/st171-check-report.log
agent/JOURNAL/st171-gate-ps.log · st171-gate-ps-lan2.log
agent/JOURNAL/st171-bao-cao.md            (báo cáo này)
```

Đề nghị commit message: `[ST-171] tách ViScript thành repo riêng: 459 commit giữ lịch sử, gate mac 1184/0 + ps 1188/0, check_report 29/197`.

### 9.2 Repo cũ `/Users/mps/dvchd/ViFastOs` — **2185 xoá + 1 thêm**

```
D  viscript/            (2180 tệp)
D  docs/viscript/       (3 tệp)
D  vis-dev · vis        (2 tệp)
A  VBSCRIPT.md          (1 tệp)
```

Đề nghị commit message: `[ST-171] xoá mã ViScript khỏi ViFastOs (đã sang repo ViScript) + VBSCRIPT.md trỏ sang`.

---

## 10. Việc chưa xong / ghi rõ

| Việc | Trạng thái |
|---|---|
| Push repo mới lên remote | **BLOCKED-CHO-A0** (§8.1) |
| Clone repo mới trên `psvn` + cổng psvn | **chưa** — chờ remote (§6) |
| Chốt vị trí `vis-host-linux`/`vis-host-wasm` | **BLOCKED-CHO-A0** (§8.2) |
| Xoá `vis-host-*` khỏi repo cũ | **chờ quyết** (§7.1) — hiện **giữ** |
| Cập nhật `AGENTS.md` gốc của `ViFastOs` | **chờ người điều phối / Agent 0** (Agent 2 không được sửa) |
| Sửa SHA trong văn xuôi `agent/reports/*.md` | **không làm** — bản ghi lịch sử; tra `st171-commit-map.txt` |
| `ST-178` (sửa tài liệu theo VD-178) | **mở khoá** — repo `ViScript` đã có; làm ở task riêng |
| `Cargo.toml:26` `repository = "…/ViFastOs"` | **giữ nguyên** — đổi thành `…/ViScript` khi A0 cấp URL thật (§8.1); không ảnh hưởng build/cổng |

**Không** đổi ngưỡng, **không** đổi kỳ vọng ca, **không** thêm crate, **không** `unsafe`, **không** sửa
`spec/`, **không** commit/push.
