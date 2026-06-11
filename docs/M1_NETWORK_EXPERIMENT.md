# M1 Network Experiment Notes

Goal: verify virtual IP connectivity, UDP unicast, and UDP broadcast handling between two Windows machines.

## Test Environment

Machine A:

- Windows version:
- Network type:
- Local subnet:
- Virtual adapter:
- Virtual IP:

Machine B:

- Windows version:
- Network type:
- Local subnet:
- Virtual adapter:
- Virtual IP:

## Checklist

- [ ] Virtual adapter installed.
- [ ] Virtual adapter enabled.
- [ ] Virtual IP assigned.
- [ ] `network-observe --adapter-scan true` reports the virtual adapter as present.
- [ ] A can ping B virtual IP.
- [ ] B can ping A virtual IP.
- [ ] `network-observe --ping-test <peer-virtual-ip>` reports tunnel and P2P state.
- [ ] UDP unicast A -> B works.
- [ ] UDP unicast B -> A works.
- [ ] UDP/TCP tests append packet observations with `--observe-file`.
- [ ] UDP broadcast is observed on the virtual adapter.
- [ ] `udp-broadcast-test --observe-file` appends a broadcast packet observation.
- [ ] Broadcast forwarding behavior is recorded.
- [ ] Windows firewall prompts and rules are recorded.
- [ ] `diagnostic-export --out <file>` writes an experiment bundle after the run.

## Measurements

| Test | Result | Notes |
|---|---|---|
| Ping latency |  |  |
| UDP packet loss |  |  |
| Broadcast packets/sec |  |  |
| CPU usage |  |  |
| 30-minute stability |  |  |

## Useful Commands

```powershell
.\dist\LocalAreaInterconnection.Cli.exe network-observe --adapter-name LocalAreaInterconnection --expected-ip 10.77.12.2 --subnet 10.77.12.0/24 --adapter-scan true
.\dist\LocalAreaInterconnection.Cli.exe network-observe --adapter-name LocalAreaInterconnection --expected-ip 10.77.12.2 --subnet 10.77.12.0/24 --adapter-netsh-output adapter.txt
.\dist\LocalAreaInterconnection.Cli.exe network-observe --ping-test 10.77.12.3 --expected-peers 1
.\dist\LocalAreaInterconnection.Cli.exe network-observe --ping-output ping.txt --expected-peers 1
.\dist\LocalAreaInterconnection.Cli.exe udp-loopback-test --port 39077 --message ping --observe-file packets.txt
.\dist\LocalAreaInterconnection.Cli.exe udp-broadcast-test --port 39078 --message discover --observe-file packets.txt
.\dist\LocalAreaInterconnection.Cli.exe network-observe --adapter-name LocalAreaInterconnection --expected-ip 10.77.12.2 --assigned-ip 10.77.12.2 --subnet 10.77.12.0/24 --packet-observations packets.txt --game-ports 39077
.\dist\LocalAreaInterconnection.Cli.exe diagnostic-export --out diagnostic-bundle.json --adapter-name LocalAreaInterconnection --expected-ip 10.77.12.2 --subnet 10.77.12.0/24 --ping-test 10.77.12.3 --packet-observations packets.txt --broadcast-ports 39078 --game-ports 39077 --game-name "Example Game" --ports 39077,39078
```

The diagnostic bundle is read-only. It includes local adapter and Windows Firewall output, so review it before sharing outside the test group.

## Conclusions

What worked:

What failed:

Next action:

