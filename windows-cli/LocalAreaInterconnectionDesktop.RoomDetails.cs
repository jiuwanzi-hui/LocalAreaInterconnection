using System;
using System.Collections.Generic;
using System.Globalization;
using System.Diagnostics;
using System.Drawing;
using System.Drawing.Drawing2D;
using System.IO;
using System.Runtime.InteropServices;
using System.Text;
using System.Threading.Tasks;
using System.Windows.Forms;

public partial class LocalAreaInterconnectionDesktop
{
    void UpdateRoomDetails(string mode)
    {
        if (roomSummary == null) return;
        heartbeatPulseActive = false;
        roomSummary.Text = T("detailRoom") + " " + SafeText(roomName.Text) + " | " + T("detailSubnet") + " " + SafeText(subnet.Text);
        connectionSummary.Text = T("detailConnection") + " " + ConnectionText(mode);
        broadcastSummary.Text = T("detailBroadcast") + " " + T("stateUnknown");
        memberSummary.Text = T("detailMembers") + " " + MemberText(mode);
        nextActionSummary.Text = T("detailNext") + " " + NextActionText(mode);
    }

    void UpdateRoomDetailsFromNetworkReport(string json)
    {
        if (roomSummary == null) return;
        string adapter = JsonStringValue(json, "virtual_adapter");
        string tunnel = JsonStringValue(json, "tunnel");
        string p2p = JsonStringValue(json, "p2p");
        string path = JsonCheckStatus(json, "connection-path");
        string broadcast = JsonStringValue(json, "broadcast");
        string gameTraffic = JsonStringValue(json, "game_traffic");
        if (adapter.Length == 0) adapter = T("stateUnknown");
        if (tunnel.Length == 0) tunnel = T("stateUnknown");
        if (p2p.Length == 0) p2p = T("stateUnknown");
        if (path.Length == 0 || path == "skipped") path = T("stateUnknown");
        if (broadcast.Length == 0) broadcast = T("stateUnknown");
        if (gameTraffic.Length == 0) gameTraffic = T("stateUnknown");

        roomSummary.Text = T("detailRoom") + " " + SafeText(roomName.Text) + " | " + T("detailSubnet") + " " + SafeText(subnet.Text);
        connectionSummary.Text = T("detailConnection") + " " + T("detailAdapter") + "=" + adapter
            + ", " + T("detailTunnel") + "=" + tunnel
            + ", P2P=" + p2p
            + ", " + T("detailPath") + "=" + path;
        broadcastSummary.Text = T("detailBroadcast") + " " + broadcast + " | " + T("detailGameTraffic") + " " + gameTraffic;
        memberSummary.Text = T("detailMembers") + " " + SafeText(hostName.Text) + " @ " + SafeText(ip.Text);
        nextActionSummary.Text = T("detailNext") + " " + DiagnosticNextAction(adapter, tunnel, p2p, broadcast, gameTraffic);
    }

    void RefreshRuntimeStatus()
    {
        RefreshRuntimeLogTail();
        RefreshCoordinationPresence();
        if (latestRuntimeSnapshot.Length == 0 || !File.Exists(latestRuntimeSnapshot))
        {
            RefreshCoordinationRoomView(false);
            return;
        }
        string text;
        try
        {
            text = File.ReadAllText(latestRuntimeSnapshot);
        }
        catch
        {
            return;
        }
        if (text.Length == 0 || text == lastRuntimeSnapshotText)
        {
            return;
        }
        lastRuntimeSnapshotText = text;
        UpdateRoomDetailsFromRuntimeSnapshot(text);
    }

    void UpdateRoomDetailsFromRuntimeSnapshot(string json)
    {
        if (roomSummary == null) return;
        string adapter = JsonStringValue(json, "virtual_adapter");
        string tunnel = JsonStringValue(json, "tunnel");
        string p2p = JsonStringValue(json, "p2p");
        string path = JsonCheckStatus(json, "connection-path");
        if (path.Length == 0) path = JsonStringValue(JsonObjectValue(json, "tunnelServiceSnapshot"), "connection_path");
        string broadcast = JsonStringValue(json, "broadcast");
        string gameTraffic = JsonStringValue(json, "game_traffic");
        string connectedPeers = JsonNumberValue(json, "connected_peer_count");
        string heartbeatPackets = JsonNumberValue(json, "heartbeatPacketsSent");
        string snapshotWrites = JsonNumberValue(json, "snapshotWriteCount");
        string runtimePeers = RuntimeCompactPeersText(json);
        string linkState = RuntimeLinkStateText(json);
        string primaryPath = RuntimePrimaryPathText(json);
        string runtimeMetrics = RuntimeMetricsText(json);
        string packetCounters = RuntimePacketCountersText(json);
        heartbeatPulseActive = RuntimeHasConnectedPeer(json);
        if (adapter.Length == 0) adapter = T("stateUnknown");
        if (tunnel.Length == 0) tunnel = T("stateUnknown");
        if (p2p.Length == 0) p2p = T("stateUnknown");
        if (primaryPath.Length > 0) path = primaryPath;
        if (path.Length == 0 || path == "skipped") path = T("stateUnknown");
        if (broadcast.Length == 0) broadcast = T("stateUnknown");
        if (gameTraffic.Length == 0) gameTraffic = T("stateUnknown");
        if (connectedPeers.Length == 0) connectedPeers = "0";
        if (heartbeatPackets.Length == 0) heartbeatPackets = "0";
        if (snapshotWrites.Length == 0) snapshotWrites = "0";

        roomSummary.Text = T("detailRoom") + " " + SafeText(roomName.Text) + " | " + T("detailSubnet") + " " + SafeText(subnet.Text);
        connectionSummary.Text = T("detailConnection") + " " + linkState
            + (path.Length > 0 && path != T("stateUnknown") ? " | " + path : "");
        broadcastSummary.Text = runtimeMetrics.Length > 0
            ? runtimeMetrics + (packetCounters.Length > 0 ? " | " + packetCounters : "")
            : T("runtimeMetricEmpty");
        memberSummary.Text = T("detailMembers") + Environment.NewLine
            + (runtimePeers.Length > 0 ? runtimePeers : SafeText(hostName.Text) + " @ " + SafeText(ip.Text));
        nextActionSummary.Text = T("detailNext") + " "
            + RuntimeNextActionText(json, linkState, adapter, tunnel, p2p, broadcast, gameTraffic);
    }

    void UpdateRoomDetailsFromRuntimeCleanupPlan(string json)
    {
        if (roomSummary == null || json.Trim().Length == 0) return;
        heartbeatPulseActive = false;
        string backend = JsonStringValue(json, "packet_io_backend");
        bool restoreAdapter = JsonBoolValue(json, "restore_adapter");
        bool requiresElevation = JsonBoolValue(json, "requires_elevation");
        int commandCount = JsonObjectCount(JsonArrayValue(json, "commands"));
        int stepCount = JsonObjectCount(JsonArrayValue(json, "process_cleanup_steps"));
        if (backend.Length == 0) backend = T("stateUnknown");

        roomSummary.Text = T("detailRoom") + " " + RuntimeRoomId() + " | " + T("detailSubnet") + " " + SafeText(subnet.Text);
        connectionSummary.Text = T("detailConnection") + " " + T("runtimeCleanup") + ": " + backend
            + ", " + T("runtimeCleanupSteps") + "=" + stepCount.ToString();
        broadcastSummary.Text = T("runtimeCleanupRestore") + " " + (restoreAdapter ? T("stateYes") : T("stateNo"))
            + ", " + T("runtimeCleanupCommands") + "=" + commandCount.ToString();
        memberSummary.Text = T("detailMembers") + " " + SafeText(hostName.Text) + " @ " + SafeText(ip.Text);
        nextActionSummary.Text = T("detailNext") + " "
            + (requiresElevation ? T("runtimeCleanupNeedsAdmin") : T("runtimeCleanupNoAdmin"));
    }

    void UpdateRoomDetailsFromRuntimeCleanupReport(string json)
    {
        if (roomSummary == null || json.Trim().Length == 0) return;
        string status = JsonStringValue(json, "status");
        string backend = JsonStringValue(json, "packet_io_backend");
        int checkCount = JsonObjectCount(JsonArrayValue(json, "checks"));
        int actionCount = JsonStringArrayCount(JsonArrayValue(json, "next_actions"));
        bool requiresElevation = JsonBoolValue(json, "requires_elevation");
        if (status.Length == 0) status = T("stateUnknown");
        if (backend.Length == 0) backend = T("stateUnknown");

        roomSummary.Text = T("detailRoom") + " " + RuntimeRoomId() + " | " + T("detailSubnet") + " " + SafeText(subnet.Text);
        connectionSummary.Text = T("detailConnection") + " " + T("runtimeCleanup") + ": " + status
            + ", backend=" + backend;
        broadcastSummary.Text = T("runtimeCleanupChecks") + "=" + checkCount.ToString()
            + ", " + T("runtimeCleanupActions") + "=" + actionCount.ToString();
        memberSummary.Text = T("detailMembers") + " " + SafeText(hostName.Text) + " @ " + SafeText(ip.Text);
        nextActionSummary.Text = T("detailNext") + " "
            + (requiresElevation ? T("runtimeCleanupNeedsAdmin") : T("runtimeCleanupNoAdmin"));
    }

    void UpdateRoomDetailsFromRuntimeCleanupApply(string json)
    {
        if (roomSummary == null || json.Trim().Length == 0) return;
        string status = JsonStringValue(json, "status");
        string nextAction = JsonStringValue(json, "nextAction");
        int commandCount = JsonObjectCount(JsonArrayValue(json, "commandResults"));
        int unsafeCount = JsonStringArrayCount(JsonArrayValue(json, "unsafeCommands"));
        bool confirmed = JsonBoolValue(json, "confirmed");
        bool requiresElevation = JsonBoolValue(json, "requires_elevation");
        if (status.Length == 0) status = T("stateUnknown");
        if (nextAction.Length == 0)
        {
            nextAction = requiresElevation ? T("runtimeCleanupNeedsAdmin") : T("runtimeCleanupNoAdmin");
        }

        roomSummary.Text = T("detailRoom") + " " + RuntimeRoomId() + " | " + T("detailSubnet") + " " + SafeText(subnet.Text);
        connectionSummary.Text = T("detailConnection") + " " + T("runtimeCleanupApply") + ": " + status
            + ", " + T("runtimeCleanupConfirmed") + "=" + (confirmed ? T("stateYes") : T("stateNo"));
        broadcastSummary.Text = T("runtimeCleanupCommands") + "=" + commandCount.ToString()
            + ", " + T("runtimeCleanupUnsafe") + "=" + unsafeCount.ToString();
        memberSummary.Text = T("detailMembers") + " " + SafeText(hostName.Text) + " @ " + SafeText(ip.Text);
        nextActionSummary.Text = T("detailNext") + " " + nextAction;
    }

    void UpdateRoomDetailsFromRouteScan(string json)
    {
        if (roomSummary == null || json.Trim().Length == 0) return;
        string status = JsonStringValue(json, "status");
        string routeCount = JsonNumberValue(json, "routeCount");
        string roomRouteCount = JsonNumberValue(json, "roomRouteCount");
        string nextAction = JsonStringValue(json, "nextAction");
        if (status.Length == 0) status = T("stateUnknown");
        if (routeCount.Length == 0) routeCount = "0";
        if (roomRouteCount.Length == 0) roomRouteCount = "0";
        if (nextAction.Length == 0) nextAction = T("runtimeRouteNoAction");

        roomSummary.Text = T("detailRoom") + " " + RuntimeRoomId() + " | " + T("detailSubnet") + " " + SafeText(subnet.Text);
        connectionSummary.Text = T("detailConnection") + " " + T("runtimeRouteScan") + ": " + status;
        broadcastSummary.Text = T("runtimeRouteCount") + "=" + routeCount
            + ", " + T("runtimeRoomRouteCount") + "=" + roomRouteCount;
        memberSummary.Text = T("detailMembers") + " " + SafeText(hostName.Text) + " @ " + SafeText(ip.Text);
        nextActionSummary.Text = T("detailNext") + " " + nextAction;
    }

    void UpdateRoomDetailsFromWintunReport(string json)
    {
        if (roomSummary == null || json.Trim().Length == 0) return;
        string status = JsonStringValue(json, "status");
        string dllPath = JsonStringValue(json, "dll_path");
        string adapterName = JsonStringValue(json, "adapter_name");
        string error = JsonStringValue(json, "error");
        string nextAction = JsonFirstStringInArray(JsonArrayValue(json, "next_actions"));
        if (nextAction.Length == 0) nextAction = JsonStringValue(json, "next_action");
        if (status.Length == 0) status = T("stateUnknown");
        if (dllPath.Length == 0) dllPath = T("stateUnknown");
        if (adapterName.Length == 0) adapterName = "LocalAreaInterconnection";
        if (nextAction.Length == 0)
        {
            nextAction = error.Length > 0 ? error : T("wintunReadyNext");
        }

        roomSummary.Text = T("detailRoom") + " " + RuntimeRoomId() + " | " + T("detailSubnet") + " " + SafeText(subnet.Text);
        connectionSummary.Text = T("detailConnection") + " " + T("wintunStatus") + "=" + status
            + ", " + T("detailAdapter") + "=" + adapterName;
        broadcastSummary.Text = "wintun.dll " + dllPath;
        memberSummary.Text = error.Length > 0 ? error : SafeText(hostName.Text) + " @ " + SafeText(ip.Text);
        nextActionSummary.Text = T("detailNext") + " " + nextAction;
    }

    void UpdateRoomDetailsFromCoordinationView(string json)
    {
        if (roomSummary == null || json.Trim().Length == 0) return;
        if (runtimeProcess == null || runtimeProcess.HasExited)
        {
            heartbeatPulseActive = false;
        }
        string status = JsonStringValue(json, "status");
        string memberCount = JsonNumberValue(json, "member_count");
        if (memberCount.Length == 0) memberCount = JsonNumberValue(json, "memberCount");
        string onlineCount = JsonNumberValue(json, "online_count");
        if (onlineCount.Length == 0) onlineCount = JsonNumberValue(json, "onlineCount");
        string expiredCount = JsonNumberValue(json, "expired_count");
        if (expiredCount.Length == 0) expiredCount = JsonNumberValue(json, "expiredCount");
        string nextAction = JsonStringValue(json, "next_action");
        if (nextAction.Length == 0) nextAction = JsonStringValue(json, "nextAction");
        string members = CoordinationMembersText(json);
        latestCoordinationViewText = json;
        if (status.Length == 0) status = T("stateUnknown");
        if (memberCount.Length == 0 && JsonArrayValue(json, "peers").Length > 0)
        {
            memberCount = JsonObjectArrayCount(json, "peers").ToString(CultureInfo.InvariantCulture);
        }
        if (onlineCount.Length == 0 && JsonArrayValue(json, "peers").Length > 0)
        {
            onlineCount = JsonObjectArrayStatusCount(json, "peers", "online").ToString(CultureInfo.InvariantCulture);
        }
        if (expiredCount.Length == 0 && JsonArrayValue(json, "peers").Length > 0)
        {
            expiredCount = JsonObjectArrayStatusCount(json, "peers", "expired").ToString(CultureInfo.InvariantCulture);
        }
        if (memberCount.Length == 0) memberCount = "0";
        if (onlineCount.Length == 0) onlineCount = "0";
        if (expiredCount.Length == 0) expiredCount = "0";
        Int32.TryParse(memberCount, NumberStyles.Integer, CultureInfo.InvariantCulture, out latestCoordinationMemberCount);
        Int32.TryParse(onlineCount, NumberStyles.Integer, CultureInfo.InvariantCulture, out latestCoordinationOnlineCount);
        if (nextAction.Length == 0) nextAction = NextActionText("joined");
        if (members.Length == 0) members = MemberText("joined");

        if (runtimeProcess != null && !runtimeProcess.HasExited && latestRuntimeSnapshot.Length > 0 && File.Exists(latestRuntimeSnapshot))
        {
            return;
        }

        roomSummary.Text = T("detailRoom") + " " + RuntimeRoomId() + " | " + T("detailSubnet") + " " + SafeText(subnet.Text);
        connectionSummary.Text = T("detailConnection") + " " + T("coordinationRoomStatus") + "=" + status
            + ", " + T("coordinationOnline") + "=" + onlineCount + "/" + memberCount
            + ", " + T("coordinationExpired") + "=" + expiredCount;
        memberSummary.Text = T("detailMembers") + Environment.NewLine + members;
        nextActionSummary.Text = T("detailNext") + " " + nextAction;
    }

    void UpdateRoomDetailsFromRelayPlan(string json)
    {
        if (roomSummary == null || json.Trim().Length == 0) return;
        string status = JsonStringValue(json, "status");
        string localPeer = JsonStringValue(json, "local_peer_id");
        string remote = JsonStringValue(json, "remote_peer_id");
        string p2pCount = JsonNumberValue(json, "p2p_candidate_count");
        string relayCount = JsonNumberValue(json, "relay_candidate_count");
        string relayEndpoint = JsonFirstStringInArray(JsonArrayValue(json, "selected_relay_endpoints"));
        string nextAction = JsonFirstStringInArray(JsonArrayValue(json, "recommended_actions"));
        if (status.Length == 0) status = T("stateUnknown");
        if (localPeer.Length == 0) localPeer = RuntimePeerId();
        if (remote.Length == 0) remote = RemotePeerIdForKick();
        if (p2pCount.Length == 0) p2pCount = "0";
        if (relayCount.Length == 0) relayCount = "0";
        if (relayEndpoint.Length == 0) relayEndpoint = T("stateUnknown");
        if (nextAction.Length == 0) nextAction = T("nextFixTunnel");

        roomSummary.Text = T("detailRoom") + " " + RuntimeRoomId() + " | " + T("detailSubnet") + " " + SafeText(subnet.Text);
        connectionSummary.Text = T("detailConnection") + " " + T("detailPath") + "=" + status
            + ", P2P=" + p2pCount
            + ", " + T("detailRelay") + "=" + relayCount;
        broadcastSummary.Text = T("relaySelected") + " " + relayEndpoint;
        memberSummary.Text = T("detailMembers") + " " + localPeer + " -> " + SafeText(remote);
        nextActionSummary.Text = T("detailNext") + " " + nextAction;
    }

    void UpdateRoomDetailsFromConnectionPathPlan(string json)
    {
        if (roomSummary == null || json.Trim().Length == 0) return;
        string status = JsonStringValue(json, "status");
        string selectedPath = JsonStringValue(json, "selected_path");
        string localNat = JsonStringValue(json, "local_nat_assessment");
        string remoteNat = JsonStringValue(json, "remote_nat_assessment");
        string localHost = JsonNumberValue(json, "local_host_candidate_count");
        string localSrflx = JsonNumberValue(json, "local_srflx_candidate_count");
        string remoteHost = JsonNumberValue(json, "remote_host_candidate_count");
        string remoteSrflx = JsonNumberValue(json, "remote_srflx_candidate_count");
        string endpoint = JsonFirstStringInArray(JsonArrayValue(json, "selected_endpoints"));
        string nextAction = JsonFirstStringInArray(JsonArrayValue(json, "recommended_actions"));
        if (status.Length == 0) status = T("stateUnknown");
        if (selectedPath.Length == 0) selectedPath = T("stateUnknown");
        if (localNat.Length == 0) localNat = T("stateUnknown");
        if (remoteNat.Length == 0) remoteNat = T("stateUnknown");
        if (localHost.Length == 0) localHost = "0";
        if (localSrflx.Length == 0) localSrflx = "0";
        if (remoteHost.Length == 0) remoteHost = "0";
        if (remoteSrflx.Length == 0) remoteSrflx = "0";
        if (endpoint.Length == 0) endpoint = T("stateUnknown");
        if (nextAction.Length == 0) nextAction = T("nextFixTunnel");

        roomSummary.Text = T("detailRoom") + " " + RuntimeRoomId() + " | " + T("detailSubnet") + " " + SafeText(subnet.Text);
        connectionSummary.Text = T("detailConnection") + " " + T("connectionPathPlan") + "=" + status
            + ", " + T("detailPath") + "=" + selectedPath;
        broadcastSummary.Text = "NAT local=" + localNat + " (host=" + localHost + ", srflx=" + localSrflx + ")"
            + ", remote=" + remoteNat + " (host=" + remoteHost + ", srflx=" + remoteSrflx + ")";
        memberSummary.Text = T("relaySelected") + " " + endpoint;
        nextActionSummary.Text = T("detailNext") + " " + nextAction;
    }

    void UpdateFromGameProfilePlan(string json)
    {
        if (json.Trim().Length == 0) return;
        string profile = JsonObjectValue(json, "profile");
        string plan = JsonObjectValue(json, "plan");
        string profileName = JsonStringValue(profile, "game_name");
        string discovery = JsonStringValue(profile, "discovery");
        string compatibility = JsonStringValue(profile, "compatibility");
        string profilePorts = JsonPortArrayCsv(JsonArrayValue(profile, "ports"));
        string joinInstruction = JsonStringValue(plan, "join_instruction");
        string broadcast = JsonObjectValue(plan, "broadcast");
        string broadcastExpectation = JsonStringValue(broadcast, "expectation");

        if (profileName.Length > 0)
        {
            gameName.Text = profileName;
        }
        if (profilePorts.Length > 0)
        {
            ports.Text = profilePorts;
        }
        if (joinInstruction.Length == 0)
        {
            joinInstruction = NextActionText("created");
        }
        if (broadcastExpectation.Length == 0)
        {
            broadcastExpectation = T("stateUnknown");
        }
        if (discovery.Length == 0)
        {
            discovery = T("stateUnknown");
        }
        if (compatibility.Length == 0)
        {
            compatibility = T("stateUnknown");
        }

        UpdateRoomDetailsFromGameProfilePlan(profileName, discovery, compatibility, profilePorts, broadcastExpectation, joinInstruction);
    }

    void UpdateFromGameProfileList(string json)
    {
        if (json.Trim().Length == 0) return;
        string profiles = JsonArrayValue(json, "profiles");
        string totalCount = JsonNumberValue(json, "total_count");
        string matchedCount = JsonNumberValue(json, "matched_count");
        if (profiles.Length == 0 && totalCount.Length == 0 && matchedCount.Length == 0)
        {
            return;
        }
        string firstProfile = FirstJsonObject(profiles);
        string profileName = JsonStringValue(firstProfile, "game_name");
        string discovery = JsonStringValue(firstProfile, "discovery");
        string compatibility = JsonStringValue(firstProfile, "compatibility");
        string profilePorts = JsonPortArrayCsv(JsonArrayValue(firstProfile, "ports"));
        string steamAppId = JsonStringValue(firstProfile, "steam_app_id");
        if (profileName.Length > 0)
        {
            gameName.Text = profileName;
        }
        if (profilePorts.Length > 0)
        {
            ports.Text = profilePorts;
        }
        UpdateRoomDetailsFromGameProfileList(profileName, steamAppId, discovery, compatibility, profilePorts, totalCount, matchedCount);
    }

    void UpdateRoomDetailsFromGameProfileList(string profileName, string steamAppId, string discovery, string compatibility, string profilePorts, string totalCount, string matchedCount)
    {
        if (roomSummary == null) return;
        if (profileName.Length == 0) profileName = SafeText(gameName.Text);
        if (profilePorts.Length == 0) profilePorts = SafeText(ports.Text);
        if (totalCount.Length == 0) totalCount = "0";
        if (matchedCount.Length == 0) matchedCount = "0";
        if (discovery.Length == 0) discovery = T("stateUnknown");
        if (compatibility.Length == 0) compatibility = T("stateUnknown");

        roomSummary.Text = T("detailRoom") + " " + SafeText(roomName.Text) + " | " + T("detailSubnet") + " " + SafeText(subnet.Text);
        connectionSummary.Text = T("detailConnection") + " " + T("detailGameProfile") + "=" + profileName
            + ", " + T("gameProfileMatches") + "=" + matchedCount + "/" + totalCount;
        broadcastSummary.Text = T("detailBroadcast") + " " + discovery
            + " | " + T("detailCompatibility") + "=" + compatibility;
        memberSummary.Text = T("detailGamePorts") + " " + profilePorts
            + (steamAppId.Length > 0 ? Environment.NewLine + "Steam App ID " + steamAppId : "");
        nextActionSummary.Text = T("detailNext") + " " + (matchedCount == "0" ? T("gameProfileNoMatch") : T("gameProfileSelected"));
    }

    void UpdateRoomDetailsFromGameProfilePlan(string profileName, string discovery, string compatibility, string profilePorts, string broadcastExpectation, string joinInstruction)
    {
        if (roomSummary == null) return;
        if (profileName.Length == 0) profileName = SafeText(gameName.Text);
        if (profilePorts.Length == 0) profilePorts = SafeText(ports.Text);
        roomSummary.Text = T("detailRoom") + " " + SafeText(roomName.Text) + " | " + T("detailSubnet") + " " + SafeText(subnet.Text);
        connectionSummary.Text = T("detailConnection") + " " + T("detailGameProfile") + "=" + profileName
            + ", " + T("detailCompatibility") + "=" + compatibility;
        broadcastSummary.Text = T("detailBroadcast") + " " + discovery + " | " + T("detailGamePorts") + " " + profilePorts;
        memberSummary.Text = T("detailMembers") + " " + SafeText(hostName.Text) + " @ " + SafeText(ip.Text);
        nextActionSummary.Text = T("detailNext") + " " + joinInstruction + " " + broadcastExpectation;
    }

    void UpdateRoomDetailsFromGamePortScan(string json)
    {
        if (roomSummary == null || json.Trim().Length == 0) return;
        string status = JsonStringValue(json, "status");
        string endpointCount = JsonNumberValue(json, "endpointCount");
        string matchCount = JsonNumberValue(json, "matchCount");
        string nextAction = JsonStringValue(json, "nextAction");
        if (status.Length == 0) status = T("stateUnknown");
        if (endpointCount.Length == 0) endpointCount = "0";
        if (matchCount.Length == 0) matchCount = "0";
        if (nextAction.Length == 0) nextAction = T("nextStartGame");

        roomSummary.Text = T("detailRoom") + " " + SafeText(roomName.Text) + " | " + T("detailSubnet") + " " + SafeText(subnet.Text);
        connectionSummary.Text = T("detailConnection") + " " + T("gamePortScan") + ": " + status;
        broadcastSummary.Text = T("gamePortEndpoints") + "=" + endpointCount
            + ", " + T("gamePortMatches") + "=" + matchCount;
        memberSummary.Text = T("detailGamePorts") + " " + SafeText(ports.Text);
        nextActionSummary.Text = T("detailNext") + " " + nextAction;
    }

    void UpdateRoomDetailsFromGameReadiness(string json)
    {
        if (roomSummary == null || json.Trim().Length == 0) return;
        string status = JsonStringValue(json, "status");
        string matchCount = JsonNumberValue(json, "matchCount");
        string networkStatus = JsonStringValue(json, "networkStatus");
        string report = JsonObjectValue(json, "report");
        string connectionPath = JsonStringValue(JsonObjectValue(json, "connectionPathReport"), "selected_path");
        string nextAction = JsonFirstStringInArray(JsonArrayValue(report, "next_actions"));
        if (status.Length == 0) status = T("stateUnknown");
        if (matchCount.Length == 0) matchCount = "0";
        if (networkStatus.Length == 0) networkStatus = T("stateUnknown");
        if (connectionPath.Length == 0) connectionPath = T("stateUnknown");
        if (nextAction.Length == 0) nextAction = T("nextHealthy");

        roomSummary.Text = T("detailRoom") + " " + SafeText(roomName.Text) + " | " + T("detailSubnet") + " " + SafeText(subnet.Text);
        connectionSummary.Text = T("detailConnection") + " " + T("gameReadiness") + "=" + status
            + ", " + T("detailPath") + "=" + connectionPath;
        broadcastSummary.Text = T("gamePortMatches") + "=" + matchCount
            + " | " + T("networkDiagnose") + "=" + networkStatus;
        memberSummary.Text = T("detailGamePorts") + " " + SafeText(ports.Text);
        nextActionSummary.Text = T("detailNext") + " " + nextAction;
    }

    void UpdateRoomDetailsFromDiagnosticBundle(string path)
    {
        if (roomSummary == null) return;
        string json;
        try
        {
            json = File.ReadAllText(path);
        }
        catch
        {
            UpdateRoomDetails("exported");
            return;
        }

        string status = JsonStringValue(json, "status");
        string readiness = JsonStringValue(JsonObjectValue(json, "game_readiness"), "status");
        string broadcastForward = JsonStringValue(JsonObjectValue(json, "broadcast_forward"), "status");
        string connectionSection = JsonObjectValue(json, "connection_path");
        string connectionPath = JsonStringValue(JsonObjectValue(connectionSection, "report"), "selected_path");
        if (connectionPath.Length == 0) connectionPath = JsonStringValue(connectionSection, "runtime_path");
        string runtimePeerSection = JsonObjectValue(json, "runtime_peers");
        string runtimePeerCount = JsonNumberValue(runtimePeerSection, "peer_count");
        string runtimePeers = RuntimePeersText(runtimePeerSection);
        string relayFallback = RuntimeRelayFallbackText(json);
        string nextAction = JsonFirstStringInArray(JsonArrayValue(JsonObjectValue(json, "game_readiness"), "next_actions"));
        if (status.Length == 0) status = T("stateUnknown");
        if (readiness.Length == 0) readiness = T("stateUnknown");
        if (broadcastForward.Length == 0) broadcastForward = T("stateUnknown");
        if (connectionPath.Length == 0) connectionPath = T("stateUnknown");
        if (runtimePeerCount.Length == 0) runtimePeerCount = "0";
        if (nextAction.Length == 0) nextAction = T("nextShareBundle");

        roomSummary.Text = T("detailRoom") + " " + SafeText(roomName.Text) + " | " + T("detailSubnet") + " " + SafeText(subnet.Text);
        connectionSummary.Text = T("connectionExported") + " | " + T("gameReadiness") + "=" + readiness
            + ", " + T("detailPath") + "=" + connectionPath;
        broadcastSummary.Text = T("detailBroadcast") + " forward=" + broadcastForward
            + " | " + T("detailGameTraffic") + " " + status;
        memberSummary.Text = T("detailMembers") + Environment.NewLine
            + (runtimePeers.Length > 0 ? runtimePeers : T("runtimePeers") + "=" + runtimePeerCount)
            + Environment.NewLine
            + T("detailGamePorts") + " " + SafeText(ports.Text);
        if (relayFallback.Length > 0)
        {
            memberSummary.Text += Environment.NewLine + T("detailRelay") + Environment.NewLine + relayFallback;
        }
        nextActionSummary.Text = T("detailNext") + " " + nextAction;
    }

    void RefreshRuntimeLogTail()
    {
        if (runtimeProcess == null || runtimeProcess.HasExited || output == null)
        {
            return;
        }
        if (runtimeOutput.Length == lastRuntimeLogLength)
        {
            return;
        }
        lastRuntimeLogLength = runtimeOutput.Length;
        string current = output.Text ?? "";
        if (!current.StartsWith(T("runtimeStarted"), StringComparison.Ordinal)
            && !current.StartsWith(T("runtimeLogTail"), StringComparison.Ordinal))
        {
            return;
        }
        output.Text = T("runtimeLogTail") + Environment.NewLine + TailText(runtimeOutput.ToString(), 80);
    }

    string ConnectionText(string mode)
    {
        if (mode == "created") return T("connectionHostReady");
        if (mode == "joined") return T("connectionJoined");
        if (mode == "exported") return T("connectionExported");
        if (mode == "closed") return T("connectionClosed");
        return T("stateUnknown");
    }

    string MemberText(string mode)
    {
        if (mode == "joined") return SafeText(hostName.Text) + " @ " + SafeText(ip.Text) + ", " + T("detailHost") + " @ " + SafeText(pingTarget.Text);
        return SafeText(hostName.Text) + " @ " + SafeText(ip.Text);
    }

    string NextActionText(string mode)
    {
        if (mode == "created") return T("nextCreateLanRoom");
        if (mode == "joined") return T("nextFindLanRoom");
        if (mode == "decoded") return T("nextJoinRoom");
        if (mode == "exported") return T("nextShareBundle");
        if (mode == "closed") return T("nextCreateOrJoin");
        return T("nextCreateOrJoin");
    }

    string DiagnosticNextAction(string adapter, string tunnel, string p2p, string broadcast, string gameTraffic)
    {
        if (adapter != "ok") return T("nextFixAdapter");
        if (tunnel != "ok" || p2p != "ok") return T("nextFixTunnel");
        if (broadcast != "seen") return T("nextCheckBroadcast");
        if (gameTraffic != "seen") return T("nextStartGame");
        return T("nextHealthy");
    }

    string CustomerNetworkSummary(string network, string readiness)
    {
        string adapter = JsonStringValue(network, "virtual_adapter");
        string tunnel = JsonStringValue(network, "tunnel");
        string p2p = JsonStringValue(network, "p2p");
        string broadcast = JsonStringValue(network, "broadcast");
        string gameTraffic = JsonStringValue(network, "game_traffic");
        string readinessStatus = JsonStringValue(readiness, "status");
        string readinessNext = JsonFirstStringInArray(JsonArrayValue(JsonObjectValue(readiness, "report"), "next_actions"));
        string pathStatus = JsonStringValue(readiness, "status");
        string selectedPath = JsonStringValue(readiness, "selected_path");
        if (selectedPath.Length == 0) selectedPath = JsonStringValue(readiness, "selectedPath");
        string selectedEndpoint = JsonFirstStringInArray(JsonArrayValue(readiness, "selected_endpoints"));
        string pathBootstrap = JsonStringValue(readiness, "bootstrapStatus");
        string runtimePeers = RuntimePeersText(network);
        string runtimePaths = RuntimeConnectionPathText(network);
        if (pathStatus.Length == 0) pathStatus = JsonStringValue(readiness, "status");
        if (adapter.Length == 0) adapter = T("stateUnknown");
        if (tunnel.Length == 0) tunnel = T("stateUnknown");
        if (p2p.Length == 0) p2p = T("stateUnknown");
        if (broadcast.Length == 0) broadcast = T("stateUnknown");
        if (gameTraffic.Length == 0) gameTraffic = T("stateUnknown");
        if (readinessStatus.Length == 0) readinessStatus = T("stateUnknown");

        string next = DiagnosticNextAction(adapter, tunnel, p2p, broadcast, gameTraffic);
        if (readinessNext.Length > 0)
        {
            next = readinessNext;
        }

        return T("networkDiagnoseDone")
            + Environment.NewLine + T("summaryAdapter") + " " + FriendlyStatus(adapter)
            + Environment.NewLine + T("summaryTunnel") + " " + FriendlyStatus(tunnel)
            + Environment.NewLine + "P2P: " + FriendlyStatus(p2p)
            + Environment.NewLine + T("summaryBroadcast") + " " + FriendlyStatus(broadcast)
            + Environment.NewLine + T("summaryGame") + " " + FriendlyStatus(gameTraffic)
            + Environment.NewLine + T("summaryReadiness") + " " + FriendlyStatus(readinessStatus)
            + (selectedPath.Length > 0 || selectedEndpoint.Length > 0 || pathBootstrap.Length > 0
                ? Environment.NewLine + T("summaryPath") + " "
                    + (pathBootstrap.Length > 0 ? pathBootstrap + " " : "")
                    + FriendlyStatus(pathStatus)
                    + (selectedPath.Length > 0 ? " [" + selectedPath + "]" : "")
                    + (selectedEndpoint.Length > 0 ? " -> " + selectedEndpoint : "")
                : "")
            + (runtimePeers.Length > 0
                ? Environment.NewLine + T("summaryRuntimePeers") + Environment.NewLine + runtimePeers
                : "")
            + (runtimePaths.Length > 0
                ? Environment.NewLine + T("summaryRuntimePaths") + Environment.NewLine + runtimePaths
                : "")
            + Environment.NewLine
            + Environment.NewLine + T("detailNext") + " " + next;
    }

    string FriendlyStatus(string status)
    {
        if (status == "ok" || status == "seen" || status == "ready") return T("stateOk");
        if (status == "missing" || status == "missing-peers" || status == "needs-attention") return T("stateNeedsAttention");
        if (status == "skipped") return T("stateSkipped");
        if (status == "failed") return T("stateFailed");
        return status;
    }

    string SafeText(string value)
    {
        string text = value == null ? "" : value.Trim();
        return text.Length == 0 ? T("stateUnknown") : text;
    }

    string SafePeerId(string value)
    {
        string text = value == null ? "" : value.Trim();
        if (text.Length == 0) return "desktop_peer";
        char[] chars = text.ToCharArray();
        for (int i = 0; i < chars.Length; i++)
        {
            if (!Char.IsLetterOrDigit(chars[i]) && chars[i] != '_' && chars[i] != '-')
            {
                chars[i] = '_';
            }
        }
        return new string(chars);
    }
}

