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
    string JsonStringValue(string json, string key)
    {
        string marker = "\"" + key + "\":";
        int start = json.IndexOf(marker, StringComparison.Ordinal);
        if (start < 0) return "";
        start += marker.Length;
        while (start < json.Length && Char.IsWhiteSpace(json[start])) start++;
        if (start >= json.Length || json[start] != '"') return "";
        int end = json.IndexOf('"', start + 1);
        if (end < 0) return "";
        return json.Substring(start + 1, end - start - 1).Replace("\\\"", "\"").Replace("\\\\", "\\");
    }

    string JsonCheckStatus(string json, string key)
    {
        string checks = JsonArrayValue(json, "checks");
        if (checks.Length == 0) return "";
        int search = 0;
        while (search < checks.Length)
        {
            int start = checks.IndexOf('{', search);
            if (start < 0) break;
            int end = MatchingJsonBrace(checks, start);
            if (end < 0) break;
            string check = checks.Substring(start, end - start + 1);
            if (JsonStringValue(check, "key") == key)
            {
                return JsonStringValue(check, "status");
            }
            search = end + 1;
        }
        return "";
    }

    string JsonObjectValue(string json, string key)
    {
        string marker = "\"" + key + "\":";
        int markerStart = json.IndexOf(marker, StringComparison.Ordinal);
        if (markerStart < 0) return "";
        int start = json.IndexOf('{', markerStart + marker.Length);
        if (start < 0) return "";
        int depth = 0;
        bool inString = false;
        bool escaped = false;
        for (int i = start; i < json.Length; i++)
        {
            char ch = json[i];
            if (escaped)
            {
                escaped = false;
                continue;
            }
            if (ch == '\\' && inString)
            {
                escaped = true;
                continue;
            }
            if (ch == '"')
            {
                inString = !inString;
                continue;
            }
            if (inString) continue;
            if (ch == '{') depth++;
            else if (ch == '}')
            {
                depth--;
                if (depth == 0)
                {
                    return json.Substring(start, i - start + 1);
                }
            }
        }
        return "";
    }

    string CoordinationMembersText(string json)
    {
        string array = JsonArrayValue(json, "members");
        if (array.Length == 0) return "";
        List<string> members = new List<string>();
        int search = 0;
        while (search < array.Length && members.Count < 4)
        {
            int start = array.IndexOf('{', search);
            if (start < 0) break;
            int end = MatchingJsonBrace(array, start);
            if (end < 0) break;
            string member = array.Substring(start, end - start + 1);
            string peer = JsonStringValue(member, "peer_id");
            string virtualIp = JsonStringValue(member, "virtual_ip");
            string status = JsonStringValue(member, "status");
            bool isHost = JsonBoolValue(member, "is_host");
            if (peer.Length > 0)
            {
                string text = peer;
                if (virtualIp.Length > 0) text += " @ " + virtualIp;
                if (status.Length > 0) text += " (" + status + ")";
                if (isHost) text += " [" + T("detailHost") + "]";
                members.Add(text);
            }
            search = end + 1;
        }
        int memberCount;
        if (Int32.TryParse(JsonNumberValue(json, "member_count"), out memberCount) && memberCount > members.Count)
        {
            members.Add("+" + (memberCount - members.Count).ToString());
        }
        return String.Join(Environment.NewLine, members.ToArray());
    }

    string RuntimePeersText(string json)
    {
        string array = JsonArrayValue(json, "runtimePeerSummaries");
        if (array.Length == 0) array = JsonArrayValue(json, "summaries");
        if (array.Length == 0) return "";
        List<string> peers = new List<string>();
        int search = 0;
        while (search < array.Length && peers.Count < 8)
        {
            int start = array.IndexOf('{', search);
            if (start < 0) break;
            int end = MatchingJsonBrace(array, start);
            if (end < 0) break;
            string peer = array.Substring(start, end - start + 1);
            string peerId = JsonStringValue(peer, "peerId");
            string virtualIp = JsonStringValue(peer, "virtualIp");
            string selectedPath = JsonStringValue(peer, "selectedPath");
            string pathKind = JsonStringValue(peer, "pathKind");
            string latencyMs = JsonNumberValue(peer, "latencyMs");
            string loss = JsonNumberValue(peer, "heartbeatLossWindowPercent");
            if (loss.Length == 0) loss = JsonNumberValue(peer, "heartbeatLossPercent");
            string jitter = JsonNumberValue(peer, "heartbeatRttJitterMs");
            string healthObject = JsonObjectValue(peer, "health");
            string health = JsonStringValue(healthObject, "status");
            string healthReason = JsonStringValue(healthObject, "reason");
            string healthNextAction = JsonStringValue(healthObject, "nextAction");
            string bytesSent = JsonNumberValue(peer, "bytesSent");
            string bytesReceived = JsonNumberValue(peer, "bytesReceived");
            string directSent = JsonNumberValue(peer, "directBytesSent");
            string directReceived = JsonNumberValue(peer, "directBytesReceived");
            string relaySent = JsonNumberValue(peer, "relayBytesSent");
            string relayReceived = JsonNumberValue(peer, "relayBytesReceived");
            if (peerId.Length > 0)
            {
                string text = peerId;
                if (virtualIp.Length > 0) text += " @ " + virtualIp;
                if (pathKind.Length > 0) text += " [" + pathKind + "]";
                else if (selectedPath.Length > 0) text += " [" + selectedPath + "]";
                if (health.Length > 0) text += " " + health;
                if (healthReason.Length > 0 && healthReason != "healthy") text += " (" + healthReason + ")";
                if (latencyMs.Length > 0) text += " " + latencyMs + "ms";
                if (loss.Length > 0) text += " loss " + ShortNumber(loss) + "%";
                if (jitter.Length > 0) text += " jitter " + ShortNumber(jitter) + "ms";
                if (bytesSent.Length > 0 || bytesReceived.Length > 0)
                {
                    if (bytesSent.Length == 0) bytesSent = "0";
                    if (bytesReceived.Length == 0) bytesReceived = "0";
                    text += " " + bytesSent + "/" + bytesReceived + "B";
                }
                if (directSent.Length > 0 || directReceived.Length > 0 || relaySent.Length > 0 || relayReceived.Length > 0)
                {
                    if (directSent.Length == 0) directSent = "0";
                    if (directReceived.Length == 0) directReceived = "0";
                    if (relaySent.Length == 0) relaySent = "0";
                    if (relayReceived.Length == 0) relayReceived = "0";
                    text += " d " + directSent + "/" + directReceived + " r " + relaySent + "/" + relayReceived;
                }
                if (health.Length > 0 && health != "ok" && healthNextAction.Length > 0)
                {
                    text += " | " + healthNextAction;
                }
                peers.Add(text);
            }
            search = end + 1;
        }
        return String.Join(Environment.NewLine, peers.ToArray());
    }

    string RuntimeConnectionPathText(string json)
    {
        string array = JsonArrayValue(json, "connectionPathReports");
        if (array.Length == 0) array = JsonArrayValue(json, "connection_path_reports");
        if (array.Length == 0) return "";
        List<string> paths = new List<string>();
        int search = 0;
        while (search < array.Length && paths.Count < 8)
        {
            int start = array.IndexOf('{', search);
            if (start < 0) break;
            int end = MatchingJsonBrace(array, start);
            if (end < 0) break;
            string entry = array.Substring(start, end - start + 1);
            string peerId = JsonStringValue(entry, "peerId");
            string bootstrapStatus = JsonStringValue(entry, "bootstrapStatus");
            string source = JsonStringValue(entry, "source");
            string localEndpoint = JsonStringValue(entry, "localEndpoint");
            string selectedPeerEndpoint = JsonStringValue(entry, "selectedPeerEndpoint");
            string handshakeRole = JsonStringValue(entry, "handshakeRole");
            string confirmedByAck = JsonBoolTextValue(entry, "confirmedByAck");
            string report = JsonObjectValue(entry, "report");
            if (report.Length == 0) report = entry;
            string status = JsonStringValue(report, "status");
            string selectedPath = JsonStringValue(report, "selected_path");
            string selectedEndpoint = JsonFirstStringInArray(JsonArrayValue(report, "selected_endpoints"));
            string remoteHost = JsonNumberValue(report, "remote_host_candidate_count");
            string remoteSrflx = JsonNumberValue(report, "remote_srflx_candidate_count");
            string stunBehavior = JsonStringValue(JsonObjectValue(entry, "stunMapping"), "mappingBehavior");
            string upnpStatus = JsonStringValue(JsonObjectValue(entry, "upnpPortMapping"), "status");
            string relayFallback = JsonObjectValue(report, "relay_fallback");
            string nextAction = JsonFirstStringInArray(JsonArrayValue(relayFallback, "recommended_actions"));
            if (peerId.Length > 0)
            {
                string text = peerId;
                if (bootstrapStatus.Length > 0) text += " " + bootstrapStatus;
                if (status.Length > 0) text += " / " + status;
                if (selectedPath.Length > 0) text += " [" + selectedPath + "]";
                if (remoteHost.Length > 0 || remoteSrflx.Length > 0)
                {
                    if (remoteHost.Length == 0) remoteHost = "0";
                    if (remoteSrflx.Length == 0) remoteSrflx = "0";
                    text += " h/s=" + remoteHost + "/" + remoteSrflx;
                }
                if (stunBehavior.Length > 0 && stunBehavior != "not-tested") text += " nat=" + stunBehavior;
                if (upnpStatus.Length > 0 && upnpStatus != "disabled") text += " upnp=" + upnpStatus;
                if (confirmedByAck.Length > 0) text += " ack=" + confirmedByAck;
                if (handshakeRole.Length > 0) text += " role=" + handshakeRole;
                if (localEndpoint.Length > 0) text += " local=" + localEndpoint;
                if (selectedPeerEndpoint.Length > 0) text += " peer=" + selectedPeerEndpoint;
                if (selectedEndpoint.Length > 0) text += " -> " + selectedEndpoint;
                if (source.Length > 0) text += " {" + source + "}";
                if (nextAction.Length > 0) text += " | " + nextAction;
                paths.Add(text);
            }
            search = end + 1;
        }
        return String.Join(Environment.NewLine, paths.ToArray());
    }

    string StunMappingText(string json)
    {
        string mapping = JsonObjectValue(json, "stunMapping");
        if (mapping.Length == 0) return "STUN: " + T("stateUnknown");
        string status = JsonStringValue(mapping, "status");
        string behavior = JsonStringValue(mapping, "mappingBehavior");
        string detail = JsonStringValue(mapping, "detail");
        if (status.Length == 0) status = T("stateUnknown");
        if (behavior.Length == 0) behavior = T("stateUnknown");
        string text = "STUN: " + status + " / " + behavior;
        if (detail.Length > 0 && status != "disabled") text += " (" + detail + ")";
        return text;
    }

    string UpnpMappingText(string json)
    {
        string mapping = JsonObjectValue(json, "upnpPortMapping");
        if (mapping.Length == 0) return T("upnpPortMap") + ": " + T("stateUnknown");
        string status = JsonStringValue(mapping, "status");
        string external = JsonStringValue(mapping, "externalEndpoint");
        string detail = JsonStringValue(mapping, "detail");
        if (status.Length == 0) status = T("stateUnknown");
        string text = T("upnpPortMap") + ": " + status;
        if (external.Length > 0) text += " -> " + external;
        if (detail.Length > 0 && status != "disabled") text += " (" + detail + ")";
        return text;
    }

    string CandidateSourceCountText(string offerJson, string source)
    {
        if (offerJson.Trim().Length == 0 || source.Length == 0) return "";
        int count = 0;
        int search = 0;
        string needle = "\"source\":\"" + source + "\"";
        string spacedNeedle = "\"source\": \"" + source + "\"";
        while (search < offerJson.Length)
        {
            int compact = offerJson.IndexOf(needle, search, StringComparison.OrdinalIgnoreCase);
            int spaced = offerJson.IndexOf(spacedNeedle, search, StringComparison.OrdinalIgnoreCase);
            int found = compact < 0 ? spaced : (spaced < 0 ? compact : Math.Min(compact, spaced));
            if (found < 0) break;
            count++;
            search = found + source.Length;
        }
        return count > 0 ? source + "=" + count.ToString(CultureInfo.InvariantCulture) : "";
    }

    string RuntimeRelayFallbackText(string json)
    {
        string array = JsonArrayValue(json, "runtimeRelayFallbackSummaries");
        if (array.Length == 0) array = JsonArrayValue(json, "runtime_summaries");
        if (array.Length == 0) return "";
        List<string> plans = new List<string>();
        int search = 0;
        while (search < array.Length && plans.Count < 6)
        {
            int start = array.IndexOf('{', search);
            if (start < 0) break;
            int end = MatchingJsonBrace(array, start);
            if (end < 0) break;
            string plan = array.Substring(start, end - start + 1);
            string peerId = JsonStringValue(plan, "peerId");
            string status = JsonStringValue(plan, "status");
            string selectedPath = JsonStringValue(plan, "selectedPath");
            string relayEndpoint = JsonFirstStringInArray(JsonArrayValue(plan, "selectedRelayEndpoints"));
            string nextAction = JsonFirstStringInArray(JsonArrayValue(plan, "recommendedActions"));
            if (peerId.Length > 0)
            {
                string text = peerId;
                if (status.Length > 0) text += " " + status;
                if (selectedPath.Length > 0) text += " [" + selectedPath + "]";
                if (relayEndpoint.Length > 0) text += " -> " + relayEndpoint;
                if (nextAction.Length > 0) text += " | " + nextAction;
                plans.Add(text);
            }
            search = end + 1;
        }
        return String.Join(Environment.NewLine, plans.ToArray());
    }

    string ShortNumber(string value)
    {
        double number;
        if (!Double.TryParse(value, System.Globalization.NumberStyles.Float, System.Globalization.CultureInfo.InvariantCulture, out number))
        {
            return value;
        }
        if (Math.Abs(number - Math.Round(number)) < 0.01)
        {
            return Math.Round(number).ToString(System.Globalization.CultureInfo.InvariantCulture);
        }
        return number.ToString("0.0", System.Globalization.CultureInfo.InvariantCulture);
    }

    string CompactJson(string json)
    {
        if (json.Length == 0) return "";
        StringBuilder builder = new StringBuilder(json.Length);
        bool inString = false;
        bool escaped = false;
        for (int i = 0; i < json.Length; i++)
        {
            char ch = json[i];
            if (escaped)
            {
                builder.Append(ch);
                escaped = false;
                continue;
            }
            if (ch == '\\' && inString)
            {
                builder.Append(ch);
                escaped = true;
                continue;
            }
            if (ch == '"')
            {
                builder.Append(ch);
                inString = !inString;
                continue;
            }
            if (inString || !Char.IsWhiteSpace(ch))
            {
                builder.Append(ch);
            }
        }
        return builder.ToString();
    }

    string JsonArrayValue(string json, string key)
    {
        string marker = "\"" + key + "\":";
        int markerStart = json.IndexOf(marker, StringComparison.Ordinal);
        if (markerStart < 0) return "";
        int start = json.IndexOf('[', markerStart + marker.Length);
        if (start < 0) return "";
        int depth = 0;
        bool inString = false;
        bool escaped = false;
        for (int i = start; i < json.Length; i++)
        {
            char ch = json[i];
            if (escaped)
            {
                escaped = false;
                continue;
            }
            if (ch == '\\' && inString)
            {
                escaped = true;
                continue;
            }
            if (ch == '"')
            {
                inString = !inString;
                continue;
            }
            if (inString) continue;
            if (ch == '[') depth++;
            else if (ch == ']')
            {
                depth--;
                if (depth == 0)
                {
                    return json.Substring(start, i - start + 1);
                }
            }
        }
        return "";
    }

    int MatchingJsonBrace(string json, int start)
    {
        int depth = 0;
        bool inString = false;
        bool escaped = false;
        for (int i = start; i < json.Length; i++)
        {
            char ch = json[i];
            if (escaped)
            {
                escaped = false;
                continue;
            }
            if (ch == '\\' && inString)
            {
                escaped = true;
                continue;
            }
            if (ch == '"')
            {
                inString = !inString;
                continue;
            }
            if (inString) continue;
            if (ch == '{') depth++;
            else if (ch == '}')
            {
                depth--;
                if (depth == 0) return i;
            }
        }
        return -1;
    }

    string FirstJsonObject(string array)
    {
        if (array.Length == 0) return "";
        int start = array.IndexOf('{');
        if (start < 0) return "";
        int end = MatchingJsonBrace(array, start);
        if (end < 0) return "";
        return array.Substring(start, end - start + 1);
    }

    string JsonNumberValue(string json, string key)
    {
        string marker = "\"" + key + "\":";
        int start = json.IndexOf(marker, StringComparison.Ordinal);
        if (start < 0) return "";
        start += marker.Length;
        while (start < json.Length && Char.IsWhiteSpace(json[start])) start++;
        int end = start;
        while (end < json.Length && (Char.IsDigit(json[end]) || json[end] == '.' || json[end] == '-')) end++;
        return end > start ? json.Substring(start, end - start) : "";
    }

    bool JsonBoolValue(string json, string key)
    {
        string marker = "\"" + key + "\":";
        int start = json.IndexOf(marker, StringComparison.Ordinal);
        if (start < 0) return false;
        start += marker.Length;
        while (start < json.Length && Char.IsWhiteSpace(json[start])) start++;
        return json.IndexOf("true", start, StringComparison.OrdinalIgnoreCase) == start;
    }

    string JsonBoolTextValue(string json, string key)
    {
        string marker = "\"" + key + "\":";
        int start = json.IndexOf(marker, StringComparison.Ordinal);
        if (start < 0) return "";
        start += marker.Length;
        while (start < json.Length && Char.IsWhiteSpace(json[start])) start++;
        if (json.IndexOf("true", start, StringComparison.OrdinalIgnoreCase) == start) return "true";
        if (json.IndexOf("false", start, StringComparison.OrdinalIgnoreCase) == start) return "false";
        return "";
    }

    string JsonPortArrayCsv(string array)
    {
        if (array.Length == 0) return "";
        List<string> values = new List<string>();
        int index = 0;
        while (index < array.Length)
        {
            while (index < array.Length && !Char.IsDigit(array[index])) index++;
            int start = index;
            while (index < array.Length && Char.IsDigit(array[index])) index++;
            if (index > start)
            {
                int port;
                if (Int32.TryParse(array.Substring(start, index - start), out port) && port > 0 && port <= 65535)
                {
                    string text = port.ToString();
                    if (!values.Contains(text))
                    {
                        values.Add(text);
                    }
                }
            }
        }
        return String.Join(",", values.ToArray());
    }

    string JsonFirstStringInArray(string array)
    {
        if (array.Length == 0) return "";
        int start = array.IndexOf('"');
        if (start < 0) return "";
        int end = start + 1;
        bool escaped = false;
        while (end < array.Length)
        {
            char ch = array[end];
            if (escaped)
            {
                escaped = false;
            }
            else if (ch == '\\')
            {
                escaped = true;
            }
            else if (ch == '"')
            {
                return array.Substring(start + 1, end - start - 1).Replace("\\\"", "\"").Replace("\\\\", "\\");
            }
            end++;
        }
        return "";
    }

    string JsonStringLiteral(string value)
    {
        if (value == null) return "\"\"";
        return "\"" + value
            .Replace("\\", "\\\\")
            .Replace("\"", "\\\"")
            .Replace("\r", "\\r")
            .Replace("\n", "\\n")
            .Replace("\t", "\\t") + "\"";
    }

    int JsonObjectCount(string array)
    {
        if (array.Length == 0) return 0;
        int count = 0;
        int search = 0;
        while (search < array.Length)
        {
            int start = array.IndexOf('{', search);
            if (start < 0) break;
            int end = MatchingJsonBrace(array, start);
            if (end < 0) break;
            count++;
            search = end + 1;
        }
        return count;
    }

    int JsonStringArrayCount(string array)
    {
        if (array.Length == 0) return 0;
        int count = 0;
        bool inString = false;
        bool escaped = false;
        for (int i = 0; i < array.Length; i++)
        {
            char ch = array[i];
            if (escaped)
            {
                escaped = false;
                continue;
            }
            if (ch == '\\' && inString)
            {
                escaped = true;
                continue;
            }
            if (ch == '"')
            {
                inString = !inString;
                if (!inString) count++;
            }
        }
        return count;
    }

    string FirewallDiagnoseArgs()
    {
        string args = "firewall-diagnose --game-name " + Quote(gameName.Text) + GameCatalogArgs() + " --subnet " + subnet.Text + " --ports " + ports.Text;
        if (netshOutput.Text.Trim().Length > 0)
        {
            return args + " --netsh-output " + Quote(netshOutput.Text.Trim());
        }
        return args + " --observed " + observed.Text;
    }
}

