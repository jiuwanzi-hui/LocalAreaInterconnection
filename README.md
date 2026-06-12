# LocalAreaInterconnection

LocalAreaInterconnection is a Windows virtual LAN tool for LAN-only PC games.

It helps players in different places create an encrypted virtual LAN, so games that only support local network discovery or direct LAN IP joining can connect over the internet.

## Download

Prebuilt Windows executables will be published on the GitHub Releases page:

[Download from Releases](https://github.com/jiuwanzi-hui/LocalAreaInterconnection/releases)

The repository stores source code only. Release `.exe` files are built separately and uploaded to Releases.

## Features

- Create or join a virtual LAN room.
- Copy room invite codes and virtual IP addresses.
- Show room members and their virtual IPs.
- Use encrypted UDP tunnels between peers.
- Exchange P2P/NAT traversal offers through local or HTTP coordination.
- Forward UDP broadcast traffic used by LAN game discovery.
- Support IPv4 UDP, broadcast, and the core TCP packet path.
- Probe and report Wintun virtual adapter readiness.
- Diagnose adapter, firewall, ping, broadcast, and game traffic issues.
- Export a read-only diagnostic bundle for troubleshooting.
- Switch the desktop UI language between English and Chinese inside the app.

## Current Status

The project is still under active MVP development.

Implemented code already includes the Windows desktop test shell, Rust native CLI, room and invite models, diagnostics, encrypted tunnel envelope, NAT/P2P bootstrap, local JSON coordination store, lightweight HTTP coordination service, UDP forwarding, raw IPv4 UDP/TCP packet handling, and Wintun runtime probes.

Recent native builds also include a small STUN-like UDP observer for endpoint discovery and diagnostic export support for `room-runtime-run` snapshot files. This lets a troubleshooting bundle include runtime packet I/O evidence such as raw virtual packet counts, Wintun send/receive summaries, and packet observation lines captured during a run.

The desktop test shell can start and stop a controlled native runtime. Runtime snapshots and packet observation files are written under the user's application data folder, and the diagnostic export button automatically includes the latest runtime snapshot when one exists.

Real cross-network gameplay still needs validation on two Windows machines with:

- Administrator permission.
- `wintun.dll`.
- A created and openable Wintun adapter.
- Real game traffic on different NAT networks.

## Which EXE To Run

For normal use, run:

```text
LocalAreaInterconnection.exe
```

Developer and diagnostic builds may also produce:

```text
LocalAreaInterconnection.Cli.exe
LocalAreaInterconnection.Native.Cli.exe
```

Those CLI executables are mainly for testing, diagnostics, and native networking experiments.

## Build From Source

Requirements:

- Windows.
- Rust toolchain with Cargo.
- .NET SDK or the build tools used by the Windows test shell.

Run Rust tests:

```powershell
cd native
cargo test
```

Useful native diagnostics:

```powershell
.\dist\LocalAreaInterconnection.Native.Cli.exe stun-like-serve --bind 0.0.0.0:39120
.\dist\LocalAreaInterconnection.Native.Cli.exe nat-candidates --stun-server <server-ip>:39120
.\dist\LocalAreaInterconnection.Native.Cli.exe diagnostic-export --out diag.json --runtime-snapshot runtime.json
```

Build the latest Windows test shell:

```powershell
.\scripts\build-windows-test-shell.ps1
```

Or double-click:

```text
build-latest-exe.bat
```

Build and launch:

```powershell
.\scripts\run-windows-test-shell.ps1
```

Or double-click:

```text
build-and-run-exe.bat
```

## Development Notes

- `native/` contains the Rust native core and CLI.
- `windows-cli/` contains the current Windows desktop test shell source.
- `scripts/` contains local build helpers.
- Compiled outputs under `dist/` and `native/target/` are not committed.
- Planning and progress documents are local development references and are not required for release downloads.

## License

This project currently uses the license file included in the repository.
