use serde_json::Value;
use std::fs;
use std::process::Command;

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

    assert_eq!(bundle["schema_version"], 1);
    assert_eq!(bundle["tool"], "LocalAreaInterconnection Rust CLI");
    assert_eq!(bundle["adapter_scan"]["status"], "ok");
    assert_eq!(bundle["packet_observations"]["broadcast_count"], 1);
    assert_eq!(bundle["packet_observations"]["game_traffic_count"], 1);
}
