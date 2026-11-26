param(
  [Parameter(Mandatory = $true)]
  [string]$ManifestPath,
  [string]$PrivateKeyPath,
  [string]$PrivateKeyEnv = "MEETIN_SIGNING_PRIVATE_KEY",
  [string]$KeyId
)

$ErrorActionPreference = "Stop"

if (-not (Test-Path -LiteralPath $ManifestPath)) {
  throw "Manifest dosyasi bulunamadi: $ManifestPath"
}

$rawKey = ""
if (-not [string]::IsNullOrWhiteSpace($PrivateKeyPath)) {
  if (-not (Test-Path -LiteralPath $PrivateKeyPath)) {
    throw "Ozel anahtar dosyasi bulunamadi: $PrivateKeyPath"
  }
  $rawKey = [System.IO.File]::ReadAllText((Resolve-Path -LiteralPath $PrivateKeyPath).Path)
} else {
  $envVal = [Environment]::GetEnvironmentVariable($PrivateKeyEnv)
  if (-not [string]::IsNullOrWhiteSpace($envVal)) {
    $rawKey = $envVal
  }
}

if ([string]::IsNullOrWhiteSpace($rawKey)) {
  throw "Ozel anahtar saglanmadi (-PrivateKeyPath veya $PrivateKeyEnv ortam degiskeni gereklidir)."
}

function Import-P256PrivateKey([string]$KeyText) {
  $sb = New-Object System.Text.StringBuilder
  foreach ($line in ($KeyText -split "`r?`n")) {
    $t = $line.Trim()
    if ([string]::IsNullOrEmpty($t) -or $t.StartsWith("-----")) { continue }
    [void]$sb.Append($t)
  }
  $bytes = [Convert]::FromBase64String($sb.ToString())

  $ecParams = New-Object System.Security.Cryptography.ECParameters
  $ecParams.Curve = [System.Security.Cryptography.ECCurve+NamedCurves]::nistP256

  if ($bytes.Length -eq 138 -and $bytes[0] -eq 0x30 -and $bytes[34] -eq 0x04 -and $bytes[35] -eq 0x20 -and $bytes[73] -eq 0x04) {
    # Standart P-256 PKCS#8 DER (D + acik anahtar)
    $d = New-Object byte[] 32
    $x = New-Object byte[] 32
    $y = New-Object byte[] 32
    [Array]::Copy($bytes, 36, $d, 0, 32)
    [Array]::Copy($bytes, 74, $x, 0, 32)
    [Array]::Copy($bytes, 106, $y, 0, 32)
    $ecParams.D = $d
    $q = New-Object System.Security.Cryptography.ECPoint
    $q.X = $x
    $q.Y = $y
    $ecParams.Q = $q
    return [System.Security.Cryptography.ECDsa]::Create($ecParams)
  }

  if ($bytes.Length -eq 104 -and $bytes[0] -eq 0x45 -and $bytes[1] -eq 0x43 -and $bytes[2] -eq 0x53 -and $bytes[3] -eq 0x32) {
    # Windows CNG BCRYPT_ECCPRIVATE_BLOB (ECS2 + 32 + X + Y + D)
    $x = New-Object byte[] 32
    $y = New-Object byte[] 32
    $d = New-Object byte[] 32
    [Array]::Copy($bytes, 8, $x, 0, 32)
    [Array]::Copy($bytes, 40, $y, 0, 32)
    [Array]::Copy($bytes, 72, $d, 0, 32)
    $ecParams.D = $d
    $q = New-Object System.Security.Cryptography.ECPoint
    $q.X = $x
    $q.Y = $y
    $ecParams.Q = $q
    return [System.Security.Cryptography.ECDsa]::Create($ecParams)
  }

  if ($bytes.Length -eq 96) {
    # Ham X || Y || D (96 bayt)
    $x = New-Object byte[] 32
    $y = New-Object byte[] 32
    $d = New-Object byte[] 32
    [Array]::Copy($bytes, 0, $x, 0, 32)
    [Array]::Copy($bytes, 32, $y, 0, 32)
    [Array]::Copy($bytes, 64, $d, 0, 32)
    $ecParams.D = $d
    $q = New-Object System.Security.Cryptography.ECPoint
    $q.X = $x
    $q.Y = $y
    $ecParams.Q = $q
    return [System.Security.Cryptography.ECDsa]::Create($ecParams)
  }

  # .NET Core / PowerShell 7 genel PKCS#8 yedek yolu
  $ecFallback = [System.Security.Cryptography.ECDsa]::Create()
  $importMethod = $ecFallback.GetType().GetMethod("ImportPkcs8PrivateKey")
  if ($null -ne $importMethod) {
    $readBytes = 0
    $ecFallback.ImportPkcs8PrivateKey($bytes, [ref]$readBytes)
    return $ecFallback
  }
  $ecFallback.Dispose()
  throw "Desteklenmeyen ECDSA P-256 ozel anahtar bicimi ($($bytes.Length) bayt)."
}

$manifestFull = (Resolve-Path -LiteralPath $ManifestPath).Path
$rawJson = [System.IO.File]::ReadAllText($manifestFull)
$manifest = $rawJson | ConvertFrom-Json

$version = [string]$manifest.version
$sha256 = ([string]$manifest.sha256).Trim().ToLowerInvariant()
$downloadUrl = ([string]$manifest.download_url).Trim()

if ([string]::IsNullOrWhiteSpace($version)) { throw "Manifest icinde 'version' eksik." }
if ($sha256.Length -ne 64 -or $sha256 -notmatch '^[0-9a-f]{64}$') {
  throw "Manifest icinde gecerli 64 karakterlik 'sha256' ozeti eksik."
}
if ([string]::IsNullOrWhiteSpace($downloadUrl)) { throw "Manifest icinde 'download_url' eksik." }

# Kanonik imza yuku: version\nsha256\ndownload_url (UTF-8, LF)
$canonical = "$($version.Trim())`n$sha256`n$downloadUrl"
$payloadBytes = [System.Text.Encoding]::UTF8.GetBytes($canonical)

$ec = Import-P256PrivateKey $rawKey
try {
  $sigBytes = $ec.SignData($payloadBytes, [System.Security.Cryptography.HashAlgorithmName]::SHA256)
  if (-not $ec.VerifyData($payloadBytes, $sigBytes, [System.Security.Cryptography.HashAlgorithmName]::SHA256)) {
    throw "Uretilen imza oz-dogrulamadan gecemedi."
  }
  $sigB64 = [Convert]::ToBase64String($sigBytes)
}
finally {
  $ec.Dispose()
}

$effectiveKeyId = $KeyId
if ([string]::IsNullOrWhiteSpace($effectiveKeyId)) {
  if ($manifest.PSObject.Properties.Name -contains "key_id" -and -not [string]::IsNullOrWhiteSpace([string]$manifest.key_id)) {
    $effectiveKeyId = [string]$manifest.key_id
  } else {
    $effectiveKeyId = "meetin-release-1"
  }
}

$outObj = [ordered]@{
  version      = $version.Trim()
  pub_date     = if ($manifest.PSObject.Properties.Name -contains "pub_date") { $manifest.pub_date } else { (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ") }
  notes        = if ($manifest.PSObject.Properties.Name -contains "notes") { $manifest.notes } else { "" }
  download_url = $downloadUrl
  sha256       = $sha256
}

if ($manifest.PSObject.Properties.Name -contains "portable_url" -and -not [string]::IsNullOrWhiteSpace([string]$manifest.portable_url)) {
  $outObj["portable_url"] = ([string]$manifest.portable_url).Trim()
}
if ($manifest.PSObject.Properties.Name -contains "portable_sha256" -and -not [string]::IsNullOrWhiteSpace([string]$manifest.portable_sha256)) {
  $outObj["portable_sha256"] = ([string]$manifest.portable_sha256).Trim().ToLowerInvariant()
}

$outObj["signature"] = $sigB64
$outObj["key_id"] = $effectiveKeyId

$utf8NoBom = New-Object System.Text.UTF8Encoding($false)
$jsonOut = ($outObj | ConvertTo-Json -Depth 4) + "`n"
[System.IO.File]::WriteAllText($manifestFull, $jsonOut, $utf8NoBom)

Write-Host "Manifest basariyla imzalandi: $manifestFull (key_id: $effectiveKeyId)"
