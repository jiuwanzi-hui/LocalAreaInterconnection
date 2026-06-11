$ErrorActionPreference = 'Stop'

$repoRoot = Split-Path -Parent $PSScriptRoot
$csc = Join-Path $env:WINDIR 'Microsoft.NET\Framework64\v4.0.30319\csc.exe'
$dist = Join-Path $repoRoot 'dist'
$icon = Join-Path $repoRoot 'assets\LocalAreaInterconnection.ico'
$cliSource = Join-Path $repoRoot 'windows-cli\LocalAreaInterconnectionCli.cs'
$desktopSource = Join-Path $repoRoot 'windows-cli\LocalAreaInterconnectionDesktop.cs'
$cliOut = Join-Path $dist 'LocalAreaInterconnection.Cli.exe'
$desktopOut = Join-Path $dist 'LocalAreaInterconnection.exe'

if (-not (Test-Path -LiteralPath $csc)) {
    throw "C# compiler not found: $csc"
}

New-Item -ItemType Directory -Force -Path $dist | Out-Null

& $csc /nologo /target:exe /out:$cliOut /win32icon:$icon $cliSource
if ($LASTEXITCODE -ne 0) {
    throw "Failed to compile CLI backend."
}

& $csc /nologo /target:winexe /out:$desktopOut /win32icon:$icon /reference:System.Windows.Forms.dll /reference:System.Drawing.dll $desktopSource
if ($LASTEXITCODE -ne 0) {
    throw "Failed to compile desktop shell."
}

Write-Host "Built latest Windows test shell:"
Write-Host "  $desktopOut"
Write-Host "  $cliOut"
