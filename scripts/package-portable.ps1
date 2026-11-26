param(
  [string]$Version = "1.2.0",
  [string]$ExePath = "src-tauri/target/release/meetin.exe",
  [string]$OutDir = "dist-packages",
  [switch]$SkipBuild
)

$ErrorActionPreference = "Stop"

if (-not $SkipBuild) {
  npm run build
  if ($LASTEXITCODE -ne 0) { throw "Ön yüz derlemesi (npm run build) başarısız." }
  cargo build --release --offline --manifest-path src-tauri/Cargo.toml --features custom-protocol
  if ($LASTEXITCODE -ne 0) { throw "Rust release derlemesi başarısız." }
}

if (-not (Test-Path $ExePath)) {
  throw "Release EXE bulunamadı: $ExePath."
}

$stage = Join-Path $OutDir "Meetin_${Version}_portable"
$zipPath = Join-Path $OutDir "Meetin_${Version}_portable.zip"

if (Test-Path $stage) { Remove-Item $stage -Recurse -Force }
if (Test-Path $zipPath) { Remove-Item $zipPath -Force }

New-Item -ItemType Directory -Path $stage -Force | Out-Null
Copy-Item $ExePath (Join-Path $stage "Meetin.exe")
if (Test-Path "CHANGELOG.md") {
  Copy-Item "CHANGELOG.md" (Join-Path $stage "CHANGELOG.md")
}

$ProgressPreference = "SilentlyContinue"
Add-Type -AssemblyName System.IO.Compression.FileSystem
[System.IO.Compression.ZipFile]::CreateFromDirectory((Resolve-Path $stage).Path, (Join-Path (Resolve-Path $OutDir).Path "Meetin_${Version}_portable.zip"))
Write-Host "Portable paket hazır: $zipPath"
