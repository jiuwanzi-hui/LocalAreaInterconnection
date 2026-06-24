use base64::engine::general_purpose::STANDARD_NO_PAD;
use base64::Engine as _;
use serde_json::Value;
use std::fs;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, UdpSocket};
use std::process::Command;
use std::process::Stdio;
use std::thread::JoinHandle;
use std::time::Duration;

fn run_cli(args: &[&str]) -> Value {
    let output = Command::new(env!("CARGO_BIN_EXE_lai-cli"))
        .args(args)
        .output()
        .expect("run lai-cli");
    assert!(
        output.status.success(),
        "lai-cli failed\nstatus: {}\nstdout: {}\nstderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("valid json stdout")
}

fn test_internet_checksum(bytes: &[u8]) -> u16 {
    let mut sum = 0u32;
    let mut chunks = bytes.chunks_exact(2);
    for chunk in &mut chunks {
        sum += u16::from_be_bytes([chunk[0], chunk[1]]) as u32;
    }
    if let Some(byte) = chunks.remainder().first() {
        sum += (*byte as u32) << 8;
    }
    while (sum >> 16) != 0 {
        sum = (sum & 0xffff) + (sum >> 16);
    }
    !(sum as u16)
}

fn test_icmp_echo_request(source_ip: [u8; 4], destination_ip: [u8; 4]) -> Vec<u8> {
    let total_len = 33usize;
    let mut bytes = vec![0u8; total_len];
    bytes[0] = 0x45;
    bytes[2..4].copy_from_slice(&(total_len as u16).to_be_bytes());
    bytes[8] = 64;
    bytes[9] = 1;
    bytes[12..16].copy_from_slice(&source_ip);
    bytes[16..20].copy_from_slice(&destination_ip);
    let ipv4_checksum = test_internet_checksum(&bytes[..20]);
    bytes[10..12].copy_from_slice(&ipv4_checksum.to_be_bytes());
    bytes[20] = 8;
    bytes[24..26].copy_from_slice(&0x1234u16.to_be_bytes());
    bytes[26..28].copy_from_slice(&7u16.to_be_bytes());
    bytes[28..].copy_from_slice(b"hello");
    let icmp_checksum = test_internet_checksum(&bytes[20..]);
    bytes[22..24].copy_from_slice(&icmp_checksum.to_be_bytes());
    bytes
}

fn spawn_fake_standard_stun(response_port_delta: u16) -> (SocketAddr, JoinHandle<()>) {
    let server = UdpSocket::bind("127.0.0.1:0").expect("bind fake stun server");
    let server_addr = server.local_addr().unwrap();
    let handle = std::thread::spawn(move || {
        let mut buffer = [0u8; 1500];
        let (received, peer) = server.recv_from(&mut buffer).expect("receive stun request");
        assert!(received >= 20, "short STUN request");
        let transaction_id = &buffer[8..20];
        let magic_cookie = 0x2112A442u32;
        let cookie_bytes = magic_cookie.to_be_bytes();
        let peer_ip = match peer.ip() {
            std::net::IpAddr::V4(ip) => ip.octets(),
            std::net::IpAddr::V6(_) => panic!("expected IPv4 peer"),
        };
        let mapped_port = peer.port().saturating_add(response_port_delta);
        let xport = mapped_port ^ ((magic_cookie >> 16) as u16);

        let mut response = Vec::new();
        response.extend_from_slice(&0x0101u16.to_be_bytes());
        response.extend_from_slice(&12u16.to_be_bytes());
        response.extend_from_slice(&magic_cookie.to_be_bytes());
        response.extend_from_slice(transaction_id);
        response.extend_from_slice(&0x0020u16.to_be_bytes());
        response.extend_from_slice(&8u16.to_be_bytes());
        response.push(0);
        response.push(0x01);
        response.extend_from_slice(&xport.to_be_bytes());
        response.push(peer_ip[0] ^ cookie_bytes[0]);
        response.push(peer_ip[1] ^ cookie_bytes[1]);
        response.push(peer_ip[2] ^ cookie_bytes[2]);
        response.push(peer_ip[3] ^ cookie_bytes[3]);
        server.send_to(&response, peer).expect("send stun response");
    });
    (server_addr, handle)
}

#[test]
fn no_args_prints_help_successfully() {
    let output = Command::new(env!("CARGO_BIN_EXE_lai-cli"))
        .output()
        .expect("run lai-cli");

    assert!(
        output.status.success(),
        "lai-cli --help fallback failed\nstatus: {}\nstdout: {}\nstderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Usage:"));
    assert!(stdout.contains("diagnostic-export"));
}

#[test]
fn init_outputs_room_and_invite() {
    let value = run_cli(&["init", "--room-name", "Friday LAN", "--host", "Alice"]);
    let room = &value[0];
    let invite = value[1].as_str().expect("invite string");

    assert_eq!(room["version"], 1);
    assert_eq!(room["room_name"], "Friday LAN");
    assert_eq!(room["host_name"], "Alice");
    assert!(room["virtual_subnet"].as_str().unwrap().ends_with("/24"));
    assert!(invite.contains('.'));
}

#[test]
fn network_observe_reports_packet_statuses() {
    let value = run_cli(&[
        "network-observe",
        "--adapter-name",
        "LocalAreaInterconnection",
        "--expected-ip",
        "10.77.12.2",
        "--assigned-ip",
        "10.77.12.2",
        "--subnet",
        "10.77.12.0/24",
        "--tunnel-state",
        "connected",
        "--connected-peers",
        "1",
        "--expected-peers",
        "1",
        "--packets",
        "udp:10.77.12.2:10.77.12.255:39078:broadcast:outbound:8,udp:10.77.12.2:10.77.12.1:39077:unicast:outbound:8",
        "--broadcast-ports",
        "39078",
        "--game-ports",
        "39077",
    ]);

    assert_eq!(value["status"], "ok");
    assert_eq!(value["diagnostic_snapshot"]["virtual_adapter"], "ok");
    assert_eq!(value["diagnostic_snapshot"]["broadcast"], "seen");
    assert_eq!(value["diagnostic_snapshot"]["game_traffic"], "seen");
}

#[test]
fn network_observe_reports_connection_path_relay() {
    let value = run_cli(&[
        "network-observe",
        "--adapter-name",
        "LocalAreaInterconnection",
        "--expected-ip",
        "10.77.12.2",
        "--assigned-ip",
        "10.77.12.2",
        "--subnet",
        "10.77.12.0/24",
        "--tunnel-state",
        "connected",
        "--connected-peers",
        "1",
        "--expected-peers",
        "1",
        "--connection-path",
        "relay",
        "--packets",
        "udp:10.77.12.2:10.77.12.255:27015:broadcast:outbound:8,udp:10.77.12.2:10.77.12.1:27015:unicast:outbound:8",
        "--broadcast-ports",
        "27015",
        "--game-ports",
        "27015",
    ]);

    assert_eq!(value["status"], "needs-attention");
    assert!(value["checks"]
        .as_array()
        .unwrap()
        .iter()
        .any(|check| check["key"] == "connection-path" && check["status"] == "relay"));
}

#[test]
fn relay_udp_loopback_forwards_encrypted_peer_payload() {
    let value = run_cli(&[
        "relay-udp-loopback-test",
        "--key",
        "room-key",
        "--room-id",
        "room_test",
        "--message",
        "relay hello",
    ]);

    assert_eq!(value["status"], "ok");
    assert_eq!(value["deliveredMessage"], "relay hello");
    assert_eq!(value["relayEvents"][0]["status"], "registered");
    assert_eq!(value["relayEvents"][1]["status"], "forwarded");
    assert_eq!(value["relayEvents"][1]["fromPeerId"], "peer_a");
    assert_eq!(value["relayEvents"][1]["toPeerId"], "peer_b");
    assert!(value["knownPeers"]
        .as_array()
        .unwrap()
        .iter()
        .any(|peer| peer["peerId"] == "peer_b"));
}

#[test]
fn network_observe_reports_room_route_present() {
    let route_path = std::env::temp_dir().join(format!(
        "lai-cli-network-observe-route-{}.txt",
        std::process::id()
    ));
    let route_arg = route_path.to_string_lossy().to_string();
    fs::write(
        &route_path,
        r#"
IPv4 Route Table
===========================================================================
Active Routes:
Network Destination        Netmask          Gateway       Interface  Metric
       10.77.12.0    255.255.255.0         On-link       10.77.12.2      5
"#,
    )
    .expect("write route output");

    let value = run_cli(&[
        "network-observe",
        "--adapter-name",
        "LocalAreaInterconnection",
        "--expected-ip",
        "10.77.12.2",
        "--assigned-ip",
        "10.77.12.2",
        "--subnet",
        "10.77.12.0/24",
        "--route-output",
        &route_arg,
        "--tunnel-state",
        "connected",
        "--connected-peers",
        "1",
        "--expected-peers",
        "1",
        "--packets",
        "udp:10.77.12.2:10.77.12.255:27015:broadcast:outbound:8,udp:10.77.12.2:10.77.12.1:27015:unicast:outbound:8",
        "--broadcast-ports",
        "27015",
        "--game-ports",
        "27015",
    ]);
    fs::remove_file(&route_path).ok();

    assert_eq!(value["status"], "ok");
    assert_eq!(value["routeSource"]["source"], "route-file");
    assert_eq!(value["routeCount"], 1);
    assert!(value["checks"]
        .as_array()
        .unwrap()
        .iter()
        .any(|check| check["key"] == "route" && check["status"] == "ok"));
}

#[test]
fn network_observe_reads_runtime_snapshot_evidence() {
    let snapshot_path = std::env::temp_dir().join(format!(
        "lai-cli-network-observe-runtime-snapshot-{}.json",
        std::process::id()
    ));
    let snapshot_arg = snapshot_path.to_string_lossy().to_string();
    fs::write(
        &snapshot_path,
        serde_json::json!({
            "tunnelServiceSnapshot": {
                "service_running": true,
                "connected_peer_count": 1,
                "connection_path": "p2p",
                "average_latency_ms": 12,
                "packet_loss_percent": 0.0,
                "bytes_sent": 128,
                "bytes_received": 256,
                "last_error": null
            },
            "packetCaptureSummaries": [
                {
                    "protocol": "udp",
                    "source_ip": "10.77.12.2",
                    "destination_ip": "10.77.12.255",
                    "destination_port": 27015,
                    "direction": "outbound",
                    "broadcast": true,
                    "packet_count": 1,
                    "bytes": 8
                },
                {
                    "protocol": "udp",
                    "source_ip": "10.77.12.2",
                    "destination_ip": "10.77.12.3",
                    "destination_port": 27015,
                    "direction": "outbound",
                    "broadcast": false,
                    "packet_count": 1,
                    "bytes": 8
                }
            ],
            "runtimePeerSummaries": [{
                "peerId": "peer_b",
                "virtualIp": "10.77.12.3",
                "selectedPath": "p2p",
                "connectionPathStatus": "observed",
                "bootstrapStatus": "ok",
                "connected": true,
                "pathKind": "direct",
                "latencyMs": 12,
                "lastSeenAtMs": 200,
                "lastSentAtMs": 220,
                "bytesSent": 32,
                "bytesReceived": 32,
                "directBytesSent": 32,
                "directBytesReceived": 32,
                "relayBytesSent": 0,
                "relayBytesReceived": 0,
                "unknownPathBytesSent": 0,
                "unknownPathBytesReceived": 0,
                "heartbeatPacketsSent": 2,
                "heartbeatAckPacketsReceived": 2,
                "heartbeatLossPercent": 0.0,
                "heartbeatLossWindowSize": 2,
                "heartbeatLossWindowPercent": 0.0,
                "heartbeatRttSampleCount": 2,
                "heartbeatRttJitterMs": 1.0,
                "forwardedPacketsSent": 1,
                "tunnelPacketsReceived": 1
            }]
        })
        .to_string(),
    )
    .expect("write runtime snapshot");

    let value = run_cli(&[
        "network-observe",
        "--adapter-name",
        "LocalAreaInterconnection",
        "--expected-ip",
        "10.77.12.2",
        "--assigned-ip",
        "10.77.12.2",
        "--subnet",
        "10.77.12.0/24",
        "--runtime-snapshot",
        &snapshot_arg,
        "--broadcast-ports",
        "27015",
        "--game-ports",
        "27015",
    ]);
    fs::remove_file(&snapshot_path).ok();

    assert_eq!(value["runtimeSnapshotSource"]["loaded"], true);
    assert_eq!(value["diagnostic_snapshot"]["tunnel"], "ok");
    assert_eq!(value["diagnostic_snapshot"]["p2p"], "ok");
    assert_eq!(value["diagnostic_snapshot"]["broadcast"], "seen");
    assert_eq!(value["diagnostic_snapshot"]["game_traffic"], "seen");
    assert!(value["checks"]
        .as_array()
        .unwrap()
        .iter()
        .any(|check| { check["key"] == "runtime-peer:peer_b" && check["status"] == "ok" }));
}

#[test]
fn room_summary_outputs_session_members() {
    let value = run_cli(&[
        "room-summary",
        "--room-name",
        "Friday LAN",
        "--host",
        "Alice",
        "--peer",
        "Bob",
        "--peer",
        "Carol",
    ]);

    assert_eq!(value["session"]["room_name"], "Friday LAN");
    assert_eq!(value["summary"]["member_count"], 3);
    assert_eq!(value["summary"]["online_count"], 3);
    assert_eq!(value["session"]["members"][0]["role"], "Host");
    assert_eq!(value["session"]["members"][1]["display_name"], "Bob");
}

#[test]
fn room_runtime_plan_outputs_tunnel_capture_and_forwarding_contract() {
    let value = run_cli(&[
        "room-runtime-plan",
        "--room-id",
        "room_test",
        "--peer-id",
        "peer_a",
        "--virtual-ip",
        "10.77.12.2",
        "--bind",
        "0.0.0.0:39090",
        "--peer",
        "peer_b,10.77.12.3,127.0.0.1:39091",
        "--game-ports",
        "27015",
        "--broadcast-ports",
        "27015,39078",
    ]);

    assert_eq!(value["room_id"], "room_test");
    assert_eq!(value["local_peer_id"], "peer_a");
    assert_eq!(value["tunnel"]["peer_count"], 1);
    assert_eq!(value["capture_ports"].as_array().unwrap().len(), 2);
    assert_eq!(value["udp_forwarders"].as_array().unwrap().len(), 2);
    assert_eq!(value["warnings"].as_array().unwrap().len(), 0);
}

#[test]
fn room_runtime_plan_uses_nat_bootstrap_selected_peer_endpoint() {
    let path = std::env::temp_dir().join(format!(
        "lai-cli-nat-bootstrap-result-{}.json",
        std::process::id()
    ));
    let path_string = path.display().to_string();
    fs::write(
        &path,
        serde_json::json!({
            "status": "ok",
            "selectedPeer": {
                "endpoint": "127.0.0.1:39091",
                "responderPeerId": "peer_b",
                "observedEndpoint": "127.0.0.1:50000",
                "nonceMatched": true,
                "accepted": true,
                "latencyMs": 3
            }
        })
        .to_string(),
    )
    .unwrap();

    let value = run_cli(&[
        "room-runtime-plan",
        "--room-id",
        "room_test",
        "--peer-id",
        "peer_a",
        "--virtual-ip",
        "10.77.12.2",
        "--bind",
        "0.0.0.0:39090",
        "--nat-bootstrap-peer",
        &format!("peer_b,10.77.12.3,{path_string}"),
        "--broadcast-ports",
        "39078",
    ]);
    fs::remove_file(&path).ok();

    assert_eq!(value["tunnel"]["peer_count"], 1);
    assert_eq!(value["peers"][0]["peer_id"], "peer_b");
    assert_eq!(value["peers"][0]["virtual_ip"], "10.77.12.3");
    assert_eq!(value["peers"][0]["endpoint"], "127.0.0.1:39091");
    assert_eq!(
        value["udp_forwarders"][0]["forward_to_peers"][0],
        "127.0.0.1:39091"
    );
}

#[test]
fn game_profile_plan_reads_catalog_and_outputs_network_plan() {
    let path = std::env::temp_dir().join(format!(
        "lai-cli-game-profile-catalog-{}.json",
        std::process::id()
    ));
    let path_string = path.display().to_string();
    fs::write(
        &path,
        serde_json::json!({
            "profiles": [
                {
                    "game_name": "Example Game",
                    "steam_app_id": "123456",
                    "discovery": "udp_broadcast",
                    "ports": [27016, 27015, 27015],
                    "join_method": "lan_list_or_direct_ip",
                    "compatibility": "A",
                    "notes": "Allow private network firewall access."
                }
            ]
        })
        .to_string(),
    )
    .unwrap();

    let value = run_cli(&[
        "game-profile-plan",
        "--catalog",
        &path_string,
        "--game-name",
        "example game",
        "--subnet",
        "10.77.12.0/24",
        "--host-ip",
        "10.77.12.1",
        "--local-ip",
        "10.77.12.2",
    ]);
    fs::remove_file(&path).ok();

    assert_eq!(value["status"], "ok");
    assert_eq!(value["matched_by"], "game_name");
    assert_eq!(value["profile"]["game_name"], "Example Game");
    assert_eq!(value["profile"]["discovery"], "udp_broadcast");
    assert_eq!(value["profile"]["compatibility"], "A");
    assert_eq!(value["plan"]["game_name"], "Example Game");
    assert_eq!(value["plan"]["broadcast"]["enabled"], true);
    assert_eq!(value["plan"]["firewall_rules"].as_array().unwrap().len(), 4);
    assert_eq!(value["plan"]["host_ip"], "10.77.12.1");
    assert_eq!(value["plan"]["local_ip"], "10.77.12.2");
}

#[test]
fn firewall_apply_without_confirmation_returns_preview() {
    let value = run_cli(&[
        "firewall-apply",
        "--game-name",
        "Example Game",
        "--subnet",
        "10.77.12.0/24",
        "--ports",
        "27015",
    ]);

    assert_eq!(value["status"], "needs-confirmation");
    assert_eq!(value["confirmed"], false);
    assert_eq!(value["requiresElevation"], true);
    assert_eq!(value["executionPreview"]["can_execute_now"], false);
    assert_eq!(value["commandResults"].as_array().unwrap().len(), 0);
}

#[test]
fn game_profile_list_filters_catalog_summaries() {
    let path = std::env::temp_dir().join(format!(
        "lai-cli-game-profile-list-{}.json",
        std::process::id()
    ));
    let path_string = path.display().to_string();
    fs::write(
        &path,
        serde_json::json!({
            "profiles": [
                {
                    "game_name": "Zeta Direct",
                    "steam_app_id": "200",
                    "discovery": "direct_ip",
                    "ports": [7777, 7777],
                    "compatibility": "B"
                },
                {
                    "game_name": "Alpha Broadcast",
                    "steam_app_id": "100",
                    "discovery": "udp_broadcast",
                    "ports": [27016, 27015],
                    "compatibility": "A"
                }
            ]
        })
        .to_string(),
    )
    .unwrap();

    let all = run_cli(&["game-profile-list", "--catalog", &path_string]);
    let filtered = run_cli(&[
        "game-profile-list",
        "--catalog",
        &path_string,
        "--query",
        "direct",
    ]);
    fs::remove_file(&path).ok();

    assert_eq!(all["status"], "ok");
    assert_eq!(all["total_count"], 2);
    assert_eq!(all["matched_count"], 2);
    assert_eq!(all["profiles"][0]["game_name"], "Alpha Broadcast");
    assert_eq!(all["profiles"][0]["discovery"], "udp_broadcast");
    assert_eq!(all["profiles"][0]["ports"].as_array().unwrap().len(), 2);

    assert_eq!(filtered["matched_count"], 1);
    assert_eq!(filtered["profiles"][0]["game_name"], "Zeta Direct");
    assert_eq!(filtered["profiles"][0]["port_count"], 1);
    assert_eq!(filtered["profiles"][0]["ports"][0], 7777);
}

#[test]
fn room_runtime_run_can_bootstrap_nat_remote_peer_before_starting() {
    let probe = UdpSocket::bind("127.0.0.1:0").expect("free udp port");
    let listener_addr = probe.local_addr().unwrap();
    drop(probe);
    let snapshot_path = std::env::temp_dir().join(format!(
        "lai-cli-runtime-bootstrap-snapshot-{}.json",
        std::process::id()
    ));
    let snapshot_path_string = snapshot_path.display().to_string();
    fs::remove_file(&snapshot_path).ok();

    let listener = Command::new(env!("CARGO_BIN_EXE_lai-cli"))
        .args([
            "p2p-handshake-listen",
            "--bind",
            &listener_addr.to_string(),
            "--key",
            "test-room-key",
            "--responder-peer-id",
            "peer_b",
            "--max-packets",
            "1",
            "--timeout-ms",
            "2000",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn p2p listener");
    std::thread::sleep(Duration::from_millis(80));

    let remote_offer = serde_json::json!({
        "schema_version": 1,
        "room_id": "room_test",
        "peer_id": "peer_b",
        "nonce": "peer_b-runtime-offer",
        "created_at_ms": 1,
        "candidates": [{
            "candidate_type": "host",
            "transport": "udp",
            "endpoint": listener_addr.to_string(),
            "priority": 100,
            "source": "test-listener"
        }]
    });
    let remote_offer = serde_json::to_string(&remote_offer).unwrap();
    let value = run_cli(&[
        "room-runtime-run",
        "--room-id",
        "room_test",
        "--peer-id",
        "peer_a",
        "--virtual-ip",
        "10.77.12.2",
        "--bind",
        "127.0.0.1:0",
        "--key",
        "test-room-key",
        "--duration-ms",
        "50",
        "--snapshot-out",
        &snapshot_path_string,
        "--peer-timeout-ms",
        "0",
        "--nat-bootstrap-remote-peer",
        &format!("peer_b,10.77.12.3,{remote_offer}"),
        "--nat-bootstrap-attempts",
        "1",
        "--nat-bootstrap-interval-ms",
        "0",
        "--nat-bootstrap-timeout-ms",
        "2000",
    ]);
    let listener_output = listener.wait_with_output().expect("listener exits");

    assert_eq!(value["status"], "ok");
    assert_eq!(value["natBootstrapSocketReused"], true);
    assert_eq!(value["plan"]["tunnel"]["peer_count"], 1);
    assert_eq!(value["plan"]["peers"][0]["peer_id"], "peer_b");
    assert_eq!(
        value["plan"]["peers"][0]["endpoint"],
        listener_addr.to_string()
    );
    assert_eq!(value["natBootstrapResults"][0]["status"], "ok");
    assert!(value["natBootstrapResults"][0]["localEndpoint"].is_string());
    assert_eq!(
        value["natBootstrapResults"][0]["localEndpoint"],
        value["actualTunnelEndpoint"]
    );
    assert_eq!(
        value["natBootstrapResults"][0]["selectedPeer"]["responderPeerId"],
        "peer_b"
    );
    assert_eq!(
        value["natBootstrapResults"][0]["selectedPeer"]["handshakeRole"],
        "received-ack"
    );
    assert_eq!(
        value["natBootstrapResults"][0]["selectedPeer"]["confirmedByAck"],
        true
    );
    assert_eq!(
        value["connectionPathReports"][0]["report"]["selected_path"],
        "p2p"
    );
    assert!(value["connectionPathReports"][0]["localEndpoint"].is_string());
    assert_eq!(
        value["connectionPathReports"][0]["selectedPeerEndpoint"],
        listener_addr.to_string()
    );
    assert_eq!(
        value["connectionPathReports"][0]["handshakeRole"],
        "received-ack"
    );
    assert_eq!(value["connectionPathReports"][0]["confirmedByAck"], true);
    assert_eq!(value["runtimePeerSummaries"][0]["peerId"], "peer_b");
    assert_eq!(value["runtimePeerSummaries"][0]["selectedPath"], "p2p");
    assert_eq!(
        value["runtimePeerSummaries"][0]["connectionPathStatus"],
        "p2p-candidate-ready"
    );
    assert_eq!(value["runtimePeerSummaries"][0]["bootstrapStatus"], "ok");
    assert!(value["runtimePeerSummaries"][0]["latencyMs"].is_number());
    assert!(value["runtimePeerSummaries"][0]["lastSentAtMs"].is_number());
    let snapshot: Value =
        serde_json::from_str(&fs::read_to_string(&snapshot_path).expect("runtime snapshot"))
            .unwrap();
    fs::remove_file(&snapshot_path).ok();
    assert_eq!(
        snapshot["connectionPathReports"][0]["report"]["selected_path"],
        "p2p"
    );
    assert_eq!(
        snapshot["connectionPathReports"][0]["handshakeRole"],
        "received-ack"
    );
    assert_eq!(snapshot["connectionPathReports"][0]["confirmedByAck"], true);
    assert_eq!(snapshot["runtimePeerSummaries"][0]["peerId"], "peer_b");
    assert_eq!(snapshot["runtimePeerSummaries"][0]["selectedPath"], "p2p");
    assert!(snapshot["runtimePeerSummaries"][0]["latencyMs"].is_number());
    assert!(
        listener_output.status.success(),
        "listener failed\nstatus: {}\nstdout: {}\nstderr: {}",
        listener_output.status,
        String::from_utf8_lossy(&listener_output.stdout),
        String::from_utf8_lossy(&listener_output.stderr)
    );
}

#[test]
fn room_runtime_run_falls_back_to_udp_relay_after_p2p_timeout() {
    let dead_listener = UdpSocket::bind("127.0.0.1:0").expect("reserve unreachable udp port");
    let dead_addr = dead_listener.local_addr().unwrap();
    drop(dead_listener);
    let relay_listener = UdpSocket::bind("127.0.0.1:0").expect("reserve relay udp port");
    let relay_addr = relay_listener.local_addr().unwrap();
    drop(relay_listener);
    let remote_offer = serde_json::json!({
        "schema_version": 1,
        "room_id": "room_test",
        "peer_id": "peer_b",
        "nonce": "peer_b-runtime-offer",
        "created_at_ms": 1,
        "candidates": [{
            "candidate_type": "host",
            "transport": "udp",
            "endpoint": dead_addr.to_string(),
            "priority": 100,
            "source": "test-unreachable"
        }]
    });
    let remote_offer = serde_json::to_string(&remote_offer).unwrap();

    let value = run_cli(&[
        "room-runtime-run",
        "--room-id",
        "room_test",
        "--peer-id",
        "peer_a",
        "--virtual-ip",
        "10.77.12.2",
        "--bind",
        "127.0.0.1:0",
        "--key",
        "test-room-key",
        "--duration-ms",
        "50",
        "--peer-timeout-ms",
        "0",
        "--nat-bootstrap-remote-peer",
        &format!("peer_b,10.77.12.3,{remote_offer}"),
        "--relay",
        &relay_addr.to_string(),
        "--nat-bootstrap-attempts",
        "1",
        "--nat-bootstrap-interval-ms",
        "0",
        "--nat-bootstrap-timeout-ms",
        "100",
    ]);

    assert_eq!(value["status"], "ok");
    assert_eq!(
        value["natBootstrapResults"][0]["status"],
        "handshake-timeout"
    );
    assert_eq!(value["plan"]["peers"][0]["peer_id"], "peer_b");
    assert_eq!(
        value["plan"]["peers"][0]["endpoint"],
        relay_addr.to_string()
    );
    assert_eq!(value["plan"]["peers"][0]["connection_path"], "relay");
    assert_eq!(
        value["plan"]["peers"][0]["direct_endpoint"],
        dead_addr.to_string()
    );
    assert_eq!(
        value["plan"]["peers"][0]["fallback_endpoint"],
        relay_addr.to_string()
    );
    assert_eq!(
        value["connectionPathReports"][0]["report"]["selected_path"],
        "relay"
    );
    assert_eq!(
        value["connectionPathReports"][0]["report"]["relay_fallback"]["selected_relay_endpoints"]
            [0],
        relay_addr.to_string()
    );
    assert!(value["heartbeatPackets"]
        .as_array()
        .unwrap()
        .iter()
        .any(|packet| packet["connectionPath"] == "relay"));
    assert!(value["heartbeatPackets"]
        .as_array()
        .unwrap()
        .iter()
        .any(|packet| {
            packet["connectionPath"] == "direct" && packet["target"] == dead_addr.to_string()
        }));
    assert!(
        value["runtimePeerSummaries"][0]["selectedPath"] == "p2p"
            || value["runtimePeerSummaries"][0]["selectedPath"] == "relay"
    );
}

#[test]
fn room_runtime_run_falls_back_to_http_relay_candidate_after_p2p_timeout() {
    let dead_listener = UdpSocket::bind("127.0.0.1:0").expect("reserve unreachable udp port");
    let dead_addr = dead_listener.local_addr().unwrap();
    drop(dead_listener);
    let remote_offer = serde_json::json!({
        "schema_version": 1,
        "room_id": "room_test",
        "peer_id": "peer_b",
        "nonce": "peer_b-runtime-offer",
        "created_at_ms": 1,
        "candidates": [{
            "candidate_type": "host",
            "transport": "udp",
            "endpoint": dead_addr.to_string(),
            "priority": 100,
            "source": "test-unreachable"
        }]
    });
    let remote_offer = serde_json::to_string(&remote_offer).unwrap();

    let value = run_cli(&[
        "room-runtime-run",
        "--room-id",
        "room_test",
        "--peer-id",
        "peer_a",
        "--virtual-ip",
        "10.77.12.2",
        "--bind",
        "127.0.0.1:0",
        "--key",
        "test-room-key",
        "--duration-ms",
        "50",
        "--peer-timeout-ms",
        "0",
        "--nat-bootstrap-remote-peer",
        &format!("peer_b,10.77.12.3,{remote_offer}"),
        "--relay",
        "http://127.0.0.1",
        "--nat-bootstrap-attempts",
        "1",
        "--nat-bootstrap-interval-ms",
        "0",
        "--nat-bootstrap-timeout-ms",
        "100",
    ]);

    assert_eq!(value["status"], "ok");
    assert_eq!(
        value["natBootstrapResults"][0]["status"],
        "handshake-timeout"
    );
    assert_eq!(value["plan"]["peers"][0]["endpoint"], "http://127.0.0.1");
    assert_eq!(value["plan"]["peers"][0]["connection_path"], "relay");
    assert_eq!(
        value["plan"]["peers"][0]["direct_endpoint"],
        dead_addr.to_string()
    );
    assert_eq!(
        value["plan"]["peers"][0]["fallback_endpoint"],
        "http://127.0.0.1"
    );
    assert_eq!(
        value["connectionPathReports"][0]["report"]["selected_path"],
        "relay"
    );
    assert_eq!(
        value["connectionPathReports"][0]["report"]["relay_fallback"]["selected_relay_endpoints"]
            [0],
        "http://127.0.0.1"
    );
    assert_eq!(value["runtimePeerSummaries"][0]["pathKind"], "relay");
}

#[test]
fn room_runtime_run_activates_relay_fallback_after_direct_peer_timeout() {
    let relay_listener = UdpSocket::bind("127.0.0.1:0").expect("reserve relay udp port");
    let relay_addr = relay_listener.local_addr().unwrap();
    drop(relay_listener);
    let remote_offer = serde_json::json!({
        "schema_version": 1,
        "room_id": "room_test",
        "peer_id": "peer_b",
        "nonce": "nonce-b",
        "created_at_ms": 1,
        "candidates": [
            {
                "candidate_type": "host",
                "transport": "udp",
                "endpoint": "127.0.0.1:9",
                "priority": 100,
                "source": "test-direct"
            },
            {
                "candidate_type": "relay",
                "transport": "udp",
                "endpoint": relay_addr.to_string(),
                "priority": 10,
                "source": "test-relay"
            }
        ]
    });
    let local_offer = serde_json::json!({
        "schema_version": 1,
        "room_id": "room_test",
        "peer_id": "peer_a",
        "nonce": "nonce-a",
        "created_at_ms": 1,
        "candidates": [{
            "candidate_type": "host",
            "transport": "udp",
            "endpoint": "127.0.0.1:39090",
            "priority": 100,
            "source": "test-local"
        }]
    });
    let bootstrap_result = serde_json::json!({
        "status": "ok",
        "localEndpoint": "127.0.0.1:39090",
        "localOffer": local_offer,
        "remoteOffer": remote_offer,
        "selectedPeer": {
            "endpoint": "127.0.0.1:9",
            "responderPeerId": "peer_b",
            "observedEndpoint": "127.0.0.1:9",
            "nonceMatched": true,
            "accepted": true,
            "handshakeRole": "received-ack",
            "confirmedByAck": true,
            "latencyMs": 1
        }
    });
    let bootstrap_result = serde_json::to_string(&bootstrap_result).unwrap();

    let value = run_cli(&[
        "room-runtime-run",
        "--room-id",
        "room_test",
        "--peer-id",
        "peer_a",
        "--virtual-ip",
        "10.77.12.2",
        "--bind",
        "127.0.0.1:0",
        "--key",
        "test-room-key",
        "--duration-ms",
        "320",
        "--heartbeat-interval-ms",
        "50",
        "--peer-timeout-ms",
        "80",
        "--nat-bootstrap-peer",
        &format!("peer_b,10.77.12.3,{bootstrap_result}"),
    ]);

    assert_eq!(value["plan"]["peers"][0]["endpoint"], "127.0.0.1:9");
    assert_eq!(
        value["plan"]["peers"][0]["fallback_endpoint"],
        relay_addr.to_string()
    );
    assert_eq!(value["relayFallbackActive"], true);
    assert_eq!(value["relayFallbackEvents"][0]["status"], "activated");
    assert_eq!(value["status"], "degraded");
    assert!(value["tunnelServiceSnapshot"]["last_error"]
        .as_str()
        .is_some_and(|error| error.contains("No runtime tunnel packets")));
    assert!(value["heartbeatTargets"]
        .as_array()
        .unwrap()
        .iter()
        .any(|endpoint| endpoint.as_str() == Some(relay_addr.to_string().as_str())));
    assert!(value["heartbeatPackets"]
        .as_array()
        .unwrap()
        .iter()
        .any(|packet| packet["connectionPath"] == "relay"));
}

#[test]
fn room_runtime_run_does_not_restore_direct_from_self_probe() {
    let relay_listener = UdpSocket::bind("127.0.0.1:0").expect("reserve relay udp port");
    let relay_addr = relay_listener.local_addr().unwrap();
    drop(relay_listener);
    let remote_offer = serde_json::json!({
        "schema_version": 1,
        "room_id": "room_self_probe_fallback",
        "peer_id": "peer_b",
        "nonce": "nonce-b",
        "created_at_ms": 1,
        "candidates": [
            {
                "candidate_type": "host",
                "transport": "udp",
                "endpoint": "127.0.0.1:9",
                "priority": 100,
                "source": "test-direct"
            },
            {
                "candidate_type": "relay",
                "transport": "udp",
                "endpoint": relay_addr.to_string(),
                "priority": 10,
                "source": "test-relay"
            }
        ]
    });
    let local_offer = serde_json::json!({
        "schema_version": 1,
        "room_id": "room_self_probe_fallback",
        "peer_id": "peer_a",
        "nonce": "nonce-a",
        "created_at_ms": 1,
        "candidates": [{
            "candidate_type": "host",
            "transport": "udp",
            "endpoint": "127.0.0.1:39090",
            "priority": 100,
            "source": "test-local"
        }]
    });
    let bootstrap_result = serde_json::json!({
        "status": "ok",
        "localEndpoint": "127.0.0.1:39090",
        "localOffer": local_offer,
        "remoteOffer": remote_offer,
        "selectedPeer": {
            "endpoint": "127.0.0.1:9",
            "responderPeerId": "peer_b",
            "observedEndpoint": "127.0.0.1:9",
            "nonceMatched": true,
            "accepted": true,
            "handshakeRole": "received-ack",
            "confirmedByAck": true,
            "latencyMs": 1
        }
    });
    let bootstrap_result = serde_json::to_string(&bootstrap_result).unwrap();

    let value = run_cli(&[
        "room-runtime-run",
        "--room-id",
        "room_self_probe_fallback",
        "--peer-id",
        "peer_a",
        "--virtual-ip",
        "10.77.12.2",
        "--bind",
        "127.0.0.1:0",
        "--key",
        "test-room-key",
        "--duration-ms",
        "260",
        "--heartbeat-interval-ms",
        "50",
        "--peer-timeout-ms",
        "80",
        "--self-probe",
        "true",
        "--nat-bootstrap-peer",
        &format!("peer_b,10.77.12.3,{bootstrap_result}"),
    ]);

    assert_eq!(value["relayFallbackActive"], true);
    assert_eq!(
        value["tunnelServiceSnapshot"]["connected_peer_count"].as_u64(),
        Some(0)
    );
    assert!(value["relayFallbackEvents"]
        .as_array()
        .unwrap()
        .iter()
        .any(|event| event["status"] == "activated"));
    assert!(!value["relayFallbackEvents"]
        .as_array()
        .unwrap()
        .iter()
        .any(|event| event["status"] == "restored-direct"));
    assert!(value["heartbeatPackets"]
        .as_array()
        .unwrap()
        .iter()
        .any(|packet| packet["targetPeerId"] == "self-probe"));
    assert!(value["heartbeatAckPackets"]
        .as_array()
        .unwrap()
        .iter()
        .any(|packet| packet["peer"] == value["actualTunnelEndpoint"]));
}

#[test]
fn room_runtime_run_recovers_after_relay_fallback_receives_acks() {
    let probe = UdpSocket::bind("127.0.0.1:0").expect("reserve relay udp port");
    let relay_addr = probe.local_addr().unwrap();
    drop(probe);
    let room_id = "room_fallback_recover";
    let key = "test-room-key";
    let peer_a_out = std::env::temp_dir().join(format!(
        "lai-peer-a-fallback-recover-{}.json",
        std::process::id()
    ));
    let peer_b_out = std::env::temp_dir().join(format!(
        "lai-peer-b-fallback-recover-{}.json",
        std::process::id()
    ));
    fs::remove_file(&peer_a_out).ok();
    fs::remove_file(&peer_b_out).ok();
    let peer_a_out_string = peer_a_out.display().to_string();
    let peer_b_out_string = peer_b_out.display().to_string();

    let relay = Command::new(env!("CARGO_BIN_EXE_lai-cli"))
        .args([
            "relay-udp-server",
            "--bind",
            &relay_addr.to_string(),
            "--key",
            key,
            "--room-id",
            room_id,
            "--allowed-peer",
            "peer_a",
            "--allowed-peer",
            "peer_b",
            "--max-packets",
            "20",
            "--timeout-ms",
            "5000",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn relay server");
    std::thread::sleep(Duration::from_millis(80));

    let local_offer_a = serde_json::json!({
        "schema_version": 1,
        "room_id": room_id,
        "peer_id": "peer_a",
        "nonce": "nonce-a",
        "created_at_ms": 1,
        "candidates": [{
            "candidate_type": "host",
            "transport": "udp",
            "endpoint": "127.0.0.1:39090",
            "priority": 100,
            "source": "test-local"
        }]
    });
    let local_offer_b = serde_json::json!({
        "schema_version": 1,
        "room_id": room_id,
        "peer_id": "peer_b",
        "nonce": "nonce-b",
        "created_at_ms": 1,
        "candidates": [{
            "candidate_type": "host",
            "transport": "udp",
            "endpoint": "127.0.0.1:39091",
            "priority": 100,
            "source": "test-local"
        }]
    });
    let remote_offer_a = serde_json::json!({
        "schema_version": 1,
        "room_id": room_id,
        "peer_id": "peer_b",
        "nonce": "nonce-b",
        "created_at_ms": 1,
        "candidates": [
            {
                "candidate_type": "host",
                "transport": "udp",
                "endpoint": "127.0.0.1:9",
                "priority": 100,
                "source": "test-direct"
            },
            {
                "candidate_type": "relay",
                "transport": "udp",
                "endpoint": relay_addr.to_string(),
                "priority": 10,
                "source": "test-relay"
            }
        ]
    });
    let remote_offer_b = serde_json::json!({
        "schema_version": 1,
        "room_id": room_id,
        "peer_id": "peer_a",
        "nonce": "nonce-a",
        "created_at_ms": 1,
        "candidates": [
            {
                "candidate_type": "host",
                "transport": "udp",
                "endpoint": "127.0.0.1:9",
                "priority": 100,
                "source": "test-direct"
            },
            {
                "candidate_type": "relay",
                "transport": "udp",
                "endpoint": relay_addr.to_string(),
                "priority": 10,
                "source": "test-relay"
            }
        ]
    });
    let bootstrap_a = serde_json::json!({
        "status": "ok",
        "localEndpoint": "127.0.0.1:39090",
        "localOffer": local_offer_a,
        "remoteOffer": remote_offer_a,
        "selectedPeer": {
            "endpoint": "127.0.0.1:9",
            "responderPeerId": "peer_b",
            "observedEndpoint": "127.0.0.1:9",
            "nonceMatched": true,
            "accepted": true,
            "handshakeRole": "received-ack",
            "confirmedByAck": true,
            "latencyMs": 1
        }
    });
    let bootstrap_b = serde_json::json!({
        "status": "ok",
        "localEndpoint": "127.0.0.1:39091",
        "localOffer": local_offer_b,
        "remoteOffer": remote_offer_b,
        "selectedPeer": {
            "endpoint": "127.0.0.1:9",
            "responderPeerId": "peer_a",
            "observedEndpoint": "127.0.0.1:9",
            "nonceMatched": true,
            "accepted": true,
            "handshakeRole": "received-ack",
            "confirmedByAck": true,
            "latencyMs": 1
        }
    });
    let bootstrap_a = serde_json::to_string(&bootstrap_a).unwrap();
    let bootstrap_b = serde_json::to_string(&bootstrap_b).unwrap();

    let peer_a = Command::new(env!("CARGO_BIN_EXE_lai-cli"))
        .args([
            "room-runtime-run",
            "--room-id",
            room_id,
            "--peer-id",
            "peer_a",
            "--virtual-ip",
            "10.77.12.2",
            "--bind",
            "127.0.0.1:0",
            "--key",
            key,
            "--duration-ms",
            "650",
            "--heartbeat-interval-ms",
            "50",
            "--peer-timeout-ms",
            "200",
            "--nat-bootstrap-peer",
            &format!("peer_b,10.77.12.3,{bootstrap_a}"),
            "--snapshot-out",
            &peer_a_out_string,
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn peer a runtime");
    let peer_b = Command::new(env!("CARGO_BIN_EXE_lai-cli"))
        .args([
            "room-runtime-run",
            "--room-id",
            room_id,
            "--peer-id",
            "peer_b",
            "--virtual-ip",
            "10.77.12.3",
            "--bind",
            "127.0.0.1:0",
            "--key",
            key,
            "--duration-ms",
            "650",
            "--heartbeat-interval-ms",
            "50",
            "--peer-timeout-ms",
            "200",
            "--nat-bootstrap-peer",
            &format!("peer_a,10.77.12.2,{bootstrap_b}"),
            "--snapshot-out",
            &peer_b_out_string,
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn peer b runtime");

    let output_a = peer_a.wait_with_output().expect("peer a exits");
    let output_b = peer_b.wait_with_output().expect("peer b exits");
    let relay_output = relay.wait_with_output().expect("relay exits");
    fs::remove_file(&peer_a_out).ok();
    fs::remove_file(&peer_b_out).ok();

    assert!(
        output_a.status.success(),
        "peer a failed\nstatus: {}\nstdout: {}\nstderr: {}",
        output_a.status,
        String::from_utf8_lossy(&output_a.stdout),
        String::from_utf8_lossy(&output_a.stderr)
    );
    assert!(
        output_b.status.success(),
        "peer b failed\nstatus: {}\nstdout: {}\nstderr: {}",
        output_b.status,
        String::from_utf8_lossy(&output_b.stdout),
        String::from_utf8_lossy(&output_b.stderr)
    );
    assert!(
        relay_output.status.success(),
        "relay failed\nstatus: {}\nstdout: {}\nstderr: {}",
        relay_output.status,
        String::from_utf8_lossy(&relay_output.stdout),
        String::from_utf8_lossy(&relay_output.stderr)
    );
    let value_a: Value = serde_json::from_slice(&output_a.stdout).expect("peer a json");
    let value_b: Value = serde_json::from_slice(&output_b.stdout).expect("peer b json");
    let relay_value: Value = serde_json::from_slice(&relay_output.stdout).expect("relay json");

    for value in [&value_a, &value_b] {
        assert_eq!(value["relayFallbackActive"], true);
        assert_eq!(value["status"], "ok");
        assert_eq!(value["tunnelServiceSnapshot"]["last_error"], Value::Null);
        assert!(value["relayFallbackEvents"]
            .as_array()
            .unwrap()
            .iter()
            .any(|event| event["status"] == "activated"));
        assert!(value["relayFallbackEvents"]
            .as_array()
            .unwrap()
            .iter()
            .any(|event| event["status"] == "recovered"));
        assert!(
            value["heartbeatAckPackets"]
                .as_array()
                .unwrap()
                .iter()
                .any(|packet| packet["direction"] == "received"
                    && packet["connectionPath"] == "relay")
        );
        assert_eq!(value["runtimePeerSummaries"][0]["pathKind"], "relay");
        assert_ne!(
            value["runtimePeerSummaries"][0]["health"]["status"],
            "needs-attention"
        );
        assert!(
            value["runtimePeerSummaries"][0]["heartbeatLossWindowPercent"]
                .as_f64()
                .unwrap_or_default()
                < 50.0
        );
    }
    assert!(relay_value["forwardedPackets"].as_u64().unwrap_or_default() > 0);
}

#[test]
fn room_runtime_run_restores_direct_path_after_relay_fallback() {
    let direct_probe = UdpSocket::bind("127.0.0.1:0").expect("reserve delayed direct endpoint");
    let direct_addr = direct_probe.local_addr().unwrap();
    drop(direct_probe);
    let relay_probe = UdpSocket::bind("127.0.0.1:0").expect("reserve relay endpoint");
    let relay_addr = relay_probe.local_addr().unwrap();
    drop(relay_probe);
    let room_id = "room_direct_restore";
    let key = "test-room-key";
    let local_offer = serde_json::json!({
        "schema_version": 1,
        "room_id": room_id,
        "peer_id": "peer_a",
        "nonce": "nonce-a",
        "created_at_ms": 1,
        "candidates": [{
            "candidate_type": "host",
            "transport": "udp",
            "endpoint": "127.0.0.1:39090",
            "priority": 100,
            "source": "test-local"
        }]
    });
    let remote_offer = serde_json::json!({
        "schema_version": 1,
        "room_id": room_id,
        "peer_id": "peer_b",
        "nonce": "nonce-b",
        "created_at_ms": 1,
        "candidates": [
            {
                "candidate_type": "host",
                "transport": "udp",
                "endpoint": direct_addr.to_string(),
                "priority": 100,
                "source": "test-direct"
            },
            {
                "candidate_type": "relay",
                "transport": "udp",
                "endpoint": relay_addr.to_string(),
                "priority": 10,
                "source": "test-relay"
            }
        ]
    });
    let bootstrap_result = serde_json::json!({
        "status": "ok",
        "localEndpoint": "127.0.0.1:39090",
        "localOffer": local_offer,
        "remoteOffer": remote_offer,
        "selectedPeer": {
            "endpoint": direct_addr.to_string(),
            "responderPeerId": "peer_b",
            "observedEndpoint": direct_addr.to_string(),
            "nonceMatched": true,
            "accepted": true,
            "handshakeRole": "received-ack",
            "confirmedByAck": true,
            "latencyMs": 1
        }
    });
    let bootstrap_result = serde_json::to_string(&bootstrap_result).unwrap();

    let peer_a = Command::new(env!("CARGO_BIN_EXE_lai-cli"))
        .args([
            "room-runtime-run",
            "--room-id",
            room_id,
            "--peer-id",
            "peer_a",
            "--virtual-ip",
            "10.77.12.2",
            "--bind",
            "127.0.0.1:0",
            "--key",
            key,
            "--duration-ms",
            "900",
            "--heartbeat-interval-ms",
            "50",
            "--peer-timeout-ms",
            "180",
            "--nat-bootstrap-peer",
            &format!("peer_b,10.77.12.3,{bootstrap_result}"),
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn peer a runtime");

    std::thread::sleep(Duration::from_millis(320));
    let peer_b = Command::new(env!("CARGO_BIN_EXE_lai-cli"))
        .args([
            "room-runtime-run",
            "--room-id",
            room_id,
            "--peer-id",
            "peer_b",
            "--virtual-ip",
            "10.77.12.3",
            "--bind",
            &direct_addr.to_string(),
            "--key",
            key,
            "--duration-ms",
            "450",
            "--heartbeat-interval-ms",
            "50",
            "--peer-timeout-ms",
            "0",
            "--peer",
            "peer_a,10.77.12.2,127.0.0.1:9",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn delayed peer b runtime");

    let output_a = peer_a.wait_with_output().expect("peer a exits");
    let output_b = peer_b.wait_with_output().expect("peer b exits");
    assert!(
        output_a.status.success(),
        "peer a failed\nstatus: {}\nstdout: {}\nstderr: {}",
        output_a.status,
        String::from_utf8_lossy(&output_a.stdout),
        String::from_utf8_lossy(&output_a.stderr)
    );
    assert!(
        output_b.status.success(),
        "peer b failed\nstatus: {}\nstdout: {}\nstderr: {}",
        output_b.status,
        String::from_utf8_lossy(&output_b.stdout),
        String::from_utf8_lossy(&output_b.stderr)
    );
    let value_a: Value = serde_json::from_slice(&output_a.stdout).expect("peer a json");
    assert_eq!(value_a["status"], "ok");
    assert_eq!(value_a["relayFallbackActive"], false);
    assert_eq!(value_a["tunnelServiceSnapshot"]["connection_path"], "p2p");
    assert_eq!(value_a["runtimePeerSummaries"][0]["pathKind"], "direct");
    assert_eq!(value_a["tunnelServiceSnapshot"]["last_error"], Value::Null);
    assert!(value_a["relayFallbackEvents"]
        .as_array()
        .unwrap()
        .iter()
        .any(|event| event["status"] == "activated"));
    assert!(value_a["relayFallbackEvents"]
        .as_array()
        .unwrap()
        .iter()
        .any(|event| event["status"] == "restored-direct"));
    assert!(value_a["heartbeatAckPackets"]
        .as_array()
        .unwrap()
        .iter()
        .any(|packet| packet["direction"] == "received" && packet["connectionPath"] == "direct"));
}

#[test]
fn room_runtime_run_can_use_stun_for_nat_bootstrap_remote_peer() {
    let stun_server = UdpSocket::bind("127.0.0.1:0").expect("bind fake stun server");
    let stun_addr = stun_server.local_addr().unwrap();
    let stun_handle = std::thread::spawn(move || {
        let mut buffer = [0u8; 1500];
        let (received, peer) = stun_server
            .recv_from(&mut buffer)
            .expect("receive stun request");
        assert!(received >= 20, "short STUN request");
        let transaction_id = &buffer[8..20];
        let magic_cookie = 0x2112A442u32;
        let cookie_bytes = magic_cookie.to_be_bytes();
        let peer_ip = match peer.ip() {
            std::net::IpAddr::V4(ip) => ip.octets(),
            std::net::IpAddr::V6(_) => panic!("expected IPv4 peer"),
        };
        let xport = peer.port() ^ ((magic_cookie >> 16) as u16);

        let mut response = Vec::new();
        response.extend_from_slice(&0x0101u16.to_be_bytes());
        response.extend_from_slice(&12u16.to_be_bytes());
        response.extend_from_slice(&magic_cookie.to_be_bytes());
        response.extend_from_slice(transaction_id);
        response.extend_from_slice(&0x0020u16.to_be_bytes());
        response.extend_from_slice(&8u16.to_be_bytes());
        response.push(0);
        response.push(0x01);
        response.extend_from_slice(&xport.to_be_bytes());
        response.push(peer_ip[0] ^ cookie_bytes[0]);
        response.push(peer_ip[1] ^ cookie_bytes[1]);
        response.push(peer_ip[2] ^ cookie_bytes[2]);
        response.push(peer_ip[3] ^ cookie_bytes[3]);
        stun_server
            .send_to(&response, peer)
            .expect("send stun response");
    });

    let probe = UdpSocket::bind("127.0.0.1:0").expect("free udp port");
    let listener_addr = probe.local_addr().unwrap();
    drop(probe);
    let snapshot_path = std::env::temp_dir().join(format!(
        "lai-cli-runtime-bootstrap-stun-snapshot-{}.json",
        std::process::id()
    ));
    let snapshot_path_string = snapshot_path.display().to_string();
    fs::remove_file(&snapshot_path).ok();

    let listener = Command::new(env!("CARGO_BIN_EXE_lai-cli"))
        .args([
            "p2p-handshake-listen",
            "--bind",
            &listener_addr.to_string(),
            "--key",
            "test-room-key",
            "--responder-peer-id",
            "peer_b",
            "--max-packets",
            "1",
            "--timeout-ms",
            "2000",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn p2p listener");
    std::thread::sleep(Duration::from_millis(80));

    let remote_offer = serde_json::json!({
        "schema_version": 1,
        "room_id": "room_test",
        "peer_id": "peer_b",
        "nonce": "peer_b-runtime-offer",
        "created_at_ms": 1,
        "candidates": [{
            "candidate_type": "host",
            "transport": "udp",
            "endpoint": listener_addr.to_string(),
            "priority": 100,
            "source": "test-listener"
        }]
    });
    let remote_offer = serde_json::to_string(&remote_offer).unwrap();
    let value = run_cli(&[
        "room-runtime-run",
        "--room-id",
        "room_test",
        "--peer-id",
        "peer_a",
        "--virtual-ip",
        "10.77.12.2",
        "--bind",
        "127.0.0.1:0",
        "--key",
        "test-room-key",
        "--duration-ms",
        "50",
        "--snapshot-out",
        &snapshot_path_string,
        "--peer-timeout-ms",
        "0",
        "--nat-bootstrap-remote-peer",
        &format!("peer_b,10.77.12.3,{remote_offer}"),
        "--nat-bootstrap-attempts",
        "1",
        "--nat-bootstrap-interval-ms",
        "0",
        "--nat-bootstrap-timeout-ms",
        "2000",
        "--nat-bootstrap-stun-server",
        &format!("localhost:{}", stun_addr.port()),
        "--nat-bootstrap-stun-timeout-ms",
        "2000",
    ]);
    let listener_output = listener.wait_with_output().expect("listener exits");
    stun_handle.join().expect("fake stun server exits");

    assert_eq!(value["status"], "ok");
    assert_eq!(value["natBootstrapResults"][0]["status"], "ok");
    assert_eq!(
        value["natBootstrapResults"][0]["localOffer"]["candidates"]
            .as_array()
            .unwrap()
            .iter()
            .any(|candidate| candidate["candidate_type"] == "srflx"
                && candidate["source"] == "observed-endpoint"),
        true
    );
    assert!(
        listener_output.status.success(),
        "listener failed\nstatus: {}\nstdout: {}\nstderr: {}",
        listener_output.status,
        String::from_utf8_lossy(&listener_output.stdout),
        String::from_utf8_lossy(&listener_output.stderr)
    );
}

#[test]
fn room_runtime_run_fetches_coordination_offer_before_bootstrap() {
    let probe = UdpSocket::bind("127.0.0.1:0").expect("free udp port");
    let listener_addr = probe.local_addr().unwrap();
    drop(probe);
    let store_path = std::env::temp_dir().join(format!(
        "lai-cli-runtime-coordination-store-{}.json",
        std::process::id()
    ));
    let store_path_string = store_path.display().to_string();
    fs::remove_file(&store_path).ok();

    let listener = Command::new(env!("CARGO_BIN_EXE_lai-cli"))
        .args([
            "p2p-handshake-listen",
            "--bind",
            &listener_addr.to_string(),
            "--key",
            "test-room-key",
            "--responder-peer-id",
            "peer_b",
            "--max-packets",
            "1",
            "--timeout-ms",
            "2000",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn p2p listener");
    std::thread::sleep(Duration::from_millis(80));

    run_cli(&["coordination-store-init", "--out", &store_path_string]);
    let remote_offer = serde_json::json!({
        "schema_version": 1,
        "room_id": "room_test",
        "peer_id": "peer_b",
        "nonce": "nonce-b",
        "created_at_ms": 1,
        "candidates": [{
            "candidate_type": "host",
            "transport": "udp",
            "endpoint": listener_addr.to_string(),
            "priority": 100,
            "source": "test-listener"
        }]
    });
    let remote_offer = serde_json::to_string(&remote_offer).unwrap();
    run_cli(&[
        "coordination-offer-publish",
        "--store",
        &store_path_string,
        "--offer",
        &remote_offer,
        "--ttl-ms",
        "30000",
    ]);

    let value = run_cli(&[
        "room-runtime-run",
        "--room-id",
        "room_test",
        "--peer-id",
        "peer_a",
        "--virtual-ip",
        "10.77.12.2",
        "--bind",
        "127.0.0.1:0",
        "--key",
        "test-room-key",
        "--duration-ms",
        "50",
        "--peer-timeout-ms",
        "0",
        "--coordination-store",
        &store_path_string,
        "--coordination-peer",
        "peer_b,10.77.12.3",
        "--nat-bootstrap-attempts",
        "1",
        "--nat-bootstrap-interval-ms",
        "0",
        "--nat-bootstrap-timeout-ms",
        "2000",
    ]);
    let listener_output = listener.wait_with_output().expect("listener exits");
    fs::remove_file(&store_path).ok();

    assert_eq!(value["status"], "ok");
    assert_eq!(value["plan"]["tunnel"]["peer_count"], 1);
    assert_eq!(
        value["plan"]["peers"][0]["endpoint"],
        listener_addr.to_string()
    );
    assert_eq!(value["coordinationBootstrapResults"][0]["status"], "ok");
    assert_eq!(
        value["coordinationBootstrapResults"][1]["result"]["selectedPeer"]["responderPeerId"],
        "peer_b"
    );
    assert!(
        listener_output.status.success(),
        "listener failed\nstatus: {}\nstdout: {}\nstderr: {}",
        listener_output.status,
        String::from_utf8_lossy(&listener_output.stdout),
        String::from_utf8_lossy(&listener_output.stderr)
    );
}

#[test]
fn coordination_http_server_publishes_fetches_and_heartbeats_offers() {
    let probe = TcpListener::bind("127.0.0.1:0").expect("free local port");
    let server_addr = probe.local_addr().unwrap();
    drop(probe);
    let store_path = std::env::temp_dir().join(format!(
        "lai-cli-coordination-http-store-{}.json",
        std::process::id()
    ));
    let store_path_string = store_path.display().to_string();
    fs::remove_file(&store_path).ok();

    let server = Command::new(env!("CARGO_BIN_EXE_lai-cli"))
        .args([
            "coordination-http-serve",
            "--bind",
            &server_addr.to_string(),
            "--store",
            &store_path_string,
            "--max-requests",
            "4",
            "--request-timeout-ms",
            "5000",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn coordination http server");
    std::thread::sleep(Duration::from_millis(100));
    let server_url = format!("http://{server_addr}");

    let offer_b = serde_json::json!({
        "schema_version": 1,
        "room_id": "room_test",
        "peer_id": "peer_b",
        "nonce": "nonce-b",
        "created_at_ms": 1,
        "candidates": [{
            "candidate_type": "host",
            "transport": "udp",
            "endpoint": "127.0.0.1:39091",
            "priority": 100,
            "source": "test"
        }]
    });
    let offer_b = serde_json::to_string(&offer_b).unwrap();
    let publish = run_cli(&[
        "coordination-http-offer-publish",
        "--server",
        &server_url,
        "--offer",
        &offer_b,
        "--ttl-ms",
        "30000",
    ]);
    let fetch = run_cli(&[
        "coordination-http-offer-fetch",
        "--server",
        &server_url,
        "--room-id",
        "room_test",
        "--peer-id",
        "peer_a",
    ]);
    let heartbeat = run_cli(&[
        "coordination-http-heartbeat",
        "--server",
        &server_url,
        "--room-id",
        "room_test",
        "--peer-id",
        "peer_a",
        "--ttl-ms",
        "30000",
    ]);
    let prune = run_cli(&["coordination-http-prune", "--server", &server_url]);
    let server_output = server.wait_with_output().expect("server exits");
    fs::remove_file(&store_path).ok();

    assert_eq!(publish["status"], "ok");
    assert_eq!(publish["peer_id"], "peer_b");
    assert_eq!(fetch["status"], "ok");
    assert_eq!(fetch["offers"][0]["peer_id"], "peer_b");
    assert_eq!(heartbeat["status"], "ok");
    assert_eq!(heartbeat["peer_id"], "peer_a");
    assert_eq!(prune["status"], "ok");
    assert!(
        server_output.status.success(),
        "server failed\nstatus: {}\nstdout: {}\nstderr: {}",
        server_output.status,
        String::from_utf8_lossy(&server_output.stdout),
        String::from_utf8_lossy(&server_output.stderr)
    );
    let server_json: Value =
        serde_json::from_slice(&server_output.stdout).expect("server final json");
    assert_eq!(server_json["handledRequests"], 4);
}

#[test]
fn coordination_http_server_leaves_and_closes_rooms() {
    let probe = TcpListener::bind("127.0.0.1:0").expect("free local port");
    let server_addr = probe.local_addr().unwrap();
    drop(probe);
    let store_path = std::env::temp_dir().join(format!(
        "lai-cli-coordination-http-leave-close-store-{}.json",
        std::process::id()
    ));
    let store_path_string = store_path.display().to_string();
    fs::remove_file(&store_path).ok();

    let server = Command::new(env!("CARGO_BIN_EXE_lai-cli"))
        .args([
            "coordination-http-serve",
            "--bind",
            &server_addr.to_string(),
            "--store",
            &store_path_string,
            "--max-requests",
            "4",
            "--request-timeout-ms",
            "5000",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn coordination http server");
    std::thread::sleep(Duration::from_millis(100));
    let server_url = format!("http://{server_addr}");

    let offer_a = serde_json::json!({
        "schema_version": 1,
        "room_id": "room_test",
        "peer_id": "peer_a",
        "nonce": "nonce-a",
        "created_at_ms": 1,
        "candidates": [{
            "candidate_type": "host",
            "transport": "udp",
            "endpoint": "127.0.0.1:39090",
            "priority": 100,
            "source": "test"
        }]
    });
    let offer_b = serde_json::json!({
        "schema_version": 1,
        "room_id": "room_test",
        "peer_id": "peer_b",
        "nonce": "nonce-b",
        "created_at_ms": 1,
        "candidates": [{
            "candidate_type": "host",
            "transport": "udp",
            "endpoint": "127.0.0.1:39091",
            "priority": 100,
            "source": "test"
        }]
    });
    let offer_a = serde_json::to_string(&offer_a).unwrap();
    let offer_b = serde_json::to_string(&offer_b).unwrap();
    run_cli(&[
        "coordination-http-offer-publish",
        "--server",
        &server_url,
        "--offer",
        &offer_a,
        "--ttl-ms",
        "30000",
    ]);
    run_cli(&[
        "coordination-http-offer-publish",
        "--server",
        &server_url,
        "--offer",
        &offer_b,
        "--ttl-ms",
        "30000",
    ]);
    let leave = run_cli(&[
        "coordination-http-leave",
        "--server",
        &server_url,
        "--room-id",
        "room_test",
        "--peer-id",
        "peer_a",
    ]);
    let close = run_cli(&[
        "coordination-http-close",
        "--server",
        &server_url,
        "--room-id",
        "room_test",
    ]);
    let server_output = server.wait_with_output().expect("server exits");
    fs::remove_file(&store_path).ok();

    assert_eq!(leave["status"], "ok");
    assert_eq!(leave["peer_removed"], true);
    assert_eq!(leave["remaining_peer_count"], 1);
    assert_eq!(close["status"], "ok");
    assert_eq!(close["room_removed"], true);
    assert_eq!(close["removed_peer_count"], 1);
    assert!(
        server_output.status.success(),
        "server failed\nstatus: {}\nstdout: {}\nstderr: {}",
        server_output.status,
        String::from_utf8_lossy(&server_output.stdout),
        String::from_utf8_lossy(&server_output.stderr)
    );
    let server_json: Value =
        serde_json::from_slice(&server_output.stdout).expect("server final json");
    assert_eq!(server_json["handledRequests"], 4);
}

#[test]
fn coordination_http_server_kicks_peer_from_room() {
    let probe = TcpListener::bind("127.0.0.1:0").expect("free local port");
    let server_addr = probe.local_addr().unwrap();
    drop(probe);
    let store_path = std::env::temp_dir().join(format!(
        "lai-cli-coordination-http-kick-store-{}.json",
        std::process::id()
    ));
    let store_path_string = store_path.display().to_string();
    fs::remove_file(&store_path).ok();

    let server = Command::new(env!("CARGO_BIN_EXE_lai-cli"))
        .args([
            "coordination-http-serve",
            "--bind",
            &server_addr.to_string(),
            "--store",
            &store_path_string,
            "--max-requests",
            "5",
            "--request-timeout-ms",
            "5000",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn coordination http server");
    std::thread::sleep(Duration::from_millis(100));
    let server_url = format!("http://{server_addr}");

    let offer_a = serde_json::json!({
        "schema_version": 1,
        "room_id": "room_test",
        "peer_id": "peer_a",
        "nonce": "nonce-a",
        "created_at_ms": 1,
        "candidates": [{
            "candidate_type": "host",
            "transport": "udp",
            "endpoint": "127.0.0.1:39090",
            "priority": 100,
            "source": "test"
        }]
    });
    let offer_b = serde_json::json!({
        "schema_version": 1,
        "room_id": "room_test",
        "peer_id": "peer_b",
        "nonce": "nonce-b",
        "created_at_ms": 1,
        "candidates": [{
            "candidate_type": "host",
            "transport": "udp",
            "endpoint": "127.0.0.1:39091",
            "priority": 100,
            "source": "test"
        }]
    });
    let offer_a = serde_json::to_string(&offer_a).unwrap();
    let offer_b = serde_json::to_string(&offer_b).unwrap();
    run_cli(&[
        "coordination-http-offer-publish",
        "--server",
        &server_url,
        "--offer",
        &offer_a,
        "--ttl-ms",
        "30000",
    ]);
    run_cli(&[
        "coordination-http-offer-publish",
        "--server",
        &server_url,
        "--offer",
        &offer_b,
        "--ttl-ms",
        "30000",
    ]);
    let forbidden = run_cli(&[
        "coordination-http-kick",
        "--server",
        &server_url,
        "--room-id",
        "room_test",
        "--peer-id",
        "peer_a",
        "--kicked-by",
        "peer_b",
    ]);
    let kick = run_cli(&[
        "coordination-http-kick",
        "--server",
        &server_url,
        "--room-id",
        "room_test",
        "--peer-id",
        "peer_b",
        "--kicked-by",
        "peer_a",
    ]);
    let fetch = run_cli(&[
        "coordination-http-offer-fetch",
        "--server",
        &server_url,
        "--room-id",
        "room_test",
        "--peer-id",
        "peer_a",
    ]);
    let server_output = server.wait_with_output().expect("server exits");
    fs::remove_file(&store_path).ok();

    assert_eq!(forbidden["status"], "forbidden");
    assert_eq!(forbidden["peer_removed"], false);
    assert_eq!(forbidden["host_peer_id"], "peer_a");
    assert_eq!(kick["status"], "ok");
    assert_eq!(kick["peer_removed"], true);
    assert_eq!(kick["kicked_by"], "peer_a");
    assert_eq!(kick["host_peer_id"], "peer_a");
    assert_eq!(kick["remaining_peer_count"], 1);
    assert_eq!(fetch["status"], "empty");
    assert!(
        server_output.status.success(),
        "server failed\nstatus: {}\nstdout: {}\nstderr: {}",
        server_output.status,
        String::from_utf8_lossy(&server_output.stdout),
        String::from_utf8_lossy(&server_output.stderr)
    );
    let server_json: Value =
        serde_json::from_slice(&server_output.stdout).expect("server final json");
    assert_eq!(server_json["handledRequests"], 5);
}

#[test]
fn coordination_http_server_closes_room_only_by_host_when_actor_is_provided() {
    let probe = TcpListener::bind("127.0.0.1:0").expect("free local port");
    let server_addr = probe.local_addr().unwrap();
    drop(probe);
    let store_path = std::env::temp_dir().join(format!(
        "lai-cli-coordination-http-close-auth-store-{}.json",
        std::process::id()
    ));
    let store_path_string = store_path.display().to_string();
    fs::remove_file(&store_path).ok();

    let server = Command::new(env!("CARGO_BIN_EXE_lai-cli"))
        .args([
            "coordination-http-serve",
            "--bind",
            &server_addr.to_string(),
            "--store",
            &store_path_string,
            "--max-requests",
            "4",
            "--request-timeout-ms",
            "5000",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn coordination http server");
    std::thread::sleep(Duration::from_millis(100));
    let server_url = format!("http://{server_addr}");

    let offer_a = serde_json::json!({
        "schema_version": 1,
        "room_id": "room_test",
        "peer_id": "peer_a",
        "nonce": "nonce-a",
        "created_at_ms": 1,
        "candidates": [{
            "candidate_type": "host",
            "transport": "udp",
            "endpoint": "127.0.0.1:39090",
            "priority": 100,
            "source": "test"
        }]
    });
    let offer_b = serde_json::json!({
        "schema_version": 1,
        "room_id": "room_test",
        "peer_id": "peer_b",
        "nonce": "nonce-b",
        "created_at_ms": 1,
        "candidates": [{
            "candidate_type": "host",
            "transport": "udp",
            "endpoint": "127.0.0.1:39091",
            "priority": 100,
            "source": "test"
        }]
    });
    let offer_a = serde_json::to_string(&offer_a).unwrap();
    let offer_b = serde_json::to_string(&offer_b).unwrap();
    run_cli(&[
        "coordination-http-offer-publish",
        "--server",
        &server_url,
        "--offer",
        &offer_a,
        "--ttl-ms",
        "30000",
    ]);
    run_cli(&[
        "coordination-http-offer-publish",
        "--server",
        &server_url,
        "--offer",
        &offer_b,
        "--ttl-ms",
        "30000",
    ]);
    let forbidden = run_cli(&[
        "coordination-http-close",
        "--server",
        &server_url,
        "--room-id",
        "room_test",
        "--closed-by",
        "peer_b",
    ]);
    let close = run_cli(&[
        "coordination-http-close",
        "--server",
        &server_url,
        "--room-id",
        "room_test",
        "--closed-by",
        "peer_a",
    ]);
    let server_output = server.wait_with_output().expect("server exits");
    fs::remove_file(&store_path).ok();

    assert_eq!(forbidden["status"], "forbidden");
    assert_eq!(forbidden["closed_by"], "peer_b");
    assert_eq!(forbidden["host_peer_id"], "peer_a");
    assert_eq!(forbidden["room_removed"], false);
    assert_eq!(close["status"], "ok");
    assert_eq!(close["closed_by"], "peer_a");
    assert_eq!(close["host_peer_id"], "peer_a");
    assert_eq!(close["room_removed"], true);
    assert_eq!(close["removed_peer_count"], 2);
    assert!(
        server_output.status.success(),
        "server failed\nstatus: {}\nstdout: {}\nstderr: {}",
        server_output.status,
        String::from_utf8_lossy(&server_output.stdout),
        String::from_utf8_lossy(&server_output.stderr)
    );
    let server_json: Value =
        serde_json::from_slice(&server_output.stdout).expect("server final json");
    assert_eq!(server_json["handledRequests"], 4);
}

#[test]
fn room_runtime_run_fetches_http_coordination_offer_before_bootstrap() {
    let http_probe = TcpListener::bind("127.0.0.1:0").expect("free local port");
    let server_addr = http_probe.local_addr().unwrap();
    drop(http_probe);
    let handshake_probe = UdpSocket::bind("127.0.0.1:0").expect("free udp port");
    let listener_addr = handshake_probe.local_addr().unwrap();
    drop(handshake_probe);
    let store_path = std::env::temp_dir().join(format!(
        "lai-cli-runtime-coordination-http-store-{}.json",
        std::process::id()
    ));
    let store_path_string = store_path.display().to_string();
    fs::remove_file(&store_path).ok();

    let server = Command::new(env!("CARGO_BIN_EXE_lai-cli"))
        .args([
            "coordination-http-serve",
            "--bind",
            &server_addr.to_string(),
            "--store",
            &store_path_string,
            "--max-requests",
            "20",
            "--request-timeout-ms",
            "10000",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn coordination http server");
    std::thread::sleep(Duration::from_millis(100));

    let listener = Command::new(env!("CARGO_BIN_EXE_lai-cli"))
        .args([
            "p2p-handshake-listen",
            "--bind",
            &listener_addr.to_string(),
            "--key",
            "test-room-key",
            "--responder-peer-id",
            "peer_b",
            "--max-packets",
            "1",
            "--timeout-ms",
            "2000",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn p2p listener");
    std::thread::sleep(Duration::from_millis(80));

    let server_url = format!("http://{server_addr}");
    let remote_offer = serde_json::json!({
        "schema_version": 1,
        "room_id": "room_test",
        "peer_id": "peer_b",
        "nonce": "peer_b-runtime-offer",
        "created_at_ms": 1,
        "candidates": [{
            "candidate_type": "host",
            "transport": "udp",
            "endpoint": listener_addr.to_string(),
            "priority": 100,
            "source": "test-listener"
        }]
    });
    let remote_offer = serde_json::to_string(&remote_offer).unwrap();
    run_cli(&[
        "coordination-http-offer-publish",
        "--server",
        &server_url,
        "--offer",
        &remote_offer,
        "--ttl-ms",
        "30000",
    ]);

    let value = run_cli(&[
        "room-runtime-run",
        "--room-id",
        "room_test",
        "--peer-id",
        "peer_a",
        "--virtual-ip",
        "10.77.12.2",
        "--bind",
        "127.0.0.1:0",
        "--key",
        "test-room-key",
        "--duration-ms",
        "50",
        "--peer-timeout-ms",
        "0",
        "--coordination-server",
        &server_url,
        "--coordination-peer",
        "peer_b,10.77.12.3",
        "--nat-bootstrap-attempts",
        "1",
        "--nat-bootstrap-interval-ms",
        "0",
        "--nat-bootstrap-timeout-ms",
        "2000",
    ]);
    let listener_output = listener.wait_with_output().expect("listener exits");
    let mut server = server;
    let mut server_was_killed = false;
    let server_output = match server.try_wait().expect("server wait check") {
        Some(_) => server.wait_with_output().expect("server exits"),
        None => {
            server_was_killed = true;
            server.kill().ok();
            server.wait_with_output().expect("server killed")
        }
    };
    fs::remove_file(&store_path).ok();

    assert_eq!(value["status"], "ok");
    assert_eq!(value["plan"]["tunnel"]["peer_count"], 1);
    assert_eq!(
        value["plan"]["peers"][0]["endpoint"],
        listener_addr.to_string()
    );
    assert_eq!(
        value["coordinationBootstrapResults"][0]["source"],
        "coordination-http"
    );
    assert_eq!(
        value["coordinationBootstrapResults"][1]["result"]["selectedPeer"]["responderPeerId"],
        "peer_b"
    );
    assert!(
        listener_output.status.success(),
        "listener failed\nstatus: {}\nstdout: {}\nstderr: {}",
        listener_output.status,
        String::from_utf8_lossy(&listener_output.stdout),
        String::from_utf8_lossy(&listener_output.stderr)
    );
    assert!(
        server_was_killed || server_output.status.success(),
        "server failed\nstatus: {}\nstdout: {}\nstderr: {}",
        server_output.status,
        String::from_utf8_lossy(&server_output.stdout),
        String::from_utf8_lossy(&server_output.stderr)
    );
}

#[test]
fn room_runtime_run_waits_for_delayed_http_coordination_offer() {
    let http_probe = TcpListener::bind("127.0.0.1:0").expect("free local port");
    let server_addr = http_probe.local_addr().unwrap();
    drop(http_probe);
    let handshake_probe = UdpSocket::bind("127.0.0.1:0").expect("free udp port");
    let listener_addr = handshake_probe.local_addr().unwrap();
    drop(handshake_probe);
    let store_path = std::env::temp_dir().join(format!(
        "lai-cli-runtime-coordination-http-wait-store-{}.json",
        std::process::id()
    ));
    let store_path_string = store_path.display().to_string();
    fs::remove_file(&store_path).ok();

    let server = Command::new(env!("CARGO_BIN_EXE_lai-cli"))
        .args([
            "coordination-http-serve",
            "--bind",
            &server_addr.to_string(),
            "--store",
            &store_path_string,
            "--max-requests",
            "8",
            "--request-timeout-ms",
            "5000",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn coordination http server");
    std::thread::sleep(Duration::from_millis(100));

    let listener = Command::new(env!("CARGO_BIN_EXE_lai-cli"))
        .args([
            "p2p-handshake-listen",
            "--bind",
            &listener_addr.to_string(),
            "--key",
            "test-room-key",
            "--responder-peer-id",
            "peer_b",
            "--max-packets",
            "1",
            "--timeout-ms",
            "3000",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn p2p listener");
    std::thread::sleep(Duration::from_millis(80));

    let server_url = format!("http://{server_addr}");
    let stale_addr = "127.0.0.1:9";
    let stale_offer = serde_json::json!({
        "schema_version": 1,
        "room_id": "room_wait_test",
        "peer_id": "peer_b",
        "nonce": "peer_b-desktop-offer",
        "created_at_ms": 1,
        "candidates": [{
            "candidate_type": "host",
            "transport": "udp",
            "endpoint": stale_addr,
            "priority": 100,
            "source": "test-stale-desktop"
        }]
    });
    let stale_offer = serde_json::to_string(&stale_offer).unwrap();
    run_cli(&[
        "coordination-http-offer-publish",
        "--server",
        &server_url,
        "--offer",
        &stale_offer,
        "--ttl-ms",
        "30000",
    ]);

    let remote_offer = serde_json::json!({
        "schema_version": 1,
        "room_id": "room_wait_test",
        "peer_id": "peer_b",
        "nonce": "peer_b-runtime-offer",
        "created_at_ms": 1,
        "candidates": [{
            "candidate_type": "host",
            "transport": "udp",
            "endpoint": listener_addr.to_string(),
            "priority": 100,
            "source": "test-listener"
        }]
    });
    let remote_offer = serde_json::to_string(&remote_offer).unwrap();
    let publish_server_url = server_url.clone();
    let publisher = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(350));
        run_cli(&[
            "coordination-http-offer-publish",
            "--server",
            &publish_server_url,
            "--offer",
            &remote_offer,
            "--ttl-ms",
            "30000",
        ])
    });

    let value = run_cli(&[
        "room-runtime-run",
        "--room-id",
        "room_wait_test",
        "--peer-id",
        "peer_a",
        "--virtual-ip",
        "10.77.12.2",
        "--bind",
        "127.0.0.1:0",
        "--key",
        "test-room-key",
        "--duration-ms",
        "50",
        "--peer-timeout-ms",
        "0",
        "--coordination-server",
        &server_url,
        "--coordination-peer",
        "peer_b,10.77.12.3",
        "--nat-bootstrap-attempts",
        "1",
        "--nat-bootstrap-interval-ms",
        "100",
        "--nat-bootstrap-timeout-ms",
        "2500",
    ]);
    let publish_value = publisher.join().expect("publisher joins");
    let listener_output = listener.wait_with_output().expect("listener exits");
    let mut server = server;
    let mut server_was_killed = false;
    let server_output = match server.try_wait().expect("server wait check") {
        Some(_) => server.wait_with_output().expect("server exits"),
        None => {
            server_was_killed = true;
            server.kill().ok();
            server.wait_with_output().expect("server killed")
        }
    };
    fs::remove_file(&store_path).ok();

    assert_eq!(publish_value["status"], "ok");
    assert_eq!(value["status"], "ok");
    assert_eq!(value["plan"]["tunnel"]["peer_count"], 1);
    assert_eq!(
        value["plan"]["peers"][0]["endpoint"],
        listener_addr.to_string()
    );
    assert_eq!(
        value["coordinationBootstrapResults"][0]["fetchAttempts"]
            .as_u64()
            .unwrap_or_default()
            > 1,
        true
    );
    assert_eq!(
        value["coordinationBootstrapResults"][1]["result"]["selectedPeer"]["responderPeerId"],
        "peer_b"
    );
    assert!(
        listener_output.status.success(),
        "listener failed\nstatus: {}\nstdout: {}\nstderr: {}",
        listener_output.status,
        String::from_utf8_lossy(&listener_output.stdout),
        String::from_utf8_lossy(&listener_output.stderr)
    );
    assert!(
        server_was_killed || server_output.status.success(),
        "server failed\nstatus: {}\nstdout: {}\nstderr: {}",
        server_output.status,
        String::from_utf8_lossy(&server_output.stdout),
        String::from_utf8_lossy(&server_output.stderr)
    );
}

#[test]
fn room_runtime_run_stops_when_coordination_store_peer_is_removed() {
    let store_path = std::env::temp_dir().join(format!(
        "lai-cli-runtime-monitor-kick-store-{}.json",
        std::process::id()
    ));
    let store_path_string = store_path.display().to_string();
    fs::remove_file(&store_path).ok();

    run_cli(&["coordination-store-init", "--out", &store_path_string]);
    let remote = run_cli(&[
        "nat-candidates",
        "--room-id",
        "room_test",
        "--peer-id",
        "peer_b",
        "--bind",
        "127.0.0.1:0",
        "--nonce",
        "nonce-b",
    ]);
    let remote_offer = serde_json::to_string(&remote["offer"]).unwrap();
    run_cli(&[
        "coordination-offer-publish",
        "--store",
        &store_path_string,
        "--offer",
        &remote_offer,
        "--ttl-ms",
        "30000",
    ]);

    let value = run_cli(&[
        "room-runtime-run",
        "--room-id",
        "room_test",
        "--peer-id",
        "peer_a",
        "--virtual-ip",
        "10.77.12.2",
        "--bind",
        "127.0.0.1:0",
        "--key",
        "test-room-key",
        "--duration-ms",
        "1000",
        "--peer-timeout-ms",
        "0",
        "--coordination-store",
        &store_path_string,
        "--coordination-monitor",
        "true",
        "--coordination-monitor-interval-ms",
        "1",
    ]);
    fs::remove_file(&store_path).ok();

    assert_eq!(value["status"], "degraded");
    assert_eq!(value["stopReason"], "coordination-peer-removed");
    assert_eq!(
        value["coordinationMonitorReports"][0]["status"],
        "peer-removed"
    );
    assert_eq!(
        value["coordinationMonitorReports"][0]["peer_present"],
        false
    );
    assert_eq!(value["coordinationMonitorReports"][0]["room_present"], true);
}

#[test]
fn room_runtime_run_stops_when_http_coordination_room_is_closed() {
    let probe = TcpListener::bind("127.0.0.1:0").expect("free local port");
    let server_addr = probe.local_addr().unwrap();
    drop(probe);
    let store_path = std::env::temp_dir().join(format!(
        "lai-cli-runtime-monitor-http-store-{}.json",
        std::process::id()
    ));
    let store_path_string = store_path.display().to_string();
    fs::remove_file(&store_path).ok();

    let server = Command::new(env!("CARGO_BIN_EXE_lai-cli"))
        .args([
            "coordination-http-serve",
            "--bind",
            &server_addr.to_string(),
            "--store",
            &store_path_string,
            "--max-requests",
            "1",
            "--request-timeout-ms",
            "5000",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn coordination http server");
    std::thread::sleep(Duration::from_millis(100));
    let server_url = format!("http://{server_addr}");

    let value = run_cli(&[
        "room-runtime-run",
        "--room-id",
        "room_test",
        "--peer-id",
        "peer_a",
        "--virtual-ip",
        "10.77.12.2",
        "--bind",
        "127.0.0.1:0",
        "--key",
        "test-room-key",
        "--duration-ms",
        "1000",
        "--peer-timeout-ms",
        "0",
        "--coordination-server",
        &server_url,
        "--coordination-monitor",
        "true",
        "--coordination-monitor-interval-ms",
        "1",
    ]);
    let server_output = server.wait_with_output().expect("server exits");
    fs::remove_file(&store_path).ok();

    assert_eq!(value["status"], "degraded");
    assert_eq!(value["stopReason"], "coordination-room-closed");
    assert_eq!(
        value["coordinationMonitorReports"][0]["status"],
        "room-closed"
    );
    assert_eq!(
        value["coordinationMonitorReports"][0]["peer_present"],
        false
    );
    assert_eq!(
        value["coordinationMonitorReports"][0]["room_present"],
        false
    );
    assert!(
        server_output.status.success(),
        "server failed\nstatus: {}\nstdout: {}\nstderr: {}",
        server_output.status,
        String::from_utf8_lossy(&server_output.stdout),
        String::from_utf8_lossy(&server_output.stderr)
    );
}

#[test]
fn room_runtime_run_outputs_snapshots_and_packet_observations() {
    let path = std::env::temp_dir().join(format!(
        "lai-cli-room-runtime-observation-{}.txt",
        std::process::id()
    ));
    let snapshot_path = std::env::temp_dir().join(format!(
        "lai-cli-room-runtime-snapshot-{}.json",
        std::process::id()
    ));
    let path_string = path.display().to_string();
    let snapshot_path_string = snapshot_path.display().to_string();
    let value = run_cli(&[
        "room-runtime-run",
        "--room-id",
        "room_test",
        "--peer-id",
        "peer_a",
        "--virtual-ip",
        "10.77.12.2",
        "--bind",
        "127.0.0.1:0",
        "--key",
        "test-room-key",
        "--game-ports",
        "0",
        "--broadcast-ports",
        "0",
        "--duration-ms",
        "150",
        "--self-probe",
        "true",
        "--capture-self-probe",
        "true",
        "--forward-self-probe",
        "true",
        "--inject-self-probe",
        "true",
        "--observe-file",
        &path_string,
        "--snapshot-out",
        &snapshot_path_string,
    ]);

    assert_eq!(value["status"], "ok");
    assert_eq!(value["tunnelServiceSnapshot"]["service_running"], true);
    assert_eq!(value["tunnelServiceSnapshot"]["connected_peer_count"], 0);
    assert_eq!(
        value["networkObservation"]["diagnostic_snapshot"]["p2p"],
        "failed"
    );
    assert_eq!(
        value["networkObservation"]["diagnostic_snapshot"]["game_traffic"],
        "seen"
    );
    assert_eq!(
        value["networkObservation"]["diagnostic_snapshot"]["broadcast"],
        "seen"
    );
    assert_eq!(value["packetCaptureSummaries"].as_array().unwrap().len(), 2);
    assert_eq!(value["forwardedPackets"].as_array().unwrap().len(), 1);
    assert_eq!(value["broadcastForwardReport"]["status"], "ok");
    assert_eq!(value["broadcastForwardReport"]["event_count"], 1);
    assert_eq!(value["broadcastForwardReport"]["forwarded_event_count"], 1);
    assert_eq!(
        value["broadcastForwardReport"]["events"][0]["reason"],
        "userspace-capture-forwarded"
    );
    assert_eq!(value["injectedPackets"].as_array().unwrap().len(), 1);
    assert_eq!(
        value["injectedReceivedPackets"].as_array().unwrap().len(),
        1
    );
    assert!(value["tunnelPackets"]
        .as_array()
        .unwrap()
        .iter()
        .any(|packet| packet["kind"] == "runtime-udp-forward"));
    let peer_summaries = value["runtimePeerSummaries"].as_array().unwrap();
    assert_eq!(peer_summaries.len(), 1);
    assert_eq!(peer_summaries[0]["peerId"], "peer_a-self-probe");
    assert_eq!(peer_summaries[0]["pathKind"], "direct");
    assert_eq!(peer_summaries[0]["health"]["status"], "ok");
    assert_eq!(peer_summaries[0]["forwardedPacketsSent"], 1);
    assert_eq!(peer_summaries[0]["tunnelPacketsReceived"], 3);
    assert_eq!(
        value["runtimeCleanupPlan"]["packet_io_backend"],
        "userspace-udp"
    );
    assert_eq!(value["runtimeCleanupPlan"]["requires_elevation"], false);
    assert!(value["runtimeCleanupPlan"]["process_cleanup_steps"]
        .as_array()
        .unwrap()
        .iter()
        .any(|step| step["key"] == "close-tunnel-socket"));

    let observations = fs::read_to_string(&path).expect("observation file");
    let snapshot = fs::read_to_string(&snapshot_path).expect("snapshot file");
    fs::remove_file(&path).ok();
    fs::remove_file(&snapshot_path).ok();
    assert!(observations.contains(":unicast:inbound:21"));
    assert!(observations.contains(":broadcast:inbound:21"));
    assert!(snapshot.contains("\"tunnelServiceSnapshot\""));
}

#[test]
fn runtime_cleanup_plan_can_include_adapter_restore_commands() {
    let value = run_cli(&[
        "runtime-cleanup-plan",
        "--room-id",
        "room_test",
        "--peer-id",
        "peer_a",
        "--virtual-ip",
        "10.77.12.2",
        "--adapter-name",
        "Local Area Interconnection",
        "--packet-io-backend",
        "wintun",
        "--restore-adapter",
        "true",
    ]);

    assert_eq!(value["platform"], "windows");
    assert_eq!(value["dry_run"], true);
    assert_eq!(value["requires_elevation"], true);
    assert_eq!(value["commands"].as_array().unwrap().len(), 4);
    assert!(value["commands"][0]["command"]
        .as_str()
        .unwrap()
        .contains("set address name=\"Local Area Interconnection\" dhcp"));
    assert!(value["process_cleanup_steps"]
        .as_array()
        .unwrap()
        .iter()
        .any(|step| step["key"] == "close-wintun-session"));
}

#[test]
fn runtime_cleanup_report_flags_adapter_that_still_has_room_ip() {
    let snapshot_path = std::env::temp_dir().join(format!(
        "lai-runtime-cleanup-snapshot-{}.json",
        std::process::id()
    ));
    let netsh_path = std::env::temp_dir().join(format!(
        "lai-runtime-cleanup-netsh-{}.txt",
        std::process::id()
    ));
    let route_path = std::env::temp_dir().join(format!(
        "lai-runtime-cleanup-route-{}.txt",
        std::process::id()
    ));
    let snapshot_arg = snapshot_path.to_string_lossy().to_string();
    let netsh_arg = netsh_path.to_string_lossy().to_string();
    let route_arg = route_path.to_string_lossy().to_string();
    fs::write(
        &snapshot_path,
        serde_json::to_string_pretty(&serde_json::json!({
            "status": "ok",
            "wintunRuntime": {
                "close": {
                    "session_ended": true,
                    "closed": true
                }
            },
            "runtimeCleanupPlan": {
                "platform": "windows",
                "dry_run": true,
                "room_id": "room_test",
                "local_peer_id": "peer_a",
                "local_virtual_ip": "10.77.12.2",
                "adapter_name": "LocalAreaInterconnection",
                "packet_io_backend": "wintun",
                "restore_adapter": true,
                "requires_elevation": true,
                "process_cleanup_steps": [{
                    "key": "close-wintun-session",
                    "status": "automatic",
                    "detail": "Close Wintun session."
                }],
                "commands": [{
                    "tool": "netsh",
                    "args": ["interface", "ipv4", "set", "address", "name=LocalAreaInterconnection", "dhcp"],
                    "command": "netsh interface ipv4 set address name=LocalAreaInterconnection dhcp",
                    "purpose": "Restore adapter IPv4 address mode."
                }],
                "verification_checks": ["Adapter configuration reviewed."],
                "warnings": []
            }
        }))
        .unwrap(),
    )
    .expect("write runtime snapshot");
    fs::write(
        &netsh_path,
        r#"
Configuration for interface "LocalAreaInterconnection"
    DHCP enabled:                         No
    IP Address:                           10.77.12.2
    Subnet Prefix:                        10.77.12.0/24 (mask 255.255.255.0)
    MTU:                                  1420
"#,
    )
    .expect("write netsh output");
    fs::write(
        &route_path,
        r#"
IPv4 Route Table
===========================================================================
Active Routes:
Network Destination        Netmask          Gateway       Interface  Metric
       10.77.12.0    255.255.255.0         On-link       10.77.12.2      5
"#,
    )
    .expect("write route output");

    let value = run_cli(&[
        "runtime-cleanup-report",
        "--runtime-snapshot",
        &snapshot_arg,
        "--adapter-netsh-output",
        &netsh_arg,
        "--route-output",
        &route_arg,
    ]);
    fs::remove_file(&snapshot_path).ok();
    fs::remove_file(&netsh_path).ok();
    fs::remove_file(&route_path).ok();

    assert_eq!(value["adapterSource"]["source"], "netsh-file");
    assert_eq!(value["routeSource"]["source"], "route-file");
    assert_eq!(value["report"]["status"], "needs-attention");
    assert_eq!(value["report"]["wintun_close"]["closed"], true);
    assert!(value["report"]["checks"]
        .as_array()
        .unwrap()
        .iter()
        .any(|check| check["key"] == "adapter-restore" && check["status"] == "needs-attention"));
    assert!(value["report"]["checks"]
        .as_array()
        .unwrap()
        .iter()
        .any(|check| check["key"] == "route-cleanup" && check["status"] == "needs-attention"));
}

#[test]
fn runtime_cleanup_apply_requires_confirmation_for_safe_plan() {
    let plan = run_cli(&[
        "runtime-cleanup-plan",
        "--room-id",
        "room_test",
        "--peer-id",
        "peer_a",
        "--virtual-ip",
        "10.77.12.2",
        "--subnet",
        "10.77.12.0/24",
        "--adapter-name",
        "LocalAreaInterconnection",
        "--packet-io-backend",
        "wintun",
        "--restore-adapter",
        "true",
        "--cleanup-routes",
        "true",
    ]);
    let plan_arg = serde_json::to_string(&plan).unwrap();
    let value = run_cli(&["runtime-cleanup-apply", "--cleanup-plan", &plan_arg]);

    assert_eq!(value["status"], "needs-confirmation");
    assert_eq!(value["executionPreview"]["confirmed"], false);
    assert_eq!(value["executionPreview"]["can_execute_now"], false);
    assert_eq!(value["commandResults"].as_array().unwrap().len(), 0);
    assert_eq!(value["unsafeCommands"].as_array().unwrap().len(), 0);
    assert!(value["nextAction"].as_str().unwrap().contains("--yes true"));
}

#[test]
fn runtime_cleanup_apply_blocks_tampered_commands() {
    let plan = serde_json::json!({
        "platform": "windows",
        "dry_run": true,
        "room_id": "room_test",
        "local_peer_id": "peer_a",
        "local_virtual_ip": "10.77.12.2",
        "adapter_name": "LocalAreaInterconnection",
        "packet_io_backend": "userspace-udp",
        "restore_adapter": false,
        "cleanup_routes": false,
        "requires_elevation": false,
        "process_cleanup_steps": [],
        "commands": [{
            "tool": "powershell",
            "args": ["-NoProfile", "-Command", "Write-Output bad"],
            "command": "powershell -NoProfile -Command Write-Output bad",
            "purpose": "Unexpected command."
        }],
        "verification_checks": [],
        "warnings": []
    });
    let plan_arg = serde_json::to_string(&plan).unwrap();
    let value = run_cli(&[
        "runtime-cleanup-apply",
        "--cleanup-plan",
        &plan_arg,
        "--yes",
        "true",
    ]);

    assert_eq!(value["status"], "blocked-unsafe-command");
    assert_eq!(value["executionPreview"]["can_execute_now"], true);
    assert_eq!(value["commandResults"].as_array().unwrap().len(), 0);
    assert!(value["unsafeCommands"][0]
        .as_str()
        .unwrap()
        .contains("Rejected cleanup command"));
}

#[test]
fn route_scan_reports_room_route_matches() {
    let route_path =
        std::env::temp_dir().join(format!("lai-cli-route-scan-{}.txt", std::process::id()));
    let route_arg = route_path.to_string_lossy().to_string();
    fs::write(
        &route_path,
        r#"
IPv4 Route Table
===========================================================================
Active Routes:
Network Destination        Netmask          Gateway       Interface  Metric
        0.0.0.0          0.0.0.0      192.168.1.1    192.168.1.10     25
       10.77.12.0    255.255.255.0         On-link       10.77.12.2      5
Persistent Routes:
  Network Address          Netmask  Gateway Address  Metric
       10.77.12.2  255.255.255.255         On-link       1
"#,
    )
    .expect("write route output");

    let value = run_cli(&[
        "route-scan",
        "--route-output",
        &route_arg,
        "--route-scan",
        "false",
        "--virtual-ip",
        "10.77.12.2",
        "--subnet",
        "10.77.12.0/24",
    ]);
    fs::remove_file(&route_path).ok();

    assert_eq!(value["status"], "needs-attention");
    assert_eq!(value["routeSource"]["source"], "route-file");
    assert_eq!(value["routeCount"], 3);
    assert_eq!(value["roomRouteCount"], 2);
    assert!(value["roomRoutes"]
        .as_array()
        .unwrap()
        .iter()
        .any(|route| { route["destination"] == "10.77.12.0/24" && route["persistent"] == false }));
    assert!(value["roomRoutes"]
        .as_array()
        .unwrap()
        .iter()
        .any(|route| { route["destination"] == "10.77.12.2/32" && route["persistent"] == true }));
}

#[test]
fn game_port_scan_matches_netstat_ports() {
    let netstat_path =
        std::env::temp_dir().join(format!("lai-cli-netstat-scan-{}.txt", std::process::id()));
    let netstat_arg = netstat_path.to_string_lossy().to_string();
    fs::write(
        &netstat_path,
        r#"
Active Connections

  Proto  Local Address          Foreign Address        State           PID
  TCP    0.0.0.0:27015          0.0.0.0:0              LISTENING       4242
  UDP    0.0.0.0:27016          *:*                                    4243
  TCP    127.0.0.1:50000        127.0.0.1:50001        ESTABLISHED     4244
"#,
    )
    .expect("write netstat output");

    let value = run_cli(&[
        "game-port-scan",
        "--netstat-output",
        &netstat_arg,
        "--netstat-scan",
        "false",
        "--ports",
        "27015,27016",
    ]);
    fs::remove_file(&netstat_path).ok();

    assert_eq!(value["status"], "ok");
    assert_eq!(value["netstatSource"]["source"], "netstat-file");
    assert_eq!(value["endpointCount"], 3);
    assert_eq!(value["matchCount"], 2);
    assert!(value["matches"]
        .as_array()
        .unwrap()
        .iter()
        .any(|endpoint| endpoint["protocol"] == "tcp" && endpoint["local_port"] == 27015));
    assert!(value["matches"]
        .as_array()
        .unwrap()
        .iter()
        .any(|endpoint| endpoint["protocol"] == "udp" && endpoint["local_port"] == 27016));
}

#[test]
fn game_port_scan_can_use_catalog_profile_ports() {
    let catalog_path = std::env::temp_dir().join(format!(
        "lai-cli-port-scan-catalog-{}.json",
        std::process::id()
    ));
    let netstat_path = std::env::temp_dir().join(format!(
        "lai-cli-port-scan-catalog-netstat-{}.txt",
        std::process::id()
    ));
    let catalog_arg = catalog_path.to_string_lossy().to_string();
    let netstat_arg = netstat_path.to_string_lossy().to_string();
    fs::write(
        &catalog_path,
        serde_json::json!({
            "profiles": [{
                "game_name": "Catalog Port Game",
                "discovery": "udp_broadcast",
                "ports": [28015],
                "compatibility": "A"
            }]
        })
        .to_string(),
    )
    .expect("write catalog");
    fs::write(
        &netstat_path,
        r#"
  Proto  Local Address          Foreign Address        State           PID
  UDP    0.0.0.0:28015          *:*                                    4242
"#,
    )
    .expect("write netstat output");

    let value = run_cli(&[
        "game-port-scan",
        "--catalog",
        &catalog_arg,
        "--game-name",
        "catalog port game",
        "--netstat-output",
        &netstat_arg,
        "--netstat-scan",
        "false",
    ]);
    fs::remove_file(&catalog_path).ok();
    fs::remove_file(&netstat_path).ok();

    assert_eq!(value["status"], "ok");
    assert_eq!(value["gameName"], "Catalog Port Game");
    assert_eq!(value["expectedPorts"][0], 28015);
    assert_eq!(value["matchCount"], 1);
}

#[test]
fn firewall_plan_can_use_catalog_profile_ports() {
    let catalog_path = std::env::temp_dir().join(format!(
        "lai-cli-firewall-plan-catalog-{}.json",
        std::process::id()
    ));
    let catalog_arg = catalog_path.to_string_lossy().to_string();
    fs::write(
        &catalog_path,
        serde_json::json!({
            "profiles": [{
                "game_name": "Catalog Firewall Game",
                "discovery": "udp_broadcast",
                "ports": [28016],
                "compatibility": "A"
            }]
        })
        .to_string(),
    )
    .expect("write catalog");

    let value = run_cli(&[
        "firewall-plan",
        "--catalog",
        &catalog_arg,
        "--game-name",
        "catalog firewall game",
        "--subnet",
        "10.77.12.0/24",
    ]);
    fs::remove_file(&catalog_path).ok();

    assert_eq!(value["dry_run"], true);
    assert!(value["commands"]
        .as_array()
        .unwrap()
        .iter()
        .any(|command| command["command"].as_str().unwrap().contains("28016")));
}

#[test]
fn game_readiness_combines_network_report_and_netstat_ports() {
    let network = run_cli(&[
        "network-observe",
        "--adapter-name",
        "LocalAreaInterconnection",
        "--expected-ip",
        "10.77.12.2",
        "--assigned-ip",
        "10.77.12.2",
        "--subnet",
        "10.77.12.0/24",
        "--tunnel-state",
        "connected",
        "--connected-peers",
        "1",
        "--expected-peers",
        "1",
        "--packets",
        "udp:10.77.12.2:10.77.12.255:27015:broadcast:outbound:8,udp:10.77.12.2:10.77.12.1:27015:unicast:outbound:8",
        "--broadcast-ports",
        "27015",
        "--game-ports",
        "27015",
    ]);
    let network_path = std::env::temp_dir().join(format!(
        "lai-cli-game-readiness-network-{}.json",
        std::process::id()
    ));
    let netstat_path = std::env::temp_dir().join(format!(
        "lai-cli-game-readiness-netstat-{}.txt",
        std::process::id()
    ));
    let firewall_path = std::env::temp_dir().join(format!(
        "lai-cli-game-readiness-firewall-{}.txt",
        std::process::id()
    ));
    let network_arg = network_path.to_string_lossy().to_string();
    let netstat_arg = netstat_path.to_string_lossy().to_string();
    let firewall_arg = firewall_path.to_string_lossy().to_string();
    fs::write(
        &network_path,
        serde_json::to_string_pretty(&network).unwrap(),
    )
    .expect("write network report");
    fs::write(
        &netstat_path,
        r#"
  Proto  Local Address          Foreign Address        State           PID
  UDP    0.0.0.0:27015          *:*                                    4242
"#,
    )
    .expect("write netstat output");
    fs::write(
        &firewall_path,
        r#"
Rule Name:                            Example UDP 27015
Enabled:                              Yes
Direction:                            In
Profiles:                             Private
RemoteIP:                             10.77.12.0/24
Protocol:                             UDP
LocalPort:                            27015
Action:                               Allow

Rule Name:                            Example TCP 27015
Enabled:                              Yes
Direction:                            In
Profiles:                             Private
RemoteIP:                             10.77.12.0/24
Protocol:                             TCP
LocalPort:                            27015
Action:                               Allow
"#,
    )
    .expect("write firewall output");

    let value = run_cli(&[
        "game-readiness",
        "--network-report",
        &network_arg,
        "--game-name",
        "Example Game",
        "--subnet",
        "10.77.12.0/24",
        "--discovery",
        "udp_broadcast",
        "--ports",
        "27015",
        "--compatibility",
        "A",
        "--firewall-netsh-output",
        &firewall_arg,
        "--netstat-output",
        &netstat_arg,
        "--netstat-scan",
        "false",
    ]);
    fs::remove_file(&network_path).ok();
    fs::remove_file(&netstat_path).ok();
    fs::remove_file(&firewall_path).ok();

    assert_eq!(value["status"], "ready");
    assert_eq!(value["report"]["game_name"], "Example Game");
    assert_eq!(value["firewallReport"]["status"], "ok");
    assert_eq!(value["matchCount"], 1);
    assert!(value["report"]["checks"]
        .as_array()
        .unwrap()
        .iter()
        .any(|check| check["key"] == "game-port-binding" && check["status"] == "ok"));
    assert!(value["report"]["checks"]
        .as_array()
        .unwrap()
        .iter()
        .any(|check| check["key"] == "firewall" && check["status"] == "ok"));
}

#[test]
fn game_readiness_uses_relay_connection_path_when_p2p_is_missing() {
    let network = run_cli(&[
        "network-observe",
        "--adapter-name",
        "LocalAreaInterconnection",
        "--expected-ip",
        "10.77.12.2",
        "--assigned-ip",
        "10.77.12.2",
        "--subnet",
        "10.77.12.0/24",
        "--tunnel-state",
        "connected",
        "--connected-peers",
        "0",
        "--expected-peers",
        "1",
        "--packets",
        "udp:10.77.12.2:10.77.12.255:27015:broadcast:outbound:8,udp:10.77.12.2:10.77.12.1:27015:unicast:outbound:8",
        "--broadcast-ports",
        "27015",
        "--game-ports",
        "27015",
    ]);
    let network_path = std::env::temp_dir().join(format!(
        "lai-cli-game-readiness-relay-network-{}.json",
        std::process::id()
    ));
    let netstat_path = std::env::temp_dir().join(format!(
        "lai-cli-game-readiness-relay-netstat-{}.txt",
        std::process::id()
    ));
    let network_arg = network_path.to_string_lossy().to_string();
    let netstat_arg = netstat_path.to_string_lossy().to_string();
    fs::write(
        &network_path,
        serde_json::to_string_pretty(&network).unwrap(),
    )
    .expect("write network report");
    fs::write(
        &netstat_path,
        r#"
  Proto  Local Address          Foreign Address        State           PID
  UDP    0.0.0.0:27015          *:*                                    4242
"#,
    )
    .expect("write netstat output");
    let local_offer = serde_json::json!({
        "schema_version": 1,
        "room_id": "room_test",
        "peer_id": "peer_a",
        "nonce": "nonce-a",
        "created_at_ms": 1,
        "candidates": [{
            "candidate_type": "host",
            "transport": "udp",
            "endpoint": "10.0.0.2:39090",
            "priority": 100,
            "source": "local"
        }]
    })
    .to_string();
    let remote_offer = serde_json::json!({
        "schema_version": 1,
        "room_id": "room_test",
        "peer_id": "peer_b",
        "nonce": "nonce-b",
        "created_at_ms": 1,
        "candidates": [
            {
                "candidate_type": "srflx",
                "transport": "udp",
                "endpoint": "198.51.100.20:44000",
                "priority": 90,
                "source": "stun"
            },
            {
                "candidate_type": "relay",
                "transport": "udp",
                "endpoint": "203.0.113.10:39090",
                "priority": 10,
                "source": "relay"
            }
        ]
    })
    .to_string();

    let value = run_cli(&[
        "game-readiness",
        "--network-report",
        &network_arg,
        "--game-name",
        "Relay Game",
        "--subnet",
        "10.77.12.0/24",
        "--discovery",
        "udp_broadcast",
        "--ports",
        "27015",
        "--compatibility",
        "A",
        "--netstat-output",
        &netstat_arg,
        "--netstat-scan",
        "false",
        "--relay-local-offer",
        &local_offer,
        "--relay-remote-offer",
        &remote_offer,
        "--relay-p2p-status",
        "failed",
    ]);
    fs::remove_file(&network_path).ok();
    fs::remove_file(&netstat_path).ok();

    assert_eq!(value["status"], "ready-to-try");
    assert_eq!(value["connectionPathReport"]["selected_path"], "relay");
    assert!(value["report"]["checks"]
        .as_array()
        .unwrap()
        .iter()
        .any(|check| check["key"] == "p2p" && check["status"] == "pending"));
    assert!(value["report"]["checks"]
        .as_array()
        .unwrap()
        .iter()
        .any(|check| { check["key"] == "connection-path" && check["status"] == "pending" }));
}

#[test]
fn game_readiness_can_use_catalog_profile_ports() {
    let network = run_cli(&[
        "network-observe",
        "--adapter-name",
        "LocalAreaInterconnection",
        "--expected-ip",
        "10.77.12.2",
        "--assigned-ip",
        "10.77.12.2",
        "--subnet",
        "10.77.12.0/24",
        "--tunnel-state",
        "connected",
        "--connected-peers",
        "1",
        "--expected-peers",
        "1",
        "--packets",
        "udp:10.77.12.2:10.77.12.255:27016:broadcast:outbound:8,udp:10.77.12.2:10.77.12.1:27016:unicast:outbound:8",
        "--broadcast-ports",
        "27016",
        "--game-ports",
        "27016",
    ]);
    let network_path = std::env::temp_dir().join(format!(
        "lai-cli-game-readiness-catalog-network-{}.json",
        std::process::id()
    ));
    let catalog_path = std::env::temp_dir().join(format!(
        "lai-cli-game-readiness-catalog-{}.json",
        std::process::id()
    ));
    let netstat_path = std::env::temp_dir().join(format!(
        "lai-cli-game-readiness-catalog-netstat-{}.txt",
        std::process::id()
    ));
    let network_arg = network_path.to_string_lossy().to_string();
    let catalog_arg = catalog_path.to_string_lossy().to_string();
    let netstat_arg = netstat_path.to_string_lossy().to_string();
    fs::write(
        &network_path,
        serde_json::to_string_pretty(&network).unwrap(),
    )
    .expect("write network report");
    fs::write(
        &catalog_path,
        serde_json::json!({
            "profiles": [{
                "game_name": "Catalog Game",
                "steam_app_id": "424242",
                "discovery": "udp_broadcast",
                "ports": [27016],
                "compatibility": "A"
            }]
        })
        .to_string(),
    )
    .expect("write catalog");
    fs::write(
        &netstat_path,
        r#"
  Proto  Local Address          Foreign Address        State           PID
  UDP    0.0.0.0:27016          *:*                                    4242
"#,
    )
    .expect("write netstat output");

    let value = run_cli(&[
        "game-readiness",
        "--network-report",
        &network_arg,
        "--catalog",
        &catalog_arg,
        "--game-name",
        "catalog game",
        "--subnet",
        "10.77.12.0/24",
        "--netstat-output",
        &netstat_arg,
        "--netstat-scan",
        "false",
    ]);
    fs::remove_file(&network_path).ok();
    fs::remove_file(&catalog_path).ok();
    fs::remove_file(&netstat_path).ok();

    assert_eq!(value["status"], "ready");
    assert_eq!(value["report"]["game_name"], "Catalog Game");
    assert_eq!(value["gamePlan"]["game_name"], "Catalog Game");
    assert_eq!(value["gamePlan"]["firewall_rules"][0]["port"], 27016);
    assert_eq!(value["matchCount"], 1);
}

#[test]
fn room_runtime_run_publishes_coordination_offer_from_runtime_socket() {
    let http_probe = TcpListener::bind("127.0.0.1:0").expect("free local port");
    let server_addr = http_probe.local_addr().unwrap();
    drop(http_probe);
    let store_path = std::env::temp_dir().join(format!(
        "lai-cli-runtime-publish-http-store-{}.json",
        std::process::id()
    ));
    let store_path_string = store_path.display().to_string();
    fs::remove_file(&store_path).ok();

    let mut server = Command::new(env!("CARGO_BIN_EXE_lai-cli"))
        .args([
            "coordination-http-serve",
            "--bind",
            &server_addr.to_string(),
            "--store",
            &store_path_string,
            "--max-requests",
            "2",
            "--request-timeout-ms",
            "3000",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn coordination http server");
    std::thread::sleep(Duration::from_millis(100));

    let server_url = format!("http://{server_addr}");
    let value = run_cli(&[
        "room-runtime-run",
        "--room-id",
        "room_publish_socket",
        "--peer-id",
        "peer_a",
        "--virtual-ip",
        "10.77.12.2",
        "--bind",
        "127.0.0.1:0",
        "--key",
        "test-room-key",
        "--duration-ms",
        "1600",
        "--peer-timeout-ms",
        "0",
        "--coordination-server",
        &server_url,
        "--coordination-publish-ttl-ms",
        "3000",
    ]);
    let server_output = match server.try_wait().expect("server wait check") {
        Some(_) => server.wait_with_output().expect("server exits"),
        None => {
            server.kill().ok();
            server.wait_with_output().expect("server killed")
        }
    };
    fs::remove_file(&store_path).ok();

    assert_eq!(value["status"], "ok");
    assert_eq!(value["runtimePublishedOffer"]["status"], "ok");
    assert_eq!(
        value["runtimePublishedOffer"]["localEndpoint"],
        value["actualTunnelEndpoint"]
    );
    assert_eq!(
        value["runtimePublishedOffer"]["offer"]["candidates"][0]["endpoint"],
        value["actualTunnelEndpoint"]
    );
    assert_eq!(
        value["coordinationPublishReports"][0]["status"], "ok",
        "{}",
        value
    );
    assert_eq!(
        value["coordinationPublishReports"][0]["localEndpoint"],
        value["actualTunnelEndpoint"]
    );
    let _ = server_output;
}

#[test]
fn room_runtime_run_can_inject_to_explicit_udp_target() {
    let listener = UdpSocket::bind("127.0.0.1:0").expect("inject listener");
    listener
        .set_read_timeout(Some(Duration::from_millis(1000)))
        .expect("listener timeout");
    let inject_target = listener.local_addr().unwrap().to_string();
    let value = run_cli(&[
        "room-runtime-run",
        "--room-id",
        "room_test",
        "--peer-id",
        "peer_a",
        "--virtual-ip",
        "10.77.12.2",
        "--bind",
        "127.0.0.1:0",
        "--key",
        "test-room-key",
        "--broadcast-ports",
        "0",
        "--duration-ms",
        "150",
        "--capture-self-probe",
        "true",
        "--forward-self-probe",
        "true",
        "--inject-target",
        &inject_target,
    ]);

    assert_eq!(value["status"], "ok");
    assert_eq!(value["injectTarget"], inject_target);
    assert_eq!(value["injectedPackets"].as_array().unwrap().len(), 1);
    assert_eq!(
        value["injectedReceivedPackets"].as_array().unwrap().len(),
        0
    );

    let mut buffer = [0u8; 64];
    let (received, _) = listener.recv_from(&mut buffer).expect("injected payload");
    assert_eq!(&buffer[..received], b"runtime-capture-probe");
}

#[test]
fn room_runtime_run_can_forward_raw_ipv4_packet_payloads() {
    let value = run_cli(&[
        "room-runtime-run",
        "--room-id",
        "room_test",
        "--peer-id",
        "peer_a",
        "--virtual-ip",
        "10.77.12.2",
        "--bind",
        "127.0.0.1:0",
        "--key",
        "test-room-key",
        "--game-ports",
        "0",
        "--broadcast-ports",
        "0",
        "--duration-ms",
        "180",
        "--self-probe",
        "true",
        "--capture-self-probe",
        "true",
        "--forward-self-probe",
        "true",
        "--inject-self-probe",
        "true",
        "--packet-io-backend",
        "wintun",
        "--forward-raw-ipv4",
        "true",
    ]);

    assert_eq!(value["status"], "ok");
    assert_eq!(value["packetIoPlan"]["backend"], "wintun");
    assert_eq!(value["packetIoPlan"]["can_read_ipv4"], true);
    assert!(value["packetIoProbe"]["status"].is_string());
    assert_eq!(value["packetIoProbe"]["backend"], "wintun");
    assert!(value["packetIoProbe"]["sessionProbe"]["status"].is_string());
    assert!(value["packetIoProbe"]["receiveProbe"]["status"].is_string());
    assert_eq!(
        value["packetIoProbe"]["sendProbe"]["status"],
        "not-run-needs-confirmation"
    );
    assert!(value["adapterReadStatus"].is_string());
    assert!(value["adapterWriteStatus"].is_string());
    assert_eq!(value["forwardRawIpv4"], true);
    assert_eq!(value["rawVirtualPackets"].as_array().unwrap().len(), 1);
    assert_eq!(value["rawVirtualPackets"][0]["sourceIp"], "10.77.12.2");
    assert_eq!(
        value["rawVirtualPackets"][0]["destinationIp"],
        "10.77.12.255"
    );
    assert_eq!(value["rawVirtualPackets"][0]["payloadBytes"], 21);
    assert!(value["packetObservationLines"]
        .as_array()
        .unwrap()
        .iter()
        .any(|line| line.as_str().unwrap().contains(":virtual-adapter:21")));
    assert!(
        value["forwardedPackets"][0]["rawIpv4PacketBytes"]
            .as_u64()
            .unwrap()
            >= 29
    );
    assert!(
        value["injectedPackets"][0]["rawIpv4PacketBytes"]
            .as_u64()
            .unwrap()
            >= 29
    );
}

#[test]
fn room_runtime_run_replies_to_icmp_echo_request_inside_tunnel() {
    let fake_peer = UdpSocket::bind("127.0.0.1:0").expect("bind fake peer");
    fake_peer
        .set_read_timeout(Some(Duration::from_millis(1500)))
        .unwrap();
    let runtime_probe = UdpSocket::bind("127.0.0.1:0").expect("reserve runtime port");
    let runtime_addr = runtime_probe.local_addr().unwrap();
    drop(runtime_probe);
    let room_id = "room_icmp_echo";
    let key = "test-room-key";

    let runtime = Command::new(env!("CARGO_BIN_EXE_lai-cli"))
        .args([
            "room-runtime-run",
            "--room-id",
            room_id,
            "--peer-id",
            "peer_b",
            "--virtual-ip",
            "10.77.12.3",
            "--bind",
            &runtime_addr.to_string(),
            "--key",
            key,
            "--duration-ms",
            "900",
            "--heartbeat-interval-ms",
            "200",
            "--peer-timeout-ms",
            "0",
            "--peer",
            &format!("peer_a,10.77.12.2,{}", fake_peer.local_addr().unwrap()),
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn runtime");
    std::thread::sleep(Duration::from_millis(80));

    let request = test_icmp_echo_request([10, 77, 12, 2], [10, 77, 12, 3]);
    let forward_payload = serde_json::json!({
        "room_id": room_id,
        "peer_id": "peer_a",
        "kind": "runtime-ipv4-forward",
        "source": "10.77.12.2",
        "destination": "10.77.12.3",
        "broadcast": false,
        "payload_encoding": "raw-ipv4",
        "raw_ipv4_packet": STANDARD_NO_PAD.encode(&request),
        "raw_ipv4_packet_bytes": request.len(),
        "ipv4_protocol": "icmp",
        "ipv4_protocol_number": 1,
    });
    let sealed = run_cli(&[
        "tunnel-seal",
        "--key",
        key,
        "--packet-kind",
        "runtime-ipv4-forward",
        "--sequence",
        "1",
        "--message",
        &serde_json::to_string(&forward_payload).unwrap(),
    ]);
    let wire = serde_json::to_vec(&sealed).unwrap();
    fake_peer
        .send_to(&wire, runtime_addr)
        .expect("send echo request");

    let mut buffer = [0u8; 4096];
    let mut reply_bytes = None;
    for _ in 0..8 {
        let (received, _) = fake_peer.recv_from(&mut buffer).expect("runtime reply");
        let envelope = std::str::from_utf8(&buffer[..received]).unwrap().to_owned();
        let opened = run_cli(&["tunnel-open", "--key", key, "--envelope", &envelope]);
        if opened["metadata"]["packet_kind"] != "runtime-ipv4-forward" {
            continue;
        }
        let message: Value =
            serde_json::from_str(opened["message"].as_str().unwrap()).expect("forward json");
        if message["icmp_echo_reply"] != true {
            continue;
        }
        let encoded = message["raw_ipv4_packet"].as_str().unwrap();
        reply_bytes = Some(STANDARD_NO_PAD.decode(encoded).unwrap());
        break;
    }
    let reply = reply_bytes.expect("icmp echo reply");
    let summary = lai_core::parse_ipv4_packet_summary(&reply).unwrap();

    let output = runtime.wait_with_output().expect("runtime exits");
    assert!(
        output.status.success(),
        "runtime failed\nstatus: {}\nstdout: {}\nstderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let value: Value = serde_json::from_slice(&output.stdout).expect("runtime json");

    assert_eq!(summary.protocol, "icmp");
    assert_eq!(summary.source_ip.to_string(), "10.77.12.3");
    assert_eq!(summary.destination_ip.to_string(), "10.77.12.2");
    assert_eq!(reply[20], 0);
    assert_eq!(reply[21], 0);
    assert_eq!(&reply[24..26], &0x1234u16.to_be_bytes());
    assert_eq!(&reply[26..28], &7u16.to_be_bytes());
    assert_eq!(&reply[28..], b"hello");
    assert_eq!(value["icmpEchoReplies"].as_array().unwrap().len(), 1);
    assert_eq!(value["packetPathCounters"]["icmpEchoRepliesSent"], 1);
    assert_eq!(value["packetPathCounters"]["rawVirtualPacketsReceived"], 1);
    assert!(
        value["packetPathCounters"]["forwardedPacketsSent"]
            .as_u64()
            .unwrap_or_default()
            >= 1
    );
    assert!(value["forwardedPackets"]
        .as_array()
        .unwrap()
        .iter()
        .any(|packet| {
            packet["packetIoBackend"] == "icmp-responder" && packet["protocol"] == "icmp"
        }));
}

#[test]
fn room_runtime_wintun_probe_send_requires_explicit_runtime_flag() {
    let default_value = run_cli(&[
        "room-runtime-run",
        "--room-id",
        "room_test",
        "--peer-id",
        "peer_a",
        "--virtual-ip",
        "10.77.12.2",
        "--bind",
        "127.0.0.1:0",
        "--key",
        "test-room-key",
        "--duration-ms",
        "50",
        "--packet-io-backend",
        "wintun",
    ]);
    let confirmed_value = run_cli(&[
        "room-runtime-run",
        "--room-id",
        "room_test",
        "--peer-id",
        "peer_a",
        "--virtual-ip",
        "10.77.12.2",
        "--bind",
        "127.0.0.1:0",
        "--key",
        "test-room-key",
        "--duration-ms",
        "50",
        "--packet-io-backend",
        "wintun",
        "--wintun-probe-send",
        "true",
    ]);

    assert_eq!(
        default_value["packetIoProbe"]["sendProbe"]["status"],
        "not-run-needs-confirmation"
    );
    assert_ne!(
        confirmed_value["packetIoProbe"]["sendProbe"]["status"],
        "not-run-needs-confirmation"
    );
    assert!(confirmed_value["packetIoProbe"]["sendProbe"]["packet_sent"].is_boolean());
}

#[test]
fn room_runtime_wintun_runtime_reports_session_state() {
    let value = run_cli(&[
        "room-runtime-run",
        "--room-id",
        "room_test",
        "--peer-id",
        "peer_a",
        "--virtual-ip",
        "10.77.12.2",
        "--bind",
        "127.0.0.1:0",
        "--key",
        "test-room-key",
        "--duration-ms",
        "50",
        "--packet-io-backend",
        "wintun",
        "--wintun-runtime",
        "true",
    ]);

    assert_eq!(value["wintunRuntime"]["enabled"], true);
    assert!(value["wintunRuntime"]["open"]["status"].is_string());
    assert!(value["wintunRuntime"]["close"]["closed"].is_boolean());
    assert!(value["wintunRuntime"]["receivedPackets"].is_array());
    assert!(value["wintunRuntime"]["sentPackets"].is_array());
    assert!(value["wintunRuntime"]["errors"].is_array());
}

#[test]
fn room_runtime_run_emits_periodic_heartbeats_and_snapshots() {
    let snapshot_path =
        std::env::temp_dir().join(format!("lai-runtime-periodic-{}.json", std::process::id()));
    fs::remove_file(&snapshot_path).ok();
    let snapshot_arg = snapshot_path.to_string_lossy().to_string();
    let value = run_cli(&[
        "room-runtime-run",
        "--room-id",
        "room_test",
        "--peer-id",
        "peer_a",
        "--virtual-ip",
        "10.77.12.2",
        "--bind",
        "127.0.0.1:0",
        "--key",
        "test-room-key",
        "--duration-ms",
        "180",
        "--self-probe",
        "true",
        "--heartbeat-interval-ms",
        "40",
        "--snapshot-out",
        &snapshot_arg,
        "--snapshot-interval-ms",
        "40",
    ]);

    assert_eq!(value["status"], "ok");
    assert_eq!(value["stopReason"], "duration");
    assert!(value["heartbeatPacketsSent"].as_u64().unwrap() >= 2);
    assert!(value["heartbeatAckPackets"]
        .as_array()
        .unwrap()
        .iter()
        .any(|packet| packet["direction"] == "received" && packet["roundTripMs"].is_number()));
    assert!(
        value["runtimePeerSummaries"][0]["heartbeatLossWindowSize"]
            .as_u64()
            .unwrap()
            >= 1
    );
    assert!(value["runtimePeerSummaries"][0]["heartbeatLossWindowPercent"].is_number());
    assert!(
        value["runtimePeerSummaries"][0]["heartbeatRttSampleCount"]
            .as_u64()
            .unwrap()
            >= 1
    );
    assert_eq!(value["runtimePeerSummaries"][0]["pathKind"], "direct");
    assert!(value["runtimePeerSummaries"][0]["latencyMs"].is_number());
    assert_eq!(
        value["tunnelServiceSnapshot"]["average_latency_ms"],
        value["runtimePeerSummaries"][0]["latencyMs"]
    );
    assert_ne!(value["tunnelServiceSnapshot"]["average_latency_ms"], 180);
    assert!(
        value["runtimePeerSummaries"][0]["directBytesSent"]
            .as_u64()
            .unwrap()
            >= 1
    );
    assert_eq!(value["runtimePeerSummaries"][0]["relayBytesSent"], 0);
    assert!(value["snapshotWriteCount"].as_u64().unwrap() >= 1);
    assert!(
        value["tunnelPackets"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|packet| packet["kind"] == "runtime-heartbeat")
            .count()
            >= 2
    );
    assert!(
        value["tunnelPackets"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|packet| packet["kind"] == "runtime-heartbeat-ack")
            .count()
            >= 1
    );

    let snapshot: Value =
        serde_json::from_str(&fs::read_to_string(&snapshot_path).expect("runtime snapshot"))
            .expect("snapshot json");
    fs::remove_file(&snapshot_path).ok();
    assert!(snapshot["snapshotWriteCount"].is_number());
    assert!(snapshot["packetPathCounters"]["tunnelPacketsReceived"].is_number());
    assert!(snapshot["packetPathCounters"]["forwardedPacketsSent"].is_number());
}

#[test]
fn room_runtime_run_stops_when_stop_file_appears() {
    let stop_path =
        std::env::temp_dir().join(format!("lai-runtime-stop-{}.txt", std::process::id()));
    fs::remove_file(&stop_path).ok();
    let stop_arg = stop_path.to_string_lossy().to_string();
    let child = Command::new(env!("CARGO_BIN_EXE_lai-cli"))
        .args([
            "room-runtime-run",
            "--room-id",
            "room_test",
            "--peer-id",
            "peer_a",
            "--virtual-ip",
            "10.77.12.2",
            "--bind",
            "127.0.0.1:0",
            "--key",
            "test-room-key",
            "--duration-ms",
            "5000",
            "--self-probe",
            "true",
            "--heartbeat-interval-ms",
            "50",
            "--stop-file",
            &stop_arg,
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn runtime");

    std::thread::sleep(Duration::from_millis(140));
    fs::write(&stop_path, "stop").expect("write stop file");
    let output = child.wait_with_output().expect("runtime exits");
    fs::remove_file(&stop_path).ok();

    assert!(
        output.status.success(),
        "lai-cli failed\nstatus: {}\nstdout: {}\nstderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let value: Value = serde_json::from_slice(&output.stdout).expect("valid json stdout");
    assert_eq!(value["status"], "ok");
    assert_eq!(value["stopReason"], "stop-file");
    assert!(value["heartbeatPacketsSent"].as_u64().unwrap() >= 1);
}

#[test]
fn adapter_apply_without_yes_returns_admin_guidance() {
    let value = run_cli(&[
        "adapter-apply",
        "--adapter-name",
        "LocalAreaInterconnection",
        "--subnet",
        "10.77.12.0/24",
        "--ip",
        "10.77.12.2",
    ]);

    assert_eq!(value["requires_elevation"], true);
    assert_eq!(value["confirmed"], false);
    assert_eq!(value["can_execute_now"], false);
    assert_eq!(value["commands"][0]["status"], "SkippedNeedsConfirmation");
    assert!(value["next_action"]
        .as_str()
        .unwrap()
        .contains("--yes true"));
}

#[test]
fn adapter_ensure_reports_ready_from_netsh_output_file() {
    let path = std::env::temp_dir().join(format!("lai-adapter-netsh-{}.txt", std::process::id()));
    fs::write(
        &path,
        r#"
Configuration for interface "LocalAreaInterconnection"
    DHCP enabled:                         No
    IP Address:                           10.77.12.2
    Subnet Prefix:                        10.77.12.0/24 (mask 255.255.255.0)
    MTU:                                  1420
    InterfaceMetric:                      5
"#,
    )
    .expect("write adapter sample");
    let path_arg = path.to_string_lossy().to_string();
    let value = run_cli(&[
        "adapter-ensure",
        "--adapter-name",
        "LocalAreaInterconnection",
        "--subnet",
        "10.77.12.0/24",
        "--ip",
        "10.77.12.2",
        "--adapter-netsh-output",
        &path_arg,
        "--adapter-scan",
        "false",
    ]);
    fs::remove_file(&path).ok();

    assert_eq!(value["status"], "ready");
    assert_eq!(value["ready"], true);
    assert!(value["checks"]
        .as_array()
        .unwrap()
        .iter()
        .all(|check| check["status"] == "ok"));
    assert_eq!(value["commandResults"].as_array().unwrap().len(), 0);
}

#[test]
fn adapter_ensure_without_observation_requires_admin_apply() {
    let value = run_cli(&[
        "adapter-ensure",
        "--adapter-name",
        "LocalAreaInterconnection",
        "--subnet",
        "10.77.12.0/24",
        "--ip",
        "10.77.12.2",
        "--adapter-scan",
        "false",
    ]);

    assert_eq!(value["status"], "needs-apply");
    assert_eq!(value["ready"], false);
    assert_eq!(value["checks"][0]["status"], "missing");
    assert!(value["nextAction"].as_str().unwrap().contains("--yes true"));
}

#[test]
fn virtual_packet_plan_reports_userspace_and_raw_backends() {
    let userspace = run_cli(&[
        "virtual-packet-plan",
        "--adapter-name",
        "LocalAreaInterconnection",
        "--backend",
        "userspace-udp",
    ]);
    let wintun = run_cli(&[
        "virtual-packet-plan",
        "--adapter-name",
        "LocalAreaInterconnection",
        "--backend",
        "wintun",
    ]);

    assert_eq!(userspace["backend"], "userspace-udp");
    assert_eq!(userspace["can_read_ipv4"], false);
    assert_eq!(userspace["can_observe_udp"], true);
    assert_eq!(wintun["can_read_ipv4"], true);
    assert_eq!(wintun["can_write_ipv4"], true);
}

#[test]
fn virtual_packet_loopback_builds_and_parses_ipv4_udp() {
    let value = run_cli(&[
        "virtual-packet-loopback-test",
        "--source-ip",
        "10.77.12.2",
        "--destination-ip",
        "10.77.12.255",
        "--source-port",
        "39077",
        "--destination-port",
        "27015",
        "--message",
        "discover",
    ]);

    assert_eq!(value["status"], "ok");
    assert_eq!(value["packet"]["source_ip"], "10.77.12.2");
    assert_eq!(value["packet"]["destination_ip"], "10.77.12.255");
    assert_eq!(value["packet"]["broadcast"], true);
    assert!(value["packetObservationLine"]
        .as_str()
        .unwrap()
        .contains(":virtual-adapter:8"));
}

#[test]
fn virtual_packet_build_output_can_be_parsed() {
    let built = run_cli(&[
        "virtual-packet-build-udp",
        "--source-ip",
        "10.77.12.2",
        "--destination-ip",
        "10.77.12.3",
        "--source-port",
        "39077",
        "--destination-port",
        "27015",
        "--message",
        "hello",
    ]);
    let packet_base64 = built["packetBase64"].as_str().unwrap();
    let parsed = run_cli(&["virtual-packet-parse", "--packet-base64", packet_base64]);

    assert_eq!(parsed["status"], "ok");
    assert_eq!(parsed["packet"]["destination_ip"], "10.77.12.3");
    assert_eq!(parsed["packet"]["payload"], built["packet"]["payload"]);
    assert_eq!(parsed["packet"]["broadcast"], false);
}

#[test]
fn virtual_packet_tcp_build_output_has_summary() {
    let built = run_cli(&[
        "virtual-packet-build-tcp",
        "--source-ip",
        "10.77.12.2",
        "--destination-ip",
        "10.77.12.3",
        "--source-port",
        "50123",
        "--destination-port",
        "27015",
        "--message",
        "hello tcp",
    ]);
    let packet_base64 = built["packetBase64"].as_str().unwrap();
    let summary = run_cli(&[
        "virtual-packet-parse-summary",
        "--packet-base64",
        packet_base64,
    ]);

    assert_eq!(built["status"], "ok");
    assert!(built["packetObservationLine"]
        .as_str()
        .unwrap()
        .starts_with("tcp:10.77.12.2:10.77.12.3:27015:unicast:virtual-adapter:"));
    assert_eq!(summary["status"], "ok");
    assert_eq!(summary["summary"]["protocol"], "tcp");
    assert_eq!(summary["summary"]["destination_port"], 27015);
    assert_eq!(summary["summary"]["payload_bytes"], 9);
}

#[test]
fn encrypted_tunnel_loopback_round_trips_message() {
    let value = run_cli(&[
        "tunnel-loopback-test",
        "--key",
        "test-room-key",
        "--message",
        "hello tunnel",
    ]);

    assert_eq!(value["status"], "ok");
    assert_eq!(value["message"], "hello tunnel");
    assert_eq!(value["metadata"]["packet_kind"], "loopback-test");
}

#[test]
fn encrypted_p2p_handshake_loopback_accepts_peer() {
    let value = run_cli(&[
        "p2p-handshake-loopback-test",
        "--key",
        "test-room-key",
        "--virtual-ip",
        "10.77.12.2",
        "--room-id",
        "room_test",
        "--peer-id",
        "peer_a",
        "--responder-peer-id",
        "peer_b",
    ]);

    assert_eq!(value["status"], "ok");
    assert_eq!(value["roomId"], "room_test");
    assert_eq!(value["peerId"], "peer_a");
    assert_eq!(value["responderPeerId"], "peer_b");
    assert_eq!(value["virtualIp"], "10.77.12.2");
    assert_eq!(value["nonceMatched"], true);
}

#[test]
fn encrypted_p2p_handshake_send_and_listen_exchange_ack() {
    let probe = UdpSocket::bind("127.0.0.1:0").expect("free udp port");
    let listener_addr = probe.local_addr().unwrap();
    drop(probe);

    let listener = Command::new(env!("CARGO_BIN_EXE_lai-cli"))
        .args([
            "p2p-handshake-listen",
            "--bind",
            &listener_addr.to_string(),
            "--key",
            "test-room-key",
            "--responder-peer-id",
            "peer_b",
            "--max-packets",
            "1",
            "--timeout-ms",
            "2000",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn p2p listener");
    std::thread::sleep(Duration::from_millis(80));

    let sender = run_cli(&[
        "p2p-handshake-send",
        "--peer",
        &listener_addr.to_string(),
        "--key",
        "test-room-key",
        "--virtual-ip",
        "10.77.12.2",
        "--room-id",
        "room_test",
        "--peer-id",
        "peer_a",
        "--timeout-ms",
        "2000",
    ]);
    let listener_output = listener.wait_with_output().expect("listener exits");

    assert_eq!(sender["status"], "ok");
    assert_eq!(sender["responderPeerId"], "peer_b");
    assert_eq!(sender["nonceMatched"], true);
    assert!(
        listener_output.status.success(),
        "listener failed\nstatus: {}\nstdout: {}\nstderr: {}",
        listener_output.status,
        String::from_utf8_lossy(&listener_output.stdout),
        String::from_utf8_lossy(&listener_output.stderr)
    );
    let listener_json: Value =
        serde_json::from_slice(&listener_output.stdout).expect("listener json");
    assert_eq!(listener_json["status"], "ok");
    assert_eq!(listener_json["handshakes"][0]["peerId"], "peer_a");
}

#[test]
fn nat_candidates_and_plan_exchange_endpoints() {
    let local = run_cli(&[
        "nat-candidates",
        "--room-id",
        "room_test",
        "--peer-id",
        "peer_a",
        "--bind",
        "127.0.0.1:0",
        "--nonce",
        "nonce-a",
    ]);
    let remote = run_cli(&[
        "nat-candidates",
        "--room-id",
        "room_test",
        "--peer-id",
        "peer_b",
        "--bind",
        "127.0.0.1:0",
        "--nonce",
        "nonce-b",
    ]);
    let local_offer = serde_json::to_string(&local["offer"]).unwrap();
    let remote_offer = serde_json::to_string(&remote["offer"]).unwrap();
    let plan = run_cli(&[
        "nat-plan",
        "--local-offer",
        &local_offer,
        "--remote-offer",
        &remote_offer,
        "--attempts",
        "3",
        "--interval-ms",
        "10",
    ]);

    assert_eq!(local["status"], "ok");
    assert_eq!(local["offer"]["schema_version"], 1);
    assert_eq!(
        local["coordinationMessage"]["message_type"],
        "candidate-offer"
    );
    assert_eq!(plan["status"], "ready");
    assert_eq!(plan["attempt_count"], 3);
    assert!(plan["target_endpoints"].as_array().unwrap().len() >= 1);
}

#[test]
fn nat_plan_orders_routable_candidates_before_private_host_and_excludes_relay() {
    let local_offer = serde_json::json!({
        "schema_version": 1,
        "room_id": "room_test",
        "peer_id": "peer_a",
        "virtual_ip": "10.77.12.2",
        "nonce": "nonce-a",
        "created_at_ms": 1,
        "candidates": [{
            "candidate_type": "host",
            "transport": "udp",
            "endpoint": "10.0.0.2:39090",
            "priority": 100,
            "source": "local-socket"
        }]
    });
    let remote_offer = serde_json::json!({
        "schema_version": 1,
        "room_id": "room_test",
        "peer_id": "peer_b",
        "virtual_ip": "10.77.12.3",
        "nonce": "nonce-b",
        "created_at_ms": 1,
        "candidates": [
            {
                "candidate_type": "host",
                "transport": "udp",
                "endpoint": "192.168.1.20:39090",
                "priority": 100,
                "source": "local-socket"
            },
            {
                "candidate_type": "relay",
                "transport": "udp",
                "endpoint": "203.0.113.10:39091",
                "priority": 10,
                "source": "relay"
            },
            {
                "candidate_type": "srflx",
                "transport": "udp",
                "endpoint": "198.51.100.20:44000",
                "priority": 90,
                "source": "observed-endpoint"
            },
            {
                "candidate_type": "srflx",
                "transport": "udp",
                "endpoint": "198.51.100.20:39090",
                "priority": 90,
                "source": "upnp-port-mapping"
            }
        ]
    });
    let local_offer = serde_json::to_string(&local_offer).unwrap();
    let remote_offer = serde_json::to_string(&remote_offer).unwrap();

    let plan = run_cli(&[
        "nat-plan",
        "--local-offer",
        &local_offer,
        "--remote-offer",
        &remote_offer,
    ]);

    assert_eq!(plan["status"], "ready");
    assert_eq!(
        plan["target_endpoints"],
        serde_json::json!([
            "198.51.100.20:39090",
            "198.51.100.20:44000",
            "192.168.1.20:39090"
        ])
    );
}

#[test]
fn relay_fallback_plan_selects_relay_after_p2p_failure() {
    let local = run_cli(&[
        "nat-candidates",
        "--room-id",
        "room_test",
        "--peer-id",
        "peer_a",
        "--bind",
        "127.0.0.1:0",
        "--nonce",
        "nonce-a",
    ]);
    let remote = run_cli(&[
        "nat-candidates",
        "--room-id",
        "room_test",
        "--peer-id",
        "peer_b",
        "--bind",
        "127.0.0.1:0",
        "--relay",
        "203.0.113.10:39090",
        "--nonce",
        "nonce-b",
    ]);
    let local_offer = serde_json::to_string(&local["offer"]).unwrap();
    let remote_offer = serde_json::to_string(&remote["offer"]).unwrap();

    let plan = run_cli(&[
        "relay-fallback-plan",
        "--local-offer",
        &local_offer,
        "--remote-offer",
        &remote_offer,
        "--p2p-status",
        "failed",
    ]);

    assert_eq!(plan["status"], "relay-available");
    assert_eq!(plan["local_peer_id"], "peer_a");
    assert_eq!(plan["remote_peer_id"], "peer_b");
    assert!(plan["p2p_candidate_count"].as_u64().unwrap() >= 1);
    assert_eq!(plan["relay_candidate_count"], 1);
    assert_eq!(plan["selected_relay_endpoints"][0], "203.0.113.10:39090");
    assert!(plan["recommended_actions"]
        .as_array()
        .unwrap()
        .iter()
        .any(|action| action.as_str().unwrap().contains("relay endpoint")));
}

#[test]
fn connection_path_plan_selects_relay_after_p2p_failure() {
    let local = run_cli(&[
        "nat-candidates",
        "--room-id",
        "room_test",
        "--peer-id",
        "peer_a",
        "--bind",
        "127.0.0.1:0",
        "--nonce",
        "nonce-a",
    ]);
    let remote = run_cli(&[
        "nat-candidates",
        "--room-id",
        "room_test",
        "--peer-id",
        "peer_b",
        "--bind",
        "127.0.0.1:0",
        "--observed-endpoint",
        "198.51.100.20:44000",
        "--relay",
        "203.0.113.10:39090",
        "--nonce",
        "nonce-b",
    ]);
    let local_offer = serde_json::to_string(&local["offer"]).unwrap();
    let remote_offer = serde_json::to_string(&remote["offer"]).unwrap();

    let report = run_cli(&[
        "connection-path-plan",
        "--local-offer",
        &local_offer,
        "--remote-offer",
        &remote_offer,
        "--p2p-status",
        "failed",
    ]);

    assert_eq!(report["status"], "relay-ready");
    assert_eq!(report["selected_path"], "relay");
    assert_eq!(report["remote_nat_assessment"], "nat-mapped-with-relay");
    assert_eq!(report["remote_relay_candidate_count"], 1);
    assert_eq!(report["selected_endpoints"][0], "203.0.113.10:39090");
    assert_eq!(report["relay_fallback"]["status"], "relay-available");
}

#[test]
fn connection_path_plan_prefers_p2p_before_failure() {
    let local = run_cli(&[
        "nat-candidates",
        "--room-id",
        "room_test",
        "--peer-id",
        "peer_a",
        "--bind",
        "127.0.0.1:0",
        "--nonce",
        "nonce-a",
    ]);
    let remote = run_cli(&[
        "nat-candidates",
        "--room-id",
        "room_test",
        "--peer-id",
        "peer_b",
        "--bind",
        "127.0.0.1:0",
        "--observed-endpoint",
        "198.51.100.20:44000",
        "--nonce",
        "nonce-b",
    ]);
    let local_offer = serde_json::to_string(&local["offer"]).unwrap();
    let remote_offer = serde_json::to_string(&remote["offer"]).unwrap();

    let report = run_cli(&[
        "connection-path-plan",
        "--local-offer",
        &local_offer,
        "--remote-offer",
        &remote_offer,
    ]);

    assert_eq!(report["status"], "p2p-candidate-ready");
    assert_eq!(report["selected_path"], "p2p");
    assert!(report["remote_p2p_candidate_count"].as_u64().unwrap() >= 1);
    assert!(report["selected_endpoints"].as_array().unwrap().len() >= 1);
}

#[test]
fn relay_fallback_plan_requests_relay_without_relay_candidate() {
    let local = run_cli(&[
        "nat-candidates",
        "--room-id",
        "room_test",
        "--peer-id",
        "peer_a",
        "--bind",
        "127.0.0.1:0",
        "--nonce",
        "nonce-a",
    ]);
    let remote = run_cli(&[
        "nat-candidates",
        "--room-id",
        "room_test",
        "--peer-id",
        "peer_b",
        "--bind",
        "127.0.0.1:0",
        "--nonce",
        "nonce-b",
    ]);
    let local_offer = serde_json::to_string(&local["offer"]).unwrap();
    let remote_offer = serde_json::to_string(&remote["offer"]).unwrap();

    let plan = run_cli(&[
        "relay-fallback-plan",
        "--local-offer",
        &local_offer,
        "--remote-offer",
        &remote_offer,
        "--p2p-status",
        "timeout",
    ]);

    assert_eq!(plan["status"], "needs-relay");
    assert_eq!(plan["relay_candidate_count"], 0);
    assert!(plan["selected_relay_endpoints"]
        .as_array()
        .unwrap()
        .is_empty());
}

#[test]
fn nat_candidates_can_query_stun_like_server_for_observed_endpoint() {
    let probe = UdpSocket::bind("127.0.0.1:0").expect("free udp port");
    let server_addr = probe.local_addr().unwrap();
    drop(probe);

    let server = Command::new(env!("CARGO_BIN_EXE_lai-cli"))
        .args([
            "stun-like-serve",
            "--bind",
            &server_addr.to_string(),
            "--max-requests",
            "1",
            "--timeout-ms",
            "5000",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn stun-like server");
    std::thread::sleep(Duration::from_millis(80));

    let value = run_cli(&[
        "nat-candidates",
        "--room-id",
        "room_test",
        "--peer-id",
        "peer_a",
        "--bind",
        "127.0.0.1:0",
        "--stun-server",
        &server_addr.to_string(),
        "--stun-timeout-ms",
        "2000",
        "--nonce",
        "stun-nonce",
    ]);
    let server_output = server.wait_with_output().expect("server exits");

    assert_eq!(value["status"], "ok");
    assert!(value["offer"]["candidates"]
        .as_array()
        .unwrap()
        .iter()
        .any(|candidate| candidate["candidate_type"] == "srflx"
            && candidate["source"] == "observed-endpoint"));
    assert!(
        server_output.status.success(),
        "server failed\nstatus: {}\nstdout: {}\nstderr: {}",
        server_output.status,
        String::from_utf8_lossy(&server_output.stdout),
        String::from_utf8_lossy(&server_output.stderr)
    );
    let server_json: Value =
        serde_json::from_slice(&server_output.stdout).expect("server final json");
    assert_eq!(server_json["handledRequests"], 1);
}

#[test]
fn nat_candidates_can_query_standard_stun_server_for_observed_endpoint() {
    let server = UdpSocket::bind("127.0.0.1:0").expect("bind fake stun server");
    let server_addr = server.local_addr().unwrap();
    let handle = std::thread::spawn(move || {
        let mut buffer = [0u8; 1500];
        let (received, peer) = server.recv_from(&mut buffer).expect("receive stun request");
        assert!(received >= 20, "short STUN request");
        let transaction_id = &buffer[8..20];
        let magic_cookie = 0x2112A442u32;
        let cookie_bytes = magic_cookie.to_be_bytes();
        let peer_ip = match peer.ip() {
            std::net::IpAddr::V4(ip) => ip.octets(),
            std::net::IpAddr::V6(_) => panic!("expected IPv4 peer"),
        };
        let xport = peer.port() ^ ((magic_cookie >> 16) as u16);

        let mut response = Vec::new();
        response.extend_from_slice(&0x0101u16.to_be_bytes());
        response.extend_from_slice(&12u16.to_be_bytes());
        response.extend_from_slice(&magic_cookie.to_be_bytes());
        response.extend_from_slice(transaction_id);
        response.extend_from_slice(&0x0020u16.to_be_bytes());
        response.extend_from_slice(&8u16.to_be_bytes());
        response.push(0);
        response.push(0x01);
        response.extend_from_slice(&xport.to_be_bytes());
        response.push(peer_ip[0] ^ cookie_bytes[0]);
        response.push(peer_ip[1] ^ cookie_bytes[1]);
        response.push(peer_ip[2] ^ cookie_bytes[2]);
        response.push(peer_ip[3] ^ cookie_bytes[3]);
        server.send_to(&response, peer).expect("send stun response");
    });

    let value = run_cli(&[
        "nat-candidates",
        "--room-id",
        "room_test",
        "--peer-id",
        "peer_a",
        "--bind",
        "127.0.0.1:0",
        "--stun-server",
        &format!("localhost:{}", server_addr.port()),
        "--stun-timeout-ms",
        "2000",
        "--nonce",
        "standard-stun-nonce",
    ]);
    handle.join().expect("fake stun server exits");

    assert_eq!(value["status"], "ok");
    assert!(value["offer"]["candidates"]
        .as_array()
        .unwrap()
        .iter()
        .any(|candidate| candidate["candidate_type"] == "srflx"
            && candidate["source"] == "observed-endpoint"));
}

#[test]
fn nat_candidates_reports_endpoint_dependent_stun_mapping() {
    let (server_a, handle_a) = spawn_fake_standard_stun(0);
    let (server_b, handle_b) = spawn_fake_standard_stun(1);
    let stun_servers = format!(
        "localhost:{},localhost:{}",
        server_a.port(),
        server_b.port()
    );

    let value = run_cli(&[
        "nat-candidates",
        "--room-id",
        "room_test",
        "--peer-id",
        "peer_a",
        "--bind",
        "127.0.0.1:0",
        "--stun-server",
        &stun_servers,
        "--stun-timeout-ms",
        "2000",
        "--nonce",
        "endpoint-dependent-stun-nonce",
    ]);
    handle_a.join().expect("first fake stun server exits");
    handle_b.join().expect("second fake stun server exits");

    assert_eq!(value["status"], "ok");
    assert_eq!(
        value["stunMapping"]["mappingBehavior"],
        "endpoint-dependent"
    );
    assert_eq!(
        value["stunMapping"]["observedEndpoints"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert!(
        value["offer"]["candidates"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|candidate| candidate["candidate_type"] == "srflx"
                && candidate["source"] == "observed-endpoint")
            .count()
            >= 2
    );
}

#[test]
fn nat_candidates_can_query_standard_stun_server_for_ipv6_observed_endpoint() {
    let server = UdpSocket::bind("[::1]:0").expect("bind fake IPv6 stun server");
    let server_addr = server.local_addr().unwrap();
    let handle = std::thread::spawn(move || {
        let mut buffer = [0u8; 1500];
        let (received, peer) = server.recv_from(&mut buffer).expect("receive stun request");
        assert!(received >= 20, "short STUN request");
        let transaction_id = &buffer[8..20];
        let magic_cookie = 0x2112A442u32;
        let mut mask = [0u8; 16];
        mask[..4].copy_from_slice(&magic_cookie.to_be_bytes());
        mask[4..].copy_from_slice(transaction_id);
        let peer_ip = match peer.ip() {
            std::net::IpAddr::V6(ip) => ip.octets(),
            std::net::IpAddr::V4(_) => panic!("expected IPv6 peer"),
        };
        let xport = peer.port() ^ ((magic_cookie >> 16) as u16);

        let mut response = Vec::new();
        response.extend_from_slice(&0x0101u16.to_be_bytes());
        response.extend_from_slice(&24u16.to_be_bytes());
        response.extend_from_slice(&magic_cookie.to_be_bytes());
        response.extend_from_slice(transaction_id);
        response.extend_from_slice(&0x0020u16.to_be_bytes());
        response.extend_from_slice(&20u16.to_be_bytes());
        response.push(0);
        response.push(0x02);
        response.extend_from_slice(&xport.to_be_bytes());
        for index in 0..16 {
            response.push(peer_ip[index] ^ mask[index]);
        }
        server.send_to(&response, peer).expect("send stun response");
    });

    let value = run_cli(&[
        "nat-candidates",
        "--room-id",
        "room_test",
        "--peer-id",
        "peer_a",
        "--bind",
        "[::1]:0",
        "--stun-server",
        &format!("[::1]:{}", server_addr.port()),
        "--stun-timeout-ms",
        "2000",
        "--nonce",
        "standard-ipv6-stun-nonce",
    ]);
    handle.join().expect("fake IPv6 stun server exits");

    assert_eq!(value["status"], "ok");
    assert!(value["offer"]["candidates"]
        .as_array()
        .unwrap()
        .iter()
        .any(|candidate| candidate["candidate_type"] == "srflx"
            && candidate["source"] == "observed-endpoint"
            && candidate["endpoint"]
                .as_str()
                .unwrap()
                .starts_with("[::1]:")));
}

#[test]
fn nat_candidates_do_not_publish_unspecified_host_endpoint() {
    let value = run_cli(&[
        "nat-candidates",
        "--room-id",
        "room_test",
        "--peer-id",
        "peer_a",
        "--bind",
        "0.0.0.0:0",
        "--nonce",
        "unspecified-host-nonce",
    ]);

    assert_eq!(value["status"], "ok");
    assert!(!value["offer"]["candidates"]
        .as_array()
        .unwrap()
        .iter()
        .any(|candidate| candidate["endpoint"]
            .as_str()
            .unwrap_or_default()
            .starts_with("0.0.0.0:")));
}

#[test]
fn nat_candidates_can_add_upnp_port_mapping_candidate() {
    let server = TcpListener::bind("127.0.0.1:0").expect("bind fake upnp gateway");
    let server_addr = server.local_addr().unwrap();
    let handle = std::thread::spawn(move || {
        for _ in 0..3 {
            let (mut stream, _) = server.accept().expect("accept upnp request");
            stream
                .set_read_timeout(Some(Duration::from_millis(100)))
                .expect("set fake upnp read timeout");
            let mut request_bytes = Vec::new();
            let mut buffer = [0u8; 1024];
            loop {
                match stream.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(read) => request_bytes.extend_from_slice(&buffer[..read]),
                    Err(err)
                        if matches!(
                            err.kind(),
                            std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                        ) =>
                    {
                        break;
                    }
                    Err(err) => panic!("read upnp request: {err}"),
                }
            }
            let request = String::from_utf8_lossy(&request_bytes);
            let body = if request.starts_with("GET /root.xml ") {
                r#"<?xml version="1.0"?>
<root>
  <device>
    <serviceList>
      <service>
        <serviceType>urn:schemas-upnp-org:service:WANIPConnection:1</serviceType>
        <controlURL>/control</controlURL>
      </service>
    </serviceList>
  </device>
</root>"#
                    .to_owned()
            } else if request.contains("AddPortMapping") {
                assert!(request.contains("<NewProtocol>UDP</NewProtocol>"));
                r#"<?xml version="1.0"?>
<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/">
  <s:Body><u:AddPortMappingResponse xmlns:u="urn:schemas-upnp-org:service:WANIPConnection:1" /></s:Body>
</s:Envelope>"#
                    .to_owned()
            } else if request.contains("GetExternalIPAddress") {
                r#"<?xml version="1.0"?>
<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/">
  <s:Body><u:GetExternalIPAddressResponse xmlns:u="urn:schemas-upnp-org:service:WANIPConnection:1">
    <NewExternalIPAddress>198.51.100.77</NewExternalIPAddress>
  </u:GetExternalIPAddressResponse></s:Body>
</s:Envelope>"#
                    .to_owned()
            } else {
                panic!("unexpected upnp request: {request}");
            };
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.as_bytes().len(),
                body,
            );
            stream
                .write_all(response.as_bytes())
                .expect("write upnp response");
        }
    });

    let value = run_cli(&[
        "nat-candidates",
        "--room-id",
        "room_test",
        "--peer-id",
        "peer_a",
        "--bind",
        "127.0.0.1:0",
        "--upnp-port-map",
        "true",
        "--upnp-gateway-location",
        &format!("http://{server_addr}/root.xml"),
        "--upnp-timeout-ms",
        "2000",
        "--nonce",
        "upnp-nonce",
    ]);
    handle.join().expect("fake upnp gateway exits");

    assert_eq!(value["status"], "ok");
    assert_eq!(value["upnpPortMapping"]["status"], "mapped");
    assert!(value["offer"]["candidates"]
        .as_array()
        .unwrap()
        .iter()
        .any(|candidate| candidate["candidate_type"] == "srflx"
            && candidate["source"] == "upnp-port-mapping"
            && candidate["endpoint"]
                .as_str()
                .unwrap()
                .starts_with("198.51.100.77:")));
}

#[test]
fn coordination_offer_publish_accepts_utf8_bom_offer_file() {
    let store_path = std::env::temp_dir().join(format!(
        "lai-cli-bom-coordination-store-{}.json",
        std::process::id()
    ));
    let offer_path =
        std::env::temp_dir().join(format!("lai-cli-bom-offer-{}.json", std::process::id()));
    let store_path_string = store_path.display().to_string();
    let offer_path_string = offer_path.display().to_string();
    fs::remove_file(&store_path).ok();
    fs::remove_file(&offer_path).ok();

    let local = run_cli(&[
        "nat-candidates",
        "--room-id",
        "room_test",
        "--peer-id",
        "peer_a",
        "--bind",
        "127.0.0.1:0",
        "--nonce",
        "nonce-a",
    ]);
    let offer = serde_json::to_string(&local["offer"]).unwrap();
    fs::write(&offer_path, format!("\u{feff}{offer}")).unwrap();

    let publish = run_cli(&[
        "coordination-offer-publish",
        "--store",
        &store_path_string,
        "--offer",
        &offer_path_string,
        "--ttl-ms",
        "30000",
    ]);
    fs::remove_file(&store_path).ok();
    fs::remove_file(&offer_path).ok();

    assert_eq!(publish["status"], "ok");
    assert_eq!(publish["peer_id"], "peer_a");
}

#[test]
fn coordination_store_publishes_fetches_and_heartbeats_offers() {
    let path = std::env::temp_dir().join(format!(
        "lai-cli-coordination-store-{}.json",
        std::process::id()
    ));
    let path_string = path.display().to_string();
    fs::remove_file(&path).ok();

    let init = run_cli(&["coordination-store-init", "--out", &path_string]);
    let local = run_cli(&[
        "nat-candidates",
        "--room-id",
        "room_test",
        "--peer-id",
        "peer_a",
        "--bind",
        "127.0.0.1:0",
        "--nonce",
        "nonce-a",
    ]);
    let remote = run_cli(&[
        "nat-candidates",
        "--room-id",
        "room_test",
        "--peer-id",
        "peer_b",
        "--bind",
        "127.0.0.1:0",
        "--nonce",
        "nonce-b",
    ]);
    let local_offer = serde_json::to_string(&local["offer"]).unwrap();
    let remote_offer = serde_json::to_string(&remote["offer"]).unwrap();
    let publish_a = run_cli(&[
        "coordination-offer-publish",
        "--store",
        &path_string,
        "--offer",
        &local_offer,
        "--ttl-ms",
        "30000",
    ]);
    let publish_b = run_cli(&[
        "coordination-offer-publish",
        "--store",
        &path_string,
        "--offer",
        &remote_offer,
        "--ttl-ms",
        "30000",
    ]);
    let fetch_for_a = run_cli(&[
        "coordination-offer-fetch",
        "--store",
        &path_string,
        "--room-id",
        "room_test",
        "--peer-id",
        "peer_a",
    ]);
    let heartbeat = run_cli(&[
        "coordination-heartbeat",
        "--store",
        &path_string,
        "--room-id",
        "room_test",
        "--peer-id",
        "peer_a",
        "--ttl-ms",
        "30000",
    ]);
    fs::remove_file(&path).ok();

    assert_eq!(init["status"], "ok");
    assert_eq!(publish_a["status"], "ok");
    assert_eq!(publish_a["remote_offer_count"], 0);
    assert_eq!(publish_b["remote_offer_count"], 1);
    assert_eq!(fetch_for_a["status"], "ok");
    assert_eq!(fetch_for_a["offers"][0]["peer_id"], "peer_b");
    assert_eq!(heartbeat["status"], "ok");
    assert_eq!(heartbeat["peer_id"], "peer_a");
}

#[test]
fn coordination_store_leaves_and_closes_rooms() {
    let path = std::env::temp_dir().join(format!(
        "lai-cli-coordination-leave-close-{}.json",
        std::process::id()
    ));
    let path_string = path.display().to_string();
    fs::remove_file(&path).ok();

    run_cli(&["coordination-store-init", "--out", &path_string]);
    let local = run_cli(&[
        "nat-candidates",
        "--room-id",
        "room_test",
        "--peer-id",
        "peer_a",
        "--bind",
        "127.0.0.1:0",
        "--nonce",
        "nonce-a",
    ]);
    let remote = run_cli(&[
        "nat-candidates",
        "--room-id",
        "room_test",
        "--peer-id",
        "peer_b",
        "--bind",
        "127.0.0.1:0",
        "--nonce",
        "nonce-b",
    ]);
    let local_offer = serde_json::to_string(&local["offer"]).unwrap();
    let remote_offer = serde_json::to_string(&remote["offer"]).unwrap();
    run_cli(&[
        "coordination-offer-publish",
        "--store",
        &path_string,
        "--offer",
        &local_offer,
        "--ttl-ms",
        "30000",
    ]);
    run_cli(&[
        "coordination-offer-publish",
        "--store",
        &path_string,
        "--offer",
        &remote_offer,
        "--ttl-ms",
        "30000",
    ]);

    let leave = run_cli(&[
        "coordination-leave",
        "--store",
        &path_string,
        "--room-id",
        "room_test",
        "--peer-id",
        "peer_a",
    ]);
    let view_after_leave = run_cli(&[
        "coordination-room-view",
        "--store",
        &path_string,
        "--room-id",
        "room_test",
        "--peer-id",
        "peer_b",
        "--subnet",
        "10.77.12.0/24",
    ]);
    let close = run_cli(&[
        "coordination-close",
        "--store",
        &path_string,
        "--room-id",
        "room_test",
    ]);
    let view_after_close = run_cli(&[
        "coordination-room-view",
        "--store",
        &path_string,
        "--room-id",
        "room_test",
        "--peer-id",
        "peer_b",
        "--subnet",
        "10.77.12.0/24",
    ]);
    fs::remove_file(&path).ok();

    assert_eq!(leave["status"], "ok");
    assert_eq!(leave["peer_removed"], true);
    assert_eq!(leave["room_removed"], false);
    assert_eq!(leave["remaining_peer_count"], 1);
    assert_eq!(view_after_leave["member_count"], 1);
    assert_eq!(view_after_leave["members"][0]["peer_id"], "peer_b");
    assert_eq!(close["status"], "ok");
    assert_eq!(close["room_removed"], true);
    assert_eq!(close["removed_peer_count"], 1);
    assert_eq!(view_after_close["member_count"], 0);
}

#[test]
fn coordination_store_kicks_peer_from_room() {
    let path = std::env::temp_dir().join(format!(
        "lai-cli-coordination-kick-{}.json",
        std::process::id()
    ));
    let path_string = path.display().to_string();
    fs::remove_file(&path).ok();

    run_cli(&["coordination-store-init", "--out", &path_string]);
    let host = run_cli(&[
        "nat-candidates",
        "--room-id",
        "room_test",
        "--peer-id",
        "peer_a",
        "--bind",
        "127.0.0.1:0",
        "--nonce",
        "nonce-a",
    ]);
    let guest = run_cli(&[
        "nat-candidates",
        "--room-id",
        "room_test",
        "--peer-id",
        "peer_b",
        "--bind",
        "127.0.0.1:0",
        "--nonce",
        "nonce-b",
    ]);
    let host_offer = serde_json::to_string(&host["offer"]).unwrap();
    let guest_offer = serde_json::to_string(&guest["offer"]).unwrap();
    run_cli(&[
        "coordination-offer-publish",
        "--store",
        &path_string,
        "--offer",
        &host_offer,
        "--ttl-ms",
        "30000",
    ]);
    run_cli(&[
        "coordination-offer-publish",
        "--store",
        &path_string,
        "--offer",
        &guest_offer,
        "--ttl-ms",
        "30000",
    ]);

    let forbidden = run_cli(&[
        "coordination-kick",
        "--store",
        &path_string,
        "--room-id",
        "room_test",
        "--peer-id",
        "peer_a",
        "--kicked-by",
        "peer_b",
    ]);
    let kick = run_cli(&[
        "coordination-kick",
        "--store",
        &path_string,
        "--room-id",
        "room_test",
        "--peer-id",
        "peer_b",
        "--kicked-by",
        "peer_a",
    ]);
    let view_after_kick = run_cli(&[
        "coordination-room-view",
        "--store",
        &path_string,
        "--room-id",
        "room_test",
        "--peer-id",
        "peer_a",
        "--subnet",
        "10.77.12.0/24",
    ]);
    fs::remove_file(&path).ok();

    assert_eq!(forbidden["status"], "forbidden");
    assert_eq!(forbidden["peer_removed"], false);
    assert_eq!(forbidden["host_peer_id"], "peer_a");
    assert_eq!(kick["status"], "ok");
    assert_eq!(kick["peer_removed"], true);
    assert_eq!(kick["kicked_by"], "peer_a");
    assert_eq!(kick["host_peer_id"], "peer_a");
    assert_eq!(kick["room_removed"], false);
    assert_eq!(kick["remaining_peer_count"], 1);
    assert_eq!(view_after_kick["member_count"], 1);
    assert_eq!(view_after_kick["members"][0]["peer_id"], "peer_a");
}

#[test]
fn coordination_room_view_outputs_online_members() {
    let path = std::env::temp_dir().join(format!(
        "lai-cli-coordination-room-view-{}.json",
        std::process::id()
    ));
    let path_string = path.display().to_string();
    fs::remove_file(&path).ok();

    run_cli(&["coordination-store-init", "--out", &path_string]);
    let local = run_cli(&[
        "nat-candidates",
        "--room-id",
        "room_test",
        "--peer-id",
        "peer_a",
        "--bind",
        "127.0.0.1:0",
        "--nonce",
        "nonce-a",
    ]);
    let remote = run_cli(&[
        "nat-candidates",
        "--room-id",
        "room_test",
        "--peer-id",
        "peer_b",
        "--bind",
        "127.0.0.1:0",
        "--nonce",
        "nonce-b",
    ]);
    let local_offer = serde_json::to_string(&local["offer"]).unwrap();
    let remote_offer = serde_json::to_string(&remote["offer"]).unwrap();
    run_cli(&[
        "coordination-offer-publish",
        "--store",
        &path_string,
        "--offer",
        &local_offer,
        "--ttl-ms",
        "30000",
    ]);
    run_cli(&[
        "coordination-offer-publish",
        "--store",
        &path_string,
        "--offer",
        &remote_offer,
        "--ttl-ms",
        "30000",
    ]);

    let view = run_cli(&[
        "coordination-room-view",
        "--store",
        &path_string,
        "--room-id",
        "room_test",
        "--peer-id",
        "peer_a",
        "--subnet",
        "10.77.12.0/24",
    ]);
    fs::remove_file(&path).ok();

    assert_eq!(view["status"], "ready");
    assert_eq!(view["room_id"], "room_test");
    assert_eq!(view["local_peer_id"], "peer_a");
    assert_eq!(view["member_count"], 2);
    assert_eq!(view["online_count"], 2);
    assert_eq!(view["expired_count"], 0);
    assert_eq!(view["members"][0]["peer_id"], "peer_a");
    assert_eq!(view["members"][0]["status"], "online");
    assert!(view["members"][0]["virtual_ip"]
        .as_str()
        .unwrap()
        .starts_with("10.77.12."));
    assert_eq!(view["members"][1]["peer_id"], "peer_b");
    assert!(view["members"][1]["candidate_count"].as_u64().unwrap() >= 1);
    assert!(view["members"][1]["candidate_signature"]
        .as_str()
        .unwrap()
        .contains("host:udp:"));
    assert!(view["next_action"]
        .as_str()
        .unwrap()
        .contains("runtime bootstrap"));
}

#[test]
fn nat_hole_punch_loopback_exchanges_udp_packets() {
    let value = run_cli(&[
        "nat-hole-punch-loopback-test",
        "--room-id",
        "room_test",
        "--peer-a",
        "peer_a",
        "--peer-b",
        "peer_b",
        "--attempts",
        "2",
        "--interval-ms",
        "0",
        "--message",
        "hello",
    ]);

    assert_eq!(value["status"], "ok");
    assert!(value["sentByA"].as_u64().unwrap() >= 1);
    assert!(value["sentByB"].as_u64().unwrap() >= 1);
    assert!(value["receivedByA"].as_u64().unwrap() >= 1);
    assert!(value["receivedByB"].as_u64().unwrap() >= 1);
}

#[test]
fn nat_hole_punch_sends_udp_to_remote_candidate() {
    let receiver = UdpSocket::bind("127.0.0.1:0").unwrap();
    receiver
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    let endpoint = receiver.local_addr().unwrap().to_string();
    let remote_offer = serde_json::json!({
        "schema_version": 1,
        "room_id": "room_test",
        "peer_id": "peer_b",
        "nonce": "nonce-b",
        "created_at_ms": 1,
        "candidates": [{
            "candidate_type": "host",
            "transport": "udp",
            "endpoint": endpoint,
            "priority": 100,
            "source": "test-socket"
        }]
    });
    let remote_offer = serde_json::to_string(&remote_offer).unwrap();
    let value = run_cli(&[
        "nat-hole-punch",
        "--room-id",
        "room_test",
        "--peer-id",
        "peer_a",
        "--bind",
        "127.0.0.1:0",
        "--remote-offer",
        &remote_offer,
        "--attempts",
        "2",
        "--interval-ms",
        "0",
        "--receive-timeout-ms",
        "0",
        "--message",
        "hello",
    ]);

    assert_eq!(value["status"], "sent-no-response");
    assert_eq!(value["plan"]["status"], "ready");
    assert_eq!(value["sentPackets"].as_array().unwrap().len(), 2);
    let mut buffer = [0u8; 2048];
    let (bytes, _) = receiver.recv_from(&mut buffer).unwrap();
    let text = String::from_utf8_lossy(&buffer[..bytes]);
    assert!(text.contains("\"type\":\"nat-punch\""));
    assert!(text.contains("\"peerId\":\"peer_a\""));
}

#[test]
fn nat_p2p_bootstrap_punches_then_completes_encrypted_handshake() {
    let probe = UdpSocket::bind("127.0.0.1:0").expect("free udp port");
    let listener_addr = probe.local_addr().unwrap();
    drop(probe);

    let listener = Command::new(env!("CARGO_BIN_EXE_lai-cli"))
        .args([
            "p2p-handshake-listen",
            "--bind",
            &listener_addr.to_string(),
            "--key",
            "test-room-key",
            "--responder-peer-id",
            "peer_b",
            "--max-packets",
            "1",
            "--timeout-ms",
            "2000",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn p2p listener");
    std::thread::sleep(Duration::from_millis(80));

    let remote_offer = serde_json::json!({
        "schema_version": 1,
        "room_id": "room_test",
        "peer_id": "peer_b",
        "nonce": "nonce-b",
        "created_at_ms": 1,
        "candidates": [{
            "candidate_type": "host",
            "transport": "udp",
            "endpoint": listener_addr.to_string(),
            "priority": 100,
            "source": "test-listener"
        }]
    });
    let remote_offer = serde_json::to_string(&remote_offer).unwrap();
    let bootstrap = run_cli(&[
        "nat-p2p-bootstrap",
        "--room-id",
        "room_test",
        "--peer-id",
        "peer_a",
        "--virtual-ip",
        "10.77.12.2",
        "--key",
        "test-room-key",
        "--bind",
        "127.0.0.1:0",
        "--remote-offer",
        &remote_offer,
        "--punch-attempts",
        "1",
        "--punch-interval-ms",
        "0",
        "--handshake-timeout-ms",
        "2000",
    ]);
    let listener_output = listener.wait_with_output().expect("listener exits");

    assert_eq!(bootstrap["status"], "ok");
    assert!(bootstrap["localEndpoint"]
        .as_str()
        .unwrap()
        .starts_with("127.0.0.1:"));
    assert_eq!(bootstrap["selectedPeer"]["responderPeerId"], "peer_b");
    assert_eq!(bootstrap["selectedPeer"]["nonceMatched"], true);
    assert_eq!(bootstrap["selectedPeer"]["handshakeRole"], "received-ack");
    assert_eq!(bootstrap["selectedPeer"]["confirmedByAck"], true);
    assert_eq!(bootstrap["punchPackets"].as_array().unwrap().len(), 1);
    assert_eq!(bootstrap["handshakePackets"].as_array().unwrap().len(), 2);
    assert!(
        listener_output.status.success(),
        "listener failed\nstatus: {}\nstdout: {}\nstderr: {}",
        listener_output.status,
        String::from_utf8_lossy(&listener_output.stdout),
        String::from_utf8_lossy(&listener_output.stderr)
    );
    let listener_json: Value =
        serde_json::from_slice(&listener_output.stdout).expect("listener json");
    assert_eq!(listener_json["status"], "ok");
    assert_eq!(listener_json["handshakes"][0]["peerId"], "peer_a");
    assert!(!listener_json["ignoredPackets"]
        .as_array()
        .unwrap()
        .is_empty());
}

#[test]
fn nat_p2p_bootstrap_can_complete_when_both_peers_start_together() {
    let probe_a = UdpSocket::bind("127.0.0.1:0").expect("free udp port a");
    let addr_a = probe_a.local_addr().unwrap();
    let probe_b = UdpSocket::bind("127.0.0.1:0").expect("free udp port b");
    let addr_b = probe_b.local_addr().unwrap();
    drop(probe_a);
    drop(probe_b);

    let offer_a = serde_json::json!({
        "schema_version": 1,
        "room_id": "room_test",
        "peer_id": "peer_a",
        "nonce": "nonce-a",
        "created_at_ms": 1,
        "candidates": [{
            "candidate_type": "host",
            "transport": "udp",
            "endpoint": addr_a.to_string(),
            "priority": 100,
            "source": "test-bootstrap"
        }]
    });
    let offer_b = serde_json::json!({
        "schema_version": 1,
        "room_id": "room_test",
        "peer_id": "peer_b",
        "nonce": "nonce-b",
        "created_at_ms": 1,
        "candidates": [{
            "candidate_type": "host",
            "transport": "udp",
            "endpoint": addr_b.to_string(),
            "priority": 100,
            "source": "test-bootstrap"
        }]
    });
    let offer_a = serde_json::to_string(&offer_a).unwrap();
    let offer_b = serde_json::to_string(&offer_b).unwrap();

    let peer_a = Command::new(env!("CARGO_BIN_EXE_lai-cli"))
        .args([
            "nat-p2p-bootstrap",
            "--room-id",
            "room_test",
            "--peer-id",
            "peer_a",
            "--virtual-ip",
            "10.77.12.2",
            "--key",
            "test-room-key",
            "--bind",
            &addr_a.to_string(),
            "--remote-offer",
            &offer_b,
            "--punch-attempts",
            "2",
            "--punch-interval-ms",
            "25",
            "--handshake-timeout-ms",
            "3000",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn peer a bootstrap");

    let peer_b = Command::new(env!("CARGO_BIN_EXE_lai-cli"))
        .args([
            "nat-p2p-bootstrap",
            "--room-id",
            "room_test",
            "--peer-id",
            "peer_b",
            "--virtual-ip",
            "10.77.12.3",
            "--key",
            "test-room-key",
            "--bind",
            &addr_b.to_string(),
            "--remote-offer",
            &offer_a,
            "--punch-attempts",
            "2",
            "--punch-interval-ms",
            "25",
            "--handshake-timeout-ms",
            "3000",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn peer b bootstrap");

    let output_a = peer_a.wait_with_output().expect("peer a exits");
    let output_b = peer_b.wait_with_output().expect("peer b exits");

    assert!(
        output_a.status.success(),
        "peer a failed\nstatus: {}\nstdout: {}\nstderr: {}",
        output_a.status,
        String::from_utf8_lossy(&output_a.stdout),
        String::from_utf8_lossy(&output_a.stderr)
    );
    assert!(
        output_b.status.success(),
        "peer b failed\nstatus: {}\nstdout: {}\nstderr: {}",
        output_b.status,
        String::from_utf8_lossy(&output_b.stdout),
        String::from_utf8_lossy(&output_b.stderr)
    );
    let value_a: Value = serde_json::from_slice(&output_a.stdout).expect("peer a json");
    let value_b: Value = serde_json::from_slice(&output_b.stdout).expect("peer b json");

    assert_eq!(value_a["status"], "ok");
    assert_eq!(value_a["localEndpoint"], addr_a.to_string());
    assert_eq!(value_a["selectedPeer"]["responderPeerId"], "peer_b");
    assert_eq!(value_a["selectedPeer"]["nonceMatched"], true);
    assert!(
        value_a["selectedPeer"]["handshakeRole"] == "received-ack"
            || value_a["selectedPeer"]["handshakeRole"] == "answered-remote-hello"
    );
    assert_eq!(value_a["selectedPeer"]["confirmedByAck"], true);
    assert_eq!(value_b["status"], "ok");
    assert_eq!(value_b["localEndpoint"], addr_b.to_string());
    assert_eq!(value_b["selectedPeer"]["responderPeerId"], "peer_a");
    assert_eq!(value_b["selectedPeer"]["nonceMatched"], true);
    assert!(
        value_b["selectedPeer"]["handshakeRole"] == "received-ack"
            || value_b["selectedPeer"]["handshakeRole"] == "answered-remote-hello"
    );
    assert_eq!(value_b["selectedPeer"]["confirmedByAck"], true);
}

#[test]
fn udp_forward_loopback_writes_packet_observation() {
    let path = std::env::temp_dir().join(format!(
        "lai-cli-udp-forward-observation-{}.txt",
        std::process::id()
    ));
    let path_string = path.display().to_string();
    let value = run_cli(&[
        "udp-forward-loopback-test",
        "--message",
        "hello udp",
        "--observe-file",
        &path_string,
    ]);

    assert_eq!(value["status"], "ok");
    assert_eq!(value["message"], "hello udp");
    assert_eq!(value["summary"]["forwarded_packets"], 1);

    let observations = fs::read_to_string(&path).expect("observation file");
    fs::remove_file(&path).ok();
    assert!(observations.contains("udp:127.0.0.1:127.0.0.1:"));
    assert!(observations.contains(":unicast:outbound:9"));
}

#[test]
fn udp_capture_loopback_writes_inbound_packet_observation() {
    let path = std::env::temp_dir().join(format!(
        "lai-cli-udp-capture-observation-{}.txt",
        std::process::id()
    ));
    let path_string = path.display().to_string();
    let value = run_cli(&[
        "udp-capture-loopback-test",
        "--message",
        "hello capture",
        "--observe-file",
        &path_string,
    ]);

    assert_eq!(value["status"], "ok");
    assert_eq!(value["message"], "hello capture");
    assert_eq!(value["summary"]["forwarded_packets"], 1);

    let observations = fs::read_to_string(&path).expect("observation file");
    fs::remove_file(&path).ok();
    assert!(observations.contains("udp:127.0.0.1:127.0.0.1:"));
    assert!(observations.contains(":unicast:inbound:13"));
}

#[test]
fn diagnostic_export_writes_bundle_file() {
    let path = std::env::temp_dir().join(format!(
        "lai-cli-diagnostic-export-{}.json",
        std::process::id()
    ));
    let path_string = path.display().to_string();
    let stdout = run_cli(&[
        "diagnostic-export",
        "--out",
        &path_string,
        "--adapter-name",
        "LocalAreaInterconnection",
        "--expected-ip",
        "10.77.12.2",
        "--assigned-ip",
        "10.77.12.2",
        "--subnet",
        "10.77.12.0/24",
        "--adapter-scan",
        "false",
        "--firewall-scan",
        "false",
        "--expected-peers",
        "1",
        "--packets",
        "udp:10.77.12.2:10.77.12.255:39078:broadcast:outbound:8,udp:10.77.12.2:10.77.12.1:39077:unicast:outbound:8",
        "--broadcast-ports",
        "39078",
        "--game-ports",
        "39077",
        "--game-name",
        "Example Game",
        "--ports",
        "39077,39078",
    ]);

    assert_eq!(stdout["status"], "ok");
    assert_eq!(stdout["bundleStatus"], "needs-attention");

    let bundle: Value =
        serde_json::from_str(&fs::read_to_string(&path).expect("bundle file")).unwrap();
    fs::remove_file(&path).ok();

    assert_eq!(bundle["schema_version"], 18);
    assert_eq!(bundle["tool"], "LocalAreaInterconnection Rust CLI");
    assert_eq!(bundle["adapter_scan"]["status"], "ok");
    assert_eq!(bundle["packet_io"]["status"], "ok");
    assert_eq!(bundle["packet_io"]["backend"], "userspace-udp");
    assert_eq!(bundle["packet_io"]["probe"]["status"], "ready");
    assert_eq!(bundle["route_scan"]["status"], "skipped");
    assert_eq!(bundle["game_port_scan"]["status"], "skipped");
    assert!(!bundle["game_readiness"]["status"]
        .as_str()
        .unwrap()
        .is_empty());
    assert_eq!(bundle["runtime_cleanup"]["status"], "skipped");
    assert_eq!(bundle["relay_fallback"]["status"], "skipped");
    assert_eq!(bundle["connection_path"]["status"], "skipped");
    assert_eq!(bundle["packet_observations"]["broadcast_count"], 1);
    assert_eq!(bundle["packet_observations"]["game_traffic_count"], 1);
}

#[test]
fn diagnostic_export_can_use_catalog_profile_for_game_readiness() {
    let bundle_path = std::env::temp_dir().join(format!(
        "lai-cli-diagnostic-export-catalog-{}.json",
        std::process::id()
    ));
    let catalog_path = std::env::temp_dir().join(format!(
        "lai-cli-diagnostic-export-catalog-profile-{}.json",
        std::process::id()
    ));
    let netstat_path = std::env::temp_dir().join(format!(
        "lai-cli-diagnostic-export-catalog-netstat-{}.txt",
        std::process::id()
    ));
    let bundle_arg = bundle_path.to_string_lossy().to_string();
    let catalog_arg = catalog_path.to_string_lossy().to_string();
    let netstat_arg = netstat_path.to_string_lossy().to_string();
    fs::write(
        &catalog_path,
        serde_json::json!({
            "profiles": [{
                "game_name": "Catalog Export Game",
                "steam_app_id": "555000",
                "discovery": "udp_broadcast",
                "ports": [27016],
                "compatibility": "A"
            }]
        })
        .to_string(),
    )
    .expect("write catalog");
    fs::write(
        &netstat_path,
        r#"
  Proto  Local Address          Foreign Address        State           PID
  UDP    0.0.0.0:27016          *:*                                    4242
"#,
    )
    .expect("write netstat output");

    let stdout = run_cli(&[
        "diagnostic-export",
        "--out",
        &bundle_arg,
        "--adapter-name",
        "LocalAreaInterconnection",
        "--expected-ip",
        "10.77.12.2",
        "--assigned-ip",
        "10.77.12.2",
        "--subnet",
        "10.77.12.0/24",
        "--adapter-scan",
        "false",
        "--firewall-scan",
        "false",
        "--expected-peers",
        "1",
        "--packets",
        "udp:10.77.12.2:10.77.12.255:27016:broadcast:outbound:8,udp:10.77.12.2:10.77.12.1:27016:unicast:outbound:8",
        "--broadcast-ports",
        "27016",
        "--game-ports",
        "27016",
        "--catalog",
        &catalog_arg,
        "--game-name",
        "catalog export game",
        "--netstat-output",
        &netstat_arg,
        "--netstat-scan",
        "false",
    ]);

    assert_eq!(stdout["status"], "ok");
    let bundle: Value =
        serde_json::from_str(&fs::read_to_string(&bundle_path).expect("bundle file")).unwrap();
    fs::remove_file(&bundle_path).ok();
    fs::remove_file(&catalog_path).ok();
    fs::remove_file(&netstat_path).ok();

    assert_eq!(bundle["schema_version"], 18);
    assert_eq!(bundle["inputs"]["game_name"], "Catalog Export Game");
    assert_eq!(bundle["inputs"]["ports"][0], 27016);
    assert_eq!(bundle["game_port_scan"]["expected_ports"][0], 27016);
    assert_eq!(bundle["game_port_scan"]["match_count"], 1);
    assert_eq!(bundle["game_readiness"]["game_name"], "Catalog Export Game");
    assert!(!bundle["game_readiness"]["status"]
        .as_str()
        .unwrap()
        .is_empty());
}

#[test]
fn diagnostic_export_can_include_relay_fallback_plan() {
    let path = std::env::temp_dir().join(format!(
        "lai-cli-diagnostic-export-relay-{}.json",
        std::process::id()
    ));
    let path_string = path.display().to_string();
    let local_offer = serde_json::json!({
        "schema_version": 1,
        "room_id": "room_test",
        "peer_id": "peer_a",
        "nonce": "nonce-a",
        "created_at_ms": 1,
        "candidates": [{
            "candidate_type": "host",
            "transport": "udp",
            "endpoint": "127.0.0.1:39090",
            "priority": 100,
            "source": "test"
        }]
    });
    let remote_offer = serde_json::json!({
        "schema_version": 1,
        "room_id": "room_test",
        "peer_id": "peer_b",
        "nonce": "nonce-b",
        "created_at_ms": 1,
        "candidates": [
            {
                "candidate_type": "host",
                "transport": "udp",
                "endpoint": "127.0.0.1:39091",
                "priority": 100,
                "source": "test"
            },
            {
                "candidate_type": "relay",
                "transport": "udp",
                "endpoint": "203.0.113.10:39090",
                "priority": 10,
                "source": "test-relay"
            }
        ]
    });
    let local_offer = serde_json::to_string(&local_offer).unwrap();
    let remote_offer = serde_json::to_string(&remote_offer).unwrap();

    let stdout = run_cli(&[
        "diagnostic-export",
        "--out",
        &path_string,
        "--adapter-name",
        "LocalAreaInterconnection",
        "--expected-ip",
        "10.77.12.2",
        "--assigned-ip",
        "10.77.12.2",
        "--subnet",
        "10.77.12.0/24",
        "--adapter-scan",
        "false",
        "--firewall-scan",
        "false",
        "--expected-peers",
        "1",
        "--relay-local-offer",
        &local_offer,
        "--relay-remote-offer",
        &remote_offer,
        "--relay-p2p-status",
        "failed",
    ]);

    assert_eq!(stdout["status"], "ok");
    let bundle: Value =
        serde_json::from_str(&fs::read_to_string(&path).expect("bundle file")).unwrap();
    fs::remove_file(&path).ok();

    assert_eq!(bundle["schema_version"], 18);
    assert_eq!(bundle["relay_fallback"]["status"], "ok");
    assert_eq!(bundle["connection_path"]["status"], "ok");
    assert_eq!(
        bundle["connection_path"]["report"]["selected_path"],
        "relay"
    );
    assert_eq!(
        bundle["connection_path"]["report"]["selected_endpoints"][0],
        "203.0.113.10:39090"
    );
    assert!(bundle["game_readiness"]["checks"]
        .as_array()
        .unwrap()
        .iter()
        .any(|check| check["key"] == "connection-path"));
    assert_eq!(bundle["route_scan"]["status"], "skipped");
    assert_eq!(bundle["game_port_scan"]["status"], "skipped");
    assert!(!bundle["game_readiness"]["status"]
        .as_str()
        .unwrap()
        .is_empty());
    assert_eq!(bundle["runtime_cleanup"]["status"], "skipped");
    assert_eq!(
        bundle["relay_fallback"]["plan"]["status"],
        "relay-available"
    );
    assert_eq!(
        bundle["relay_fallback"]["plan"]["selected_relay_endpoints"][0],
        "203.0.113.10:39090"
    );
}

#[test]
fn diagnostic_export_merges_runtime_snapshot_packet_io_evidence() {
    let bundle_path = std::env::temp_dir().join(format!(
        "lai-cli-diagnostic-export-runtime-{}.json",
        std::process::id()
    ));
    let snapshot_path = std::env::temp_dir().join(format!(
        "lai-cli-runtime-snapshot-{}.json",
        std::process::id()
    ));
    let route_path =
        std::env::temp_dir().join(format!("lai-cli-runtime-routes-{}.txt", std::process::id()));
    let bundle_path_string = bundle_path.display().to_string();
    let snapshot_path_string = snapshot_path.display().to_string();
    let route_path_string = route_path.display().to_string();
    let local_offer = serde_json::json!({
        "schema_version": 1,
        "room_id": "room_test",
        "peer_id": "peer_a",
        "nonce": "nonce-a",
        "created_at_ms": 1,
        "candidates": [{
            "candidate_type": "host",
            "transport": "udp",
            "endpoint": "10.0.0.2:39090",
            "priority": 100,
            "source": "test"
        }]
    });
    let remote_offer = serde_json::json!({
        "schema_version": 1,
        "room_id": "room_test",
        "peer_id": "peer_b",
        "nonce": "nonce-b",
        "created_at_ms": 1,
        "candidates": [{
            "candidate_type": "srflx",
            "transport": "udp",
            "endpoint": "198.51.100.20:44000",
            "priority": 90,
            "source": "test"
        }]
    });
    let connection_path_report = run_cli(&[
        "connection-path-plan",
        "--local-offer",
        &local_offer.to_string(),
        "--remote-offer",
        &remote_offer.to_string(),
        "--p2p-status",
        "ok",
    ]);
    fs::write(
        &snapshot_path,
        serde_json::to_string_pretty(&serde_json::json!({
            "status": "ok",
            "connectionPathReports": [{
                "source": "nat-bootstrap-remote-peer",
                "peerId": "peer_b",
                "bootstrapStatus": "ok",
                "report": connection_path_report
            }],
            "runtimePeerSummaries": [{
                "peerId": "peer_b",
                "virtualIp": "10.77.12.3",
                "endpoint": "198.51.100.20:44000",
                "selectedPath": "p2p",
                "pathKind": "direct",
                "connectionPathStatus": "p2p-candidate-ready",
                "bootstrapStatus": "ok",
                "connected": true,
                "latencyMs": 12,
                "lastSeenAtMs": 123456,
                "lastSentAtMs": 123460,
                "bytesSent": 21,
                "bytesReceived": 21,
                "directBytesSent": 21,
                "directBytesReceived": 21,
                "relayBytesSent": 0,
                "relayBytesReceived": 0,
                "unknownPathBytesSent": 0,
                "unknownPathBytesReceived": 0,
                "heartbeatPacketsSent": 1,
                "heartbeatAckPacketsReceived": 1,
                "heartbeatAckPacketsSent": 0,
                "heartbeatLossPercent": 0.0,
                "heartbeatLossWindowSize": 1,
                "heartbeatLossWindowPercent": 0.0,
                "heartbeatRttSampleCount": 1,
                "heartbeatRttJitterMs": null,
                "forwardedPacketsSent": 1,
                "tunnelPacketsReceived": 1
            }],
            "packetIoPlan": {
                "backend": "wintun",
                "adapterName": "LocalAreaInterconnection",
                "canReadIpv4": true,
                "canWriteIpv4": true
            },
            "packetIoProbe": {
                "backend": "wintun",
                "status": "partial",
                "adapterReadStatus": "ready",
                "adapterWriteStatus": "ready"
            },
            "adapterReadStatus": "ready",
            "adapterWriteStatus": "ready",
            "packetObservationLines": [
                "udp:10.77.12.2:10.77.12.255:39078:broadcast:outbound:21",
                "tcp:10.77.12.2:10.77.12.3:39077:unicast:outbound:32"
            ],
            "rawVirtualPackets": [{
                "sourceIp": "10.77.12.2",
                "destinationIp": "10.77.12.255",
                "protocol": "udp",
                "payloadBytes": 21
            }],
            "forwardedPackets": [{"bytes": 21}],
            "broadcastForwardReport": {
                "status": "ok",
                "event_count": 1,
                "forwarded_event_count": 1,
                "dropped_event_count": 0,
                "forwarded_target_count": 1,
                "rate_limited_count": 0,
                "allowed_ports": [39078],
                "max_packets_per_second": 30,
                "events": [{
                    "protocol": "udp",
                    "source_ip": "10.77.12.2",
                    "destination_ip": "10.77.12.255",
                    "destination_port": 39078,
                    "forwarded": true,
                    "reason": "room-broadcast-allowed",
                    "target_count": 1,
                    "packet_io_backend": "wintun"
                }],
                "next_action": "Broadcast forwarding decisions look healthy."
            },
            "injectedPackets": [{"bytes": 21}],
            "wintunRuntime": {
                "receivedPackets": [{"protocol": "udp"}],
                "sentPackets": [{"protocol": "udp"}],
                "errors": [],
                "close": {
                    "session_ended": true,
                    "closed": true
                }
            },
            "tunnelServiceSnapshot": {
                "service_running": true,
                "connected_peer_count": 1,
                "connection_path": "p2p",
                "average_latency_ms": 12,
                "packet_loss_percent": 0.0,
                "bytes_sent": 21,
                "bytes_received": 21,
                "last_error": null
            },
            "runtimeCleanupPlan": {
                "platform": "windows",
                "dry_run": true,
                "room_id": "room_test",
                "local_peer_id": "peer_a",
                "local_virtual_ip": "10.77.12.2",
                "adapter_name": "LocalAreaInterconnection",
                "packet_io_backend": "wintun",
                "restore_adapter": true,
                "requires_elevation": true,
                "process_cleanup_steps": [{
                    "key": "close-wintun-session",
                    "status": "automatic",
                    "detail": "Close Wintun session."
                }],
                "commands": [{
                    "tool": "netsh",
                    "args": ["interface", "ipv4", "set", "address", "name=LocalAreaInterconnection", "dhcp"],
                    "command": "netsh interface ipv4 set address name=LocalAreaInterconnection dhcp",
                    "purpose": "Restore adapter IPv4 address mode."
                }],
                "verification_checks": ["Adapter configuration reviewed."],
                "warnings": [{
                    "key": "review-before-restore",
                    "message": "Review commands before running."
                }]
            }
        }))
        .unwrap(),
    )
    .expect("write runtime snapshot");
    fs::write(
        &route_path,
        r#"
IPv4 Route Table
===========================================================================
Active Routes:
Network Destination        Netmask          Gateway       Interface  Metric
       10.77.12.0    255.255.255.0         On-link       10.77.12.2      5
"#,
    )
    .expect("write route output");

    let stdout = run_cli(&[
        "diagnostic-export",
        "--out",
        &bundle_path_string,
        "--adapter-name",
        "LocalAreaInterconnection",
        "--expected-ip",
        "10.77.12.2",
        "--assigned-ip",
        "10.77.12.2",
        "--subnet",
        "10.77.12.0/24",
        "--adapter-scan",
        "false",
        "--firewall-scan",
        "false",
        "--expected-peers",
        "1",
        "--runtime-snapshot",
        &snapshot_path_string,
        "--route-output",
        &route_path_string,
        "--broadcast-ports",
        "39078",
        "--game-ports",
        "39077",
        "--ports",
        "39077,39078",
    ]);

    assert_eq!(stdout["status"], "ok");

    let bundle: Value =
        serde_json::from_str(&fs::read_to_string(&bundle_path).expect("bundle file")).unwrap();
    fs::remove_file(&bundle_path).ok();
    fs::remove_file(&snapshot_path).ok();
    fs::remove_file(&route_path).ok();

    assert_eq!(bundle["packet_io"]["backend"], "wintun");
    assert_eq!(bundle["packet_io"]["adapter_read_status"], "ready");
    assert_eq!(bundle["packet_io"]["adapter_write_status"], "ready");
    assert_eq!(bundle["packet_io"]["runtime"]["status"], "ok");
    assert_eq!(
        bundle["packet_io"]["runtime"]["raw_virtual_packet_count"],
        1
    );
    assert_eq!(bundle["packet_io"]["runtime"]["forwarded_packet_count"], 1);
    assert_eq!(bundle["packet_io"]["runtime"]["injected_packet_count"], 1);
    assert_eq!(
        bundle["packet_io"]["runtime"]["wintun_received_packet_count"],
        1
    );
    assert_eq!(
        bundle["packet_io"]["runtime"]["wintun_sent_packet_count"],
        1
    );
    assert_eq!(bundle["schema_version"], 18);
    assert_eq!(bundle["connection_path"]["status"], "ok");
    assert_eq!(
        bundle["connection_path"]["source"],
        "runtime-snapshot-report"
    );
    assert_eq!(bundle["connection_path"]["runtime_path"], "p2p");
    assert_eq!(bundle["connection_path"]["report"]["selected_path"], "p2p");
    assert_eq!(bundle["runtime_peers"]["status"], "ok");
    assert_eq!(bundle["runtime_peers"]["peer_count"], 1);
    assert_eq!(bundle["runtime_peers"]["connected_peer_count"], 1);
    assert_eq!(bundle["runtime_peers"]["total_direct_bytes_sent"], 21);
    assert_eq!(bundle["runtime_peers"]["total_direct_bytes_received"], 21);
    assert_eq!(bundle["runtime_peers"]["total_relay_bytes_sent"], 0);
    assert_eq!(bundle["runtime_peers"]["total_relay_bytes_received"], 0);
    assert_eq!(
        bundle["runtime_peers"]["summaries"][0]["selectedPath"],
        "p2p"
    );
    assert_eq!(
        bundle["runtime_peers"]["summaries"][0]["pathKind"],
        "direct"
    );
    assert_eq!(
        bundle["runtime_peers"]["summaries"][0]["directBytesSent"],
        21
    );
    assert_eq!(bundle["runtime_peers"]["summaries"][0]["relayBytesSent"], 0);
    assert_eq!(bundle["runtime_peers"]["summaries"][0]["latencyMs"], 12);
    assert_eq!(
        bundle["runtime_peers"]["summaries"][0]["lastSeenAtMs"],
        123456
    );
    assert_eq!(
        bundle["runtime_peers"]["summaries"][0]["lastSentAtMs"],
        123460
    );
    assert_eq!(
        bundle["runtime_peers"]["summaries"][0]["heartbeatAckPacketsReceived"],
        1
    );
    assert_eq!(
        bundle["runtime_peers"]["summaries"][0]["heartbeatLossPercent"],
        0.0
    );
    assert_eq!(
        bundle["runtime_peers"]["summaries"][0]["heartbeatLossWindowPercent"],
        0.0
    );
    assert_eq!(
        bundle["runtime_peers"]["summaries"][0]["heartbeatRttSampleCount"],
        1
    );
    assert_eq!(
        bundle["runtime_peers"]["summaries"][0]["health"]["status"],
        "ok"
    );
    assert_eq!(
        bundle["runtime_peers"]["summaries"][0]["health"]["reason"],
        "runtime-peer-healthy"
    );
    assert_eq!(bundle["broadcast_forward"]["status"], "ok");
    assert_eq!(bundle["broadcast_forward"]["source"], "runtime-snapshot");
    assert_eq!(bundle["broadcast_forward"]["event_count"], 1);
    assert_eq!(bundle["broadcast_forward"]["forwarded_event_count"], 1);
    assert_eq!(bundle["route_scan"]["status"], "ok");
    assert_eq!(bundle["game_port_scan"]["status"], "skipped");
    assert!(!bundle["game_readiness"]["status"]
        .as_str()
        .unwrap()
        .is_empty());
    assert_eq!(bundle["route_scan"]["route_count"], 1);
    assert_eq!(bundle["route_scan"]["room_route_count"], 1);
    assert_eq!(bundle["runtime_cleanup"]["status"], "needs-attention");
    assert_eq!(bundle["runtime_cleanup"]["requires_elevation"], true);
    assert_eq!(bundle["runtime_cleanup"]["restore_adapter"], true);
    assert_eq!(bundle["runtime_cleanup"]["process_step_count"], 1);
    assert_eq!(bundle["runtime_cleanup"]["command_count"], 1);
    assert_eq!(bundle["runtime_cleanup"]["check_count"], 5);
    assert_eq!(bundle["runtime_cleanup"]["next_action_count"], 3);
    assert_eq!(bundle["runtime_cleanup"]["route_count"], 1);
    assert_eq!(
        bundle["runtime_cleanup"]["plan"]["packet_io_backend"],
        "wintun"
    );
    assert_eq!(
        bundle["runtime_cleanup"]["report"]["status"],
        "needs-attention"
    );
    assert!(bundle["runtime_cleanup"]["report"]["checks"]
        .as_array()
        .unwrap()
        .iter()
        .any(|check| check["key"] == "adapter-restore" && check["status"] == "needs-attention"));
    assert!(bundle["runtime_cleanup"]["report"]["checks"]
        .as_array()
        .unwrap()
        .iter()
        .any(|check| check["key"] == "route-cleanup" && check["status"] == "needs-attention"));
    assert_eq!(bundle["packet_observations"]["broadcast_count"], 1);
    assert_eq!(bundle["packet_observations"]["game_traffic_count"], 1);
}

#[test]
fn wintun_detect_outputs_dll_and_admin_status() {
    let value = run_cli(&["wintun-detect"]);
    assert!(value["dll_found"].is_boolean());
    assert!(value["is_admin"].is_boolean());
    assert!(!value["status"].as_str().unwrap().is_empty());
    assert!(value["next_actions"].as_array().unwrap().len() > 0);
}

#[test]
fn wintun_adapter_create_requires_explicit_confirmation() {
    let value = run_cli(&[
        "wintun-adapter-create",
        "--adapter-name",
        "LocalAreaInterconnection",
        "--tunnel-type",
        "LocalAreaInterconnection",
    ]);

    assert_eq!(value["status"], "needs-confirmation");
    assert_eq!(value["requiresElevation"], true);
    assert_eq!(value["confirmed"], false);
    assert_eq!(value["canExecuteNow"], false);
    assert!(value["nextAction"].as_str().unwrap().contains("--yes true"));
}

#[test]
fn wintun_adapter_ensure_requires_explicit_confirmation() {
    let value = run_cli(&[
        "wintun-adapter-ensure",
        "--adapter-name",
        "LocalAreaInterconnection",
        "--tunnel-type",
        "LocalAreaInterconnection",
    ]);

    assert_eq!(value["status"], "needs-confirmation");
    assert_eq!(value["requiresElevation"], true);
    assert_eq!(value["confirmed"], false);
    assert_eq!(value["canExecuteNow"], false);
    assert!(value["nextAction"].as_str().unwrap().contains("--yes true"));
}

#[test]
fn wintun_adapter_delete_requires_explicit_confirmation() {
    let value = run_cli(&[
        "wintun-adapter-delete",
        "--adapter-name",
        "LocalAreaInterconnection",
        "--tunnel-type",
        "LocalAreaInterconnection",
    ]);

    assert_eq!(value["status"], "needs-confirmation");
    assert_eq!(value["requiresElevation"], true);
    assert_eq!(value["confirmed"], false);
    assert_eq!(value["canExecuteNow"], false);
    assert!(value["nextAction"].as_str().unwrap().contains("--yes true"));
}

#[test]
fn wintun_adapter_open_reports_probe_status() {
    let value = run_cli(&[
        "wintun-adapter-open",
        "--adapter-name",
        "LocalAreaInterconnection",
    ]);

    assert!(!value["status"].as_str().unwrap().is_empty());
    assert!(value["opened"].is_boolean());
    assert!(value["closed"].is_boolean());
}

#[test]
fn wintun_session_probe_reports_lifecycle_status() {
    let value = run_cli(&[
        "wintun-session-probe",
        "--adapter-name",
        "LocalAreaInterconnection",
    ]);

    assert!(!value["status"].as_str().unwrap().is_empty());
    assert_eq!(value["ring_capacity"], 131072);
    assert!(value["opened"].is_boolean());
    assert!(value["session_started"].is_boolean());
    assert!(value["session_ended"].is_boolean());
    assert!(value["closed"].is_boolean());
}

#[test]
fn wintun_packet_send_probe_requires_explicit_confirmation() {
    let value = run_cli(&[
        "wintun-packet-send-probe",
        "--adapter-name",
        "LocalAreaInterconnection",
        "--source-ip",
        "10.77.12.2",
        "--destination-ip",
        "10.77.12.255",
        "--source-port",
        "39077",
        "--destination-port",
        "27015",
        "--message",
        "probe",
        "--broadcast",
        "true",
    ]);

    assert_eq!(value["status"], "needs-confirmation");
    assert_eq!(value["requiresElevation"], true);
    assert_eq!(value["confirmed"], false);
    assert_eq!(value["canExecuteNow"], false);
    assert_eq!(value["packet"]["payloadBytes"], 5);
    assert!(value["nextAction"].as_str().unwrap().contains("--yes true"));
}

#[test]
fn wintun_packet_receive_probe_reports_probe_status() {
    let value = run_cli(&[
        "wintun-packet-receive-probe",
        "--adapter-name",
        "LocalAreaInterconnection",
        "--max-attempts",
        "1",
        "--poll-interval-ms",
        "0",
    ]);

    assert!(!value["status"].as_str().unwrap().is_empty());
    assert_eq!(value["ring_capacity"], 131072);
    assert_eq!(value["max_attempts"], 1);
    assert_eq!(value["poll_interval_ms"], 0);
    assert!(value["opened"].is_boolean());
    assert!(value["session_started"].is_boolean());
    assert!(value["receive_attempts"].is_u64());
    assert!(value["packet_received"].is_boolean());
    assert!(value["packet_released"].is_boolean());
    assert!(value["session_ended"].is_boolean());
    assert!(value["closed"].is_boolean());
}
