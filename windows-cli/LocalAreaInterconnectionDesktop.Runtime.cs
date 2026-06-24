using System;
using System.Collections.Generic;
using System.Globalization;
using System.Diagnostics;
using System.Drawing;
using System.Drawing.Drawing2D;
using System.IO;
using System.Net;
using System.Net.NetworkInformation;
using System.Net.Sockets;
using System.Runtime.InteropServices;
using System.Text;
using System.Threading.Tasks;
using System.Windows.Forms;

public partial class LocalAreaInterconnectionDesktop
{
    const int CoordinationOfferTtlMs = 300000;

    string InvitePayloadJson()
    {
        string value = invite == null ? "" : invite.Text.Trim();
        if (value.Length == 0) return "";
        int dot = value.IndexOf('.');
        if (dot >= 0) value = value.Substring(0, dot);
        value = value.Replace('-', '+').Replace('_', '/');
        while (value.Length % 4 != 0) value += "=";
        try
        {
            return Encoding.UTF8.GetString(Convert.FromBase64String(value));
        }
        catch
        {
            return "";
        }
    }

    string RuntimeRoomId()
    {
        string roomId = JsonStringValue(InvitePayloadJson(), "room_id");
        if (roomId.Length == 0) roomId = SafePeerId(roomName.Text);
        return roomId.Length == 0 ? "desktop_runtime" : SafePeerId(roomId);
    }

    string RuntimeRoomKey()
    {
        return "desktop-runtime-room-key-" + RuntimeRoomId();
    }

    string HostVirtualIp()
    {
        return OffsetIpFromSubnet(subnet.Text.Trim(), 1);
    }

    string PeerVirtualIp()
    {
        return OffsetIpFromSubnet(subnet.Text.Trim(), 2);
    }

    string OffsetIpFromSubnet(string subnetValue, int offset)
    {
        try
        {
            string network = subnetValue.Split('/')[0];
            string[] parts = network.Split('.');
            if (parts.Length != 4) return "";
            int a = Int32.Parse(parts[0], CultureInfo.InvariantCulture);
            int b = Int32.Parse(parts[1], CultureInfo.InvariantCulture);
            int c = Int32.Parse(parts[2], CultureInfo.InvariantCulture);
            int d = Int32.Parse(parts[3], CultureInfo.InvariantCulture) + offset;
            if (a < 0 || a > 255 || b < 0 || b > 255 || c < 0 || c > 255 || d < 0 || d > 255) return "";
            return a.ToString(CultureInfo.InvariantCulture) + "."
                + b.ToString(CultureInfo.InvariantCulture) + "."
                + c.ToString(CultureInfo.InvariantCulture) + "."
                + d.ToString(CultureInfo.InvariantCulture);
        }
        catch
        {
            return "";
        }
    }

    string InviteHostPeerId()
    {
        string hostPeer = JsonStringValue(InvitePayloadJson(), "host_peer_id");
        if (hostPeer.Length > 0)
        {
            hostRuntimePeerId = SafePeerId(hostPeer);
        }
        return hostRuntimePeerId;
    }

    string RuntimePeerId()
    {
        if (localRuntimePeerId.Length > 0) return SafePeerId(localRuntimePeerId);
        string hostPeer = InviteHostPeerId();
        string hostIp = HostVirtualIp();
        if (hostPeer.Length > 0 && ip.Text.Trim() == hostIp)
        {
            localRuntimePeerId = hostPeer;
            return localRuntimePeerId;
        }
        string suffix = ip.Text.Trim().Replace('.', '_').Replace(':', '_');
        localRuntimePeerId = SafePeerId(hostName.Text + (suffix.Length > 0 ? "_" + suffix : ""));
        return localRuntimePeerId;
    }

    string RuntimeFilePrefix()
    {
        return RuntimeRoomId() + "-" + RuntimePeerId();
    }

    string RuntimeFilePath(string name, string extension)
    {
        return Path.Combine(LogDirectory(), name + "-" + RuntimeFilePrefix() + "." + extension);
    }

    string RuntimeRoomFilePath(string name, string extension)
    {
        return Path.Combine(LogDirectory(), name + "-" + RuntimeRoomId() + "." + extension);
    }

    string NetshExportArgs()
    {
        string path = netshOutput.Text.Trim();
        if (path.Length == 0) return "";
        return " --firewall-netsh-output " + Quote(path);
    }

    string ObserveFileArgs()
    {
        string path = packetObservations.Text.Trim();
        if (path.Length == 0) return "";
        return " --observe-file " + Quote(path);
    }

    string PacketObservationArgs()
    {
        string path = packetObservations.Text.Trim();
        if (path.Length == 0 || !File.Exists(path)) return "";
        return " --packet-observations " + Quote(path);
    }

    string RuntimeSnapshotArgs()
    {
        string path = latestRuntimeSnapshot;
        if (path.Length == 0)
        {
            path = RuntimeFilePath("runtime-snapshot", "json");
        }
        if (!File.Exists(path)) return "";
        return " --runtime-snapshot " + Quote(path);
    }

    string GameCatalogArgs()
    {
        string path = gameCatalog.Text.Trim();
        if (path.Length == 0) return "";
        return " --catalog " + Quote(path);
    }

    string FirewallReadinessArgs()
    {
        string path = netshOutput.Text.Trim();
        if (path.Length > 0)
        {
            return " --firewall-netsh-output " + Quote(path);
        }
        return " --firewall-scan true";
    }

    string RuntimeCoordinationArgs()
    {
        string args = "";
        string server = NormalizeCoordinationServer(coordinationServer.Text.Trim());
        string peer = remotePeer.Text.Trim();
        string bootstrap = RuntimeDirectBootstrapArgs(peer);
        if (bootstrap.Length > 0)
        {
            args += bootstrap;
        }
        else if (server.Length > 0 && peer.Length > 0)
        {
            string peerId;
            string virtualIp;
            string offerValue;
            if (TryParseRemotePeerOfferSpec(peer, out peerId, out virtualIp, out offerValue))
            {
                peer = SafePeerId(peerId) + "," + virtualIp;
            }
            else if (TryParseCoordinationPeerSpec(peer, out peerId, out virtualIp))
            {
                peer = SafePeerId(peerId) + "," + virtualIp;
            }
            args += " --coordination-server " + Quote(server);
            args += " --coordination-peer " + Quote(peer);
        }
        return args;
    }

    string DefaultCoordinationServer()
    {
        return "http://49.235.146.152";
    }

    string DefaultHostName()
    {
        try
        {
            string name = Environment.MachineName.Trim();
            return name.Length > 0 ? name : "Alice";
        }
        catch
        {
            return "Alice";
        }
    }

    string DefaultRelayServer()
    {
        return "49.235.146.152:39091";
    }

    string DefaultStunServer()
    {
        return "49.235.146.152:39091,stun.l.google.com:19302,stun.cloudflare.com:3478";
    }

    string RuntimeDirectBootstrapArgs(string peerSpec)
    {
        string peerId;
        string virtualIp;
        string offerValue;
        if (!TryParseRemotePeerOfferSpec(peerSpec, out peerId, out virtualIp, out offerValue))
        {
            return "";
        }
        string preparedOffer = PrepareOfferArgumentFile(offerValue, "remote-offer-direct.json");
        if (preparedOffer.Length == 0)
        {
            return "";
        }
        string spec = SafePeerId(peerId) + "," + virtualIp + "," + preparedOffer;
        return " --nat-bootstrap-remote-peer " + Quote(spec)
            + " --nat-bootstrap-attempts 60"
            + " --nat-bootstrap-interval-ms 80"
            + " --nat-bootstrap-timeout-ms 12000";
    }

    bool TryParseRemotePeerOfferSpec(string value, out string peerId, out string virtualIp, out string offerValue)
    {
        peerId = "";
        virtualIp = "";
        offerValue = "";
        value = value.Trim();
        if (value.Length == 0) return false;
        if (value.StartsWith("{", StringComparison.Ordinal))
        {
            peerId = JsonStringValue(value, "peer_id");
            virtualIp = JsonStringValue(value, "virtual_ip");
            offerValue = value;
            return peerId.Length > 0 && LooksLikeIpv4(virtualIp) && offerValue.Length > 0;
        }
        int first = value.IndexOf(',');
        if (first < 0) return false;
        int second = value.IndexOf(',', first + 1);
        if (second < 0) return false;
        peerId = value.Substring(0, first).Trim();
        virtualIp = value.Substring(first + 1, second - first - 1).Trim();
        offerValue = value.Substring(second + 1).Trim();
        return peerId.Length > 0 && LooksLikeIpv4(virtualIp) && offerValue.Length > 0;
    }

    bool TryParseCoordinationPeerSpec(string value, out string peerId, out string virtualIp)
    {
        peerId = "";
        virtualIp = "";
        string offerValue;
        if (TryParseRemotePeerOfferSpec(value, out peerId, out virtualIp, out offerValue))
        {
            return true;
        }
        value = value.Trim();
        if (value.Length == 0) return false;
        if (value.StartsWith("{", StringComparison.Ordinal))
        {
            peerId = JsonStringValue(value, "peer_id");
            virtualIp = JsonStringValue(value, "virtual_ip");
            return peerId.Length > 0 && LooksLikeIpv4(virtualIp);
        }
        int comma = value.IndexOf(',');
        if (comma < 0) return false;
        peerId = value.Substring(0, comma).Trim();
        virtualIp = value.Substring(comma + 1).Trim();
        int nextComma = virtualIp.IndexOf(',');
        if (nextComma >= 0)
        {
            virtualIp = virtualIp.Substring(0, nextComma).Trim();
        }
        return peerId.Length > 0 && LooksLikeIpv4(virtualIp);
    }

    string RuntimeCoordinationMonitorArgs()
    {
        if (coordinationServer.Text.Trim().Length == 0 && coordinationStoreFile.Length == 0)
        {
            return "";
        }
        return " --coordination-monitor true --coordination-monitor-interval-ms 1000";
    }

    string RelayExportArgs()
    {
        string peer = RuntimePeerId();
        if (CreateNativeOffer(peer, false).Length == 0)
        {
            return "";
        }
        string remoteOffer = RemoteOfferForRelayPlan(peer);
        if (remoteOffer.Length == 0)
        {
            return "";
        }
        return " --relay-local-offer " + Quote(latestNativeOfferFile)
            + " --relay-remote-offer " + Quote(remoteOffer)
            + " --relay-p2p-status failed";
    }

    string RemoteOfferForRelayPlan(string localPeer)
    {
        string value = remotePeer.Text.Trim();
        if (value.Length == 0)
        {
            return "";
        }

        string explicitOffer = RemoteOfferPart(value);
        string prepared = PrepareOfferArgumentFile(explicitOffer, "remote-offer-relay.json");
        if (prepared.Length > 0)
        {
            return prepared;
        }

        string server = coordinationServer.Text.Trim();
        string remoteId = RemotePeerIdForKick();
        if (server.Length == 0 || remoteId.Length == 0)
        {
            return "";
        }

        string fetch = RunNativeCliCapture("coordination-http-offer-fetch"
            + " --server " + Quote(server)
            + " --room-id " + Quote(RuntimeRoomId())
            + " --peer-id " + Quote(localPeer));
        string offer = NatOfferObjectByPeer(fetch, remoteId);
        if (offer.Length == 0)
        {
            return "";
        }
        string path = RuntimeFilePath("remote-offer-relay-" + remoteId, "json");
        File.WriteAllText(path, offer + Environment.NewLine, Encoding.UTF8);
        return path;
    }

    string RemoteOfferPart(string value)
    {
        int first = value.IndexOf(',');
        if (first < 0) return value.Trim();
        int second = value.IndexOf(',', first + 1);
        if (second < 0) return value.Trim();
        return value.Substring(second + 1).Trim();
    }

    string PrepareOfferArgumentFile(string value, string fileName)
    {
        if (value.Length == 0)
        {
            return "";
        }
        if (File.Exists(value))
        {
            return value;
        }
        if (value.StartsWith("{", StringComparison.Ordinal))
        {
            string path = Path.Combine(LogDirectory(), fileName);
            File.WriteAllText(path, value + Environment.NewLine, Encoding.UTF8);
            return path;
        }
        return "";
    }

    string NatOfferObjectByPeer(string json, string peerId)
    {
        string array = JsonArrayValue(json, "offers");
        if (array.Length == 0) return "";
        int search = 0;
        while (search < array.Length)
        {
            int start = array.IndexOf('{', search);
            if (start < 0) break;
            int end = MatchingJsonBrace(array, start);
            if (end < 0) break;
            string offer = array.Substring(start, end - start + 1);
            if (JsonStringValue(offer, "peer_id") == peerId)
            {
                return offer;
            }
            search = end + 1;
        }
        return "";
    }

    void RefreshCoordinationPresence()
    {
        if (coordinationServer.Text.Trim().Length == 0)
        {
            return;
        }
        if (invite.Text.Trim().Length == 0 && roomName.Text.Trim().Length == 0)
        {
            return;
        }
        if ((DateTime.UtcNow - lastCoordinationRefreshUtc).TotalSeconds < 15)
        {
            return;
        }
        if (coordinationPresenceRefreshRunning)
        {
            return;
        }
        if (runtimeProcess != null && !runtimeProcess.HasExited)
        {
            lastCoordinationRefreshUtc = DateTime.UtcNow;
            RefreshCoordinationRoomView(false);
            return;
        }
        lastCoordinationRefreshUtc = DateTime.UtcNow;
        string peer = RuntimePeerId();
        string roomId = RuntimeRoomId();
        string virtualIp = ip.Text.Trim();
        string bind = NativeRuntimeBind();
        string stun = stunServer.Text.Trim();
        string server = coordinationServer.Text.Trim();
        coordinationPresenceRefreshRunning = true;
        Task.Factory.StartNew(delegate
        {
            try
            {
                if (latestNativeOfferFile.Length == 0 || !File.Exists(latestNativeOfferFile))
                {
                    CreateNativeOfferSnapshot(roomId, peer, virtualIp, bind, stun, false);
                }
                PublishNativeOfferFileIfConfigured(server, false);
            }
            catch
            {
            }
            if (IsDisposed || !IsHandleCreated)
            {
                coordinationPresenceRefreshRunning = false;
                return;
            }
            try
            {
                BeginInvoke((MethodInvoker)delegate
                {
                    coordinationPresenceRefreshRunning = false;
                    RefreshCoordinationRoomView(false);
                });
            }
            catch
            {
                coordinationPresenceRefreshRunning = false;
            }
        });
    }

    string PublishNativeOfferIfConfigured(string peer, bool showOutput)
    {
        if (coordinationServer.Text.Trim().Length == 0) return "";
        if (CreateNativeOffer(peer, false).Length == 0) return "";
        return PublishNativeOfferFileIfConfigured(showOutput);
    }

    string PublishNativeOfferFileIfConfigured(bool showOutput)
    {
        return PublishNativeOfferFileIfConfigured(coordinationServer.Text.Trim(), showOutput);
    }

    string PublishNativeOfferFileIfConfigured(string server, bool showOutput)
    {
        if (server.Length == 0 || latestNativeOfferFile.Length == 0 || !File.Exists(latestNativeOfferFile))
        {
            return "";
        }
        string arguments = "coordination-http-offer-publish"
            + " --server " + Quote(server)
            + " --offer " + Quote(latestNativeOfferFile)
            + " --ttl-ms " + CoordinationOfferTtlMs.ToString(CultureInfo.InvariantCulture);
        return showOutput ? RunNativeCli(arguments) : RunNativeCliCapture(arguments, 12000);
    }

    bool CoordinationPublishLooksSuccessful(string text)
    {
        return JsonStringValue(text, "status") == "ok";
    }

    string LeaveCoordinationRoomIfConfigured()
    {
        string server = coordinationServer.Text.Trim();
        if (server.Length == 0)
        {
            return "";
        }
        string peer = RuntimePeerId();
        string result = RunNativeCliCapture("coordination-http-leave"
            + " --server " + Quote(server)
            + " --room-id " + Quote(RuntimeRoomId())
            + " --peer-id " + Quote(peer));
        RefreshCoordinationRoomView(false);
        return T("coordinationLeft") + Environment.NewLine + result;
    }

    string CloseCoordinationRoomIfConfigured()
    {
        string server = coordinationServer.Text.Trim();
        if (server.Length == 0)
        {
            return T("coordinationServerRequired");
        }
        string result = RunNativeCliCapture("coordination-http-close"
            + " --server " + Quote(server)
            + " --room-id " + Quote(RuntimeRoomId())
            + " --closed-by " + Quote(RuntimePeerId()));
        return T("coordinationRoomClosed") + Environment.NewLine + result;
    }

    string KickCoordinationPeerIfConfigured()
    {
        string server = coordinationServer.Text.Trim();
        if (server.Length == 0)
        {
            return T("coordinationServerRequired");
        }
        string peer = RemotePeerIdForKick();
        if (peer.Length == 0)
        {
            return T("coordinationPeerRequired");
        }
        string kickedBy = RuntimePeerId();
        string result = RunNativeCliCapture("coordination-http-kick"
            + " --server " + Quote(server)
            + " --room-id " + Quote(RuntimeRoomId())
            + " --peer-id " + Quote(peer)
            + " --kicked-by " + Quote(kickedBy));
        return T("coordinationPeerKicked") + Environment.NewLine + result;
    }

    string RemotePeerIdForKick()
    {
        string value = remotePeer.Text.Trim();
        if (value.Length == 0)
        {
            return "";
        }
        int comma = value.IndexOf(',');
        if (comma >= 0)
        {
            value = value.Substring(0, comma).Trim();
        }
        return SafePeerId(value);
    }

    string CreateNativeOffer(string peer, bool showOutput)
    {
        return CreateNativeOfferSnapshot(
            RuntimeRoomId(),
            peer,
            ip.Text.Trim(),
            AllocateNativeRuntimeBind(),
            stunServer.Text.Trim(),
            showOutput);
    }

    string CreateNativeOfferSnapshot(string roomId, string peer, string virtualIp, string bind, string stunServerValue, bool showOutput)
    {
        latestNativeOfferFile = RuntimeFilePath("native-offer", "json");
        string arguments = "nat-candidates"
            + " --room-id " + Quote(roomId)
            + " --peer-id " + Quote(peer)
            + " --virtual-ip " + Quote(virtualIp)
            + " --bind " + Quote(bind)
            + RelayCandidateArgs()
            + StunArgs(stunServerValue)
            + UpnpPortMapArgs()
            + " --nonce " + Quote(peer + "-desktop-offer");
        string text = showOutput ? RunNativeCli(arguments) : RunNativeCliCapture(arguments, 30000);
        string offer = JsonObjectValue(text, "offer");
        if (offer.Length > 0)
        {
            File.WriteAllText(latestNativeOfferFile, offer + Environment.NewLine, Encoding.UTF8);
            latestNativeOfferBind = bind;
            if (showOutput)
            {
                output.Text = text + Environment.NewLine + T("nativeOfferPath") + latestNativeOfferFile;
            }
            return text;
        }
        if (showOutput)
        {
            output.Text = text;
        }
        return "";
    }

    string StunArgs()
    {
        return StunArgs(stunServer.Text.Trim());
    }

    string RelayCandidateArgs()
    {
        if (relayServer == null) return "";
        string value = NormalizeRelayServer(relayServer.Text.Trim());
        if (value.Length == 0) return "";
        return " --relay " + Quote(value);
    }

    string NormalizeCoordinationServer(string value)
    {
        value = value.Trim();
        if (value.Length == 0) return "";
        if (value.IndexOf("://", StringComparison.Ordinal) >= 0) return value;
        return "http://" + value;
    }

    string NormalizeRelayServer(string value)
    {
        value = value.Trim();
        if (value.Length == 0) return "";
        if (IsLegacyDefaultRelayServer(value)) return DefaultRelayServer();
        if (value.IndexOf("://", StringComparison.Ordinal) >= 0) return value;
        if (value.IndexOf(':') >= 0) return value;
        return value + ":39091";
    }

    bool IsLegacyDefaultRelayServer(string value)
    {
        return value.Equals("http://49.235.146.152", StringComparison.OrdinalIgnoreCase)
            || value.Equals("https://49.235.146.152", StringComparison.OrdinalIgnoreCase);
    }

    string StunArgs(string server)
    {
        if (server.Length == 0) return "";
        return " --stun-server " + Quote(server) + " --stun-timeout-ms 2500";
    }

    string RuntimeNatBootstrapStunArgs(string server)
    {
        if (server.Length == 0) return "";
        return " --nat-bootstrap-stun-server " + Quote(server)
            + " --nat-bootstrap-stun-timeout-ms 2500";
    }

    string UpnpPortMapArgs()
    {
        return UpnpPortMapEnabled()
            ? " --upnp-port-map true --upnp-timeout-ms 1500 --upnp-lease-seconds 7200"
            : "";
    }

    string RuntimeNatBootstrapUpnpArgs()
    {
        return UpnpPortMapEnabled()
            ? " --nat-bootstrap-upnp-port-map true --nat-bootstrap-upnp-timeout-ms 1500 --nat-bootstrap-upnp-lease-seconds 7200"
            : "";
    }

    string RuntimeCoordinationPublishArgs(string server)
    {
        server = NormalizeCoordinationServer(server);
        if (server.Length == 0) return "";
        return " --coordination-publish-ttl-ms " + CoordinationOfferTtlMs.ToString(CultureInfo.InvariantCulture);
    }

    bool UpnpPortMapEnabled()
    {
        if (upnpPortMap == null) return false;
        string value = upnpPortMap.Text.Trim();
        return value.Equals("true", StringComparison.OrdinalIgnoreCase)
            || value.Equals("yes", StringComparison.OrdinalIgnoreCase)
            || value.Equals("on", StringComparison.OrdinalIgnoreCase)
            || value == "1";
    }

    string NativeRuntimeBind()
    {
        return "0.0.0.0:" + NativeRuntimePort().ToString(CultureInfo.InvariantCulture);
    }

    string AllocateNativeRuntimeBind()
    {
        int preferred = NativeRuntimePort();
        int port = FirstAvailableUdpPort(preferred, 200);
        return "0.0.0.0:" + port.ToString(CultureInfo.InvariantCulture);
    }

    int FirstAvailableUdpPort(int preferred, int attempts)
    {
        for (int offset = 0; offset <= attempts; offset++)
        {
            int port = preferred + offset;
            if (port > 65535) break;
            if (CanBindUdpPort(port)) return port;
        }
        for (int port = 39000; port <= 39250; port++)
        {
            if (CanBindUdpPort(port)) return port;
        }
        return preferred;
    }

    bool CanBindUdpPort(int port)
    {
        try
        {
            using (Socket socket = new Socket(AddressFamily.InterNetwork, SocketType.Dgram, ProtocolType.Udp))
            {
                socket.ExclusiveAddressUse = true;
                socket.Bind(new IPEndPoint(IPAddress.Any, port));
                return true;
            }
        }
        catch
        {
            return false;
        }
    }

    int RuntimePortFromBind(string bind)
    {
        int colon = bind.LastIndexOf(':');
        int port;
        if (colon >= 0 && Int32.TryParse(bind.Substring(colon + 1), out port))
        {
            return port;
        }
        return NativeRuntimePort();
    }

    int NativeRuntimePort()
    {
        string[] parts = ip.Text.Trim().Split('.');
        int last;
        if (parts.Length == 4 && Int32.TryParse(parts[3], out last) && last > 0 && last < 255)
        {
            return 39000 + last;
        }
        return 39090;
    }

    string CoordinationBind()
    {
        string value = coordinationServer.Text.Trim();
        if (value.Length == 0) return "0.0.0.0:39110";
        if (value.StartsWith("http://", StringComparison.OrdinalIgnoreCase))
        {
            value = value.Substring("http://".Length);
        }
        if (value.StartsWith("https://", StringComparison.OrdinalIgnoreCase))
        {
            value = value.Substring("https://".Length);
        }
        int slash = value.IndexOf('/');
        if (slash >= 0)
        {
            value = value.Substring(0, slash);
        }
        int colon = value.LastIndexOf(':');
        string port = colon >= 0 ? value.Substring(colon + 1) : "39110";
        string host = colon >= 0 ? value.Substring(0, colon) : value;
        if (host.Length == 0 || host == "0.0.0.0") return "0.0.0.0:" + port;
        if (host == "127.0.0.1" || host.Equals("localhost", StringComparison.OrdinalIgnoreCase))
        {
            return host + ":" + port;
        }
        return "0.0.0.0:" + port;
    }

    string CoordinationPort()
    {
        string bind = CoordinationBind();
        int colon = bind.LastIndexOf(':');
        return colon >= 0 ? bind.Substring(colon + 1) : "39110";
    }

    string LocalCoordinationEndpoint()
    {
        string server = coordinationServer.Text.Trim();
        if (server.Length > 0) return server;
        return "http://" + PreferredLanIpv4() + ":39110";
    }

    bool RequireInternetCoordinationEndpoint(string actionKey)
    {
        string server = coordinationServer.Text.Trim();
        if (server.Length == 0)
        {
            output.Text = T("internetCoordinationRequired")
                + Environment.NewLine
                + T("internetCoordinationExample");
            return false;
        }
        string reason;
        if (!IsPublicCoordinationEndpoint(server, out reason))
        {
            output.Text = T("internetCoordinationInvalid")
                + Environment.NewLine + server
                + Environment.NewLine + reason
                + Environment.NewLine
                + T("internetCoordinationExample");
            return false;
        }
        return true;
    }

    bool IsPublicCoordinationEndpoint(string endpoint, out string reason)
    {
        reason = "";
        Uri uri;
        if (!Uri.TryCreate(endpoint, UriKind.Absolute, out uri)
            || (uri.Scheme != Uri.UriSchemeHttp && uri.Scheme != Uri.UriSchemeHttps)
            || uri.Host.Length == 0)
        {
            reason = T("internetCoordinationBadUrl");
            return false;
        }
        if (uri.Scheme == Uri.UriSchemeHttps)
        {
            reason = T("internetCoordinationHttpOnly");
            return false;
        }
        IPAddress address;
        if (IPAddress.TryParse(uri.Host, out address))
        {
            if (IsPrivateOrLocalAddress(address))
            {
                reason = T("internetCoordinationPrivate");
                return false;
            }
            return true;
        }
        string host = uri.Host.Trim().TrimEnd('.');
        if (host.Equals("localhost", StringComparison.OrdinalIgnoreCase)
            || host.IndexOf('.') < 0)
        {
            reason = T("internetCoordinationPrivate");
            return false;
        }
        return true;
    }

    bool IsPrivateOrLocalAddress(IPAddress address)
    {
        if (IPAddress.IsLoopback(address)) return true;
        if (address.AddressFamily == AddressFamily.InterNetwork)
        {
            byte[] bytes = address.GetAddressBytes();
            if (bytes[0] == 10) return true;
            if (bytes[0] == 172 && bytes[1] >= 16 && bytes[1] <= 31) return true;
            if (bytes[0] == 192 && bytes[1] == 168) return true;
            if (bytes[0] == 169 && bytes[1] == 254) return true;
            if (bytes[0] == 0) return true;
            return false;
        }
        if (address.AddressFamily == AddressFamily.InterNetworkV6)
        {
            byte[] bytes = address.GetAddressBytes();
            if (address.IsIPv6LinkLocal || address.IsIPv6SiteLocal) return true;
            if ((bytes[0] & 0xfe) == 0xfc) return true;
        }
        return false;
    }

    string PreferredLanIpv4()
    {
        try
        {
            NetworkInterface[] interfaces = NetworkInterface.GetAllNetworkInterfaces();
            for (int i = 0; i < interfaces.Length; i++)
            {
                NetworkInterface item = interfaces[i];
                if (item.OperationalStatus != OperationalStatus.Up) continue;
                if (item.NetworkInterfaceType != NetworkInterfaceType.Ethernet
                    && item.NetworkInterfaceType != NetworkInterfaceType.Wireless80211) continue;
                IPInterfaceProperties props = item.GetIPProperties();
                if (props.GatewayAddresses.Count == 0) continue;
                foreach (UnicastIPAddressInformation address in props.UnicastAddresses)
                {
                    if (address.Address.AddressFamily != AddressFamily.InterNetwork) continue;
                    string text = address.Address.ToString();
                    if (text.StartsWith("127.", StringComparison.Ordinal) || text.StartsWith("169.254.", StringComparison.Ordinal)) continue;
                    return text;
                }
            }
        }
        catch
        {
        }
        try
        {
            IPAddress[] addresses = Dns.GetHostEntry(Dns.GetHostName()).AddressList;
            for (int i = 0; i < addresses.Length; i++)
            {
                IPAddress address = addresses[i];
                if (address.AddressFamily != AddressFamily.InterNetwork) continue;
                string text = address.ToString();
                if (text.StartsWith("127.", StringComparison.Ordinal) || text.StartsWith("169.254.", StringComparison.Ordinal)) continue;
                return text;
            }
        }
        catch
        {
        }
        return "127.0.0.1";
    }

    void StartRuntimePostStartWork(
        bool showDetails,
        string roomId,
        string peer,
        string virtualIp,
        string bind,
        string stunServerValue,
        string coordinationServerValue,
        int runtimePort)
    {
        Task.Factory.StartNew(delegate
        {
            if (IsDisposed || !IsHandleCreated)
            {
                return;
            }
            try
            {
                BeginInvoke((MethodInvoker)delegate
                {
                    RefreshCoordinationRoomView(false);
                });
            }
            catch
            {
            }
        });
    }

    void AutoConfigureRemotePeerFromCoordinationView(string json)
    {
        if (restartingRuntimeForRemotePeer || json.Trim().Length == 0) return;
        string spec = RemotePeerSpecFromCoordinationView(json);
        if (spec.Length == 0) return;
        string current = remotePeer.Text.Trim();
        bool samePeer = RemotePeerTargetsSamePeer(current, spec);
        string offerSignature = RemotePeerOfferSignatureFromCoordinationView(json, spec);
        string previousOfferSignature = lastRemotePeerOfferSignature;
        if (offerSignature.Length > 0)
        {
            lastRemotePeerOfferSignature = offerSignature;
        }
        if (samePeer && !RuntimeShouldRetryP2pForSamePeer(spec, offerSignature, previousOfferSignature)) return;

        remotePeer.Text = spec;
        if (runtimeProcess == null || runtimeProcess.HasExited) return;

        lastRuntimeP2pRetryUtc = DateTime.UtcNow;
        lastRuntimeP2pRetrySpec = spec;
        lastRuntimeP2pRetrySignature = offerSignature;
        restartingRuntimeForRemotePeer = true;
        try
        {
            StopRuntimeProcess(5000);
            StartNativeRuntime(false);
        }
        finally
        {
            restartingRuntimeForRemotePeer = false;
        }
    }

    bool RuntimeShouldRetryP2pForSamePeer(string spec, string offerSignature, string previousOfferSignature)
    {
        if (runtimeProcess == null || runtimeProcess.HasExited) return false;
        if (lastRuntimeP2pRetrySpec == spec
            && lastRuntimeP2pRetrySignature == offerSignature
            && DateTime.UtcNow - lastRuntimeP2pRetryUtc < TimeSpan.FromSeconds(30))
        {
            return false;
        }

        string snapshot = lastRuntimeSnapshotText;
        try
        {
            if (latestRuntimeSnapshot.Length > 0 && File.Exists(latestRuntimeSnapshot))
            {
                snapshot = File.ReadAllText(latestRuntimeSnapshot);
            }
        }
        catch
        {
            snapshot = lastRuntimeSnapshotText;
        }
        if (snapshot.Trim().Length == 0)
        {
            return false;
        }

        if (!RuntimeHasConnectedPeer(snapshot) || RuntimeHasUnstablePeer(snapshot))
        {
            return true;
        }

        string path = RuntimePrimaryPathKind(snapshot);
        if (path == "relay"
            && offerSignature.Length > 0
            && previousOfferSignature.Length > 0
            && offerSignature != previousOfferSignature)
        {
            return true;
        }

        return false;
    }

    bool RemotePeerTargetsSamePeer(string current, string next)
    {
        string currentPeer;
        string currentIp;
        string nextPeer;
        string nextIp;
        if (!TryParseCoordinationPeerSpec(current, out currentPeer, out currentIp)) return false;
        if (!TryParseCoordinationPeerSpec(next, out nextPeer, out nextIp)) return false;
        return SafePeerId(currentPeer) == SafePeerId(nextPeer)
            && currentIp == nextIp;
    }

    string RemotePeerOfferSignatureFromCoordinationView(string json, string spec)
    {
        string specPeer;
        string specIp;
        if (!TryParseCoordinationPeerSpec(spec, out specPeer, out specIp)) return "";
        string members = JsonArrayValue(json, "members");
        if (members.Length == 0) members = JsonArrayValue(json, "peers");
        int search = 0;
        while (search < members.Length)
        {
            int start = members.IndexOf('{', search);
            if (start < 0) break;
            int end = MatchingJsonBrace(members, start);
            if (end < 0) break;
            string member = members.Substring(start, end - start + 1);
            string peer = JsonStringValue(member, "peer_id");
            if (peer.Length == 0) peer = JsonStringValue(member, "peerId");
            string virtualIp = JsonStringValue(member, "virtual_ip");
            if (virtualIp.Length == 0) virtualIp = JsonStringValue(member, "virtualIp");
            if (SafePeerId(peer) == SafePeerId(specPeer) && virtualIp == specIp)
            {
                string candidateCount = JsonNumberValue(member, "candidate_count");
                if (candidateCount.Length == 0) candidateCount = JsonNumberValue(member, "candidateCount");
                string endpoint = JsonStringValue(member, "preferred_endpoint");
                if (endpoint.Length == 0) endpoint = JsonStringValue(member, "preferredEndpoint");
                if (candidateCount.Length == 0 && endpoint.Length == 0)
                {
                    return "";
                }
                return SafePeerId(peer) + "|" + virtualIp + "|" + candidateCount + "|" + endpoint;
            }
            search = end + 1;
        }
        return "";
    }

    string RemotePeerSpecFromCoordinationView(string json)
    {
        string members = JsonArrayValue(json, "members");
        if (members.Length == 0) members = JsonArrayValue(json, "peers");
        if (members.Length == 0) return "";
        string localPeer = RuntimePeerId();
        int search = 0;
        while (search < members.Length)
        {
            int start = members.IndexOf('{', search);
            if (start < 0) break;
            int end = MatchingJsonBrace(members, start);
            if (end < 0) break;
            string member = members.Substring(start, end - start + 1);
            string peer = JsonStringValue(member, "peer_id");
            if (peer.Length == 0) peer = JsonStringValue(member, "peerId");
            string virtualIp = JsonStringValue(member, "virtual_ip");
            if (virtualIp.Length == 0) virtualIp = JsonStringValue(member, "virtualIp");
            string status = JsonStringValue(member, "status");
            if (peer.Length > 0
                && peer != localPeer
                && virtualIp.Length > 0
                && LooksLikeIpv4(virtualIp)
                && (status.Length == 0 || status == "online"))
            {
                return SafePeerId(peer) + "," + virtualIp;
            }
            search = end + 1;
        }
        return "";
    }

    string TailText(string value, int maxLines)
    {
        if (value.Length == 0) return "";
        string[] lines = value.Replace("\r\n", "\n").Replace('\r', '\n').Split('\n');
        int start = Math.Max(0, lines.Length - maxLines);
        StringBuilder builder = new StringBuilder();
        for (int i = start; i < lines.Length; i++)
        {
            if (lines[i].Length == 0 && i == lines.Length - 1) continue;
            if (builder.Length > 0) builder.AppendLine();
            builder.Append(lines[i]);
        }
        return builder.ToString();
    }

    string PingArgs()
    {
        string target = pingTarget.Text.Trim();
        if (target.Length == 0) return "";
        return " --ping-test " + Quote(target) + " --expected-peers 1";
    }

    string FirstPortText(string fallback)
    {
        string[] parts = ports.Text.Split(',');
        for (int i = 0; i < parts.Length; i++)
        {
            int port;
            if (Int32.TryParse(parts[i].Trim(), out port) && port > 0 && port <= 65535)
            {
                return port.ToString();
            }
        }
        return fallback;
    }

    string RunCli(string arguments)
    {
        string exe = Path.Combine(AppDomain.CurrentDomain.BaseDirectory, "LocalAreaInterconnection.Cli.exe");
        if (!File.Exists(exe))
        {
            output.Text = T("missingCli") + exe;
            return output.Text;
        }
        return RunExecutable(exe, arguments);
    }

    string RunNativeCli(string arguments)
    {
        string exe = Path.Combine(AppDomain.CurrentDomain.BaseDirectory, "LocalAreaInterconnection.Native.Cli.exe");
        if (!File.Exists(exe))
        {
            output.Text = T("missingNativeCli") + exe;
            return output.Text;
        }
        return RunExecutable(exe, arguments);
    }

    string RunNativeCliCapture(string arguments)
    {
        return RunNativeCliCapture(arguments, 5000);
    }

    string RunNativeCliCapture(string arguments, int timeoutMs)
    {
        string exe = Path.Combine(AppDomain.CurrentDomain.BaseDirectory, "LocalAreaInterconnection.Native.Cli.exe");
        if (!File.Exists(exe))
        {
            return T("missingNativeCli") + exe;
        }
        return RunExecutableCapture(exe, arguments, timeoutMs);
    }

    Process StartNativeRuntimeProcess(string arguments)
    {
        if (arguments.IndexOf("--wintun-runtime true", StringComparison.OrdinalIgnoreCase) >= 0
            || arguments.IndexOf("--packet-io-backend wintun", StringComparison.OrdinalIgnoreCase) >= 0)
        {
            return StartNativeRuntimeProcessElevated(arguments);
        }
        return StartNativeBackgroundProcess(arguments, runtimeOutput, T("runtimeExited"));
    }

    Process StartNativeRuntimeProcessElevated(string arguments)
    {
        string exe = Path.Combine(AppDomain.CurrentDomain.BaseDirectory, "LocalAreaInterconnection.Native.Cli.exe");
        if (!File.Exists(exe))
        {
            output.Text = T("missingNativeCli") + exe;
            return null;
        }

        string stamp = DateTime.UtcNow.ToString("yyyyMMddHHmmssfff");
        string prefix = SafePeerId("runtime-" + RuntimeFilePrefix());
        string scriptPath = Path.Combine(LogDirectory(), "elevated-" + prefix + "-" + stamp + ".cmd");
        string stdoutPath = Path.Combine(LogDirectory(), "elevated-" + prefix + "-" + stamp + ".out.txt");
        string stderrPath = Path.Combine(LogDirectory(), "elevated-" + prefix + "-" + stamp + ".err.txt");
        string exitPath = Path.Combine(LogDirectory(), "elevated-" + prefix + "-" + stamp + ".exit.txt");

        StringBuilder script = new StringBuilder();
        script.AppendLine("@echo off");
        script.AppendLine("chcp 65001 > nul");
        script.AppendLine("cd /d " + QuoteBatch(AppDomain.CurrentDomain.BaseDirectory));
        script.Append(QuoteBatch(exe)).Append(" ").Append(arguments)
            .Append(" 1> ").Append(QuoteBatch(stdoutPath))
            .Append(" 2> ").Append(QuoteBatch(stderrPath)).AppendLine();
        script.AppendLine("> " + QuoteBatch(exitPath) + " echo %ERRORLEVEL%");
        script.AppendLine("exit /b %ERRORLEVEL%");
        File.WriteAllText(scriptPath, script.ToString(), new UTF8Encoding(false));

        ProcessStartInfo start = new ProcessStartInfo();
        start.FileName = scriptPath;
        start.UseShellExecute = true;
        start.Verb = "runas";
        start.WindowStyle = ProcessWindowStyle.Hidden;
        start.WorkingDirectory = AppDomain.CurrentDomain.BaseDirectory;

        try
        {
            Process process = new Process();
            process.StartInfo = start;
            process.EnableRaisingEvents = true;
            process.Exited += delegate
            {
                if (IsDisposed || !IsHandleCreated) return;
                try
                {
                    BeginInvoke((MethodInvoker)delegate
                    {
                        if (output != null && !IsDisposed)
                        {
                            string stdout = ReadOptionalText(stdoutPath).Trim();
                            string stderr = ReadOptionalText(stderrPath).Trim();
                            string exitCode = ReadOptionalText(exitPath).Trim();
                            output.Text = T("runtimeExited")
                                + Environment.NewLine
                                + T("adminActionFinished") + " " + (exitCode.Length == 0 ? T("stateUnknown") : exitCode)
                                + Environment.NewLine
                                + T("runtimeSnapshotReady") + latestRuntimeSnapshot;
                            if (stdout.Length > 0)
                            {
                                output.Text += Environment.NewLine + Environment.NewLine + T("adminActionStdout") + Environment.NewLine + stdout;
                            }
                            if (stderr.Length > 0)
                            {
                                output.Text += Environment.NewLine + Environment.NewLine + T("adminActionStderr") + Environment.NewLine + stderr;
                            }
                        }
                    });
                }
                catch
                {
                }
            };
            process.Start();
            runtimeOutput.AppendLine(T("runtimeStartedElevated"));
            runtimeOutput.AppendLine(T("runtimeStdoutPath") + stdoutPath);
            runtimeOutput.AppendLine(T("runtimeStderrPath") + stderrPath);
            RegisterBackgroundProcess(process);
            return process;
        }
        catch (Exception ex)
        {
            output.Text = T("adminActionCancelled") + Environment.NewLine + ex.Message;
            return null;
        }
    }

    Process StartNativeBackgroundProcess(string arguments, StringBuilder log, string exitedPrefix)
    {
        string exe = Path.Combine(AppDomain.CurrentDomain.BaseDirectory, "LocalAreaInterconnection.Native.Cli.exe");
        if (!File.Exists(exe))
        {
            output.Text = T("missingNativeCli") + exe;
            return null;
        }

        ProcessStartInfo start = new ProcessStartInfo();
        start.FileName = exe;
        start.Arguments = arguments;
        start.UseShellExecute = false;
        start.RedirectStandardOutput = true;
        start.RedirectStandardError = true;
        start.StandardOutputEncoding = Encoding.UTF8;
        start.StandardErrorEncoding = Encoding.UTF8;
        start.CreateNoWindow = true;

        Process process = new Process();
        process.StartInfo = start;
        process.EnableRaisingEvents = true;
        process.OutputDataReceived += delegate(object sender, DataReceivedEventArgs e)
        {
            if (e.Data != null) log.AppendLine(e.Data);
        };
        process.ErrorDataReceived += delegate(object sender, DataReceivedEventArgs e)
        {
            if (e.Data != null) log.AppendLine(e.Data);
        };
        process.Exited += delegate
        {
            if (IsDisposed || !IsHandleCreated) return;
            try
            {
                BeginInvoke((MethodInvoker)delegate
                {
                    if (output != null && !IsDisposed)
                    {
                        output.Text = exitedPrefix + Environment.NewLine + log.ToString();
                    }
                });
            }
            catch
            {
            }
        };
        process.Start();
        RegisterBackgroundProcess(process);
        process.BeginOutputReadLine();
        process.BeginErrorReadLine();
        return process;
    }

    void RegisterBackgroundProcess(Process process)
    {
        lock (backgroundProcessLock)
        {
            backgroundProcesses.Add(process);
        }
    }

    void UnregisterBackgroundProcess(Process process)
    {
        lock (backgroundProcessLock)
        {
            backgroundProcesses.Remove(process);
        }
    }

    void StopRuntimeProcess(int waitMs)
    {
        if (runtimeProcess == null) return;
        Process process = runtimeProcess;
        try
        {
            if (!process.HasExited)
            {
                if (runtimeStopFile.Length > 0 && !File.Exists(runtimeStopFile))
                {
                    File.WriteAllText(runtimeStopFile, "stop");
                }
                if (!process.WaitForExit(waitMs))
                {
                    KillProcessTree(process, 2000);
                }
            }
            UnregisterBackgroundProcess(process);
            process.Dispose();
            runtimeProcess = null;
        }
        catch
        {
            KillProcessTree(process, 1000);
            UnregisterBackgroundProcess(process);
            runtimeProcess = null;
        }
    }

    string RuntimeStatusText()
    {
        if (runtimeProcess != null && !runtimeProcess.HasExited)
        {
            return T("runtimeRunning");
        }
        return T("runtimeStoppedState");
    }

    void StopCoordinationProcess(int waitMs)
    {
        if (coordinationProcess == null) return;
        Process process = coordinationProcess;
        try
        {
            if (!process.HasExited)
            {
                KillProcessTree(process, waitMs);
            }
            UnregisterBackgroundProcess(process);
            process.Dispose();
            coordinationProcess = null;
        }
        catch
        {
            KillProcessTree(process, 1000);
            UnregisterBackgroundProcess(process);
            coordinationProcess = null;
        }
    }

    void ShutdownChildProcesses()
    {
        try
        {
            if (runtimeStopFile.Length > 0 && !File.Exists(runtimeStopFile))
            {
                File.WriteAllText(runtimeStopFile, "stop");
            }
        }
        catch
        {
        }

        StopRuntimeProcess(3000);
        StopCoordinationProcess(2000);

        Process[] remaining;
        lock (backgroundProcessLock)
        {
            remaining = backgroundProcesses.ToArray();
            backgroundProcesses.Clear();
        }
        for (int i = 0; i < remaining.Length; i++)
        {
            KillProcessTree(remaining[i], 1500);
            try
            {
                remaining[i].Dispose();
            }
            catch
            {
            }
        }
    }

    void KillProcessTree(Process process, int waitMs)
    {
        if (process == null) return;
        try
        {
            if (process.HasExited) return;
            ProcessStartInfo start = new ProcessStartInfo();
            start.FileName = "taskkill.exe";
            start.Arguments = "/PID " + process.Id.ToString(CultureInfo.InvariantCulture) + " /T /F";
            start.UseShellExecute = false;
            start.CreateNoWindow = true;
            using (Process killer = Process.Start(start))
            {
                if (killer != null) killer.WaitForExit(Math.Max(500, waitMs));
            }
            if (!process.HasExited)
            {
                process.Kill();
            }
            process.WaitForExit(Math.Max(500, waitMs));
        }
        catch
        {
            try
            {
                if (!process.HasExited)
                {
                    process.Kill();
                    process.WaitForExit(Math.Max(500, waitMs));
                }
            }
            catch
            {
            }
        }
    }

    string CoordinationStatusText()
    {
        if (coordinationProcess != null && !coordinationProcess.HasExited)
        {
            return T("coordinationRunning");
        }
        return T("coordinationStoppedState");
    }

    string RunExecutable(string exe, string arguments)
    {
        string text = RunExecutableCapture(exe, arguments);
        output.Text = text;
        return text;
    }

    string RunExecutableCapture(string exe, string arguments)
    {
        return RunExecutableCapture(exe, arguments, 5000);
    }

    string RunExecutableCapture(string exe, string arguments, int timeoutMs)
    {
        ProcessStartInfo start = new ProcessStartInfo();
        start.FileName = exe;
        start.Arguments = arguments;
        start.UseShellExecute = false;
        start.RedirectStandardOutput = true;
        start.RedirectStandardError = true;
        start.StandardOutputEncoding = Encoding.UTF8;
        start.StandardErrorEncoding = Encoding.UTF8;
        start.CreateNoWindow = true;

        using (Process process = new Process())
        {
            StringBuilder stdout = new StringBuilder();
            StringBuilder stderr = new StringBuilder();
            process.StartInfo = start;
            process.OutputDataReceived += delegate(object sender, DataReceivedEventArgs e)
            {
                if (e.Data != null) stdout.AppendLine(e.Data);
            };
            process.ErrorDataReceived += delegate(object sender, DataReceivedEventArgs e)
            {
                if (e.Data != null) stderr.AppendLine(e.Data);
            };
            process.Start();
            process.BeginOutputReadLine();
            process.BeginErrorReadLine();
            if (!process.WaitForExit(timeoutMs))
            {
                try
                {
                    process.Kill();
                }
                catch
                {
                }
                return T("commandTimedOut");
            }
            process.WaitForExit();
            string text = stdout.ToString();
            if (stderr.Length > 0)
            {
                text += Environment.NewLine + stderr.ToString();
            }
            return text;
        }
    }
}

