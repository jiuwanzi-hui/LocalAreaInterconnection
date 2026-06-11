# Architecture

## Current Scope

The final product should run as a Windows client with a native networking core. The current repository is now centered on one implementation layer:

- `native/`: Rust native core scaffold for the future Windows client.

The implementation does not install virtual adapters, open tunnels, modify Windows Firewall, or forward real packets yet.

Implemented areas:

- Room creation and summary.
- Room member lifecycle state modeling for host, peers, virtual IPs, online/left state, connection path, latency, and room close transitions.
- Virtual subnet selection.
- Invite encoding, decoding, and verification.
- Broadcast forwarding policy decisions.
- Game profile modeling.
- Game network plan generation.
- Windows Firewall dry-run command generation.
- Windows Firewall rule diagnostics from observed rule data.
- Network experiment observation modeling for adapter, tunnel, P2P, broadcast, and game traffic state.
- Runtime observation conversion for future tunnel service snapshots and packet capture summaries.
- Windows adapter `netsh` output parsing into adapter observations.
- Packet observation line parsing for UDP/TCP broadcast and game traffic diagnostics.
- Windows ping output parsing and Windows ping-test collection for tunnel/P2P latency and loss diagnostics.
- Read-only diagnostic bundle export in Rust core/CLI and the Windows test CLI, combining adapter, firewall, ping, packet, and network observation data.
- Diagnosis result prioritization.
- Rust CLI for room creation, invite decoding, diagnosis reports, game plans, firewall dry-run plans, firewall diagnostics previews, network observation reports, and diagnostic bundle export.

## Target Runtime Layers

```text
Windows desktop client
  -> UI shell
  -> Rust native core
      -> room lifecycle
      -> invite handling
      -> virtual adapter service
      -> tunnel service
      -> broadcast proxy
      -> diagnostics collector
```

## Current Rust Layers

```text
lai-cli
  -> lai-core
      -> room
      -> room lifecycle
      -> invite
      -> ip/subnet
      -> broadcast policy
      -> game profile
      -> game network plan
      -> firewall dry-run plan
      -> firewall diagnostics
      -> network observation
      -> runtime observation conversion
      -> diagnostic export
      -> Windows adapter parser
      -> packet observation parser
      -> Windows ping parser
      -> diagnostics
```

## Module Boundaries

### Rust Native Core

Location: `native/crates/lai-core`

Purpose:

- Become the efficient runtime core for the Windows client.
- Own networking-sensitive logic and future OS integration boundaries.
- Keep OS-facing plans explicit before adding side effects.

### Rust CLI

Location: `native/crates/lai-cli`

Purpose:

- Exercise the room and invite flow before a desktop UI exists.
- Emit local room session summaries for desktop room-detail integration before a real coordination service exists.
- Provide diagnostics output for future experiments.
- Render game network plans and Windows Firewall dry-run plans.
- Preview firewall diagnostics from expected rules and observed rule summaries.
- Evaluate network experiment observations before real packet capture and tunnel services are integrated.
- Export a JSON diagnostic bundle for M1/M3 experiments and failure reports through the Rust core diagnostic export schema.
- Avoid hidden system changes while the network PoC is still being designed.

### Future Native Services

Native services should sit behind narrow interfaces:

- Virtual adapter detection and configuration.
- UDP tunnel transport.
- Broadcast packet capture and replay.
- NAT candidate gathering.
- Windows firewall inspection.
- Windows firewall rule application, after dry-run plans are validated.

The current core modules should remain usable from a future Tauri or native Windows UI shell.

## Security Notes

- Invites contain a random join token.
- Invite payloads are signed with the room key in the prototype.
- The current invite format is not encrypted; do not treat it as a final production format.
- Real tunnel encryption is not implemented yet.

## Next Engineering Step

The network experiment observation, runtime observation conversion, and diagnostic export boundaries now exist at the core model level. Windows adapter `netsh` output, firewall `netsh` output, packet observation files, ping observations, future tunnel service snapshots, and future packet capture summaries can now feed them. The next concrete step is to connect the remaining real collection code:

- Replace ping-derived tunnel observations with tunnel service status once the transport service exists.
- Replace test-generated packet observation files with real broadcast/game packet capture summaries.
- Replace text-based adapter parsing with Windows API collection when the native service layer is ready.
- Feed collected records into `network_observation` instead of manually supplied CLI samples.
