param(
    [Parameter(Mandatory)][string]$Baseline,
    [Parameter(Mandatory)][string]$Candidate,
    [ValidateRange(3,100)][int]$Runs = 10,
    [string]$Output = "artifacts/startup-benchmark.json"
)
$ErrorActionPreference = 'Stop'
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class StartupWindow {
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hwnd);
}
'@
function Measure-Launch([string]$Path) {
    $watch = [Diagnostics.Stopwatch]::StartNew()
    $process = Start-Process -FilePath $Path -PassThru
    try {
        do {
            $process.Refresh()
            if ($process.HasExited) { throw "Process exited before showing a window: $Path" }
            if ($watch.Elapsed.TotalSeconds -gt 30) { throw "Window startup timed out: $Path" }
            $ready = $process.MainWindowHandle -ne 0 -and [StartupWindow]::IsWindowVisible($process.MainWindowHandle)
            if (!$ready) { [Threading.Thread]::Sleep(1) }
        } until ($ready)
        $elapsed = $watch.Elapsed.TotalMilliseconds
        # Wait for initialization to finish before a normal WM_CLOSE shutdown.
        if (!$process.WaitForInputIdle(10000)) { throw "Window was not responsive: $Path" }
        return [Math]::Round($elapsed,2)
    } finally {
        if (!$process.HasExited) {
            $null = $process.CloseMainWindow()
            if (!$process.WaitForExit(10000)) { $process.Kill(); throw "Process did not close: $Path" }
        }
        $process.Dispose()
    }
}
$paths = @((Resolve-Path -LiteralPath $Baseline).Path, (Resolve-Path -LiteralPath $Candidate).Path)
$samples = @(@(), @())
# Warm both binaries first. Alternating order reduces cache/order bias.
foreach ($path in $paths) { $null = Measure-Launch $path }
for ($i=0; $i -lt $Runs; $i++) {
    $order = if ($i % 2) { @(1,0) } else { @(0,1) }
    foreach ($index in $order) { $samples[$index] += Measure-Launch $paths[$index] }
}
$results = for ($index=0; $index -lt 2; $index++) {
    $sorted = @($samples[$index] | Sort-Object)
    $middle = [int][Math]::Floor($Runs/2)
    $median = if ($Runs % 2) { $sorted[$middle] } else { ($sorted[$middle-1]+$sorted[$middle])/2 }
    [ordered]@{
        path = $paths[$index]; bytes = (Get-Item -LiteralPath $paths[$index]).Length
        sha256 = (Get-FileHash -LiteralPath $paths[$index] -Algorithm SHA256).Hash.ToLowerInvariant()
        median_ms = [Math]::Round($median,2); samples_ms = $samples[$index]
    }
}
$report = [ordered]@{
    measured_at = [DateTimeOffset]::Now.ToString('o')
    method = 'Warm launch; process start to visible main window; 1 warmup each; alternating order; normal close. Includes PowerShell Start-Process overhead. Not cold boot.'
    runs = $Runs; baseline = $results[0]; candidate = $results[1]
}
$outputPath = [IO.Path]::GetFullPath($Output)
New-Item -ItemType Directory -Force (Split-Path $outputPath -Parent) | Out-Null
$report | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $outputPath -Encoding utf8
$report | ConvertTo-Json -Depth 5
