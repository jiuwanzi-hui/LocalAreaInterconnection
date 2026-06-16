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
    void ApplyLanguage()
    {
        Text = T("appTitle");
        if (titleLabel != null) titleLabel.Text = T("appTitle");
        if (languageButton != null) languageButton.Invalidate();
        foreach (KeyValuePair<string, Label> item in labelControls)
        {
            item.Value.Text = T(item.Key);
        }
        foreach (KeyValuePair<string, Button> item in buttonControls)
        {
            item.Value.Text = T(item.Key);
        }
        if (moreToolsButton != null)
        {
            moreToolsButton.Text = T(advancedActionsVisible ? "hideTools" : "moreTools");
        }
        UpdateChromeTooltips();
        if (output != null && output.Text.Length == 0)
        {
            output.Text = T("outputHelp");
        }
        foreach (Button nav in navButtons)
        {
            nav.Invalidate();
        }
        if (sidebarPanel != null) sidebarPanel.Invalidate();
        AdjustActionLayout();
    }

    void UpdateChromeTooltips()
    {
        if (chromeTips == null) return;
        foreach (Control control in Controls)
        {
            UpdateChromeTooltipsRecursive(control);
        }
    }

    void UpdateChromeTooltipsRecursive(Control parent)
    {
        foreach (Control child in parent.Controls)
        {
            if (child.Tag is string)
            {
                chromeTips.SetToolTip(child, T((string)child.Tag));
            }
            UpdateChromeTooltipsRecursive(child);
        }
    }

    string LoadLanguage()
    {
        string path = SettingsPath();
        if (File.Exists(path))
        {
            string saved = File.ReadAllText(path).Trim();
            if (saved == "zh" || saved == "en") return saved;
        }
        return CultureInfo.CurrentUICulture.TwoLetterISOLanguageName == "zh" ? "zh" : "en";
    }

    void SaveLanguage()
    {
        string path = SettingsPath();
        string directory = Path.GetDirectoryName(path);
        if (!Directory.Exists(directory)) Directory.CreateDirectory(directory);
        File.WriteAllText(path, language);
    }

    string SettingsPath()
    {
        return Path.Combine(AppDataDirectory(), "settings.lang");
    }

    string AppDataDirectory()
    {
        string directory = Path.Combine(
            Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData),
            "LocalAreaInterconnection");
        if (!Directory.Exists(directory)) Directory.CreateDirectory(directory);
        return directory;
    }

    string DefaultGameCatalogPath()
    {
        string path = Path.Combine(AppDomain.CurrentDomain.BaseDirectory, "game-profiles.example.json");
        return File.Exists(path) ? path : "";
    }

    string T(string key)
    {
        if (language == "zh")
        {
            if (key == "appTitle") return "局域网互联";
            if (key == "appTagline") return "异地玩家虚拟局域网";
            if (key == "navHome") return "首页 / 房间";
            if (key == "navDiagnose") return "联机诊断";
            if (key == "navGames") return "游戏模板";
            if (key == "navTools") return "更多工具";
            if (key == "navAbout") return "关于";
            if (key == "aboutVersion") return "版本 0.1.0  ·  Rust 原生核心 + Wintun";
            if (key == "aboutDesc") return "为只支持局域网联机的 PC 游戏创建一个低延迟、可诊断的虚拟局域网。优先 P2P，P2P 失败自动中继，房间隔离、端到端加密。";
            if (key == "roomName") return "房间名称";
            if (key == "host") return "主机名";
            if (key == "virtualSubnet") return "虚拟网段";
            if (key == "myVirtualIp") return "我的虚拟 IP";
            if (key == "gameName") return "游戏名称";
            if (key == "gameCatalog") return "游戏模板库";
            if (key == "gamePorts") return "游戏端口";
            if (key == "observedRules") return "已观测规则";
            if (key == "netshOutputFile") return "Netsh 输出文件";
            if (key == "pingTarget") return "Ping 目标";
            if (key == "packetObservations") return "包观测文件";
            if (key == "invite") return "邀请码";
            if (key == "output") return "命令输出 / 诊断结果";
            if (key == "outputHelp") return "点击上方按钮后，这里会显示命令输出、计划 JSON 或诊断结果。创建房间会自动填入邀请码，计划类操作默认不会修改系统。";
            if (key == "quickHostRoom") return "一键开房";
            if (key == "quickJoinRoom") return "加入朋友";
            if (key == "startLanSession") return "启动联机";
            if (key == "checkConnection") return "检查连接";
            if (key == "moreTools") return "更多工具";
            if (key == "hideTools") return "收起工具";
            if (key == "quickInviteCopied") return "邀请码已复制，直接发给朋友。";
            if (key == "quickInviteCopyFailed") return "房间已创建，但自动复制失败。请从“邀请码”输入框手动复制给朋友。";
            if (key == "quickNextHost") return "下一步：两边都点“生成直连 Offer”并互相粘贴，再点“启动联机”。";
            if (key == "quickJoinedNext") return "已读取邀请并加入房间。下一步：两边都点“生成直连 Offer”并互相粘贴，再点“启动联机”。";
            if (key == "quickLanStarting") return "正在启动联机组件，请稍等。";
            if (key == "quickLanStarted") return "联机组件已启动。另一端也需要加入房间并点击“启动联机”；两边都启动后，进游戏试试 LAN 房间。";
            if (key == "actionAlreadyRunning") return "正在处理上一步，请稍等完成后再点。";
            if (key == "networkDiagnoseRunning") return "正在检查连接，请稍等。另一端未启动或网络不通时，检查可能需要几秒。";
            if (key == "networkDiagnoseDone") return "连接检查完成:";
            if (key == "commandTimedOut") return "操作超时。请确认另一端也已加入并启动联机，或稍后再点“检查连接”。";
            if (key == "summaryAdapter") return "虚拟网卡:";
            if (key == "summaryTunnel") return "隧道:";
            if (key == "summaryBroadcast") return "广播发现:";
            if (key == "summaryGame") return "游戏流量:";
            if (key == "summaryReadiness") return "游戏就绪:";
            if (key == "summaryPath") return "连接路径:";
            if (key == "stateOk") return "正常";
            if (key == "stateNeedsAttention") return "需要处理";
            if (key == "stateSkipped") return "已跳过";
            if (key == "stateFailed") return "失败";
            if (key == "actionCouldNotFinish") return "这一步没有完成。";
            if (key == "technicalSummary") return "简要原因:";
            if (key == "hostNeedsName") return "请先确认“房间名称”和“主机名”已填写，然后再点“一键开房”。";
            if (key == "joinNeedsInvite") return "请先把朋友发来的邀请码粘贴到“邀请码”输入框，再点“加入朋友”。";
            if (key == "startNeedsRoom") return "请先“一键开房”或粘贴邀请码点“加入朋友”，再点“启动联机”。";
            if (key == "copyInviteNeedsRoom") return "请先点“一键开房”生成邀请码，再复制给朋友。";
            if (key == "tryMainFlowAgain") return "请按主流程操作：一键开房或加入朋友，然后启动联机，最后检查连接。";
            if (key == "clipboardCopyFailed") return "复制到剪贴板失败，请手动复制下面的内容:";
            if (key == "createRoom") return "创建房间";
            if (key == "copyInvite") return "复制邀请";
            if (key == "copyIp") return "复制我的 IP";
            if (key == "decodeInvite") return "解析邀请";
            if (key == "joinRoom") return "加入房间";
            if (key == "adapterPlan") return "网卡计划";
            if (key == "adapterScan") return "扫描网卡";
            if (key == "nativeAdapterEnsure") return "原生网卡检查";
            if (key == "nativeAdapterApply") return "配置虚拟网卡";
            if (key == "prepareLanEnvironment") return "准备联机环境";
            if (key == "gamePlan") return "游戏计划";
            if (key == "gameProfileList") return "模板列表";
            if (key == "gameProfilePlan") return "模板游戏计划";
            if (key == "gamePortScan") return "游戏端口扫描";
            if (key == "gameReadiness") return "游戏就绪";
            if (key == "gameReadinessCheck") return "游戏就绪检查";
            if (key == "firewallPlan") return "防火墙计划";
            if (key == "firewallDiagnose") return "防火墙诊断";
            if (key == "firewallScan") return "扫描防火墙";
            if (key == "firewallApply") return "应用防火墙";
            if (key == "generalDiagnose") return "通用诊断";
            if (key == "networkDiagnose") return "网络诊断";
            if (key == "exportDiagnostics") return "导出诊断";
            if (key == "udpTest") return "UDP 测试";
            if (key == "broadcastTest") return "广播测试";
            if (key == "nativeRuntimeSelfTest") return "原生隧道自检";
            if (key == "wintunDetect") return "Wintun 检测";
            if (key == "wintunProbe") return "Wintun 探针";
            if (key == "directOffer") return "生成直连 Offer";
            if (key == "directSelfTest") return "直连自检";
            if (key == "startRuntime") return "启动 runtime";
            if (key == "stopRuntime") return "停止 runtime";
            if (key == "runtimeCleanupPlan") return "清理计划";
            if (key == "runtimeCleanupApply") return "应用清理";
            if (key == "routeScan") return "扫描路由";
            if (key == "startCoordination") return "启动协调";
            if (key == "stopCoordination") return "停止协调";
            if (key == "closeRoom") return "关闭房间";
            if (key == "kickPeer") return "踢出 Peer";
            if (key == "nativeNatSelfTest") return "NAT 自检";
            if (key == "relayFallbackPlan") return "中继计划";
            if (key == "connectionPathPlan") return "连接路径";
            if (key == "tcpTest") return "TCP 测试";
            if (key == "browseGameCatalog") return "选择模板库";
            if (key == "browseNetsh") return "选择 Netsh";
            if (key == "browsePackets") return "选择包文件";
            if (key == "copyOutput") return "复制输出";
            if (key == "autoNetworkDiagnose") return "已自动刷新网络诊断:";
            if (key == "roomDetails") return "房间详情";
            if (key == "detailRoom") return "房间:";
            if (key == "detailSubnet") return "网段:";
            if (key == "detailConnection") return "连接:";
            if (key == "detailBroadcast") return "广播:";
            if (key == "detailMembers") return "成员:";
            if (key == "detailNext") return "下一步:";
            if (key == "detailAdapter") return "网卡";
            if (key == "detailTunnel") return "隧道";
            if (key == "detailGameTraffic") return "游戏流量:";
            if (key == "detailGameProfile") return "游戏模板";
            if (key == "gameProfileMatches") return "匹配模板";
            if (key == "gameProfileSelected") return "已回填匹配到的模板；继续运行游戏就绪检查。";
            if (key == "gameProfileNoMatch") return "没有匹配模板；调整游戏名称或选择其他模板库。";
            if (key == "detailCompatibility") return "兼容等级";
            if (key == "detailGamePorts") return "端口:";
            if (key == "gamePortEndpoints") return "端点";
            if (key == "gamePortMatches") return "端口命中";
            if (key == "detailPath") return "路径";
            if (key == "detailRelay") return "中继";
            if (key == "detailHost") return "房主";
            if (key == "stateUnknown") return "未知";
            if (key == "stateYes") return "是";
            if (key == "stateNo") return "否";
            if (key == "connectionHostReady") return "房主模式，等待朋友加入";
            if (key == "connectionJoined") return "已加入，等待连通性验证";
            if (key == "connectionExported") return "诊断包已导出";
            if (key == "connectionClosed") return "房间已关闭";
            if (key == "nextCreateLanRoom") return "复制邀请码给朋友，然后启动游戏并创建 LAN 房间。";
            if (key == "nextFindLanRoom") return "进入游戏 LAN 页面查找房间；找不到时运行网络诊断。";
            if (key == "nextJoinRoom") return "点击加入房间，获得建议虚拟 IP。";
            if (key == "nextShareBundle") return "把诊断包发给测试者前先检查本机配置内容。";
            if (key == "nextCreateOrJoin") return "先创建房间或粘贴邀请码加入房间。";
            if (key == "nextFixAdapter") return "检查虚拟网卡是否存在、启用并分配了房间 IP。";
            if (key == "nextFixTunnel") return "检查 ping/P2P 状态，必要时换网络或使用端口转发。";
            if (key == "nextCheckBroadcast") return "检查广播代理和游戏发现端口。";
            if (key == "nextStartGame") return "启动游戏并确认它绑定到虚拟网卡。";
            if (key == "nextHealthy") return "连接指标正常，可以尝试进入游戏 LAN 房间。";
            if (key == "inviteCopied") return "已复制邀请码:";
            if (key == "ipCopied") return "已复制虚拟 IP:";
            if (key == "nothingToCopy") return "没有可复制的内容。";
            if (key == "minimizeTip") return "最小化";
            if (key == "maximizeTip") return "最大化 / 还原";
            if (key == "closeTip") return "关闭";
            if (key == "selectNetshOutput") return "选择 netsh 输出文件";
            if (key == "selectGameCatalog") return "选择游戏模板库 JSON";
            if (key == "selectPacketObservations") return "选择或创建包观测文件";
            if (key == "saveDiagnosticBundle") return "保存诊断包";
            if (key == "runtimeAlreadyRunning") return "runtime 已在运行。";
            if (key == "runtimeStarted") return "runtime 已启动，正在写入 snapshot 和包观测文件。";
            if (key == "runtimeStopped") return "runtime 已停止。";
            if (key == "runtimeNotRunning") return "runtime 当前没有运行。";
            if (key == "runtimeRunning") return "runtime: 运行中";
            if (key == "runtimeStoppedState") return "runtime: 已停止";
            if (key == "runtimeExited") return "runtime 进程已退出:";
            if (key == "runtimeSnapshotPath") return "Snapshot: ";
            if (key == "runtimeObservationPath") return "包观测: ";
            if (key == "runtimeSnapshotReady") return "可用于诊断导出的 snapshot: ";
            if (key == "nativeOffer") return "生成 Offer";
            if (key == "coordinationServer") return "协调服务";
            if (key == "stunServer") return "STUN 地址发现";
            if (key == "upnpPortMap") return "UPnP 端口映射";
            if (key == "remotePeer") return "远端 Peer / Offer";
            if (key == "nativeOfferPath") return "Offer 文件: ";
            if (key == "directOfferReady") return "直连 Offer 已生成。把下面整段发给对方；对方粘贴到“远端 Peer / Offer”。";
            if (key == "directOfferFailed") return "直连 Offer 生成失败，请先创建/加入房间并确认虚拟 IP。";
            if (key == "directOfferNext") return "两边都粘贴对方的直连 Offer 后，同时点击“启动局域网”。如果 NAT 不支持无中继直连，连接诊断会显示超时或 no-path。";
            if (key == "directSelfTestReady") return "直连自检结果:";
            if (key == "directRemoteOfferRequired") return "无服务器直连需要先互换直连 Offer：两边都点“生成直连 Offer”，互相发送整段文本，再粘贴到“远端 Peer / Offer”。";
            if (key == "directBootstrapStarting") return "正在尝试无服务器 P2P 直连:";
            if (key == "runtimePeers") return "peer";
            if (key == "runtimeHeartbeats") return "心跳";
            if (key == "runtimeSnapshots") return "snapshot";
            if (key == "runtimeConnectionPaths") return "连接路径";
            if (key == "runtimeLogTail") return "runtime 最近日志:";
            if (key == "runtimeCleanup") return "清理";
            if (key == "runtimeCleanupSteps") return "步骤";
            if (key == "runtimeCleanupChecks") return "检查";
            if (key == "runtimeCleanupActions") return "动作";
            if (key == "runtimeCleanupRestore") return "还原网卡:";
            if (key == "runtimeCleanupCommands") return "命令";
            if (key == "runtimeCleanupConfirmed") return "已确认";
            if (key == "runtimeCleanupUnsafe") return "拦截";
            if (key == "runtimeCleanupApplyConfirm") return "将尝试执行 runtime 清理计划中的安全白名单命令；网卡和路由清理通常需要管理员权限。选择“否”会只输出预览。";
            if (key == "runtimeCleanupNeedsAdmin") return "网卡还原命令需要管理员终端执行；先检查输出中的 dry-run 命令。";
            if (key == "runtimeCleanupNoAdmin") return "当前清理计划只包含进程内资源释放，不需要管理员命令。";
            if (key == "adapterApplyConfirm") return "将通过管理员权限配置 LocalAreaInterconnection 虚拟网卡的 IP、MTU 和接口跃点。请确认这是你当前要用于联机的房间网段。选择“否”只显示预览，不修改系统。";
            if (key == "firewallApplyConfirm") return "将通过管理员权限添加 Windows 防火墙入站允许规则，用于游戏端口。选择“否”只显示预览，不修改系统。";
            if (key == "prepareLanConfirm") return "将请求管理员权限并依次创建/打开 Wintun 虚拟网卡、配置房间虚拟 IP、添加游戏端口和 runtime 端口防火墙规则。请只在准备开始联机测试时确认。";
            if (key == "coordinationFirewallConfirm") return "如果要让其他电脑连接到本机协调服务，需要添加 Windows 防火墙入站 TCP 规则。是否用管理员权限添加该规则？选择“否”仍会启动服务，但远端电脑可能连不上。";
            if (key == "coordinationFirewallSkipped") return "已跳过协调服务防火墙规则；如果远端连不上，请重新启动协调服务并允许管理员确认。";
            if (key == "prepareLanCancelled") return "已取消管理员准备步骤，未启动 runtime。";
            if (key == "prepareLanStarting") return "正在准备联机环境，请在弹出的管理员窗口中确认；准备过程可能需要几十秒。";
            if (key == "prepareLanFinished") return "联机环境准备结果:";
            if (key == "prepareLanFailed") return "联机环境准备未成功，已停止启动。请查看上面的退出码、错误输出或运行 Wintun 检测/网络诊断。";
            if (key == "adminActionPreviewOnly") return "已取消管理员执行；上面是预览输出，系统未被修改。";
            if (key == "adminActionStartFailed") return "无法启动管理员进程。";
            if (key == "adminActionCancelled") return "管理员权限请求已取消或启动失败。";
            if (key == "adminActionTimedOut") return "管理员命令超时，已尝试停止后台命令。";
            if (key == "adminActionFinished") return "管理员命令结束，退出码:";
            if (key == "adminActionStdout") return "标准输出:";
            if (key == "adminActionStderr") return "错误输出:";
            if (key == "runtimeRouteScan") return "路由扫描";
            if (key == "runtimeRouteCount") return "路由";
            if (key == "runtimeRoomRouteCount") return "房间路由";
            if (key == "runtimeRouteNoAction") return "未发现需要处理的房间路由。";
            if (key == "wintunStatus") return "Wintun";
            if (key == "wintunReadyNext") return "Wintun 环境可用于下一步 adapter/session 验证。";
            if (key == "coordinationAlreadyRunning") return "协调服务已在运行。";
            if (key == "coordinationStarted") return "协调服务已启动。";
            if (key == "coordinationStopped") return "协调服务已停止。";
            if (key == "coordinationNotRunning") return "协调服务当前没有运行。";
            if (key == "coordinationRunning") return "coordination: 运行中";
            if (key == "coordinationStoppedState") return "coordination: 已停止";
            if (key == "coordinationExited") return "协调服务进程已退出:";
            if (key == "coordinationServerUrl") return "协调服务: ";
            if (key == "coordinationStorePath") return "协调存储: ";
            if (key == "coordinationRoomStatus") return "房间";
            if (key == "coordinationOnline") return "在线";
            if (key == "coordinationExpired") return "过期";
            if (key == "coordinationLeft") return "已从协调房间离开:";
            if (key == "coordinationRoomClosed") return "协调房间已关闭:";
            if (key == "coordinationServerRequired") return "需要先填写或启动协调服务。";
            if (key == "internetCoordinationRequired") return "异地组网需要一个公网可访问的协调/中继服务地址；两台电脑都没有公网 IP 时，必须通过第三台公网服务器中转。";
            if (key == "internetCoordinationInvalid") return "这个协调服务地址不能用于异地组网:";
            if (key == "internetCoordinationBadUrl") return "请填写完整 HTTP 地址，例如 http://公网服务器IP:39110。";
            if (key == "internetCoordinationHttpOnly") return "当前内置协调服务只支持 HTTP；HTTPS 需要反向代理或后续补 TLS 支持。";
            if (key == "internetCoordinationPrivate") return "这是本机/私网/不可路由地址，只适合同局域网、端口映射或本机测试，不适合异地电脑直接使用。";
            if (key == "internetCoordinationExample") return "正确方向: 在 VPS/云服务器上运行协调/中继服务，然后两台电脑都填写 http://公网服务器IP:39110。";
            if (key == "coordinationPeerRequired") return "需要先填写远端 Peer。";
            if (key == "coordinationPeerKicked") return "已请求踢出远端 Peer:";
            if (key == "firewallRuleTried") return "已尝试添加防火墙入站规则:";
            if (key == "firewallRuleFailed") return "防火墙规则添加失败";
            if (key == "relaySelected") return "中继候选:";
            if (key == "textFilesFilter") return "文本文件 (*.txt)|*.txt|所有文件 (*.*)|*.*";
            if (key == "jsonFilesFilter") return "JSON 文件 (*.json)|*.json|所有文件 (*.*)|*.*";
            if (key == "missingCli") return "缺少 CLI 程序: ";
            if (key == "missingNativeCli") return "缺少 Rust 原生 CLI 程序，请先重新生成 exe: ";
            if (key == "missingGameCatalog") return "请先选择游戏模板库 JSON 文件。";
            if (key == "remoteOfferRequired") return "需要在远端 Peer 中填写远端 offer JSON、offer 文件路径，或先通过协调服务发布远端 offer。";
        }
        else
        {
            if (key == "appTitle") return "LocalAreaInterconnection";
            if (key == "appTagline") return "Virtual LAN for remote players";
            if (key == "navHome") return "Home / Room";
            if (key == "navDiagnose") return "Diagnostics";
            if (key == "navGames") return "Game profiles";
            if (key == "navTools") return "More tools";
            if (key == "navAbout") return "About";
            if (key == "aboutVersion") return "Version 0.1.0  ·  Rust core + Wintun";
            if (key == "aboutDesc") return "Creates a low-latency, diagnosable virtual LAN for PC games that only support LAN multiplayer. Prefers P2P, falls back to relay, room-isolated and end-to-end encrypted.";
            if (key == "roomName") return "Room name";
            if (key == "host") return "Host";
            if (key == "virtualSubnet") return "Virtual subnet";
            if (key == "myVirtualIp") return "My virtual IP";
            if (key == "gameName") return "Game name";
            if (key == "gameCatalog") return "Game catalog";
            if (key == "gamePorts") return "Game ports";
            if (key == "observedRules") return "Observed rules";
            if (key == "netshOutputFile") return "Netsh output file";
            if (key == "pingTarget") return "Ping target";
            if (key == "packetObservations") return "Packet observation file";
            if (key == "invite") return "Invite";
            if (key == "output") return "Command output / diagnostics";
            if (key == "outputHelp") return "Click a button above to show command output, plan JSON, or diagnostics here. Create room fills the invite automatically. Plan commands do not modify the system by default.";
            if (key == "quickHostRoom") return "Host room";
            if (key == "quickJoinRoom") return "Join friend";
            if (key == "startLanSession") return "Start LAN";
            if (key == "checkConnection") return "Check connection";
            if (key == "moreTools") return "More tools";
            if (key == "hideTools") return "Hide tools";
            if (key == "quickInviteCopied") return "Invite copied. Send it to your friend.";
            if (key == "quickInviteCopyFailed") return "Room created, but automatic copy failed. Copy the Invite field manually and send it to your friend.";
            if (key == "quickNextHost") return "Next: both sides click Create direct offer, paste each other's offer, then click Start LAN.";
            if (key == "quickJoinedNext") return "Invite decoded and room joined. Next: both sides click Create direct offer, paste each other's offer, then click Start LAN.";
            if (key == "quickLanStarting") return "Starting LAN components. Please wait.";
            if (key == "quickLanStarted") return "LAN components started. The other side also needs to join and click Start LAN; when both sides are ready, try the game LAN room.";
            if (key == "actionAlreadyRunning") return "The previous step is still running. Please wait.";
            if (key == "networkDiagnoseRunning") return "Checking connection. This may take a few seconds when the other side is offline or unreachable.";
            if (key == "networkDiagnoseDone") return "Connection check complete:";
            if (key == "commandTimedOut") return "Operation timed out. Make sure the other side has joined and started LAN, or try Check connection again later.";
            if (key == "summaryAdapter") return "Virtual adapter:";
            if (key == "summaryTunnel") return "Tunnel:";
            if (key == "summaryBroadcast") return "Broadcast discovery:";
            if (key == "summaryGame") return "Game traffic:";
            if (key == "summaryReadiness") return "Game readiness:";
            if (key == "summaryPath") return "Connection path:";
            if (key == "stateOk") return "OK";
            if (key == "stateNeedsAttention") return "Needs attention";
            if (key == "stateSkipped") return "Skipped";
            if (key == "stateFailed") return "Failed";
            if (key == "actionCouldNotFinish") return "This step did not finish.";
            if (key == "technicalSummary") return "Short reason:";
            if (key == "hostNeedsName") return "Fill Room name and Host first, then click Host room.";
            if (key == "joinNeedsInvite") return "Paste your friend's invite into the Invite field first, then click Join friend.";
            if (key == "startNeedsRoom") return "Host a room or paste an invite and join first, then click Start LAN.";
            if (key == "copyInviteNeedsRoom") return "Click Host room first to generate an invite, then copy it for your friend.";
            if (key == "tryMainFlowAgain") return "Use the main flow: Host room or Join friend, then Start LAN, then Check connection.";
            if (key == "clipboardCopyFailed") return "Copy to clipboard failed. Copy the text below manually:";
            if (key == "createRoom") return "Create room";
            if (key == "copyInvite") return "Copy invite";
            if (key == "copyIp") return "Copy my IP";
            if (key == "decodeInvite") return "Decode invite";
            if (key == "joinRoom") return "Join room";
            if (key == "adapterPlan") return "Adapter plan";
            if (key == "adapterScan") return "Adapter scan";
            if (key == "nativeAdapterEnsure") return "Native adapter check";
            if (key == "nativeAdapterApply") return "Apply adapter";
            if (key == "prepareLanEnvironment") return "Prepare LAN";
            if (key == "gamePlan") return "Game plan";
            if (key == "gameProfileList") return "Profile list";
            if (key == "gameProfilePlan") return "Profile game plan";
            if (key == "gamePortScan") return "Game port scan";
            if (key == "gameReadiness") return "game readiness";
            if (key == "gameReadinessCheck") return "Game readiness";
            if (key == "firewallPlan") return "Firewall plan";
            if (key == "firewallDiagnose") return "Firewall diagnose";
            if (key == "firewallScan") return "Firewall scan";
            if (key == "firewallApply") return "Apply firewall";
            if (key == "generalDiagnose") return "General diagnose";
            if (key == "networkDiagnose") return "Network diagnose";
            if (key == "exportDiagnostics") return "Export diagnostics";
            if (key == "udpTest") return "UDP test";
            if (key == "broadcastTest") return "Broadcast test";
            if (key == "nativeRuntimeSelfTest") return "Native tunnel self-test";
            if (key == "wintunDetect") return "Wintun detect";
            if (key == "wintunProbe") return "Wintun probe";
            if (key == "directOffer") return "Create direct offer";
            if (key == "directSelfTest") return "Direct self-test";
            if (key == "startRuntime") return "Start runtime";
            if (key == "stopRuntime") return "Stop runtime";
            if (key == "runtimeCleanupPlan") return "Cleanup plan";
            if (key == "runtimeCleanupApply") return "Apply cleanup";
            if (key == "routeScan") return "Route scan";
            if (key == "startCoordination") return "Start coordination";
            if (key == "stopCoordination") return "Stop coordination";
            if (key == "closeRoom") return "Close room";
            if (key == "kickPeer") return "Kick peer";
            if (key == "nativeNatSelfTest") return "NAT self-test";
            if (key == "relayFallbackPlan") return "Relay plan";
            if (key == "connectionPathPlan") return "Connection path";
            if (key == "tcpTest") return "TCP test";
            if (key == "browseGameCatalog") return "Browse catalog";
            if (key == "browseNetsh") return "Browse netsh";
            if (key == "browsePackets") return "Browse packets";
            if (key == "copyOutput") return "Copy output";
            if (key == "autoNetworkDiagnose") return "Network diagnostics refreshed:";
            if (key == "roomDetails") return "Room details";
            if (key == "detailRoom") return "Room:";
            if (key == "detailSubnet") return "Subnet:";
            if (key == "detailConnection") return "Connection:";
            if (key == "detailBroadcast") return "Broadcast:";
            if (key == "detailMembers") return "Members:";
            if (key == "detailNext") return "Next:";
            if (key == "detailAdapter") return "Adapter";
            if (key == "detailTunnel") return "Tunnel";
            if (key == "detailGameTraffic") return "Game traffic:";
            if (key == "detailGameProfile") return "Game profile";
            if (key == "gameProfileMatches") return "profile matches";
            if (key == "gameProfileSelected") return "Matched profile filled in; continue with game readiness.";
            if (key == "gameProfileNoMatch") return "No matching profile; adjust the game name or choose another catalog.";
            if (key == "detailCompatibility") return "Compatibility";
            if (key == "detailGamePorts") return "Ports:";
            if (key == "gamePortEndpoints") return "endpoints";
            if (key == "gamePortMatches") return "port matches";
            if (key == "detailPath") return "Path";
            if (key == "detailRelay") return "Relay";
            if (key == "detailHost") return "Host";
            if (key == "stateUnknown") return "unknown";
            if (key == "stateYes") return "yes";
            if (key == "stateNo") return "no";
            if (key == "connectionHostReady") return "Host mode, waiting for friends";
            if (key == "connectionJoined") return "Joined, waiting for connectivity checks";
            if (key == "connectionExported") return "Diagnostic bundle exported";
            if (key == "connectionClosed") return "Room closed";
            if (key == "nextCreateLanRoom") return "Copy the invite to friends, then start the game and create a LAN room.";
            if (key == "nextFindLanRoom") return "Open the game LAN page; run network diagnostics if the room is missing.";
            if (key == "nextJoinRoom") return "Click join room to get the suggested virtual IP.";
            if (key == "nextShareBundle") return "Review local configuration before sharing the diagnostic bundle.";
            if (key == "nextCreateOrJoin") return "Create a room or paste an invite to join one.";
            if (key == "nextFixAdapter") return "Check that the virtual adapter exists, is enabled, and has the room IP.";
            if (key == "nextFixTunnel") return "Check ping/P2P state; switch networks or try port forwarding if needed.";
            if (key == "nextCheckBroadcast") return "Check broadcast proxy rules and game discovery ports.";
            if (key == "nextStartGame") return "Start the game and confirm it binds to the virtual adapter.";
            if (key == "nextHealthy") return "Connectivity indicators look healthy; try the game LAN room.";
            if (key == "inviteCopied") return "Invite copied:";
            if (key == "ipCopied") return "Virtual IP copied:";
            if (key == "nothingToCopy") return "Nothing to copy.";
            if (key == "minimizeTip") return "Minimize";
            if (key == "maximizeTip") return "Maximize / restore";
            if (key == "closeTip") return "Close";
            if (key == "selectNetshOutput") return "Select netsh output";
            if (key == "selectGameCatalog") return "Select game catalog JSON";
            if (key == "selectPacketObservations") return "Select or create packet observation file";
            if (key == "saveDiagnosticBundle") return "Save diagnostic bundle";
            if (key == "runtimeAlreadyRunning") return "runtime is already running.";
            if (key == "runtimeStarted") return "runtime started and is writing snapshots and packet observations.";
            if (key == "runtimeStopped") return "runtime stopped.";
            if (key == "runtimeNotRunning") return "runtime is not running.";
            if (key == "runtimeRunning") return "runtime: running";
            if (key == "runtimeStoppedState") return "runtime: stopped";
            if (key == "runtimeExited") return "runtime process exited:";
            if (key == "runtimeSnapshotPath") return "Snapshot: ";
            if (key == "runtimeObservationPath") return "Packet observations: ";
            if (key == "runtimeSnapshotReady") return "Snapshot available for diagnostic export: ";
            if (key == "nativeOffer") return "Create offer";
            if (key == "coordinationServer") return "Coordination server";
            if (key == "stunServer") return "STUN discovery";
            if (key == "upnpPortMap") return "UPnP port map";
            if (key == "remotePeer") return "Remote peer / offer";
            if (key == "nativeOfferPath") return "Offer file: ";
            if (key == "directOfferReady") return "Direct offer created. Send the full text below to the other PC; they should paste it into Remote peer / offer.";
            if (key == "directOfferFailed") return "Failed to create direct offer. Create or join a room first and confirm the virtual IP.";
            if (key == "directOfferNext") return "After both sides paste each other's direct offer, click Start LAN at roughly the same time. If the NATs cannot connect without relay, diagnostics will show timeout or no-path.";
            if (key == "directSelfTestReady") return "Direct self-test result:";
            if (key == "directRemoteOfferRequired") return "Serverless direct mode needs exchanged direct offers: both sides should click Create direct offer, send the full text to each other, then paste it into Remote peer / offer.";
            if (key == "directBootstrapStarting") return "Trying serverless P2P direct connection:";
            if (key == "runtimePeers") return "peers";
            if (key == "runtimeHeartbeats") return "heartbeats";
            if (key == "runtimeSnapshots") return "snapshots";
            if (key == "runtimeConnectionPaths") return "connection paths";
            if (key == "runtimeLogTail") return "Recent runtime log:";
            if (key == "runtimeCleanup") return "cleanup";
            if (key == "runtimeCleanupSteps") return "steps";
            if (key == "runtimeCleanupChecks") return "checks";
            if (key == "runtimeCleanupActions") return "actions";
            if (key == "runtimeCleanupRestore") return "Restore adapter:";
            if (key == "runtimeCleanupCommands") return "commands";
            if (key == "runtimeCleanupConfirmed") return "confirmed";
            if (key == "runtimeCleanupUnsafe") return "blocked";
            if (key == "runtimeCleanupApplyConfirm") return "This will try to execute safe-listed commands from the runtime cleanup plan. Adapter and route cleanup usually require Administrator privileges. Choose No to preview only.";
            if (key == "runtimeCleanupNeedsAdmin") return "Adapter restore commands require an Administrator terminal; review the dry-run output first.";
            if (key == "runtimeCleanupNoAdmin") return "This cleanup plan only releases in-process runtime resources and does not need admin commands.";
            if (key == "adapterApplyConfirm") return "This will configure the LocalAreaInterconnection virtual adapter IP, MTU, and interface metric with Administrator privileges. Confirm that this is the room subnet you want to test. Choose No to preview only.";
            if (key == "firewallApplyConfirm") return "This will add Windows Firewall inbound allow rules for the game ports with Administrator privileges. Choose No to preview only.";
            if (key == "prepareLanConfirm") return "This will request Administrator privileges, then create/open the Wintun virtual adapter, assign the room virtual IP, and add game/runtime firewall rules. Confirm only when you are ready to test LAN connectivity.";
            if (key == "coordinationFirewallConfirm") return "Other PCs need an inbound Windows Firewall TCP rule to reach this local coordination server. Add it with Administrator privileges? Choosing No still starts the server, but remote PCs may not connect.";
            if (key == "coordinationFirewallSkipped") return "Coordination firewall rule was skipped; if remote PCs cannot connect, restart coordination and approve the Administrator prompt.";
            if (key == "prepareLanCancelled") return "Administrator preparation was cancelled; runtime was not started.";
            if (key == "prepareLanStarting") return "Preparing LAN environment. Confirm the Administrator prompt; this may take several seconds.";
            if (key == "prepareLanFinished") return "LAN preparation result:";
            if (key == "prepareLanFailed") return "LAN preparation did not complete successfully, so runtime was not started. Check the exit code, stderr, Wintun detect, or network diagnostics above.";
            if (key == "adminActionPreviewOnly") return "Administrator execution was cancelled; the text above is a preview and the system was not modified.";
            if (key == "adminActionStartFailed") return "Could not start the Administrator process.";
            if (key == "adminActionCancelled") return "Administrator request was cancelled or failed to start.";
            if (key == "adminActionTimedOut") return "Administrator command timed out; attempted to stop the background command.";
            if (key == "adminActionFinished") return "Administrator command finished, exit code:";
            if (key == "adminActionStdout") return "stdout:";
            if (key == "adminActionStderr") return "stderr:";
            if (key == "runtimeRouteScan") return "route scan";
            if (key == "runtimeRouteCount") return "routes";
            if (key == "runtimeRoomRouteCount") return "room routes";
            if (key == "runtimeRouteNoAction") return "No room route needs attention.";
            if (key == "wintunStatus") return "Wintun";
            if (key == "wintunReadyNext") return "Wintun environment is ready for adapter/session verification.";
            if (key == "coordinationAlreadyRunning") return "coordination server is already running.";
            if (key == "coordinationStarted") return "coordination server started.";
            if (key == "coordinationStopped") return "coordination server stopped.";
            if (key == "coordinationNotRunning") return "coordination server is not running.";
            if (key == "coordinationRunning") return "coordination: running";
            if (key == "coordinationStoppedState") return "coordination: stopped";
            if (key == "coordinationExited") return "coordination server process exited:";
            if (key == "coordinationServerUrl") return "Coordination server: ";
            if (key == "coordinationStorePath") return "Coordination store: ";
            if (key == "coordinationRoomStatus") return "room";
            if (key == "coordinationOnline") return "online";
            if (key == "coordinationExpired") return "expired";
            if (key == "coordinationLeft") return "Left coordination room:";
            if (key == "coordinationRoomClosed") return "Coordination room closed:";
            if (key == "coordinationServerRequired") return "Fill or start the coordination server first.";
            if (key == "internetCoordinationRequired") return "Remote Internet LAN mode requires a publicly reachable coordination/relay server; if both PCs have no public IP, they must use a third public server.";
            if (key == "internetCoordinationInvalid") return "This coordination server cannot be used for remote Internet LAN mode:";
            if (key == "internetCoordinationBadUrl") return "Enter a full HTTP URL, for example http://public-server-ip:39110.";
            if (key == "internetCoordinationHttpOnly") return "The built-in coordination server currently supports HTTP only; HTTPS needs a reverse proxy or later TLS support.";
            if (key == "internetCoordinationPrivate") return "This is a local/private/non-routable address. It is only suitable for same-LAN, port-forwarding, or local testing, not remote PCs.";
            if (key == "internetCoordinationExample") return "Correct setup: run coordination/relay on a VPS or cloud server, then both PCs use http://public-server-ip:39110.";
            if (key == "coordinationPeerRequired") return "Fill the remote peer first.";
            if (key == "coordinationPeerKicked") return "Requested remote peer kick:";
            if (key == "firewallRuleTried") return "Tried to add inbound firewall rule:";
            if (key == "firewallRuleFailed") return "Failed to add firewall rule";
            if (key == "relaySelected") return "Relay candidate:";
            if (key == "textFilesFilter") return "Text files (*.txt)|*.txt|All files (*.*)|*.*";
            if (key == "jsonFilesFilter") return "JSON files (*.json)|*.json|All files (*.*)|*.*";
            if (key == "missingCli") return "Missing CLI executable: ";
            if (key == "missingNativeCli") return "Missing Rust native CLI executable. Build the latest exe first: ";
            if (key == "missingGameCatalog") return "Select a game catalog JSON file first.";
            if (key == "remoteOfferRequired") return "Fill Remote peer with remote offer JSON, an offer file path, or publish the remote offer through coordination first.";
        }
        return key;
    }

    string Quote(string value)
    {
        return "\"" + value.Replace("\"", "\\\"") + "\"";
    }

}

