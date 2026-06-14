$ErrorActionPreference = 'Stop'

$repoRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
& (Join-Path $repoRoot 'scripts\build-windows-test-shell.ps1')

$desktopOut = Join-Path $repoRoot 'dist\LocalAreaInterconnection.exe'
if (-not (Test-Path -LiteralPath $desktopOut)) {
    throw "Desktop executable was not built: $desktopOut"
}

Write-Host ''
Write-Host 'Build complete:'
Write-Host "  $desktopOut"
