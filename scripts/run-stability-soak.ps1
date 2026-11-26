<#
.SYNOPSIS
  Meetin 24 saatlik VM / CI canlı kararlılık ve kaynak izleme (soak) betiği (Faz E).
.DESCRIPTION
  Derlenmiş Meetin çalıştırılabilirini (veya -ProcessId ile çalışan bir süreci ya da
  -DryRun kendi kendini test sürecini) belirli aralıklarla örnekleyerek Working Set
  bellek artışını (MB), ortalama boşta CPU yüzdesini ve beklenmeyen çıkış sayısını
  ölçer; eşikleri denetleyip hem JSON hem Markdown rapor üretir.
  PowerShell 5.1 ve PowerShell 7 (pwsh) ile tam uyumludur.
#>
param(
  [int]$DurationSeconds = 0,
  [double]$DurationHours = 0,
  [int]$SampleIntervalSeconds = 5,
  [int]$WarmupSeconds = 2,
  [double]$MaxMemoryGrowthMb = 15.0,
  [double]$MaxIdleCpuPercent = 1.0,
  [string]$ExePath = "",
  [int]$ProcessId = 0,
  [string]$OutputReportPath = "soak-report.md",
  [switch]$DryRun
)

$ErrorActionPreference = "Stop"

if ($DurationSeconds -le 0 -and $DurationHours -le 0) {
  $DurationSeconds = 30
} elseif ($DurationHours -gt 0) {
  $DurationSeconds = [int][Math]::Ceiling($DurationHours * 3600.0)
}

if ($SampleIntervalSeconds -lt 1) {
  $SampleIntervalSeconds = 1
}
if ($WarmupSeconds -lt 0) {
  $WarmupSeconds = 0
}
if ($WarmupSeconds -ge $DurationSeconds) {
  $WarmupSeconds = [Math]::Max(0, [int][Math]::Floor($DurationSeconds / 4.0))
}

function Resolve-ReportPaths([string]$BasePath) {
  $fullPath = $BasePath
  if (-not [System.IO.Path]::IsPathRooted($fullPath)) {
    $fullPath = Join-Path (Get-Location).Path $BasePath
  }
  $dir = Split-Path -Path $fullPath -Parent
  if (-not [string]::IsNullOrWhiteSpace($dir) -and -not (Test-Path -LiteralPath $dir)) {
    New-Item -ItemType Directory -Path $dir -Force | Out-Null
  }
  $ext = [System.IO.Path]::GetExtension($fullPath)
  if ([string]::IsNullOrWhiteSpace($ext)) {
    $mdPath = "$fullPath.md"
    $jsonPath = "$fullPath.json"
  } elseif ($ext.ToLowerInvariant() -eq ".json") {
    $jsonPath = $fullPath
    $mdPath = [System.IO.Path]::ChangeExtension($fullPath, ".md")
  } else {
    $mdPath = $fullPath
    $jsonPath = [System.IO.Path]::ChangeExtension($fullPath, ".json")
  }
  return @{
    Markdown = $mdPath
    Json     = $jsonPath
  }
}

function Write-Utf8NoBom([string]$Path, [string]$Text) {
  $utf8NoBom = New-Object System.Text.UTF8Encoding($false)
  [System.IO.File]::WriteAllText($Path, $Text, $utf8NoBom)
}

function Resolve-TargetExecutable([string]$ProvidedPath) {
  if (-not [string]::IsNullOrWhiteSpace($ProvidedPath)) {
    if (-not (Test-Path -LiteralPath $ProvidedPath)) {
      throw "Hedef calistirilabilir dosya bulunamadi: $ProvidedPath"
    }
    return (Resolve-Path -LiteralPath $ProvidedPath).Path
  }

  $candidates = @(
    "src-tauri\target\release\meetin.exe",
    "src-tauri\target\debug\meetin.exe"
  )
  foreach ($cand in $candidates) {
    if (Test-Path -LiteralPath $cand) {
      return (Resolve-Path -LiteralPath $cand).Path
    }
  }
  throw "meetin.exe bulunamadi. -ExePath ile yol verin, -ProcessId kullanin veya kendi kendini test icin -DryRun gecin."
}

$reportPaths = Resolve-ReportPaths -BasePath $OutputReportPath
$logicalCores = [Math]::Max(1, [Environment]::ProcessorCount)
$startedUtc = (Get-Date).ToUniversalTime()
$launchedByScript = $false
$targetProc = $null
$targetLabel = ""
$unexpectedExits = 0
$exitCodeObserved = $null
$samples = New-Object System.Collections.Generic.List[object]

try {
  if ($DryRun) {
    $psExe = (Get-Process -Id $PID).Path
    if ([string]::IsNullOrWhiteSpace($psExe)) {
      $psExe = "powershell.exe"
    }
    $sleepLimit = [Math]::Max($DurationSeconds + 30, 60)
    $idleScript = "Start-Sleep -Seconds $sleepLimit"
    $targetProc = Start-Process -FilePath $psExe -ArgumentList @("-NoProfile", "-NonInteractive", "-WindowStyle", "Hidden", "-Command", $idleScript) -PassThru -WindowStyle Hidden
    $launchedByScript = $true
    $targetLabel = "dry-run-idle-probe (PID $($targetProc.Id))"
  } elseif ($ProcessId -gt 0) {
    $targetProc = [System.Diagnostics.Process]::GetProcessById($ProcessId)
    $targetLabel = "$($targetProc.ProcessName) (PID $($targetProc.Id))"
  } else {
    $exeFull = Resolve-TargetExecutable -ProvidedPath $ExePath
    $targetProc = Start-Process -FilePath $exeFull -PassThru
    $launchedByScript = $true
    $targetLabel = "$(Split-Path -Leaf $exeFull) (PID $($targetProc.Id))"
  }

  Write-Host "[Meetin Soak] Hedef: $targetLabel | Sure: ${DurationSeconds}s | Aralik: ${SampleIntervalSeconds}s | Isinma: ${WarmupSeconds}s"

  if ($WarmupSeconds -gt 0) {
    $warmupEnd = (Get-Date).AddSeconds($WarmupSeconds)
    while ((Get-Date) -lt $warmupEnd) {
      $targetProc.Refresh()
      if ($targetProc.HasExited) {
        $unexpectedExits = 1
        $exitCodeObserved = $targetProc.ExitCode
        break
      }
      Start-Sleep -Milliseconds 250
    }
  }

  $measureStopwatch = [System.Diagnostics.Stopwatch]::StartNew()
  $baselineWsMb = 0.0
  $baselineCpuSec = 0.0
  $prevCpuSec = 0.0
  $prevElapsedSec = 0.0
  $peakWsMb = 0.0
  $minWsMb = [double]::MaxValue
  $finalWsMb = 0.0
  $peakCpuPct = 0.0
  $lastCpuSec = 0.0
  $lastElapsedSec = 0.0

  if ($unexpectedExits -eq 0) {
    $targetProc.Refresh()
    if ($targetProc.HasExited) {
      $unexpectedExits = 1
      $exitCodeObserved = $targetProc.ExitCode
    } else {
      $baselineWsMb = [Math]::Round($targetProc.WorkingSet64 / 1MB, 3)
      $baselineCpuSec = $targetProc.TotalProcessorTime.TotalSeconds
      $prevCpuSec = $baselineCpuSec
      $prevElapsedSec = 0.0
      $peakWsMb = $baselineWsMb
      $minWsMb = $baselineWsMb
      $finalWsMb = $baselineWsMb
      $lastCpuSec = $baselineCpuSec

      $samples.Add([ordered]@{
        elapsed_seconds    = 0.0
        timestamp_utc      = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")
        working_set_mb     = $baselineWsMb
        private_memory_mb  = [Math]::Round($targetProc.PrivateMemorySize64 / 1MB, 3)
        cpu_percent        = 0.0
        handle_count       = $targetProc.HandleCount
        thread_count       = $targetProc.Threads.Count
      })

      $measureTargetSec = [Math]::Max(1, $DurationSeconds - $WarmupSeconds)
      while ($measureStopwatch.Elapsed.TotalSeconds -lt $measureTargetSec) {
        $remaining = $measureTargetSec - $measureStopwatch.Elapsed.TotalSeconds
        $sleepSec = [Math]::Min([double]$SampleIntervalSeconds, [Math]::Max(0.25, $remaining))
        $sleepMs = [int][Math]::Ceiling($sleepSec * 1000.0)
        Start-Sleep -Milliseconds $sleepMs

        $targetProc.Refresh()
        if ($targetProc.HasExited) {
          $unexpectedExits = 1
          $exitCodeObserved = $targetProc.ExitCode
          break
        }

        $nowElapsed = $measureStopwatch.Elapsed.TotalSeconds
        $dt = [Math]::Max(0.001, $nowElapsed - $prevElapsedSec)
        $nowCpu = $targetProc.TotalProcessorTime.TotalSeconds
        $dCpu = [Math]::Max(0.0, $nowCpu - $prevCpuSec)
        $intervalCpuPct = [Math]::Round(($dCpu / ($dt * $logicalCores)) * 100.0, 3)
        $wsMb = [Math]::Round($targetProc.WorkingSet64 / 1MB, 3)
        $privMb = [Math]::Round($targetProc.PrivateMemorySize64 / 1MB, 3)

        if ($wsMb -gt $peakWsMb) { $peakWsMb = $wsMb }
        if ($wsMb -lt $minWsMb) { $minWsMb = $wsMb }
        if ($intervalCpuPct -gt $peakCpuPct) { $peakCpuPct = $intervalCpuPct }

        $finalWsMb = $wsMb
        $lastCpuSec = $nowCpu
        $lastElapsedSec = $nowElapsed
        $prevCpuSec = $nowCpu
        $prevElapsedSec = $nowElapsed

        $samples.Add([ordered]@{
          elapsed_seconds    = [Math]::Round($nowElapsed, 2)
          timestamp_utc      = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")
          working_set_mb     = $wsMb
          private_memory_mb  = $privMb
          cpu_percent        = $intervalCpuPct
          handle_count       = $targetProc.HandleCount
          thread_count       = $targetProc.Threads.Count
        })
      }
    }
  }

  $measureStopwatch.Stop()
  $finishedUtc = (Get-Date).ToUniversalTime()

  if ($minWsMb -eq [double]::MaxValue) {
    $minWsMb = 0.0
  }

  $netMemoryGrowthMb = [Math]::Round([Math]::Max(0.0, $finalWsMb - $baselineWsMb), 3)
  $peakMemoryGrowthMb = [Math]::Round([Math]::Max(0.0, $peakWsMb - $baselineWsMb), 3)
  $avgIdleCpuPct = 0.0
  if ($lastElapsedSec -gt 0.0) {
    $totalCpuDelta = [Math]::Max(0.0, $lastCpuSec - $baselineCpuSec)
    $avgIdleCpuPct = [Math]::Round(($totalCpuDelta / ($lastElapsedSec * $logicalCores)) * 100.0, 3)
  }

  $memoryPass = ($netMemoryGrowthMb -lt $MaxMemoryGrowthMb)
  $cpuPass = ($avgIdleCpuPct -lt $MaxIdleCpuPercent)
  $stabilityPass = ($unexpectedExits -eq 0)
  $overallPass = ($memoryPass -and $cpuPass -and $stabilityPass)
  $statusText = if ($overallPass) { "PASS" } else { "FAIL" }

  $reportObject = [ordered]@{
    status                     = $statusText
    mode                       = if ($DryRun) { "dry-run" } elseif ($ProcessId -gt 0) { "attached-pid" } else { "launched-exe" }
    target                     = $targetLabel
    started_utc                = $startedUtc.ToString("yyyy-MM-ddTHH:mm:ssZ")
    finished_utc               = $finishedUtc.ToString("yyyy-MM-ddTHH:mm:ssZ")
    configured_duration_sec    = $DurationSeconds
    measured_duration_sec      = [Math]::Round($lastElapsedSec, 2)
    sample_interval_sec        = $SampleIntervalSeconds
    warmup_sec                 = $WarmupSeconds
    logical_cpu_cores          = $logicalCores
    sample_count               = $samples.Count
    thresholds                 = [ordered]@{
      max_memory_growth_mb = $MaxMemoryGrowthMb
      max_idle_cpu_percent = $MaxIdleCpuPercent
      max_unexpected_exits = 0
    }
    metrics                    = [ordered]@{
      baseline_working_set_mb = $baselineWsMb
      final_working_set_mb    = $finalWsMb
      min_working_set_mb      = $minWsMb
      peak_working_set_mb     = $peakWsMb
      net_memory_growth_mb    = $netMemoryGrowthMb
      peak_memory_growth_mb   = $peakMemoryGrowthMb
      avg_idle_cpu_percent    = $avgIdleCpuPct
      peak_idle_cpu_percent   = $peakCpuPct
      unexpected_exits        = $unexpectedExits
      exit_code_observed      = $exitCodeObserved
    }
    checks                     = [ordered]@{
      memory_within_limit = $memoryPass
      cpu_within_limit    = $cpuPass
      zero_crashes        = $stabilityPass
    }
    samples                    = $samples
  }

  $jsonText = ($reportObject | ConvertTo-Json -Depth 6) + "`n"
  Write-Utf8NoBom -Path $reportPaths.Json -Text $jsonText

  $memBadge = if ($memoryPass) { "PASS" } else { "FAIL" }
  $cpuBadge = if ($cpuPass) { "PASS" } else { "FAIL" }
  $crashBadge = if ($stabilityPass) { "PASS" } else { "FAIL" }

  $mdLines = New-Object System.Collections.Generic.List[string]
  $mdLines.Add("# Meetin Stability (Soak) Report")
  $mdLines.Add("")
  $mdLines.Add("- **Status:** ``$statusText``")
  $mdLines.Add("- **Target:** ``$targetLabel`` (mode: ``$($reportObject.mode)``)")
  $mdLines.Add("- **Started (UTC):** $($reportObject.started_utc)")
  $mdLines.Add("- **Finished (UTC):** $($reportObject.finished_utc)")
  $mdLines.Add("- **Measured Duration:** $($reportObject.measured_duration_sec) s (configured: $DurationSeconds s, samples: $($samples.Count))")
  $mdLines.Add("")
  $mdLines.Add("## Threshold Checks")
  $mdLines.Add("")
  $mdLines.Add("| Metric | Measured | Threshold | Result |")
  $mdLines.Add("| --- | --- | --- | --- |")
  $mdLines.Add("| Net Working Set Growth | $netMemoryGrowthMb MB (baseline: $baselineWsMb MB -> final: $finalWsMb MB, peak: $peakWsMb MB) | < $MaxMemoryGrowthMb MB | $memBadge |")
  $mdLines.Add("| Average Idle CPU | $avgIdleCpuPct % (peak: $peakCpuPct %) | < $MaxIdleCpuPercent % | $cpuBadge |")
  $mdLines.Add("| Unexpected Exits / Crashes | $unexpectedExits | 0 | $crashBadge |")
  $mdLines.Add("")
  $mdLines.Add("## Recent Samples")
  $mdLines.Add("")
  $mdLines.Add("| Elapsed (s) | Working Set (MB) | Private Memory (MB) | CPU (%) | Handles | Threads |")
  $mdLines.Add("| --- | --- | --- | --- | --- | --- |")

  $tailStart = [Math]::Max(0, $samples.Count - 10)
  for ($i = $tailStart; $i -lt $samples.Count; $i++) {
    $s = $samples[$i]
    $mdLines.Add("| $($s.elapsed_seconds) | $($s.working_set_mb) | $($s.private_memory_mb) | $($s.cpu_percent) | $($s.handle_count) | $($s.thread_count) |")
  }
  $mdLines.Add("")

  Write-Utf8NoBom -Path $reportPaths.Markdown -Text (($mdLines -join "`n"))

  Write-Host "[Meetin Soak] Durum: $statusText | Bellek artisi: ${netMemoryGrowthMb} MB (< ${MaxMemoryGrowthMb} MB) | Ort. CPU: ${avgIdleCpuPct}% (< ${MaxIdleCpuPercent}%) | Cikis: $unexpectedExits"
  Write-Host "[Meetin Soak] Raporlar: $($reportPaths.Markdown) ve $($reportPaths.Json)"

  if (-not $overallPass) {
    exit 1
  }
}
finally {
  if ($launchedByScript -and $null -ne $targetProc) {
    try {
      $targetProc.Refresh()
      if (-not $targetProc.HasExited) {
        [void]$targetProc.CloseMainWindow()
        if (-not $targetProc.WaitForExit(2000)) {
          $targetProc.Kill()
          [void]$targetProc.WaitForExit(3000)
        }
      }
    } catch {
      # Süreç zaten kapanmış olabilir
    }
    $targetProc.Dispose()
  }
}
