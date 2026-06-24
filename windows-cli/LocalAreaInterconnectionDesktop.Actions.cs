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
        EnsureRelayDefaults();
        if (roomName.Text.Trim().Length == 0 || hostName.Text.Trim().Length == 0)
        {
            output.Text = T("hostNeedsName");
            return;
        }
        CreateRoom();
        string publishOutput = PublishNativeOfferIfConfigured(RuntimePeerId(), false);
        if (invite.Text.Trim().Length > 0)
        {
            bool copied = TryCopyToClipboard(invite.Text.Trim());
            output.Text += Environment.NewLine
                + Environment.NewLine
                + T(copied ? "quickInviteCopied" : "quickInviteCopyFailed")
                + Environment.NewLine
                + T("quickNextHost");
        }
        if (CoordinationPublishLooksSuccessful(publishOutput))
        {
            output.Text += Environment.NewLine + Environment.NewLine + T("coordinationOfferPublished");
        }
        else if (publishOutput.Length > 0)
        {
            output.Text += Environment.NewLine + Environment.NewLine + T("coordinationPublishFailed")
                + Environment.NewLine + publishOutput
                + Environment.NewLine
                + T("coordinationManualOfferFallback");
        }
        roomUiMode = "host";
        UpdateHomeActionButtons();
        RefreshCoordinationRoomView(false);
    }

    void QuickJoinRoom()
    {
        EnsureRelayDefaults();
        if (invite.Text.Trim().Length == 0)
        {
            output.Text = T("joinNeedsInvite");
            return;
        }
        DecodeInvite();
        JoinRoom();
        string publishOutput = PublishNativeOfferIfConfigured(RuntimePeerId(), false);
        output.Text += Environment.NewLine
            + Environment.NewLine
            + T("quickJoinedNext");
        if (CoordinationPublishLooksSuccessful(publishOutput))
        {
            output.Text += Environment.NewLine + T("coordinationOfferPublished");
        }
        else if (publishOutput.Length > 0)
        {
            output.Text += Environment.NewLine + T("coordinationPublishFailed")
                + Environment.NewLine + publishOutput
                + Environment.NewLine
                + T("coordinationManualOfferFallback");
        }
        roomUiMode = "joined";
        UpdateHomeActionButtons();
        RefreshCoordinationRoomView(false);
    }

    void StartLanSession()
    {
        EnsureRelayDefaults();
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
        string coordinationPeerId;
        string coordinationVirtualIp;
        bool hasCoordinationPeer = TryParseCoordinationPeerSpec(
            remotePeer.Text.Trim(),
            out coordinationPeerId,
            out coordinationVirtualIp);
        if (!hasDirectOffer && !hasCoordinationPeer && coordinationServer.Text.Trim().Length > 0)
        {
            string discovered = ConfigureRemotePeerFromCoordinationNow();
            hasDirectOffer = TryParseRemotePeerOfferSpec(
                remotePeer.Text.Trim(),
                out directPeerId,
                out directVirtualIp,
                out directOffer);
            hasCoordinationPeer = TryParseCoordinationPeerSpec(
                remotePeer.Text.Trim(),
                out coordinationPeerId,
                out coordinationVirtualIp);
            if (!hasDirectOffer && !hasCoordinationPeer && discovered.Length > 0)
            {
                output.Text = discovered;
                return;
            }
        }
        if (coordinationServer.Text.Trim().Length == 0 && !hasDirectOffer)
        {
            output.Text = T("directRemoteOfferRequired");
            return;
        }
        if (coordinationServer.Text.Trim().Length > 0 && !hasDirectOffer && !hasCoordinationPeer)
        {
            output.Text = T("coordinationWaitingForPeer");
            return;
        }
        string startText = T("quickLanStarting");
        string modeBeforeStart = roomUiMode == "host" ? "host" : "joined";
        if (hasDirectOffer)
        {
            startText += Environment.NewLine + T("directBootstrapStarting") + " " + directPeerId + " @ " + directVirtualIp;
        }
        else if (hasCoordinationPeer)
        {
            startText += Environment.NewLine + T("coordinationBootstrapStarting") + " " + coordinationPeerId + " @ " + coordinationVirtualIp;
        }
        output.Text = startText;
        roomUiMode = "running";
        UpdateHomeActionButtons();
        RunPrepareLanEnvironmentAsync(true, startText, modeBeforeStart);
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
        string text = value.Trim();
        if (TryCopyToClipboard(text))
        {
            output.Text = T(messageKey);
        }
        else
        {
            output.Text = T("clipboardCopyFailed") + Environment.NewLine + text;
        }
    }

    void EnsureRelayDefaults()
    {
        if (coordinationServer != null && coordinationServer.Text.Trim().Length == 0)
        {
            coordinationServer.Text = DefaultCoordinationServer();
        }
        else if (coordinationServer != null)
        {
            coordinationServer.Text = NormalizeCoordinationServer(coordinationServer.Text);
        }
        if (relayServer != null && relayServer.Text.Trim().Length == 0)
        {
            relayServer.Text = DefaultRelayServer();
        }
        else if (relayServer != null)
        {
            relayServer.Text = NormalizeRelayServer(relayServer.Text);
        }
        if (stunServer != null && stunServer.Text.Trim().Length == 0)
        {
            stunServer.Text = DefaultStunServer();
        }
    }

    bool TryCopyToClipboard(string value)
    {
        for (int attempt = 0; attempt < 5; attempt++)
        {
            try
            {
                Clipboard.Clear();
                Clipboard.SetText(value, TextDataFormat.UnicodeText);
                if (ClipboardTextMatches(value)) return true;
            }
            catch
            {
                try
                {
                    Clipboard.SetDataObject(value, true, 10, 150);
                    if (ClipboardTextMatches(value)) return true;
                }
                catch
                {
                    System.Threading.Thread.Sleep(80);
                }
            }
        }
        if (TryCopyToClipboardWithWin32(value)) return true;
        return TryCopyToClipboardWithPowerShell(value);
    }

    bool ClipboardTextMatches(string value)
    {
        try
        {
            return Clipboard.ContainsText() && Clipboard.GetText(TextDataFormat.UnicodeText) == value;
        }
        catch
        {
            return false;
        }
    }

    bool TryCopyToClipboardWithPowerShell(string value)
    {
        string path = "";
        try
        {
            path = Path.Combine(LogDirectory(), "clipboard-" + DateTime.UtcNow.ToString("yyyyMMddHHmmssfff") + ".txt");
            File.WriteAllText(path, value, new UTF8Encoding(false));
            ProcessStartInfo start = new ProcessStartInfo();
            start.FileName = "powershell.exe";
            start.Arguments = "-NoProfile -ExecutionPolicy Bypass -Command \"$text = Get-Content -Raw -LiteralPath "
                + PowerShellSingleQuoted(path)
                + "; Set-Clipboard -Value $text\"";
            start.UseShellExecute = false;
            start.CreateNoWindow = true;
            using (Process process = Process.Start(start))
            {
                if (process == null) return false;
                if (!process.WaitForExit(5000)) return false;
            }
            System.Threading.Thread.Sleep(120);
            return ClipboardTextMatches(value);
        }
        catch
        {
            return false;
        }
        finally
        {
            try
            {
                if (path.Length > 0 && File.Exists(path)) File.Delete(path);
            }
            catch
            {
            }
        }
    }

    bool TryCopyToClipboardWithWin32(string value)
    {
        IntPtr global = IntPtr.Zero;
        IntPtr locked = IntPtr.Zero;
        bool opened = false;
        try
        {
            byte[] bytes = Encoding.Unicode.GetBytes(value + "\0");
            global = Native.GlobalAlloc(Native.GmemMoveable, (UIntPtr)bytes.Length);
            if (global == IntPtr.Zero) return false;
            locked = Native.GlobalLock(global);
            if (locked == IntPtr.Zero) return false;
            Marshal.Copy(bytes, 0, locked, bytes.Length);
            Native.GlobalUnlock(global);
            locked = IntPtr.Zero;

            for (int attempt = 0; attempt < 8; attempt++)
            {
                opened = Native.OpenClipboard(Handle);
                if (opened) break;
                System.Threading.Thread.Sleep(60);
            }
            if (!opened) return false;
            if (!Native.EmptyClipboard()) return false;
            if (Native.SetClipboardData(Native.CfUnicodeText, global) == IntPtr.Zero) return false;
            global = IntPtr.Zero;
            Native.CloseClipboard();
            opened = false;
            System.Threading.Thread.Sleep(80);
            return ClipboardTextMatches(value);
        }
        catch
        {
            return false;
        }
        finally
        {
            if (opened) Native.CloseClipboard();
            if (locked != IntPtr.Zero) Native.GlobalUnlock(global);
            if (global != IntPtr.Zero) Native.GlobalFree(global);
        }
    }

    string PowerShellSingleQuoted(string value)
    {
        return "'" + value.Replace("'", "''") + "'";
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
            + RuntimeSnapshotArgs()
            + " --broadcast-ports " + ports.Text
            + " --game-ports " + ports.Text;
    }

    void ExportDiagnostics()
    {
        string path = Path.Combine(
            LogDirectory(),
            "local-area-interconnection-diagnostic-" + DateTime.UtcNow.ToString("yyyyMMddHHmmssfff") + ".json");
        RunNativeCli("diagnostic-export --out " + Quote(path)
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
        UpdateRoomDetailsFromDiagnosticBundle(path);
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
        string networkPath = Path.Combine(LogDirectory(), "game-readiness-network.json");
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

    void RunNativeAdapterApply()
    {
        RunNativeCliElevatedWithConfirmation(
            "adapter-ensure --adapter-name LocalAreaInterconnection"
                + " --subnet " + subnet.Text
                + " --ip " + ip.Text
                + " --adapter-scan true"
                + " --yes true",
            "adapterApplyConfirm",
            "nativeAdapterApply",
            "adapter-apply");
    }

    void RunFirewallApply()
    {
        RunNativeCliElevatedWithConfirmation(
            FirewallApplyArgs() + " --yes true",
            "firewallApplyConfirm",
            "firewallApply",
            "firewall-apply");
    }

    bool PrepareLanEnvironmentForStart()
    {
        string[] steps = LanEnvironmentPrepareArgs();
        if (steps.Length == 0)
        {
            output.Text = T("prepareLanSkipped");
            return true;
        }
        if (!ConfirmAdminAction("prepareLanConfirm", "prepareLanEnvironment"))
        {
            output.Text = T("prepareLanCancelled");
            return false;
        }
        string text = RunNativeCliElevatedBatch(steps, "prepare-lan");
        output.Text = T("prepareLanFinished") + Environment.NewLine + text;
        return ElevatedTextLooksSuccessful(text) || FirewallPrepareFailureCanContinue(text);
    }

    void RunPrepareLanEnvironment()
    {
        RunPrepareLanEnvironmentAsync(false, T("prepareLanStarting"), roomUiMode);
    }

    void RunPrepareLanEnvironmentAsync(bool startRuntimeAfterPrepare, string initialText)
    {
        RunPrepareLanEnvironmentAsync(startRuntimeAfterPrepare, initialText, roomUiMode, null);
    }

    void RunPrepareLanEnvironmentAsync(bool startRuntimeAfterPrepare, string initialText, string modeAfterFailure)
    {
        RunPrepareLanEnvironmentAsync(startRuntimeAfterPrepare, initialText, modeAfterFailure, null);
    }

    void RunPrepareLanEnvironmentAsync(bool startRuntimeAfterPrepare, string initialText, string modeAfterFailure, string runtimeBindForStart)
    {
        if (userActionRunning)
        {
            output.Text = T("actionAlreadyRunning");
            return;
        }
        if (startRuntimeAfterPrepare && runtimeBindForStart == null)
        {
            runtimeBindForStart = AllocateNativeRuntimeBindForStart();
        }
        int runtimePortForStart = startRuntimeAfterPrepare && runtimeBindForStart != null
            ? RuntimePortFromBind(runtimeBindForStart)
            : 0;
        string[] steps = LanEnvironmentPrepareArgs(runtimePortForStart);
        if (steps.Length == 0)
        {
            output.Text = initialText + Environment.NewLine + T("prepareLanSkipped");
            if (startRuntimeAfterPrepare)
            {
                StartNativeRuntime(false, runtimeBindForStart);
                if (runtimeProcess != null && !runtimeProcess.HasExited)
                {
                    output.Text += Environment.NewLine
                        + Environment.NewLine
                        + T("quickLanStarted");
                }
            }
            return;
        }
        if (!ConfirmAdminAction("prepareLanConfirm", "prepareLanEnvironment"))
        {
            output.Text = T("prepareLanCancelled");
            return;
        }
        userActionRunning = true;
        SetActionButtonsEnabled(false);
        output.Text = initialText;
        Task.Factory.StartNew(delegate
        {
            string prepareText = "";
            Exception error = null;
            try
            {
                prepareText = RunNativeCliElevatedBatch(steps, "prepare-lan");
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
                    ShowActionError("prepareLanEnvironment", error);
                    roomUiMode = modeAfterFailure;
                    UpdateHomeActionButtons();
                    return;
                }
                bool prepared = ElevatedTextLooksSuccessful(prepareText);
                bool firewallWarningOnly = !prepared && FirewallPrepareFailureCanContinue(prepareText);
                output.Text = T("prepareLanFinished") + Environment.NewLine + prepareText;
                if (!prepared && !firewallWarningOnly)
                {
                    output.Text += Environment.NewLine + Environment.NewLine + T("prepareLanFailed");
                    roomUiMode = modeAfterFailure;
                    UpdateHomeActionButtons();
                    return;
                }
                if (firewallWarningOnly)
                {
                    output.Text += Environment.NewLine + Environment.NewLine + T("prepareLanFirewallWarningContinue");
                }
                if (startRuntimeAfterPrepare)
                {
                    StartNativeRuntime(false, runtimeBindForStart);
                    if (runtimeProcess != null && !runtimeProcess.HasExited)
                    {
                        output.Text += Environment.NewLine
                            + Environment.NewLine
                            + T("quickLanStarted");
                    }
                }
            });
        });
    }

    string[] LanEnvironmentPrepareArgs()
    {
        return LanEnvironmentPrepareArgs(0);
    }

    string[] LanEnvironmentPrepareArgs(int runtimePort)
    {
        return new string[]
        {
            "adapter-apply --adapter-name LocalAreaInterconnection"
                + " --subnet " + subnet.Text
                + " --ip " + ip.Text
                + " --yes true",
            FirewallApplyArgs(runtimePort) + " --yes true",
        };
    }

    bool FirewallPrepareFailureCanContinue(string text)
    {
        if (ElevatedExitCode(text) != "1") return false;
        return text.IndexOf("advfirewall", StringComparison.OrdinalIgnoreCase) >= 0
            || text.IndexOf("firewall apply did not complete", StringComparison.OrdinalIgnoreCase) >= 0;
    }

    string FirewallApplyArgs()
    {
        return FirewallApplyArgs(0);
    }

    string FirewallApplyArgs(int runtimePort)
    {
        return "firewall-apply --game-name " + Quote(gameName.Text)
            + GameCatalogArgs()
            + " --subnet " + subnet.Text
            + " --ports " + FirewallPortsText(runtimePort)
            + " --remote-scope any";
    }

    string FirewallPortsText(int runtimePort)
    {
        List<string> values = new List<string>();
        Dictionary<string, bool> seen = new Dictionary<string, bool>(StringComparer.OrdinalIgnoreCase);
        string[] parts = ports.Text.Split(new char[] { ',', ';', ' ', '\t', '\r', '\n' }, StringSplitOptions.RemoveEmptyEntries);
        foreach (string rawPart in parts)
        {
            string part = rawPart.Trim();
            if (part.Length == 0 || seen.ContainsKey(part)) continue;
            seen[part] = true;
            values.Add(part);
        }
        if (runtimePort > 0)
        {
            string runtimePortText = runtimePort.ToString(CultureInfo.InvariantCulture);
            if (!seen.ContainsKey(runtimePortText))
            {
                values.Add(runtimePortText);
            }
        }
        if (values.Count == 0)
        {
            values.Add(FirstPortText("27015"));
        }
        return String.Join(",", values.ToArray());
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
        if (runtimeProcess != null && !runtimeProcess.HasExited)
        {
            output.Text = T("runtimeAlreadyRunning") + Environment.NewLine + RuntimeStatusText();
            return;
        }
        RunPrepareLanEnvironmentAsync(true, T("prepareLanStarting"));
    }

    void StartNativeRuntime(bool showDetails)
    {
        StartNativeRuntime(showDetails, null);
    }

    void StartNativeRuntime(bool showDetails, string preferredBind)
    {
        EnsureRelayDefaults();
        if (runtimeProcess != null && !runtimeProcess.HasExited)
        {
            output.Text = T("runtimeAlreadyRunning") + Environment.NewLine + RuntimeStatusText();
            return;
        }

        string observePath = packetObservations.Text.Trim();
        latestRuntimeObservationFile = observePath;
        latestRuntimeSnapshot = RuntimeFilePath("runtime-snapshot", "json");
        runtimeStopFile = RuntimeFilePath("runtime", "stop");
        if (File.Exists(runtimeStopFile)) File.Delete(runtimeStopFile);
        string peer = RuntimePeerId();
        string roomId = RuntimeRoomId();
        string virtualIp = ip.Text.Trim();
        string bind = preferredBind != null && preferredBind.Trim().Length > 0
            ? preferredBind
            : AllocateNativeRuntimeBindForStart();
        string roomKey = RuntimeRoomKey();
        string gamePort = FirstPortText("27015");
        string broadcastPort = FirstPortText("39078");
        string coordinationServerValue = coordinationServer.Text.Trim();
        string stunServerValue = stunServer.Text.Trim();
        int runtimePort = RuntimePortFromBind(bind);
        string args = "room-runtime-run"
            + " --room-id " + Quote(roomId)
            + " --peer-id " + Quote(peer)
            + " --virtual-ip " + virtualIp
            + " --bind " + bind
            + " --key " + Quote(roomKey)
            + " --game-ports " + gamePort
            + " --broadcast-ports " + broadcastPort
            + " --duration-ms 3600000"
            + " --peer-timeout-ms 5000"
            + " --packet-io-backend wintun"
            + " --forward-raw-ipv4 true"
            + " --wintun-runtime true"
            + " --heartbeat-interval-ms 500"
            + " --snapshot-out " + Quote(latestRuntimeSnapshot)
            + " --snapshot-interval-ms 1000"
            + " --stop-file " + Quote(runtimeStopFile)
            + " --nat-bootstrap-attempts 60"
            + " --nat-bootstrap-interval-ms 80"
            + " --nat-bootstrap-timeout-ms 12000"
            + RuntimeNatBootstrapStunArgs(stunServerValue)
            + RuntimeNatBootstrapUpnpArgs()
            + RelayCandidateArgs()
            + RuntimeCoordinationPublishArgs(coordinationServerValue)
            + RuntimeCoordinationArgs()
            + RuntimeCoordinationMonitorArgs();
        if (observePath.Length > 0)
        {
            args += " --observe-file " + Quote(observePath);
        }

        runtimeOutput.Length = 0;
        lastRuntimeLogLength = 0;
        runtimeProcess = StartNativeRuntimeProcess(args);
        if (runtimeProcess == null)
        {
            return;
        }
        output.Text = T("runtimeStarted")
            + Environment.NewLine + RuntimeStatusText()
            + Environment.NewLine + "UDP bind: " + bind
            + Environment.NewLine + T("runtimeSnapshotPath") + latestRuntimeSnapshot
            + Environment.NewLine + T("runtimeObservationPath") + observePath;
        string directPeerId;
        string directVirtualIp;
        string directOffer;
        if (TryParseRemotePeerOfferSpec(remotePeer.Text.Trim(), out directPeerId, out directVirtualIp, out directOffer))
        {
            output.Text += Environment.NewLine + T("directBootstrapStarting") + " " + directPeerId + " @ " + directVirtualIp;
        }
        else
        {
            string coordinationPeerId;
            string coordinationVirtualIp;
            if (TryParseCoordinationPeerSpec(remotePeer.Text.Trim(), out coordinationPeerId, out coordinationVirtualIp)
                && coordinationServer.Text.Trim().Length > 0)
            {
                output.Text += Environment.NewLine + T("coordinationBootstrapStarting") + " " + coordinationPeerId + " @ " + coordinationVirtualIp;
            }
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
        heartbeatPulseActive = false;
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
        roomUiMode = hostRuntimePeerId.Length > 0 && RuntimePeerId() == hostRuntimePeerId ? "host" : "joined";
        UpdateHomeActionButtons();
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
            string elevatedText = RunNativeCliElevated(args, "runtime-cleanup-apply");
            UpdateRoomDetailsFromRuntimeCleanupApply(elevatedText);
            return;
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
        string spec;
        string result = CreateDirectOfferSpec(peer, out spec);
        if (result.Length == 0 || spec.Length == 0)
        {
            output.Text = DirectOfferFailureText(result);
            return;
        }
        bool copied = TryCopyToClipboard(spec);
        output.Text = T("directOfferReady")
            + Environment.NewLine
            + T(copied ? "directOfferCopied" : "directOfferCopyFailed")
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

    void CopyDirectCode()
    {
        string peer = RuntimePeerId();
        string spec;
        string result = CreateDirectOfferSpec(peer, out spec);
        if (result.Length == 0 || spec.Length == 0)
        {
            output.Text = DirectOfferFailureText(result);
            return;
        }
        bool copied = TryCopyToClipboard(spec);
        output.Text = T(copied ? "directCodeCopied" : "directCodeCopyFailed")
            + Environment.NewLine
            + StunMappingText(result)
            + Environment.NewLine
            + T("directCodeNext");
        if (!copied)
        {
            output.Text += Environment.NewLine
                + Environment.NewLine
                + spec;
        }
    }

    string CreateDirectOfferSpec(string peer, out string spec)
    {
        spec = "";
        string result = CreateNativeOffer(peer, false);
        if (result.Length == 0 || latestNativeOfferFile.Length == 0 || !File.Exists(latestNativeOfferFile))
        {
            return "";
        }
        string offer = CompactJson(File.ReadAllText(latestNativeOfferFile, Encoding.UTF8).Trim());
        if (OfferCandidateCount(offer) == 0)
        {
            return result;
        }
        spec = peer + "," + ip.Text.Trim() + "," + offer;
        return result;
    }

    int OfferCandidateCount(string offer)
    {
        string candidates = JsonArrayValue(offer, "candidates");
        if (candidates.Length == 0) return 0;
        int count = 0;
        int search = 0;
        while (search < candidates.Length)
        {
            int start = candidates.IndexOf('{', search);
            if (start < 0) break;
            int end = MatchingJsonBrace(candidates, start);
            if (end < 0) break;
            count++;
            search = end + 1;
        }
        return count;
    }

    string DirectOfferFailureText(string result)
    {
        string text = T("directOfferFailed");
        string trimmed = result.Trim();
        if (trimmed.StartsWith("{", StringComparison.Ordinal))
        {
            string offer = JsonObjectValue(trimmed, "offer");
            string candidateCount = offer.Length > 0 ? OfferCandidateCount(CompactJson(offer)).ToString(CultureInfo.InvariantCulture) : "0";
            text += Environment.NewLine
                + T("directCandidateCount") + " " + candidateCount
                + Environment.NewLine
                + StunMappingText(trimmed)
                + Environment.NewLine
                + UpnpMappingText(trimmed);
            if (candidateCount == "0")
            {
                text += Environment.NewLine
                    + T("directNoCandidatesHint");
            }
        }
        else if (trimmed.Length > 0)
        {
            text += Environment.NewLine + Environment.NewLine + trimmed;
        }
        text += Environment.NewLine
            + T("directOfferFailureHint")
            + " bind=" + NativeRuntimeBind()
            + ", peer=" + RuntimePeerId()
            + ", ip=" + ip.Text.Trim();
        return text;
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
            + " --punch-attempts 24"
            + " --punch-interval-ms 100"
            + " --handshake-timeout-ms 30000"
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
        string firewallOutput = "";
        if (ConfirmAdminAction("coordinationFirewallConfirm", "startCoordination"))
        {
            firewallOutput = RunNativeCliElevated(
                "firewall-apply --game-name " + Quote("LocalAreaInterconnection Coordination")
                    + " --subnet " + subnet.Text
                    + " --ports " + CoordinationPort()
                    + " --remote-scope any"
                    + " --yes true",
                "coordination-firewall");
        }
        else
        {
            firewallOutput = T("coordinationFirewallSkipped");
        }
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

    bool EnsureLocalCoordinationServerForRoom()
    {
        if (coordinationServer.Text.Trim().Length == 0)
        {
            coordinationServer.Text = LocalCoordinationEndpoint();
        }
        if (coordinationProcess != null && !coordinationProcess.HasExited)
        {
            return true;
        }
        StartLocalCoordinationServer();
        return coordinationProcess != null && !coordinationProcess.HasExited;
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

    string ConfigureRemotePeerFromCoordinationNow()
    {
        string currentSubnet = subnet.Text.Trim();
        string server = coordinationServer.Text.Trim();
        if (currentSubnet.Length == 0 || server.Length == 0)
        {
            return "";
        }
        string peer = RuntimePeerId();
        string arguments = "coordination-http-room-view"
            + " --server " + Quote(server)
            + " --room-id " + Quote(RuntimeRoomId())
            + " --peer-id " + Quote(peer)
            + " --subnet " + Quote(currentSubnet);
        try
        {
            string text = RunNativeCliCapture(arguments);
            UpdateRoomDetailsFromCoordinationView(text);
            AutoConfigureRemotePeerFromCoordinationView(text);
            if (remotePeer.Text.Trim().Length == 0)
            {
                return T("coordinationWaitingForPeer") + Environment.NewLine + text;
            }
            return text;
        }
        catch (Exception ex)
        {
            return T("coordinationFetchFailed") + Environment.NewLine + ex.Message;
        }
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

