use clap::{CommandFactory, Parser, Subcommand};
use lai_core::{
    add_room_member, close_room, create_diagnostic_export_bundle, create_game_network_plan,
    create_invite, create_join_plan, create_room, create_room_session,
    create_windows_firewall_plan, create_windows_virtual_adapter_plan, decode_invite,
    evaluate_firewall_diagnostics, evaluate_network_observations, observation_from_expected_rule,
    parse_netsh_adapter_observation, parse_netsh_firewall_rules, parse_windows_ping_observation,
    AdapterObservation, CompatibilityLevel, DiagnosticExportEnvironment, DiagnosticExportInputs,
    DiagnosticExportSources, DiagnosticSnapshot, DiagnosticTextSource, DiscoveryMode, FirewallRule,
    FirewallRuleObservation, GameProfile, Ipv4Subnet, NetworkObservationSnapshot,
    PacketObservation, TunnelObservation,
};
use std::fs;
use std::net::Ipv4Addr;
use std::path::Path;
use std::process::Command as ProcessCommand;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Parser)]
#[command(name = "lai-cli")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    Init {
        #[arg(long, default_value = "LAN Room")]
        room_name: String,
        #[arg(long, default_value = "Host")]
        host: String,
    },
    Decode {
        #[arg(long)]
        invite: String,
    },
    Join {
        #[arg(long)]
        invite: String,
    },
    RoomSummary {
        #[arg(long, default_value = "LAN Room")]
        room_name: String,
        #[arg(long, default_value = "Host")]
        host: String,
        #[arg(long = "peer")]
        peers: Vec<String>,
        #[arg(long, default_value_t = false, action = clap::ArgAction::Set)]
        close: bool,
    },
    Diagnose {
        #[arg(long)]
        p2p: Option<String>,
        #[arg(long)]
        firewall: Option<String>,
    },
    GamePlan {
        #[arg(long, default_value = "Generic LAN Game")]
        game_name: String,
        #[arg(long)]
        subnet: String,
        #[arg(long, default_value = "udp_broadcast")]
        discovery: String,
        #[arg(long, default_value = "")]
        ports: String,
        #[arg(long, default_value = "unknown")]
        compatibility: String,
        #[arg(long)]
        host_ip: Option<String>,
        #[arg(long)]
        local_ip: Option<String>,
    },
    FirewallPlan {
        #[arg(long, default_value = "Generic LAN Game")]
        game_name: String,
        #[arg(long)]
        subnet: String,
        #[arg(long, default_value = "manual_ports")]
        discovery: String,
        #[arg(long, default_value = "")]
        ports: String,
        #[arg(long, default_value = "unknown")]
        compatibility: String,
        #[arg(long)]
        program: Option<String>,
    },
    FirewallDiagnose {
        #[arg(long, default_value = "Generic LAN Game")]
        game_name: String,
        #[arg(long)]
        subnet: String,
        #[arg(long, default_value = "manual_ports")]
        discovery: String,
        #[arg(long, default_value = "")]
        ports: String,
        #[arg(long, default_value = "unknown")]
        compatibility: String,
        #[arg(long, default_value = "")]
        observed: String,
        #[arg(long)]
        netsh_output: Option<String>,
        #[arg(long)]
        program: Option<String>,
    },
    AdapterPlan {
        #[arg(long, default_value = "LocalAreaInterconnection")]
        adapter_name: String,
        #[arg(long)]
        subnet: String,
        #[arg(long)]
        ip: String,
        #[arg(long, default_value_t = 1420)]
        mtu: u16,
        #[arg(long, default_value_t = 5)]
        metric: u16,
    },
    NetworkObserve {
        #[arg(long)]
        adapter_name: Option<String>,
        #[arg(long, default_value_t = true)]
        adapter_enabled: bool,
        #[arg(long)]
        expected_ip: Option<String>,
        #[arg(long)]
        assigned_ip: Option<String>,
        #[arg(long)]
        subnet: Option<String>,
        #[arg(long)]
        adapter_netsh_output: Option<String>,
        #[arg(long, default_value = "connected")]
        tunnel_state: String,
        #[arg(long, default_value_t = 0)]
        connected_peers: u16,
        #[arg(long, default_value_t = 0)]
        expected_peers: u16,
        #[arg(long)]
        latency_ms: Option<u32>,
        #[arg(long)]
        packet_loss_percent: Option<f32>,
        #[arg(long)]
        ping_output: Option<String>,
        #[arg(long, default_value = "")]
        broadcast_ports: String,
        #[arg(long, default_value = "")]
        game_ports: String,
        #[arg(long, default_value = "")]
        packets: String,
        #[arg(long)]
        packet_observations: Option<String>,
    },
    DiagnosticExport {
        #[arg(long)]
        out: String,
        #[arg(long, default_value = "LocalAreaInterconnection")]
        adapter_name: String,
        #[arg(long)]
        expected_ip: Option<String>,
        #[arg(long)]
        assigned_ip: Option<String>,
        #[arg(long)]
        subnet: Option<String>,
        #[arg(long)]
        adapter_netsh_output: Option<String>,
        #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
        adapter_scan: bool,
        #[arg(long)]
        firewall_netsh_output: Option<String>,
        #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
        firewall_scan: bool,
        #[arg(long)]
        ping_test: Option<String>,
        #[arg(long)]
        ping_output: Option<String>,
        #[arg(long, default_value_t = 0)]
        expected_peers: u16,
        #[arg(long, default_value = "")]
        broadcast_ports: String,
        #[arg(long, default_value = "")]
        game_ports: String,
        #[arg(long, default_value = "")]
        packets: String,
        #[arg(long)]
        packet_observations: Option<String>,
        #[arg(long, default_value = "Generic LAN Game")]
        game_name: String,
        #[arg(long, default_value = "manual_ports")]
        discovery: String,
        #[arg(long, default_value = "")]
        ports: String,
        #[arg(long, default_value = "unknown")]
        compatibility: String,
        #[arg(long)]
        program: Option<String>,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let Some(command) = cli.command else {
        Cli::command().print_help()?;
        println!();
        return Ok(());
    };
    match command {
        Command::Init { room_name, host } => {
            let room = create_room(room_name, host, &[])?;
            let invite = create_invite(&room)?;
            println!("{}", serde_json::to_string_pretty(&(room, invite))?);
        }
        Command::Decode { invite } => {
            println!(
                "{}",
                serde_json::to_string_pretty(&decode_invite(&invite)?)?
            );
        }
        Command::Join { invite } => {
            println!(
                "{}",
                serde_json::to_string_pretty(&create_join_plan(&invite, 0)?)?
            );
        }
        Command::RoomSummary {
            room_name,
            host,
            peers,
            close,
        } => {
            let room = create_room(room_name, host, &[])?;
            let mut session = create_room_session(&room, current_epoch_ms())?;
            for (index, peer_name) in peers.iter().enumerate() {
                add_room_member(
                    &mut session,
                    peer_name,
                    format!("peer_{}", index + 1),
                    index as u32,
                    current_epoch_ms(),
                )?;
            }
            if close {
                close_room(&mut session, current_epoch_ms())?;
            }
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "session": session,
                    "summary": session.summary(),
                }))?
            );
        }
        Command::Diagnose { p2p, firewall } => {
            let report = lai_core::evaluate_diagnostics(DiagnosticSnapshot {
                p2p,
                firewall,
                ..DiagnosticSnapshot::default()
            });
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        Command::GamePlan {
            game_name,
            subnet,
            discovery,
            ports,
            compatibility,
            host_ip,
            local_ip,
        } => {
            let profile = profile_from_args(game_name, discovery, ports, compatibility)?;
            let subnet = subnet.parse::<Ipv4Subnet>()?;
            let plan = create_game_network_plan(
                &profile,
                subnet,
                parse_optional_ipv4(host_ip.as_deref())?,
                parse_optional_ipv4(local_ip.as_deref())?,
                30,
            );
            println!("{}", serde_json::to_string_pretty(&plan)?);
        }
        Command::FirewallPlan {
            game_name,
            subnet,
            discovery,
            ports,
            compatibility,
            program,
        } => {
            let profile = profile_from_args(game_name, discovery, ports, compatibility)?;
            let subnet = subnet.parse::<Ipv4Subnet>()?;
            let network_plan = create_game_network_plan(&profile, subnet, None, None, 30);
            let firewall_plan = create_windows_firewall_plan(
                &network_plan.firewall_rules,
                "LocalAreaInterconnection",
                program,
            );
            println!("{}", serde_json::to_string_pretty(&firewall_plan)?);
        }
        Command::FirewallDiagnose {
            game_name,
            subnet,
            discovery,
            ports,
            compatibility,
            observed,
            netsh_output,
            program,
        } => {
            let profile = profile_from_args(game_name, discovery, ports, compatibility)?;
            let subnet = subnet.parse::<Ipv4Subnet>()?;
            let network_plan = create_game_network_plan(&profile, subnet, None, None, 30);
            let observed_rules = if let Some(path) = netsh_output {
                parse_netsh_firewall_rules(&fs::read_to_string(path)?)
            } else {
                observed_firewall_rules(&network_plan.firewall_rules, &observed, program.clone())?
            };
            let report = evaluate_firewall_diagnostics(
                &network_plan.firewall_rules,
                &observed_rules,
                program.as_deref(),
            );
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        Command::AdapterPlan {
            adapter_name,
            subnet,
            ip,
            mtu,
            metric,
        } => {
            let subnet = subnet.parse::<Ipv4Subnet>()?;
            let ip = ip.parse::<Ipv4Addr>()?;
            let plan = create_windows_virtual_adapter_plan(adapter_name, subnet, ip, mtu, metric);
            println!("{}", serde_json::to_string_pretty(&plan)?);
        }
        Command::NetworkObserve {
            adapter_name,
            adapter_enabled,
            expected_ip,
            assigned_ip,
            subnet,
            adapter_netsh_output,
            tunnel_state,
            connected_peers,
            expected_peers,
            latency_ms,
            packet_loss_percent,
            ping_output,
            broadcast_ports,
            game_ports,
            packets,
            packet_observations,
        } => {
            let expected_ip = parse_optional_ipv4(expected_ip.as_deref())?;
            let expected_subnet = parse_optional_subnet(subnet.as_deref())?;
            let adapter = if let Some(path) = adapter_netsh_output {
                parse_netsh_adapter_observation(
                    adapter_name.unwrap_or_else(|| "LocalAreaInterconnection".to_owned()),
                    &fs::read_to_string(path)?,
                    expected_ip,
                    expected_subnet,
                )
            } else {
                adapter_name
                    .map(|adapter_name| {
                        Ok::<_, Box<dyn std::error::Error>>(AdapterObservation {
                            adapter_name,
                            enabled: adapter_enabled,
                            expected_ip,
                            assigned_ip: parse_optional_ipv4(assigned_ip.as_deref())?,
                            virtual_subnet: expected_subnet,
                            mtu: None,
                            interface_metric: None,
                        })
                    })
                    .transpose()?
            };
            let mut packet_observations_data = if let Some(path) = packet_observations {
                lai_core::parse_packet_observation_lines(&fs::read_to_string(path)?)?
            } else {
                Vec::new()
            };
            packet_observations_data.extend(parse_packet_observations(&packets)?);
            let report = evaluate_network_observations(NetworkObservationSnapshot {
                adapter,
                tunnel: Some(if let Some(path) = ping_output {
                    parse_windows_ping_observation(&fs::read_to_string(path)?, expected_peers)
                } else {
                    TunnelObservation {
                        state: tunnel_state,
                        connected_peer_count: connected_peers,
                        latency_ms,
                        packet_loss_percent,
                        path: None,
                    }
                }),
                packets: packet_observations_data,
                expected_peer_count: expected_peers,
                expected_broadcast_ports: parse_ports(&broadcast_ports)?,
                expected_game_ports: parse_ports(&game_ports)?,
            });
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        Command::DiagnosticExport {
            out,
            adapter_name,
            expected_ip,
            assigned_ip,
            subnet,
            adapter_netsh_output,
            adapter_scan,
            firewall_netsh_output,
            firewall_scan,
            ping_test,
            ping_output,
            expected_peers,
            broadcast_ports,
            game_ports,
            packets,
            packet_observations,
            game_name,
            discovery,
            ports,
            compatibility,
            program,
        } => {
            let expected_ip = parse_optional_ipv4(expected_ip.as_deref())?;
            let assigned_ip = parse_optional_ipv4(assigned_ip.as_deref())?;
            let subnet = parse_optional_subnet(subnet.as_deref())?;
            let broadcast_ports = parse_ports(&broadcast_ports)?;
            let game_ports = parse_ports(&game_ports)?;
            let packet_observations_path = packet_observations.clone();
            let packet_data = load_packet_observations(packet_observations.as_deref(), &packets);
            let inputs = DiagnosticExportInputs {
                adapter_name: adapter_name.clone(),
                expected_ip,
                assigned_ip,
                subnet,
                expected_peers,
                ping_host: ping_test.clone(),
                packet_observations: packet_observations_path,
                broadcast_ports,
                game_ports,
                game_name,
                discovery: parse_discovery(&discovery)?,
                ports: parse_ports(&ports)?,
                compatibility: parse_compatibility(&compatibility)?,
                program,
            };
            let sources = DiagnosticExportSources {
                adapter_netsh: load_adapter_source(
                    &adapter_name,
                    adapter_netsh_output.as_deref(),
                    adapter_scan,
                ),
                firewall_netsh: load_firewall_source(
                    firewall_netsh_output.as_deref(),
                    firewall_scan,
                ),
                ping_output: load_ping_source(ping_output.as_deref(), ping_test.as_deref()),
                packets: packet_data.packets,
                packet_raw_lines: packet_data.raw_lines,
                packet_error: packet_data.error,
            };
            let bundle = create_diagnostic_export_bundle(
                current_epoch_ms(),
                diagnostic_environment()?,
                inputs,
                sources,
            );
            write_json_file(&out, &bundle)?;
            let bytes_written = fs::metadata(&out)?.len();
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "status": "ok",
                    "path": fs::canonicalize(&out)
                        .unwrap_or_else(|_| Path::new(&out).to_path_buf())
                        .display()
                        .to_string(),
                    "bytesWritten": bytes_written,
                    "bundleStatus": bundle.status,
                }))?
            );
        }
    }
    Ok(())
}

fn profile_from_args(
    game_name: String,
    discovery: String,
    ports: String,
    compatibility: String,
) -> Result<GameProfile, Box<dyn std::error::Error>> {
    Ok(GameProfile {
        game_name,
        steam_app_id: None,
        discovery: parse_discovery(&discovery)?,
        ports: parse_ports(&ports)?,
        join_method: "lan_list_or_direct_ip".to_owned(),
        compatibility: parse_compatibility(&compatibility)?,
        notes: String::new(),
    })
}

fn parse_ports(value: &str) -> Result<Vec<u16>, Box<dyn std::error::Error>> {
    if value.trim().is_empty() {
        return Ok(Vec::new());
    }
    value
        .split(',')
        .filter(|item| !item.trim().is_empty())
        .map(|item| {
            item.trim()
                .parse::<u16>()
                .map_err(|err| invalid_input(format!("invalid port `{item}`: {err}")))
        })
        .collect()
}

fn parse_discovery(value: &str) -> Result<DiscoveryMode, Box<dyn std::error::Error>> {
    match value {
        "udp_broadcast" => Ok(DiscoveryMode::UdpBroadcast),
        "direct_ip" => Ok(DiscoveryMode::DirectIp),
        "manual_ports" => Ok(DiscoveryMode::ManualPorts),
        "unknown" => Ok(DiscoveryMode::Unknown),
        other => Err(invalid_input(format!(
            "unsupported discovery mode `{other}`"
        ))),
    }
}

fn parse_compatibility(value: &str) -> Result<CompatibilityLevel, Box<dyn std::error::Error>> {
    match value {
        "A" | "a" => Ok(CompatibilityLevel::A),
        "B" | "b" => Ok(CompatibilityLevel::B),
        "C" | "c" => Ok(CompatibilityLevel::C),
        "D" | "d" => Ok(CompatibilityLevel::D),
        "unknown" => Ok(CompatibilityLevel::Unknown),
        other => Err(invalid_input(format!(
            "unsupported compatibility level `{other}`"
        ))),
    }
}

fn parse_optional_ipv4(
    value: Option<&str>,
) -> Result<Option<Ipv4Addr>, Box<dyn std::error::Error>> {
    value
        .map(|item| {
            item.parse::<Ipv4Addr>()
                .map_err(|err| invalid_input(format!("invalid IPv4 address `{item}`: {err}")))
        })
        .transpose()
}

fn parse_optional_subnet(
    value: Option<&str>,
) -> Result<Option<Ipv4Subnet>, Box<dyn std::error::Error>> {
    value
        .map(|item| {
            item.parse::<Ipv4Subnet>()
                .map_err(|err| invalid_input(format!("invalid CIDR `{item}`: {err}")))
        })
        .transpose()
}

fn parse_packet_observations(
    value: &str,
) -> Result<Vec<PacketObservation>, Box<dyn std::error::Error>> {
    if value.trim().is_empty() {
        return Ok(Vec::new());
    }
    Ok(value
        .split(',')
        .filter(|item| !item.trim().is_empty())
        .map(lai_core::parse_packet_observation_line)
        .collect::<lai_core::Result<Vec<_>>>()?)
}

fn observed_firewall_rules(
    expected_rules: &[FirewallRule],
    observed: &str,
    program: Option<String>,
) -> Result<Vec<FirewallRuleObservation>, Box<dyn std::error::Error>> {
    if observed.trim().is_empty() {
        return Ok(Vec::new());
    }
    let observed_ports = observed
        .split(',')
        .filter(|item| !item.trim().is_empty())
        .map(parse_observed_port)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(expected_rules
        .iter()
        .filter(|rule| {
            observed_ports.iter().any(|(protocol, port)| {
                rule.protocol.eq_ignore_ascii_case(protocol) && rule.port == *port
            })
        })
        .map(|rule| observation_from_expected_rule(rule, program.clone()))
        .collect())
}

fn parse_observed_port(value: &str) -> Result<(String, u16), Box<dyn std::error::Error>> {
    let (protocol, port) = value.trim().split_once(':').ok_or_else(|| {
        invalid_input(format!(
            "invalid observed rule `{value}`, expected protocol:port"
        ))
    })?;
    let protocol = protocol.trim().to_lowercase();
    if protocol != "udp" && protocol != "tcp" {
        return Err(invalid_input(format!(
            "unsupported observed protocol `{protocol}`"
        )));
    }
    let port = port
        .trim()
        .parse::<u16>()
        .map_err(|err| invalid_input(format!("invalid observed port `{port}`: {err}")))?;
    Ok((protocol, port))
}

struct PacketLoadResult {
    packets: Vec<PacketObservation>,
    raw_lines: Vec<String>,
    error: Option<String>,
}

fn load_adapter_source(
    adapter_name: &str,
    adapter_netsh_output: Option<&str>,
    adapter_scan: bool,
) -> DiagnosticTextSource {
    if let Some(path) = adapter_netsh_output {
        return read_text_source("netsh-file", path);
    }
    if adapter_scan {
        return run_text_source(
            "netsh-scan",
            "netsh",
            &[
                "interface",
                "ipv4",
                "show",
                "config",
                &format!("name={adapter_name}"),
            ],
        );
    }
    DiagnosticTextSource {
        source: "manual-input".to_owned(),
        raw_output: String::new(),
        error: None,
    }
}

fn load_firewall_source(
    firewall_netsh_output: Option<&str>,
    firewall_scan: bool,
) -> DiagnosticTextSource {
    if let Some(path) = firewall_netsh_output {
        return read_text_source("netsh-file", path);
    }
    if firewall_scan {
        return run_text_source(
            "netsh-scan",
            "netsh",
            &["advfirewall", "firewall", "show", "rule", "name=all"],
        );
    }
    DiagnosticTextSource {
        source: "skipped".to_owned(),
        raw_output: String::new(),
        error: None,
    }
}

fn load_ping_source(
    ping_output: Option<&str>,
    ping_test: Option<&str>,
) -> Option<DiagnosticTextSource> {
    if let Some(path) = ping_output {
        return Some(read_text_source("ping-file", path));
    }
    ping_test.map(|host| run_text_source("ping-test", "ping", &["-n", "4", host]))
}

fn read_text_source(source: &str, path: &str) -> DiagnosticTextSource {
    match fs::read_to_string(path) {
        Ok(raw_output) => DiagnosticTextSource {
            source: source.to_owned(),
            raw_output,
            error: None,
        },
        Err(err) => DiagnosticTextSource {
            source: source.to_owned(),
            raw_output: String::new(),
            error: Some(err.to_string()),
        },
    }
}

fn run_text_source(source: &str, program: &str, args: &[&str]) -> DiagnosticTextSource {
    match ProcessCommand::new(program).args(args).output() {
        Ok(output) => {
            let mut raw_output = String::from_utf8_lossy(&output.stdout).to_string();
            if !output.stderr.is_empty() {
                raw_output.push_str(&String::from_utf8_lossy(&output.stderr));
            }
            DiagnosticTextSource {
                source: source.to_owned(),
                raw_output,
                error: if output.status.success() {
                    None
                } else {
                    Some(format!("{program} exited with status {}", output.status))
                },
            }
        }
        Err(err) => DiagnosticTextSource {
            source: source.to_owned(),
            raw_output: String::new(),
            error: Some(err.to_string()),
        },
    }
}

fn load_packet_observations(packet_observations: Option<&str>, packets: &str) -> PacketLoadResult {
    let mut raw_lines = Vec::new();
    let mut error = None;

    if let Some(path) = packet_observations {
        match fs::read_to_string(path) {
            Ok(text) => raw_lines.extend(
                text.lines()
                    .map(str::trim)
                    .filter(|line| !line.is_empty() && !line.starts_with('#'))
                    .map(str::to_owned),
            ),
            Err(err) => error = Some(err.to_string()),
        }
    }
    raw_lines.extend(
        packets
            .split(',')
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(str::to_owned),
    );

    let mut parsed = Vec::new();
    if error.is_none() {
        for line in &raw_lines {
            match lai_core::parse_packet_observation_line(line) {
                Ok(packet) => parsed.push(packet),
                Err(err) => {
                    error = Some(err.to_string());
                    parsed.clear();
                    break;
                }
            }
        }
    }

    PacketLoadResult {
        packets: parsed,
        raw_lines,
        error,
    }
}

fn diagnostic_environment() -> Result<DiagnosticExportEnvironment, Box<dyn std::error::Error>> {
    Ok(DiagnosticExportEnvironment {
        machine_name: std::env::var("COMPUTERNAME")
            .or_else(|_| std::env::var("HOSTNAME"))
            .unwrap_or_default(),
        user_name: std::env::var("USERNAME")
            .or_else(|_| std::env::var("USER"))
            .unwrap_or_default(),
        os_version: std::env::consts::OS.to_owned(),
        current_directory: std::env::current_dir()?.display().to_string(),
        architecture: std::env::consts::ARCH.to_owned(),
    })
}

fn current_epoch_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}

fn write_json_file<T: serde::Serialize>(
    path: &str,
    value: &T,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = Path::new(path).parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    fs::write(path, format!("{}\n", serde_json::to_string_pretty(value)?))?;
    Ok(())
}

fn invalid_input(message: String) -> Box<dyn std::error::Error> {
    std::io::Error::new(std::io::ErrorKind::InvalidInput, message).into()
}
