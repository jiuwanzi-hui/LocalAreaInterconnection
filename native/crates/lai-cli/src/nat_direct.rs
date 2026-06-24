use lai_core::{
    create_p2p_handshake_ack, create_p2p_handshake_confirm, create_p2p_handshake_hello,
    open_tunnel_payload, seal_tunnel_payload, P2pHandshakeAck, P2pHandshakeConfirm,
    P2pHandshakeHello, TunnelEnvelope,
};
use rand::RngCore;
use serde::Serialize;
use socket2::{Domain, Protocol, Socket, Type};
use std::collections::HashSet;
use std::io::{self, ErrorKind, Read, Write};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, TcpStream, ToSocketAddrs, UdpSocket};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpnpPortMappingReport {
    pub status: String,
    pub external_endpoint: Option<String>,
    pub gateway_location: Option<String>,
    pub control_url: Option<String>,
    pub detail: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StunMappingReport {
    pub status: String,
    pub mapping_behavior: String,
    pub observed_endpoints: Vec<String>,
    pub queries: Vec<StunMappingQuery>,
    pub detail: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StunMappingQuery {
    pub server: String,
    pub status: String,
    pub observed_endpoint: Option<String>,
    pub detail: String,
}

impl StunMappingReport {
    pub fn disabled() -> Self {
        Self {
            status: "disabled".to_owned(),
            mapping_behavior: "not-tested".to_owned(),
            observed_endpoints: Vec::new(),
            queries: Vec::new(),
            detail: "STUN discovery was not configured.".to_owned(),
        }
    }
}

impl UpnpPortMappingReport {
    pub fn disabled() -> Self {
        Self {
            status: "disabled".to_owned(),
            external_endpoint: None,
            gateway_location: None,
            control_url: None,
            detail: "UPnP port mapping was not requested.".to_owned(),
        }
    }
}

pub fn bind_udp_socket(bind: &str) -> Result<UdpSocket, Box<dyn std::error::Error>> {
    let Ok(addr) = bind.parse::<SocketAddr>() else {
        return UdpSocket::bind(bind).map_err(Into::into);
    };
    if addr.ip().is_unspecified() {
        if let Ok(socket) = bind_dual_stack_udp_socket(addr.port()) {
            return Ok(socket);
        }
    }
    UdpSocket::bind(addr).map_err(Into::into)
}

pub fn send_udp_to<A: ToSocketAddrs>(
    socket: &UdpSocket,
    bytes: &[u8],
    target: A,
) -> io::Result<usize> {
    let local_is_ipv6 = socket.local_addr()?.is_ipv6();
    let mut last_error = None;
    for target in target.to_socket_addrs()? {
        let mapped_target = if local_is_ipv6 {
            ipv4_mapped_socket_addr(target)
        } else {
            target
        };
        match socket.send_to(bytes, mapped_target) {
            Ok(sent) => return Ok(sent),
            Err(err) => last_error = Some(err),
        }
    }
    Err(last_error.unwrap_or_else(|| {
        io::Error::new(
            ErrorKind::InvalidInput,
            "UDP target resolved to no addresses",
        )
    }))
}

fn ipv4_mapped_socket_addr(target: SocketAddr) -> SocketAddr {
    match target {
        SocketAddr::V4(value) => {
            SocketAddr::new(IpAddr::V6(value.ip().to_ipv6_mapped()), value.port())
        }
        SocketAddr::V6(_) => target,
    }
}

fn bind_dual_stack_udp_socket(port: u16) -> Result<UdpSocket, Box<dyn std::error::Error>> {
    let socket = Socket::new(Domain::IPV6, Type::DGRAM, Some(Protocol::UDP))?;
    socket.set_only_v6(false)?;
    let address = SocketAddr::new(IpAddr::V6(Ipv6Addr::UNSPECIFIED), port);
    socket.bind(&address.into())?;
    Ok(socket.into())
}

#[derive(Clone, Debug)]
struct UpnpWanService {
    service_type: String,
    control_url: String,
}

#[derive(Clone, Debug)]
struct ParsedHttpUrl {
    host: String,
    port: u16,
    path: String,
}

pub fn apply_stun_mapping_candidates_to_offer(
    offer: &mut lai_core::NatTraversalOffer,
    socket: &UdpSocket,
    stun_server: Option<&str>,
    stun_timeout_ms: u64,
) -> StunMappingReport {
    let Some(stun_server) = stun_server else {
        return StunMappingReport::disabled();
    };
    let servers = stun_server_list(stun_server);
    if servers.is_empty() {
        return StunMappingReport::disabled();
    }

    let mut queries = Vec::new();
    let mut observed_endpoints = Vec::new();
    for server in servers {
        match query_observed_endpoint_once(socket, &server, stun_timeout_ms) {
            Ok(endpoint) => {
                let endpoint_text = endpoint.to_string();
                queries.push(StunMappingQuery {
                    server,
                    status: "ok".to_owned(),
                    observed_endpoint: Some(endpoint_text.clone()),
                    detail: "STUN query returned an observed UDP endpoint.".to_owned(),
                });
                observed_endpoints.push(endpoint_text);
                offer.candidates.push(lai_core::NatCandidate {
                    candidate_type: "srflx".to_owned(),
                    transport: "udp".to_owned(),
                    endpoint: endpoint.to_string(),
                    priority: 90,
                    source: "observed-endpoint".to_owned(),
                });
            }
            Err(err) => queries.push(StunMappingQuery {
                server,
                status: "failed".to_owned(),
                observed_endpoint: None,
                detail: err.to_string(),
            }),
        }
    }

    observed_endpoints.sort();
    observed_endpoints.dedup();
    deduplicate_and_sort_nat_candidates(&mut offer.candidates);

    let successful = queries.iter().filter(|query| query.status == "ok").count();
    let mapping_behavior = if successful == 0 {
        "unknown"
    } else if successful == 1 {
        "single-observation"
    } else if observed_endpoints.len() == 1 {
        "stable"
    } else {
        "endpoint-dependent"
    }
    .to_owned();
    let status = if successful == 0 { "unavailable" } else { "ok" }.to_owned();
    let detail = match mapping_behavior.as_str() {
        "stable" => "Multiple STUN servers observed the same UDP mapping.".to_owned(),
        "endpoint-dependent" => {
            "Different STUN servers observed different UDP mappings; direct P2P is less likely without port mapping or IPv6.".to_owned()
        }
        "single-observation" => {
            "Only one STUN observation is available; NAT mapping stability is unknown.".to_owned()
        }
        _ => "No STUN server returned an observed UDP endpoint.".to_owned(),
    };

    StunMappingReport {
        status,
        mapping_behavior,
        observed_endpoints,
        queries,
        detail,
    }
}

fn query_observed_endpoint_once(
    socket: &UdpSocket,
    stun_server: &str,
    stun_timeout_ms: u64,
) -> Result<SocketAddr, Box<dyn std::error::Error>> {
    let standard_first = stun_server_prefers_standard_stun(stun_server);
    if standard_first {
        if let Ok(observed) = query_standard_stun_server(socket, stun_server, stun_timeout_ms) {
            return Ok(observed);
        }
    }
    if let Ok(response) = query_stun_like_server(socket, stun_server, stun_timeout_ms) {
        if let Some(observed) = response
            .get("observedEndpoint")
            .and_then(serde_json::Value::as_str)
            .and_then(|value| value.parse::<SocketAddr>().ok())
        {
            return Ok(observed);
        }
    }
    if !standard_first {
        if let Ok(observed) = query_standard_stun_server(socket, stun_server, stun_timeout_ms) {
            return Ok(observed);
        }
    }
    Err(invalid_input("STUN response is missing observed endpoint".to_owned()).into())
}

pub fn run_stun_like_server(
    bind: &str,
    max_requests: u32,
    timeout_ms: u64,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let socket = bind_udp_socket(bind)?;
    socket.set_read_timeout(Some(Duration::from_millis(timeout_ms)))?;
    let local_addr = socket.local_addr()?;
    let mut handled_requests = 0u32;
    let mut requests = Vec::new();
    let mut buffer = [0u8; 2048];

    while max_requests == 0 || handled_requests < max_requests {
        match socket.recv_from(&mut buffer) {
            Ok((received, peer)) => {
                let request = serde_json::from_slice::<serde_json::Value>(&buffer[..received])
                    .unwrap_or_else(|_| serde_json::json!({ "type": "unknown" }));
                let response = serde_json::json!({
                    "schemaVersion": 1,
                    "type": "stun-like-response",
                    "status": "ok",
                    "observedEndpoint": peer.to_string(),
                    "serverEndpoint": local_addr.to_string(),
                    "receivedBytes": received,
                    "request": request,
                });
                let response_bytes = serde_json::to_vec(&response)?;
                send_udp_to(&socket, &response_bytes, peer)?;
                handled_requests = handled_requests.saturating_add(1);
                requests.push(serde_json::json!({
                    "peer": peer.to_string(),
                    "bytes": received,
                    "request": request,
                }));
            }
            Err(err)
                if matches!(
                    err.kind(),
                    ErrorKind::WouldBlock
                        | ErrorKind::TimedOut
                        | ErrorKind::Interrupted
                        | ErrorKind::ConnectionReset
                ) =>
            {
                break;
            }
            Err(err) => return Err(err.into()),
        }
    }

    Ok(serde_json::json!({
        "status": "ok",
        "bind": local_addr.to_string(),
        "handledRequests": handled_requests,
        "requests": requests,
    }))
}

pub fn query_stun_like_server(
    socket: &UdpSocket,
    server: &str,
    timeout_ms: u64,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let server = resolve_udp_server(server, socket.local_addr()?.is_ipv6())?;
    let previous_timeout = socket.read_timeout()?;
    socket.set_read_timeout(Some(Duration::from_millis(timeout_ms)))?;
    let request = serde_json::json!({
        "schemaVersion": 1,
        "type": "stun-like-query",
        "sentAtMs": current_epoch_ms(),
        "localEndpoint": socket.local_addr()?.to_string(),
    });
    let request_bytes = serde_json::to_vec(&request)?;
    let sent = send_udp_to(socket, &request_bytes, server)?;
    let mut buffer = [0u8; 2048];
    let result = match socket.recv_from(&mut buffer) {
        Ok((received, peer)) => {
            let mut response: serde_json::Value = serde_json::from_slice(&buffer[..received])?;
            response["status"] = serde_json::Value::String("ok".to_owned());
            response["queryLocalEndpoint"] =
                serde_json::Value::String(socket.local_addr()?.to_string());
            response["server"] = serde_json::Value::String(server.to_string());
            response["responsePeer"] = serde_json::Value::String(peer.to_string());
            response["bytesSent"] = serde_json::Value::from(sent as u64);
            response["bytesReceived"] = serde_json::Value::from(received as u64);
            Ok(response)
        }
        Err(err)
            if matches!(
                err.kind(),
                ErrorKind::WouldBlock
                    | ErrorKind::TimedOut
                    | ErrorKind::Interrupted
                    | ErrorKind::ConnectionReset
            ) =>
        {
            Ok(serde_json::json!({
                "status": "timeout",
                "server": server.to_string(),
                "queryLocalEndpoint": socket.local_addr()?.to_string(),
                "bytesSent": sent,
            }))
        }
        Err(err) => Err(err.into()),
    };
    socket.set_read_timeout(previous_timeout)?;
    result
}

pub fn enrich_offer_with_local_host_candidates(
    offer: &mut lai_core::NatTraversalOffer,
    socket: &UdpSocket,
) -> Result<(), Box<dyn std::error::Error>> {
    let local_endpoint = socket.local_addr()?;
    if local_endpoint.ip().is_unspecified() {
        offer.candidates.retain(|candidate| {
            !candidate.candidate_type.eq_ignore_ascii_case("host")
                || candidate
                    .endpoint
                    .parse::<SocketAddr>()
                    .map(|endpoint| !endpoint.ip().is_unspecified())
                    .unwrap_or(true)
        });
    }
    for endpoint in default_route_host_candidates(local_endpoint) {
        offer.candidates.push(lai_core::NatCandidate {
            candidate_type: "host".to_owned(),
            transport: "udp".to_owned(),
            endpoint: endpoint.to_string(),
            priority: 95,
            source: "default-route-local-ip".to_owned(),
        });
    }
    deduplicate_and_sort_nat_candidates(&mut offer.candidates);
    Ok(())
}

pub fn apply_upnp_port_mapping_to_offer(
    offer: &mut lai_core::NatTraversalOffer,
    socket: &UdpSocket,
    timeout_ms: u64,
    lease_seconds: u32,
    gateway_location: Option<&str>,
) -> UpnpPortMappingReport {
    match try_upnp_port_mapping(socket, timeout_ms, lease_seconds, gateway_location) {
        Ok(report) => {
            if let Some(endpoint) = report
                .external_endpoint
                .as_deref()
                .and_then(|value| value.parse::<SocketAddr>().ok())
            {
                offer.candidates.push(lai_core::NatCandidate {
                    candidate_type: "srflx".to_owned(),
                    transport: "udp".to_owned(),
                    endpoint: endpoint.to_string(),
                    priority: 92,
                    source: "upnp-port-mapping".to_owned(),
                });
                deduplicate_and_sort_nat_candidates(&mut offer.candidates);
            }
            report
        }
        Err(err) => UpnpPortMappingReport {
            status: "unavailable".to_owned(),
            external_endpoint: None,
            gateway_location: gateway_location.map(str::to_owned),
            control_url: None,
            detail: err.to_string(),
        },
    }
}

pub fn run_nat_hole_punch(
    room_id: &str,
    peer_id: &str,
    bind: &str,
    remote_offer: &lai_core::NatTraversalOffer,
    observed_endpoint: Option<SocketAddr>,
    stun_server: Option<&str>,
    stun_timeout_ms: u64,
    upnp_port_map: bool,
    upnp_timeout_ms: u64,
    upnp_lease_seconds: u32,
    upnp_gateway_location: Option<&str>,
    relay_endpoints: Vec<SocketAddr>,
    attempts: u16,
    interval_ms: u64,
    receive_timeout_ms: u64,
    message: &str,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let socket = bind_udp_socket(bind)?;
    if receive_timeout_ms > 0 {
        socket.set_read_timeout(Some(Duration::from_millis(receive_timeout_ms)))?;
    }
    let mut local_offer = lai_core::create_nat_traversal_offer(
        room_id,
        peer_id,
        random_nonce(),
        current_epoch_ms(),
        socket.local_addr()?,
        observed_endpoint,
        relay_endpoints.iter().map(SocketAddr::to_string).collect(),
    );
    enrich_offer_with_local_host_candidates(&mut local_offer, &socket)?;
    let stun_mapping = apply_stun_mapping_candidates_to_offer(
        &mut local_offer,
        &socket,
        stun_server,
        stun_timeout_ms,
    );
    let upnp_mapping = if upnp_port_map {
        apply_upnp_port_mapping_to_offer(
            &mut local_offer,
            &socket,
            upnp_timeout_ms,
            upnp_lease_seconds,
            upnp_gateway_location,
        )
    } else {
        UpnpPortMappingReport::disabled()
    };
    let plan = lai_core::create_nat_punch_plan(&local_offer, remote_offer, attempts, interval_ms);
    let mut sent_packets = Vec::new();
    let mut received_packets = Vec::new();
    let mut buffer = [0u8; 2048];

    if plan.status == "ready" {
        for attempt in 0..plan.attempt_count {
            let payload = serde_json::json!({
                "schemaVersion": 1,
                "type": "nat-punch",
                "roomId": room_id,
                "peerId": peer_id,
                "attempt": attempt,
                "message": message,
                "sentAtMs": current_epoch_ms(),
            })
            .to_string();
            for target in &plan.target_endpoints {
                let sent = send_udp_to(&socket, payload.as_bytes(), target)?;
                sent_packets.push(serde_json::json!({
                    "target": target,
                    "attempt": attempt,
                    "bytes": sent,
                }));
            }
            if receive_timeout_ms > 0 {
                drain_udp_socket_packet_records(&socket, &mut buffer, &mut received_packets)?;
            }
            if interval_ms > 0 && attempt + 1 < plan.attempt_count {
                std::thread::sleep(Duration::from_millis(interval_ms));
            }
        }
        if receive_timeout_ms > 0 {
            drain_udp_socket_packet_records(&socket, &mut buffer, &mut received_packets)?;
        }
    }

    let status = if plan.status != "ready" {
        plan.status.clone()
    } else if received_packets.is_empty() {
        "sent-no-response".to_owned()
    } else {
        "ok".to_owned()
    };

    Ok(serde_json::json!({
        "status": status,
        "localEndpoint": socket.local_addr()?.to_string(),
        "localOffer": local_offer,
        "remoteOffer": remote_offer,
        "stunMapping": stun_mapping,
        "upnpPortMapping": upnp_mapping,
        "plan": plan,
        "sentPackets": sent_packets,
        "receivedPackets": received_packets,
    }))
}

pub fn run_nat_p2p_bootstrap(
    room_id: &str,
    peer_id: &str,
    virtual_ip: Ipv4Addr,
    key: &str,
    bind: &str,
    remote_offer: &lai_core::NatTraversalOffer,
    observed_endpoint: Option<SocketAddr>,
    stun_server: Option<&str>,
    stun_timeout_ms: u64,
    upnp_port_map: bool,
    upnp_timeout_ms: u64,
    upnp_lease_seconds: u32,
    upnp_gateway_location: Option<&str>,
    relay_endpoints: Vec<SocketAddr>,
    punch_attempts: u16,
    punch_interval_ms: u64,
    handshake_timeout_ms: u64,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let socket = bind_udp_socket(bind)?;
    run_nat_p2p_bootstrap_on_socket(
        &socket,
        room_id,
        peer_id,
        virtual_ip,
        key,
        remote_offer,
        observed_endpoint,
        stun_server,
        stun_timeout_ms,
        upnp_port_map,
        upnp_timeout_ms,
        upnp_lease_seconds,
        upnp_gateway_location,
        relay_endpoints,
        punch_attempts,
        punch_interval_ms,
        handshake_timeout_ms,
    )
}

pub fn run_nat_p2p_bootstrap_on_socket(
    socket: &UdpSocket,
    room_id: &str,
    peer_id: &str,
    virtual_ip: Ipv4Addr,
    key: &str,
    remote_offer: &lai_core::NatTraversalOffer,
    observed_endpoint: Option<SocketAddr>,
    stun_server: Option<&str>,
    stun_timeout_ms: u64,
    upnp_port_map: bool,
    upnp_timeout_ms: u64,
    upnp_lease_seconds: u32,
    upnp_gateway_location: Option<&str>,
    relay_endpoints: Vec<SocketAddr>,
    punch_attempts: u16,
    punch_interval_ms: u64,
    handshake_timeout_ms: u64,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    socket.set_read_timeout(Some(Duration::from_millis(25)))?;
    let local_endpoint = socket.local_addr()?;
    let mut local_offer = lai_core::create_nat_traversal_offer(
        room_id,
        peer_id,
        random_nonce(),
        current_epoch_ms(),
        local_endpoint,
        observed_endpoint,
        relay_endpoints.iter().map(SocketAddr::to_string).collect(),
    );
    enrich_offer_with_local_host_candidates(&mut local_offer, &socket)?;
    let stun_mapping = apply_stun_mapping_candidates_to_offer(
        &mut local_offer,
        &socket,
        stun_server,
        stun_timeout_ms,
    );
    let upnp_mapping = if upnp_port_map {
        apply_upnp_port_mapping_to_offer(
            &mut local_offer,
            &socket,
            upnp_timeout_ms,
            upnp_lease_seconds,
            upnp_gateway_location,
        )
    } else {
        UpnpPortMappingReport::disabled()
    };
    let plan = lai_core::create_nat_punch_plan(
        &local_offer,
        remote_offer,
        punch_attempts,
        punch_interval_ms,
    );
    let mut punch_packets = Vec::new();
    let mut handshake_packets = Vec::new();
    let mut ignored_packets = Vec::new();
    let mut selected_peer = None;
    let mut answered_hello_nonce = None::<String>;
    let mut buffer = vec![0u8; 65_535];

    if plan.status == "ready" {
        let started_at_ms = current_epoch_ms();
        let hello = create_p2p_handshake_hello(
            room_id,
            peer_id,
            virtual_ip,
            local_endpoint.to_string(),
            random_nonce(),
            started_at_ms,
        );
        let hello_bytes = serde_json::to_vec(&hello)?;
        let envelope =
            seal_tunnel_payload(key, "p2p-handshake-hello", 1, started_at_ms, &hello_bytes)?;
        let envelope_bytes = serde_json::to_vec(&envelope)?;
        let deadline = Instant::now() + Duration::from_millis(handshake_timeout_ms);
        let punch_interval = Duration::from_millis(punch_interval_ms.max(25));
        let handshake_resend_interval = Duration::from_millis(100);
        let mut punch_attempt = 0u16;
        let mut next_punch_at = Instant::now();
        let mut next_handshake_resend_at = Instant::now();
        while handshake_timeout_ms > 0 && Instant::now() < deadline {
            let now = Instant::now();
            if punch_attempt < plan.attempt_count && now >= next_punch_at {
                let payload = serde_json::json!({
                    "schemaVersion": 1,
                    "type": "nat-punch",
                    "roomId": room_id,
                    "peerId": peer_id,
                    "attempt": punch_attempt,
                    "sentAtMs": current_epoch_ms(),
                })
                .to_string();
                for target in &plan.target_endpoints {
                    let sent = send_udp_to(socket, payload.as_bytes(), target)?;
                    punch_packets.push(serde_json::json!({
                        "target": target,
                        "attempt": punch_attempt,
                        "bytes": sent,
                    }));
                }
                punch_attempt = punch_attempt.saturating_add(1);
                next_punch_at = now + punch_interval;
            }
            if Instant::now() >= next_handshake_resend_at {
                for target in &plan.target_endpoints {
                    let sent = send_udp_to(socket, &envelope_bytes, target)?;
                    handshake_packets.push(serde_json::json!({
                        "target": target,
                        "packetKind": "p2p-handshake-hello",
                        "bytes": sent,
                    }));
                }
                next_handshake_resend_at = Instant::now() + handshake_resend_interval;
            }
            match socket.recv_from(&mut buffer) {
                Ok((received, peer)) => {
                    let envelope: TunnelEnvelope = match serde_json::from_slice(&buffer[..received])
                    {
                        Ok(value) => value,
                        Err(_) => {
                            ignored_packets.push(serde_json::json!({
                                "peer": peer.to_string(),
                                "bytes": received,
                                "reason": "not-tunnel-envelope",
                            }));
                            continue;
                        }
                    };
                    let payload = match open_tunnel_payload(key, &envelope) {
                        Ok(value) => value,
                        Err(_) => {
                            ignored_packets.push(serde_json::json!({
                                "peer": peer.to_string(),
                                "bytes": received,
                                "reason": "decrypt-failed",
                            }));
                            continue;
                        }
                    };
                    if payload.metadata.packet_kind == "p2p-handshake-hello" {
                        let remote_hello: P2pHandshakeHello =
                            serde_json::from_slice(&payload.plaintext)?;
                        let remote_hello_matches = remote_hello.room_id == room_id
                            && remote_hello.peer_id == remote_offer.peer_id;
                        if !remote_hello_matches {
                            ignored_packets.push(serde_json::json!({
                                "peer": peer.to_string(),
                                "bytes": received,
                                "reason": "invalid-handshake-hello",
                                "packetKind": payload.metadata.packet_kind,
                            }));
                            continue;
                        }
                        let ack_sent_at_ms = current_epoch_ms();
                        let ack = create_p2p_handshake_ack(
                            &remote_hello,
                            peer_id,
                            peer.to_string(),
                            ack_sent_at_ms,
                        );
                        let ack_bytes = serde_json::to_vec(&ack)?;
                        let ack_envelope = seal_tunnel_payload(
                            key,
                            "p2p-handshake-ack",
                            handshake_packets.len() as u64 + 1,
                            ack_sent_at_ms,
                            &ack_bytes,
                        )?;
                        let ack_wire = serde_json::to_vec(&ack_envelope)?;
                        let sent = send_udp_to(socket, &ack_wire, peer)?;
                        handshake_packets.push(serde_json::json!({
                            "target": peer.to_string(),
                            "packetKind": "p2p-handshake-ack",
                            "bytes": sent,
                        }));
                        ignored_packets.push(serde_json::json!({
                            "peer": peer.to_string(),
                            "bytes": received,
                            "reason": "answered-remote-handshake-hello",
                            "packetKind": payload.metadata.packet_kind,
                        }));
                        answered_hello_nonce = Some(remote_hello.nonce.clone());
                        selected_peer = Some(serde_json::json!({
                            "endpoint": peer.to_string(),
                            "responderPeerId": remote_hello.peer_id,
                            "observedEndpoint": peer.to_string(),
                            "nonceMatched": true,
                            "accepted": true,
                            "handshakeRole": "answered-remote-hello",
                            "confirmedByAck": false,
                            "latencyMs": current_epoch_ms().saturating_sub(started_at_ms),
                        }));
                        continue;
                    }
                    if payload.metadata.packet_kind == "p2p-handshake-confirm" {
                        let confirm: P2pHandshakeConfirm =
                            serde_json::from_slice(&payload.plaintext)?;
                        let confirmed = confirm.room_id == room_id
                            && confirm.confirmer_peer_id == remote_offer.peer_id
                            && confirm.responder_peer_id == peer_id
                            && answered_hello_nonce.as_deref() == Some(confirm.nonce.as_str());
                        ignored_packets.push(serde_json::json!({
                            "peer": peer.to_string(),
                            "bytes": received,
                            "reason": if confirmed { "confirmed-remote-handshake" } else { "invalid-handshake-confirm" },
                            "packetKind": payload.metadata.packet_kind,
                        }));
                        if confirmed {
                            selected_peer = Some(serde_json::json!({
                                "endpoint": peer.to_string(),
                                "responderPeerId": confirm.confirmer_peer_id,
                                "observedEndpoint": peer.to_string(),
                                "nonceMatched": true,
                                "accepted": true,
                                "handshakeRole": "answered-remote-hello",
                                "confirmedByAck": true,
                                "latencyMs": current_epoch_ms().saturating_sub(started_at_ms),
                            }));
                            break;
                        }
                        continue;
                    }
                    if payload.metadata.packet_kind != "p2p-handshake-ack" {
                        ignored_packets.push(serde_json::json!({
                            "peer": peer.to_string(),
                            "bytes": received,
                            "reason": "unexpected-packet-kind",
                            "packetKind": payload.metadata.packet_kind,
                        }));
                        continue;
                    }
                    let ack: P2pHandshakeAck = serde_json::from_slice(&payload.plaintext)?;
                    let nonce_matched = ack.nonce == hello.nonce;
                    let ack_room_matched = ack.room_id == room_id;
                    let ack_responder_matched = ack.responder_peer_id == remote_offer.peer_id;
                    let ack_confirmed =
                        ack.accepted && nonce_matched && ack_room_matched && ack_responder_matched;
                    if ack_confirmed {
                        let confirm_sent_at_ms = current_epoch_ms();
                        let confirm = create_p2p_handshake_confirm(
                            &ack.room_id,
                            peer_id,
                            &ack.responder_peer_id,
                            &ack.nonce,
                            confirm_sent_at_ms,
                        );
                        let confirm_bytes = serde_json::to_vec(&confirm)?;
                        let confirm_envelope = seal_tunnel_payload(
                            key,
                            "p2p-handshake-confirm",
                            handshake_packets.len() as u64 + 1,
                            confirm_sent_at_ms,
                            &confirm_bytes,
                        )?;
                        let confirm_wire = serde_json::to_vec(&confirm_envelope)?;
                        let sent = send_udp_to(socket, &confirm_wire, peer)?;
                        handshake_packets.push(serde_json::json!({
                            "target": peer.to_string(),
                            "packetKind": "p2p-handshake-confirm",
                            "bytes": sent,
                        }));
                    }
                    selected_peer = Some(serde_json::json!({
                        "endpoint": peer.to_string(),
                        "responderPeerId": ack.responder_peer_id,
                        "observedEndpoint": ack.observed_endpoint,
                        "nonceMatched": nonce_matched,
                        "accepted": ack.accepted,
                        "roomMatched": ack_room_matched,
                        "responderMatched": ack_responder_matched,
                        "handshakeRole": "received-ack",
                        "confirmedByAck": ack_confirmed,
                        "latencyMs": current_epoch_ms().saturating_sub(started_at_ms),
                    }));
                    if ack_confirmed {
                        break;
                    }
                }
                Err(err)
                    if matches!(
                        err.kind(),
                        ErrorKind::WouldBlock
                            | ErrorKind::TimedOut
                            | ErrorKind::Interrupted
                            | ErrorKind::ConnectionReset
                    ) => {}
                Err(err) => return Err(err.into()),
            }
        }
    }

    let status = if plan.status != "ready" {
        plan.status.clone()
    } else if selected_peer
        .as_ref()
        .and_then(|peer| peer["accepted"].as_bool())
        .unwrap_or(false)
        && selected_peer
            .as_ref()
            .and_then(|peer| peer["nonceMatched"].as_bool())
            .unwrap_or(false)
        && selected_peer
            .as_ref()
            .and_then(|peer| peer["confirmedByAck"].as_bool())
            .unwrap_or(false)
    {
        "ok".to_owned()
    } else {
        "handshake-timeout".to_owned()
    };

    Ok(serde_json::json!({
        "status": status,
        "localEndpoint": local_endpoint.to_string(),
        "localOffer": local_offer,
        "remoteOffer": remote_offer,
        "stunMapping": stun_mapping,
        "upnpPortMapping": upnp_mapping,
        "plan": plan,
        "punchPackets": punch_packets,
        "handshakePackets": handshake_packets,
        "ignoredPackets": ignored_packets,
        "selectedPeer": selected_peer,
    }))
}

pub fn run_nat_hole_punch_loopback_test(
    room_id: &str,
    peer_a: &str,
    peer_b: &str,
    attempts: u16,
    interval_ms: u64,
    message: &str,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let socket_a = UdpSocket::bind("127.0.0.1:0")?;
    let socket_b = UdpSocket::bind("127.0.0.1:0")?;
    socket_a.set_read_timeout(Some(Duration::from_millis(50)))?;
    socket_b.set_read_timeout(Some(Duration::from_millis(50)))?;
    let offer_a = lai_core::create_nat_traversal_offer(
        room_id,
        peer_a,
        random_nonce(),
        current_epoch_ms(),
        socket_a.local_addr()?,
        Some(socket_a.local_addr()?),
        Vec::new(),
    );
    let offer_b = lai_core::create_nat_traversal_offer(
        room_id,
        peer_b,
        random_nonce(),
        current_epoch_ms(),
        socket_b.local_addr()?,
        Some(socket_b.local_addr()?),
        Vec::new(),
    );
    let plan_a = lai_core::create_nat_punch_plan(&offer_a, &offer_b, attempts, interval_ms);
    let plan_b = lai_core::create_nat_punch_plan(&offer_b, &offer_a, attempts, interval_ms);
    let mut sent_a = 0u16;
    let mut sent_b = 0u16;
    let mut received_by_a = 0u16;
    let mut received_by_b = 0u16;
    let mut buffer = [0u8; 2048];

    for attempt in 0..attempts.max(1) {
        let payload_a = format!("{}:{}:{attempt}:{message}", room_id, peer_a);
        for target in &plan_a.target_endpoints {
            send_udp_to(&socket_a, payload_a.as_bytes(), target)?;
            sent_a += 1;
        }
        let payload_b = format!("{}:{}:{attempt}:{message}", room_id, peer_b);
        for target in &plan_b.target_endpoints {
            send_udp_to(&socket_b, payload_b.as_bytes(), target)?;
            sent_b += 1;
        }
        drain_udp_socket(&socket_a, &mut buffer, &mut received_by_a)?;
        drain_udp_socket(&socket_b, &mut buffer, &mut received_by_b)?;
        if interval_ms > 0 && attempt + 1 < attempts.max(1) {
            std::thread::sleep(Duration::from_millis(interval_ms));
        }
    }
    drain_udp_socket(&socket_a, &mut buffer, &mut received_by_a)?;
    drain_udp_socket(&socket_b, &mut buffer, &mut received_by_b)?;

    Ok(serde_json::json!({
        "status": if received_by_a > 0 && received_by_b > 0 { "ok" } else { "timeout" },
        "offerA": offer_a,
        "offerB": offer_b,
        "planA": plan_a,
        "planB": plan_b,
        "sentByA": sent_a,
        "sentByB": sent_b,
        "receivedByA": received_by_a,
        "receivedByB": received_by_b,
    }))
}

fn try_upnp_port_mapping(
    socket: &UdpSocket,
    timeout_ms: u64,
    lease_seconds: u32,
    gateway_location: Option<&str>,
) -> Result<UpnpPortMappingReport, Box<dyn std::error::Error>> {
    let local_endpoint = socket.local_addr()?;
    let internal_client = local_ipv4_for_mapping(local_endpoint)?;
    let location = match gateway_location {
        Some(location) => location.to_owned(),
        None => discover_upnp_gateway(timeout_ms)?,
    };
    let description = http_get_text(&location, timeout_ms)?;
    let service = parse_upnp_wan_service(&description, &location)?;
    add_upnp_port_mapping(
        &service,
        local_endpoint.port(),
        internal_client,
        lease_seconds,
        timeout_ms,
    )?;
    let external_ip = get_upnp_external_ip(&service, timeout_ms)?;
    let external_endpoint = SocketAddr::new(IpAddr::V4(external_ip), local_endpoint.port());
    Ok(UpnpPortMappingReport {
        status: "mapped".to_owned(),
        external_endpoint: Some(external_endpoint.to_string()),
        gateway_location: Some(location),
        control_url: Some(service.control_url),
        detail: "UPnP UDP port mapping was added temporarily.".to_owned(),
    })
}

fn discover_upnp_gateway(timeout_ms: u64) -> Result<String, Box<dyn std::error::Error>> {
    let socket = UdpSocket::bind("0.0.0.0:0")?;
    socket.set_read_timeout(Some(Duration::from_millis(timeout_ms.max(1))))?;
    let request = concat!(
        "M-SEARCH * HTTP/1.1\r\n",
        "HOST: 239.255.255.250:1900\r\n",
        "MAN: \"ssdp:discover\"\r\n",
        "MX: 1\r\n",
        "ST: urn:schemas-upnp-org:device:InternetGatewayDevice:1\r\n",
        "\r\n"
    );
    socket.send_to(request.as_bytes(), "239.255.255.250:1900")?;
    let deadline = Instant::now() + Duration::from_millis(timeout_ms.max(1));
    let mut buffer = [0u8; 4096];
    while Instant::now() < deadline {
        match socket.recv_from(&mut buffer) {
            Ok((received, _)) => {
                let response = String::from_utf8_lossy(&buffer[..received]);
                if let Some(location) = http_header_value(&response, "location") {
                    return Ok(location);
                }
            }
            Err(err)
                if matches!(
                    err.kind(),
                    ErrorKind::WouldBlock
                        | ErrorKind::TimedOut
                        | ErrorKind::Interrupted
                        | ErrorKind::ConnectionReset
                ) =>
            {
                break;
            }
            Err(err) => return Err(err.into()),
        }
    }
    Err(invalid_input("UPnP gateway discovery timed out".to_owned()).into())
}

fn add_upnp_port_mapping(
    service: &UpnpWanService,
    port: u16,
    internal_client: Ipv4Addr,
    lease_seconds: u32,
    timeout_ms: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    let body = format!(
        concat!(
            r#"<?xml version="1.0"?>"#,
            r#"<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/" "#,
            r#"s:encodingStyle="http://schemas.xmlsoap.org/soap/encoding/">"#,
            r#"<s:Body><u:AddPortMapping xmlns:u="{service_type}">"#,
            r#"<NewRemoteHost></NewRemoteHost>"#,
            r#"<NewExternalPort>{port}</NewExternalPort>"#,
            r#"<NewProtocol>UDP</NewProtocol>"#,
            r#"<NewInternalPort>{port}</NewInternalPort>"#,
            r#"<NewInternalClient>{internal_client}</NewInternalClient>"#,
            r#"<NewEnabled>1</NewEnabled>"#,
            r#"<NewPortMappingDescription>LocalAreaInterconnection</NewPortMappingDescription>"#,
            r#"<NewLeaseDuration>{lease_seconds}</NewLeaseDuration>"#,
            r#"</u:AddPortMapping></s:Body></s:Envelope>"#
        ),
        service_type = service.service_type,
        port = port,
        internal_client = internal_client,
        lease_seconds = lease_seconds,
    );
    let response = soap_request(
        &service.control_url,
        &service.service_type,
        "AddPortMapping",
        &body,
        timeout_ms,
    )?;
    if http_status_code(&response).is_some_and(|code| (200..300).contains(&code)) {
        Ok(())
    } else {
        Err(invalid_input("UPnP AddPortMapping was rejected".to_owned()).into())
    }
}

fn get_upnp_external_ip(
    service: &UpnpWanService,
    timeout_ms: u64,
) -> Result<Ipv4Addr, Box<dyn std::error::Error>> {
    let body = format!(
        concat!(
            r#"<?xml version="1.0"?>"#,
            r#"<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/" "#,
            r#"s:encodingStyle="http://schemas.xmlsoap.org/soap/encoding/">"#,
            r#"<s:Body><u:GetExternalIPAddress xmlns:u="{service_type}">"#,
            r#"</u:GetExternalIPAddress></s:Body></s:Envelope>"#
        ),
        service_type = service.service_type,
    );
    let response = soap_request(
        &service.control_url,
        &service.service_type,
        "GetExternalIPAddress",
        &body,
        timeout_ms,
    )?;
    if !http_status_code(&response).is_some_and(|code| (200..300).contains(&code)) {
        return Err(invalid_input("UPnP GetExternalIPAddress was rejected".to_owned()).into());
    }
    let body = http_body(&response);
    xml_tag_value(body, "NewExternalIPAddress")
        .ok_or_else(|| invalid_input("UPnP response did not include external IP".to_owned()))?
        .parse::<Ipv4Addr>()
        .map_err(|err| invalid_input(format!("invalid UPnP external IP: {err}")))
}

fn parse_upnp_wan_service(
    description: &str,
    description_url: &str,
) -> Result<UpnpWanService, Box<dyn std::error::Error>> {
    let mut search = 0usize;
    while let Some(start_relative) = description[search..].find("<service>") {
        let start = search + start_relative;
        let Some(end_relative) = description[start..].find("</service>") else {
            break;
        };
        let end = start + end_relative + "</service>".len();
        let block = &description[start..end];
        let service_type = xml_tag_value(block, "serviceType").unwrap_or_default();
        if service_type.contains("WANIPConnection") || service_type.contains("WANPPPConnection") {
            let control = xml_tag_value(block, "controlURL").ok_or_else(|| {
                invalid_input("UPnP WAN service is missing controlURL".to_owned())
            })?;
            return Ok(UpnpWanService {
                service_type,
                control_url: resolve_url(description_url, &control)?,
            });
        }
        search = end;
    }
    Err(
        invalid_input("UPnP description did not include a WAN connection service".to_owned())
            .into(),
    )
}

fn local_ipv4_for_mapping(bound: SocketAddr) -> Result<Ipv4Addr, Box<dyn std::error::Error>> {
    match bound.ip() {
        IpAddr::V4(ip) if !ip.is_unspecified() && !ip.is_loopback() => Ok(ip),
        _ => default_route_host_candidates(SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0))
            .into_iter()
            .find_map(|endpoint| match endpoint.ip() {
                IpAddr::V4(ip) if !ip.is_unspecified() && !ip.is_loopback() => Some(ip),
                _ => None,
            })
            .ok_or_else(|| invalid_input("could not determine local IPv4 for UPnP".to_owned())),
    }
}

fn http_get_text(url: &str, timeout_ms: u64) -> Result<String, Box<dyn std::error::Error>> {
    let parsed = parse_http_url(url)?;
    let request = format!(
        "GET {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
        parsed.path,
        http_host_header(&parsed)
    );
    http_request(&parsed, &request, timeout_ms)
}

fn soap_request(
    url: &str,
    service_type: &str,
    action: &str,
    body: &str,
    timeout_ms: u64,
) -> Result<String, Box<dyn std::error::Error>> {
    let parsed = parse_http_url(url)?;
    let request = format!(
        concat!(
            "POST {} HTTP/1.1\r\n",
            "Host: {}\r\n",
            "Content-Type: text/xml; charset=\"utf-8\"\r\n",
            "SOAPAction: \"{}#{}\"\r\n",
            "Content-Length: {}\r\n",
            "Connection: close\r\n",
            "\r\n",
            "{}"
        ),
        parsed.path,
        http_host_header(&parsed),
        service_type,
        action,
        body.as_bytes().len(),
        body,
    );
    http_request(&parsed, &request, timeout_ms)
}

fn http_request(
    parsed: &ParsedHttpUrl,
    request: &str,
    timeout_ms: u64,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut stream = TcpStream::connect((parsed.host.as_str(), parsed.port))?;
    let timeout = Some(Duration::from_millis(timeout_ms.max(1)));
    stream.set_read_timeout(timeout)?;
    stream.set_write_timeout(timeout)?;
    stream.write_all(request.as_bytes())?;
    let mut response = String::new();
    stream.read_to_string(&mut response)?;
    Ok(response)
}

fn parse_http_url(url: &str) -> Result<ParsedHttpUrl, Box<dyn std::error::Error>> {
    let Some(rest) = url.strip_prefix("http://") else {
        return Err(invalid_input(format!(
            "unsupported URL `{url}`; expected http://"
        )));
    };
    let (authority, path) = match rest.find('/') {
        Some(index) => (&rest[..index], &rest[index..]),
        None => (rest, "/"),
    };
    let (host, port) = if authority.starts_with('[') {
        let end = authority
            .find(']')
            .ok_or_else(|| invalid_input(format!("invalid IPv6 URL authority `{authority}`")))?;
        let host = authority[1..end].to_owned();
        let port = authority[end + 1..]
            .strip_prefix(':')
            .map(str::parse::<u16>)
            .transpose()?
            .unwrap_or(80);
        (host, port)
    } else {
        let (host, port) = if let Some((host, port)) = authority.rsplit_once(':') {
            (host.to_owned(), port.parse::<u16>()?)
        } else {
            (authority.to_owned(), 80)
        };
        (host, port)
    };
    Ok(ParsedHttpUrl {
        host,
        port,
        path: path.to_owned(),
    })
}

fn resolve_url(base: &str, value: &str) -> Result<String, Box<dyn std::error::Error>> {
    if value.starts_with("http://") {
        return Ok(value.to_owned());
    }
    let parsed = parse_http_url(base)?;
    if value.starts_with('/') {
        return Ok(format!("http://{}{}", http_host_header(&parsed), value));
    }
    let base_dir = parsed
        .path
        .rsplit_once('/')
        .map(|(dir, _)| format!("{dir}/"))
        .unwrap_or_else(|| "/".to_owned());
    Ok(format!(
        "http://{}{}{}",
        http_host_header(&parsed),
        base_dir,
        value
    ))
}

fn http_host_header(parsed: &ParsedHttpUrl) -> String {
    let host = if parsed.host.contains(':') {
        format!("[{}]", parsed.host)
    } else {
        parsed.host.clone()
    };
    if parsed.port == 80 {
        host
    } else {
        format!("{host}:{}", parsed.port)
    }
}

fn http_header_value(response: &str, header: &str) -> Option<String> {
    response.lines().find_map(|line| {
        let (name, value) = line.split_once(':')?;
        name.trim()
            .eq_ignore_ascii_case(header)
            .then(|| value.trim().to_owned())
    })
}

fn http_status_code(response: &str) -> Option<u16> {
    response
        .lines()
        .next()?
        .split_whitespace()
        .nth(1)?
        .parse::<u16>()
        .ok()
}

fn http_body(response: &str) -> &str {
    response
        .split_once("\r\n\r\n")
        .map(|(_, body)| body)
        .or_else(|| response.split_once("\n\n").map(|(_, body)| body))
        .unwrap_or("")
}

fn xml_tag_value(text: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let start = text.find(&open)? + open.len();
    let end = text[start..].find(&close)? + start;
    Some(text[start..end].trim().to_owned())
}

fn resolve_udp_server(
    server: &str,
    prefer_ipv6: bool,
) -> Result<SocketAddr, Box<dyn std::error::Error>> {
    let server = normalize_stun_server(server);
    if let Ok(addr) = server.parse::<SocketAddr>() {
        return Ok(addr);
    }
    let addrs = server.to_socket_addrs()?.collect::<Vec<_>>();
    addrs
        .iter()
        .copied()
        .find(|addr| addr.is_ipv6() == prefer_ipv6)
        .or_else(|| addrs.first().copied())
        .ok_or_else(|| invalid_input(format!("could not resolve UDP server `{server}`")).into())
}

fn normalize_stun_server(server: &str) -> &str {
    server
        .trim()
        .strip_prefix("stun://")
        .or_else(|| server.trim().strip_prefix("stun:"))
        .unwrap_or_else(|| server.trim())
}

fn stun_server_list(servers: &str) -> Vec<String> {
    servers
        .split(|ch| ch == ',' || ch == ';')
        .map(str::trim)
        .filter(|server| !server.is_empty())
        .map(str::to_owned)
        .collect()
}

fn stun_server_prefers_standard_stun(server: &str) -> bool {
    let server = normalize_stun_server(server);
    if server.chars().any(|ch| ch.is_ascii_alphabetic()) {
        return true;
    }
    server.starts_with('[')
}

fn query_standard_stun_server(
    socket: &UdpSocket,
    server: &str,
    timeout_ms: u64,
) -> Result<SocketAddr, Box<dyn std::error::Error>> {
    let prefer_ipv6 = socket.local_addr()?.is_ipv6();
    let server = resolve_udp_server(server, prefer_ipv6)?;
    let previous_timeout = socket.read_timeout()?;
    socket.set_read_timeout(Some(Duration::from_millis(timeout_ms)))?;

    let mut transaction_id = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut transaction_id);
    let mut request = Vec::with_capacity(20);
    request.extend_from_slice(&0x0001u16.to_be_bytes());
    request.extend_from_slice(&0u16.to_be_bytes());
    request.extend_from_slice(&0x2112A442u32.to_be_bytes());
    request.extend_from_slice(&transaction_id);
    send_udp_to(socket, &request, server)?;

    let mut buffer = [0u8; 1500];
    let result = match socket.recv_from(&mut buffer) {
        Ok((received, _)) => {
            parse_standard_stun_observed_endpoint(&buffer[..received], &transaction_id)
        }
        Err(err) => Err(err.into()),
    };
    socket.set_read_timeout(previous_timeout)?;
    result
}

fn parse_standard_stun_observed_endpoint(
    packet: &[u8],
    transaction_id: &[u8; 12],
) -> Result<SocketAddr, Box<dyn std::error::Error>> {
    if packet.len() < 20 {
        return Err(invalid_input("short STUN response".to_owned()).into());
    }
    let message_type = u16::from_be_bytes([packet[0], packet[1]]);
    if message_type != 0x0101 {
        return Err(invalid_input("not a STUN binding success response".to_owned()).into());
    }
    let message_len = u16::from_be_bytes([packet[2], packet[3]]) as usize;
    let magic_cookie = u32::from_be_bytes([packet[4], packet[5], packet[6], packet[7]]);
    if magic_cookie != 0x2112A442 || &packet[8..20] != transaction_id {
        return Err(invalid_input("STUN response transaction mismatch".to_owned()).into());
    }
    let end = 20usize.saturating_add(message_len).min(packet.len());
    let mut offset = 20usize;
    while offset + 4 <= end {
        let attr_type = u16::from_be_bytes([packet[offset], packet[offset + 1]]);
        let attr_len = u16::from_be_bytes([packet[offset + 2], packet[offset + 3]]) as usize;
        let value_start = offset + 4;
        let value_end = value_start.saturating_add(attr_len);
        if value_end > end {
            break;
        }
        let value = &packet[value_start..value_end];
        if attr_type == 0x0020 {
            return parse_stun_xor_mapped_address(value, magic_cookie, transaction_id);
        }
        if attr_type == 0x0001 {
            return parse_stun_mapped_address(value);
        }
        offset = value_end + ((4 - (attr_len % 4)) % 4);
    }
    Err(invalid_input("STUN response did not include mapped address".to_owned()).into())
}

fn parse_stun_xor_mapped_address(
    value: &[u8],
    magic_cookie: u32,
    transaction_id: &[u8; 12],
) -> Result<SocketAddr, Box<dyn std::error::Error>> {
    if value.len() < 4 {
        return Err(invalid_input("unsupported STUN XOR-MAPPED-ADDRESS".to_owned()).into());
    }
    let xport = u16::from_be_bytes([value[2], value[3]]);
    let port = xport ^ ((magic_cookie >> 16) as u16);
    match value[1] {
        0x01 if value.len() >= 8 => {
            let cookie_bytes = magic_cookie.to_be_bytes();
            let addr = Ipv4Addr::new(
                value[4] ^ cookie_bytes[0],
                value[5] ^ cookie_bytes[1],
                value[6] ^ cookie_bytes[2],
                value[7] ^ cookie_bytes[3],
            );
            Ok(SocketAddr::from((addr, port)))
        }
        0x02 if value.len() >= 20 => {
            let mut mask = [0u8; 16];
            mask[..4].copy_from_slice(&magic_cookie.to_be_bytes());
            mask[4..].copy_from_slice(transaction_id);
            let mut addr = [0u8; 16];
            for index in 0..16 {
                addr[index] = value[4 + index] ^ mask[index];
            }
            Ok(SocketAddr::from((Ipv6Addr::from(addr), port)))
        }
        _ => Err(invalid_input("unsupported STUN XOR-MAPPED-ADDRESS".to_owned()).into()),
    }
}

fn parse_stun_mapped_address(value: &[u8]) -> Result<SocketAddr, Box<dyn std::error::Error>> {
    if value.len() < 4 {
        return Err(invalid_input("unsupported STUN MAPPED-ADDRESS".to_owned()).into());
    }
    let port = u16::from_be_bytes([value[2], value[3]]);
    match value[1] {
        0x01 if value.len() >= 8 => {
            let addr = Ipv4Addr::new(value[4], value[5], value[6], value[7]);
            Ok(SocketAddr::from((addr, port)))
        }
        0x02 if value.len() >= 20 => {
            let mut addr = [0u8; 16];
            addr.copy_from_slice(&value[4..20]);
            Ok(SocketAddr::from((Ipv6Addr::from(addr), port)))
        }
        _ => Err(invalid_input("unsupported STUN MAPPED-ADDRESS".to_owned()).into()),
    }
}

fn default_route_host_candidates(bound: SocketAddr) -> Vec<SocketAddr> {
    let mut endpoints = Vec::new();
    let port = bound.port();
    match bound.ip() {
        IpAddr::V4(addr) => {
            if addr.is_unspecified() {
                push_default_route_candidate(&mut endpoints, "8.8.8.8:80", port);
                push_default_route_candidate(&mut endpoints, "1.1.1.1:80", port);
            }
        }
        IpAddr::V6(addr) => {
            if addr.is_unspecified() {
                push_default_route_candidate(&mut endpoints, "8.8.8.8:80", port);
                push_default_route_candidate(&mut endpoints, "1.1.1.1:80", port);
                push_default_route_candidate(&mut endpoints, "[2001:4860:4860::8888]:80", port);
                push_default_route_candidate(&mut endpoints, "[2606:4700:4700::1111]:80", port);
            }
        }
    }
    endpoints.sort();
    endpoints.dedup();
    endpoints
}

fn push_default_route_candidate(endpoints: &mut Vec<SocketAddr>, probe: &str, port: u16) {
    let Ok(probe_endpoint) = probe.parse::<SocketAddr>() else {
        return;
    };
    let bind = if probe_endpoint.is_ipv4() {
        "0.0.0.0:0"
    } else {
        "[::]:0"
    };
    let Ok(socket) = UdpSocket::bind(bind) else {
        return;
    };
    if socket.connect(probe_endpoint).is_err() {
        return;
    }
    let Ok(local) = socket.local_addr() else {
        return;
    };
    if !is_usable_host_candidate_ip(local.ip())
        || endpoints
            .iter()
            .any(|endpoint| endpoint.ip() == local.ip() && endpoint.port() == port)
    {
        return;
    }
    endpoints.push(SocketAddr::new(local.ip(), port));
}

fn is_usable_host_candidate_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(value) => {
            let octets = value.octets();
            !value.is_unspecified()
                && !value.is_loopback()
                && !value.is_multicast()
                && octets[0] != 0
                && octets[0] != 127
                && !(octets[0] == 169 && octets[1] == 254)
                && !(octets[0] == 198 && (octets[1] == 18 || octets[1] == 19))
        }
        IpAddr::V6(value) => {
            !value.is_unspecified()
                && !value.is_loopback()
                && !value.is_multicast()
                && !value.is_unicast_link_local()
        }
    }
}

fn deduplicate_and_sort_nat_candidates(candidates: &mut Vec<lai_core::NatCandidate>) {
    let mut seen = HashSet::new();
    candidates.retain(|candidate| {
        seen.insert((
            candidate.transport.to_ascii_lowercase(),
            candidate.endpoint.clone(),
            candidate.candidate_type.to_ascii_lowercase(),
        ))
    });
    candidates.sort_by(|left, right| {
        right
            .priority
            .cmp(&left.priority)
            .then_with(|| left.candidate_type.cmp(&right.candidate_type))
            .then_with(|| left.endpoint.cmp(&right.endpoint))
    });
}

fn drain_udp_socket(
    socket: &UdpSocket,
    buffer: &mut [u8],
    received_count: &mut u16,
) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        match socket.recv_from(buffer) {
            Ok((_, _)) => *received_count = received_count.saturating_add(1),
            Err(err)
                if matches!(
                    err.kind(),
                    ErrorKind::WouldBlock
                        | ErrorKind::TimedOut
                        | ErrorKind::Interrupted
                        | ErrorKind::ConnectionReset
                ) =>
            {
                break;
            }
            Err(err) => return Err(err.into()),
        }
    }
    Ok(())
}

fn drain_udp_socket_packet_records(
    socket: &UdpSocket,
    buffer: &mut [u8],
    received_packets: &mut Vec<serde_json::Value>,
) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        match socket.recv_from(buffer) {
            Ok((bytes, peer)) => {
                received_packets.push(serde_json::json!({
                    "peer": peer.to_string(),
                    "bytes": bytes,
                    "text": String::from_utf8_lossy(&buffer[..bytes]).to_string(),
                }));
            }
            Err(err)
                if matches!(
                    err.kind(),
                    ErrorKind::WouldBlock
                        | ErrorKind::TimedOut
                        | ErrorKind::Interrupted
                        | ErrorKind::ConnectionReset
                ) =>
            {
                break;
            }
            Err(err) => return Err(err.into()),
        }
    }
    Ok(())
}

fn current_epoch_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

fn random_nonce() -> String {
    let mut bytes = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut bytes);
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn invalid_input(message: String) -> Box<dyn std::error::Error> {
    Box::new(std::io::Error::new(ErrorKind::InvalidInput, message))
}

#[cfg(test)]
mod tests {
    use super::{bind_udp_socket, send_udp_to};
    use std::net::UdpSocket;
    use std::time::Duration;

    #[test]
    fn dual_stack_bind_keeps_ipv4_udp_reachable() {
        let receiver = UdpSocket::bind("127.0.0.1:0").unwrap();
        receiver
            .set_read_timeout(Some(Duration::from_millis(500)))
            .unwrap();
        let sender = bind_udp_socket("0.0.0.0:0").unwrap();
        send_udp_to(&sender, b"ping", receiver.local_addr().unwrap()).unwrap();

        let mut buffer = [0u8; 16];
        let (received, _) = receiver.recv_from(&mut buffer).unwrap();
        assert_eq!(&buffer[..received], b"ping");
    }
}
