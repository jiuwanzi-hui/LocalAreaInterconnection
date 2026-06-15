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
    void CreateRoom()
    {
        string args = "init --room-name " + Quote(roomName.Text)
            + " --host " + Quote(hostName.Text);
        if (coordinationServer.Text.Trim().Length > 0)
        {
            args += " --coordination-endpoint " + Quote(coordinationServer.Text.Trim());
        }
        string text = RunCli(args);
        string generatedInvite = JsonStringValue(text, "invite");
        string generatedSubnet = JsonStringValue(text, "virtualSubnet");
        string generatedHostIp = JsonStringValue(text, "hostIp");
        string generatedHostPeer = JsonStringValue(text, "hostPeerId");
        if (generatedInvite.Length > 0) invite.Text = generatedInvite;
        if (generatedSubnet.Length > 0) subnet.Text = generatedSubnet;
        if (generatedHostIp.Length > 0)
        {
            ip.Text = generatedHostIp;
            pingTarget.Text = generatedHostIp;
        }
        if (generatedHostPeer.Length > 0)
        {
            hostRuntimePeerId = SafePeerId(generatedHostPeer);
            localRuntimePeerId = hostRuntimePeerId;
        }
        UpdateRoomDetails("created");
    }

    void QuickHostRoom()
    {
        if (roomName.Text.Trim().Length == 0 || hostName.Text.Trim().Length == 0)
        {
            output.Text = T("hostNeedsName");
            return;
        }
        CreateRoom();
        if (invite.Text.Trim().Length > 0)
        {
            bool copied = TryCopyToClipboard(invite.Text.Trim());
            output.Text += Environment.NewLine
                + Environment.NewLine
                + T(copied ? "quickInviteCopied" : "quickInviteCopyFailed")
                + Environment.NewLine
                + T("quickNextHost");
        }
    }

    void QuickJoinRoom()
    {
        if (invite.Text.Trim().Length == 0)
        {
            output.Text = T("joinNeedsInvite");
            return;
        }
        DecodeInvite();
        JoinRoom();
        output.Text += Environment.NewLine
            + Environment.NewLine
            + T("quickJoinedNext");
    }

    void StartLanSession()
    {
        if (invite.Text.Trim().Length == 0)
        {
            output.Text = T("startNeedsRoom");
            return;
        }
        string directPeerId;
        string directVirtualIp;
        string directOffer;
        bool hasDirectOffer = TryParseRemotePeerOfferSpec(
            remotePeer.Text.Trim(),
            out directPeerId,
            out directVirtualIp,
            out directOffer);
        if (coordinationServer.Text.Trim().Length == 0 && !hasDirectOffer)
        {
            output.Text = T("directRemoteOfferRequired");
            return;
        }
        output.Text = T("quickLanStarting");
        if (hasDirectOffer)
        {
            output.Text += Environment.NewLine + T("directBootstrapStarting") + " " + directPeerId + " @ " + directVirtualIp;
        }
        Application.DoEvents();
        StartNativeRuntime(false);
        output.Text += Environment.NewLine
            + Environment.NewLine
            + T("quickLanStarted");
    }

    void DecodeInvite()
    {
        if (invite.Text.Trim().Length == 0)
        {
            output.Text = T("joinNeedsInvite");
            return;
        }
        string text = RunCli("decode --invite " + Quote(invite.Text));
        string decodedSubnet = JsonStringValue(text, "virtual_subnet");
        if (decodedSubnet.Length > 0) subnet.Text = decodedSubnet;
        string hostPeer = JsonStringValue(text, "host_peer_id");
        string coordinationEndpoint = JsonStringValue(text, "coordination_endpoint");
        if (hostPeer.Length > 0)
        {
            hostRuntimePeerId = SafePeerId(hostPeer);
        }
        if (hostPeer.Length > 0 && hostName.Text.Trim().Length == 0)
        {
            hostName.Text = hostPeer;
        }
        if (coordinationEndpoint.Length > 0)
        {
            coordinationServer.Text = coordinationEndpoint;
        }
        UpdateRoomDetails("decoded");
    }

    void JoinRoom()
    {
        if (invite.Text.Trim().Length == 0)
        {
            output.Text = T("joinNeedsInvite");
            return;
        }
        string text = RunCli("join --invite " + Quote(invite.Text) + " --peer " + Quote(hostName.Text));
        string joinedSubnet = JsonStringValue(text, "virtualSubnet");
        string suggestedIp = JsonStringValue(text, "suggestedLocalIp");
        string hostIp = JsonStringValue(text, "hostIp");
        string hostPeer = JsonStringValue(text, "hostPeerId");
        string coordinationEndpoint = JsonStringValue(text, "coordinationEndpoint");
        if (joinedSubnet.Length > 0) subnet.Text = joinedSubnet;
        if (suggestedIp.Length > 0)
        {
            ip.Text = suggestedIp;
            localRuntimePeerId = SafePeerId(hostName.Text + "_" + suggestedIp.Replace('.', '_'));
        }
        if (hostIp.Length > 0) pingTarget.Text = hostIp;
        if (hostPeer.Length > 0)
        {
            hostRuntimePeerId = SafePeerId(hostPeer);
            if (hostIp.Length > 0)
            {
                remotePeer.Text = hostRuntimePeerId + "," + hostIp;
            }
        }
        if (coordinationEndpoint.Length > 0)
        {
            coordinationServer.Text = coordinationEndpoint;
        }
        UpdateRoomDetails("joined");
    }

    void CopyInvite()
    {
        CopyText(invite.Text, "inviteCopied");
    }

    void CopyVirtualIp()
    {
        CopyText(ip.Text, "ipCopied");
    }

    void CopyText(string value, string messageKey)
    {
        if (value.Trim().Length == 0)
        {
            output.Text = messageKey == "inviteCopied" ? T("copyInviteNeedsRoom") : T("nothingToCopy");
            return;
        }
        if (TryCopyToClipboard(value.Trim()))
        {
            output.Text = T(messageKey) + Environment.NewLine + value.Trim();
        }
        else
        {
            output.Text = T("clipboardCopyFailed") + Environment.NewLine + value.Trim();
        }
    }

    bool TryCopyToClipboard(string value)
    {
        try
        {
            Clipboard.SetText(value);
            return true;
        }
        catch
        {
            return false;
        }
    }

    void RunNetworkDiagnose()
    {
        if (userActionRunning)
        {
            output.Text = T("actionAlreadyRunning");
            return;
        }
        userActionRunning = true;
        SetActionButtonsEnabled(false);
        output.Text = T("networkDiagnoseRunning");
        string arguments = NetworkDiagnoseArgs();
        Task.Factory.StartNew(delegate
        {
            string network = "";
            Exception error = null;
            try
            {
                network = RunNativeCliCapture(arguments);
            }
            catch (Exception ex)
            {
                error = ex;
            }
            if (IsDisposed || !IsHandleCreated)
            {
                return;
            }
            BeginInvoke((MethodInvoker)delegate
            {
                userActionRunning = false;
                SetActionButtonsEnabled(true);
                if (error != null)
                {
                    ShowActionError("checkConnection", error);
                    return;
                }
                UpdateRoomDetailsFromNetworkReport(network);
                string pathReport = "";
                try
                {
                    pathReport = RunConnectionPathPlanAndReturn(false);
                }
                catch
                {
                }
                output.Text = CustomerNetworkSummary(network, pathReport);
            });
        });
    }

    string RunNetworkDiagnoseAndReturn()
    {
        return RunNetworkDiagnoseAndReturn(true);
    }

    string RunNetworkDiagnoseAndReturn(bool updateOutput)
    {
        string arguments = NetworkDiagnoseArgs();
        string text = updateOutput ? RunNativeCli(arguments) : RunNativeCliCapture(arguments);
        if (updateOutput)
        {
            UpdateRoomDetailsFromNetworkReport(text);
        }
        return text;
    }

    string NetworkDiagnoseArgs()
    {
        return "network-observe --adapter-name LocalAreaInterconnection --expected-ip " + ip.Text
            + " --subnet " + subnet.Text
            + " --adapter-scan true"
            + " --route-scan true"
            + PingArgs()
            + PacketObservationArgs()
            + " --broadcast-ports " + ports.Text
            + " --game-ports " + ports.Text;
    }

    void ExportDiagnostics()
    {
        using (SaveFileDialog dialog = new SaveFileDialog())
        {
            dialog.Title = T("saveDiagnosticBundle");
            dialog.Filter = T("jsonFilesFilter");
            dialog.FileName = "local-area-interconnection-diagnostic.json";
            if (dialog.ShowDialog(this) != DialogResult.OK)
            {
                return;
            }

            RunNativeCli("diagnostic-export --out " + Quote(dialog.FileName)
                + " --adapter-name LocalAreaInterconnection"
                + " --expected-ip " + ip.Text
                + " --subnet " + subnet.Text
                + PingArgs()
                + PacketObservationArgs()
                + RuntimeSnapshotArgs()
                + " --broadcast-ports " + ports.Text
                + " --game-ports " + ports.Text
                + " --game-name " + Quote(gameName.Text)
                + GameCatalogArgs()
                + " --ports " + ports.Text
                + " --packet-io-backend wintun"
                + " --route-scan true"
                + " --netstat-scan true"
                + RelayExportArgs()
                + NetshExportArgs());
            UpdateRoomDetailsFromDiagnosticBundle(dialog.FileName);
        }
    }

    void RunGameProfilePlan()
    {
        string catalog = gameCatalog.Text.Trim();
        if (catalog.Length == 0)
        {
            output.Text = T("missingGameCatalog");
            return;
        }

        string args = "game-profile-plan"
            + " --catalog " + Quote(catalog)
            + " --game-name " + Quote(gameName.Text)
            + " --subnet " + subnet.Text;
        string hostIp = pingTarget.Text.Trim();
        string localIp = ip.Text.Trim();
        if (LooksLikeIpv4(hostIp))
        {
            args += " --host-ip " + hostIp;
        }
        if (LooksLikeIpv4(localIp))
        {
            args += " --local-ip " + localIp;
        }
        string text = RunNativeCli(args);
        UpdateFromGameProfilePlan(text);
    }

    void RunGameProfileList()
    {
        string catalog = gameCatalog.Text.Trim();
        if (catalog.Length == 0)
        {
            output.Text = T("missingGameCatalog");
            return;
        }
        string query = gameName.Text.Trim();
        string args = "game-profile-list"
            + " --catalog " + Quote(catalog);
        if (query.Length > 0)
        {
            args += " --query " + Quote(query);
        }
        string text = RunNativeCli(args);
        UpdateFromGameProfileList(text);
    }

    void RunGamePortScan()
    {
        string text = RunNativeCli("game-port-scan"
            + " --netstat-scan true"
            + " --game-name " + Quote(gameName.Text)
            + GameCatalogArgs()
            + " --ports " + ports.Text);
        UpdateRoomDetailsFromGamePortScan(text);
    }

    void RunGameReadinessCheck()
    {
        string network = RunNetworkDiagnoseAndReturn();
        string text = RunGameReadinessFromNetworkReport(network);
        if (text.Length > 0)
        {
            output.Text = text;
        }
    }

    string RunGameReadinessFromNetworkReport(string network)
    {
        if (network.Trim().Length == 0)
        {
            return "";
        }
        string networkPath = Path.Combine(AppDataDirectory(), "game-readiness-network.json");
        File.WriteAllText(networkPath, network, Encoding.UTF8);
        string text = RunNativeCliCapture("game-readiness"
            + " --network-report " + Quote(networkPath)
            + " --game-name " + Quote(gameName.Text)
            + GameCatalogArgs()
            + " --subnet " + subnet.Text
            + " --discovery manual_ports"
            + " --ports " + ports.Text
            + FirewallReadinessArgs()
            + " --netstat-scan true"
            + " --local-ip " + ip.Text
            + RelayExportArgs());
        if (JsonStringValue(text, "status").Length > 0)
        {
            UpdateRoomDetailsFromGameReadiness(text);
        }
        return text;
    }

    void RunUdpTest()
    {
        RunPacketTestAndRefresh("udp-loopback-test --port " + FirstPortText("39077") + " --message ping");
    }

    void RunBroadcastTest()
    {
        RunPacketTestAndRefresh("udp-broadcast-test --port " + FirstPortText("39078") + " --message discover");
    }

    void RunNativeAdapterEnsure()
    {
        RunNativeCli("adapter-ensure --adapter-name LocalAreaInterconnection"
            + " --subnet " + subnet.Text
            + " --ip " + ip.Text
            + " --adapter-scan true");
    }

    void RunNativeRuntimeSelfTest()
    {
        string observePath = packetObservations.Text.Trim();
        if (observePath.Length == 0)
        {
            observePath = RuntimeFilePath("runtime-packets-self-test", "txt");
            packetObservations.Text = observePath;
        }
        string snapshotPath = RuntimeFilePath("runtime-snapshot-self-test", "json");
        latestRuntimeSnapshot = snapshotPath;
        latestRuntimeObservationFile = observePath;
        string peer = RuntimePeerId();
        string nativeOutput = RunNativeCli("room-runtime-run"
            + " --room-id desktop_self_test"
            + " --peer-id " + Quote(peer)
            + " --virtual-ip " + ip.Text
            + " --bind 127.0.0.1:0"
            + " --key desktop-test-room-key"
            + " --game-ports 0"
            + " --broadcast-ports 0"
            + " --duration-ms 300"
            + " --self-probe true"
            + " --capture-self-probe true"
            + " --forward-self-probe true"
            + " --inject-self-probe true"
            + " --packet-io-backend wintun"
            + " --forward-raw-ipv4 true"
            + " --wintun-runtime true"
            + " --heartbeat-interval-ms 75"
            + " --observe-file " + Quote(observePath)
            + " --snapshot-out " + Quote(snapshotPath)
            + " --snapshot-interval-ms 75");
        string diagnosticOutput = RunNetworkDiagnoseAndReturn();
        output.Text = nativeOutput + Environment.NewLine + Environment.NewLine + T("autoNetworkDiagnose") + Environment.NewLine + diagnosticOutput;
    }

    void RunWintunDetect()
    {
        string text = RunNativeCli("wintun-detect");
        UpdateRoomDetailsFromWintunReport(text);
    }

    void RunWintunSessionProbe()
    {
        string text = RunNativeCli("wintun-session-probe --adapter-name LocalAreaInterconnection --tunnel-type LocalAreaInterconnection");
        UpdateRoomDetailsFromWintunReport(text);
    }

    void StartNativeRuntime()
    {
        StartNativeRuntime(true);
    }

    void StartNativeRuntime(bool showDetails)
    {
        if (runtimeProcess != null && !runtimeProcess.HasExited)
        {
            output.Text = T("runtimeAlreadyRunning") + Environment.NewLine + RuntimeStatusText();
            return;
        }

        string observePath = packetObservations.Text.Trim();
        if (observePath.Length == 0)
        {
            observePath = RuntimeFilePath("runtime-packets", "txt");
            packetObservations.Text = observePath;
        }
        latestRuntimeObservationFile = observePath;
        latestRuntimeSnapshot = RuntimeFilePath("runtime-snapshot", "json");
        runtimeStopFile = RuntimeFilePath("runtime", "stop");
        if (File.Exists(runtimeStopFile)) File.Delete(runtimeStopFile);
        string peer = RuntimePeerId();
        string roomId = RuntimeRoomId();
        string virtualIp = ip.Text.Trim();
        string bind = NativeRuntimeBind();
        string roomKey = RuntimeRoomKey();
        string gamePort = FirstPortText("27015");
        string broadcastPort = FirstPortText("39078");
        string coordinationServerValue = coordinationServer.Text.Trim();
        string stunServerValue = stunServer.Text.Trim();
        int runtimePort = NativeRuntimePort();
        string args = "room-runtime-run"
            + " --room-id " + Quote(roomId)
            + " --peer-id " + Quote(peer)
            + " --virtual-ip " + virtualIp
            + " --bind " + bind
            + " --key " + Quote(roomKey)
            + " --game-ports " + gamePort
            + " --broadcast-ports " + broadcastPort
            + " --duration-ms 3600000"
            + " --peer-timeout-ms 0"
            + " --self-probe true"
            + " --capture-self-probe true"
            + " --forward-self-probe true"
            + " --inject-self-probe true"
            + " --packet-io-backend wintun"
            + " --forward-raw-ipv4 true"
            + " --wintun-runtime true"
            + " --heartbeat-interval-ms 1000"
            + " --observe-file " + Quote(observePath)
            + " --snapshot-out " + Quote(latestRuntimeSnapshot)
            + " --snapshot-interval-ms 1000"
            + " --stop-file " + Quote(runtimeStopFile)
            + RuntimeNatBootstrapStunArgs(stunServerValue)
            + RuntimeNatBootstrapUpnpArgs()
            + RuntimeCoordinationArgs()
            + RuntimeCoordinationMonitorArgs();

        runtimeOutput.Length = 0;
        lastRuntimeLogLength = 0;
        runtimeProcess = StartNativeRuntimeProcess(args);
        if (runtimeProcess == null)
        {
            return;
        }
        output.Text = T("runtimeStarted")
            + Environment.NewLine + RuntimeStatusText()
            + Environment.NewLine + T("runtimeSnapshotPath") + latestRuntimeSnapshot
            + Environment.NewLine + T("runtimeObservationPath") + observePath;
        string directPeerId;
        string directVirtualIp;
        string directOffer;
        if (TryParseRemotePeerOfferSpec(remotePeer.Text.Trim(), out directPeerId, out directVirtualIp, out directOffer))
        {
            output.Text += Environment.NewLine + T("directBootstrapStarting") + " " + directPeerId + " @ " + directVirtualIp;
        }
        if (latestNativeOfferFile.Length > 0)
        {
            output.Text += Environment.NewLine + T("nativeOfferPath") + latestNativeOfferFile;
        }
        StartRuntimePostStartWork(showDetails, roomId, peer, virtualIp, bind, stunServerValue, coordinationServerValue, runtimePort);
        UpdateRoomDetails("joined");
        RefreshCoordinationRoomView(false);
    }

    void StopNativeRuntime()
    {
        StopNativeRuntime(true);
    }

    void StopNativeRuntime(bool leaveCoordination)
    {
        if (runtimeProcess == null || runtimeProcess.HasExited)
        {
            output.Text = T("runtimeNotRunning");
            return;
        }
        string leaveOutput = leaveCoordination ? LeaveCoordinationRoomIfConfigured() : "";
        if (runtimeStopFile.Length > 0)
        {
            File.WriteAllText(runtimeStopFile, "stop");
        }
        StopRuntimeProcess(5000);
        output.Text = T("runtimeStopped")
            + Environment.NewLine + RuntimeStatusText()
            + Environment.NewLine + runtimeOutput.ToString();
        if (leaveOutput.Length > 0)
        {
            output.Text += Environment.NewLine + Environment.NewLine + leaveOutput;
        }
        if (latestRuntimeSnapshot.Length > 0 && File.Exists(latestRuntimeSnapshot))
        {
            output.Text += Environment.NewLine + T("runtimeSnapshotReady") + latestRuntimeSnapshot;
            RefreshRuntimeStatus();
        }
    }

    void RunRuntimeCleanupPlan()
    {
        string text;
        if (latestRuntimeSnapshot.Length > 0 && File.Exists(latestRuntimeSnapshot))
        {
            text = RunNativeCli("runtime-cleanup-report"
                + " --runtime-snapshot " + Quote(latestRuntimeSnapshot)
                + " --adapter-name LocalAreaInterconnection"
                + " --adapter-scan true"
                + " --route-scan true");
            UpdateRoomDetailsFromRuntimeCleanupReport(text);
            return;
        }

        text = RunNativeCli("runtime-cleanup-plan"
                + " --room-id " + Quote(RuntimeRoomId())
                + " --peer-id " + Quote(RuntimePeerId())
                + " --virtual-ip " + ip.Text
                + " --subnet " + subnet.Text
                + " --adapter-name LocalAreaInterconnection"
                + " --packet-io-backend wintun"
                + " --restore-adapter true"
                + " --cleanup-routes true");
        UpdateRoomDetailsFromRuntimeCleanupPlan(text);
    }

    void RunRuntimeCleanupApply()
    {
        bool confirmed = MessageBox.Show(
            this,
            T("runtimeCleanupApplyConfirm"),
            T("runtimeCleanupApply"),
            MessageBoxButtons.YesNo,
            MessageBoxIcon.Warning) == DialogResult.Yes;
        string args = "runtime-cleanup-apply";
        if (latestRuntimeSnapshot.Length > 0 && File.Exists(latestRuntimeSnapshot))
        {
            args += " --runtime-snapshot " + Quote(latestRuntimeSnapshot);
        }
        else
        {
            string planText = RunNativeCliCapture("runtime-cleanup-plan"
                + " --room-id " + Quote(RuntimeRoomId())
                + " --peer-id " + Quote(RuntimePeerId())
                + " --virtual-ip " + ip.Text
                + " --subnet " + subnet.Text
                + " --adapter-name LocalAreaInterconnection"
                + " --packet-io-backend wintun"
                + " --restore-adapter true"
                + " --cleanup-routes true");
            string planPath = RuntimeFilePath("runtime-cleanup-plan-apply", "json");
            File.WriteAllText(planPath, planText, Encoding.UTF8);
            args += " --cleanup-plan " + Quote(planPath);
        }
        args += " --adapter-name LocalAreaInterconnection"
            + " --adapter-scan true"
            + " --route-scan true";
        if (confirmed)
        {
            args += " --yes true";
        }

        string text = RunNativeCli(args);
        UpdateRoomDetailsFromRuntimeCleanupApply(text);
    }

    void RunRouteScan()
    {
        string text = RunNativeCli("route-scan"
            + " --route-scan true"
            + " --virtual-ip " + ip.Text
            + " --subnet " + subnet.Text);
        UpdateRoomDetailsFromRouteScan(text);
    }

    void CloseCoordinationRoom()
    {
        if (coordinationServer.Text.Trim().Length == 0)
        {
            output.Text = T("coordinationServerRequired");
            return;
        }
        string closeOutput = CloseCoordinationRoomIfConfigured();
        if (runtimeProcess != null && !runtimeProcess.HasExited)
        {
            StopNativeRuntime(false);
            if (closeOutput.Length > 0)
            {
                output.Text += Environment.NewLine + Environment.NewLine + closeOutput;
            }
        }
        else if (closeOutput.Length > 0)
        {
            output.Text = closeOutput;
        }
        UpdateRoomDetails("closed");
        RefreshCoordinationRoomView(false);
    }

    void KickCoordinationPeer()
    {
        string result = KickCoordinationPeerIfConfigured();
        if (result.Length > 0)
        {
            output.Text = result;
        }
        RefreshCoordinationRoomView(false);
    }

    void RunNativeNatSelfTest()
    {
        RunNativeCli("nat-hole-punch-loopback-test"
            + " --room-id desktop_self_test"
            + " --peer-a " + Quote(RuntimePeerId() + "_a")
            + " --peer-b " + Quote(RuntimePeerId() + "_b")
            + " --attempts 3"
            + " --interval-ms 0"
            + " --message desktop-nat");
    }

    void RunRelayFallbackPlan()
    {
        string peer = RuntimePeerId();
        if (CreateNativeOffer(peer, false).Length == 0)
        {
            return;
        }
        string remoteOffer = RemoteOfferForRelayPlan(peer);
        if (remoteOffer.Length == 0)
        {
            output.Text = T("remoteOfferRequired");
            return;
        }

        string text = RunNativeCli("relay-fallback-plan"
            + " --local-offer " + Quote(latestNativeOfferFile)
            + " --remote-offer " + Quote(remoteOffer)
            + " --p2p-status failed");
        UpdateRoomDetailsFromRelayPlan(text);
    }

    void RunConnectionPathPlan()
    {
        string text = RunConnectionPathPlanAndReturn(true, "unknown");
        if (text.Length == 0)
        {
            return;
        }
        UpdateRoomDetailsFromConnectionPathPlan(text);
    }

    string RunConnectionPathPlanAndReturn(bool updateOutput)
    {
        return RunConnectionPathPlanAndReturn(updateOutput, "unknown");
    }

    string RunConnectionPathPlanAndReturn(bool updateOutput, string p2pStatus)
    {
        string peer = RuntimePeerId();
        string remoteOffer = RemoteOfferForRelayPlan(peer);
        if (remoteOffer.Length == 0)
        {
            if (updateOutput)
            {
                output.Text = T("remoteOfferRequired");
            }
            return "";
        }
        if (CreateNativeOffer(peer, false).Length == 0)
        {
            return "";
        }

        string text = updateOutput
            ? RunNativeCli("connection-path-plan"
                + " --local-offer " + Quote(latestNativeOfferFile)
                + " --remote-offer " + Quote(remoteOffer)
                + " --p2p-status " + Quote(p2pStatus))
            : RunNativeCliCapture("connection-path-plan"
                + " --local-offer " + Quote(latestNativeOfferFile)
                + " --remote-offer " + Quote(remoteOffer)
                + " --p2p-status " + Quote(p2pStatus));
        return text;
    }

    void RunNativeOffer()
    {
        string peer = RuntimePeerId();
        string result = CreateNativeOffer(peer, true);
        if (result.Length == 0) return;

        string publishOutput = PublishNativeOfferFileIfConfigured(true);
        if (publishOutput.Length > 0)
        {
            output.Text = result + Environment.NewLine + Environment.NewLine + publishOutput;
        }
        RefreshCoordinationRoomView(false);
    }

    void RunDirectOffer()
    {
        string peer = RuntimePeerId();
        string result = CreateNativeOffer(peer, false);
        if (result.Length == 0 || latestNativeOfferFile.Length == 0 || !File.Exists(latestNativeOfferFile))
        {
            output.Text = T("directOfferFailed");
            return;
        }
        string offer = CompactJson(File.ReadAllText(latestNativeOfferFile, Encoding.UTF8).Trim());
        string spec = peer + "," + ip.Text.Trim() + "," + offer;
        bool copied = TryCopyToClipboard(spec);
        output.Text = T("directOfferReady")
            + Environment.NewLine
            + T(copied ? "quickInviteCopied" : "quickInviteCopyFailed")
            + Environment.NewLine
            + StunMappingText(result)
            + Environment.NewLine
            + UpnpMappingText(result)
            + Environment.NewLine
            + spec
            + Environment.NewLine
            + Environment.NewLine
            + T("directOfferNext");
    }

    void RunDirectSelfTest()
    {
        string text = RunConnectionPathPlanAndReturn(false, "unknown");
        if (text.Length == 0)
        {
            output.Text = T("directRemoteOfferRequired");
            return;
        }

        string status = JsonStringValue(text, "status");
        string selectedPath = JsonStringValue(text, "selected_path");
        string localNat = JsonStringValue(text, "local_nat_assessment");
        string remoteNat = JsonStringValue(text, "remote_nat_assessment");
        string localHost = JsonNumberValue(text, "local_host_candidate_count");
        string localSrflx = JsonNumberValue(text, "local_srflx_candidate_count");
        string remoteHost = JsonNumberValue(text, "remote_host_candidate_count");
        string remoteSrflx = JsonNumberValue(text, "remote_srflx_candidate_count");
        string endpoint = JsonFirstStringInArray(JsonArrayValue(text, "selected_endpoints"));
        string nextAction = JsonFirstStringInArray(JsonArrayValue(text, "recommended_actions"));
        string bootstrap = RunDirectBootstrapProbeAndReturn();
        string bootstrapStatus = JsonStringValue(bootstrap, "status");
        string bootstrapLocal = JsonStringValue(bootstrap, "localEndpoint");
        string selectedPeer = JsonObjectValue(bootstrap, "selectedPeer");
        string bootstrapPeer = JsonStringValue(selectedPeer, "endpoint");
        string bootstrapRole = JsonStringValue(selectedPeer, "handshakeRole");
        string bootstrapAck = JsonBoolTextValue(selectedPeer, "confirmedByAck");
        string bootstrapError = JsonStringValue(bootstrap, "error");
        string localOfferText = latestNativeOfferFile.Length > 0 && File.Exists(latestNativeOfferFile)
            ? File.ReadAllText(latestNativeOfferFile, Encoding.UTF8)
            : "";
        string upnpHint = CandidateSourceCountText(localOfferText, "upnp-port-mapping");
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

        output.Text = T("directSelfTestReady")
            + Environment.NewLine
            + "status=" + status + ", path=" + selectedPath
            + Environment.NewLine
            + "local NAT=" + localNat + ", host/srflx=" + localHost + "/" + localSrflx
            + (upnpHint.Length > 0 ? ", " + upnpHint : "")
            + Environment.NewLine
            + "remote NAT=" + remoteNat + ", host/srflx=" + remoteHost + "/" + remoteSrflx
            + Environment.NewLine
            + "target=" + endpoint
            + Environment.NewLine
            + "bootstrap=" + (bootstrapStatus.Length > 0 ? bootstrapStatus : T("stateUnknown"))
            + (bootstrapAck.Length > 0 ? ", ack=" + bootstrapAck : "")
            + (bootstrapRole.Length > 0 ? ", role=" + bootstrapRole : "")
            + (bootstrapLocal.Length > 0 ? ", local=" + bootstrapLocal : "")
            + (bootstrapPeer.Length > 0 ? ", peer=" + bootstrapPeer : "")
            + (bootstrapError.Length > 0 ? ", error=" + bootstrapError : "")
            + Environment.NewLine
            + T("detailNext") + " " + nextAction
            + Environment.NewLine
            + Environment.NewLine
            + text
            + (bootstrap.Length > 0
                ? Environment.NewLine + Environment.NewLine + T("directBootstrapStarting") + Environment.NewLine + bootstrap
                : "");
        UpdateRoomDetailsFromConnectionPathPlan(text);
    }

    string RunDirectBootstrapProbeAndReturn()
    {
        string peerId;
        string virtualIp;
        string offerValue;
        if (!TryParseRemotePeerOfferSpec(remotePeer.Text.Trim(), out peerId, out virtualIp, out offerValue))
        {
            return "";
        }
        string preparedOffer = PrepareOfferArgumentFile(offerValue, "remote-offer-direct-self-test.json");
        if (preparedOffer.Length == 0)
        {
            return "";
        }
        string arguments = "nat-p2p-bootstrap"
            + " --room-id " + Quote(RuntimeRoomId())
            + " --peer-id " + Quote(RuntimePeerId())
            + " --virtual-ip " + Quote(ip.Text.Trim())
            + " --key " + Quote(RuntimeRoomKey())
            + " --bind " + Quote(NativeRuntimeBind())
            + " --remote-offer " + Quote(preparedOffer)
            + " --punch-attempts 8"
            + " --punch-interval-ms 50"
            + " --handshake-timeout-ms 5000"
            + StunArgs(stunServer.Text.Trim())
            + UpnpPortMapArgs();
        try
        {
            return RunNativeCliCapture(arguments);
        }
        catch (Exception ex)
        {
            return "{\"status\":\"error\",\"error\":" + JsonStringLiteral(ex.Message) + "}";
        }
    }

    void StartLocalCoordinationServer()
    {
        if (coordinationProcess != null && !coordinationProcess.HasExited)
        {
            output.Text = T("coordinationAlreadyRunning") + Environment.NewLine + CoordinationStatusText();
            return;
        }
        string bind = CoordinationBind();
        if (coordinationServer.Text.Trim().Length == 0)
        {
            coordinationServer.Text = LocalCoordinationEndpoint();
            bind = CoordinationBind();
        }
        coordinationStoreFile = RuntimeRoomFilePath("coordination-store", "json");
        string firewallOutput = EnsureCoordinationFirewallRule();
        coordinationOutput.Length = 0;
        coordinationProcess = StartNativeBackgroundProcess(
            "coordination-http-serve"
            + " --bind " + Quote(bind)
            + " --store " + Quote(coordinationStoreFile)
            + " --max-requests 0"
            + " --request-timeout-ms 30000",
            coordinationOutput,
            T("coordinationExited"));
        if (coordinationProcess == null)
        {
            return;
        }
        output.Text = T("coordinationStarted")
            + Environment.NewLine + CoordinationStatusText()
            + Environment.NewLine + T("coordinationServerUrl") + coordinationServer.Text.Trim()
            + Environment.NewLine + T("coordinationStorePath") + coordinationStoreFile;
        if (firewallOutput.Length > 0)
        {
            output.Text += Environment.NewLine + Environment.NewLine + firewallOutput;
        }
        RefreshCoordinationRoomView(false);
    }

    void StopLocalCoordinationServer()
    {
        if (coordinationProcess == null || coordinationProcess.HasExited)
        {
            output.Text = T("coordinationNotRunning");
            return;
        }
        StopCoordinationProcess(2000);
        output.Text = T("coordinationStopped")
            + Environment.NewLine + CoordinationStatusText()
            + Environment.NewLine + coordinationOutput.ToString();
    }

    void RefreshCoordinationRoomView(bool showOutput)
    {
        if (coordinationRoomRefreshRunning)
        {
            return;
        }
        string currentSubnet = subnet.Text.Trim();
        if (currentSubnet.Length == 0)
        {
            return;
        }
        string peer = RuntimePeerId();
        string server = coordinationServer.Text.Trim();
        string roomId = RuntimeRoomId();
        string store = coordinationStoreFile;
        string arguments;
        if (server.Length > 0)
        {
            arguments = "coordination-http-room-view"
                + " --server " + Quote(server)
                + " --room-id " + Quote(roomId)
                + " --peer-id " + Quote(peer)
                + " --subnet " + Quote(currentSubnet);
        }
        else
        {
            if (store.Length == 0 || !File.Exists(store))
            {
                return;
            }
            arguments = "coordination-room-view"
                + " --store " + Quote(store)
                + " --room-id " + Quote(roomId)
                + " --peer-id " + Quote(peer)
                + " --subnet " + Quote(currentSubnet);
        }
        coordinationRoomRefreshRunning = true;
        Task.Factory.StartNew(delegate
        {
            string text = "";
            try
            {
                text = RunNativeCliCapture(arguments);
            }
            catch (Exception ex)
            {
                text = ex.Message;
            }
            if (IsDisposed || !IsHandleCreated)
            {
                coordinationRoomRefreshRunning = false;
                return;
            }
            try
            {
                BeginInvoke((MethodInvoker)delegate
                {
                    coordinationRoomRefreshRunning = false;
                    if (showOutput && text.Length > 0)
                    {
                        output.Text = text;
                    }
                    UpdateRoomDetailsFromCoordinationView(text);
                    AutoConfigureRemotePeerFromCoordinationView(text);
                });
            }
            catch
            {
                coordinationRoomRefreshRunning = false;
            }
        });
    }

    void RunTcpTest()
    {
        RunPacketTestAndRefresh("tcp-loopback-test --port " + FirstPortText("39079") + " --message ping");
    }

    void RunPacketTestAndRefresh(string command)
    {
        string testOutput = RunNativeCli(command + ObserveFileArgs());
        string path = packetObservations.Text.Trim();
        if (path.Length == 0 || !File.Exists(path))
        {
            return;
        }

        string diagnosticOutput = RunNetworkDiagnoseAndReturn();
        output.Text = testOutput + Environment.NewLine + Environment.NewLine + T("autoNetworkDiagnose") + Environment.NewLine + diagnosticOutput;
    }
}

