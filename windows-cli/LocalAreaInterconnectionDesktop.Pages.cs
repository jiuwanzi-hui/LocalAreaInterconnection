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
    void SelectPage(string page)
    {
        activePage = page;
        foreach (Panel p in contentPages)
        {
            p.Visible = false;
        }
        Panel target = page == "home" ? pageHome
            : page == "diagnose" ? pageDiagnose
            : page == "games" ? pageGames
            : page == "tools" ? pageTools
            : pageAbout;
        if (target != null)
        {
            target.Visible = true;
            target.BringToFront();
        }
        // make sure shared output console is visible on top of nothing
        foreach (Button b in navButtons) b.Invalidate();
    }

    // ===== Pages =====
    void BuildPages()
    {
        BuildHomePage();
        BuildDiagnosePage();
        BuildGamesPage();
        BuildToolsPage();
        BuildAboutPage();
        EnsureHiddenRuntimeFields();

        output = new ThemedOutputBox();
        output.Dock = DockStyle.Fill;
        output.Font = new Font("Cascadia Mono", 10f, FontStyle.Regular);

        // Shared output console docked at the bottom of every page via a wrapper.
        // We attach a copy reference so each page can show it, but only one
        // instance exists (output). Pages each get their own frame container.
        // Simpler: a single shared output frame added to contentArea on top.
        Panel outputHost = new Panel();
        outputHost.Dock = DockStyle.Bottom;
        outputHost.Height = 190;
        outputHost.Padding = new Padding(16, 2, 16, 12);
        outputHost.BackColor = Color.Transparent;
        Label outputLabel = Label("output");
        outputLabel.Dock = DockStyle.Top;
        outputLabel.Height = 18;
        Panel outputFrame = Framed(output);
        outputFrame.Dock = DockStyle.Fill;
        outputFrame.BackColor = CardDark;
        outputFrame.Margin = new Padding(0, 2, 0, 0);
        outputHost.Controls.Add(outputFrame);
        outputHost.Controls.Add(outputLabel);
        contentArea.Controls.Add(outputHost);
        outputHost.BringToFront();
    }

    void EnsureHiddenRuntimeFields()
    {
        if (subnet == null) subnet = HiddenTextBox("10.77.12.0/24");
        if (ip == null) ip = HiddenTextBox("10.77.12.2");
        if (gameName == null) gameName = HiddenTextBox("Generic UDP Broadcast LAN Game");
        if (gameCatalog == null) gameCatalog = HiddenTextBox(DefaultGameCatalogPath());
        if (ports == null) ports = HiddenTextBox("27015");
        if (observed == null) observed = HiddenTextBox("udp:27015");
        if (netshOutput == null) netshOutput = HiddenTextBox("");
        if (pingTarget == null) pingTarget = HiddenTextBox("127.0.0.1");
        if (packetObservations == null) packetObservations = HiddenTextBox("");
        if (coordinationServer == null) coordinationServer = HiddenTextBox("");
        if (stunServer == null) stunServer = HiddenTextBox("");
        if (upnpPortMap == null) upnpPortMap = HiddenTextBox("false");
        if (remotePeer == null) remotePeer = HiddenTextBox("");
    }

    TextBox HiddenTextBox(string value)
    {
        TextBox box = new TextBox();
        box.Text = value;
        box.Visible = false;
        Controls.Add(box);
        return box;
    }

    TableLayoutPanel NewFieldTable()
    {
        TableLayoutPanel t = new TableLayoutPanel();
        t.Dock = DockStyle.Fill;
        t.BackColor = Color.Transparent;
        t.ColumnCount = 3;
        t.ColumnStyles.Add(new ColumnStyle(SizeType.Absolute, 200));
        t.ColumnStyles.Add(new ColumnStyle(SizeType.Percent, 100));
        t.ColumnStyles.Add(new ColumnStyle(SizeType.Absolute, 8));
        return t;
    }

    void AddFieldRow(TableLayoutPanel t, int row, string key, string value)
    {
        while (t.RowCount <= row + 1)
        {
            t.RowCount++;
            t.RowStyles.Add(new RowStyle(SizeType.Absolute, 34));
        }
        t.Controls.Add(Label(key), 0, row);
        TextBox box = new TextBox();
        box.Dock = DockStyle.Fill;
        box.Text = value;
        StyleTextBox(box);
        Panel frame = Framed(box);
        t.Controls.Add(frame, 1, row);
        // keep field references aligned with the old field members
        switch (key)
        {
            case "roomName": roomName = box; break;
            case "host": hostName = box; break;
            case "virtualSubnet": subnet = box; break;
            case "myVirtualIp": ip = box; break;
            case "gameName": gameName = box; break;
            case "gameCatalog": gameCatalog = box; break;
            case "gamePorts": ports = box; break;
            case "observedRules": observed = box; break;
            case "netshOutputFile": netshOutput = box; break;
            case "pingTarget": pingTarget = box; break;
            case "packetObservations": packetObservations = box; break;
            case "coordinationServer": coordinationServer = box; break;
            case "stunServer": stunServer = box; break;
            case "upnpPortMap": upnpPortMap = box; break;
            case "remotePeer": remotePeer = box; break;
            case "invite": invite = box; break;
        }
    }

    Panel CardPanel()
    {
        Panel outer = new Panel();
        outer.Dock = DockStyle.Fill;
        outer.BackColor = CardBorder;
        outer.Padding = new Padding(1);
        outer.Margin = new Padding(16, 12, 16, 8);
        Panel inner = new Panel();
        inner.Dock = DockStyle.Fill;
        inner.BackColor = CardDark;
        outer.Controls.Add(inner);
        outer.Resize += delegate { ApplyRoundedRegion(outer, 12); };
        ApplyRoundedRegion(outer, 12);
        outer.Tag = inner;
        return outer;
    }

    Label PageTitle(string key)
    {
        Label title = new Label();
        title.Text = T(key);
        title.Font = new Font(Font.FontFamily, 13f, FontStyle.Bold);
        title.ForeColor = TextBright;
        title.BackColor = Color.Transparent;
        title.Height = 30;
        title.TextAlign = ContentAlignment.BottomLeft;
        labelControls[key] = title;
        return title;
    }

    Label AddPageTitle(Panel page, string key)
    {
        Label title = PageTitle(key);
        title.SetBounds(0, 0, page.ClientSize.Width, 40);
        title.Anchor = AnchorStyles.Top | AnchorStyles.Left | AnchorStyles.Right;
        title.Padding = new Padding(16, 8, 0, 0);
        page.Controls.Add(title);
        return title;
    }

    Panel AddPageContent(Panel page, Label title)
    {
        Panel content = new Panel();
        content.BackColor = Color.Transparent;
        Action layout = delegate
        {
            int top = title.Bottom;
            int height = Math.Max(1, page.ClientSize.Height - top - page.Padding.Bottom);
            content.SetBounds(0, top, page.ClientSize.Width, height);
            title.Width = page.ClientSize.Width;
        };
        page.Controls.Add(content);
        page.Resize += delegate { layout(); };
        layout();
        content.SendToBack();
        title.BringToFront();
        return content;
    }

    void BuildHomePage()
    {
        pageHome = new Panel();
        pageHome.Dock = DockStyle.Fill;
        pageHome.BackColor = Color.Transparent;
        pageHome.Padding = new Padding(0, 0, 0, 202);
        contentArea.Controls.Add(pageHome);
        contentPages.Add(pageHome);

        Label title = AddPageTitle(pageHome, "navHome");
        Panel pageContent = AddPageContent(pageHome, title);

        TableLayoutPanel body = new TableLayoutPanel();
        body.Dock = DockStyle.Fill;
        body.ColumnCount = 2;
        body.RowCount = 1;
        body.ColumnStyles.Add(new ColumnStyle(SizeType.Percent, 58));
        body.ColumnStyles.Add(new ColumnStyle(SizeType.Percent, 42));
        body.Padding = new Padding(16, 0, 16, 12);
        pageContent.Controls.Add(body);

        // Left: quick-flow card (fields + main actions)
        Panel leftCard = CardPanel();
        Panel leftInner = (Panel)leftCard.Tag;

        TableLayoutPanel quickLayout = new TableLayoutPanel();
        quickLayout.Dock = DockStyle.Fill;
        quickLayout.BackColor = Color.Transparent;
        quickLayout.ColumnCount = 1;
        quickLayout.RowCount = 3;
        quickLayout.RowStyles.Add(new RowStyle(SizeType.Absolute, 232));
        quickLayout.RowStyles.Add(new RowStyle(SizeType.Absolute, 136));
        quickLayout.RowStyles.Add(new RowStyle(SizeType.Percent, 100));
        leftInner.Controls.Add(quickLayout);

        TableLayoutPanel leftTable = NewFieldTable();
        leftTable.RowCount = 0;
        leftTable.Padding = new Padding(18, 14, 18, 14);
        leftTable.Dock = DockStyle.Fill;
        leftTable.Margin = new Padding(0);
        AddFieldRow(leftTable, 0, "roomName", "Friday LAN");
        AddFieldRow(leftTable, 1, "host", "Alice");
        AddFieldRow(leftTable, 2, "invite", "");
        AddFieldRow(leftTable, 3, "coordinationServer", "");
        AddFieldRow(leftTable, 4, "remotePeer", "");
        AddFieldRow(leftTable, 5, "stunServer", "stun.l.google.com:19302,stun1.l.google.com:19302");
        AddFieldRow(leftTable, 6, "upnpPortMap", "false");
        leftTable.RowCount = 7;
        leftTable.RowStyles.Add(new RowStyle(SizeType.Absolute, 34));
        leftTable.RowStyles.Add(new RowStyle(SizeType.Absolute, 34));
        leftTable.RowStyles.Add(new RowStyle(SizeType.Absolute, 34));
        leftTable.RowStyles.Add(new RowStyle(SizeType.Absolute, 34));
        leftTable.RowStyles.Add(new RowStyle(SizeType.Absolute, 34));
        leftTable.RowStyles.Add(new RowStyle(SizeType.Absolute, 34));
        leftTable.RowStyles.Add(new RowStyle(SizeType.Absolute, 34));
        leftTable.GrowStyle = TableLayoutPanelGrowStyle.AddRows;
        quickLayout.Controls.Add(leftTable, 0, 0);

        // quick action row inside left card
        FlowLayoutPanel quickActions = new FlowLayoutPanel();
        quickActions.BackColor = Color.Transparent;
        quickActions.WrapContents = true;
        quickActions.Dock = DockStyle.Fill;
        quickActions.Padding = new Padding(18, 0, 18, 14);
        AddButton(quickActions, "quickHostRoom", delegate { QuickHostRoom(); });
        AddButton(quickActions, "quickJoinRoom", delegate { QuickJoinRoom(); });
        AddButton(quickActions, "directOffer", delegate { RunDirectOffer(); });
        AddButton(quickActions, "directSelfTest", delegate { RunDirectSelfTest(); });
        AddButton(quickActions, "startLanSession", delegate { StartLanSession(); });
        AddButton(quickActions, "checkConnection", delegate { RunNetworkDiagnose(); });
        moreToolsButton = AddButton(quickActions, "moreTools", delegate { SelectPage("tools"); });
        quickLayout.Controls.Add(quickActions, 0, 1);

        body.Controls.Add(leftCard, 0, 0);

        // Right: room details card
        Panel details = RoomDetailsPanel();
        details.Margin = new Padding(8, 12, 16, 8);
        body.Controls.Add(details, 1, 0);
    }

    void BuildDiagnosePage()
    {
        pageDiagnose = new Panel();
        pageDiagnose.Dock = DockStyle.Fill;
        pageDiagnose.BackColor = Color.Transparent;
        pageDiagnose.Padding = new Padding(0, 0, 0, 202);
        contentArea.Controls.Add(pageDiagnose);
        contentPages.Add(pageDiagnose);

        Label title = AddPageTitle(pageDiagnose, "navDiagnose");
        Panel pageContent = AddPageContent(pageDiagnose, title);

        Panel card = CardPanel();
        Panel inner = (Panel)card.Tag;
        TableLayoutPanel fields = NewFieldTable();
        fields.Padding = new Padding(18, 14, 18, 14);
        AddFieldRow(fields, 0, "myVirtualIp", "10.77.12.2");
        AddFieldRow(fields, 1, "pingTarget", "127.0.0.1");
        AddFieldRow(fields, 2, "netshOutputFile", "");
        AddFieldRow(fields, 3, "packetObservations", "");
        AddFieldRow(fields, 4, "remotePeer", "");
        inner.Controls.Add(fields);

        FlowLayoutPanel actions = new FlowLayoutPanel();
        actions.BackColor = Color.Transparent;
        actions.WrapContents = true;
        actions.Dock = DockStyle.Bottom;
        actions.Height = 150;
        actions.Padding = new Padding(18, 0, 18, 14);
        AddButton(actions, "checkConnection", delegate { RunNetworkDiagnose(); });
        AddButton(actions, "networkDiagnose", delegate { RunNetworkDiagnose(); }, true);
        AddButton(actions, "generalDiagnose", delegate { RunNativeCli("diagnose --p2p ok --firewall allowed"); }, true);
        AddButton(actions, "firewallDiagnose", delegate { RunNativeCli(FirewallDiagnoseArgs()); }, true);
        AddButton(actions, "firewallScan", delegate { RunNativeCli(FirewallDiagnoseArgs()); }, true);
        AddButton(actions, "udpTest", delegate { RunUdpTest(); }, true);
        AddButton(actions, "tcpTest", delegate { RunTcpTest(); }, true);
        AddButton(actions, "broadcastTest", delegate { RunBroadcastTest(); }, true);
        AddButton(actions, "exportDiagnostics", delegate { ExportDiagnostics(); }, true);
        AddButton(actions, "browseNetsh", delegate { BrowseNetshOutput(); }, true);
        AddButton(actions, "browsePackets", delegate { BrowsePacketObservations(); }, true);
        inner.Controls.Add(actions);

        pageContent.Controls.Add(card);
    }

    void BuildGamesPage()
    {
        pageGames = new Panel();
        pageGames.Dock = DockStyle.Fill;
        pageGames.BackColor = Color.Transparent;
        pageGames.Padding = new Padding(0, 0, 0, 202);
        contentArea.Controls.Add(pageGames);
        contentPages.Add(pageGames);

        Label title = AddPageTitle(pageGames, "navGames");
        Panel pageContent = AddPageContent(pageGames, title);

        Panel card = CardPanel();
        Panel inner = (Panel)card.Tag;
        TableLayoutPanel fields = NewFieldTable();
        fields.Padding = new Padding(18, 14, 18, 14);
        AddFieldRow(fields, 0, "gameName", "Generic UDP Broadcast LAN Game");
        AddFieldRow(fields, 1, "gameCatalog", DefaultGameCatalogPath());
        AddFieldRow(fields, 2, "gamePorts", "27015");
        AddFieldRow(fields, 3, "virtualSubnet", "10.77.12.0/24");
        AddFieldRow(fields, 4, "observedRules", "udp:27015");
        inner.Controls.Add(fields);

        FlowLayoutPanel actions = new FlowLayoutPanel();
        actions.BackColor = Color.Transparent;
        actions.WrapContents = true;
        actions.Dock = DockStyle.Bottom;
        actions.Height = 150;
        actions.Padding = new Padding(18, 0, 18, 14);
        AddButton(actions, "gamePlan", delegate { RunNativeCli("game-plan --game-name " + Quote(gameName.Text) + " --subnet " + subnet.Text + " --ports " + ports.Text); });
        AddButton(actions, "gameProfileList", delegate { RunGameProfileList(); }, true);
        AddButton(actions, "gameProfilePlan", delegate { RunGameProfilePlan(); }, true);
        AddButton(actions, "gamePortScan", delegate { RunGamePortScan(); }, true);
        AddButton(actions, "gameReadinessCheck", delegate { RunGameReadinessCheck(); }, true);
        AddButton(actions, "firewallPlan", delegate { RunNativeCli("firewall-plan --game-name " + Quote(gameName.Text) + GameCatalogArgs() + " --subnet " + subnet.Text + " --ports " + ports.Text); }, true);
        AddButton(actions, "browseGameCatalog", delegate { BrowseGameCatalog(); }, true);
        inner.Controls.Add(actions);

        pageContent.Controls.Add(card);
    }

    void BuildToolsPage()
    {
        pageTools = new Panel();
        pageTools.Dock = DockStyle.Fill;
        pageTools.BackColor = Color.Transparent;
        pageTools.Padding = new Padding(0, 0, 0, 202);
        contentArea.Controls.Add(pageTools);
        contentPages.Add(pageTools);

        Label title = AddPageTitle(pageTools, "navTools");
        Panel pageContent = AddPageContent(pageTools, title);

        Panel card = CardPanel();
        Panel inner = (Panel)card.Tag;

        // action host with scroll (reuse existing scroll machinery)
        actionsHost = new Panel();
        actionsHost.Dock = DockStyle.Fill;
        actionsHost.BackColor = CardDark;
        actionsHost.Padding = new Padding(12);
        actionsHost.Margin = new Padding(0);
        actionsHost.MouseWheel += ScrollActionsWheel;

        actionsViewport = new Panel();
        actionsViewport.Dock = DockStyle.Fill;
        actionsViewport.BackColor = CardDark;
        actionsViewport.MouseWheel += ScrollActionsWheel;
        actionsViewport.Resize += delegate { AdjustActionLayout(); };

        actionsPanel = new FlowLayoutPanel();
        actionsPanel.BackColor = Color.Transparent;
        actionsPanel.Padding = new Padding(0);
        actionsPanel.Margin = new Padding(0);
        actionsPanel.WrapContents = true;
        actionsPanel.AutoScroll = false;
        actionsPanel.MouseWheel += ScrollActionsWheel;
        actionsViewport.Controls.Add(actionsPanel);

        actionScrollBar = new Panel();
        actionScrollBar.Dock = DockStyle.Right;
        actionScrollBar.Width = 10;
        actionScrollBar.BackColor = CardDark;
        actionScrollBar.MouseWheel += ScrollActionsWheel;
        actionScrollBar.Paint += PaintActionScrollBar;

        actionScrollThumb = new Panel();
        actionScrollThumb.Left = 2;
        actionScrollThumb.Width = 6;
        actionScrollThumb.BackColor = AccentCyan;
        actionScrollThumb.Cursor = Cursors.Hand;
        actionScrollThumb.Resize += delegate { ApplyRoundedRegion(actionScrollThumb, 4); };
        actionScrollThumb.MouseDown += BeginActionScrollThumbDrag;
        actionScrollThumb.MouseMove += DragActionScrollThumb;
        actionScrollThumb.MouseUp += EndActionScrollThumbDrag;
        actionScrollBar.Controls.Add(actionScrollThumb);

        actionsHost.Controls.Add(actionsViewport);
        actionsHost.Controls.Add(actionScrollBar);
        inner.Controls.Add(actionsHost);

        // All tools here, fully visible (advanced = false so they are not
        // hidden by the old "more tools" toggle, but styled as secondary).
        AddToolButton("createRoom", delegate { CreateRoom(); });
        AddToolButton("copyInvite", delegate { CopyInvite(); });
        AddToolButton("copyIp", delegate { CopyVirtualIp(); });
        AddToolButton("decodeInvite", delegate { DecodeInvite(); });
        AddToolButton("joinRoom", delegate { JoinRoom(); });
        AddToolButton("adapterPlan", delegate { RunNativeCli("adapter-plan --adapter-name LocalAreaInterconnection --subnet " + subnet.Text + " --ip " + ip.Text); });
        AddToolButton("adapterScan", delegate { RunNativeAdapterEnsure(); });
        AddToolButton("nativeAdapterEnsure", delegate { RunNativeAdapterEnsure(); });
        AddToolButton("nativeRuntimeSelfTest", delegate { RunNativeRuntimeSelfTest(); });
        AddToolButton("wintunDetect", delegate { RunWintunDetect(); });
        AddToolButton("wintunProbe", delegate { RunWintunSessionProbe(); });
        AddToolButton("directOffer", delegate { RunDirectOffer(); });
        AddToolButton("directSelfTest", delegate { RunDirectSelfTest(); });
        AddToolButton("nativeOffer", delegate { RunNativeOffer(); });
        AddToolButton("startCoordination", delegate { StartLocalCoordinationServer(); });
        AddToolButton("stopCoordination", delegate { StopLocalCoordinationServer(); });
        AddToolButton("startRuntime", delegate { StartNativeRuntime(); });
        AddToolButton("stopRuntime", delegate { StopNativeRuntime(); });
        AddToolButton("runtimeCleanupPlan", delegate { RunRuntimeCleanupPlan(); });
        AddToolButton("runtimeCleanupApply", delegate { RunRuntimeCleanupApply(); });
        AddToolButton("routeScan", delegate { RunRouteScan(); });
        AddToolButton("closeRoom", delegate { CloseCoordinationRoom(); });
        AddToolButton("kickPeer", delegate { KickCoordinationPeer(); });
        AddToolButton("nativeNatSelfTest", delegate { RunNativeNatSelfTest(); });
        AddToolButton("relayFallbackPlan", delegate { RunRelayFallbackPlan(); });
        AddToolButton("connectionPathPlan", delegate { RunConnectionPathPlan(); });
        AddToolButton("copyOutput", delegate { if (output.Text.Length > 0) Clipboard.SetText(output.Text); });

        // Tools page buttons are secondary styled but always visible.
        pageContent.Controls.Add(card);
    }

    void AddToolButton(string key, EventHandler handler)
    {
        Button button = new ClearTextButton();
        button.Text = T(key);
        button.Width = Math.Min(184, Math.Max(116, TextRenderer.MeasureText(button.Text, Font).Width + 24));
        button.Height = 30;
        button.Margin = new Padding(0, 0, 8, 8);
        button.Font = new Font(Font, FontStyle.Regular);
        StyleClearButton((ClearTextButton)button, true);
        button.Click += delegate(object sender, EventArgs e) { RunUserAction(key, handler, sender, e); };
        button.MouseWheel += ScrollActionsWheel;
        buttonControls[key] = button;
        actionsPanel.Controls.Add(button);
    }

    void BuildAboutPage()
    {
        pageAbout = new Panel();
        pageAbout.Dock = DockStyle.Fill;
        pageAbout.BackColor = Color.Transparent;
        pageAbout.Padding = new Padding(0, 0, 0, 202);
        contentArea.Controls.Add(pageAbout);
        contentPages.Add(pageAbout);

        Label title = AddPageTitle(pageAbout, "navAbout");
        Panel pageContent = AddPageContent(pageAbout, title);

        Panel card = CardPanel();
        Panel inner = (Panel)card.Tag;
        TableLayoutPanel layout = new TableLayoutPanel();
        layout.Dock = DockStyle.Fill;
        layout.ColumnCount = 1;
        layout.RowCount = 3;
        layout.RowStyles.Add(new RowStyle(SizeType.Absolute, 36));
        layout.RowStyles.Add(new RowStyle(SizeType.Absolute, 80));
        layout.RowStyles.Add(new RowStyle(SizeType.Percent, 100));
        layout.Padding = new Padding(20, 14, 20, 14);
        Label appName = new Label();
        appName.Text = T("appTitle");
        appName.Font = new Font(Font.FontFamily, 14f, FontStyle.Bold);
        appName.ForeColor = AccentCyan;
        appName.BackColor = Color.Transparent;
        appName.Dock = DockStyle.Fill;
        appName.TextAlign = ContentAlignment.MiddleLeft;
        layout.Controls.Add(appName, 0, 0);
        Label version = new Label();
        version.Text = T("aboutVersion");
        version.ForeColor = TextMuted;
        version.BackColor = Color.Transparent;
        version.Dock = DockStyle.Fill;
        version.TextAlign = ContentAlignment.TopLeft;
        layout.Controls.Add(version, 0, 1);
        Label desc = new Label();
        desc.Text = T("aboutDesc");
        desc.ForeColor = TextBright;
        desc.BackColor = Color.Transparent;
        desc.Dock = DockStyle.Fill;
        desc.TextAlign = ContentAlignment.TopLeft;
        layout.Controls.Add(desc, 0, 2);
        inner.Controls.Add(layout);

        pageContent.Controls.Add(card);
    }

}

