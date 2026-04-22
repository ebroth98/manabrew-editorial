# Setup script for a Windows self-hosted GitHub Actions runner that builds
# the Tauri .exe bundle for this repo.
#
# Run as Administrator:
#   PS> Set-ExecutionPolicy -Scope Process -ExecutionPolicy Bypass -Force
#   PS> .\scripts\setup-windows-runner.ps1
#
# Installs: Chocolatey, Git, Node LTS, Yarn, NSIS, jq, WebView2 runtime,
# Visual Studio 2022 Build Tools (C++ workload + Win11 SDK), Rust (MSVC),
# and the Tauri CLI.
#
# Idempotent: each step skips work already done. Safe to re-run.
#
# ASCII-only on purpose so PowerShell 5.1 (Windows default codepage) parses
# the file correctly without a UTF-8 BOM.

#Requires -RunAsAdministrator

$ErrorActionPreference = "Stop"
$ProgressPreference    = "SilentlyContinue"   # faster Invoke-WebRequest

function Section($msg) {
    $bar = "-" * [Math]::Max(0, 70 - $msg.Length)
    Write-Host "`n== $msg $bar" -ForegroundColor Cyan
}

function Refresh-Path {
    $env:Path = [System.Environment]::GetEnvironmentVariable("Path","Machine") + ";" +
                [System.Environment]::GetEnvironmentVariable("Path","User")  + ";" +
                "$env:USERPROFILE\.cargo\bin"
}

function Has-Command($name) {
    $null -ne (Get-Command $name -ErrorAction SilentlyContinue)
}

function Add-ToSystemPath($dir) {
    # Persists to HKLM so every account - including the GitHub Actions runner
    # service - sees the entry. Running processes keep their old PATH until
    # restarted; the runner service restart is handled later in this script.
    $current = [Environment]::GetEnvironmentVariable("Path", "Machine")
    $entries = $current -split ';' | Where-Object { $_ -ne '' }
    if ($entries -notcontains $dir) {
        [Environment]::SetEnvironmentVariable("Path", "$current;$dir", "Machine")
        Write-Host "Added to system PATH: $dir"
    } else {
        Write-Host "Already on system PATH: $dir"
    }
}

# --- 1. TLS 1.2 -----------------------------------------------------------
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12

# --- 2. Chocolatey --------------------------------------------------------
Section "Chocolatey"
if (Has-Command choco) {
    Write-Host "choco already installed: $(choco --version)"
} else {
    iex ((New-Object System.Net.WebClient).DownloadString('https://community.chocolatey.org/install.ps1'))
    Refresh-Path
}

# --- 3. Core tools via Chocolatey ----------------------------------------
Section "Core tools (git, node, yarn, nsis, jq)"
$packages = @(
    "git",
    "nodejs-lts",
    "yarn",
    "nsis",
    "jq"
)
foreach ($pkg in $packages) {
    Write-Host "-> $pkg"
    choco install -y --no-progress $pkg
}
Refresh-Path

# WebView2 Runtime ships preinstalled on Windows 10 (20H1+) and Windows 11,
# so no choco package is needed. Verify the install key is present; if
# missing, point the user at the Evergreen Bootstrapper instead of failing
# the build later inside Tauri.
Section "WebView2 Runtime (preinstalled check)"
$wv2Keys = @(
    "HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}",
    "HKLM:\SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}"
)
$wv2Installed = $false
foreach ($k in $wv2Keys) {
    if (Test-Path $k) {
        $ver = (Get-ItemProperty $k -ErrorAction SilentlyContinue).pv
        if ($ver) {
            Write-Host "WebView2 Runtime present: $ver"
            $wv2Installed = $true
            break
        }
    }
}
if (-not $wv2Installed) {
    Write-Warning "WebView2 Runtime not detected. Install the Evergreen Bootstrapper manually from https://developer.microsoft.com/microsoft-edge/webview2/ before running a Tauri build."
}

# --- 4. Visual Studio 2022 Build Tools (C++ workload) --------------------
Section "Visual Studio 2022 Build Tools + C++ workload"
$vsWhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
$vsPath  = ""
if (Test-Path $vsWhere) {
    $vsPath = & $vsWhere -latest -products * `
        -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 `
        -property installationPath
}

if ($vsPath) {
    Write-Host "VS Build Tools with C++ already installed at: $vsPath"
} else {
    Write-Host "Downloading vs_buildtools.exe (direct bootstrapper - chocolatey's param-passing is flaky for this)..."
    $installer = "$env:TEMP\vs_buildtools.exe"
    Invoke-WebRequest -Uri "https://aka.ms/vs/17/release/vs_buildtools.exe" -OutFile $installer
    Write-Host "Running installer (10-30 min, no progress output; wait for prompt)..."
    $vsArgs = @(
        "--quiet", "--wait", "--norestart", "--nocache",
        "--add", "Microsoft.VisualStudio.Workload.VCTools",
        "--add", "Microsoft.VisualStudio.Component.Windows11SDK.22621",
        "--includeRecommended"
    )
    $proc = Start-Process -Wait -PassThru -FilePath $installer -ArgumentList $vsArgs
    # Exit codes 0 = success, 3010 = success, reboot required
    if ($proc.ExitCode -ne 0 -and $proc.ExitCode -ne 3010) {
        throw "vs_buildtools exited with code $($proc.ExitCode)"
    }
    Remove-Item $installer -ErrorAction SilentlyContinue
    $vsPath = & $vsWhere -latest -products * `
        -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 `
        -property installationPath
    if (-not $vsPath) { throw "VS Build Tools install finished but vswhere still reports nothing." }
    Write-Host "Installed at: $vsPath"
}

# --- 5. Rust (MSVC toolchain) --------------------------------------------
Section "Rust toolchain (MSVC)"
if (Has-Command rustc) {
    Write-Host "rustc already installed: $(rustc --version)"
    rustup update stable
} else {
    $rustup = "$env:TEMP\rustup-init.exe"
    Invoke-WebRequest `
        -Uri "https://static.rust-lang.org/rustup/dist/x86_64-pc-windows-msvc/rustup-init.exe" `
        -OutFile $rustup
    & $rustup -y --default-toolchain stable --default-host x86_64-pc-windows-msvc
    Remove-Item $rustup -ErrorAction SilentlyContinue
    Refresh-Path
}

# rustup-init only touches the installing user's PATH. The GitHub Actions
# runner service runs under a different account (NetworkService or a named
# service user), so it won't see cargo unless .cargo\bin is on SYSTEM PATH.
Add-ToSystemPath "$env:USERPROFILE\.cargo\bin"
Refresh-Path

# --- 5b. wasm32 target (no linker needed) --------------------------------
Section "wasm32-unknown-unknown target"
$installedTargets = (& rustup target list --installed) -split "`n"
if ($installedTargets -contains "wasm32-unknown-unknown") {
    Write-Host "wasm32-unknown-unknown already installed."
} else {
    rustup target add wasm32-unknown-unknown
}

# --- 6. Enter VS Dev Shell + install Tauri CLI + wasm-pack ---------------
# Both cargo installs compile from source and need link.exe, so they share
# the same dev shell session. Skipping whichever is already present.
Section "Tauri CLI + wasm-pack (require MSVC linker on PATH)"
$needTauri    = -not (Has-Command cargo-tauri)
$needWasmPack = -not (Has-Command wasm-pack)
if ($needTauri -or $needWasmPack) {
    # link.exe is never on system PATH - load the dev env for this session.
    Import-Module "$vsPath\Common7\Tools\Microsoft.VisualStudio.DevShell.dll"
    Enter-VsDevShell -VsInstallPath $vsPath -SkipAutomaticLocation `
        -DevCmdArguments "-arch=x64 -host_arch=x64"
    if (-not (Has-Command link)) { throw "link.exe still not on PATH after Enter-VsDevShell" }
    if ($needTauri)    { cargo install tauri-cli --version "^2" }
    if ($needWasmPack) { cargo install wasm-pack --locked }
} else {
    Write-Host "tauri CLI and wasm-pack already installed."
}

# --- 7. Restart GitHub Actions runner service ----------------------------
# The runner captures PATH at service-start time. Any cargo/wasm-pack PATH
# changes made above won't be visible to it until the service restarts.
Section "GitHub Actions runner service"
$runners = Get-Service "actions.runner.*" -ErrorAction SilentlyContinue
if ($runners) {
    foreach ($svc in $runners) {
        Write-Host "Restarting $($svc.Name) (was: $($svc.Status))..."
        Restart-Service $svc.Name -Force
    }
    Start-Sleep -Seconds 2
    Get-Service "actions.runner.*" | Select-Object Name, Status | Format-Table | Out-String | Write-Host
} else {
    Write-Warning "No 'actions.runner.*' Windows service found."
    Write-Warning "If the runner is configured as a service, verify with: Get-Service 'actions.runner.*'"
    Write-Warning "If running interactively (run.cmd), stop it (Ctrl+C) and restart so it picks up PATH changes."
}

# --- 8. Sanity check -----------------------------------------------------
Section "Versions"
Refresh-Path
function Try-Version($cmd, $arg = "--version") {
    if (Has-Command $cmd) {
        try { "{0,-10} {1}" -f $cmd, ((& $cmd $arg) 2>&1 | Select-Object -First 1) }
        catch { "{0,-10} (error: $_)" -f $cmd }
    } else {
        "{0,-10} NOT FOUND" -f $cmd
    }
}
Try-Version git
Try-Version node
Try-Version npm
Try-Version yarn
Try-Version rustc
Try-Version cargo
Try-Version wasm-pack
Try-Version jq
if (Has-Command link)     { "link.exe   found at $((Get-Command link).Source)" }      else { "link.exe   NOT FOUND (ensure dev shell is active OR use ilammy/msvc-dev-cmd in CI)" }
if (Has-Command makensis) { "makensis   found at $((Get-Command makensis).Source)" }  else { "makensis   NOT FOUND" }
$wasm32Installed = ((& rustup target list --installed) -split "`n") -contains 'wasm32-unknown-unknown'
if ($wasm32Installed) { "wasm32     installed" } else { "wasm32     NOT INSTALLED" }

Section "Next steps"
Write-Host @"
1. Verify the runner picked up the new PATH from a FRESH shell:
     cargo --version
     wasm-pack --version
   (If this shell is the one you ran the script in, open a new PowerShell.)
2. Trigger a test build via workflow_dispatch on the Release artifacts
   workflow, or push a tag like v0.0.1-test.
3. The release-artifacts workflow uses ilammy/msvc-dev-cmd@v1 to load MSVC
   env per run, so link.exe does NOT need to be on system PATH.
4. If the workflow still can't find cargo, confirm the runner service was
   restarted (step 7 above) - PATH changes are only picked up on restart.
"@
