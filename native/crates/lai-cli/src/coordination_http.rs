use lai_core::Ipv4Subnet;
use std::io::{ErrorKind, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::time::Duration;

use crate::{current_epoch_ms, invalid_input, read_text_file_with_retry, write_json_file};

pub(crate) fn run_coordination_http_server(
    bind: &str,
    store_path: &str,
    max_requests: u32,
    request_timeout_ms: u64,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let listener = TcpListener::bind(bind)?;
    let bound_addr = listener.local_addr()?;
    let mut store = load_coordination_store_or_default(store_path)?;
    let mut handled_requests = 0u32;

    while max_requests == 0 || handled_requests < max_requests {
        let (mut stream, remote_addr) = listener.accept()?;
        stream.set_read_timeout(Some(Duration::from_millis(request_timeout_ms)))?;
        stream.set_write_timeout(Some(Duration::from_millis(request_timeout_ms)))?;
        let request_result = read_http_request(&mut stream);
        let response = match request_result {
            Ok(request) => handle_coordination_http_request(request, &mut store, store_path),
            Err(err) => Ok((
                400,
                serde_json::json!({
                    "status": "error",
                    "error": err.to_string(),
                }),
            )),
        };
        let (status_code, body) = match response {
            Ok(response) => response,
            Err(err) => (
                500,
                serde_json::json!({
                    "status": "error",
                    "error": err.to_string(),
                }),
            ),
        };
        write_http_json_response(&mut stream, status_code, &body)?;
        handled_requests = handled_requests.saturating_add(1);
        let _ = remote_addr;
    }

    Ok(serde_json::json!({
        "status": "ok",
        "bind": bound_addr.to_string(),
        "store": store_path,
        "handledRequests": handled_requests,
    }))
}

fn handle_coordination_http_request(
    request: HttpRequest,
    store: &mut lai_core::CoordinationStore,
    store_path: &str,
) -> Result<(u16, serde_json::Value), Box<dyn std::error::Error>> {
    let path_segments = request
        .path
        .trim_matches('/')
        .split('/')
        .filter(|segment| !segment.is_empty())
        .map(percent_decode)
        .collect::<Result<Vec<_>, _>>()?;
    let path_segments = path_segments.iter().map(String::as_str).collect::<Vec<_>>();

    if request.method == "GET" && request.path == "/health" {
        return Ok((
            200,
            serde_json::json!({
                "status": "ok",
                "schemaVersion": store.schema_version,
            }),
        ));
    }

    match (request.method.as_str(), path_segments.as_slice()) {
        ("POST", ["v1", "offers"]) => {
            let body: serde_json::Value = serde_json::from_slice(&request.body)?;
            let offer_value = body
                .get("offer")
                .ok_or_else(|| invalid_input("missing JSON field `offer`".to_owned()))?
                .clone();
            let offer: lai_core::NatTraversalOffer = serde_json::from_value(offer_value)?;
            let ttl_ms = body
                .get("ttlMs")
                .or_else(|| body.get("ttl_ms"))
                .and_then(serde_json::Value::as_u64)
                .map(u128::from)
                .unwrap_or(30_000);
            let update =
                lai_core::publish_coordination_offer(store, offer, current_epoch_ms(), ttl_ms);
            write_json_file(store_path, store)?;
            Ok((200, serde_json::to_value(update)?))
        }
        ("GET", ["v1", "rooms", room_id, "offers"]) => {
            let peer_id = query_value(&request.query, "peer_id")
                .or_else(|| query_value(&request.query, "peerId"))
                .ok_or_else(|| invalid_input("missing query parameter `peer_id`".to_owned()))?;
            let result = lai_core::fetch_coordination_offers(
                store,
                room_id.to_owned(),
                peer_id,
                current_epoch_ms(),
            );
            write_json_file(store_path, store)?;
            Ok((200, serde_json::to_value(result)?))
        }
        ("GET", ["v1", "rooms", room_id, "view"]) => {
            let peer_id = query_value(&request.query, "peer_id")
                .or_else(|| query_value(&request.query, "peerId"))
                .ok_or_else(|| invalid_input("missing query parameter `peer_id`".to_owned()))?;
            let subnet = query_value(&request.query, "subnet")
                .ok_or_else(|| invalid_input("missing query parameter `subnet`".to_owned()))?
                .parse::<Ipv4Subnet>()?;
            let view = lai_core::coordination_room_view(
                store,
                room_id.to_owned(),
                peer_id,
                subnet,
                current_epoch_ms(),
            );
            Ok((200, serde_json::to_value(view)?))
        }
        ("POST", ["v1", "rooms", room_id, "peers", peer_id, "heartbeat"]) => {
            let body = if request.body.is_empty() {
                serde_json::json!({})
            } else {
                serde_json::from_slice::<serde_json::Value>(&request.body)?
            };
            let ttl_ms = body
                .get("ttlMs")
                .or_else(|| body.get("ttl_ms"))
                .and_then(serde_json::Value::as_u64)
                .map(u128::from)
                .unwrap_or(30_000);
            let update = lai_core::heartbeat_coordination_peer(
                store,
                room_id.to_owned(),
                peer_id.to_owned(),
                current_epoch_ms(),
                ttl_ms,
            );
            write_json_file(store_path, store)?;
            Ok((200, serde_json::to_value(update)?))
        }
        ("POST", ["v1", "rooms", room_id, "peers", peer_id, "leave"]) => {
            let report = lai_core::leave_coordination_room(
                store,
                room_id.to_owned(),
                peer_id.to_owned(),
                current_epoch_ms(),
            );
            write_json_file(store_path, store)?;
            Ok((200, serde_json::to_value(report)?))
        }
        ("POST", ["v1", "rooms", room_id, "peers", peer_id, "kick"]) => {
            let body = if request.body.is_empty() {
                serde_json::json!({})
            } else {
                serde_json::from_slice::<serde_json::Value>(&request.body)?
            };
            let kicked_by = body
                .get("kickedBy")
                .or_else(|| body.get("kicked_by"))
                .and_then(serde_json::Value::as_str)
                .unwrap_or("unknown");
            let report = lai_core::kick_coordination_peer(
                store,
                room_id.to_owned(),
                peer_id.to_owned(),
                kicked_by.to_owned(),
                current_epoch_ms(),
            );
            write_json_file(store_path, store)?;
            Ok((200, serde_json::to_value(report)?))
        }
        ("POST", ["v1", "rooms", room_id, "close"]) => {
            let body = if request.body.is_empty() {
                serde_json::json!({})
            } else {
                serde_json::from_slice::<serde_json::Value>(&request.body)?
            };
            let closed_by = body
                .get("closedBy")
                .or_else(|| body.get("closed_by"))
                .and_then(serde_json::Value::as_str);
            let report = if let Some(closed_by) = closed_by {
                lai_core::close_coordination_room_by_peer(
                    store,
                    room_id.to_owned(),
                    closed_by.to_owned(),
                )
            } else {
                lai_core::close_coordination_room(store, room_id.to_owned())
            };
            write_json_file(store_path, store)?;
            Ok((200, serde_json::to_value(report)?))
        }
        ("POST", ["v1", "prune"]) => {
            let report = lai_core::prune_expired_coordination_peers(store, current_epoch_ms());
            write_json_file(store_path, store)?;
            Ok((200, serde_json::to_value(report)?))
        }
        _ => Ok((
            404,
            serde_json::json!({
                "status": "error",
                "error": "not found",
                "method": request.method,
                "path": request.path,
            }),
        )),
    }
}

struct HttpRequest {
    method: String,
    path: String,
    query: String,
    body: Vec<u8>,
}

fn read_http_request(stream: &mut TcpStream) -> Result<HttpRequest, Box<dyn std::error::Error>> {
    let mut bytes = Vec::new();
    let mut buffer = [0u8; 1024];
    let (header_end, content_length) = loop {
        let read = stream.read(&mut buffer)?;
        if read == 0 {
            return Err(invalid_input(
                "connection closed before HTTP headers".to_owned(),
            ));
        }
        bytes.extend_from_slice(&buffer[..read]);
        if bytes.len() > 1024 * 1024 {
            return Err(invalid_input("HTTP request too large".to_owned()));
        }
        if let Some(header_end) = find_header_end(&bytes) {
            let headers = String::from_utf8_lossy(&bytes[..header_end]).to_string();
            let content_length = parse_content_length(&headers)?;
            break (header_end, content_length);
        }
    };
    let body_start = header_end + 4;
    while bytes.len() < body_start.saturating_add(content_length) {
        let read = stream.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        bytes.extend_from_slice(&buffer[..read]);
    }
    if bytes.len() < body_start.saturating_add(content_length) {
        return Err(invalid_input(
            "HTTP body shorter than Content-Length".to_owned(),
        ));
    }

    let headers = String::from_utf8_lossy(&bytes[..header_end]).to_string();
    let request_line = headers
        .lines()
        .next()
        .ok_or_else(|| invalid_input("missing HTTP request line".to_owned()))?;
    let mut request_parts = request_line.split_whitespace();
    let method = request_parts
        .next()
        .ok_or_else(|| invalid_input("missing HTTP method".to_owned()))?
        .to_owned();
    let target = request_parts
        .next()
        .ok_or_else(|| invalid_input("missing HTTP target".to_owned()))?;
    let (path, query) = split_path_query(target);
    let body = bytes[body_start..body_start + content_length].to_vec();

    Ok(HttpRequest {
        method,
        path,
        query,
        body,
    })
}

fn write_http_json_response(
    stream: &mut TcpStream,
    status_code: u16,
    body: &serde_json::Value,
) -> Result<(), Box<dyn std::error::Error>> {
    let body_text = serde_json::to_string(body)?;
    let status_text = match status_code {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        _ => "Internal Server Error",
    };
    let response = format!(
        "HTTP/1.1 {status_code} {status_text}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body_text.as_bytes().len(),
        body_text
    );
    stream.write_all(response.as_bytes())?;
    stream.flush()?;
    Ok(())
}

pub(crate) fn coordination_http_publish_offer(
    server: &str,
    offer: &lai_core::NatTraversalOffer,
    ttl_ms: u128,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    http_post_json(
        &format!("{}/v1/offers", trim_trailing_slash(server)),
        &serde_json::json!({
            "offer": offer,
            "ttlMs": ttl_ms,
        }),
    )
}

pub(crate) fn coordination_http_fetch_offers(
    server: &str,
    room_id: &str,
    peer_id: &str,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    http_get_json(&format!(
        "{}/v1/rooms/{}/offers?peer_id={}",
        trim_trailing_slash(server),
        percent_encode(room_id),
        percent_encode(peer_id)
    ))
}

pub(crate) fn coordination_http_room_view(
    server: &str,
    room_id: &str,
    peer_id: &str,
    subnet: Ipv4Subnet,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    http_get_json(&format!(
        "{}/v1/rooms/{}/view?peer_id={}&subnet={}",
        trim_trailing_slash(server),
        percent_encode(room_id),
        percent_encode(peer_id),
        percent_encode(&subnet.to_string())
    ))
}

pub(crate) fn coordination_http_heartbeat(
    server: &str,
    room_id: &str,
    peer_id: &str,
    ttl_ms: u128,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    http_post_json(
        &format!(
            "{}/v1/rooms/{}/peers/{}/heartbeat",
            trim_trailing_slash(server),
            percent_encode(room_id),
            percent_encode(peer_id)
        ),
        &serde_json::json!({ "ttlMs": ttl_ms }),
    )
}

pub(crate) fn coordination_http_leave(
    server: &str,
    room_id: &str,
    peer_id: &str,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    http_post_json(
        &format!(
            "{}/v1/rooms/{}/peers/{}/leave",
            trim_trailing_slash(server),
            percent_encode(room_id),
            percent_encode(peer_id)
        ),
        &serde_json::json!({}),
    )
}

pub(crate) fn coordination_http_kick(
    server: &str,
    room_id: &str,
    peer_id: &str,
    kicked_by: &str,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    http_post_json(
        &format!(
            "{}/v1/rooms/{}/peers/{}/kick",
            trim_trailing_slash(server),
            percent_encode(room_id),
            percent_encode(peer_id)
        ),
        &serde_json::json!({ "kickedBy": kicked_by }),
    )
}

pub(crate) fn coordination_http_close(
    server: &str,
    room_id: &str,
    closed_by: Option<&str>,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let body = closed_by
        .map(|closed_by| serde_json::json!({ "closedBy": closed_by }))
        .unwrap_or_else(|| serde_json::json!({}));
    http_post_json(
        &format!(
            "{}/v1/rooms/{}/close",
            trim_trailing_slash(server),
            percent_encode(room_id)
        ),
        &body,
    )
}

pub(crate) fn coordination_http_prune(
    server: &str,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    http_post_json(
        &format!("{}/v1/prune", trim_trailing_slash(server)),
        &serde_json::json!({}),
    )
}

fn http_get_json(url: &str) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    send_http_json_request("GET", url, None)
}

fn http_post_json(
    url: &str,
    body: &serde_json::Value,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    send_http_json_request("POST", url, Some(&serde_json::to_string(body)?))
}

fn send_http_json_request(
    method: &str,
    url: &str,
    body: Option<&str>,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let parsed = parse_http_url(url)?;
    let mut stream = TcpStream::connect((&parsed.host[..], parsed.port))?;
    stream.set_read_timeout(Some(Duration::from_secs(30)))?;
    stream.set_write_timeout(Some(Duration::from_secs(30)))?;
    let body = body.unwrap_or("");
    let request = format!(
        "{method} {} HTTP/1.1\r\nHost: {}\r\nAccept: application/json\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        parsed.path_and_query,
        parsed.host_header,
        body.as_bytes().len(),
        body
    );
    stream.write_all(request.as_bytes())?;
    stream.flush()?;
    let mut response = Vec::new();
    stream.read_to_end(&mut response)?;
    parse_http_json_response(&response)
}

struct ParsedHttpUrl {
    host: String,
    port: u16,
    host_header: String,
    path_and_query: String,
}

fn parse_http_url(url: &str) -> Result<ParsedHttpUrl, Box<dyn std::error::Error>> {
    let without_scheme = url
        .strip_prefix("http://")
        .ok_or_else(|| invalid_input("only http:// coordination URLs are supported".to_owned()))?;
    let (authority, path_and_query) = match without_scheme.split_once('/') {
        Some((authority, path)) => (authority, format!("/{path}")),
        None => (without_scheme, "/".to_owned()),
    };
    if authority.is_empty() {
        return Err(invalid_input("missing HTTP host".to_owned()));
    }
    let (host, port) = match authority.rsplit_once(':') {
        Some((host, port)) => (host.to_owned(), port.parse::<u16>()?),
        None => (authority.to_owned(), 80),
    };
    if host.is_empty() {
        return Err(invalid_input("missing HTTP host".to_owned()));
    }
    Ok(ParsedHttpUrl {
        host,
        port,
        host_header: authority.to_owned(),
        path_and_query,
    })
}

fn parse_http_json_response(
    response: &[u8],
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let header_end = find_header_end(response)
        .ok_or_else(|| invalid_input("HTTP response missing header terminator".to_owned()))?;
    let headers = String::from_utf8_lossy(&response[..header_end]).to_string();
    let status_line = headers
        .lines()
        .next()
        .ok_or_else(|| invalid_input("HTTP response missing status line".to_owned()))?;
    let status_code = status_line
        .split_whitespace()
        .nth(1)
        .ok_or_else(|| invalid_input("HTTP response missing status code".to_owned()))?
        .parse::<u16>()?;
    let body = &response[header_end + 4..];
    let value = if body.is_empty() {
        serde_json::json!({})
    } else {
        serde_json::from_slice::<serde_json::Value>(body)?
    };
    if !(200..300).contains(&status_code) {
        return Err(invalid_input(format!(
            "HTTP request failed with status {status_code}: {value}"
        )));
    }
    Ok(value)
}

fn find_header_end(bytes: &[u8]) -> Option<usize> {
    bytes.windows(4).position(|window| window == b"\r\n\r\n")
}

fn parse_content_length(headers: &str) -> Result<usize, Box<dyn std::error::Error>> {
    for line in headers.lines().skip(1) {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        if name.trim().eq_ignore_ascii_case("content-length") {
            return Ok(value.trim().parse::<usize>()?);
        }
    }
    Ok(0)
}

fn split_path_query(target: &str) -> (String, String) {
    match target.split_once('?') {
        Some((path, query)) => (path.to_owned(), query.to_owned()),
        None => (target.to_owned(), String::new()),
    }
}

fn query_value(query: &str, name: &str) -> Option<String> {
    query.split('&').find_map(|part| {
        let (key, value) = part.split_once('=')?;
        if key == name {
            percent_decode(value).ok()
        } else {
            None
        }
    })
}

fn percent_encode(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.as_bytes() {
        match *byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(*byte as char);
            }
            other => encoded.push_str(&format!("%{other:02X}")),
        }
    }
    encoded
}

fn percent_decode(value: &str) -> Result<String, Box<dyn std::error::Error>> {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0usize;
    while index < bytes.len() {
        match bytes[index] {
            b'%' if index + 2 < bytes.len() => {
                let hex = std::str::from_utf8(&bytes[index + 1..index + 3])?;
                decoded.push(u8::from_str_radix(hex, 16)?);
                index += 3;
            }
            b'+' => {
                decoded.push(b' ');
                index += 1;
            }
            byte => {
                decoded.push(byte);
                index += 1;
            }
        }
    }
    Ok(String::from_utf8(decoded)?)
}

fn trim_trailing_slash(value: &str) -> &str {
    value.trim_end_matches('/')
}

pub(crate) fn load_coordination_store_or_default(
    path: &str,
) -> Result<lai_core::CoordinationStore, Box<dyn std::error::Error>> {
    match read_text_file_with_retry(path, 12, Duration::from_millis(25)) {
        Ok(text) => Ok(serde_json::from_str(&text)?),
        Err(err) if err.kind() == ErrorKind::NotFound => Ok(lai_core::create_coordination_store()),
        Err(err) => Err(err.into()),
    }
}
