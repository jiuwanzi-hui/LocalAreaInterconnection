#!/usr/bin/env python3
import base64
import json
import os
import socket
import socketserver
import threading
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from urllib.parse import parse_qs, unquote, urlparse


HTTP_BIND = ("0.0.0.0", int(os.environ.get("LAI_HTTP_PORT", "39110")))
UDP_BIND = ("0.0.0.0", int(os.environ.get("LAI_UDP_PORT", "39091")))
OFFER_TTL_SECONDS = 60
HTTP_PACKET_TTL_MS = int(os.environ.get("LAI_HTTP_PACKET_TTL_MS", "5000"))
HTTP_QUEUE_LIMIT = int(os.environ.get("LAI_HTTP_QUEUE_LIMIT", "256"))
HTTP_POLL_LIMIT = int(os.environ.get("LAI_HTTP_POLL_LIMIT", "64"))
UDP_PEER_TTL_SECONDS = float(os.environ.get("LAI_UDP_PEER_TTL_SECONDS", "15"))
UDP_SOCKET_BUFFER_BYTES = int(os.environ.get("LAI_UDP_SOCKET_BUFFER_BYTES", str(4 * 1024 * 1024)))
UDP_BINARY_MAGIC = b"LAIR1"
UDP_BINARY_REGISTER = 1
UDP_BINARY_FORWARD = 2

lock = threading.Lock()
offers = {}
udp_peers = {}
http_peers = {}
tcp_peers = {}
udp_stats = {
    "receivedPackets": 0,
    "forwardedPackets": 0,
    "droppedPackets": 0,
    "stunQueries": 0,
    "binaryPackets": 0,
    "jsonPackets": 0,
    "lastPacketAtMs": 0,
}


class TcpPeer:
    def __init__(self, sock):
        self.sock = sock
        self.lock = threading.Lock()
        self.updated_at = time.time()

    def send_json_line(self, body):
        encoded = (json.dumps(body, separators=(",", ":")) + "\n").encode("utf-8")
        with self.lock:
            self.sock.sendall(encoded)
            self.updated_at = time.time()


def now_ms():
    return int(time.time() * 1000)


def prune():
    cutoff = now_ms()
    with lock:
        for room_id in list(offers):
            for peer_id in list(offers[room_id]):
                if offers[room_id][peer_id]["expiresAtMs"] <= cutoff:
                    del offers[room_id][peer_id]
            if not offers[room_id]:
                del offers[room_id]


def prune_udp_peers_locked(cutoff_time):
    expired = 0
    for key in list(udp_peers):
        if cutoff_time - udp_peers[key][1] > UDP_PEER_TTL_SECONDS:
            del udp_peers[key]
            expired += 1
    return expired


def relay_health_snapshot():
    with lock:
        prune_udp_peers_locked(time.time())
        return {
            "status": "ok",
            "schemaVersion": 1,
            "udp": {
                "bind": f"{UDP_BIND[0]}:{UDP_BIND[1]}",
                "peerTtlSeconds": UDP_PEER_TTL_SECONDS,
                "activePeerCount": len(udp_peers),
                "stats": dict(udp_stats),
            },
            "http": {
                "bind": f"{HTTP_BIND[0]}:{HTTP_BIND[1]}",
                "queuedPeerCount": len(http_peers),
            },
            "tcp": {
                "connectedPeerCount": len(tcp_peers),
            },
        }


def decode_relay_packet_kind(encoded):
    try:
        padded = encoded + "=" * (-len(encoded) % 4)
        envelope = json.loads(base64.b64decode(padded).decode("utf-8"))
        metadata = envelope.get("metadata") or {}
        return metadata.get("packet_kind") or metadata.get("packetKind") or ""
    except Exception:
        return ""


def relay_packet_priority(packet):
    kind = packet.get("packet_kind") or ""
    if kind in ("runtime-heartbeat", "runtime-heartbeat-ack"):
        return 0
    if kind == "runtime-ipv4-forward":
        return 1
    if kind == "runtime-udp-forward":
        return 2
    return 3


def fresh_http_packets(queue):
    cutoff = now_ms() - HTTP_PACKET_TTL_MS
    return [packet for packet in queue if int(packet.get("receivedAtMs") or 0) >= cutoff]


def trim_http_queue(queue):
    fresh = fresh_http_packets(queue)
    fresh.sort(key=lambda packet: (relay_packet_priority(packet), -int(packet.get("receivedAtMs") or 0)))
    del fresh[HTTP_QUEUE_LIMIT:]
    queue[:] = fresh


def preferred_offer_endpoint(offer):
    candidates = offer.get("candidates") if isinstance(offer, dict) else None
    if not isinstance(candidates, list) or not candidates:
        return None
    best = max(
        (
            candidate for candidate in candidates
            if isinstance(candidate, dict) and candidate.get("endpoint")
        ),
        key=lambda candidate: (int(candidate.get("priority") or 0), str(candidate.get("endpoint") or "")),
        default=None,
    )
    return best.get("endpoint") if best else None


def offer_candidate_signature(offer):
    candidates = offer.get("candidates") if isinstance(offer, dict) else None
    if not isinstance(candidates, list) or not candidates:
        return None
    values = []
    for candidate in candidates:
        if not isinstance(candidate, dict):
            continue
        endpoint = str(candidate.get("endpoint") or "")
        if not endpoint:
            continue
        values.append("{}:{}:{}:{}".format(
            candidate.get("candidate_type") or "",
            candidate.get("transport") or "",
            endpoint,
            int(candidate.get("priority") or 0),
        ))
    if not values:
        return None
    return "|".join(sorted(values))


def parse_binary_udp_relay_packet(data):
    header_len = len(UDP_BINARY_MAGIC) + 1 + 2 + 2 + 2 + 4
    if len(data) < header_len or not data.startswith(UDP_BINARY_MAGIC):
        return None
    offset = len(UDP_BINARY_MAGIC)
    kind = data[offset]
    offset += 1
    room_len = int.from_bytes(data[offset:offset + 2], "big")
    offset += 2
    from_len = int.from_bytes(data[offset:offset + 2], "big")
    offset += 2
    to_len = int.from_bytes(data[offset:offset + 2], "big")
    offset += 2
    payload_len = int.from_bytes(data[offset:offset + 4], "big")
    offset += 4
    total_len = offset + room_len + from_len + to_len + payload_len
    if total_len != len(data):
        return None
    room_id = data[offset:offset + room_len].decode("utf-8", "strict")
    offset += room_len
    from_peer = data[offset:offset + from_len].decode("utf-8", "strict")
    offset += from_len
    to_peer = data[offset:offset + to_len].decode("utf-8", "strict")
    return {
        "binary": True,
        "kind": kind,
        "room_id": room_id,
        "from_peer": from_peer,
        "to_peer": to_peer,
    }


class Handler(BaseHTTPRequestHandler):
    server_version = "LocalAreaInterconnectionRelay/1"

    def log_message(self, fmt, *args):
        return

    def send_json(self, status, body):
        encoded = json.dumps(body, separators=(",", ":")).encode("utf-8")
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(encoded)))
        self.send_header("Connection", "close")
        self.end_headers()
        self.wfile.write(encoded)

    def read_json(self):
        length = int(self.headers.get("Content-Length", "0") or "0")
        if length == 0:
            return {}
        return json.loads(self.rfile.read(length).decode("utf-8"))

    def do_GET(self):
        prune()
        parsed = urlparse(self.path)
        if parsed.path == "/health":
            self.send_json(200, relay_health_snapshot())
            return
        parts = [unquote(part) for part in parsed.path.strip("/").split("/") if part]
        if len(parts) == 4 and parts[:2] == ["v1", "rooms"] and parts[3] == "offers":
            room_id = parts[2]
            query = parse_qs(parsed.query)
            peer_id = (query.get("peer_id") or query.get("peerId") or [""])[0]
            with lock:
                room = offers.get(room_id, {})
                values = [
                    item["offer"]
                    for item in room.values()
                    if item["peerId"] != peer_id and item["expiresAtMs"] > now_ms()
                ]
            self.send_json(200, {"status": "ok", "room_id": room_id, "peer_id": peer_id, "offers": values, "expired_peer_count": 0})
            return
        if len(parts) == 4 and parts[:2] == ["v1", "rooms"] and parts[3] == "view":
            room_id = parts[2]
            query = parse_qs(parsed.query)
            peer_id = (query.get("peer_id") or query.get("peerId") or [""])[0]
            with lock:
                room = offers.get(room_id, {})
                peers = [
                    {
                        "peerId": item["peerId"],
                        "virtualIp": item["offer"].get("virtual_ip"),
                        "offerCreatedAtMs": item["offer"].get("created_at_ms"),
                        "status": "online" if item["expiresAtMs"] > now_ms() else "expired",
                        "updatedAtMs": item["updatedAtMs"],
                    }
                    for item in room.values()
                ]
            members = [
                {
                    "peer_id": peer["peerId"],
                    "virtual_ip": peer["virtualIp"],
                    "status": peer["status"],
                    "is_host": index == 0,
                    "candidate_count": len(room.get(peer["peerId"], {}).get("offer", {}).get("candidates", [])) if isinstance(room.get(peer["peerId"], {}).get("offer", {}), dict) else 0,
                    "preferred_endpoint": preferred_offer_endpoint(room.get(peer["peerId"], {}).get("offer", {})),
                    "candidate_signature": offer_candidate_signature(room.get(peer["peerId"], {}).get("offer", {})),
                    "offer_created_at_ms": peer.get("offerCreatedAtMs"),
                    "last_seen_ms": peer["updatedAtMs"],
                    "expires_at_ms": room.get(peer["peerId"], {}).get("expiresAtMs", 0),
                }
                for index, peer in enumerate(peers)
            ]
            online_count = len([peer for peer in peers if peer["status"] == "online"])
            expired_count = len([peer for peer in peers if peer["status"] == "expired"])
            remote_online = any(peer["peerId"] != peer_id and peer["status"] == "online" for peer in peers)
            if not peers:
                next_action = "Publish a local offer to the coordination server."
            elif not any(peer["peerId"] == peer_id for peer in peers):
                next_action = "Publish or heartbeat the local peer before waiting for others."
            elif not remote_online:
                next_action = "Wait for remote peers to publish offers, then start runtime bootstrap."
            else:
                next_action = "Start or refresh runtime bootstrap with the listed peers."
            self.send_json(200, {
                "status": "ready" if remote_online else "waiting",
                "roomId": room_id,
                "room_id": room_id,
                "local_peer_id": peer_id,
                "peers": peers,
                "members": members,
                "member_count": len(members),
                "online_count": online_count,
                "expired_count": expired_count,
                "next_action": next_action,
            })
            return
        if len(parts) == 3 and parts[:2] == ["v1", "relay"] and parts[2] == "poll":
            query = parse_qs(parsed.query)
            room_id = (query.get("room_id") or query.get("roomId") or [""])[0]
            peer_id = (query.get("peer_id") or query.get("peerId") or [""])[0]
            if not room_id or not peer_id:
                self.send_json(400, {"status": "error", "error": "missing room_id or peer_id"})
                return
            deadline = time.time() + min(float((query.get("timeout_ms") or ["1000"])[0]) / 1000.0, 5.0)
            packets = []
            while time.time() <= deadline:
                with lock:
                    queue = http_peers.setdefault((room_id, peer_id), [])
                    if queue:
                        queue[:] = fresh_http_packets(queue)
                        queue.sort(key=lambda packet: (relay_packet_priority(packet), int(packet.get("receivedAtMs") or 0)))
                        packets = queue[:HTTP_POLL_LIMIT]
                        del queue[:HTTP_POLL_LIMIT]
                if packets:
                    break
                time.sleep(0.05)
            self.send_json(200, {"status": "ok", "room_id": room_id, "peer_id": peer_id, "packets": packets})
            return
        self.send_json(404, {"status": "error", "error": "not found"})

    def do_POST(self):
        prune()
        parsed = urlparse(self.path)
        parts = [unquote(part) for part in parsed.path.strip("/").split("/") if part]
        if parsed.path == "/v1/offers":
            body = self.read_json()
            offer = body.get("offer") or {}
            room_id = offer.get("room_id")
            peer_id = offer.get("peer_id")
            ttl_ms = int(body.get("ttlMs") or body.get("ttl_ms") or OFFER_TTL_SECONDS * 1000)
            if not room_id or not peer_id:
                self.send_json(400, {"status": "error", "error": "missing room_id or peer_id"})
                return
            expires_at_ms = now_ms() + ttl_ms
            with lock:
                room = offers.setdefault(room_id, {})
                room[peer_id] = {
                    "peerId": peer_id,
                    "offer": offer,
                    "updatedAtMs": now_ms(),
                    "expiresAtMs": expires_at_ms,
                }
                remote_offer_count = len([item for item in room.values() if item["peerId"] != peer_id and item["expiresAtMs"] > now_ms()])
            self.send_json(200, {"status": "ok", "room_id": room_id, "peer_id": peer_id, "expires_at_ms": expires_at_ms, "remote_offer_count": remote_offer_count})
            return
        if parsed.path == "/v1/relay/register":
            body = self.read_json()
            room_id = body.get("room_id") or body.get("roomId")
            peer_id = body.get("peer_id") or body.get("peerId") or body.get("from_peer_id") or body.get("fromPeerId")
            if not room_id or not peer_id:
                self.send_json(400, {"status": "error", "error": "missing room_id or peer_id"})
                return
            with lock:
                http_peers.setdefault((room_id, peer_id), [])
            self.send_json(200, {"status": "ok", "room_id": room_id, "peer_id": peer_id})
            return
        if parsed.path == "/v1/relay/send":
            body = self.read_json()
            room_id = body.get("room_id") or body.get("roomId")
            from_peer = body.get("from_peer_id") or body.get("fromPeerId") or body.get("peer_id") or body.get("peerId")
            to_peer = body.get("to_peer_id") or body.get("toPeerId") or body.get("target_peer_id") or body.get("targetPeerId")
            encoded = body.get("bytes")
            if not room_id or not from_peer or not to_peer or not encoded:
                self.send_json(400, {"status": "error", "error": "missing relay fields"})
                return
            packet = {
                "kind": "relay-http-forward",
                "room_id": room_id,
                "from_peer_id": from_peer,
                "to_peer_id": to_peer,
                "bytes": encoded,
                "packet_kind": decode_relay_packet_kind(encoded),
                "receivedAtMs": now_ms(),
            }
            with lock:
                queue = http_peers.setdefault((room_id, to_peer), [])
                queue.append(packet)
                trim_http_queue(queue)
            self.send_json(200, {"status": "ok", "room_id": room_id, "from_peer_id": from_peer, "to_peer_id": to_peer})
            return
        if len(parts) >= 5 and parts[:2] == ["v1", "rooms"] and parts[3] == "peers":
            room_id, peer_id = parts[2], parts[4]
            action = parts[5] if len(parts) > 5 else ""
            with lock:
                if action == "leave":
                    offers.get(room_id, {}).pop(peer_id, None)
                elif action == "heartbeat" and peer_id in offers.get(room_id, {}):
                    offers[room_id][peer_id]["expiresAtMs"] = now_ms() + OFFER_TTL_SECONDS * 1000
                    offers[room_id][peer_id]["updatedAtMs"] = now_ms()
            self.send_json(200, {"status": "ok", "room_id": room_id, "peer_id": peer_id})
            return
        if len(parts) == 4 and parts[:2] == ["v1", "rooms"] and parts[3] == "close":
            room_id = parts[2]
            with lock:
                offers.pop(room_id, None)
                for key in list(udp_peers):
                    if key[0] == room_id:
                        del udp_peers[key]
            self.send_json(200, {"status": "ok", "room_id": room_id})
            return
        if parsed.path == "/v1/prune":
            self.send_json(200, {"status": "ok"})
            return
        self.send_json(404, {"status": "error", "error": "not found"})


class RelayMuxServer(socketserver.ThreadingMixIn, socketserver.TCPServer):
    allow_reuse_address = True
    daemon_threads = True


class RelayMuxHandler(socketserver.BaseRequestHandler):
    def handle(self):
        try:
            peek = self.request.recv(32, socket.MSG_PEEK)
        except Exception:
            return
        if peek.startswith(b"LAI-TCP-RELAY/1\n"):
            handle_tcp_relay_connection(self.request, self.client_address)
            return
        Handler(self.request, self.client_address, self.server)


def handle_tcp_relay_connection(sock, client_address):
    sock.setsockopt(socket.IPPROTO_TCP, socket.TCP_NODELAY, 1)
    sock.settimeout(60)
    file = sock.makefile("rb")
    registered_key = None
    peer = TcpPeer(sock)
    try:
        hello = file.readline(128)
        if hello != b"LAI-TCP-RELAY/1\n":
            return
        while True:
            line = file.readline(1024 * 1024)
            if not line:
                break
            try:
                packet = json.loads(line.decode("utf-8"))
            except Exception:
                continue
            kind = packet.get("kind") or packet.get("packetKind")
            room_id = packet.get("room_id") or packet.get("roomId")
            from_peer = packet.get("from_peer_id") or packet.get("fromPeerId") or packet.get("peer_id") or packet.get("peerId")
            to_peer = packet.get("to_peer_id") or packet.get("toPeerId") or packet.get("target_peer_id") or packet.get("targetPeerId")
            if not room_id or not from_peer:
                continue
            if kind == "tcp-register":
                registered_key = (room_id, from_peer)
                peer.updated_at = time.time()
                with lock:
                    tcp_peers[registered_key] = peer
                peer.send_json_line({"kind": "tcp-registered", "room_id": room_id, "peer_id": from_peer, "receivedAtMs": now_ms()})
                continue
            with lock:
                tcp_peers[(room_id, from_peer)] = peer
                target = tcp_peers.get((room_id, to_peer)) if to_peer else None
            if target:
                forwarded = {
                    "kind": "relay-tcp-forward",
                    "room_id": room_id,
                    "from_peer_id": from_peer,
                    "to_peer_id": to_peer,
                    "bytes": packet.get("bytes"),
                    "packet_kind": packet.get("packet_kind") or packet.get("packetKind") or decode_relay_packet_kind(packet.get("bytes") or ""),
                    "receivedAtMs": now_ms(),
                }
                try:
                    target.send_json_line(forwarded)
                except Exception:
                    with lock:
                        if tcp_peers.get((room_id, to_peer)) is target:
                            del tcp_peers[(room_id, to_peer)]
    finally:
        if registered_key:
            with lock:
                if tcp_peers.get(registered_key) is peer:
                    del tcp_peers[registered_key]
        try:
            file.close()
        except Exception:
            pass


def udp_loop():
    sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    try:
        sock.setsockopt(socket.SOL_SOCKET, socket.SO_RCVBUF, UDP_SOCKET_BUFFER_BYTES)
        sock.setsockopt(socket.SOL_SOCKET, socket.SO_SNDBUF, UDP_SOCKET_BUFFER_BYTES)
    except OSError:
        pass
    sock.bind(UDP_BIND)
    next_prune_at = time.time() + 1.0
    while True:
        data, addr = sock.recvfrom(65535)
        current_time = time.time()
        current_ms = now_ms()
        if current_time >= next_prune_at:
            with lock:
                prune_udp_peers_locked(current_time)
            next_prune_at = current_time + 1.0
        with lock:
            udp_stats["receivedPackets"] += 1
            udp_stats["lastPacketAtMs"] = current_ms
        binary_packet = parse_binary_udp_relay_packet(data)
        if binary_packet:
            try:
                with lock:
                    udp_stats["binaryPackets"] += 1
                room_id = binary_packet["room_id"]
                from_peer = binary_packet["from_peer"]
                to_peer = binary_packet["to_peer"]
                if not room_id or not from_peer:
                    with lock:
                        udp_stats["droppedPackets"] += 1
                    continue
                with lock:
                    udp_peers[(room_id, from_peer)] = (addr, current_time)
                    target = udp_peers.get((room_id, to_peer)) if to_peer else None
                if binary_packet["kind"] == UDP_BINARY_FORWARD and target:
                    sock.sendto(data, target[0])
                    with lock:
                        udp_stats["forwardedPackets"] += 1
                elif binary_packet["kind"] == UDP_BINARY_REGISTER:
                    pass
                else:
                    with lock:
                        udp_stats["droppedPackets"] += 1
            except Exception:
                with lock:
                    udp_stats["droppedPackets"] += 1
            continue
        try:
            packet = json.loads(data.decode("utf-8"))
            packet_type = packet.get("type") or packet.get("kind") or packet.get("packetKind")
            with lock:
                udp_stats["jsonPackets"] += 1
            if packet_type == "stun-like-query":
                response = {
                    "schemaVersion": 1,
                    "type": "stun-like-response",
                    "status": "ok",
                    "observedEndpoint": f"{addr[0]}:{addr[1]}",
                    "serverEndpoint": f"{UDP_BIND[0]}:{UDP_BIND[1]}",
                    "receivedBytes": len(data),
                    "request": packet,
                }
                sock.sendto(json.dumps(response, separators=(",", ":")).encode("utf-8"), addr)
                with lock:
                    udp_stats["stunQueries"] += 1
                continue
            room_id = packet.get("room_id") or packet.get("roomId")
            from_peer = packet.get("from_peer_id") or packet.get("fromPeerId") or packet.get("peer_id") or packet.get("peerId")
            to_peer = packet.get("to_peer_id") or packet.get("toPeerId") or packet.get("target_peer_id") or packet.get("targetPeerId")
            if not room_id or not from_peer:
                with lock:
                    udp_stats["droppedPackets"] += 1
                continue
            with lock:
                udp_peers[(room_id, from_peer)] = (addr, current_time)
                target = udp_peers.get((room_id, to_peer)) if to_peer else None
            if target:
                sock.sendto(data, target[0])
                with lock:
                    udp_stats["forwardedPackets"] += 1
            else:
                with lock:
                    udp_stats["droppedPackets"] += 1
        except Exception:
            with lock:
                udp_stats["droppedPackets"] += 1
            continue


def main():
    threading.Thread(target=udp_loop, daemon=True).start()
    RelayMuxServer(HTTP_BIND, RelayMuxHandler).serve_forever()


if __name__ == "__main__":
    main()
