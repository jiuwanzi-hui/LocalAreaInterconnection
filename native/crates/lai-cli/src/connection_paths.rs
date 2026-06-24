use std::fs;
use std::path::Path;

pub fn load_nat_offer_argument(
    value: &str,
) -> Result<lai_core::NatTraversalOffer, Box<dyn std::error::Error>> {
    let text = if Path::new(value).exists() {
        let bytes = fs::read(value)?;
        let bytes = bytes
            .strip_prefix(&[0xef, 0xbb, 0xbf])
            .unwrap_or(bytes.as_slice());
        String::from_utf8(bytes.to_vec())?
    } else {
        value.to_owned()
    };
    Ok(serde_json::from_str(&text)?)
}

pub fn load_relay_fallback_for_export(
    local_offer: Option<&str>,
    remote_offer: Option<&str>,
    p2p_status: &str,
) -> (
    Option<lai_core::RelayFallbackPlan>,
    Option<lai_core::ConnectionPathReport>,
    Option<String>,
) {
    match (local_offer, remote_offer) {
        (None, None) => (None, None, None),
        (Some(_), None) | (None, Some(_)) => (
            None,
            None,
            Some(
                "Both --relay-local-offer and --relay-remote-offer are required for relay fallback export."
                    .to_owned(),
            ),
        ),
        (Some(local_offer), Some(remote_offer)) => {
            let local = match load_nat_offer_argument(local_offer) {
                Ok(offer) => offer,
                Err(err) => {
                    return (
                        None,
                        None,
                        Some(format!("failed to load relay local offer: {err}")),
                    )
                }
            };
            let remote = match load_nat_offer_argument(remote_offer) {
                Ok(offer) => offer,
                Err(err) => {
                    return (
                        None,
                        None,
                        Some(format!("failed to load relay remote offer: {err}")),
                    )
                }
            };
            let relay_fallback =
                lai_core::create_relay_fallback_plan(&local, &remote, p2p_status);
            let connection_path = lai_core::evaluate_connection_path(&local, &remote, p2p_status);
            (Some(relay_fallback), Some(connection_path), None)
        }
    }
}

pub fn connection_path_reports_from_bootstrap_outputs(
    nat_results: &[serde_json::Value],
    coordination_results: &[serde_json::Value],
    coordination_server_results: &[serde_json::Value],
) -> Result<Vec<serde_json::Value>, Box<dyn std::error::Error>> {
    let mut reports = Vec::new();
    for result in nat_results {
        if let Some(report) = connection_path_report_from_bootstrap_result(
            "nat-bootstrap-remote-peer",
            result
                .get("remoteOffer")
                .and_then(|offer| offer.get("peer_id"))
                .and_then(serde_json::Value::as_str),
            result,
        )? {
            reports.push(report);
        }
    }
    for wrapper in coordination_results
        .iter()
        .chain(coordination_server_results.iter())
    {
        let Some(result) = wrapper.get("result") else {
            continue;
        };
        let source = wrapper
            .get("source")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("coordination");
        if let Some(report) = connection_path_report_from_bootstrap_result(
            source,
            wrapper.get("peerId").and_then(serde_json::Value::as_str),
            result,
        )? {
            reports.push(report);
        }
    }
    Ok(reports)
}

pub fn runtime_relay_fallback_summaries(
    connection_path_reports: &[serde_json::Value],
) -> Vec<serde_json::Value> {
    connection_path_reports
        .iter()
        .filter_map(|entry| {
            let report = entry.get("report").or(Some(entry))?;
            let fallback = report.get("relay_fallback")?;
            let status = fallback
                .get("status")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("unknown");
            let selected_path = report
                .get("selected_path")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("unknown");
            let selected_relay_endpoints = fallback
                .get("selected_relay_endpoints")
                .and_then(serde_json::Value::as_array)
                .cloned()
                .unwrap_or_default();
            let recommended_actions = fallback
                .get("recommended_actions")
                .and_then(serde_json::Value::as_array)
                .cloned()
                .unwrap_or_default();
            let warnings = fallback
                .get("warnings")
                .and_then(serde_json::Value::as_array)
                .cloned()
                .unwrap_or_default();
            Some(serde_json::json!({
                "source": entry
                    .get("source")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("connection-path"),
                "peerId": connection_path_peer_id(entry).unwrap_or_else(|| {
                    report
                        .get("remote_peer_id")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or("unknown")
                        .to_owned()
                }),
                "bootstrapStatus": entry
                    .get("bootstrapStatus")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("unknown"),
                "status": status,
                "selectedPath": selected_path,
                "p2pStatus": fallback
                    .get("p2p_status")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("unknown"),
                "p2pCandidateCount": fallback
                    .get("p2p_candidate_count")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or_default(),
                "relayCandidateCount": fallback
                    .get("relay_candidate_count")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or_default(),
                "selectedRelayEndpoints": selected_relay_endpoints,
                "recommendedActions": recommended_actions,
                "warnings": warnings,
            }))
        })
        .collect()
}

pub fn connection_path_peer_id(entry: &serde_json::Value) -> Option<String> {
    entry
        .get("peerId")
        .and_then(serde_json::Value::as_str)
        .or_else(|| {
            entry
                .get("report")
                .and_then(|report| report.get("remote_peer_id"))
                .and_then(serde_json::Value::as_str)
        })
        .map(str::to_owned)
}

fn connection_path_report_from_bootstrap_result(
    source: &str,
    peer_id: Option<&str>,
    result: &serde_json::Value,
) -> Result<Option<serde_json::Value>, Box<dyn std::error::Error>> {
    let Some(local_offer_value) = result.get("localOffer").cloned() else {
        return Ok(None);
    };
    let Some(remote_offer_value) = result.get("remoteOffer").cloned() else {
        return Ok(None);
    };
    let local_offer: lai_core::NatTraversalOffer = serde_json::from_value(local_offer_value)?;
    let remote_offer: lai_core::NatTraversalOffer = serde_json::from_value(remote_offer_value)?;
    let bootstrap_status = result
        .get("status")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("unknown");
    let p2p_status = connection_path_status_from_bootstrap_status(bootstrap_status);
    let report = lai_core::evaluate_connection_path(&local_offer, &remote_offer, p2p_status);
    let selected_peer = result.get("selectedPeer");
    let stun_mapping = result.get("stunMapping").cloned();
    let upnp_mapping = result.get("upnpPortMapping").cloned();
    Ok(Some(serde_json::json!({
        "source": source,
        "peerId": peer_id.unwrap_or(remote_offer.peer_id.as_str()),
        "bootstrapStatus": bootstrap_status,
        "localEndpoint": result
            .get("localEndpoint")
            .and_then(serde_json::Value::as_str),
        "stunMapping": stun_mapping,
        "upnpPortMapping": upnp_mapping,
        "bootstrapLatencyMs": selected_peer
            .and_then(|peer| peer.get("latencyMs"))
            .and_then(serde_json::Value::as_u64),
        "selectedPeerEndpoint": selected_peer
            .and_then(|peer| peer.get("endpoint"))
            .and_then(serde_json::Value::as_str),
        "observedEndpoint": selected_peer
            .and_then(|peer| peer.get("observedEndpoint"))
            .and_then(serde_json::Value::as_str),
        "handshakeRole": selected_peer
            .and_then(|peer| peer.get("handshakeRole"))
            .and_then(serde_json::Value::as_str),
        "confirmedByAck": selected_peer
            .and_then(|peer| peer.get("confirmedByAck"))
            .and_then(serde_json::Value::as_bool),
        "report": report,
    })))
}

pub fn connection_path_status_from_bootstrap_status(status: &str) -> &'static str {
    match status {
        "ok" | "connected" | "success" | "succeeded" => "ok",
        "relay-ready" | "relay-selected" => "failed",
        "handshake-timeout" | "timeout" | "timed-out" | "no-response" => "timeout",
        "failed" | "blocked" | "unreachable" | "disconnected" => "failed",
        _ => "unknown",
    }
}
