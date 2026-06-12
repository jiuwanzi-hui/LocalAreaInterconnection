use serde_json::Value;
use std::fs;
use std::net::{TcpListener, UdpSocket};
use std::process::Command;
use std::process::Stdio;
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
fn room_runtime_run_can_bootstrap_nat_remote_peer_before_starting() {
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
        "--nat-bootstrap-attempts",
        "1",
        "--nat-bootstrap-interval-ms",
        "0",
        "--nat-bootstrap-timeout-ms",
        "2000",
    ]);
    let listener_output = listener.wait_with_output().expect("listener exits");

    assert_eq!(value["status"], "ok");
    assert_eq!(value["plan"]["tunnel"]["peer_count"], 1);
    assert_eq!(value["plan"]["peers"][0]["peer_id"], "peer_b");
    assert_eq!(
        value["plan"]["peers"][0]["endpoint"],
        listener_addr.to_string()
    );
    assert_eq!(value["natBootstrapResults"][0]["status"], "ok");
    assert_eq!(
        value["natBootstrapResults"][0]["selectedPeer"]["responderPeerId"],
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
            "2",
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
    let server_output = server.wait_with_output().expect("server exits");
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
    assert_eq!(value["tunnelServiceSnapshot"]["connected_peer_count"], 1);
    assert_eq!(
        value["networkObservation"]["diagnostic_snapshot"]["p2p"],
        "ok"
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

    let observations = fs::read_to_string(&path).expect("observation file");
    let snapshot = fs::read_to_string(&snapshot_path).expect("snapshot file");
    fs::remove_file(&path).ok();
    fs::remove_file(&snapshot_path).ok();
    assert!(observations.contains(":unicast:inbound:21"));
    assert!(observations.contains(":broadcast:inbound:21"));
    assert!(snapshot.contains("\"tunnelServiceSnapshot\""));
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

    let snapshot = fs::read_to_string(&snapshot_path).expect("runtime snapshot");
    fs::remove_file(&snapshot_path).ok();
    assert!(snapshot.contains("\"snapshotWriteCount\""));
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
    assert_eq!(bootstrap["selectedPeer"]["responderPeerId"], "peer_b");
    assert_eq!(bootstrap["selectedPeer"]["nonceMatched"], true);
    assert_eq!(bootstrap["punchPackets"].as_array().unwrap().len(), 1);
    assert_eq!(bootstrap["handshakePackets"].as_array().unwrap().len(), 1);
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

    assert_eq!(bundle["schema_version"], 2);
    assert_eq!(bundle["tool"], "LocalAreaInterconnection Rust CLI");
    assert_eq!(bundle["adapter_scan"]["status"], "ok");
    assert_eq!(bundle["packet_io"]["status"], "ok");
    assert_eq!(bundle["packet_io"]["backend"], "userspace-udp");
    assert_eq!(bundle["packet_io"]["probe"]["status"], "ready");
    assert_eq!(bundle["packet_observations"]["broadcast_count"], 1);
    assert_eq!(bundle["packet_observations"]["game_traffic_count"], 1);
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
    let bundle_path_string = bundle_path.display().to_string();
    let snapshot_path_string = snapshot_path.display().to_string();
    fs::write(
        &snapshot_path,
        serde_json::to_string_pretty(&serde_json::json!({
            "status": "ok",
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
            "injectedPackets": [{"bytes": 21}],
            "wintunRuntime": {
                "receivedPackets": [{"protocol": "udp"}],
                "sentPackets": [{"protocol": "udp"}],
                "errors": []
            }
        }))
        .unwrap(),
    )
    .expect("write runtime snapshot");

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
