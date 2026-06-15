$ErrorActionPreference = 'Stop'

$repoRoot = Split-Path -Parent $PSScriptRoot
$csc = Join-Path $env:WINDIR 'Microsoft.NET\Framework64\v4.0.30319\csc.exe'
$dist = Join-Path $repoRoot 'dist'
$icon = Join-Path $repoRoot 'assets\LocalAreaInterconnection.ico'
$cliSource = Join-Path $repoRoot 'windows-cli\LocalAreaInterconnectionCli.cs'
$desktopSources = Get-ChildItem -Path (Join-Path $repoRoot 'windows-cli') -Filter 'LocalAreaInterconnectionDesktop*.cs' |
    Sort-Object Name |
    ForEach-Object { $_.FullName }
$gameProfilesSource = Join-Path $repoRoot 'assets\game-profiles.example.json'
$cliOut = Join-Path $dist 'LocalAreaInterconnection.Cli.exe'
$nativeCliOut = Join-Path $dist 'LocalAreaInterconnection.Native.Cli.exe'
$desktopOut = Join-Path $dist 'LocalAreaInterconnection.exe'
$gameProfilesOut = Join-Path $dist 'game-profiles.example.json'

if (-not (Test-Path -LiteralPath $csc)) {
    throw "C# compiler not found: $csc"
}

New-Item -ItemType Directory -Force -Path $dist | Out-Null

& $csc /nologo /target:exe /out:$cliOut /win32icon:$icon $cliSource
if ($LASTEXITCODE -ne 0) {
    throw "Failed to compile CLI backend."
}

& $csc /nologo /target:winexe /out:$desktopOut /win32icon:$icon /reference:System.Windows.Forms.dll /reference:System.Drawing.dll $desktopSources
if ($LASTEXITCODE -ne 0) {
    throw "Failed to compile desktop shell."
}

$cargo = Get-Command cargo -ErrorAction SilentlyContinue
if ($null -ne $cargo) {
    Push-Location (Join-Path $repoRoot 'native')
    try {
        & $cargo.Source build -p lai-cli --release
        if ($LASTEXITCODE -ne 0) {
            throw "Failed to compile Rust native CLI."
        }
    }
    finally {
        Pop-Location
    }

    $nativeCliBuild = Join-Path $repoRoot 'native\target\release\lai-cli.exe'
    if (Test-Path -LiteralPath $nativeCliBuild) {
        Copy-Item -LiteralPath $nativeCliBuild -Destination $nativeCliOut -Force
    }
    else {
        throw "Rust native CLI build output not found: $nativeCliBuild"
    }
}
else {
    Write-Warning "cargo was not found. Skipped Rust native CLI build: $nativeCliOut"
}

if (Test-Path -LiteralPath $gameProfilesSource) {
    Copy-Item -LiteralPath $gameProfilesSource -Destination $gameProfilesOut -Force
}

Write-Host "Built latest Windows test shell:"
Write-Host "  $desktopOut"
Write-Host "  $cliOut"
if (Test-Path -LiteralPath $nativeCliOut) {
    Write-Host "  $nativeCliOut"
}
if (Test-Path -LiteralPath $gameProfilesOut) {
    Write-Host "  $gameProfilesOut"
}
