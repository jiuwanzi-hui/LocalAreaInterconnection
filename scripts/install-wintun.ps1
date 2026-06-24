$ErrorActionPreference = 'Stop'

$repoRoot = Split-Path -Parent $PSScriptRoot
$dist = Join-Path $repoRoot 'dist'
$cache = Join-Path $repoRoot 'tools\wintun'
$zip = Join-Path $cache 'wintun-0.14.1.zip'
$expectedSha256 = '07c256185d6ee3652e09fa55c0b673e2624b565e02c4b9091c79ca7d2f24ef51'
$url = 'https://www.wintun.net/builds/wintun-0.14.1.zip'

New-Item -ItemType Directory -Force -Path $cache | Out-Null
New-Item -ItemType Directory -Force -Path $dist | Out-Null

if (-not (Test-Path -LiteralPath $zip)) {
    Write-Host "Downloading Wintun 0.14.1..."
    Invoke-WebRequest -UseBasicParsing -Uri $url -OutFile $zip
}

$actualSha256 = (Get-FileHash -Algorithm SHA256 -LiteralPath $zip).Hash.ToLowerInvariant()
if ($actualSha256 -ne $expectedSha256) {
    throw "Wintun zip hash mismatch. Expected $expectedSha256 but got $actualSha256."
}

$extract = Join-Path $cache 'wintun-0.14.1'
if (Test-Path -LiteralPath $extract) {
    Remove-Item -LiteralPath $extract -Recurse -Force
}
Expand-Archive -LiteralPath $zip -DestinationPath $extract -Force

$dll = Join-Path $extract 'wintun\bin\amd64\wintun.dll'
if (-not (Test-Path -LiteralPath $dll)) {
    throw "Wintun amd64 DLL not found after extraction: $dll"
}

Copy-Item -LiteralPath $dll -Destination (Join-Path $dist 'wintun.dll') -Force
Write-Host "Installed Wintun DLL:"
Write-Host "  $(Join-Path $dist 'wintun.dll')"
