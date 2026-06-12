use crate::command_execution::{
    create_command_execution_preview, CommandExecutionPreview, CommandExecutionStatus,
};
use crate::network_observation::AdapterObservation;
use crate::virtual_adapter_plan::VirtualAdapterPlan;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct VirtualAdapterEnsureReport {
    pub status: String,
    pub ready: bool,
    pub adapter_name: String,
    pub observation: Option<AdapterObservation>,
    pub checks: Vec<VirtualAdapterEnsureCheck>,
    pub execution_preview: CommandExecutionPreview,
    pub next_action: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct VirtualAdapterEnsureCheck {
    pub key: String,
    pub status: String,
    pub expected: String,
    pub observed: Option<String>,
    pub next_action: String,
}

pub fn create_windows_virtual_adapter_ensure_report(
    plan: VirtualAdapterPlan,
    observation: Option<AdapterObservation>,
    confirmed: bool,
    elevated: Option<bool>,
) -> VirtualAdapterEnsureReport {
    let checks = adapter_checks(&plan, observation.as_ref());
    let ready = checks.iter().all(|check| check.status == "ok");
    let mut execution_preview = create_command_execution_preview(
        &plan.commands,
        plan.requires_elevation,
        confirmed && !ready,
        elevated,
    );
    if ready {
        for command in &mut execution_preview.commands {
            command.status = CommandExecutionStatus::SkippedNeedsConfirmation;
            command.next_action = Some("No adapter change is needed.".to_owned());
        }
        execution_preview.next_action =
            "Adapter already matches the requested room configuration.".to_owned();
        execution_preview.can_execute_now = false;
    }
    let next_action = if ready {
        "Start the room runtime or run network diagnostics.".to_owned()
    } else {
        execution_preview.next_action.clone()
    };

    VirtualAdapterEnsureReport {
        status: if ready { "ready" } else { "needs-apply" }.to_owned(),
        ready,
        adapter_name: plan.adapter_name,
        observation,
        checks,
        execution_preview,
        next_action,
    }
}

fn adapter_checks(
    plan: &VirtualAdapterPlan,
    observation: Option<&AdapterObservation>,
) -> Vec<VirtualAdapterEnsureCheck> {
    let mut checks = Vec::new();
    let Some(observation) = observation else {
        return vec![VirtualAdapterEnsureCheck {
            key: "adapter-present".to_owned(),
            status: "missing".to_owned(),
            expected: plan.adapter_name.clone(),
            observed: None,
            next_action:
                "Install or enable the virtual adapter, then run adapter-apply as Administrator."
                    .to_owned(),
        }];
    };

    checks.push(VirtualAdapterEnsureCheck {
        key: "adapter-enabled".to_owned(),
        status: if observation.enabled {
            "ok"
        } else {
            "mismatch"
        }
        .to_owned(),
        expected: "enabled".to_owned(),
        observed: Some(
            if observation.enabled {
                "enabled"
            } else {
                "disabled"
            }
            .to_owned(),
        ),
        next_action: if observation.enabled {
            "No action needed.".to_owned()
        } else {
            "Enable the virtual adapter in Windows network settings.".to_owned()
        },
    });
    checks.push(VirtualAdapterEnsureCheck {
        key: "adapter-ip".to_owned(),
        status: if observation.assigned_ip == Some(plan.assigned_ip) {
            "ok"
        } else {
            "mismatch"
        }
        .to_owned(),
        expected: plan.assigned_ip.to_string(),
        observed: observation.assigned_ip.map(|ip| ip.to_string()),
        next_action: if observation.assigned_ip == Some(plan.assigned_ip) {
            "No action needed.".to_owned()
        } else {
            "Run adapter-apply as Administrator to assign the room virtual IP.".to_owned()
        },
    });
    checks.push(VirtualAdapterEnsureCheck {
        key: "adapter-subnet".to_owned(),
        status: if observation.virtual_subnet.as_ref().map(ToString::to_string)
            == Some(plan.virtual_subnet.clone())
        {
            "ok"
        } else {
            "mismatch"
        }
        .to_owned(),
        expected: plan.virtual_subnet.clone(),
        observed: observation.virtual_subnet.map(|subnet| subnet.to_string()),
        next_action: if observation.virtual_subnet.as_ref().map(ToString::to_string)
            == Some(plan.virtual_subnet.clone())
        {
            "No action needed.".to_owned()
        } else {
            "Run adapter-apply as Administrator to set the room subnet mask.".to_owned()
        },
    });
    checks.push(VirtualAdapterEnsureCheck {
        key: "adapter-mtu".to_owned(),
        status: if observation.mtu == Some(plan.mtu) {
            "ok"
        } else {
            "mismatch"
        }
        .to_owned(),
        expected: plan.mtu.to_string(),
        observed: observation.mtu.map(|mtu| mtu.to_string()),
        next_action: if observation.mtu == Some(plan.mtu) {
            "No action needed.".to_owned()
        } else {
            "Run adapter-apply as Administrator to set the virtual adapter MTU.".to_owned()
        },
    });
    checks.push(VirtualAdapterEnsureCheck {
        key: "adapter-metric".to_owned(),
        status: if observation.interface_metric == Some(plan.interface_metric) {
            "ok"
        } else {
            "mismatch"
        }
        .to_owned(),
        expected: plan.interface_metric.to_string(),
        observed: observation
            .interface_metric
            .map(|interface_metric| interface_metric.to_string()),
        next_action: if observation.interface_metric == Some(plan.interface_metric) {
            "No action needed.".to_owned()
        } else {
            "Run adapter-apply as Administrator to set the virtual adapter metric.".to_owned()
        },
    });
    checks
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ip::Ipv4Subnet;
    use crate::virtual_adapter_plan::create_windows_virtual_adapter_plan;
    use std::net::Ipv4Addr;

    #[test]
    fn adapter_ensure_reports_ready_when_observation_matches_plan() {
        let subnet: Ipv4Subnet = "10.77.12.0/24".parse().unwrap();
        let plan = create_windows_virtual_adapter_plan(
            "LocalAreaInterconnection",
            subnet,
            "10.77.12.2".parse().unwrap(),
            1420,
            5,
        );
        let observation = AdapterObservation {
            adapter_name: "LocalAreaInterconnection".to_owned(),
            enabled: true,
            expected_ip: Some("10.77.12.2".parse().unwrap()),
            assigned_ip: Some("10.77.12.2".parse().unwrap()),
            virtual_subnet: Some(subnet),
            mtu: Some(1420),
            interface_metric: Some(5),
        };

        let report =
            create_windows_virtual_adapter_ensure_report(plan, Some(observation), false, None);

        assert_eq!(report.status, "ready");
        assert!(report.ready);
        assert!(report.checks.iter().all(|check| check.status == "ok"));
    }

    #[test]
    fn adapter_ensure_reports_apply_needed_when_observation_is_missing() {
        let plan = create_windows_virtual_adapter_plan(
            "LocalAreaInterconnection",
            "10.77.12.0/24".parse().unwrap(),
            Ipv4Addr::new(10, 77, 12, 2),
            1420,
            5,
        );

        let report = create_windows_virtual_adapter_ensure_report(plan, None, false, Some(false));

        assert_eq!(report.status, "needs-apply");
        assert!(!report.ready);
        assert_eq!(report.checks[0].status, "missing");
        assert!(report.next_action.contains("--yes true"));
    }
}
