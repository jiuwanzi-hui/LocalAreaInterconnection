use crate::game_network_plan::FirewallRule;
use serde::{Deserialize, Serialize};
use std::net::Ipv4Addr;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct WindowsFirewallPlan {
    pub platform: String,
    pub dry_run: bool,
    pub group_name: String,
    pub requires_elevation: bool,
    pub summary: String,
    pub commands: Vec<FirewallCommand>,
    pub rollback_commands: Vec<FirewallCommand>,
    pub warnings: Vec<FirewallWarning>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FirewallCommand {
    pub rule_name: String,
    pub tool: String,
    pub args: Vec<String>,
    pub command: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub purpose: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FirewallWarning {
    pub key: String,
    pub message: String,
}

pub fn create_windows_firewall_plan(
    rules: &[FirewallRule],
    group_name: impl Into<String>,
    program_path: Option<String>,
) -> WindowsFirewallPlan {
    let group_name = group_name.into();
    let commands = rules
        .iter()
        .map(|rule| create_add_rule_command(rule, &group_name, program_path.as_deref()))
        .collect::<Vec<_>>();
    let rollback_commands = rules
        .iter()
        .map(create_delete_rule_command)
        .collect::<Vec<_>>();

    WindowsFirewallPlan {
        platform: "windows".to_owned(),
        dry_run: true,
        group_name,
        requires_elevation: !commands.is_empty(),
        summary: if commands.is_empty() {
            "No Windows Firewall rules can be generated.".to_owned()
        } else {
            format!(
                "Will generate {} Windows Firewall inbound allow rule(s).",
                commands.len()
            )
        },
        commands,
        rollback_commands,
        warnings: create_firewall_warnings(rules, program_path.as_deref()),
    }
}

fn create_add_rule_command(
    rule: &FirewallRule,
    _group_name: &str,
    program_path: Option<&str>,
) -> FirewallCommand {
    let mut args = vec![
        "advfirewall".to_owned(),
        "firewall".to_owned(),
        "add".to_owned(),
        "rule".to_owned(),
        assignment("name", &rule.name),
        assignment("dir", direction_to_netsh(&rule.direction)),
        assignment("action", &rule.action),
        assignment("protocol", &rule.protocol.to_uppercase()),
        assignment("localport", &rule.port.to_string()),
        assignment("profile", &rule.profile),
        assignment("remoteip", &netsh_remote_scope(&rule.remote_scope)),
    ];
    if let Some(program_path) = program_path {
        args.push(assignment("program", program_path));
    }
    FirewallCommand {
        rule_name: rule.name.clone(),
        tool: "netsh".to_owned(),
        command: format_command("netsh", &args),
        args,
        purpose: Some(rule.purpose.clone()),
    }
}

fn create_delete_rule_command(rule: &FirewallRule) -> FirewallCommand {
    let args = vec![
        "advfirewall".to_owned(),
        "firewall".to_owned(),
        "delete".to_owned(),
        "rule".to_owned(),
        assignment("name", &rule.name),
    ];
    FirewallCommand {
        rule_name: rule.name.clone(),
        tool: "netsh".to_owned(),
        command: format_command("netsh", &args),
        args,
        purpose: None,
    }
}

fn direction_to_netsh(direction: &str) -> &str {
    match direction {
        "inbound" => "in",
        "outbound" => "out",
        value => value,
    }
}

fn create_firewall_warnings(
    rules: &[FirewallRule],
    program_path: Option<&str>,
) -> Vec<FirewallWarning> {
    let mut warnings = Vec::new();
    if rules.is_empty() {
        warnings.push(FirewallWarning {
            key: "no-rules".to_owned(),
            message: "This game profile has no port rules, so concrete firewall commands cannot be generated.".to_owned(),
        });
    }
    if !rules.is_empty() && program_path.is_none() {
        warnings.push(FirewallWarning {
            key: "port-only-scope".to_owned(),
            message: "Rules are scoped by port and virtual subnet only; no game executable path is attached yet.".to_owned(),
        });
    }
    warnings
}

fn assignment(key: &str, value: &str) -> String {
    format!("{key}={}", quote_netsh_value(value))
}

fn netsh_remote_scope(value: &str) -> String {
    if let Some((network, prefix)) = value.split_once('/') {
        if let (Ok(network), Ok(prefix)) = (network.parse::<Ipv4Addr>(), prefix.parse::<u8>()) {
            if prefix <= 32 {
                return format!("{network}/{}", prefix_to_subnet_mask(prefix));
            }
        }
    }
    value.to_owned()
}

fn prefix_to_subnet_mask(prefix: u8) -> Ipv4Addr {
    let mask = if prefix == 0 {
        0
    } else {
        u32::MAX << (32 - u32::from(prefix))
    };
    Ipv4Addr::from(mask)
}

fn quote_netsh_value(value: &str) -> String {
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
    fn windows_firewall_plan_renders_netsh_commands() {
        let rules = vec![FirewallRule {
            name: "Example Game UDP 7777".to_owned(),
            direction: "inbound".to_owned(),
            action: "allow".to_owned(),
            protocol: "udp".to_owned(),
            port: 7777,
            profile: "private".to_owned(),
            remote_scope: "10.77.12.0/24".to_owned(),
            purpose: "Allow test traffic.".to_owned(),
        }];
        let plan = create_windows_firewall_plan(
            &rules,
            "LocalAreaInterconnection",
            Some("C:\\Games\\Example Game\\game.exe".to_owned()),
        );

        assert_eq!(plan.platform, "windows");
        assert!(plan.dry_run);
        assert!(plan.requires_elevation);
        assert_eq!(plan.commands.len(), 1);
        assert!(plan.commands[0]
            .command
            .contains("netsh advfirewall firewall add rule"));
        assert!(plan.commands[0]
            .command
            .contains("name=\"Example Game UDP 7777\""));
        assert!(plan.commands[0]
            .command
            .contains("remoteip=10.77.12.0/255.255.255.0"));
        assert!(plan.commands[0]
            .command
            .contains("program=\"C:\\Games\\Example Game\\game.exe\""));
        assert_eq!(plan.rollback_commands.len(), 1);
        assert!(plan.warnings.is_empty());
    }

    #[test]
    fn windows_firewall_plan_warns_without_rules() {
        let plan = create_windows_firewall_plan(&[], "LocalAreaInterconnection", None);

        assert!(!plan.requires_elevation);
        assert!(plan.commands.is_empty());
        assert!(plan
            .warnings
            .iter()
            .any(|warning| warning.key == "no-rules"));
    }
}
