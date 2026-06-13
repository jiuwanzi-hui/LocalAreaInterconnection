use crate::ip::Ipv4Subnet;
use crate::network_observation::AdapterObservation;
use crate::virtual_adapter_plan::NetworkCommand;
use crate::windows_route_parser::WindowsRouteObservation;
use crate::wintun_runtime::WintunPacketIoCloseReport;
use serde::{Deserialize, Serialize};
use std::net::Ipv4Addr;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RuntimeCleanupPlan {
    pub platform: String,
    pub dry_run: bool,
    pub room_id: String,
    pub local_peer_id: String,
    pub local_virtual_ip: Ipv4Addr,
    #[serde(default)]
    pub virtual_subnet: Option<Ipv4Subnet>,
    pub adapter_name: String,
    pub packet_io_backend: String,
    pub restore_adapter: bool,
    #[serde(default)]
    pub cleanup_routes: bool,
    pub requires_elevation: bool,
    pub process_cleanup_steps: Vec<RuntimeCleanupStep>,
    pub commands: Vec<NetworkCommand>,
    pub verification_checks: Vec<String>,
    pub warnings: Vec<RuntimeCleanupWarning>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RuntimeCleanupStep {
    pub key: String,
    pub status: String,
    pub detail: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RuntimeCleanupWarning {
    pub key: String,
    pub message: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RuntimeCleanupReport {
    pub status: String,
    pub summary: String,
    pub plan: RuntimeCleanupPlan,
    pub adapter_observation: Option<AdapterObservation>,
    pub route_observations: Vec<WindowsRouteObservation>,
    pub wintun_close: Option<WintunPacketIoCloseReport>,
    pub checks: Vec<RuntimeCleanupCheck>,
    pub next_actions: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RuntimeCleanupCheck {
    pub key: String,
    pub status: String,
    pub message: String,
    pub next_action: String,
}

pub fn create_windows_runtime_cleanup_plan(
    room_id: impl Into<String>,
    local_peer_id: impl Into<String>,
    local_virtual_ip: Ipv4Addr,
    adapter_name: impl Into<String>,
    packet_io_backend: impl Into<String>,
    restore_adapter: bool,
) -> RuntimeCleanupPlan {
    create_windows_runtime_cleanup_plan_with_routes(
        room_id,
        local_peer_id,
        local_virtual_ip,
        None,
        adapter_name,
        packet_io_backend,
        restore_adapter,
        false,
    )
}

pub fn create_windows_runtime_cleanup_plan_with_routes(
    room_id: impl Into<String>,
    local_peer_id: impl Into<String>,
    local_virtual_ip: Ipv4Addr,
    virtual_subnet: Option<Ipv4Subnet>,
    adapter_name: impl Into<String>,
    packet_io_backend: impl Into<String>,
    restore_adapter: bool,
    cleanup_routes: bool,
) -> RuntimeCleanupPlan {
    let adapter_name = adapter_name.into();
    let packet_io_backend = packet_io_backend.into();
    let mut process_cleanup_steps = vec![
        cleanup_step(
            "stop-runtime-loop",
            "automatic",
            "Stop heartbeat, coordination monitor and packet forwarding loops.",
        ),
        cleanup_step(
            "close-tunnel-socket",
            "automatic",
            "Drop the runtime UDP tunnel socket so the bind endpoint is released.",
        ),
        cleanup_step(
            "close-capture-sockets",
            "automatic",
            "Drop UDP capture and broadcast sockets for configured game ports.",
        ),
    ];

    if packet_io_backend == "wintun" {
        process_cleanup_steps.push(cleanup_step(
            "close-wintun-session",
            "automatic",
            "Close the Wintun packet I/O session and release its ring buffer.",
        ));
    } else {
        process_cleanup_steps.push(cleanup_step(
            "close-packet-io-session",
            "not-required",
            "The selected packet I/O backend does not keep a native adapter session open.",
        ));
    }

    let mut commands = Vec::new();
    if restore_adapter {
        commands.extend([
            reset_address_command(&adapter_name),
            reset_metric_command(&adapter_name),
            reset_mtu_command(&adapter_name),
            show_config_command(&adapter_name),
        ]);
    }
    if cleanup_routes {
        commands.extend(route_cleanup_commands(local_virtual_ip, virtual_subnet));
    }

    let mut verification_checks = vec![
        "Runtime process has exited and no longer owns the tunnel bind endpoint.".to_owned(),
        "Game and broadcast capture ports are no longer bound by the runtime.".to_owned(),
        "Packet I/O session is closed or was not required by the selected backend.".to_owned(),
    ];
    if restore_adapter {
        verification_checks.push(
            "Virtual adapter IPv4 address, MTU and interface metric have been restored or reviewed."
                .to_owned(),
        );
    }
    if cleanup_routes {
        verification_checks
            .push("Route table no longer contains stale room subnet or host routes.".to_owned());
    }

    RuntimeCleanupPlan {
        platform: "windows".to_owned(),
        dry_run: true,
        room_id: room_id.into(),
        local_peer_id: local_peer_id.into(),
        local_virtual_ip,
        virtual_subnet,
        adapter_name,
        packet_io_backend,
        restore_adapter,
        cleanup_routes,
        requires_elevation: restore_adapter || cleanup_routes,
        process_cleanup_steps,
        commands,
        verification_checks,
        warnings: cleanup_warnings(restore_adapter, cleanup_routes),
    }
}

pub fn create_runtime_cleanup_report(
    plan: RuntimeCleanupPlan,
    adapter_observation: Option<AdapterObservation>,
    route_observations: Vec<WindowsRouteObservation>,
    wintun_close: Option<WintunPacketIoCloseReport>,
) -> RuntimeCleanupReport {
    let checks = vec![
        runtime_process_check(&plan),
        wintun_close_check(&plan, wintun_close.as_ref()),
        adapter_cleanup_check(&plan, adapter_observation.as_ref()),
        route_cleanup_check(&plan, &route_observations),
        command_cleanup_check(&plan, adapter_observation.as_ref(), &route_observations),
    ];
    let problem_count = checks
        .iter()
        .filter(|check| !matches!(check.status.as_str(), "ok" | "skipped"))
        .count();
    let status = if problem_count == 0 {
        "ok"
    } else {
        "needs-attention"
    }
    .to_owned();
    let next_actions = checks
        .iter()
        .filter(|check| !matches!(check.status.as_str(), "ok" | "skipped"))
        .map(|check| check.next_action.clone())
        .collect::<Vec<_>>();

    RuntimeCleanupReport {
        status: status.clone(),
        summary: if status == "ok" {
            "Runtime cleanup evidence looks complete.".to_owned()
        } else {
            format!("Runtime cleanup has {problem_count} item(s) needing attention.")
        },
        plan,
        adapter_observation,
        route_observations,
        wintun_close,
        checks,
        next_actions,
    }
}

fn route_cleanup_check(
    plan: &RuntimeCleanupPlan,
    routes: &[WindowsRouteObservation],
) -> RuntimeCleanupCheck {
    let residual_routes = residual_room_route_count(plan, routes);
    if residual_routes == 0 {
        cleanup_check(
            "route-cleanup",
            "ok",
            "No runtime room route residue was observed.",
            "No route cleanup action is required.",
        )
    } else {
        cleanup_check(
            "route-cleanup",
            "needs-attention",
            "Route table still contains route(s) that cover the runtime room IP.",
            "Review route print output and remove stale room routes from an Administrator terminal if needed.",
        )
    }
}

fn residual_room_route_count(
    plan: &RuntimeCleanupPlan,
    routes: &[WindowsRouteObservation],
) -> usize {
    routes
        .iter()
        .filter(|route| route.destination.prefix > 0)
        .filter(|route| route.destination.contains(plan.local_virtual_ip))
        .count()
}

fn runtime_process_check(plan: &RuntimeCleanupPlan) -> RuntimeCleanupCheck {
    let automatic_steps = plan
        .process_cleanup_steps
        .iter()
        .filter(|step| matches!(step.status.as_str(), "automatic" | "not-required"))
        .count();
    let status = if automatic_steps == plan.process_cleanup_steps.len() {
        "ok"
    } else {
        "needs-attention"
    };
    cleanup_check(
        "runtime-process",
        status,
        "Runtime process cleanup steps are automatic or not required.",
        "Inspect runtime logs and stop the process before checking adapter cleanup.",
    )
}

fn wintun_close_check(
    plan: &RuntimeCleanupPlan,
    close: Option<&WintunPacketIoCloseReport>,
) -> RuntimeCleanupCheck {
    if plan.packet_io_backend != "wintun" {
        return cleanup_check(
            "wintun-session",
            "skipped",
            "Wintun session cleanup is not required for this packet I/O backend.",
            "No Wintun cleanup action is required.",
        );
    }
    match close {
        Some(close) if close.closed && close.session_ended => cleanup_check(
            "wintun-session",
            "ok",
            "Wintun packet I/O session was closed.",
            "No Wintun session action is required.",
        ),
        Some(_) => cleanup_check(
            "wintun-session",
            "needs-attention",
            "Wintun packet I/O close report did not confirm session shutdown.",
            "Stop the runtime process, then inspect Wintun session close output.",
        ),
        None => cleanup_check(
            "wintun-session",
            "needs-attention",
            "No Wintun close report was available.",
            "Export diagnostics after the runtime exits so wintunRuntime.close is available.",
        ),
    }
}

fn adapter_cleanup_check(
    plan: &RuntimeCleanupPlan,
    observation: Option<&AdapterObservation>,
) -> RuntimeCleanupCheck {
    if !plan.restore_adapter {
        return cleanup_check(
            "adapter-restore",
            "skipped",
            "Adapter restore was not requested by the cleanup plan.",
            "Run runtime-cleanup-plan with --restore-adapter true if the adapter should be reset.",
        );
    }
    let Some(observation) = observation else {
        return cleanup_check(
            "adapter-restore",
            "needs-attention",
            "No adapter observation was available after cleanup.",
            "Run adapter scan or provide netsh adapter output after cleanup.",
        );
    };
    if observation.assigned_ip == Some(plan.local_virtual_ip) {
        cleanup_check(
            "adapter-restore",
            "needs-attention",
            "Adapter still has the runtime room virtual IP after cleanup.",
            "Run the cleanup netsh commands from an Administrator terminal, then scan again.",
        )
    } else {
        cleanup_check(
            "adapter-restore",
            "ok",
            "Adapter no longer has the runtime room virtual IP.",
            "No adapter IP cleanup action is required.",
        )
    }
}

fn command_cleanup_check(
    plan: &RuntimeCleanupPlan,
    observation: Option<&AdapterObservation>,
    routes: &[WindowsRouteObservation],
) -> RuntimeCleanupCheck {
    if plan.commands.is_empty() {
        return cleanup_check(
            "cleanup-commands",
            "skipped",
            "No adapter cleanup commands were generated.",
            "No command cleanup action is required.",
        );
    }
    let adapter_goal_satisfied = !plan.restore_adapter
        || observation
            .and_then(|observation| observation.assigned_ip)
            .is_some_and(|assigned_ip| assigned_ip != plan.local_virtual_ip);
    let route_goal_satisfied = !plan.cleanup_routes || residual_room_route_count(plan, routes) == 0;
    if adapter_goal_satisfied && route_goal_satisfied {
        return cleanup_check(
            "cleanup-commands",
            "ok",
            "Cleanup command goals appear satisfied by current observations.",
            "Keep the dry-run commands in the diagnostic bundle for audit evidence.",
        );
    }
    cleanup_check(
        "cleanup-commands",
        "needs-attention",
        "Cleanup commands are still pending review or execution.",
        "Review the dry-run commands and execute them from an Administrator terminal if appropriate.",
    )
}

fn cleanup_check(key: &str, status: &str, message: &str, next_action: &str) -> RuntimeCleanupCheck {
    RuntimeCleanupCheck {
        key: key.to_owned(),
        status: status.to_owned(),
        message: message.to_owned(),
        next_action: next_action.to_owned(),
    }
}

fn cleanup_step(key: &str, status: &str, detail: &str) -> RuntimeCleanupStep {
    RuntimeCleanupStep {
        key: key.to_owned(),
        status: status.to_owned(),
        detail: detail.to_owned(),
    }
}

fn cleanup_warnings(restore_adapter: bool, cleanup_routes: bool) -> Vec<RuntimeCleanupWarning> {
    let mut warnings = Vec::new();
    if restore_adapter {
        warnings.push(RuntimeCleanupWarning {
            key: "review-before-restore".to_owned(),
            message: "Adapter restore commands are dry-run only; review them before running from an Administrator terminal.".to_owned(),
        });
    } else {
        warnings.push(RuntimeCleanupWarning {
            key: "adapter-left-configured".to_owned(),
            message: "No adapter restore commands were generated; the virtual adapter address, MTU and metric may remain configured after runtime exit.".to_owned(),
        });
    }
    if cleanup_routes {
        warnings.push(RuntimeCleanupWarning {
            key: "review-route-cleanup".to_owned(),
            message: "Route cleanup commands are dry-run only; review route print output before removing routes.".to_owned(),
        });
    }
    warnings
}

fn route_cleanup_commands(
    local_virtual_ip: Ipv4Addr,
    virtual_subnet: Option<Ipv4Subnet>,
) -> Vec<NetworkCommand> {
    let mut commands = Vec::new();
    if let Some(subnet) = virtual_subnet {
        commands.push(route_delete_command(
            subnet.network,
            subnet_mask_for_prefix(subnet.prefix),
            "Remove the room subnet route from the Windows route table.",
        ));
    }
    commands.push(route_delete_command(
        local_virtual_ip,
        Ipv4Addr::new(255, 255, 255, 255),
        "Remove the local room host route from the Windows route table.",
    ));
    commands
}

fn route_delete_command(destination: Ipv4Addr, mask: Ipv4Addr, purpose: &str) -> NetworkCommand {
    let args = vec![
        "delete".to_owned(),
        destination.to_string(),
        "mask".to_owned(),
        mask.to_string(),
    ];
    NetworkCommand {
        tool: "route".to_owned(),
        command: format_command("route", &args),
        args,
        purpose: purpose.to_owned(),
    }
}

fn reset_address_command(adapter_name: &str) -> NetworkCommand {
    let args = vec![
        "interface".to_owned(),
        "ipv4".to_owned(),
        "set".to_owned(),
        "address".to_owned(),
        assignment("name", adapter_name),
        "dhcp".to_owned(),
    ];
    command(
        args,
        "Restore the virtual adapter IPv4 address mode to DHCP.",
    )
}

fn reset_metric_command(adapter_name: &str) -> NetworkCommand {
    let args = vec![
        "interface".to_owned(),
        "ipv4".to_owned(),
        "set".to_owned(),
        "interface".to_owned(),
        quote_value(adapter_name),
        "metric=automatic".to_owned(),
    ];
    command(
        args,
        "Restore automatic interface metric on the virtual adapter.",
    )
}

fn reset_mtu_command(adapter_name: &str) -> NetworkCommand {
    let args = vec![
        "interface".to_owned(),
        "ipv4".to_owned(),
        "set".to_owned(),
        "subinterface".to_owned(),
        quote_value(adapter_name),
        "mtu=1500".to_owned(),
        "store=persistent".to_owned(),
    ];
    command(
        args,
        "Restore the virtual adapter MTU to the Windows default.",
    )
}

fn show_config_command(adapter_name: &str) -> NetworkCommand {
    let args = vec![
        "interface".to_owned(),
        "ipv4".to_owned(),
        "show".to_owned(),
        "config".to_owned(),
        assignment("name", adapter_name),
    ];
    command(
        args,
        "Verify the virtual adapter configuration after cleanup.",
    )
}

fn command(args: Vec<String>, purpose: &str) -> NetworkCommand {
    NetworkCommand {
        tool: "netsh".to_owned(),
        command: format_command("netsh", &args),
        args,
        purpose: purpose.to_owned(),
    }
}

fn assignment(key: &str, value: &str) -> String {
    format!("{key}={}", quote_value(value))
}

fn quote_value(value: &str) -> String {
    if value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.' | '/' | ':' | '\\' | '-'))
    {
        value.to_owned()
    } else {
        format!("\"{}\"", value.replace('"', "\\\""))
    }
}

fn format_command(tool: &str, args: &[String]) -> String {
    let mut parts = Vec::with_capacity(args.len() + 1);
    parts.push(tool.to_owned());
    parts.extend(args.iter().cloned());
    parts.join(" ")
}

fn subnet_mask_for_prefix(prefix: u8) -> Ipv4Addr {
    if prefix == 0 {
        Ipv4Addr::from(0)
    } else {
        Ipv4Addr::from(u32::MAX << (32 - prefix))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_cleanup_plan_reports_process_cleanup_without_adapter_restore() {
        let plan = create_windows_runtime_cleanup_plan(
            "room_1",
            "peer_a",
            "10.77.12.2".parse().unwrap(),
            "LocalAreaInterconnection",
            "userspace-udp",
            false,
        );

        assert!(plan.dry_run);
        assert!(!plan.requires_elevation);
        assert!(plan.commands.is_empty());
        assert!(plan
            .process_cleanup_steps
            .iter()
            .any(|step| step.key == "close-tunnel-socket"));
        assert!(plan
            .warnings
            .iter()
            .any(|warning| warning.key == "adapter-left-configured"));
    }

    #[test]
    fn runtime_cleanup_plan_can_render_adapter_restore_commands() {
        let plan = create_windows_runtime_cleanup_plan(
            "room_1",
            "peer_a",
            "10.77.12.2".parse().unwrap(),
            "Local Area Interconnection",
            "wintun",
            true,
        );

        assert!(plan.requires_elevation);
        assert_eq!(plan.commands.len(), 4);
        assert!(plan.commands[0]
            .command
            .contains("set address name=\"Local Area Interconnection\" dhcp"));
        assert!(plan.commands[1].command.contains("metric=automatic"));
        assert!(plan.commands[2].command.contains("mtu=1500"));
        assert!(plan
            .process_cleanup_steps
            .iter()
            .any(|step| step.key == "close-wintun-session"));
    }

    #[test]
    fn runtime_cleanup_plan_can_render_route_delete_commands() {
        let plan = create_windows_runtime_cleanup_plan_with_routes(
            "room_1",
            "peer_a",
            "10.77.12.2".parse().unwrap(),
            Some("10.77.12.0/24".parse().unwrap()),
            "LocalAreaInterconnection",
            "wintun",
            false,
            true,
        );

        assert!(plan.requires_elevation);
        assert!(plan.cleanup_routes);
        assert_eq!(plan.commands.len(), 2);
        assert_eq!(plan.commands[0].tool, "route");
        assert!(plan.commands[0]
            .command
            .contains("route delete 10.77.12.0 mask 255.255.255.0"));
        assert!(plan.commands[1]
            .command
            .contains("route delete 10.77.12.2 mask 255.255.255.255"));
    }

    #[test]
    fn runtime_cleanup_report_accepts_closed_wintun_and_restored_adapter() {
        let plan = create_windows_runtime_cleanup_plan(
            "room_1",
            "peer_a",
            "10.77.12.2".parse().unwrap(),
            "LocalAreaInterconnection",
            "wintun",
            true,
        );
        let report = create_runtime_cleanup_report(
            plan,
            Some(AdapterObservation {
                adapter_name: "LocalAreaInterconnection".to_owned(),
                enabled: true,
                expected_ip: Some("10.77.12.2".parse().unwrap()),
                assigned_ip: Some("169.254.1.10".parse().unwrap()),
                virtual_subnet: Some("10.77.12.0/24".parse().unwrap()),
                mtu: Some(1500),
                interface_metric: None,
            }),
            Vec::new(),
            Some(WintunPacketIoCloseReport {
                session_ended: true,
                closed: true,
            }),
        );

        assert_eq!(report.status, "ok");
        assert!(report.next_actions.is_empty());
        assert!(report
            .checks
            .iter()
            .any(|check| check.key == "adapter-restore" && check.status == "ok"));
    }

    #[test]
    fn runtime_cleanup_report_flags_adapter_still_using_room_ip() {
        let plan = create_windows_runtime_cleanup_plan(
            "room_1",
            "peer_a",
            "10.77.12.2".parse().unwrap(),
            "LocalAreaInterconnection",
            "wintun",
            true,
        );
        let report = create_runtime_cleanup_report(
            plan,
            Some(AdapterObservation {
                adapter_name: "LocalAreaInterconnection".to_owned(),
                enabled: true,
                expected_ip: Some("10.77.12.2".parse().unwrap()),
                assigned_ip: Some("10.77.12.2".parse().unwrap()),
                virtual_subnet: Some("10.77.12.0/24".parse().unwrap()),
                mtu: Some(1420),
                interface_metric: Some(5),
            }),
            Vec::new(),
            Some(WintunPacketIoCloseReport {
                session_ended: true,
                closed: true,
            }),
        );

        assert_eq!(report.status, "needs-attention");
        assert!(report
            .checks
            .iter()
            .any(|check| check.key == "adapter-restore" && check.status == "needs-attention"));
        assert!(report
            .next_actions
            .iter()
            .any(|action| action.contains("Administrator terminal")));
    }

    #[test]
    fn runtime_cleanup_report_flags_route_residue() {
        let plan = create_windows_runtime_cleanup_plan(
            "room_1",
            "peer_a",
            "10.77.12.2".parse().unwrap(),
            "LocalAreaInterconnection",
            "userspace-udp",
            false,
        );
        let report = create_runtime_cleanup_report(
            plan,
            None,
            vec![WindowsRouteObservation {
                destination: "10.77.12.0/24".parse().unwrap(),
                gateway: None,
                interface_ip: Some("10.77.12.2".parse().unwrap()),
                metric: Some(5),
                persistent: false,
            }],
            None,
        );

        assert_eq!(report.status, "needs-attention");
        assert!(report
            .checks
            .iter()
            .any(|check| check.key == "route-cleanup" && check.status == "needs-attention"));
    }
}
