param(
  [string]$OutDir = (Join-Path $HOME ".meetin-signing-keys"),
  [string]$KeyId = "meetin-release-1",
  [switch]$Force
)

$ErrorActionPreference = "Stop"

# Guvenlik uyarisi: ozel anahtar asla git deposuna eklenmemelidir.
$resolvedOut = [System.IO.Path]::GetFullPath($OutDir)
$repoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot ".."))
if ($resolvedOut.StartsWith($repoRoot, [System.StringComparison]::OrdinalIgnoreCase)) {
  Write-Warning "UYARI: Cikti klasoru proje deposunun icinde gorunuyor ($resolvedOut). Ozel anahtari ASLA git deposuna kaydetmeyin!"
}

New-Item -ItemType Directory -Path $resolvedOut -Force | Out-Null

$privPemPath = Join-Path $resolvedOut "meetin-signing.private.pem"
$pubPemPath = Join-Path $resolvedOut "meetin-signing.public.pem"
$pubB64Path = Join-Path $resolvedOut "meetin-signing.public.b64"

if (-not $Force) {
  foreach ($p in @($privPemPath, $pubPemPath, $pubB64Path)) {
    if (Test-Path -LiteralPath $p) {
      throw "Hedef dosya zaten var: $p. Uzerine yazmak icin -Force kullanin."
    }
  }
}

function Pad-32Bytes([byte[]]$Bytes) {
  if ($Bytes.Length -eq 32) { return $Bytes }
  if ($Bytes.Length -gt 32) {
    $out = New-Object byte[] 32
    [Array]::Copy($Bytes, $Bytes.Length - 32, $out, 0, 32)
    return $out
  }
  $out = New-Object byte[] 32
  [Array]::Copy($Bytes, 0, $out, 32 - $Bytes.Length, $Bytes.Length)
  return $out
}

function Format-Pem([string]$Label, [byte[]]$DerBytes) {
  $b64 = [Convert]::ToBase64String($DerBytes)
  $lines = New-Object System.Collections.Generic.List[string]
  $lines.Add("-----BEGIN $Label-----")
  for ($i = 0; $i -lt $b64.Length; $i += 64) {
    $len = [Math]::Min(64, $b64.Length - $i)
    $lines.Add($b64.Substring($i, $len))
  }
  $lines.Add("-----END $Label-----")
  return ($lines -join "`n") + "`n"
}

$ec = [System.Security.Cryptography.ECDsa]::Create([System.Security.Cryptography.ECCurve+NamedCurves]::nistP256)
try {
  $params = $ec.ExportParameters($true)
  $d = Pad-32Bytes $params.D
  $x = Pad-32Bytes $params.Q.X
  $y = Pad-32Bytes $params.Q.Y

  # Standart X.509 SPKI DER (ECDSA prime256v1 / NIST P-256, 91 bayt)
  [byte[]]$spkiPrefix = @(
    0x30, 0x59,
    0x30, 0x13,
    0x06, 0x07, 0x2a, 0x86, 0x48, 0xce, 0x3d, 0x02, 0x01,
    0x06, 0x08, 0x2a, 0x86, 0x48, 0xce, 0x3d, 0x03, 0x01, 0x07,
    0x03, 0x42, 0x00, 0x04
  )
  $spkiDer = New-Object byte[] 91
  [Array]::Copy($spkiPrefix, 0, $spkiDer, 0, 27)
  [Array]::Copy($x, 0, $spkiDer, 27, 32)
  [Array]::Copy($y, 0, $spkiDer, 59, 32)

  # Standart PKCS#8 DER (ECDSA prime256v1 / NIST P-256, 138 bayt)
  [byte[]]$pkcs8Prefix = @(
    0x30, 0x81, 0x87,
    0x02, 0x01, 0x00,
    0x30, 0x13,
    0x06, 0x07, 0x2a, 0x86, 0x48, 0xce, 0x3d, 0x02, 0x01,
    0x06, 0x08, 0x2a, 0x86, 0x48, 0xce, 0x3d, 0x03, 0x01, 0x07,
    0x04, 0x6d,
    0x30, 0x6b,
    0x02, 0x01, 0x01,
    0x04, 0x20
  )
  [byte[]]$pkcs8Mid = @(0xa1, 0x44, 0x03, 0x42, 0x00, 0x04)
  $pkcs8Der = New-Object byte[] 138
  [Array]::Copy($pkcs8Prefix, 0, $pkcs8Der, 0, 36)
  [Array]::Copy($d, 0, $pkcs8Der, 36, 32)
  [Array]::Copy($pkcs8Mid, 0, $pkcs8Der, 68, 6)
  [Array]::Copy($x, 0, $pkcs8Der, 74, 32)
  [Array]::Copy($y, 0, $pkcs8Der, 106, 32)

  $utf8NoBom = New-Object System.Text.UTF8Encoding($false)
  [System.IO.File]::WriteAllText($privPemPath, (Format-Pem "PRIVATE KEY" $pkcs8Der), $utf8NoBom)
  [System.IO.File]::WriteAllText($pubPemPath, (Format-Pem "PUBLIC KEY" $spkiDer), $utf8NoBom)
  $pubB64 = [Convert]::ToBase64String($spkiDer)
  [System.IO.File]::WriteAllText($pubB64Path, $pubB64 + "`n", $utf8NoBom)

  Write-Host "ECDSA P-256 guncelleme imzalama anahtar cifti uretildi (key_id: $KeyId):"
  Write-Host "  Ozel anahtar (GIZLI TUTUN): $privPemPath"
  Write-Host "  Acik anahtar (PEM):         $pubPemPath"
  Write-Host "  Acik anahtar (Base64):      $pubB64Path"
  Write-Host ""
  Write-Host "Sonraki adimlar:"
  Write-Host "  1. Ozel anahtari ASLA git deposuna eklemeyin."
  Write-Host "  2. GitHub Actions -> Secrets bolumune MEETIN_SIGNING_PRIVATE_KEY olarak ozel PEM icerigini ekleyin."
  Write-Host "  3. GitHub Actions -> Variables/Secrets bolumune MEETIN_UPDATE_PUBLIC_KEY olarak tek satirlik acik anahtari ekleyin:"
  Write-Host "     $pubB64"
}
finally {
  $ec.Dispose()
}
