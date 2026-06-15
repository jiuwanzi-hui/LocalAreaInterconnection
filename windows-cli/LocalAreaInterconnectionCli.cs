using System;
using System.Collections.Generic;
using System.IO;
using System.Net;
using System.Net.NetworkInformation;
using System.Net.Sockets;
using System.Security.Cryptography;
using System.Text;

public static class LocalAreaInterconnectionCli
{
    public static int Main(string[] argv)
    {
        try
        {
            if (argv.Length == 0 || argv[0] == "help" || argv[0] == "--help")
            {
                Usage();
                return 0;
            }

            string command = argv[0];
            Dictionary<string, string> args = ParseArgs(argv);

            if (command == "init")
            {
                Init(args);
            }
            else if (command == "decode")
            {
                Decode(args);
            }
            else if (command == "join")
            {
                Join(args);
            }
            else if (command == "game-plan")
            {
                GamePlan(args);
            }
            else if (command == "diagnose")
            {
                Diagnose(args);
            }
            else if (command == "network-observe")
            {
                NetworkObserve(args);
            }
            else if (command == "diagnostic-export")
            {
                DiagnosticExport(args);
            }
            else if (command == "firewall-plan")
            {
                FirewallPlan(args);
            }
            else if (command == "firewall-apply")
            {
                FirewallApply(args);
            }
            else if (command == "firewall-remove")
            {
                FirewallRemove(args);
            }
            else if (command == "firewall-diagnose")
            {
                FirewallDiagnose(args);
            }
            else if (command == "firewall-scan")
            {
                FirewallScan(args);
            }
            else if (command == "adapter-plan")
            {
                AdapterPlan(args);
            }
            else if (command == "adapter-apply")
            {
                AdapterApply(args);
            }
            else if (command == "adapter-diagnose")
            {
                AdapterDiagnose(args);
            }
            else if (command == "adapter-scan")
            {
                AdapterScan(args);
            }
            else if (command == "udp-loopback-test")
            {
                UdpLoopbackTest(args);
            }
            else if (command == "udp-listen")
            {
                UdpListen(args);
            }
            else if (command == "udp-send")
            {
                UdpSend(args);
            }
            else if (command == "udp-broadcast-test")
            {
                UdpBroadcastTest(args);
            }
            else if (command == "tcp-loopback-test")
            {
                TcpLoopbackTest(args);
            }
            else
            {
                Usage();
                return 1;
            }
            return 0;
        }
        catch (Exception ex)
        {
            Console.Error.WriteLine("error: " + ex.Message);
            return 1;
        }
    }

    static void Usage()
    {
        Console.WriteLine("LocalAreaInterconnection Windows CLI");
        Console.WriteLine("Usage:");
        Console.WriteLine("  LocalAreaInterconnection.Cli.exe init --room-name \"Friday LAN\" --host Alice [--coordination-endpoint http://192.168.1.10:39110]");
        Console.WriteLine("  LocalAreaInterconnection.Cli.exe decode --invite <code>");
        Console.WriteLine("  LocalAreaInterconnection.Cli.exe join --invite <code> [--peer Bob]");
        Console.WriteLine("  LocalAreaInterconnection.Cli.exe diagnose --virtual-adapter ok --firewall allowed --p2p ok --broadcast missing");
        Console.WriteLine("  LocalAreaInterconnection.Cli.exe network-observe --adapter-name LocalAreaInterconnection --expected-ip 10.77.12.2 --assigned-ip 10.77.12.2 --subnet 10.77.12.0/24 --connected-peers 1 --expected-peers 1 --broadcast-ports 27015 --game-ports 27015 --packets udp:10.77.12.2:10.77.12.255:27015:broadcast:outbound:8,udp:10.77.12.2:10.77.12.3:27015:unicast:outbound:8");
        Console.WriteLine("  LocalAreaInterconnection.Cli.exe network-observe --adapter-name LocalAreaInterconnection --expected-ip 10.77.12.2 --assigned-ip 10.77.12.2 --subnet 10.77.12.0/24 --packet-observations packets.txt --broadcast-ports 27015 --game-ports 27015");
        Console.WriteLine("  LocalAreaInterconnection.Cli.exe network-observe --adapter-name LocalAreaInterconnection --expected-ip 10.77.12.2 --subnet 10.77.12.0/24 --adapter-netsh-output adapter.txt");
        Console.WriteLine("  LocalAreaInterconnection.Cli.exe network-observe --adapter-name LocalAreaInterconnection --expected-ip 10.77.12.2 --subnet 10.77.12.0/24 --adapter-scan true");
        Console.WriteLine("  LocalAreaInterconnection.Cli.exe network-observe --ping-test 127.0.0.1 --expected-peers 1");
        Console.WriteLine("  LocalAreaInterconnection.Cli.exe network-observe --ping-output ping.txt --expected-peers 1");
        Console.WriteLine("  LocalAreaInterconnection.Cli.exe diagnostic-export --out diagnostic-bundle.json --adapter-name LocalAreaInterconnection --expected-ip 10.77.12.2 --subnet 10.77.12.0/24 --ping-test 127.0.0.1 --packet-observations packets.txt --broadcast-ports 39078 --game-ports 39077 --game-name \"Example Game\" --ports 39077,39078");
        Console.WriteLine("  LocalAreaInterconnection.Cli.exe game-plan --game-name \"Example Game\" --subnet 10.77.12.0/24 --ports 27015,27016");
        Console.WriteLine("  LocalAreaInterconnection.Cli.exe firewall-plan --game-name \"Example Game\" --subnet 10.77.12.0/24 --ports 27015");
        Console.WriteLine("  LocalAreaInterconnection.Cli.exe firewall-apply --game-name \"Example Game\" --subnet 10.77.12.0/24 --ports 27015 --yes true");
        Console.WriteLine("  LocalAreaInterconnection.Cli.exe firewall-remove --game-name \"Example Game\" --ports 27015 --yes true");
        Console.WriteLine("  LocalAreaInterconnection.Cli.exe firewall-diagnose --game-name \"Example Game\" --subnet 10.77.12.0/24 --ports 27015 --observed udp:27015");
        Console.WriteLine("  LocalAreaInterconnection.Cli.exe firewall-diagnose --game-name \"Example Game\" --subnet 10.77.12.0/24 --ports 27015 --netsh-output firewall-rules.txt");
        Console.WriteLine("  LocalAreaInterconnection.Cli.exe firewall-scan --game-name \"Example Game\" --subnet 10.77.12.0/24 --ports 27015");
        Console.WriteLine("  LocalAreaInterconnection.Cli.exe adapter-plan --subnet 10.77.12.0/24 --ip 10.77.12.2");
        Console.WriteLine("  LocalAreaInterconnection.Cli.exe adapter-apply --adapter-name LocalAreaInterconnection --subnet 10.77.12.0/24 --ip 10.77.12.2 --yes true");
        Console.WriteLine("  LocalAreaInterconnection.Cli.exe adapter-diagnose --subnet 10.77.12.0/24 --ip 10.77.12.2 --netsh-output adapter.txt");
        Console.WriteLine("  LocalAreaInterconnection.Cli.exe adapter-scan --adapter-name LocalAreaInterconnection --subnet 10.77.12.0/24 --ip 10.77.12.2");
        Console.WriteLine("  LocalAreaInterconnection.Cli.exe udp-loopback-test --port 39077 --message ping --observe-file packets.txt");
        Console.WriteLine("  LocalAreaInterconnection.Cli.exe udp-broadcast-test --port 39078 --message discover --observe-file packets.txt");
        Console.WriteLine("  LocalAreaInterconnection.Cli.exe udp-listen --port 39077 --timeout-ms 10000 --observe-file packets.txt");
        Console.WriteLine("  LocalAreaInterconnection.Cli.exe udp-send --host 127.0.0.1 --port 39077 --message ping --observe-file packets.txt");
        Console.WriteLine("  LocalAreaInterconnection.Cli.exe tcp-loopback-test --port 39079 --message ping --observe-file packets.txt");
    }

    static Dictionary<string, string> ParseArgs(string[] argv)
    {
        Dictionary<string, string> args = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
        for (int i = 1; i < argv.Length; i++)
        {
            string token = argv[i];
            if (!token.StartsWith("--"))
            {
                continue;
            }
            string key = token.Substring(2);
            string value = "true";
            if (i + 1 < argv.Length && !argv[i + 1].StartsWith("--"))
            {
                value = argv[i + 1];
                i++;
            }
            args[key] = value;
        }
        return args;
    }

    static string Arg(Dictionary<string, string> args, string key, string fallback)
    {
        string value;
        return args.TryGetValue(key, out value) ? value : fallback;
    }

    static void Init(Dictionary<string, string> args)
    {
        string roomName = Arg(args, "room-name", "LAN Room");
        string host = Arg(args, "host", "Host");
        string coordinationEndpoint = Arg(args, "coordination-endpoint", "");
        string roomId = RandomToken(8);
        string roomKey = RandomToken(32);
        string subnet = SubnetForRoom(roomId);
        string hostIp = HostIp(subnet);
        string broadcastIp = BroadcastIp(subnet);
        string hostPeerId = "peer_" + RandomToken(6);
        string payload = Obj(
            Prop("version", "1"),
            Prop("room_id", Q(roomId)),
            Prop("room_name", Q(roomName)),
            Prop("mode", Q("p2p")),
            Prop("virtual_subnet", Q(subnet)),
            Prop("host_peer_id", Q(hostPeerId)),
            Prop("host_endpoint", "null"),
            Prop("coordination_endpoint", coordinationEndpoint.Length > 0 ? Q(coordinationEndpoint) : "null"),
            Prop("join_token", Q(RandomToken(18)))
        );
        string invite = Base64Url(Encoding.UTF8.GetBytes(payload)) + "." + Base64Url(SHA256.Create().ComputeHash(Encoding.UTF8.GetBytes(payload + roomKey)));
        string room = Obj(
            Prop("roomId", Q(roomId)),
            Prop("roomName", Q(roomName)),
            Prop("hostName", Q(host)),
            Prop("hostPeerId", Q(hostPeerId)),
            Prop("virtualSubnet", Q(subnet)),
            Prop("hostIp", Q(hostIp)),
            Prop("broadcastAddress", Q(broadcastIp)),
            Prop("roomKey", Q(roomKey))
        );
        Console.WriteLine(Obj(Prop("room", room), Prop("invite", Q(invite))));
    }

    static void Decode(Dictionary<string, string> args)
    {
        string invite = Required(args, "invite");
        string payload = DecodeInvitePayload(invite);
        Console.WriteLine(Obj(Prop("payload", payload)));
    }

    static void Join(Dictionary<string, string> args)
    {
        string invite = Required(args, "invite");
        string peer = Arg(args, "peer", "Player");
        string payload = DecodeInvitePayload(invite);
        string subnet = JsonStringValue(payload, "virtual_subnet");
        string roomId = JsonStringValue(payload, "room_id");
        string roomName = JsonStringValue(payload, "room_name");
        string hostPeerId = JsonStringValue(payload, "host_peer_id");
        string coordinationEndpoint = JsonStringValue(payload, "coordination_endpoint");
        Console.WriteLine(Obj(
            Prop("roomId", Q(roomId)),
            Prop("roomName", Q(roomName)),
            Prop("peerName", Q(peer)),
            Prop("virtualSubnet", Q(subnet)),
            Prop("hostPeerId", Q(hostPeerId)),
            Prop("hostIp", Q(HostIp(subnet))),
            Prop("suggestedLocalIp", Q(PeerIp(subnet, 0))),
            Prop("coordinationEndpoint", coordinationEndpoint.Length > 0 ? Q(coordinationEndpoint) : "null"),
            Prop("nextAction", Q("Enable the virtual adapter, assign the suggested IP, then test connectivity with the host."))
        ));
    }

    static void GamePlan(Dictionary<string, string> args)
    {
        string game = Arg(args, "game-name", "Generic LAN Game");
        string subnet = Required(args, "subnet");
        string discovery = Arg(args, "discovery", "udp_broadcast");
        string compatibility = Arg(args, "compatibility", "unknown");
        int[] ports = ParsePorts(Arg(args, "ports", ""));
        bool broadcastEnabled = discovery != "direct_ip" && compatibility != "D";
        Console.WriteLine(Obj(
            Prop("gameName", Q(game)),
            Prop("compatibility", Q(compatibility)),
            Prop("virtualSubnet", Q(subnet)),
            Prop("broadcastAddress", Q(BroadcastIp(subnet))),
            Prop("firewallRules", FirewallRulesJson(game, subnet, ports)),
            Prop("broadcast", Obj(
                Prop("enabled", broadcastEnabled ? "true" : "false"),
                Prop("allowedPorts", IntArray(ports))
            )),
            Prop("diagnosticChecks", DiagnosticChecksJson(ports.Length > 0, broadcastEnabled)),
            Prop("warnings", WarningsJson(ports, discovery, compatibility))
        ));
    }

    static void Diagnose(Dictionary<string, string> args)
    {
        string[] keys = new string[] { "virtual-adapter", "firewall", "tunnel", "p2p", "broadcast", "direct-ip", "game-traffic" };
        List<string> problems = new List<string>();
        for (int i = 0; i < keys.Length; i++)
        {
            string value = Arg(args, keys[i], "unknown");
            if (value == "unknown") continue;
            if (!DiagnosticHealthy(keys[i], value))
            {
                problems.Add(Obj(
                    Prop("key", Q(keys[i])),
                    Prop("value", Q(value)),
                    Prop("message", Q(DiagnosticMessage(keys[i]))),
                    Prop("nextAction", Q(DiagnosticNextAction(keys[i])))
                ));
            }
        }

        Console.WriteLine(Obj(
            Prop("status", Q(problems.Count == 0 ? "healthy" : "needs-attention")),
            Prop("summary", Q(problems.Count == 0 ? "Connectivity indicators look healthy." : "Detected " + problems.Count + " problem(s).")),
            Prop("problems", Arr(problems.ToArray()))
        ));
    }

    static void NetworkObserve(Dictionary<string, string> args)
    {
        Console.WriteLine(NetworkObserveJson(args));
    }

    static string NetworkObserveJson(Dictionary<string, string> args)
    {
        LoadAdapterObservationArgs(args);
        LoadTunnelObservationArgs(args);
        string adapterStatus = NetworkAdapterStatus(args);
        string tunnelStatus = NetworkTunnelStatus(args);
        string p2pStatus = NetworkP2pStatus(args);
        int[] broadcastPorts = ParsePorts(Arg(args, "broadcast-ports", ""));
        int[] gamePorts = ParsePorts(Arg(args, "game-ports", ""));
        string packets = CombinedPacketObservations(args);
        string broadcastStatus = PacketObservationCount(packets, true, broadcastPorts) > 0 ? "seen" : "missing";
        string gameTrafficStatus = PacketObservationCount(packets, false, gamePorts) > 0 ? "seen" : "missing";

        List<string> checks = new List<string>();
        int observationProblems = 0;
        AddNetworkObservationCheck(checks, ref observationProblems, "adapter", adapterStatus, "Virtual adapter observation is healthy.", "Inspect virtual adapter installation, enabled state, and assigned room IP.");
        AddNetworkObservationCheck(checks, ref observationProblems, "tunnel", tunnelStatus, "Tunnel observation is healthy.", "Reconnect the tunnel, switch networks, or retry coordination.");
        AddNetworkObservationCheck(checks, ref observationProblems, "p2p", p2pStatus, "Expected peers are connected.", "Run NAT diagnostics and try port forwarding, network switching, or relay fallback.");
        AddNetworkObservationCheck(checks, ref observationProblems, "broadcast", broadcastStatus, "Broadcast packets were observed.", "Check broadcast proxy rules and the game discovery port.");
        AddNetworkObservationCheck(checks, ref observationProblems, "game-traffic", gameTrafficStatus, "Game traffic packets were observed.", "Check whether the game is using the virtual adapter and expected ports.");

        string diagnosticSnapshot = Obj(
            Prop("virtual_adapter", Q(adapterStatus)),
            Prop("tunnel", Q(tunnelStatus)),
            Prop("p2p", Q(p2pStatus)),
            Prop("broadcast", Q(broadcastStatus)),
            Prop("game_traffic", Q(gameTrafficStatus))
        );
        string diagnosticReport = DiagnosticReportFromValues(adapterStatus, tunnelStatus, p2pStatus, broadcastStatus, gameTrafficStatus);
        bool healthy = observationProblems == 0 && DiagnosticProblemCount(adapterStatus, tunnelStatus, p2pStatus, broadcastStatus, gameTrafficStatus) == 0;

        return Obj(
            Prop("status", Q(healthy ? "ok" : "needs-attention")),
            Prop("summary", Q(healthy ? "Network experiment observations look healthy." : "Detected " + observationProblems + " network observation problem(s).")),
            Prop("adapterObservation", AdapterObservationJson(args)),
            Prop("tunnelObservation", TunnelObservationJson(args)),
            Prop("diagnosticSnapshot", diagnosticSnapshot),
            Prop("diagnosticReport", diagnosticReport),
            Prop("checks", Arr(checks.ToArray()))
        );
    }

    static void DiagnosticExport(Dictionary<string, string> args)
    {
        string path = Required(args, "out");
        if (!args.ContainsKey("adapter-name"))
        {
            args["adapter-name"] = "LocalAreaInterconnection";
        }

        string adapterScan = DiagnosticAdapterScanSection(args);
        string firewallScan = DiagnosticFirewallScanSection(args);
        string ping = DiagnosticPingSection(args);
        string packets = DiagnosticPacketSection(args);

        Dictionary<string, string> networkArgs = CloneArgs(args);
        if (!networkArgs.ContainsKey("adapter-scan") && Arg(networkArgs, "adapter-netsh-output", "").Length == 0)
        {
            networkArgs["adapter-scan"] = "true";
        }
        string networkObservation = NetworkObserveJson(networkArgs);

        string bundle = Obj(
            Prop("schemaVersion", "1"),
            Prop("status", Q("created")),
            Prop("createdAtUtc", Q(DateTime.UtcNow.ToString("yyyy-MM-ddTHH:mm:ssZ"))),
            Prop("tool", Q("LocalAreaInterconnection Windows CLI")),
            Prop("environment", DiagnosticEnvironmentJson()),
            Prop("inputs", DiagnosticInputsJson(args)),
            Prop("adapterScan", adapterScan),
            Prop("firewallScan", firewallScan),
            Prop("ping", ping),
            Prop("packetObservations", packets),
            Prop("networkObservation", networkObservation),
            Prop("notes", Arr(
                Q("This bundle is read-only and does not modify Windows Firewall or adapter settings."),
                Q("Raw adapter and firewall data may contain local machine configuration; review before sharing publicly.")
            ))
        );

        string directory = Path.GetDirectoryName(Path.GetFullPath(path));
        if (!String.IsNullOrEmpty(directory))
        {
            Directory.CreateDirectory(directory);
        }
        File.WriteAllText(path, bundle + Environment.NewLine, Encoding.UTF8);
        Console.WriteLine(Obj(
            Prop("status", Q("ok")),
            Prop("path", Q(Path.GetFullPath(path))),
            Prop("bytesWritten", new FileInfo(path).Length.ToString())
        ));
    }

    static string DiagnosticEnvironmentJson()
    {
        return Obj(
            Prop("machineName", Q(Environment.MachineName)),
            Prop("userName", Q(Environment.UserName)),
            Prop("osVersion", Q(Environment.OSVersion.ToString())),
            Prop("is64BitOperatingSystem", Environment.Is64BitOperatingSystem ? "true" : "false"),
            Prop("currentDirectory", Q(Environment.CurrentDirectory))
        );
    }

    static string DiagnosticInputsJson(Dictionary<string, string> args)
    {
        List<string> props = new List<string>();
        string[] keys = new string[] {
            "adapter-name", "expected-ip", "assigned-ip", "subnet", "expected-peers",
            "ping-test", "ping-output", "packet-observations", "broadcast-ports",
            "game-ports", "game-name", "ports", "program"
        };
        for (int i = 0; i < keys.Length; i++)
        {
            if (args.ContainsKey(keys[i]))
            {
                props.Add(Prop(ToCamelName(keys[i]), Q(args[keys[i]])));
            }
        }
        return Obj(props.ToArray());
    }

    static string DiagnosticAdapterScanSection(Dictionary<string, string> args)
    {
        Dictionary<string, string> diagnoseArgs = CloneArgs(args);
        if (!diagnoseArgs.ContainsKey("ip") && diagnoseArgs.ContainsKey("expected-ip"))
        {
            diagnoseArgs["ip"] = diagnoseArgs["expected-ip"];
        }
        string adapter = Arg(args, "adapter-name", "LocalAreaInterconnection");
        string output = "";
        string error = "";
        string source = "netsh-scan";
        string path = Arg(args, "adapter-netsh-output", "");
        if (path.Length > 0)
        {
            source = "netsh-file";
            try
            {
                output = File.ReadAllText(path);
            }
            catch (Exception ex)
            {
                error = ex.Message;
            }
        }
        else
        {
            try
            {
                output = RunProcess("netsh", "interface ipv4 show config name=" + NetshQuote(adapter));
            }
            catch (Exception ex)
            {
                error = ex.Message;
            }
        }

        Dictionary<string, string> parsed = output.Length == 0
            ? new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase)
            : ParseNetshAdapterOutput(output);
        return Obj(
            Prop("status", Q(error.Length == 0 && output.Length > 0 ? "ok" : "needs-attention")),
            Prop("source", Q(source)),
            Prop("adapterName", Q(adapter)),
            Prop("error", Q(error)),
            Prop("parsed", AdapterParsedJson(parsed)),
            Prop("diagnosis", AdapterDiagnoseOrMissingInputsJson(diagnoseArgs, output, error)),
            Prop("rawOutput", Q(output))
        );
    }

    static string AdapterDiagnoseOrMissingInputsJson(Dictionary<string, string> args, string text, string error)
    {
        if (Arg(args, "ip", "").Length == 0 || Arg(args, "subnet", "").Length == 0)
        {
            return Obj(
                Prop("status", Q("needs-attention")),
                Prop("problemCount", "1"),
                Prop("checks", Arr(Obj(
                    Prop("key", Q("adapter-expected-config")),
                    Prop("status", Q("missing-input")),
                    Prop("message", Q("Expected adapter IP and subnet were not provided.")),
                    Prop("nextAction", Q("Run diagnostic-export with --expected-ip and --subnet for adapter diagnosis."))
                )))
            );
        }
        return AdapterDiagnoseJson(args, text, error);
    }

    static string DiagnosticFirewallScanSection(Dictionary<string, string> args)
    {
        string output = "";
        string error = "";
        Dictionary<string, string> observedStatus = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
        string path = Arg(args, "firewall-netsh-output", "");
        string source = path.Length > 0 ? "netsh-file" : "netsh-scan";
        try
        {
            output = path.Length > 0 ? File.ReadAllText(path) : RunProcess("netsh", "advfirewall firewall show rule name=all");
            observedStatus = ParseNetshFirewallOutput(output);
        }
        catch (Exception ex)
        {
            error = ex.Message;
        }

        return Obj(
            Prop("status", Q(error.Length == 0 ? "ok" : "needs-attention")),
            Prop("source", Q(source)),
            Prop("error", Q(error)),
            Prop("diagnosis", FirewallDiagnoseJson(args, observedStatus)),
            Prop("rawOutput", Q(output))
        );
    }

    static string DiagnosticPingSection(Dictionary<string, string> args)
    {
        if (Arg(args, "ping-test", "").Length == 0 && Arg(args, "ping-output", "").Length == 0)
        {
            return Obj(
                Prop("status", Q("skipped")),
                Prop("host", Q("")),
                Prop("observation", "null"),
                Prop("error", Q("No --ping-test or --ping-output was provided."))
            );
        }

        Dictionary<string, string> pingArgs = CloneArgs(args);
        try
        {
            LoadTunnelObservationArgs(pingArgs);
            return Obj(
                Prop("status", Q(Arg(pingArgs, "tunnel-state", "connected") == "connected" ? "ok" : "needs-attention")),
                Prop("host", Q(Arg(pingArgs, "ping-host", Arg(args, "ping-test", "")))),
                Prop("observation", TunnelObservationJson(pingArgs)),
                Prop("error", Q(""))
            );
        }
        catch (Exception ex)
        {
            return Obj(
                Prop("status", Q("needs-attention")),
                Prop("host", Q(Arg(args, "ping-test", ""))),
                Prop("observation", "null"),
                Prop("error", Q(ex.Message))
            );
        }
    }

    static string DiagnosticPacketSection(Dictionary<string, string> args)
    {
        string packets = "";
        string error = "";
        try
        {
            packets = CombinedPacketObservations(args);
        }
        catch (Exception ex)
        {
            error = ex.Message;
        }
        int[] broadcastPorts = ParsePorts(Arg(args, "broadcast-ports", ""));
        int[] gamePorts = ParsePorts(Arg(args, "game-ports", ""));
        return Obj(
            Prop("status", Q(error.Length == 0 ? "ok" : "needs-attention")),
            Prop("sourceFile", Q(Arg(args, "packet-observations", ""))),
            Prop("broadcastCount", PacketObservationCount(packets, true, broadcastPorts).ToString()),
            Prop("gameTrafficCount", PacketObservationCount(packets, false, gamePorts).ToString()),
            Prop("rawLines", PacketLinesJson(packets)),
            Prop("error", Q(error))
        );
    }

    static string AdapterParsedJson(Dictionary<string, string> parsed)
    {
        return Obj(
            Prop("assignedIp", Q(ParsedValue(parsed, "assigned-ip"))),
            Prop("observedSubnet", Q(ParsedValue(parsed, "observed-subnet"))),
            Prop("enabled", Q(ParsedValue(parsed, "enabled"))),
            Prop("mtu", Q(ParsedValue(parsed, "mtu"))),
            Prop("interfaceMetric", Q(ParsedValue(parsed, "metric")))
        );
    }

    static string ParsedValue(Dictionary<string, string> parsed, string key)
    {
        string value;
        return parsed.TryGetValue(key, out value) ? value : "";
    }

    static Dictionary<string, string> CloneArgs(Dictionary<string, string> args)
    {
        Dictionary<string, string> clone = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
        foreach (KeyValuePair<string, string> entry in args)
        {
            clone[entry.Key] = entry.Value;
        }
        return clone;
    }

    static string ToCamelName(string value)
    {
        StringBuilder builder = new StringBuilder();
        bool upperNext = false;
        for (int i = 0; i < value.Length; i++)
        {
            char ch = value[i];
            if (ch == '-')
            {
                upperNext = true;
                continue;
            }
            builder.Append(upperNext ? Char.ToUpperInvariant(ch) : ch);
            upperNext = false;
        }
        return builder.ToString();
    }

    static string PacketLinesJson(string packets)
    {
        if (packets == null || packets.Trim().Length == 0) return Arr();
        string[] items = packets.Split(new char[] { ',' }, StringSplitOptions.RemoveEmptyEntries);
        List<string> lines = new List<string>();
        for (int i = 0; i < items.Length; i++)
        {
            string line = items[i].Trim();
            if (line.Length > 0)
            {
                lines.Add(Q(line));
            }
        }
        return Arr(lines.ToArray());
    }

    static void FirewallPlan(Dictionary<string, string> args)
    {
        string game = Arg(args, "game-name", "Generic LAN Game");
        string subnet = Required(args, "subnet");
        int[] ports = ParsePorts(Arg(args, "ports", ""));
        string program = Arg(args, "program", "");
        string[] commandTexts = FirewallAddCommands(game, subnet, ports, program);
        List<string> commands = new List<string>();
        for (int i = 0; i < commandTexts.Length; i++)
        {
            commands.Add(Obj(Prop("command", Q("netsh " + commandTexts[i]))));
        }
        Console.WriteLine(Obj(
            Prop("platform", Q("windows")),
            Prop("dryRun", "true"),
            Prop("requiresElevation", commands.Count > 0 ? "true" : "false"),
            Prop("commands", Arr(commands.ToArray()))
        ));
    }

    static void FirewallApply(Dictionary<string, string> args)
    {
        string game = Arg(args, "game-name", "Generic LAN Game");
        string subnet = Required(args, "subnet");
        int[] ports = ParsePorts(Arg(args, "ports", ""));
        string program = Arg(args, "program", "");
        string[] commandTexts = FirewallAddCommands(game, subnet, ports, program);
        if (Arg(args, "yes", "false") != "true")
        {
            List<string> dryRun = new List<string>();
            for (int i = 0; i < commandTexts.Length; i++)
            {
                dryRun.Add(Obj(Prop("command", Q("netsh " + commandTexts[i]))));
            }
            Console.WriteLine(Obj(
                Prop("applied", "false"),
                Prop("requiresConfirmation", "true"),
                Prop("message", Q("Re-run with --yes true from an elevated terminal to apply these rules.")),
                Prop("commands", Arr(dryRun.ToArray()))
            ));
            return;
        }

        List<string> results = new List<string>();
        for (int i = 0; i < commandTexts.Length; i++)
        {
            string output = RunProcess("netsh", commandTexts[i]);
            results.Add(Obj(Prop("command", Q("netsh " + commandTexts[i])), Prop("output", Q(output.Trim()))));
        }
        Console.WriteLine(Obj(Prop("applied", "true"), Prop("results", Arr(results.ToArray()))));
    }

    static void FirewallRemove(Dictionary<string, string> args)
    {
        string game = Arg(args, "game-name", "Generic LAN Game");
        int[] ports = ParsePorts(Arg(args, "ports", ""));
        string[] commandTexts = FirewallDeleteCommands(game, ports);
        if (Arg(args, "yes", "false") != "true")
        {
            List<string> dryRun = new List<string>();
            for (int i = 0; i < commandTexts.Length; i++)
            {
                dryRun.Add(Obj(Prop("command", Q("netsh " + commandTexts[i]))));
            }
            Console.WriteLine(Obj(
                Prop("removed", "false"),
                Prop("requiresConfirmation", "true"),
                Prop("message", Q("Re-run with --yes true from an elevated terminal to remove these rules.")),
                Prop("commands", Arr(dryRun.ToArray()))
            ));
            return;
        }

        List<string> results = new List<string>();
        for (int i = 0; i < commandTexts.Length; i++)
        {
            string output = RunProcess("netsh", commandTexts[i]);
            results.Add(Obj(Prop("command", Q("netsh " + commandTexts[i])), Prop("output", Q(output.Trim()))));
        }
        Console.WriteLine(Obj(Prop("removed", "true"), Prop("results", Arr(results.ToArray()))));
    }

    static string[] FirewallAddCommands(string game, string subnet, int[] ports, string program)
    {
        List<string> commands = new List<string>();
        string[] protocols = new string[] { "udp", "tcp" };
        for (int i = 0; i < ports.Length; i++)
        {
            for (int p = 0; p < protocols.Length; p++)
            {
                string protocol = protocols[p];
                string name = game + " " + protocol.ToUpperInvariant() + " " + ports[i];
                string cmd = "advfirewall firewall add rule name=" + NetshQuote(name) + " dir=in action=allow protocol=" + protocol.ToUpperInvariant() + " localport=" + ports[i] + " profile=private remoteip=" + subnet + " group=LocalAreaInterconnection";
                if (program.Length > 0)
                {
                    cmd += " program=" + NetshQuote(program);
                }
                commands.Add(cmd);
            }
        }
        return commands.ToArray();
    }

    static string[] FirewallDeleteCommands(string game, int[] ports)
    {
        List<string> commands = new List<string>();
        string[] protocols = new string[] { "udp", "tcp" };
        for (int i = 0; i < ports.Length; i++)
        {
            for (int p = 0; p < protocols.Length; p++)
            {
                string name = game + " " + protocols[p].ToUpperInvariant() + " " + ports[i];
                commands.Add("advfirewall firewall delete rule name=" + NetshQuote(name));
            }
        }
        return commands.ToArray();
    }

    static void FirewallDiagnose(Dictionary<string, string> args)
    {
        Dictionary<string, string> observedStatus;
        string netshOutput = Arg(args, "netsh-output", "");
        if (netshOutput.Length > 0)
        {
            observedStatus = ParseNetshFirewallOutput(File.ReadAllText(netshOutput));
        }
        else
        {
            observedStatus = ParseObservedFirewallSummary(Arg(args, "observed", ""));
        }
        FirewallDiagnoseFromStatus(args, observedStatus);
    }

    static void FirewallScan(Dictionary<string, string> args)
    {
        string output = RunProcess("netsh", "advfirewall firewall show rule name=all");
        args["netsh-output-content"] = output;
        FirewallDiagnoseFromStatus(args, ParseNetshFirewallOutput(output));
    }

    static void FirewallDiagnoseFromStatus(Dictionary<string, string> args, Dictionary<string, string> observedStatus)
    {
        Console.WriteLine(FirewallDiagnoseJson(args, observedStatus));
    }

    static string FirewallDiagnoseJson(Dictionary<string, string> args, Dictionary<string, string> observedStatus)
    {
        string game = Arg(args, "game-name", "Generic LAN Game");
        int[] ports = ParsePorts(Arg(args, "ports", ""));
        List<string> checks = new List<string>();
        int problems = 0;
        string[] protocols = new string[] { "udp", "tcp" };
        for (int i = 0; i < ports.Length; i++)
        {
            for (int p = 0; p < protocols.Length; p++)
            {
                string key = protocols[p] + ":" + ports[i];
                string status = "missing";
                if (observedStatus.ContainsKey(key))
                {
                    status = observedStatus[key];
                }
                if (status != "present") problems++;
                checks.Add(Obj(
                    Prop("ruleName", Q(game + " " + protocols[p].ToUpperInvariant() + " " + ports[i])),
                    Prop("protocol", Q(protocols[p])),
                    Prop("port", ports[i].ToString()),
                    Prop("status", Q(status)),
                    Prop("nextAction", Q(FirewallNextAction(status)))
                ));
            }
        }
        return Obj(
            Prop("status", Q(problems == 0 ? "ok" : "needs-attention")),
            Prop("problemCount", problems.ToString()),
            Prop("checks", Arr(checks.ToArray()))
        );
    }

    static Dictionary<string, string> ParseObservedFirewallSummary(string observed)
    {
        Dictionary<string, string> result = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
        string[] observedItems = observed.Split(new char[] { ',' }, StringSplitOptions.RemoveEmptyEntries);
        for (int i = 0; i < observedItems.Length; i++)
        {
            result[observedItems[i].Trim()] = "present";
        }
        return result;
    }

    static Dictionary<string, string> ParseNetshFirewallOutput(string text)
    {
        Dictionary<string, string> result = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
        Dictionary<string, string> block = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
        string[] lines = text.Replace("\r\n", "\n").Replace('\r', '\n').Split('\n');
        for (int i = 0; i < lines.Length; i++)
        {
            int colon = lines[i].IndexOf(':');
            if (colon < 0) continue;
            string key = NormalizeNetshKey(lines[i].Substring(0, colon));
            string value = lines[i].Substring(colon + 1).Trim();
            if ((key == "rulename" || key == NormalizeNetshKey("规则名称")) && block.Count > 0)
            {
                AddNetshBlock(result, block);
                block = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
            }
            block[key] = value;
        }
        AddNetshBlock(result, block);
        return result;
    }

    static void AddNetshBlock(Dictionary<string, string> result, Dictionary<string, string> block)
    {
        string protocolValue = NetshBlockValue(block, "protocol", "协议");
        string localPortValue = NetshBlockValue(block, "localport", "本地端口");
        if (protocolValue.Length == 0 || localPortValue.Length == 0) return;
        string protocol = protocolValue.Trim().ToLowerInvariant();
        if (protocol != "udp" && protocol != "tcp") return;
        string status = "present";
        string enabledValue = NetshBlockValue(block, "enabled", "已启用");
        if (enabledValue.Length > 0 && !IsEnabled(enabledValue))
        {
            status = "disabled";
        }
        string[] ports = localPortValue.Split(',');
        for (int i = 0; i < ports.Length; i++)
        {
            int port;
            if (Int32.TryParse(FirstNumber(ports[i]), out port))
            {
                result[protocol + ":" + port] = status;
            }
        }
    }

    static string NetshBlockValue(Dictionary<string, string> block, params string[] keys)
    {
        for (int i = 0; i < keys.Length; i++)
        {
            string normalized = NormalizeNetshKey(keys[i]);
            string value;
            if (block.TryGetValue(normalized, out value))
            {
                return value;
            }
        }
        return "";
    }

    static string NormalizeNetshKey(string value)
    {
        StringBuilder builder = new StringBuilder();
        for (int i = 0; i < value.Length; i++)
        {
            if (!Char.IsWhiteSpace(value[i])) builder.Append(Char.ToLowerInvariant(value[i]));
        }
        return builder.ToString();
    }

    static bool IsEnabled(string value)
    {
        value = value.Trim().ToLowerInvariant();
        return value == "yes" || value == "true" || value == "enabled" || value == "是";
    }

    static string FirewallNextAction(string status)
    {
        if (status == "present") return "No firewall action is needed for this rule.";
        if (status == "disabled") return "Enable the existing Windows Firewall rule or recreate it from the firewall plan.";
        return "Add the inbound allow rule for this game port on the private profile.";
    }

    static string RunProcess(string fileName, string arguments)
    {
        System.Diagnostics.ProcessStartInfo start = new System.Diagnostics.ProcessStartInfo();
        start.FileName = fileName;
        start.Arguments = arguments;
        start.UseShellExecute = false;
        start.RedirectStandardOutput = true;
        start.RedirectStandardError = true;
        start.CreateNoWindow = true;
        using (System.Diagnostics.Process process = System.Diagnostics.Process.Start(start))
        {
            string stdout = process.StandardOutput.ReadToEnd();
            string stderr = process.StandardError.ReadToEnd();
            process.WaitForExit();
            if (process.ExitCode != 0)
            {
                throw new InvalidOperationException(stderr.Length > 0 ? stderr : fileName + " exited with " + process.ExitCode);
            }
            return stdout;
        }
    }

    static string RunProcessCapture(string fileName, string arguments)
    {
        System.Diagnostics.ProcessStartInfo start = new System.Diagnostics.ProcessStartInfo();
        start.FileName = fileName;
        start.Arguments = arguments;
        start.UseShellExecute = false;
        start.RedirectStandardOutput = true;
        start.RedirectStandardError = true;
        start.CreateNoWindow = true;
        using (System.Diagnostics.Process process = System.Diagnostics.Process.Start(start))
        {
            string stdout = process.StandardOutput.ReadToEnd();
            string stderr = process.StandardError.ReadToEnd();
            process.WaitForExit();
            return stdout.Length > 0 ? stdout : stderr;
        }
    }

    static void AdapterPlan(Dictionary<string, string> args)
    {
        string adapter = Arg(args, "adapter-name", "LocalAreaInterconnection");
        string subnet = Required(args, "subnet");
        string ip = Required(args, "ip");
        string mtu = Arg(args, "mtu", "1420");
        string metric = Arg(args, "metric", "5");
        string mask = MaskIp(Prefix(subnet));
        string[] commandTexts = AdapterCommandArgs(adapter, ip, mask, mtu, metric);
        string[] commands = JsonCommands(commandTexts);
        Console.WriteLine(Obj(
            Prop("platform", Q("windows")),
            Prop("dryRun", "true"),
            Prop("adapterName", Q(adapter)),
            Prop("virtualSubnet", Q(subnet)),
            Prop("assignedIp", Q(ip)),
            Prop("subnetMask", Q(mask)),
            Prop("commands", Arr(commands))
        ));
    }

    static void AdapterApply(Dictionary<string, string> args)
    {
        string adapter = Arg(args, "adapter-name", "LocalAreaInterconnection");
        string subnet = Required(args, "subnet");
        string ip = Required(args, "ip");
        string mtu = Arg(args, "mtu", "1420");
        string metric = Arg(args, "metric", "5");
        string mask = MaskIp(Prefix(subnet));
        string[] commandTexts = AdapterCommandArgs(adapter, ip, mask, mtu, metric);
        if (Arg(args, "yes", "false") != "true")
        {
            Console.WriteLine(Obj(
                Prop("applied", "false"),
                Prop("requiresConfirmation", "true"),
                Prop("message", Q("Re-run with --yes true from an elevated terminal to apply adapter configuration.")),
                Prop("commands", Arr(JsonCommands(commandTexts)))
            ));
            return;
        }

        List<string> results = new List<string>();
        for (int i = 0; i < commandTexts.Length; i++)
        {
            string output = RunProcess("netsh", commandTexts[i]);
            results.Add(Obj(Prop("command", Q("netsh " + commandTexts[i])), Prop("output", Q(output.Trim()))));
        }
        Console.WriteLine(Obj(Prop("applied", "true"), Prop("results", Arr(results.ToArray()))));
    }

    static string[] AdapterCommandArgs(string adapter, string ip, string mask, string mtu, string metric)
    {
        return new string[] {
            "interface ipv4 set address name=" + NetshQuote(adapter) + " static " + ip + " " + mask,
            "interface ipv4 set subinterface " + NetshQuote(adapter) + " mtu=" + mtu + " store=persistent",
            "interface ipv4 set interface " + NetshQuote(adapter) + " metric=" + metric,
            "interface ipv4 show config name=" + NetshQuote(adapter)
        };
    }

    static string[] JsonCommands(string[] commandTexts)
    {
        string[] commands = new string[commandTexts.Length];
        for (int i = 0; i < commandTexts.Length; i++)
        {
            commands[i] = Obj(Prop("command", Q("netsh " + commandTexts[i])));
        }
        return commands;
    }

    static void AdapterDiagnose(Dictionary<string, string> args)
    {
        string text = "";
        string path = Arg(args, "netsh-output", "");
        if (path.Length > 0)
        {
            text = File.ReadAllText(path);
        }
        AdapterDiagnoseFromText(args, text, path.Length == 0 ? "missing-output" : "");
    }

    static void AdapterScan(Dictionary<string, string> args)
    {
        string adapter = Arg(args, "adapter-name", "LocalAreaInterconnection");
        string output = "";
        string error = "";
        try
        {
            output = RunProcess("netsh", "interface ipv4 show config name=" + NetshQuote(adapter));
        }
        catch (Exception ex)
        {
            error = ex.Message;
        }
        AdapterDiagnoseFromText(args, output, error);
    }

    static void AdapterDiagnoseFromText(Dictionary<string, string> args, string text, string error)
    {
        Console.WriteLine(AdapterDiagnoseJson(args, text, error));
    }

    static string AdapterDiagnoseJson(Dictionary<string, string> args, string text, string error)
    {
        string expectedIp = Required(args, "ip");
        string subnet = Required(args, "subnet");
        string mask = MaskIp(Prefix(subnet));
        List<string> checks = new List<string>();
        int problems = 0;

        AddAdapterCheck(checks, ref problems, "adapter-readable", error.Length == 0 && text.Length > 0, error.Length == 0 ? "Adapter configuration output is readable." : error, "Check adapter name and whether the virtual adapter exists.");
        AddAdapterCheck(checks, ref problems, "assigned-ip", text.IndexOf(expectedIp, StringComparison.OrdinalIgnoreCase) >= 0, "Adapter output contains expected virtual IP.", "Assign the room virtual IP to the adapter.");
        AddAdapterCheck(checks, ref problems, "subnet-mask", text.IndexOf(mask, StringComparison.OrdinalIgnoreCase) >= 0, "Adapter output contains expected subnet mask.", "Set the adapter subnet mask from the adapter plan.");

        return Obj(
            Prop("status", Q(problems == 0 ? "ok" : "needs-attention")),
            Prop("problemCount", problems.ToString()),
            Prop("checks", Arr(checks.ToArray()))
        );
    }

    static void UdpLoopbackTest(Dictionary<string, string> args)
    {
        int port = Int32.Parse(Arg(args, "port", "39077"));
        string message = Arg(args, "message", "ping");
        byte[] payload = Encoding.UTF8.GetBytes(message);
        DateTime start = DateTime.UtcNow;
        using (UdpClient listener = new UdpClient(new IPEndPoint(IPAddress.Loopback, port)))
        using (UdpClient sender = new UdpClient())
        {
            listener.Client.ReceiveTimeout = Int32.Parse(Arg(args, "timeout-ms", "3000"));
            sender.Send(payload, payload.Length, new IPEndPoint(IPAddress.Loopback, port));
            IPEndPoint remote = new IPEndPoint(IPAddress.Any, 0);
            byte[] received = listener.Receive(ref remote);
            double elapsed = (DateTime.UtcNow - start).TotalMilliseconds;
            string receivedText = Encoding.UTF8.GetString(received);
            AppendPacketObservation(args, "udp", "127.0.0.1", "127.0.0.1", port, false, "inbound", received.Length);
            Console.WriteLine(Obj(
                Prop("status", Q(receivedText == message ? "ok" : "mismatch")),
                Prop("protocol", Q("udp")),
                Prop("localAddress", Q("127.0.0.1")),
                Prop("port", port.ToString()),
                Prop("bytesReceived", received.Length.ToString()),
                Prop("elapsedMs", ((int)elapsed).ToString()),
                Prop("message", Q(receivedText))
            ));
        }
    }

    static void UdpListen(Dictionary<string, string> args)
    {
        int port = Int32.Parse(Arg(args, "port", "39077"));
        int timeout = Int32.Parse(Arg(args, "timeout-ms", "10000"));
        DateTime start = DateTime.UtcNow;
        using (UdpClient listener = new UdpClient(port))
        {
            listener.Client.ReceiveTimeout = timeout;
            IPEndPoint remote = new IPEndPoint(IPAddress.Any, 0);
            byte[] received = listener.Receive(ref remote);
            double elapsed = (DateTime.UtcNow - start).TotalMilliseconds;
            AppendPacketObservation(args, "udp", RemoteIp(remote), "0.0.0.0", port, false, "inbound", received.Length);
            Console.WriteLine(Obj(
                Prop("status", Q("received")),
                Prop("protocol", Q("udp")),
                Prop("localPort", port.ToString()),
                Prop("remote", Q(remote.ToString())),
                Prop("bytesReceived", received.Length.ToString()),
                Prop("elapsedMs", ((int)elapsed).ToString()),
                Prop("message", Q(Encoding.UTF8.GetString(received)))
            ));
        }
    }

    static void UdpSend(Dictionary<string, string> args)
    {
        string host = Required(args, "host");
        int port = Int32.Parse(Arg(args, "port", "39077"));
        string message = Arg(args, "message", "ping");
        byte[] payload = Encoding.UTF8.GetBytes(message);
        using (UdpClient sender = new UdpClient())
        {
            sender.Send(payload, payload.Length, host, port);
        }
        AppendPacketObservation(args, "udp", "0.0.0.0", ResolveObservationIp(host), port, IsBroadcastHost(host), "outbound", payload.Length);
        Console.WriteLine(Obj(
            Prop("status", Q("sent")),
            Prop("protocol", Q("udp")),
            Prop("host", Q(host)),
            Prop("port", port.ToString()),
            Prop("bytesSent", payload.Length.ToString()),
            Prop("message", Q(message))
        ));
    }

    static void UdpBroadcastTest(Dictionary<string, string> args)
    {
        int port = Int32.Parse(Arg(args, "port", "39078"));
        string message = Arg(args, "message", "discover");
        int timeout = Int32.Parse(Arg(args, "timeout-ms", "3000"));
        byte[] payload = Encoding.UTF8.GetBytes(message);
        DateTime start = DateTime.UtcNow;
        using (UdpClient listener = new UdpClient(new IPEndPoint(IPAddress.Any, port)))
        using (UdpClient sender = new UdpClient())
        {
            listener.EnableBroadcast = true;
            listener.Client.ReceiveTimeout = timeout;
            sender.EnableBroadcast = true;
            sender.Send(payload, payload.Length, new IPEndPoint(IPAddress.Broadcast, port));
            IPEndPoint remote = new IPEndPoint(IPAddress.Any, 0);
            byte[] received = listener.Receive(ref remote);
            double elapsed = (DateTime.UtcNow - start).TotalMilliseconds;
            string receivedText = Encoding.UTF8.GetString(received);
            AppendPacketObservation(args, "udp", RemoteIp(remote), "255.255.255.255", port, true, "inbound", received.Length);
            Console.WriteLine(Obj(
                Prop("status", Q(receivedText == message ? "ok" : "mismatch")),
                Prop("protocol", Q("udp")),
                Prop("broadcastAddress", Q("255.255.255.255")),
                Prop("port", port.ToString()),
                Prop("remote", Q(remote.ToString())),
                Prop("bytesReceived", received.Length.ToString()),
                Prop("elapsedMs", ((int)elapsed).ToString()),
                Prop("message", Q(receivedText))
            ));
        }
    }

    static void TcpLoopbackTest(Dictionary<string, string> args)
    {
        int port = Int32.Parse(Arg(args, "port", "39079"));
        string message = Arg(args, "message", "ping");
        byte[] payload = Encoding.UTF8.GetBytes(message);
        DateTime start = DateTime.UtcNow;
        TcpListener listener = new TcpListener(IPAddress.Loopback, port);
        listener.Start();
        try
        {
            using (TcpClient client = new TcpClient())
            {
                client.ReceiveTimeout = Int32.Parse(Arg(args, "timeout-ms", "3000"));
                client.SendTimeout = Int32.Parse(Arg(args, "timeout-ms", "3000"));
                client.Connect(IPAddress.Loopback, port);
                using (NetworkStream clientStream = client.GetStream())
                {
                    clientStream.Write(payload, 0, payload.Length);
                }
            }

            using (TcpClient accepted = listener.AcceptTcpClient())
            {
                accepted.ReceiveTimeout = Int32.Parse(Arg(args, "timeout-ms", "3000"));
                byte[] buffer = new byte[4096];
                int received = accepted.GetStream().Read(buffer, 0, buffer.Length);
                string receivedText = Encoding.UTF8.GetString(buffer, 0, received);
                double elapsed = (DateTime.UtcNow - start).TotalMilliseconds;
                AppendPacketObservation(args, "tcp", "127.0.0.1", "127.0.0.1", port, false, "inbound", received);
                Console.WriteLine(Obj(
                    Prop("status", Q(receivedText == message ? "ok" : "mismatch")),
                    Prop("protocol", Q("tcp")),
                    Prop("localAddress", Q("127.0.0.1")),
                    Prop("port", port.ToString()),
                    Prop("bytesReceived", received.ToString()),
                    Prop("elapsedMs", ((int)elapsed).ToString()),
                    Prop("message", Q(receivedText))
                ));
            }
        }
        finally
        {
            listener.Stop();
        }
    }

    static void AddAdapterCheck(List<string> checks, ref int problems, string key, bool ok, string message, string nextAction)
    {
        if (!ok) problems++;
        checks.Add(Obj(
            Prop("key", Q(key)),
            Prop("status", Q(ok ? "ok" : "failed")),
            Prop("message", Q(message)),
            Prop("nextAction", Q(ok ? "No action needed." : nextAction))
        ));
    }

    static string NetworkAdapterStatus(Dictionary<string, string> args)
    {
        string adapter = Arg(args, "adapter-name", "");
        if (adapter.Length == 0) return "missing";
        if (args.ContainsKey("adapter-scan-error")) return "missing";
        if (!BoolArg(args, "adapter-enabled", true)) return "disabled";

        string expectedIp = Arg(args, "expected-ip", "");
        string assignedIp = Arg(args, "assigned-ip", "");
        if (expectedIp.Length > 0 && assignedIp != expectedIp) return "ip-mismatch";

        string subnet = Arg(args, "subnet", "");
        if (subnet.Length > 0 && assignedIp.Length > 0 && !CidrContains(subnet, assignedIp)) return "ip-outside-subnet";
        return "ok";
    }

    static void LoadAdapterObservationArgs(Dictionary<string, string> args)
    {
        if (!args.ContainsKey("adapter-name"))
        {
            args["adapter-name"] = "LocalAreaInterconnection";
        }
        string text = "";
        string path = Arg(args, "adapter-netsh-output", "");
        if (path.Length > 0)
        {
            text = File.ReadAllText(path);
        }
        else if (BoolArg(args, "adapter-scan", false))
        {
            string adapter = Arg(args, "adapter-name", "LocalAreaInterconnection");
            try
            {
                text = RunProcess("netsh", "interface ipv4 show config name=" + NetshQuote(adapter));
            }
            catch (Exception ex)
            {
                args["adapter-scan-error"] = ex.Message;
            }
        }

        if (text.Length == 0) return;
        Dictionary<string, string> parsed = ParseNetshAdapterOutput(text);
        args["adapter-netsh-readable"] = "true";
        if (!args.ContainsKey("assigned-ip") && parsed.ContainsKey("assigned-ip"))
        {
            args["assigned-ip"] = parsed["assigned-ip"];
        }
        if (!args.ContainsKey("adapter-enabled") && parsed.ContainsKey("enabled"))
        {
            args["adapter-enabled"] = parsed["enabled"];
        }
        if (parsed.ContainsKey("mtu")) args["adapter-mtu"] = parsed["mtu"];
        if (parsed.ContainsKey("metric")) args["adapter-metric"] = parsed["metric"];
        if (parsed.ContainsKey("observed-subnet")) args["observed-subnet"] = parsed["observed-subnet"];
    }

    static Dictionary<string, string> ParseNetshAdapterOutput(string text)
    {
        Dictionary<string, string> result = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
        string[] lines = text.Replace("\r\n", "\n").Replace('\r', '\n').Split('\n');
        for (int i = 0; i < lines.Length; i++)
        {
            int colon = lines[i].IndexOf(':');
            if (colon < 0) continue;
            string key = NormalizeNetshAdapterKey(lines[i].Substring(0, colon));
            string value = lines[i].Substring(colon + 1).Trim();
            if (key == "ipaddress" || key == "ipaddresses")
            {
                string ip = FirstIpv4(value);
                if (ip.Length > 0) result["assigned-ip"] = ip;
            }
            else if (key == "subnetprefix" || key == "subnetmask")
            {
                string subnet = FirstCidr(value);
                if (subnet.Length > 0) result["observed-subnet"] = subnet;
            }
            else if (key == "interfacemetric" || key == "metric")
            {
                string number = FirstNumber(value);
                if (number.Length > 0) result["metric"] = number;
            }
            else if (key == "mtu")
            {
                string number = FirstNumber(value);
                if (number.Length > 0) result["mtu"] = number;
            }
        }

        string lower = text.ToLowerInvariant();
        result["enabled"] = lower.IndexOf("disabled", StringComparison.Ordinal) >= 0 || lower.IndexOf("not enabled", StringComparison.Ordinal) >= 0 ? "false" : "true";
        return result;
    }

    static string AdapterObservationJson(Dictionary<string, string> args)
    {
        string adapter = Arg(args, "adapter-name", "");
        if (adapter.Length == 0) return "null";
        List<string> props = new List<string>();
        props.Add(Prop("adapterName", Q(adapter)));
        props.Add(Prop("enabled", Q(Arg(args, "adapter-enabled", "true"))));
        props.Add(Prop("expectedIp", Q(Arg(args, "expected-ip", ""))));
        props.Add(Prop("assignedIp", Q(Arg(args, "assigned-ip", ""))));
        props.Add(Prop("expectedSubnet", Q(Arg(args, "subnet", ""))));
        props.Add(Prop("observedSubnet", Q(Arg(args, "observed-subnet", ""))));
        props.Add(Prop("mtu", Q(Arg(args, "adapter-mtu", ""))));
        props.Add(Prop("interfaceMetric", Q(Arg(args, "adapter-metric", ""))));
        string source = "manual";
        if (Arg(args, "adapter-netsh-readable", "false") == "true")
        {
            source = BoolArg(args, "adapter-scan", false) ? "netsh-scan" : "netsh-file";
        }
        else if (BoolArg(args, "adapter-scan", false))
        {
            source = "netsh-scan";
        }
        props.Add(Prop("source", Q(source)));
        if (args.ContainsKey("adapter-scan-error"))
        {
            props.Add(Prop("scanError", Q(args["adapter-scan-error"])));
        }
        return Obj(props.ToArray());
    }

    static void LoadTunnelObservationArgs(Dictionary<string, string> args)
    {
        string text = "";
        string path = Arg(args, "ping-output", "");
        if (path.Length > 0)
        {
            text = File.ReadAllText(path);
            args["tunnel-source"] = "ping-file";
        }
        else
        {
            string host = Arg(args, "ping-test", "");
            if (host.Length > 0)
            {
                ApplyPingTestObservation(args, host);
                args["tunnel-source"] = "ping-test";
                args["ping-host"] = host;
                return;
            }
        }

        if (text.Length == 0) return;
        Dictionary<string, string> parsed = ParseWindowsPingOutput(text, Int32.Parse(Arg(args, "expected-peers", "1")));
        args["tunnel-state"] = parsed["state"];
        args["connected-peers"] = parsed["connected-peers"];
        args["packet-loss-percent"] = parsed["packet-loss-percent"];
        if (parsed.ContainsKey("latency-ms"))
        {
            args["latency-ms"] = parsed["latency-ms"];
        }
    }

    static void ApplyPingTestObservation(Dictionary<string, string> args, string host)
    {
        int attempts = Int32.Parse(Arg(args, "ping-count", "4"));
        int timeout = Int32.Parse(Arg(args, "timeout-ms", "1000"));
        int received = 0;
        long latencyTotal = 0;
        using (Ping ping = new Ping())
        {
            for (int i = 0; i < attempts; i++)
            {
                try
                {
                    PingReply reply = ping.Send(host, timeout);
                    if (reply != null && reply.Status == IPStatus.Success)
                    {
                        received++;
                        latencyTotal += reply.RoundtripTime;
                    }
                }
                catch
                {
                }
            }
        }

        int lost = Math.Max(0, attempts - received);
        double loss = attempts == 0 ? 100.0 : (lost * 100.0) / attempts;
        int expectedPeers = Int32.Parse(Arg(args, "expected-peers", "1"));
        args["tunnel-state"] = received > 0 ? "connected" : "disconnected";
        args["connected-peers"] = received > 0 ? Math.Max(1, expectedPeers).ToString() : "0";
        args["packet-loss-percent"] = ((int)Math.Round(loss)).ToString();
        if (received > 0)
        {
            args["latency-ms"] = ((int)Math.Round((double)latencyTotal / received)).ToString();
        }
    }

    static Dictionary<string, string> ParseWindowsPingOutput(string text, int expectedPeers)
    {
        Dictionary<string, string> result = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
        int received = ParsePingStat(text, "Received", 0);
        int sent = ParsePingStat(text, "Sent", received);
        int lost = ParsePingStat(text, "Lost", Math.Max(0, sent - received));
        int total = received + lost;
        double loss = total == 0 ? 100.0 : (lost * 100.0) / total;
        result["state"] = received > 0 ? "connected" : "disconnected";
        result["connected-peers"] = received > 0 ? Math.Max(1, expectedPeers).ToString() : "0";
        result["packet-loss-percent"] = ((int)Math.Round(loss)).ToString();
        string latency = ParseAveragePingLatency(text);
        if (latency.Length > 0)
        {
            result["latency-ms"] = latency;
        }
        return result;
    }

    static int ParsePingStat(string text, string key, int fallback)
    {
        string[] parts = text.Replace("\r\n", "\n").Replace('\r', '\n').Split(new char[] { ',', '\n' }, StringSplitOptions.RemoveEmptyEntries);
        for (int i = 0; i < parts.Length; i++)
        {
            string part = parts[i].Trim();
            if (!part.StartsWith(key, StringComparison.OrdinalIgnoreCase)) continue;
            int eq = part.IndexOf('=');
            if (eq < 0) continue;
            string number = FirstNumber(part.Substring(eq + 1));
            int value;
            if (Int32.TryParse(number, out value)) return value;
        }
        return fallback;
    }

    static string ParseAveragePingLatency(string text)
    {
        string[] lines = text.Replace("\r\n", "\n").Replace('\r', '\n').Split('\n');
        for (int i = 0; i < lines.Length; i++)
        {
            if (lines[i].IndexOf("Average", StringComparison.OrdinalIgnoreCase) < 0) continue;
            int eq = lines[i].LastIndexOf('=');
            if (eq < 0) continue;
            return FirstNumber(lines[i].Substring(eq + 1));
        }
        return "";
    }

    static string TunnelObservationJson(Dictionary<string, string> args)
    {
        List<string> props = new List<string>();
        props.Add(Prop("state", Q(Arg(args, "tunnel-state", "connected"))));
        props.Add(Prop("connectedPeers", Q(Arg(args, "connected-peers", "0"))));
        props.Add(Prop("expectedPeers", Q(Arg(args, "expected-peers", "0"))));
        props.Add(Prop("latencyMs", Q(Arg(args, "latency-ms", ""))));
        props.Add(Prop("packetLossPercent", Q(Arg(args, "packet-loss-percent", ""))));
        props.Add(Prop("source", Q(Arg(args, "tunnel-source", "manual"))));
        if (args.ContainsKey("ping-host"))
        {
            props.Add(Prop("pingHost", Q(args["ping-host"])));
        }
        return Obj(props.ToArray());
    }

    static string NetworkTunnelStatus(Dictionary<string, string> args)
    {
        string state = Arg(args, "tunnel-state", "connected");
        if (!state.Equals("connected", StringComparison.OrdinalIgnoreCase)) return "down";
        double loss;
        if (Double.TryParse(Arg(args, "packet-loss-percent", "0"), out loss) && loss > 10.0) return "high-loss";
        return "ok";
    }

    static string NetworkP2pStatus(Dictionary<string, string> args)
    {
        string state = Arg(args, "tunnel-state", "connected");
        if (!state.Equals("connected", StringComparison.OrdinalIgnoreCase)) return "failed";
        int expectedPeers = Int32.Parse(Arg(args, "expected-peers", "0"));
        int connectedPeers = Int32.Parse(Arg(args, "connected-peers", "0"));
        if (expectedPeers > 0 && connectedPeers < expectedPeers) return "missing-peers";
        return "ok";
    }

    static int PacketObservationCount(string packets, bool broadcast, int[] expectedPorts)
    {
        if (packets == null || packets.Trim().Length == 0) return 0;
        string[] items = packets.Split(new char[] { ',' }, StringSplitOptions.RemoveEmptyEntries);
        int count = 0;
        for (int i = 0; i < items.Length; i++)
        {
            string[] parts = items[i].Trim().Split(':');
            if (parts.Length != 7) continue;
            string protocol = parts[0].Trim().ToLowerInvariant();
            if (protocol != "udp" && protocol != "tcp") continue;
            bool packetBroadcast = parts[4].Trim().Equals("broadcast", StringComparison.OrdinalIgnoreCase);
            if (packetBroadcast != broadcast) continue;
            int port;
            if (!Int32.TryParse(parts[3].Trim(), out port)) continue;
            if (expectedPorts.Length > 0 && !ContainsPort(expectedPorts, port)) continue;
            count++;
        }
        return count;
    }

    static string CombinedPacketObservations(Dictionary<string, string> args)
    {
        List<string> observations = new List<string>();
        string inline = Arg(args, "packets", "");
        if (inline.Trim().Length > 0)
        {
            observations.AddRange(inline.Split(new char[] { ',' }, StringSplitOptions.RemoveEmptyEntries));
        }
        string path = Arg(args, "packet-observations", "");
        if (path.Length > 0)
        {
            string[] lines = File.ReadAllLines(path);
            for (int i = 0; i < lines.Length; i++)
            {
                string line = lines[i].Trim();
                if (line.Length > 0 && !line.StartsWith("#"))
                {
                    observations.Add(line);
                }
            }
        }
        return String.Join(",", observations.ToArray());
    }

    static void AppendPacketObservation(Dictionary<string, string> args, string protocol, string sourceIp, string destinationIp, int port, bool broadcast, string direction, int bytes)
    {
        string path = Arg(args, "observe-file", "");
        if (path.Length == 0) return;
        string line = protocol.ToLowerInvariant()
            + ":" + sourceIp
            + ":" + destinationIp
            + ":" + port
            + ":" + (broadcast ? "broadcast" : "unicast")
            + ":" + direction
            + ":" + bytes;
        File.AppendAllText(path, line + Environment.NewLine, Encoding.ASCII);
    }

    static string RemoteIp(IPEndPoint endpoint)
    {
        if (endpoint == null || endpoint.Address == null) return "0.0.0.0";
        if (endpoint.Address.AddressFamily == AddressFamily.InterNetwork) return endpoint.Address.ToString();
        return "0.0.0.0";
    }

    static string ResolveObservationIp(string host)
    {
        IPAddress address;
        if (IPAddress.TryParse(host, out address) && address.AddressFamily == AddressFamily.InterNetwork)
        {
            return address.ToString();
        }
        try
        {
            IPAddress[] addresses = Dns.GetHostAddresses(host);
            for (int i = 0; i < addresses.Length; i++)
            {
                if (addresses[i].AddressFamily == AddressFamily.InterNetwork)
                {
                    return addresses[i].ToString();
                }
            }
        }
        catch
        {
        }
        return "0.0.0.0";
    }

    static bool IsBroadcastHost(string host)
    {
        return host == "255.255.255.255" || host.EndsWith(".255", StringComparison.Ordinal);
    }

    static void AddNetworkObservationCheck(List<string> checks, ref int problems, string key, string status, string healthyMessage, string nextAction)
    {
        bool ok = status == "ok" || status == "seen";
        if (!ok) problems++;
        checks.Add(Obj(
            Prop("key", Q(key)),
            Prop("status", Q(ok ? "ok" : status)),
            Prop("message", Q(ok ? healthyMessage : key + " observation is " + status + ".")),
            Prop("nextAction", Q(ok ? "No action needed." : nextAction))
        ));
    }

    static string DiagnosticReportFromValues(string adapter, string tunnel, string p2p, string broadcast, string gameTraffic)
    {
        List<string> problems = new List<string>();
        AddDiagnosticProblem(problems, "virtual-adapter", adapter);
        AddDiagnosticProblem(problems, "tunnel", tunnel);
        AddDiagnosticProblem(problems, "p2p", p2p);
        AddDiagnosticProblem(problems, "broadcast", broadcast);
        AddDiagnosticProblem(problems, "game-traffic", gameTraffic);
        return Obj(
            Prop("status", Q(problems.Count == 0 ? "healthy" : "needs-attention")),
            Prop("summary", Q(problems.Count == 0 ? "Connectivity indicators look healthy." : "Detected " + problems.Count + " problem(s).")),
            Prop("problems", Arr(problems.ToArray()))
        );
    }

    static int DiagnosticProblemCount(string adapter, string tunnel, string p2p, string broadcast, string gameTraffic)
    {
        int count = 0;
        if (!DiagnosticHealthy("virtual-adapter", adapter)) count++;
        if (!DiagnosticHealthy("tunnel", tunnel)) count++;
        if (!DiagnosticHealthy("p2p", p2p)) count++;
        if (!DiagnosticHealthy("broadcast", broadcast)) count++;
        if (!DiagnosticHealthy("game-traffic", gameTraffic)) count++;
        return count;
    }

    static void AddDiagnosticProblem(List<string> problems, string key, string value)
    {
        if (DiagnosticHealthy(key, value)) return;
        problems.Add(Obj(
            Prop("key", Q(key)),
            Prop("value", Q(value)),
            Prop("message", Q(DiagnosticMessage(key))),
            Prop("nextAction", Q(DiagnosticNextAction(key)))
        ));
    }

    static bool CidrContains(string subnet, string ip)
    {
        string[] parts = subnet.Split('/');
        if (parts.Length != 2) return false;
        int prefix = Int32.Parse(parts[1]);
        uint mask = prefix == 0 ? 0u : UInt32.MaxValue << (32 - prefix);
        return (IpToUInt(parts[0]) & mask) == (IpToUInt(ip) & mask);
    }

    static bool ContainsPort(int[] ports, int port)
    {
        for (int i = 0; i < ports.Length; i++)
        {
            if (ports[i] == port) return true;
        }
        return false;
    }

    static bool BoolArg(Dictionary<string, string> args, string key, bool fallback)
    {
        string value;
        if (!args.TryGetValue(key, out value)) return fallback;
        return value.Equals("true", StringComparison.OrdinalIgnoreCase)
            || value.Equals("yes", StringComparison.OrdinalIgnoreCase)
            || value.Equals("1", StringComparison.OrdinalIgnoreCase)
            || value.Equals("ok", StringComparison.OrdinalIgnoreCase);
    }

    static string NormalizeNetshAdapterKey(string value)
    {
        StringBuilder builder = new StringBuilder();
        for (int i = 0; i < value.Length; i++)
        {
            char ch = value[i];
            if (Char.IsLetterOrDigit(ch))
            {
                builder.Append(Char.ToLowerInvariant(ch));
            }
        }
        return builder.ToString();
    }

    static string FirstIpv4(string value)
    {
        string[] parts = value.Split(new char[] { ' ', '\t', ',', '(', ')' }, StringSplitOptions.RemoveEmptyEntries);
        for (int i = 0; i < parts.Length; i++)
        {
            IPAddress address;
            if (IPAddress.TryParse(parts[i], out address) && address.AddressFamily == AddressFamily.InterNetwork)
            {
                return parts[i];
            }
        }
        return "";
    }

    static string FirstCidr(string value)
    {
        string[] parts = value.Split(new char[] { ' ', '\t', ',', '(', ')' }, StringSplitOptions.RemoveEmptyEntries);
        for (int i = 0; i < parts.Length; i++)
        {
            if (parts[i].IndexOf('/') > 0)
            {
                string[] cidr = parts[i].Split('/');
                IPAddress address;
                int prefix;
                if (cidr.Length == 2 && IPAddress.TryParse(cidr[0], out address) && Int32.TryParse(cidr[1], out prefix) && prefix >= 0 && prefix <= 32)
                {
                    return parts[i];
                }
            }
        }
        return "";
    }

    static string FirstNumber(string value)
    {
        StringBuilder builder = new StringBuilder();
        for (int i = 0; i < value.Length; i++)
        {
            if (Char.IsDigit(value[i]))
            {
                builder.Append(value[i]);
            }
            else if (builder.Length > 0)
            {
                break;
            }
        }
        return builder.ToString();
    }

    static bool DiagnosticHealthy(string key, string value)
    {
        if (key == "virtual-adapter") return value == "ok";
        if (key == "firewall") return value == "allowed" || value == "ok";
        if (key == "tunnel") return value == "ok";
        if (key == "p2p") return value == "ok";
        if (key == "broadcast") return value == "seen";
        if (key == "direct-ip") return value == "ok";
        if (key == "game-traffic") return value == "seen";
        return false;
    }

    static string DiagnosticMessage(string key)
    {
        if (key == "virtual-adapter") return "Virtual adapter is not ready.";
        if (key == "firewall") return "Windows Firewall may block the client or game.";
        if (key == "tunnel") return "Tunnel connection is not healthy.";
        if (key == "p2p") return "P2P connection failed.";
        if (key == "broadcast") return "Broadcast forwarding was not observed.";
        if (key == "direct-ip") return "Direct IP connection failed.";
        if (key == "game-traffic") return "No game traffic was observed.";
        return "Diagnostic check failed.";
    }

    static string DiagnosticNextAction(string key)
    {
        if (key == "virtual-adapter") return "Check driver installation, adapter state, and administrator permission.";
        if (key == "firewall") return "Add inbound rules and allow private networks.";
        if (key == "tunnel") return "Renegotiate the tunnel or switch networks.";
        if (key == "p2p") return "Try port forwarding, network switching, or coordination fallback.";
        if (key == "broadcast") return "Check UDP broadcast rules and game ports.";
        if (key == "direct-ip") return "Try joining with the host virtual IP.";
        if (key == "game-traffic") return "Check whether the game bound to the virtual adapter.";
        return "Open diagnostics and inspect the failed check.";
    }

    static string Required(Dictionary<string, string> args, string key)
    {
        string value;
        if (!args.TryGetValue(key, out value) || value.Length == 0)
        {
            throw new ArgumentException("missing --" + key);
        }
        return value;
    }

    static string FirewallRulesJson(string game, string subnet, int[] ports)
    {
        List<string> rules = new List<string>();
        string[] protocols = new string[] { "udp", "tcp" };
        for (int i = 0; i < ports.Length; i++)
        {
            for (int p = 0; p < protocols.Length; p++)
            {
                rules.Add(Obj(
                    Prop("name", Q(game + " " + protocols[p].ToUpperInvariant() + " " + ports[i])),
                    Prop("protocol", Q(protocols[p])),
                    Prop("port", ports[i].ToString()),
                    Prop("remoteScope", Q(subnet))
                ));
            }
        }
        return Arr(rules.ToArray());
    }

    static string DiagnosticChecksJson(bool hasPorts, bool broadcastEnabled)
    {
        List<string> checks = new List<string>();
        checks.Add(Q("virtual-adapter"));
        checks.Add(Q("tunnel"));
        checks.Add(Q("p2p"));
        checks.Add(Q("direct-ip"));
        if (hasPorts)
        {
            checks.Add(Q("firewall"));
            checks.Add(Q("game-traffic"));
        }
        if (broadcastEnabled)
        {
            checks.Add(Q("broadcast"));
        }
        return Arr(checks.ToArray());
    }

    static string WarningsJson(int[] ports, string discovery, string compatibility)
    {
        List<string> warnings = new List<string>();
        if (ports.Length == 0) warnings.Add(Obj(Prop("key", Q("unknown-ports"))));
        if (discovery == "direct_ip") warnings.Add(Obj(Prop("key", Q("direct-ip-only"))));
        if (compatibility == "D") warnings.Add(Obj(Prop("key", Q("poor-mvp-target"))));
        return Arr(warnings.ToArray());
    }

    static int[] ParsePorts(string value)
    {
        if (value == null || value.Trim().Length == 0) return new int[0];
        string[] parts = value.Split(',');
        SortedDictionary<int, bool> seen = new SortedDictionary<int, bool>();
        for (int i = 0; i < parts.Length; i++)
        {
            int port;
            if (Int32.TryParse(parts[i].Trim(), out port) && port > 0 && port <= 65535)
            {
                seen[port] = true;
            }
        }
        int[] ports = new int[seen.Count];
        int index = 0;
        foreach (int port in seen.Keys) ports[index++] = port;
        return ports;
    }

    static string DecodeInvitePayload(string invite)
    {
        string payload = invite.Split('.')[0];
        return Encoding.UTF8.GetString(Base64UrlDecode(payload));
    }

    static string JsonStringValue(string json, string key)
    {
        string marker = "\"" + key + "\":";
        int start = json.IndexOf(marker, StringComparison.Ordinal);
        if (start < 0) return "";
        start += marker.Length;
        while (start < json.Length && Char.IsWhiteSpace(json[start])) start++;
        if (start >= json.Length || json[start] != '"') return "";
        int end = json.IndexOf('"', start + 1);
        if (end < 0) return "";
        return json.Substring(start + 1, end - start - 1);
    }

    static string SubnetForRoom(string roomId)
    {
        byte[] hash = SHA256.Create().ComputeHash(Encoding.UTF8.GetBytes(roomId));
        return "10.77." + hash[0].ToString() + ".0/24";
    }

    static string HostIp(string subnet) { return OffsetIp(subnet, 1); }
    static string PeerIp(string subnet, int ordinal) { return OffsetIp(subnet, ordinal + 2); }
    static string BroadcastIp(string subnet) { return OffsetIp(subnet, 255); }

    static string OffsetIp(string subnet, int offset)
    {
        string network = subnet.Split('/')[0];
        uint value = IpToUInt(network) + (uint)offset;
        return UIntToIp(value);
    }

    static int Prefix(string subnet)
    {
        string[] parts = subnet.Split('/');
        return parts.Length == 2 ? Int32.Parse(parts[1]) : 24;
    }

    static string MaskIp(int prefix)
    {
        uint mask = prefix == 0 ? 0u : UInt32.MaxValue << (32 - prefix);
        return UIntToIp(mask);
    }

    static uint IpToUInt(string ip)
    {
        string[] parts = ip.Split('.');
        return (UInt32.Parse(parts[0]) << 24) | (UInt32.Parse(parts[1]) << 16) | (UInt32.Parse(parts[2]) << 8) | UInt32.Parse(parts[3]);
    }

    static string UIntToIp(uint value)
    {
        return ((value >> 24) & 255) + "." + ((value >> 16) & 255) + "." + ((value >> 8) & 255) + "." + (value & 255);
    }

    static string RandomToken(int bytes)
    {
        byte[] data = new byte[bytes];
        RandomNumberGenerator.Create().GetBytes(data);
        return Base64Url(data);
    }

    static string Base64Url(byte[] data)
    {
        return Convert.ToBase64String(data).TrimEnd('=').Replace('+', '-').Replace('/', '_');
    }

    static byte[] Base64UrlDecode(string value)
    {
        string text = value.Replace('-', '+').Replace('_', '/');
        while (text.Length % 4 != 0) text += "=";
        return Convert.FromBase64String(text);
    }

    static string NetshQuote(string value)
    {
        if (value.IndexOf(' ') >= 0 || value.IndexOf('\\') >= 0)
        {
            return "\"" + value.Replace("\"", "\\\"") + "\"";
        }
        return value;
    }

    static string Obj(params string[] props)
    {
        return "{" + String.Join(",", props) + "}";
    }

    static string Prop(string name, string json)
    {
        return Q(name) + ":" + json;
    }

    static string Arr(params string[] items)
    {
        return "[" + String.Join(",", items) + "]";
    }

    static string IntArray(int[] items)
    {
        string[] values = new string[items.Length];
        for (int i = 0; i < items.Length; i++) values[i] = items[i].ToString();
        return Arr(values);
    }

    static string Q(string value)
    {
        if (value == null) return "null";
        return "\"" + value.Replace("\\", "\\\\").Replace("\"", "\\\"").Replace("\r", "\\r").Replace("\n", "\\n") + "\"";
    }
}

