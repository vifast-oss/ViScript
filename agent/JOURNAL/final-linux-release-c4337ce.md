<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# Final Linux release QA — ViScript 0.5.0

> **Bản repo là tóm tắt (R7)**; log đầy đủ nằm ngoài repo:
> `/home/ps/vislogs/final-linux-release-c4337ce.log` (sha256 `6317c21d27d32592ad73ed849e2a58ebb48efa61131c9b2abfc83edc42666f02`).
> Tệp gốc trong `vislogs/` có sha256 `85d3cbf9785d6286247eac162ed7a054d8a92521407712cd6b9c9db533942860`;
> bản này chỉ thêm dòng SPDX + nguồn ở đầu.

- **Commit**: `c4337ce` (`c4337ce4`), version `0.5.0`
- **Host**: `ps` local, Linux x86_64, 2026-11-23T19:11+09:00
- **Clone**: `/home/ps/vis-qa-v050-check` — clean before and after; repo not modified
- **Toolchain**: rustc/cargo `1.100.0` (pinned by `rust-toolchain.toml`; `~/.cargo/bin` must be on PATH)
- **Verdict**: **PASS** — no blockers
- **Log**: `final-linux-release-c4337ce.log` — sha256 `6317c21d27d32592ad73ed849e2a58ebb48efa61131c9b2abfc83edc42666f02`

## 1. Full release workspace

```
cargo test --release --workspace --no-fail-fast
```

| Metric | Value |
|---|---|
| Test binaries | 126 |
| Passed | **3753** |
| Failed | **0** |
| Ignored | **5** |
| Exit | 0 |

Expected was ~3752 passed; observed 3753 (+1, within the stated expectation). Zero failures.
Ignored: 3 in the 339-test suite, 1 in the 34-test suite, 1 in a 0-test suite.

## 2. Targeted suites

| Package / target | Result | Exit |
|---|---|---|
| `-p vis-plugin-abi` (`parity.rs`) | 6/6 | 0 |
| `-p vis-plugin-abi` (`vs3_plugin.rs`) | 6/6 | 0 |
| `-p vis-plugin-abi` (unit) | 3/3 | 0 |
| `-p vis-cli --test vs3_plugin` | 4/4 | 0 |
| `-p vish-edit` | 17/17 (9 + 8) | 0 |
| `-p vish-term` | 19/19 | 0 |
| `-p vis-lsp` | 29/29 | 0 |
| `-p vis-loader` | 29/29 | 0 |

## 3. Host conformance suite

```
./bin/vis-host-suite
```

- `vis-host-suite: 54/54 ca xanh (nền: Linux x86_64; các nền khác chưa đo)`
- `vis-host-suite: host_posix_real PASS (lệnh POSIX thật; nền: Linux x86_64)`
- Exit 0

## 4. Version

```
target/release/vis --version  ->  vis 0.5.0
target/debug/vis   --version  ->  vis 0.5.0
```

## 5. Blockers and environment notes

**Blockers: none.**

One environment pitfall, not a code defect: the first invocation picked up the distro
`rustc 1.93.1` because `~/.cargo/bin` was not on `PATH`; cargo rejected
`rust-version = "1.100.0"` and exited 101 before running any test. Re-running with
`PATH=/home/ps/.cargo/bin:$PATH` used the pinned rustc/cargo `1.100.0` and everything passed.
No merge, push, or tag was performed.
