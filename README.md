# LocalAreaInterconnection

LocalAreaInterconnection is a virtual LAN tool for LAN-only PC games.

Current stage:

- Windows client target architecture selected: Tauri or another Windows shell plus a Rust native core.
- Rust native workspace lives under `native/`.
- Former scripting prototypes have been removed to keep implementation aligned with the Windows desktop target.
- Room, invite, subnet, room member lifecycle, broadcast policy, game profile, game network plan, Windows firewall dry-run plan, firewall diagnostics, virtual adapter command execution previews, encrypted UDP tunnel envelopes, UDP forwarding observations, network observation, runtime observation conversion, diagnostic export and general diagnostics logic are modeled in Rust.
- Rust CLI can create and decode invites, create join plans, render room session summaries, run diagnosis summaries, render game/firewall plans, preview firewall diagnostics, preview/apply adapter `netsh` commands, run encrypted UDP tunnel tests, exchange P2P/NAT traversal probes, run user-space UDP forwarding tests, evaluate network experiment observations, and write read-only diagnostic bundles.

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
cargo run -p lai-cli -- adapter-apply --adapter-name LocalAreaInterconnection --subnet 10.77.12.0/24 --ip 10.77.12.2
cargo run -p lai-cli -- adapter-ensure --adapter-name LocalAreaInterconnection --subnet 10.77.12.0/24 --ip 10.77.12.2 --adapter-scan true
cargo run -p lai-cli -- wintun-detect
cargo run -p lai-cli -- wintun-adapter-create --adapter-name LocalAreaInterconnection --tunnel-type LocalAreaInterconnection
cargo run -p lai-cli -- wintun-adapter-delete --adapter-name LocalAreaInterconnection --tunnel-type LocalAreaInterconnection
cargo run -p lai-cli -- wintun-adapter-open --adapter-name LocalAreaInterconnection
cargo run -p lai-cli -- wintun-session-probe --adapter-name LocalAreaInterconnection
cargo run -p lai-cli -- wintun-packet-send-probe --adapter-name LocalAreaInterconnection --source-ip 10.77.12.2 --destination-ip 10.77.12.255 --source-port 39077 --destination-port 27015 --message probe --broadcast true
cargo run -p lai-cli -- wintun-packet-receive-probe --adapter-name LocalAreaInterconnection --max-attempts 8 --poll-interval-ms 25
cargo run -p lai-cli -- virtual-packet-plan --adapter-name LocalAreaInterconnection --backend wintun --mtu 1420
cargo run -p lai-cli -- virtual-packet-build-tcp --source-ip 10.77.12.2 --destination-ip 10.77.12.3 --source-port 50123 --destination-port 27015 --message "hello tcp"
cargo run -p lai-cli -- virtual-packet-parse-summary --packet-base64 <packetBase64>
cargo run -p lai-cli -- virtual-packet-loopback-test --source-ip 10.77.12.2 --destination-ip 10.77.12.255 --source-port 39077 --destination-port 27015 --message discover
cargo run -p lai-cli -- tunnel-loopback-test --key test-room-key --message ping
cargo run -p lai-cli -- p2p-handshake-loopback-test --key test-room-key --virtual-ip 10.77.12.2 --room-id room_test --peer-id peer_a --responder-peer-id peer_b
cargo run -p lai-cli -- p2p-handshake-listen --bind 0.0.0.0:39090 --key test-room-key --responder-peer-id peer_b
cargo run -p lai-cli -- p2p-handshake-send --peer 127.0.0.1:39090 --key test-room-key --virtual-ip 10.77.12.2 --room-id room_test --peer-id peer_a
cargo run -p lai-cli -- nat-candidates --room-id room_test --peer-id peer_a --bind 0.0.0.0:0
cargo run -p lai-cli -- nat-plan --local-offer <localOfferJsonOrFile> --remote-offer <remoteOfferJsonOrFile>
cargo run -p lai-cli -- nat-hole-punch --room-id room_test --peer-id peer_a --bind 0.0.0.0:39090 --remote-offer <remoteOfferJsonOrFile>
cargo run -p lai-cli -- nat-p2p-bootstrap --room-id room_test --peer-id peer_a --virtual-ip 10.77.12.2 --key test-room-key --bind 0.0.0.0:39090 --remote-offer <remoteOfferJsonOrFile>
cargo run -p lai-cli -- nat-hole-punch-loopback-test --room-id room_test --peer-a peer_a --peer-b peer_b
cargo run -p lai-cli -- coordination-store-init --out coordination-store.json
cargo run -p lai-cli -- coordination-offer-publish --store coordination-store.json --offer <localOfferJsonOrFile>
cargo run -p lai-cli -- coordination-offer-fetch --store coordination-store.json --room-id room_test --peer-id peer_a
cargo run -p lai-cli -- coordination-heartbeat --store coordination-store.json --room-id room_test --peer-id peer_a
cargo run -p lai-cli -- coordination-prune --store coordination-store.json
cargo run -p lai-cli -- tunnel-listen --bind 0.0.0.0:39090 --key test-room-key --max-packets 1
cargo run -p lai-cli -- tunnel-send --peer 127.0.0.1:39090 --key test-room-key --message ping
cargo run -p lai-cli -- udp-forward --listen 0.0.0.0:39078 --forward 127.0.0.1:39079 --observe-file packets.txt --broadcast true
cargo run -p lai-cli -- udp-forward-loopback-test --message ping --observe-file packets.txt
cargo run -p lai-cli -- udp-capture --listen 0.0.0.0:27015 --observe-file packets.txt
cargo run -p lai-cli -- udp-capture-loopback-test --message ping --observe-file packets.txt
cargo run -p lai-cli -- room-runtime-plan --room-id room_test --peer-id peer_a --virtual-ip 10.77.12.2 --bind 0.0.0.0:39090 --peer peer_b,10.77.12.3,127.0.0.1:39091 --game-ports 27015 --broadcast-ports 27015,39078
cargo run -p lai-cli -- room-runtime-run --room-id room_test --peer-id peer_a --virtual-ip 10.77.12.2 --bind 127.0.0.1:0 --key test-room-key --game-ports 0 --broadcast-ports 0 --duration-ms 250 --self-probe true --capture-self-probe true --forward-self-probe true --inject-self-probe true --observe-file packets.txt --snapshot-out runtime-snapshot.json
cargo run -p lai-cli -- room-runtime-run --room-id room_test --peer-id peer_a --virtual-ip 10.77.12.2 --bind 127.0.0.1:0 --key test-room-key --nat-bootstrap-peer peer_b,10.77.12.3,nat-bootstrap-result.json --broadcast-ports 27015
cargo run -p lai-cli -- room-runtime-run --room-id room_test --peer-id peer_a --virtual-ip 10.77.12.2 --bind 0.0.0.0:39090 --key test-room-key --nat-bootstrap-remote-peer peer_b,10.77.12.3,remote-offer.json --broadcast-ports 27015
cargo run -p lai-cli -- room-runtime-run --room-id room_test --peer-id peer_a --virtual-ip 10.77.12.2 --bind 0.0.0.0:39090 --key test-room-key --coordination-store coordination-store.json --coordination-peer peer_b,10.77.12.3 --broadcast-ports 27015
cargo run -p lai-cli -- room-runtime-run --room-id room_test --peer-id peer_a --virtual-ip 10.77.12.2 --bind 127.0.0.1:0 --key test-room-key --game-ports 0 --broadcast-ports 0 --duration-ms 250 --self-probe true --capture-self-probe true --forward-self-probe true --inject-self-probe true --packet-io-backend wintun --forward-raw-ipv4 true
cargo run -p lai-cli -- room-runtime-run --room-id room_test --peer-id peer_a --virtual-ip 10.77.12.2 --bind 127.0.0.1:0 --key test-room-key --game-ports 27015 --broadcast-ports 27015 --duration-ms 5000 --packet-io-backend wintun --forward-raw-ipv4 true --wintun-runtime true
cargo run -p lai-cli -- room-runtime-run --room-id room_test --peer-id peer_a --virtual-ip 10.77.12.2 --bind 127.0.0.1:0 --key test-room-key --duration-ms 0 --self-probe true --heartbeat-interval-ms 500 --stop-file runtime.stop --snapshot-out runtime-snapshot.json --snapshot-interval-ms 1000
cargo run -p lai-cli -- network-observe --adapter-name LocalAreaInterconnection --expected-ip 10.77.12.2 --assigned-ip 10.77.12.2 --subnet 10.77.12.0/24 --connected-peers 1 --expected-peers 1 --broadcast-ports 27015 --game-ports 27015 --packets udp:10.77.12.2:10.77.12.255:27015:broadcast:outbound:8,udp:10.77.12.2:10.77.12.3:27015:unicast:outbound:8
cargo run -p lai-cli -- network-observe --adapter-name LocalAreaInterconnection --expected-ip 10.77.12.2 --subnet 10.77.12.0/24 --adapter-netsh-output adapter.txt
cargo run -p lai-cli -- network-observe --adapter-name LocalAreaInterconnection --expected-ip 10.77.12.2 --assigned-ip 10.77.12.2 --subnet 10.77.12.0/24 --packet-observations packets.txt --broadcast-ports 27015 --game-ports 27015
cargo run -p lai-cli -- network-observe --ping-output ping.txt --expected-peers 1
cargo run -p lai-cli -- diagnostic-export --out diagnostic-bundle.json --adapter-name LocalAreaInterconnection --expected-ip 10.77.12.2 --assigned-ip 10.77.12.2 --subnet 10.77.12.0/24 --ping-test 127.0.0.1 --packet-observations packets.txt --broadcast-ports 39078 --game-ports 39077 --game-name "Example Game" --ports 39077,39078 --packet-io-backend wintun
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
.\dist\LocalAreaInterconnection.Native.Cli.exe diagnostic-export --out diagnostic-bundle.json --adapter-name LocalAreaInterconnection --expected-ip 10.77.12.2 --subnet 10.77.12.0/24 --ping-test 127.0.0.1 --packet-observations packets.txt --broadcast-ports 39078 --game-ports 39077 --game-name "Example Game" --ports 39077,39078 --packet-io-backend wintun
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
.\dist\LocalAreaInterconnection.Native.Cli.exe adapter-apply --adapter-name LocalAreaInterconnection --subnet 10.77.12.0/24 --ip 10.77.12.2
.\dist\LocalAreaInterconnection.Native.Cli.exe adapter-ensure --adapter-name LocalAreaInterconnection --subnet 10.77.12.0/24 --ip 10.77.12.2 --adapter-scan true
.\dist\LocalAreaInterconnection.Native.Cli.exe wintun-detect
.\dist\LocalAreaInterconnection.Native.Cli.exe wintun-adapter-create --adapter-name LocalAreaInterconnection --tunnel-type LocalAreaInterconnection
.\dist\LocalAreaInterconnection.Native.Cli.exe wintun-adapter-delete --adapter-name LocalAreaInterconnection --tunnel-type LocalAreaInterconnection
.\dist\LocalAreaInterconnection.Native.Cli.exe wintun-adapter-open --adapter-name LocalAreaInterconnection
.\dist\LocalAreaInterconnection.Native.Cli.exe wintun-session-probe --adapter-name LocalAreaInterconnection
.\dist\LocalAreaInterconnection.Native.Cli.exe wintun-packet-send-probe --adapter-name LocalAreaInterconnection --source-ip 10.77.12.2 --destination-ip 10.77.12.255 --source-port 39077 --destination-port 27015 --message probe --broadcast true
.\dist\LocalAreaInterconnection.Native.Cli.exe wintun-packet-receive-probe --adapter-name LocalAreaInterconnection --max-attempts 8 --poll-interval-ms 25
.\dist\LocalAreaInterconnection.Native.Cli.exe virtual-packet-plan --adapter-name LocalAreaInterconnection --backend wintun --mtu 1420
.\dist\LocalAreaInterconnection.Native.Cli.exe virtual-packet-build-tcp --source-ip 10.77.12.2 --destination-ip 10.77.12.3 --source-port 50123 --destination-port 27015 --message "hello tcp"
.\dist\LocalAreaInterconnection.Native.Cli.exe virtual-packet-parse-summary --packet-base64 <packetBase64>
.\dist\LocalAreaInterconnection.Native.Cli.exe virtual-packet-loopback-test --source-ip 10.77.12.2 --destination-ip 10.77.12.255 --source-port 39077 --destination-port 27015 --message discover
.\dist\LocalAreaInterconnection.Native.Cli.exe tunnel-loopback-test --key test-room-key --message ping
.\dist\LocalAreaInterconnection.Native.Cli.exe p2p-handshake-loopback-test --key test-room-key --virtual-ip 10.77.12.2 --room-id room_test --peer-id peer_a --responder-peer-id peer_b
.\dist\LocalAreaInterconnection.Native.Cli.exe p2p-handshake-listen --bind 0.0.0.0:39090 --key test-room-key --responder-peer-id peer_b
.\dist\LocalAreaInterconnection.Native.Cli.exe p2p-handshake-send --peer 127.0.0.1:39090 --key test-room-key --virtual-ip 10.77.12.2 --room-id room_test --peer-id peer_a
.\dist\LocalAreaInterconnection.Native.Cli.exe nat-candidates --room-id room_test --peer-id peer_a --bind 0.0.0.0:0
.\dist\LocalAreaInterconnection.Native.Cli.exe nat-plan --local-offer <localOfferJsonOrFile> --remote-offer <remoteOfferJsonOrFile>
.\dist\LocalAreaInterconnection.Native.Cli.exe nat-hole-punch --room-id room_test --peer-id peer_a --bind 0.0.0.0:39090 --remote-offer <remoteOfferJsonOrFile>
.\dist\LocalAreaInterconnection.Native.Cli.exe nat-p2p-bootstrap --room-id room_test --peer-id peer_a --virtual-ip 10.77.12.2 --key test-room-key --bind 0.0.0.0:39090 --remote-offer <remoteOfferJsonOrFile>
.\dist\LocalAreaInterconnection.Native.Cli.exe nat-hole-punch-loopback-test --room-id room_test --peer-a peer_a --peer-b peer_b
.\dist\LocalAreaInterconnection.Native.Cli.exe coordination-store-init --out coordination-store.json
.\dist\LocalAreaInterconnection.Native.Cli.exe coordination-offer-publish --store coordination-store.json --offer <localOfferJsonOrFile>
.\dist\LocalAreaInterconnection.Native.Cli.exe coordination-offer-fetch --store coordination-store.json --room-id room_test --peer-id peer_a
.\dist\LocalAreaInterconnection.Native.Cli.exe coordination-heartbeat --store coordination-store.json --room-id room_test --peer-id peer_a
.\dist\LocalAreaInterconnection.Native.Cli.exe coordination-prune --store coordination-store.json
.\dist\LocalAreaInterconnection.Native.Cli.exe udp-forward-loopback-test --message ping --observe-file packets.txt
.\dist\LocalAreaInterconnection.Native.Cli.exe udp-capture-loopback-test --message ping --observe-file packets.txt
.\dist\LocalAreaInterconnection.Native.Cli.exe room-runtime-plan --room-id room_test --peer-id peer_a --virtual-ip 10.77.12.2 --bind 0.0.0.0:39090 --peer peer_b,10.77.12.3,127.0.0.1:39091 --game-ports 27015 --broadcast-ports 27015,39078
.\dist\LocalAreaInterconnection.Native.Cli.exe room-runtime-run --room-id room_test --peer-id peer_a --virtual-ip 10.77.12.2 --bind 127.0.0.1:0 --key test-room-key --game-ports 0 --broadcast-ports 0 --duration-ms 250 --self-probe true --capture-self-probe true --forward-self-probe true --inject-self-probe true --observe-file packets.txt --snapshot-out runtime-snapshot.json
.\dist\LocalAreaInterconnection.Native.Cli.exe room-runtime-run --room-id room_test --peer-id peer_a --virtual-ip 10.77.12.2 --bind 127.0.0.1:0 --key test-room-key --nat-bootstrap-peer peer_b,10.77.12.3,nat-bootstrap-result.json --broadcast-ports 27015
.\dist\LocalAreaInterconnection.Native.Cli.exe room-runtime-run --room-id room_test --peer-id peer_a --virtual-ip 10.77.12.2 --bind 0.0.0.0:39090 --key test-room-key --nat-bootstrap-remote-peer peer_b,10.77.12.3,remote-offer.json --broadcast-ports 27015
.\dist\LocalAreaInterconnection.Native.Cli.exe room-runtime-run --room-id room_test --peer-id peer_a --virtual-ip 10.77.12.2 --bind 0.0.0.0:39090 --key test-room-key --coordination-store coordination-store.json --coordination-peer peer_b,10.77.12.3 --broadcast-ports 27015
.\dist\LocalAreaInterconnection.Native.Cli.exe room-runtime-run --room-id room_test --peer-id peer_a --virtual-ip 10.77.12.2 --bind 127.0.0.1:0 --key test-room-key --game-ports 0 --broadcast-ports 0 --duration-ms 250 --self-probe true --capture-self-probe true --forward-self-probe true --inject-self-probe true --packet-io-backend wintun --forward-raw-ipv4 true
.\dist\LocalAreaInterconnection.Native.Cli.exe room-runtime-run --room-id room_test --peer-id peer_a --virtual-ip 10.77.12.2 --bind 127.0.0.1:0 --key test-room-key --game-ports 27015 --broadcast-ports 27015 --duration-ms 5000 --packet-io-backend wintun --forward-raw-ipv4 true --wintun-runtime true
.\dist\LocalAreaInterconnection.Native.Cli.exe room-runtime-run --room-id room_test --peer-id peer_a --virtual-ip 10.77.12.2 --bind 127.0.0.1:0 --key test-room-key --duration-ms 0 --self-probe true --heartbeat-interval-ms 500 --stop-file runtime.stop --snapshot-out runtime-snapshot.json --snapshot-interval-ms 1000
```

`LocalAreaInterconnection.Native.Cli.exe adapter-apply` prints the planned Administrator command sequence by default. It only runs `netsh` when you rerun it from an elevated Administrator terminal with `--yes true`.
`LocalAreaInterconnection.Native.Cli.exe adapter-ensure` reads the current adapter state, reports exact mismatches, and only applies `netsh` changes when you rerun it from an elevated Administrator terminal with `--yes true`.
`LocalAreaInterconnection.Native.Cli.exe wintun-adapter-create` is also confirmation-gated: by default it prints a plan, and only calls Wintun when rerun from an Administrator terminal with `--yes true`.
`LocalAreaInterconnection.Native.Cli.exe wintun-adapter-delete` follows the same confirmation gate, but the current public Wintun API does not expose a delete-specific adapter function; with `--yes true` it reports `adapter-delete-api-unavailable` instead of faking deletion.
`LocalAreaInterconnection.Native.Cli.exe wintun-adapter-open` is a probe: it opens an existing adapter and closes it immediately, without returning a reusable raw handle.
`LocalAreaInterconnection.Native.Cli.exe wintun-session-probe` opens an existing Wintun adapter, starts a short-lived session with a 128 KiB ring, ends it, and closes the adapter. It does not read or write packets.
`LocalAreaInterconnection.Native.Cli.exe wintun-packet-send-probe` is confirmation-gated and only allocates/sends one constructed IPv4/UDP packet through a short-lived Wintun session when rerun with `--yes true`.
`LocalAreaInterconnection.Native.Cli.exe wintun-packet-receive-probe` starts a short-lived Wintun session and polls `ReceivePacket` a bounded number of times; it returns `empty` when no packet is available.
`LocalAreaInterconnection.Native.Cli.exe virtual-packet-loopback-test` builds and parses a raw IPv4/UDP packet. `virtual-packet-build-tcp` and `virtual-packet-parse-summary` cover the TCP and generic IPv4 summary boundary used by Wintun/TAP packet reader/writer paths.
`LocalAreaInterconnection.Native.Cli.exe nat-hole-punch` keeps the UDP socket alive while it builds a local NAT offer, sends punch packets to every remote UDP candidate, and records any replies. The standalone `nat-candidates` command is mainly for checking offer/coordination JSON shape because its temporary socket closes when the command exits.
`LocalAreaInterconnection.Native.Cli.exe nat-p2p-bootstrap` continues from `nat-hole-punch`: it sends punch packets, then sends encrypted `p2p-handshake-hello` packets to the same candidate endpoints and reports the first accepted ACK as `selectedPeer`.
`LocalAreaInterconnection.Native.Cli.exe coordination-*` commands provide a local JSON-file coordination store for publish/fetch/heartbeat/TTL/prune experiments. It is not a public server yet, but it fixes the room offer exchange contract that an HTTP coordination service can reuse.
`LocalAreaInterconnection.Native.Cli.exe room-runtime-run --nat-bootstrap-peer peer_b,10.77.12.3,nat-bootstrap-result.json` converts a successful bootstrap result into a runtime peer endpoint. The peer id must match `selectedPeer.responderPeerId`, and `selectedPeer.accepted` plus `selectedPeer.nonceMatched` must both be true.
`LocalAreaInterconnection.Native.Cli.exe room-runtime-run --nat-bootstrap-remote-peer peer_b,10.77.12.3,remote-offer.json` runs NAT/P2P bootstrap before starting the runtime, then adds the accepted `selectedPeer.endpoint` to the runtime peer list.
`LocalAreaInterconnection.Native.Cli.exe room-runtime-run --coordination-store coordination-store.json --coordination-peer peer_b,10.77.12.3` fetches peer offers from the local coordination store and runs bootstrap automatically for the requested peers before the runtime starts.
`LocalAreaInterconnection.Native.Cli.exe room-runtime-run --packet-io-backend wintun --forward-raw-ipv4 true` keeps the existing UDP self-test path, embeds a raw IPv4/UDP packet in encrypted `runtime-udp-forward` payloads, writes decoded raw virtual packets back into packet observation lines with `virtual-adapter` direction, and reports Wintun session/receive/send readiness through `packetIoProbe`, `adapterReadStatus`, and `adapterWriteStatus`. The Wintun send probe is not executed unless `--wintun-probe-send true` is passed explicitly.
`LocalAreaInterconnection.Native.Cli.exe room-runtime-run --wintun-runtime true` attempts the real Wintun data path: read IPv4 UDP/TCP/ICMP packets from the virtual adapter, encrypt and forward them through the runtime tunnel, then write received raw IPv4 packets back to Wintun. This requires `wintun.dll`, an existing adapter, and usually an Administrator terminal; without that environment it reports the Wintun open status and keeps the rest of the runtime observable.
`LocalAreaInterconnection.Native.Cli.exe room-runtime-run --inject-target 127.0.0.1:<port>` injects decrypted `runtime-udp-forward` payloads into a specific local UDP port; use `--inject-self-probe true` for a built-in loopback receiver.
`LocalAreaInterconnection.Native.Cli.exe room-runtime-run --duration-ms 0 --stop-file runtime.stop` keeps the runtime alive until that file exists. Add `--heartbeat-interval-ms` for periodic tunnel heartbeats and `--snapshot-interval-ms` with `--snapshot-out` for status refresh files.

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
The build script also copies the Rust native CLI to `LocalAreaInterconnection.Native.Cli.exe`; use it for the newest native adapter/tunnel/UDP forwarding experiments.
It can create/decode/join rooms, copy the generated invite and local virtual IP, run adapter/firewall/network diagnostics, and export a diagnostic bundle.
The right-side room details panel summarizes the room, virtual subnet, member/IP, connection checks, broadcast/game-traffic state, and next suggested action.
The packet observation file field lets the desktop shell append UDP/TCP/broadcast test observations and reuse that same file in network diagnostics and diagnostic export.
When a packet observation file is selected, UDP/TCP/broadcast test buttons also refresh the network diagnostics and room details after the test finishes.

Diagnostic export:

- Rust `lai-cli diagnostic-export` writes a read-only JSON bundle with environment metadata, adapter scan/diagnosis, firewall scan/diagnosis, ping-derived tunnel observation, packet observation summary, packet I/O plan/probe status, and the combined `network-observe` report.
- The bundle may contain local adapter and Windows Firewall configuration. Review it before sharing publicly.

