param([string]$OutputDirectory = "artifacts/release")
$ErrorActionPreference = "Stop"
$projectRoot = Split-Path $PSScriptRoot -Parent
$outputPath = [IO.Path]::GetFullPath((Join-Path $projectRoot $OutputDirectory))
if (Test-Path -LiteralPath $outputPath) {
    if (Get-ChildItem -LiteralPath $outputPath -Force) {
        throw "Publish directory must be empty. Choose a new output directory; existing files will not be removed."
    }
}
dotnet publish (Join-Path $projectRoot "MouseMoverApp.csproj") -c Release -r win-x64 `
    -p:PublishSingleFile=true -p:SelfContained=true -p:PublishTrimmed=false `
    -p:EnableCompressionInSingleFile=true -p:IncludeNativeLibrariesForSelfExtract=true `
    -p:DebugType=None -p:DebugSymbols=false -o $outputPath
if ($LASTEXITCODE -ne 0) { throw "dotnet publish failed" }
$files = @(Get-ChildItem -LiteralPath $outputPath -Force)
if ($files.Count -ne 1 -or $files[0].Name -ne "MouseMoverApp.exe") {
    throw "Expected exactly one MouseMoverApp.exe in the delivery directory."
}
foreach ($instructions in @("README_EN.txt", "README_ZH-TW.txt")) {
    $sourcePath = Join-Path $projectRoot "docs/$instructions"
    # UTF-8 BOM keeps Traditional Chinese readable in Windows text editors.
    [IO.File]::WriteAllText((Join-Path $outputPath $instructions),
        [IO.File]::ReadAllText($sourcePath), [Text.UTF8Encoding]::new($true))
}
$digest = (Get-FileHash -LiteralPath $files[0].FullName -Algorithm SHA256).Hash.ToLowerInvariant()
$checksumPath = Join-Path (Split-Path $outputPath -Parent) "SHA256SUMS.txt"
[IO.File]::WriteAllText($checksumPath,"$digest  MouseMoverApp.exe`n",[Text.UTF8Encoding]::new($false))
Write-Output "Single-file EXE: $($files[0].FullName)"
Write-Output "Instructions: README_EN.txt, README_ZH-TW.txt"
Write-Output "SHA256: $digest"
