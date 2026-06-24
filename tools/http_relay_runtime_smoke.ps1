$ErrorActionPreference = 'Stop'

$repo = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
$cli = Join-Path $repo 'native\target\debug\lai-cli.exe'
$relay = if ($env:LAI_RELAY_URL) { $env:LAI_RELAY_URL } else { 'http://49.235.146.152' }
$room = 'codex_http_runtime_' + [Guid]::NewGuid().ToString('N').Substring(0, 8)
$key = 'codex-runtime-key'
$tmp = Join-Path $repo 'native\target\tmp-http-relay'
New-Item -ItemType Directory -Force -Path $tmp | Out-Null

if (!(Test-Path $cli)) {
    throw "CLI not found at $cli. Run cargo build in native first."
}

$aOut = Join-Path $tmp 'a-out.json'
$bOut = Join-Path $tmp 'b-out.json'
$aStdout = Join-Path $tmp 'a.stdout'
$bStdout = Join-Path $tmp 'b.stdout'
$aStderr = Join-Path $tmp 'a.stderr'
$bStderr = Join-Path $tmp 'b.stderr'

Remove-Item -Force -ErrorAction SilentlyContinue $aOut, $bOut, $aStdout, $bStdout, $aStderr, $bStderr

$aArgs = @(
    'room-runtime-run',
    '--room-id', $room,
    '--peer-id', 'peer_a',
    '--virtual-ip', '10.77.12.2',
    '--bind', '0.0.0.0:0',
    '--peer', "peer_b,10.77.12.3,$relay",
    '--key', $key,
    '--duration-ms', '4500',
    '--heartbeat-interval-ms', '500',
    '--peer-timeout-ms', '0',
    '--snapshot-out', $aOut
)
$bArgs = @(
    'room-runtime-run',
    '--room-id', $room,
    '--peer-id', 'peer_b',
    '--virtual-ip', '10.77.12.3',
    '--bind', '0.0.0.0:0',
    '--peer', "peer_a,10.77.12.2,$relay",
    '--key', $key,
    '--duration-ms', '4500',
    '--heartbeat-interval-ms', '500',
    '--peer-timeout-ms', '0',
    '--snapshot-out', $bOut
)

$pa = Start-Process -FilePath $cli -ArgumentList $aArgs -NoNewWindow -PassThru -RedirectStandardOutput $aStdout -RedirectStandardError $aStderr
$pb = Start-Process -FilePath $cli -ArgumentList $bArgs -NoNewWindow -PassThru -RedirectStandardOutput $bStdout -RedirectStandardError $bStderr

$deadline = (Get-Date).AddSeconds(25)
while ((!$pa.HasExited -or !$pb.HasExited) -and (Get-Date) -lt $deadline) {
    Start-Sleep -Milliseconds 200
    $pa.Refresh()
    $pb.Refresh()
}

if (!$pa.HasExited -or !$pb.HasExited) {
    Stop-Process -Id @($pa.Id, $pb.Id) -Force -ErrorAction SilentlyContinue
    throw 'runtime smoke timed out'
}
$a = Get-Content -Path $aOut -Raw | ConvertFrom-Json
$b = Get-Content -Path $bOut -Raw | ConvertFrom-Json

function Assert-RelayRuntime($name, $json) {
    if ($json.tunnelServiceSnapshot.connection_path -ne 'relay') {
        throw "$name did not report relay path"
    }
    if ([int]$json.tunnelServiceSnapshot.connected_peer_count -lt 1) {
        throw "$name did not connect to a peer"
    }
    $receivedAcks = @($json.heartbeatAckPackets | Where-Object { $_.direction -eq 'received' })
    if ($receivedAcks.Count -lt 1) {
        throw "$name did not receive heartbeat ACKs through the relay"
    }
    $badDirect = @($json.heartbeatPackets | Where-Object { $_.connectionPath -ne 'relay' })
    if ($badDirect.Count -gt 0) {
        throw "$name sent a heartbeat outside the relay path"
    }
}

Assert-RelayRuntime 'peer_a' $a
Assert-RelayRuntime 'peer_b' $b

if ($null -ne $pa.ExitCode -and $pa.ExitCode -ne 0) {
    throw "peer_a runtime returned exit code $($pa.ExitCode) after writing a valid relay snapshot: $(Get-Content -Path $aStderr -Raw)"
}
if ($null -ne $pb.ExitCode -and $pb.ExitCode -ne 0) {
    throw "peer_b runtime returned exit code $($pb.ExitCode) after writing a valid relay snapshot: $(Get-Content -Path $bStderr -Raw)"
}

[pscustomobject]@{
    status = 'ok'
    room = $room
    relay = $relay
    peerAConnectionPath = $a.tunnelServiceSnapshot.connection_path
    peerBConnectionPath = $b.tunnelServiceSnapshot.connection_path
    peerAHeartbeatAcks = @($a.heartbeatAckPackets | Where-Object { $_.direction -eq 'received' }).Count
    peerBHeartbeatAcks = @($b.heartbeatAckPackets | Where-Object { $_.direction -eq 'received' }).Count
    peerAStatus = $a.status
    peerBStatus = $b.status
    peerAStdout = $aStdout
    peerBStdout = $bStdout
} | ConvertTo-Json -Depth 5
