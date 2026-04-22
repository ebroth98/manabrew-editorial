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

#Requires -RunAsAdministrator

$ErrorActionPreference = "Stop"
$ProgressPreference    = "SilentlyContinue"   # faster Invoke-WebRequest

function Section($msg) {
    Write-Host "`n── $msg " -ForegroundColor Cyan -NoNewline
    Write-Host ("─" * [Math]::Max(0, 70 - $msg.Length)) -ForegroundColor Cyan
}

function Refresh-Path {
    $env:Path = [System.Environment]::GetEnvironmentVariable("Path","Machine") + ";" +
                [System.Environment]::GetEnvironmentVariable("Path","User")  + ";" +
                "$env:USERPROFILE\.cargo\bin"
}

function Has-Command($name) {
    $null -ne (Get-Command $name -ErrorAction SilentlyContinue)
}

# ─── 1. TLS 1.2 ────────────────────────────────────────────────────────────
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12

# ─── 2. Chocolatey ─────────────────────────────────────────────────────────
Section "Chocolatey"
if (Has-Command choco) {
    Write-Host "choco already installed: $(choco --version)"
} else {
    iex ((New-Object System.Net.WebClient).DownloadString('https://community.chocolatey.org/install.ps1'))
    Refresh-Path
}

# ─── 3. Core tools via Chocolatey ──────────────────────────────────────────
Section "Core tools (git, node, yarn, nsis, jq, webview2)"
$packages = @(
    "git",
    "nodejs-lts",
    "yarn",
    "nsis",
    "jq",
    "microsoft-edge-webview2-runtime"
)
foreach ($pkg in $packages) {
    Write-Host "→ $pkg"
    choco install -y --no-progress $pkg
}
Refresh-Path

# ─── 4. Visual Studio 2022 Build Tools (C++ workload) ──────────────────────
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
    Write-Host "Downloading vs_buildtools.exe (direct bootstrapper — chocolatey's param-passing is flaky for this)..."
    $installer = "$env:TEMP\vs_buildtools.exe"
    Invoke-WebRequest -Uri "https://aka.ms/vs/17/release/vs_buildtools.exe" -OutFile $installer
    Write-Host "Running installer (10-30 min, no progress output; wait for prompt)..."
    $args = @(
        "--quiet", "--wait", "--norestart", "--nocache",
        "--add", "Microsoft.VisualStudio.Workload.VCTools",
        "--add", "Microsoft.VisualStudio.Component.Windows11SDK.22621",
        "--includeRecommended"
    )
    $proc = Start-Process -Wait -PassThru -FilePath $installer -ArgumentList $args
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

# ─── 5. Rust (MSVC toolchain) ──────────────────────────────────────────────
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

# ─── 6. Enter VS Dev Shell + install Tauri CLI ─────────────────────────────
Section "Tauri CLI (requires MSVC linker on PATH)"
if (Has-Command cargo-tauri) {
    Write-Host "tauri CLI already installed."
} else {
    # link.exe is never on system PATH — load the dev env for this session.
    Import-Module "$vsPath\Common7\Tools\Microsoft.VisualStudio.DevShell.dll"
    Enter-VsDevShell -VsInstallPath $vsPath -SkipAutomaticLocation `
        -DevCmdArguments "-arch=x64 -host_arch=x64"
    if (-not (Has-Command link)) { throw "link.exe still not on PATH after Enter-VsDevShell" }
    cargo install tauri-cli --version "^2"
}

# ─── 7. Sanity check ───────────────────────────────────────────────────────
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
Try-Version jq
if (Has-Command link)     { "link.exe   found at $((Get-Command link).Source)" }      else { "link.exe   NOT FOUND (ensure dev shell is active OR use ilammy/msvc-dev-cmd in CI)" }
if (Has-Command makensis) { "makensis   found at $((Get-Command makensis).Source)" }  else { "makensis   NOT FOUND" }

Section "Next steps"
Write-Host @"
1. Reboot the machine so PATH and VS Build Tools are fully registered.
2. Re-register / restart the GitHub runner service so it inherits the new PATH:
     cd C:\Users\Administrator\actions-runner
     .\svc.cmd stop   # if installed as service
     .\svc.cmd start
     # OR, for interactive mode: Ctrl+C and re-run .\run.cmd
3. The release-artifacts workflow already includes ilammy/msvc-dev-cmd@v1
   to load MSVC env for every CI run — you do NOT need link.exe on the
   system PATH.
4. Trigger a test build via workflow_dispatch on the Release artifacts
   workflow, or push a tag like `v0.0.1-test`.
"@
