# SPDX-License-Identifier: MIT OR Apache-2.0
# ST-136 (VD-136, VD-140) — tái lập **thành phần toolchain** cho máy đo Windows (`psvn`).
#
# Vì sao: số đo Windows trong VS2 chỉ so sánh được với `ps`/macOS khi **cùng một toolchain**.
# `ps` đo bằng rustc 1.99.1 (58cf8f9c3) và repo khai `rust-version = "1.99.1"`; trên `psvn`
# "stable" KHÔNG phải 1.99.1 (stable = 1.100.0 sau lần đo 2026-11-16) ⇒ mọi lệnh đo phải ghim
# `RUSTUP_TOOLCHAIN=1.99.1-x86_64-pc-windows-msvc`.
#
# Dùng:
#   powershell -NoProfile -File setup.ps1                   # kiểm + cài toolchain 1.99.1 + 2 target std (idempotent)
#   powershell -NoProfile -File setup.ps1 -Check            # CHỈ kiểm; thiếu ⇒ exit 1, không cài gì
#   powershell -NoProfile -File setup.ps1 -Log C:\env.log   # ghi thêm log môi trường (mặc định $HOME\vis-setup.log)
#
# Mã thoát: 0 xanh · 1 thiếu thành phần (chế độ -Check) · 3 BLOCKED-ENV (không có rustup / cài lỗi).
#
# ĐƯỢC PHÉP CÀI (nguồn: **VD-146(1)**, 2026-11-16; kèm VD-136, VD-140) — đúng hai target std
# `wasm32-unknown-unknown` + `x86_64-unknown-linux-gnu` của toolchain đang đo, chỉ để `bin/vis-targets`
# đo được đủ ma trận 18/18. Đây là **hạ tầng máy đo**, KHÔNG phải tuyên bố hỗ trợ Windows (VD-121).
# `-Check` thì chỉ kiểm, không cài; mọi lần cài đều ghi `rustup target list --installed` TRƯỚC và SAU.
#
# KHÔNG làm gì khác: không `rustup update`, không đổi toolchain mặc định, không cài target nào khác
# hai target trên, không cài Build Tools/gói OS, không reboot (VD-140: Build Tools chỉ cập nhật khi
# build lỗi do bản cũ VÀ có log lỗi trước; trình cài đòi khởi động lại ⇒ dừng, BLOCKED-ENV).
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

# VD-146(1): cài hai target std còn thiếu (chỉ khi KHÔNG có -Check), có log TRƯỚC/SAU (VD-140).
$installed = (@(rustup target list --installed) -join "`n")
$missing = @()
foreach ($t in $WantedTargets) {
    if ($installed -notmatch [regex]::Escape($t)) { $missing += $t }
}
if ($missing.Count -gt 0) {
    if ($Check) {
        Say ("setup: THIẾU target std: " + ($missing -join " ") + " (chế độ -Check, không cài)")
        exit 1
    }
    LogEnv ("target-truoc=" + ($installed -replace "`n", ","))
    Say ("setup: cài target std thiếu của toolchain " + $Toolchain + ": " + ($missing -join " ") + "  (VD-146(1))")
    rustup target add $missing
    if ($LASTEXITCODE -ne 0) { Say "setup: cài target lỗi ⇒ BLOCKED-ENV"; exit 3 }
    $installedSau = (@(rustup target list --installed) -join "`n")
    LogEnv ("target-sau=" + ($installedSau -replace "`n", ","))
    $still = @()
    foreach ($t in $missing) {
        if ($installedSau -notmatch [regex]::Escape($t)) { $still += $t }
    }
    if ($still.Count -gt 0) {
        Say ("setup: VẪN THIẾU target std sau khi cài: " + ($still -join " ") + " ⇒ BLOCKED-ENV")
        exit 3
    }
    Say ("setup: đã cài đủ target std: " + ($missing -join " "))
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

$installedCuoi = (@(rustup target list --installed) -join "`n")
$thieuCuoi = @()
foreach ($t in $WantedTargets) {
    if ($installedCuoi -notmatch [regex]::Escape($t)) { $thieuCuoi += $t }
}
Say "setup: OK (toolchain $Toolchain; target thiếu: $($thieuCuoi.Count))"
exit 0
