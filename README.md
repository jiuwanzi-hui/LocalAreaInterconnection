# LocalAreaInterconnection

LocalAreaInterconnection is a virtual LAN tool for LAN-only PC games.

Current stage:

- Windows client target architecture selected: Tauri or another Windows shell plus a Rust native core.
- Rust native workspace lives under `native/`.
- Former scripting prototypes have been removed to keep implementation aligned with the Windows desktop target.
- Room, invite, subnet, room member lifecycle, broadcast policy, game profile, game network plan, Windows firewall dry-run plan, firewall diagnostics, network observation, runtime observation conversion, diagnostic export and general diagnostics logic are modeled in Rust.
- Rust CLI can create and decode invites, create join plans, render room session summaries, run diagnosis summaries, render game/firewall plans, preview firewall diagnostics, evaluate network experiment observations, and write read-only diagnostic bundles.

Run tests:

```bash
cd native
cargo test
```

Run the Rust CLI:

```bash
cd native
cargo run -p lai-cli -- init --room-name "Friday LAN" --host "Alice"
cargo run -p lai-cli -- room-summary --room-name "Friday LAN" --host "Alice" --peer "Bob" --peer "Carol"
cargo run -p lai-cli -- game-plan --game-name "Example Game" --subnet 10.77.12.0/24 --ports 27015,27016
cargo run -p lai-cli -- firewall-plan --game-name "Example Game" --subnet 10.77.12.0/24 --ports 27015,27016
cargo run -p lai-cli -- firewall-diagnose --game-name "Example Game" --subnet 10.77.12.0/24 --ports 27015,27016 --observed udp:27015,tcp:27015
cargo run -p lai-cli -- firewall-diagnose --game-name "Example Game" --subnet 10.77.12.0/24 --ports 27015,27016 --netsh-output firewall-rules.txt
cargo run -p lai-cli -- adapter-plan --subnet 10.77.12.0/24 --ip 10.77.12.2
cargo run -p lai-cli -- network-observe --adapter-name LocalAreaInterconnection --expected-ip 10.77.12.2 --assigned-ip 10.77.12.2 --subnet 10.77.12.0/24 --connected-peers 1 --expected-peers 1 --broadcast-ports 27015 --game-ports 27015 --packets udp:10.77.12.2:10.77.12.255:27015:broadcast:outbound:8,udp:10.77.12.2:10.77.12.3:27015:unicast:outbound:8
cargo run -p lai-cli -- network-observe --adapter-name LocalAreaInterconnection --expected-ip 10.77.12.2 --subnet 10.77.12.0/24 --adapter-netsh-output adapter.txt
cargo run -p lai-cli -- network-observe --adapter-name LocalAreaInterconnection --expected-ip 10.77.12.2 --assigned-ip 10.77.12.2 --subnet 10.77.12.0/24 --packet-observations packets.txt --broadcast-ports 27015 --game-ports 27015
cargo run -p lai-cli -- network-observe --ping-output ping.txt --expected-peers 1
cargo run -p lai-cli -- diagnostic-export --out diagnostic-bundle.json --adapter-name LocalAreaInterconnection --expected-ip 10.77.12.2 --assigned-ip 10.77.12.2 --subnet 10.77.12.0/24 --ping-test 127.0.0.1 --packet-observations packets.txt --broadcast-ports 39078 --game-ports 39077 --game-name "Example Game" --ports 39077,39078
```

These commands require a local Rust toolchain. The current development machine has been verified with `cargo 1.96.0` and `rustc 1.96.0`.

Current runnable Windows test build:

```powershell
.\dist\LocalAreaInterconnection.Cli.exe help
.\dist\LocalAreaInterconnection.Cli.exe init --room-name "Friday LAN" --host Alice
.\dist\LocalAreaInterconnection.Cli.exe adapter-plan --subnet 10.77.12.0/24 --ip 10.77.12.2
.\dist\LocalAreaInterconnection.Cli.exe adapter-apply --adapter-name LocalAreaInterconnection --subnet 10.77.12.0/24 --ip 10.77.12.2
.\dist\LocalAreaInterconnection.Cli.exe adapter-scan --adapter-name LocalAreaInterconnection --subnet 10.77.12.0/24 --ip 10.77.12.2
.\dist\LocalAreaInterconnection.Cli.exe diagnose --virtual-adapter ok --firewall allowed --p2p ok --broadcast missing
.\dist\LocalAreaInterconnection.Cli.exe network-observe --adapter-name LocalAreaInterconnection --expected-ip 10.77.12.2 --assigned-ip 10.77.12.2 --subnet 10.77.12.0/24 --connected-peers 1 --expected-peers 1 --broadcast-ports 27015 --game-ports 27015 --packets udp:10.77.12.2:10.77.12.255:27015:broadcast:outbound:8,udp:10.77.12.2:10.77.12.3:27015:unicast:outbound:8
.\dist\LocalAreaInterconnection.Cli.exe network-observe --adapter-name LocalAreaInterconnection --expected-ip 10.77.12.2 --assigned-ip 10.77.12.2 --subnet 10.77.12.0/24 --packet-observations packets.txt --broadcast-ports 27015 --game-ports 27015
.\dist\LocalAreaInterconnection.Cli.exe network-observe --adapter-name LocalAreaInterconnection --expected-ip 10.77.12.2 --subnet 10.77.12.0/24 --adapter-netsh-output adapter.txt
.\dist\LocalAreaInterconnection.Cli.exe network-observe --adapter-name LocalAreaInterconnection --expected-ip 10.77.12.2 --subnet 10.77.12.0/24 --adapter-scan true
.\dist\LocalAreaInterconnection.Cli.exe network-observe --ping-test 127.0.0.1 --expected-peers 1
.\dist\LocalAreaInterconnection.Cli.exe network-observe --ping-output ping.txt --expected-peers 1
.\dist\LocalAreaInterconnection.Cli.exe diagnostic-export --out diagnostic-bundle.json --adapter-name LocalAreaInterconnection --expected-ip 10.77.12.2 --subnet 10.77.12.0/24 --ping-test 127.0.0.1 --packet-observations packets.txt --broadcast-ports 39078 --game-ports 39077 --game-name "Example Game" --ports 39077,39078
.\dist\LocalAreaInterconnection.Cli.exe firewall-plan --game-name "Example Game" --subnet 10.77.12.0/24 --ports 27015
.\dist\LocalAreaInterconnection.Cli.exe firewall-apply --game-name "Example Game" --subnet 10.77.12.0/24 --ports 27015
.\dist\LocalAreaInterconnection.Cli.exe firewall-remove --game-name "Example Game" --ports 27015
.\dist\LocalAreaInterconnection.Cli.exe firewall-diagnose --game-name "Example Game" --subnet 10.77.12.0/24 --ports 27015 --observed udp:27015
.\dist\LocalAreaInterconnection.Cli.exe firewall-scan --game-name "Example Game" --subnet 10.77.12.0/24 --ports 27015
.\dist\LocalAreaInterconnection.Cli.exe udp-loopback-test --port 39077 --message ping --observe-file packets.txt
.\dist\LocalAreaInterconnection.Cli.exe udp-broadcast-test --port 39078 --message discover --observe-file packets.txt
.\dist\LocalAreaInterconnection.Cli.exe udp-listen --port 39077 --timeout-ms 10000 --observe-file packets.txt
.\dist\LocalAreaInterconnection.Cli.exe udp-send --host 127.0.0.1 --port 39077 --message ping --observe-file packets.txt
.\dist\LocalAreaInterconnection.Cli.exe tcp-loopback-test --port 39079 --message ping --observe-file packets.txt
```

Desktop test shell:

```powershell
.\dist\LocalAreaInterconnection.exe
```

Build the latest Windows test shell:

```powershell
.\scripts\build-windows-test-shell.ps1
```

Or double-click:

```text
build-latest-exe.bat
```

Build and launch it:

```powershell
.\scripts\run-windows-test-shell.ps1
```

Or double-click:

```text
build-and-run-exe.bat
```

In JetBrains IDEs, select `Build latest Windows exe` to only regenerate `dist\LocalAreaInterconnection.exe`, or select `Build and run Windows exe` to regenerate and launch it.

The desktop test shell includes the current app icon, mist-blue background styling, soft glow, and static particle accents.
It initializes language from the Windows UI culture, supports English/Chinese switching in the title bar, and remembers the user's choice.
The desktop shell now starts from `LocalAreaInterconnection.exe`; the command backend is `LocalAreaInterconnection.Cli.exe`.
It can create/decode/join rooms, copy the generated invite and local virtual IP, run adapter/firewall/network diagnostics, and export a diagnostic bundle.
The right-side room details panel summarizes the room, virtual subnet, member/IP, connection checks, broadcast/game-traffic state, and next suggested action.
The packet observation file field lets the desktop shell append UDP/TCP/broadcast test observations and reuse that same file in network diagnostics and diagnostic export.
When a packet observation file is selected, UDP/TCP/broadcast test buttons also refresh the network diagnostics and room details after the test finishes.

Diagnostic export:

- Rust `lai-cli diagnostic-export` and the current Windows test CLI both write read-only JSON bundles with environment metadata, adapter scan/diagnosis, firewall scan/diagnosis, ping-derived tunnel observation, packet observation summary, and the combined `network-observe` report.
- The bundle may contain local adapter and Windows Firewall configuration. Review it before sharing publicly.

