use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct DiagnosticSnapshot {
    pub virtual_adapter: Option<String>,
    pub firewall: Option<String>,
    pub tunnel: Option<String>,
    pub p2p: Option<String>,
    pub broadcast: Option<String>,
    pub direct_ip: Option<String>,
    pub game_traffic: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DiagnosticProblem {
    pub key: String,
    pub message: String,
    pub next_action: String,
    pub value: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DiagnosticReport {
    pub status: String,
    pub summary: String,
    pub problems: Vec<DiagnosticProblem>,
}

pub fn evaluate_diagnostics(snapshot: DiagnosticSnapshot) -> DiagnosticReport {
    let mut problems = Vec::new();
    push_problem(
        &mut problems,
        "virtual_adapter",
        snapshot.virtual_adapter,
        "ok",
        "Virtual adapter is not ready",
        "Check driver installation, adapter state, and administrator permission",
    );
    push_problem(
        &mut problems,
        "firewall",
        snapshot.firewall,
        "allowed",
        "Windows Firewall may block the client or game",
        "Add inbound rules and allow private networks",
    );
    push_problem(
        &mut problems,
        "tunnel",
        snapshot.tunnel,
        "ok",
        "Tunnel connection is not healthy",
        "Renegotiate the tunnel or switch networks",
    );
    push_problem(
        &mut problems,
        "p2p",
        snapshot.p2p,
        "ok",
        "P2P connection failed",
        "Try port forwarding, network switching, or coordination fallback",
    );
    push_problem(
        &mut problems,
        "broadcast",
        snapshot.broadcast,
        "seen",
        "Broadcast forwarding was not observed",
        "Check UDP broadcast rules and game ports",
    );
    push_problem(
        &mut problems,
        "direct_ip",
        snapshot.direct_ip,
        "ok",
        "Direct IP connection failed",
        "Try joining with the host virtual IP",
    );
    push_problem(
        &mut problems,
        "game_traffic",
        snapshot.game_traffic,
        "seen",
        "No game traffic observed",
        "Check whether the game bound to the virtual adapter",
    );
    let status = if problems.is_empty() {
        "healthy"
    } else {
        "needs-attention"
    }
    .to_owned();
    DiagnosticReport {
        status,
        summary: if problems.is_empty() {
            "Connectivity indicators look healthy".to_owned()
        } else {
            format!("Detected {} problem(s)", problems.len())
        },
        problems,
    }
}

fn push_problem(
    problems: &mut Vec<DiagnosticProblem>,
    key: &'static str,
    value: Option<String>,
    healthy: &'static str,
    message: &'static str,
    next_action: &'static str,
) {
    if let Some(value) = value {
        if value != healthy && !(key == "firewall" && value == "ok") {
            problems.push(DiagnosticProblem {
                key: key.to_owned(),
                message: message.to_owned(),
                next_action: next_action.to_owned(),
                value,
            });
        }
    }
}
