use crate::ip::{broadcast_address, Ipv4Subnet};
use serde::{Deserialize, Serialize};
use std::net::Ipv4Addr;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct VirtualAdapterPlan {
    pub platform: String,
    pub dry_run: bool,
    pub adapter_name: String,
    pub virtual_subnet: String,
    pub assigned_ip: Ipv4Addr,
    pub subnet_mask: Ipv4Addr,
    pub mtu: u16,
    pub interface_metric: u16,
    pub requires_elevation: bool,
    pub commands: Vec<NetworkCommand>,
    pub verification_checks: Vec<String>,
    pub warnings: Vec<AdapterPlanWarning>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct NetworkCommand {
    pub tool: String,
    pub args: Vec<String>,
    pub command: String,
    pub purpose: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AdapterPlanWarning {
    pub key: String,
    pub message: String,
}

pub fn create_windows_virtual_adapter_plan(
    adapter_name: impl Into<String>,
    virtual_subnet: Ipv4Subnet,
    assigned_ip: Ipv4Addr,
    mtu: u16,
    interface_metric: u16,
) -> VirtualAdapterPlan {
    let adapter_name = adapter_name.into();
    let subnet_mask = subnet_mask_for_prefix(virtual_subnet.prefix);
    let commands = vec![
        set_static_address_command(&adapter_name, assigned_ip, subnet_mask),
        set_mtu_command(&adapter_name, mtu),
        set_metric_command(&adapter_name, interface_metric),
        show_config_command(&adapter_name),
    ];

    VirtualAdapterPlan {
        platform: "windows".to_owned(),
        dry_run: true,
        adapter_name,
        virtual_subnet: virtual_subnet.to_string(),
        assigned_ip,
        subnet_mask,
        mtu,
        interface_metric,
        requires_elevation: true,
        commands,
        verification_checks: vec![
            "Adapter exists and is enabled.".to_owned(),
            "Adapter IPv4 address matches the assigned room IP.".to_owned(),
            "Adapter subnet mask matches the room prefix.".to_owned(),
            "Adapter MTU matches the tunnel packet budget.".to_owned(),
            "Adapter metric keeps the virtual LAN preferred for room traffic.".to_owned(),
        ],
        warnings: adapter_warnings(virtual_subnet, assigned_ip, mtu, interface_metric),
    }
}

fn set_static_address_command(
    adapter_name: &str,
    assigned_ip: Ipv4Addr,
    subnet_mask: Ipv4Addr,
) -> NetworkCommand {
    let args = vec![
        "interface".to_owned(),
        "ipv4".to_owned(),
        "set".to_owned(),
        "address".to_owned(),
        assignment("name", adapter_name),
        "static".to_owned(),
        assigned_ip.to_string(),
        subnet_mask.to_string(),
    ];
    command(args, "Assign the room IPv4 address to the virtual adapter.")
}

fn set_mtu_command(adapter_name: &str, mtu: u16) -> NetworkCommand {
    let args = vec![
        "interface".to_owned(),
        "ipv4".to_owned(),
        "set".to_owned(),
        "subinterface".to_owned(),
        quote_value(adapter_name),
        assignment("mtu", &mtu.to_string()),
        "store=persistent".to_owned(),
    ];
    command(args, "Set the virtual adapter MTU.")
}

fn set_metric_command(adapter_name: &str, interface_metric: u16) -> NetworkCommand {
    let args = vec![
        "interface".to_owned(),
        "ipv4".to_owned(),
        "set".to_owned(),
        "interface".to_owned(),
        quote_value(adapter_name),
        assignment("metric", &interface_metric.to_string()),
    ];
    command(args, "Set the virtual adapter interface metric.")
}

fn show_config_command(adapter_name: &str) -> NetworkCommand {
    let args = vec![
        "interface".to_owned(),
        "ipv4".to_owned(),
        "show".to_owned(),
        "config".to_owned(),
        assignment("name", adapter_name),
    ];
    command(args, "Verify the virtual adapter IPv4 configuration.")
}

fn command(args: Vec<String>, purpose: &str) -> NetworkCommand {
    NetworkCommand {
        tool: "netsh".to_owned(),
        command: format_command("netsh", &args),
        args,
        purpose: purpose.to_owned(),
    }
}

fn adapter_warnings(
    virtual_subnet: Ipv4Subnet,
    assigned_ip: Ipv4Addr,
    mtu: u16,
    interface_metric: u16,
) -> Vec<AdapterPlanWarning> {
    let mut warnings = Vec::new();
    if !virtual_subnet.contains(assigned_ip) {
        warnings.push(AdapterPlanWarning {
            key: "ip-outside-subnet".to_owned(),
            message: "Assigned IP is outside the room virtual subnet.".to_owned(),
        });
    }
    if assigned_ip == virtual_subnet.network || assigned_ip == broadcast_address(virtual_subnet) {
        warnings.push(AdapterPlanWarning {
            key: "reserved-ip".to_owned(),
            message: "Assigned IP is the subnet network or broadcast address.".to_owned(),
        });
    }
    if mtu < 1200 {
        warnings.push(AdapterPlanWarning {
            key: "low-mtu".to_owned(),
            message: "MTU is unusually low and may reduce game traffic efficiency.".to_owned(),
        });
    }
    if interface_metric > 50 {
        warnings.push(AdapterPlanWarning {
            key: "high-interface-metric".to_owned(),
            message: "Interface metric is high; some games may prefer another adapter.".to_owned(),
        });
    }
    warnings
}

fn subnet_mask_for_prefix(prefix: u8) -> Ipv4Addr {
    if prefix == 0 {
        Ipv4Addr::from(0)
    } else {
        Ipv4Addr::from(u32::MAX << (32 - prefix))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn virtual_adapter_plan_renders_windows_netsh_commands() {
        let plan = create_windows_virtual_adapter_plan(
            "LocalAreaInterconnection",
            "10.77.12.0/24".parse().unwrap(),
            "10.77.12.2".parse().unwrap(),
            1420,
            5,
        );

        assert_eq!(plan.platform, "windows");
        assert!(plan.dry_run);
        assert_eq!(
            plan.subnet_mask,
            "255.255.255.0".parse::<Ipv4Addr>().unwrap()
        );
        assert_eq!(plan.commands.len(), 4);
        assert!(plan.commands[0]
            .command
            .contains("set address name=LocalAreaInterconnection static 10.77.12.2 255.255.255.0"));
        assert!(plan.commands[1].command.contains("mtu=1420"));
        assert!(plan.commands[2].command.contains("metric=5"));
        assert!(plan.warnings.is_empty());
    }

    #[test]
    fn virtual_adapter_plan_warns_for_bad_ip_and_metric() {
        let plan = create_windows_virtual_adapter_plan(
            "LocalAreaInterconnection",
            "10.77.12.0/24".parse().unwrap(),
            "10.77.99.2".parse().unwrap(),
            1100,
            100,
        );

        assert!(plan
            .warnings
            .iter()
            .any(|warning| warning.key == "ip-outside-subnet"));
        assert!(plan.warnings.iter().any(|warning| warning.key == "low-mtu"));
        assert!(plan
            .warnings
            .iter()
            .any(|warning| warning.key == "high-interface-metric"));
    }
}
