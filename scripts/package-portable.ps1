param(
  [string]$Version = ((Get-Content package.json -Raw | ConvertFrom-Json).version),
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

# ZIP açılınca tek bir "Meetin" klasörü çıkar (onedir): Meetin.exe + portable.flag + data\
$stageRoot = Join-Path $OutDir "Meetin_${Version}_portable"
$stage = Join-Path $stageRoot "Meetin"
$zipPath = Join-Path $OutDir "Meetin_${Version}_portable.zip"

if (Test-Path $stageRoot) { Remove-Item $stageRoot -Recurse -Force }
if (Test-Path $zipPath) { Remove-Item $zipPath -Force }

New-Item -ItemType Directory -Path $stage -Force | Out-Null
Copy-Item $ExePath (Join-Path $stage "Meetin.exe")
# Bayrak: portable mod. Veriler kullanıcı klasöründe (%APPDATA%\Meetin), yedekler data\backups'ta.
$utf8 = New-Object System.Text.UTF8Encoding($false)
[System.IO.File]::WriteAllText((Join-Path (Resolve-Path $stage).Path "portable.flag"), "Meetin taşınabilir mod: yedekler bu klasördeki data\backups altına yazılır. Bu dosyayı silmeyin.`r`n", $utf8)
New-Item -ItemType Directory -Path (Join-Path $stage "data") -Force | Out-Null
if (Test-Path "CHANGELOG.md") {
  Copy-Item "CHANGELOG.md" (Join-Path $stage "CHANGELOG.md")
}

$ProgressPreference = "SilentlyContinue"
Add-Type -AssemblyName System.IO.Compression.FileSystem
[System.IO.Compression.ZipFile]::CreateFromDirectory((Resolve-Path $stageRoot).Path, (Join-Path (Resolve-Path $OutDir).Path "Meetin_${Version}_portable.zip"))
Write-Host "Portable paket hazır: $zipPath"
