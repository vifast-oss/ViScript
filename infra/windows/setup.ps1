# SPDX-License-Identifier: MIT OR Apache-2.0
# ST-136 (VD-136, VD-140) — tái lập **thành phần toolchain** cho máy đo Windows (`psvn`).
#
# Vì sao: số đo Windows trong VS2 chỉ so sánh được với `ps`/macOS khi **cùng một toolchain**.
# `ps` đo bằng rustc 1.99.1 (58cf8f9c3) và repo khai `rust-version = "1.99.1"`; trên `psvn`
# "stable" KHÔNG phải 1.99.1 (stable = 1.100.0 sau lần đo 2026-11-16) ⇒ mọi lệnh đo phải ghim
# `RUSTUP_TOOLCHAIN=1.99.1-x86_64-pc-windows-msvc`.
#
# Dùng:
#   powershell -NoProfile -File setup.ps1                   # kiểm + cài toolchain 1.99.1 (idempotent)
#   powershell -NoProfile -File setup.ps1 -Check            # CHỈ kiểm; thiếu ⇒ exit 1, không cài
#   powershell -NoProfile -File setup.ps1 -Log C:\env.log   # ghi thêm log môi trường (mặc định $HOME\vis-setup.log)
#
# Mã thoát: 0 xanh · 1 thiếu thành phần (chế độ -Check) · 3 BLOCKED-ENV (không có rustup).
#
# KHÔNG làm gì khác: không `rustup update`, không đổi toolchain mặc định, không cài target phụ,
# không cài Build Tools/gói OS, không reboot (VD-140: Build Tools chỉ cập nhật khi build lỗi do
# bản cũ VÀ có log lỗi trước; trình cài đòi khởi động lại ⇒ dừng, BLOCKED-ENV).
#
# CẢNH BÁO: đừng chạy `rustup`/`cargo` "trần" trong cây repo — `rust-toolchain.toml` khai
# channel = "stable" và rustup sẽ TỰ cài/cập nhật "stable" (đã xảy ra 2026-11-16T07:55:32Z).
# Script này đặt $env:RUSTUP_TOOLCHAIN trước mọi lệnh.

param(
    [string]$Log = "$env:USERPROFILE\vis-setup.log",
    [switch]$Check
)

$ErrorActionPreference = "Stop"
# Console Windows mặc định cp1252 ⇒ đặt UTF-8 để tiếng Việt trong log không bị hỏng.
try { [Console]::OutputEncoding = [Text.Encoding]::UTF8 } catch { }
$Toolchain = "1.99.1-x86_64-pc-windows-msvc"
$WantedTargets = @("wasm32-unknown-unknown", "x86_64-unknown-linux-gnu")   # cần cho bin/vis-targets

function Say([string]$m) { Write-Output $m }
function LogEnv([string]$m) {
    Say $m
    try { Add-Content -Path $Log -Value $m -Encoding UTF8 } catch { }
}

if (-not (Get-Command rustup -ErrorAction SilentlyContinue)) {
    Say "setup: KHÔNG có rustup trong PATH (BLOCKED-ENV)"; exit 3
}

# Ghim toolchain TRƯỚC mọi lệnh rustup/cargo ⇒ không kích hoạt rust-toolchain.toml.
$env:RUSTUP_TOOLCHAIN = $Toolchain

LogEnv "== vis-setup (Windows) $(Get-Date -Format o) =="
LogEnv ("may=" + $env:COMPUTERNAME)
$os = Get-CimInstance Win32_OperatingSystem
LogEnv ("os=" + $os.Caption + " | version=" + $os.Version + " | build=" + $os.BuildNumber)
$rustupV = @(rustup -V)
LogEnv ("rustup=" + ($rustupV -join " / "))

$haveArr = @(rustup toolchain list)
$have = $haveArr -join "`n"
if ($have -notmatch [regex]::Escape($Toolchain)) {
    if ($Check) {
        Say "setup: THIẾU toolchain $Toolchain (chế độ -Check, không cài)"; exit 1
    }
    Say "setup: cài toolchain ĐÚNG phiên bản của ps: $Toolchain"
    rustup toolchain install $Toolchain
    if ($LASTEXITCODE -ne 0) { Say "setup: cài toolchain lỗi ⇒ BLOCKED-ENV"; exit 3 }
}

$rustcVv = @(rustc -Vv)
LogEnv ("rustc=" + ($rustcVv -join " | "))
$cargoV = @(cargo -V)
LogEnv ("cargo=" + ($cargoV -join " "))
$tg = @(rustup target list --installed)
LogEnv ("targets=" + ($tg -join ","))
$cp = @(rustup component list --installed)
LogEnv ("components=" + ($cp -join ","))

# Không tự cài target phụ (ngoài phạm vi được phép của ST-136) — chỉ BÁO.
$installed = (@(rustup target list --installed) -join "`n")
$missing = @()
foreach ($t in $WantedTargets) {
    if ($installed -notmatch [regex]::Escape($t)) { $missing += $t }
}
if ($missing.Count -gt 0) {
    Say ("setup: THIẾU target std: " + ($missing -join " ") + "  ⇒ bin/vis-targets sẽ đỏ các ô này")
    Say ("setup: muốn đủ ma trận 18/18, chạy (cần Agent 0 cho phép): rustup target add " + ($missing -join " "))
    if ($Check) { exit 1 }
}

# Hash của cây nguồn nếu đang đứng trong repo (đủ mục "hash đang đo" của VD-136).
try {
    $top = (& git rev-parse --show-toplevel 2>$null)
    if ($LASTEXITCODE -eq 0 -and $top) {
        LogEnv ("repo=" + $top)
        $hashArr = @(& git -C $top rev-parse HEAD)
        LogEnv ("hash=" + ($hashArr -join ""))
        $acArr = @(& git -C $top config core.autocrlf)
        LogEnv ("autocrlf=" + ($acArr -join ""))
    }
} catch { }

Say "setup: OK (toolchain $Toolchain; target thiếu: $($missing.Count))"
exit 0
