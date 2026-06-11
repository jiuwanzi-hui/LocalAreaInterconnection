$ErrorActionPreference = 'Stop'

$repoRoot = Split-Path -Parent $PSScriptRoot
& (Join-Path $PSScriptRoot 'build-windows-test-shell.ps1')

$desktopOut = Join-Path $repoRoot 'dist\LocalAreaInterconnection.exe'
if (-not (Test-Path -LiteralPath $desktopOut)) {
    throw "Desktop executable was not built: $desktopOut"
}

Start-Process -FilePath $desktopOut -WorkingDirectory (Split-Path -Parent $desktopOut)
Write-Host "Started:"
Write-Host "  $desktopOut"
