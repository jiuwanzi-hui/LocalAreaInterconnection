use crate::game_network_plan::FirewallRule;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FirewallRuleObservation {
    pub rule_name: Option<String>,
    pub direction: String,
    pub action: String,
    pub protocol: String,
    pub port: u16,
    pub profile: String,
    pub remote_scope: Option<String>,
    pub program: Option<String>,
    pub enabled: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FirewallDiagnosticsReport {
    pub status: String,
    pub summary: String,
    pub expected_rule_count: usize,
    pub observed_rule_count: usize,
    pub problem_count: usize,
    pub checks: Vec<FirewallRuleCheck>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FirewallRuleCheck {
    pub rule_name: String,
    pub protocol: String,
    pub port: u16,
    pub status: String,
    pub message: String,
    pub next_action: String,
}

pub fn evaluate_firewall_diagnostics(
    expected_rules: &[FirewallRule],
    observed_rules: &[FirewallRuleObservation],
    expected_program_path: Option<&str>,
) -> FirewallDiagnosticsReport {
    let checks = expected_rules
        .iter()
        .map(|expected| evaluate_rule(expected, observed_rules, expected_program_path))
        .collect::<Vec<_>>();
    let problem_count = checks
        .iter()
        .filter(|check| check.status != "present")
        .count();
    let status = if expected_rules.is_empty() {
        "unknown"
    } else if problem_count == 0 {
        "ok"
    } else {
        "needs-attention"
    }
    .to_owned();

    FirewallDiagnosticsReport {
        status: status.clone(),
        summary: match status.as_str() {
            "ok" => "Expected Windows Firewall rules are present.".to_owned(),
            "unknown" => "No expected Windows Firewall rules were provided.".to_owned(),
            _ => format!("Detected {problem_count} Windows Firewall rule problem(s)."),
        },
        expected_rule_count: expected_rules.len(),
        observed_rule_count: observed_rules.len(),
        problem_count,
        checks,
    }
}

pub fn observation_from_expected_rule(
    rule: &FirewallRule,
    program: Option<String>,
) -> FirewallRuleObservation {
    FirewallRuleObservation {
        rule_name: Some(rule.name.clone()),
        direction: rule.direction.clone(),
        action: rule.action.clone(),
        protocol: rule.protocol.clone(),
        port: rule.port,
        profile: rule.profile.clone(),
        remote_scope: Some(rule.remote_scope.clone()),
        program,
        enabled: true,
    }
}

fn evaluate_rule(
    expected: &FirewallRule,
    observed_rules: &[FirewallRuleObservation],
    expected_program_path: Option<&str>,
) -> FirewallRuleCheck {
    let Some(observed) = observed_rules
        .iter()
        .find(|observed| matches_expected_rule(expected, observed))
    else {
        return check(
            expected,
            "missing",
            "Expected Windows Firewall rule is missing.",
            "Add the inbound allow rule for this game port on the private profile.",
        );
    };

    if !observed.enabled {
        return check(
            expected,
            "disabled",
            "Windows Firewall rule exists but is disabled.",
            "Enable the rule or recreate it from the firewall plan.",
        );
    }
    if !observed.action.eq_ignore_ascii_case(&expected.action) {
        return check(
            expected,
            "wrong-action",
            "Windows Firewall rule action does not match the expected allow rule.",
            "Change the rule action to allow or recreate it from the firewall plan.",
        );
    }
    if !observed.direction.eq_ignore_ascii_case(&expected.direction) {
        return check(
            expected,
            "wrong-direction",
            "Windows Firewall rule direction does not match the expected inbound rule.",
            "Create an inbound rule for this game port.",
        );
    }
    if let Some(scope) = observed.remote_scope.as_deref() {
        if !scope.eq_ignore_ascii_case(&expected.remote_scope) {
            return check(
                expected,
                "wrong-scope",
                "Windows Firewall rule remote scope does not match the room virtual subnet.",
                "Restrict the rule remote scope to the room virtual subnet.",
            );
        }
    }
    if let Some(expected_program_path) = expected_program_path {
        if let Some(program) = observed.program.as_deref() {
            if !program.eq_ignore_ascii_case(expected_program_path) {
                return check(
                    expected,
                    "program-mismatch",
                    "Windows Firewall rule is bound to a different executable.",
                    "Bind the rule to the selected game executable or recreate it.",
                );
            }
        }
    }

    check(
        expected,
        "present",
        "Expected Windows Firewall rule is present.",
        "No firewall action is needed for this rule.",
    )
}

fn matches_expected_rule(expected: &FirewallRule, observed: &FirewallRuleObservation) -> bool {
    observed.protocol.eq_ignore_ascii_case(&expected.protocol)
        && observed.port == expected.port
        && observed.profile.eq_ignore_ascii_case(&expected.profile)
}

fn check(
    expected: &FirewallRule,
    status: &str,
    message: &str,
    next_action: &str,
) -> FirewallRuleCheck {
    FirewallRuleCheck {
        rule_name: expected.name.clone(),
        protocol: expected.protocol.clone(),
        port: expected.port,
        status: status.to_owned(),
        message: message.to_owned(),
        next_action: next_action.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game_network_plan::create_firewall_rules;
    use crate::game_profile::{CompatibilityLevel, DiscoveryMode, GameProfile};
    use crate::ip::Ipv4Subnet;

    fn expected_rules() -> Vec<FirewallRule> {
        let profile = GameProfile {
            game_name: "Example".to_owned(),
            steam_app_id: None,
            discovery: DiscoveryMode::ManualPorts,
            ports: vec![7777],
            join_method: "direct_ip".to_owned(),
            compatibility: CompatibilityLevel::C,
            notes: String::new(),
        };
        create_firewall_rules(&profile, "10.77.12.0/24".parse::<Ipv4Subnet>().unwrap())
    }

    #[test]
    fn firewall_diagnostics_passes_when_expected_rules_are_observed() {
        let rules = expected_rules();
        let observed = rules
            .iter()
            .map(|rule| observation_from_expected_rule(rule, Some("C:\\Game\\game.exe".to_owned())))
            .collect::<Vec<_>>();

        let report = evaluate_firewall_diagnostics(&rules, &observed, Some("C:\\Game\\game.exe"));

        assert_eq!(report.status, "ok");
        assert_eq!(report.problem_count, 0);
        assert!(report.checks.iter().all(|check| check.status == "present"));
    }

    #[test]
    fn firewall_diagnostics_reports_missing_rules() {
        let rules = expected_rules();
        let report = evaluate_firewall_diagnostics(&rules, &[], None);

        assert_eq!(report.status, "needs-attention");
        assert_eq!(report.problem_count, 2);
        assert!(report.checks.iter().all(|check| check.status == "missing"));
    }

    #[test]
    fn firewall_diagnostics_reports_program_mismatch() {
        let rules = expected_rules();
        let observed = rules
            .iter()
            .map(|rule| {
                observation_from_expected_rule(rule, Some("C:\\Other\\game.exe".to_owned()))
            })
            .collect::<Vec<_>>();

        let report = evaluate_firewall_diagnostics(&rules, &observed, Some("C:\\Game\\game.exe"));

        assert_eq!(report.status, "needs-attention");
        assert!(report
            .checks
            .iter()
            .any(|check| check.status == "program-mismatch"));
    }
}
