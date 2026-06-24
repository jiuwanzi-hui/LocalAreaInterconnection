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
        outputHost.Height = OutputHostHeight();
        outputHost.Padding = new Padding(16, 2, 16, 12);
        outputHost.BackColor = Color.Transparent;
        Label outputLabel = Label("output");
        outputLabel.Dock = DockStyle.Top;
        outputLabel.Height = 18;
        Panel outputFrame = Framed(output);
        outputFrame.Dock = DockStyle.Fill;
        outputFrame.BackColor = CardDark;
        outputFrame.Margin = new Padding(0, 2, 0, 0);
        outputHost.MouseEnter += delegate { output.FocusTextBoxForWheel(); };
        outputHost.MouseWheel += delegate(object sender, MouseEventArgs e) { output.ScrollWheelDelta(e.Delta); };
        outputFrame.MouseEnter += delegate { output.FocusTextBoxForWheel(); };
        outputFrame.MouseWheel += delegate(object sender, MouseEventArgs e) { output.ScrollWheelDelta(e.Delta); };
        outputHost.Controls.Add(outputFrame);
        outputHost.Controls.Add(outputLabel);
        contentArea.Controls.Add(outputHost);
        contentArea.Resize += delegate { outputHost.Height = OutputHostHeight(); };
        outputHost.BringToFront();
    }

    int OutputHostHeight()
    {
        int height = contentArea == null ? 260 : contentArea.ClientSize.Height;
        return Math.Max(250, Math.Min(360, height * 34 / 100));
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
        if (coordinationServer == null) coordinationServer = HiddenTextBox(DefaultCoordinationServer());
        if (relayServer == null) relayServer = HiddenTextBox(DefaultRelayServer());
        if (stunServer == null) stunServer = HiddenTextBox(DefaultStunServer());
        if (upnpPortMap == null) upnpPortMap = HiddenTextBox("true");
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
        box.Text = FieldInitialText(key, value);
        StyleTextBox(box);
        Panel frame = Framed(box);
        t.Controls.Add(frame, 1, row);
        BindFieldTextBox(key, box);
    }

    string FieldInitialText(string key, string fallback)
    {
        TextBox current = FieldReference(key);
        if (current != null)
        {
            return current.Text;
        }
        return fallback;
    }

    TextBox FieldReference(string key)
    {
        switch (key)
        {
            case "roomName": return roomName;
            case "host": return hostName;
            case "virtualSubnet": return subnet;
            case "myVirtualIp": return ip;
            case "gameName": return gameName;
            case "gameCatalog": return gameCatalog;
            case "gamePorts": return ports;
            case "observedRules": return observed;
            case "netshOutputFile": return netshOutput;
            case "pingTarget": return pingTarget;
            case "packetObservations": return packetObservations;
            case "coordinationServer": return coordinationServer;
            case "relayServer": return relayServer;
            case "stunServer": return stunServer;
            case "upnpPortMap": return upnpPortMap;
            case "remotePeer": return remotePeer;
            case "invite": return invite;
        }
        return null;
    }

    void SetFieldReference(string key, TextBox box)
    {
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
            case "relayServer": relayServer = box; break;
            case "stunServer": stunServer = box; break;
            case "upnpPortMap": upnpPortMap = box; break;
            case "remotePeer": remotePeer = box; break;
            case "invite": invite = box; break;
        }
    }

    void BindFieldTextBox(string key, TextBox box)
    {
        if (!fieldTextBoxes.ContainsKey(key))
        {
            fieldTextBoxes[key] = new List<TextBox>();
            SetFieldReference(key, box);
        }
        fieldTextBoxes[key].Add(box);
        box.TextChanged += delegate
        {
            SyncFieldText(key, box);
        };
    }

    void SyncFieldText(string key, TextBox source)
    {
        if (syncingFieldTextBoxes) return;
        if (!fieldTextBoxes.ContainsKey(key)) return;
        syncingFieldTextBoxes = true;
        try
        {
            for (int i = 0; i < fieldTextBoxes[key].Count; i++)
            {
                TextBox box = fieldTextBoxes[key][i];
                if (box != source && box.Text != source.Text)
                {
                    box.Text = source.Text;
                }
            }
        }
        finally
        {
            syncingFieldTextBoxes = false;
        }
    }

    Panel CardPanel()
    {
        Panel outer = new Panel();
        outer.Dock = DockStyle.Fill;
        outer.BackColor = CardShadow;
        outer.Padding = new Padding(3, 3, 5, 5);
        outer.Margin = new Padding(16, 12, 16, 8);
        outer.Paint += PaintRaisedCardFrame;
        Panel inner = new Panel();
        inner.Dock = DockStyle.Fill;
        inner.BackColor = CardDark;
        outer.Controls.Add(inner);
        outer.Resize += delegate { ApplyRoundedRegion(outer, 12); };
        inner.Resize += delegate { ApplyRoundedRegion(inner, 10); };
        ApplyRoundedRegion(outer, 12);
        ApplyRoundedRegion(inner, 10);
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
        pageHome.Padding = new Padding(0);
        contentArea.Controls.Add(pageHome);
        contentPages.Add(pageHome);

        Label title = AddPageTitle(pageHome, "navHome");
        Panel pageContent = AddPageContent(pageHome, title);

        Panel body = new Panel();
        body.Dock = DockStyle.Fill;
        body.BackColor = Color.Transparent;
        pageContent.Controls.Add(body);

        // Left: quick-flow fields card.
        Panel leftCard = CardPanel();
        leftCard.Dock = DockStyle.None;
        Panel leftInner = (Panel)leftCard.Tag;

        homeFieldTable = NewFieldTable();
        homeFieldTable.RowCount = 0;
        homeFieldTable.Padding = new Padding(18, 14, 18, 14);
        homeFieldTable.Margin = new Padding(0);
        AddFieldRow(homeFieldTable, 0, "roomName", "Friday LAN");
        AddFieldRow(homeFieldTable, 1, "host", DefaultHostName());
        AddFieldRow(homeFieldTable, 2, "invite", "");
        AddFieldRow(homeFieldTable, 3, "coordinationServer", DefaultCoordinationServer());
        AddFieldRow(homeFieldTable, 4, "relayServer", DefaultRelayServer());
        AddFieldRow(homeFieldTable, 5, "remotePeer", "");
        AddFieldRow(homeFieldTable, 6, "stunServer", DefaultStunServer());
        AddFieldRow(homeFieldTable, 7, "upnpPortMap", "true");
        homeFieldTable.RowCount = 8;
        homeFieldTable.RowStyles.Clear();
        for (int i = 0; i < 8; i++)
        {
            homeFieldTable.RowStyles.Add(new RowStyle(SizeType.Absolute, 34));
        }
        homeFieldTable.GrowStyle = TableLayoutPanelGrowStyle.AddRows;
        homeFieldTable.AutoSize = false;
        homeFieldTable.Dock = DockStyle.None;
        homeFieldTable.MouseWheel += ScrollHomeFieldsWheel;

        homeFieldViewport = new Panel();
        homeFieldViewport.Dock = DockStyle.Fill;
        homeFieldViewport.BackColor = CardDark;
        homeFieldViewport.Padding = new Padding(0);
        homeFieldViewport.MouseWheel += ScrollHomeFieldsWheel;
        homeFieldViewport.Resize += delegate { AdjustHomeFieldLayout(); };
        homeFieldViewport.Controls.Add(homeFieldTable);

        homeFieldScrollBar = new Panel();
        homeFieldScrollBar.Dock = DockStyle.Right;
        homeFieldScrollBar.Width = 10;
        homeFieldScrollBar.BackColor = CardDark;
        homeFieldScrollBar.MouseWheel += ScrollHomeFieldsWheel;
        homeFieldScrollBar.Paint += PaintHomeFieldScrollBar;

        homeFieldScrollThumb = new Panel();
        homeFieldScrollThumb.BackColor = FieldBorder;
        homeFieldScrollThumb.Cursor = Cursors.Hand;
        homeFieldScrollThumb.MouseDown += BeginHomeFieldScrollThumbDrag;
        homeFieldScrollThumb.MouseMove += DragHomeFieldScrollThumb;
        homeFieldScrollThumb.MouseUp += EndHomeFieldScrollThumbDrag;
        homeFieldScrollThumb.Resize += delegate { ApplyRoundedRegion(homeFieldScrollThumb, 4); };
        homeFieldScrollBar.Controls.Add(homeFieldScrollThumb);

        leftInner.Controls.Add(homeFieldViewport);
        leftInner.Controls.Add(homeFieldScrollBar);
        AdjustHomeFieldLayout();

        // Quick actions live below the cards so they cannot be clipped by the fields card.
        FlowLayoutPanel quickActions = new FlowLayoutPanel();
        quickActions.BackColor = Color.Transparent;
        quickActions.WrapContents = true;
        quickActions.Dock = DockStyle.Fill;
        quickActions.Padding = new Padding(4, 8, 4, 8);
        homeButtonControls["quickHostRoom"] = AddButton(quickActions, "quickHostRoom", delegate { QuickHostRoom(); });
        homeButtonControls["quickJoinRoom"] = AddButton(quickActions, "quickJoinRoom", delegate { QuickJoinRoom(); });
        homeButtonControls["copyInvite"] = AddButton(quickActions, "copyInvite", delegate { CopyInvite(); });
        homeButtonControls["startLanSession"] = AddButton(quickActions, "startLanSession", delegate { StartLanSession(); });
        homeButtonControls["checkConnection"] = AddButton(quickActions, "checkConnection", delegate { RunNetworkDiagnose(); });
        moreToolsButton = AddButton(quickActions, "moreTools", delegate { SelectPage("tools"); });
        homeButtonControls["moreTools"] = moreToolsButton;
        Panel quickActionHost = new Panel();
        quickActionHost.BackColor = Color.Transparent;
        quickActionHost.Controls.Add(quickActions);

        body.Controls.Add(leftCard);

        // Right: room details card
        Panel details = RoomDetailsPanel();
        details.Dock = DockStyle.None;
        details.Margin = new Padding(8, 12, 16, 8);
        body.Controls.Add(details);
        body.Controls.Add(quickActionHost);
        Action layoutHome = delegate
        {
            int pad = 16;
            int gap = 24;
            int availableWidth = Math.Max(320, body.ClientSize.Width - pad * 2);
            int cardHeight = Math.Min(282, Math.Max(230, body.ClientSize.Height - 124));
            int actionTop = cardHeight + 12;
            int actionHeight = Math.Min(100, Math.Max(82, body.ClientSize.Height - actionTop - 12));
            int leftWidth = Math.Max(360, (int)((availableWidth - gap) * 0.58));
            int rightWidth = Math.Max(260, availableWidth - gap - leftWidth);
            if (leftWidth + gap + rightWidth > availableWidth)
            {
                leftWidth = Math.Max(320, availableWidth - gap - rightWidth);
            }
            leftCard.SetBounds(pad, 0, leftWidth, cardHeight);
            details.SetBounds(pad + leftWidth + gap, 0, rightWidth, cardHeight);
            quickActionHost.SetBounds(pad, actionTop, availableWidth, actionHeight);
        };
        body.Resize += delegate { layoutHome(); };
        layoutHome();
        UpdateHomeActionButtons();
    }

    void BuildDiagnosePage()
    {
        pageDiagnose = new Panel();
        pageDiagnose.Dock = DockStyle.Fill;
        pageDiagnose.BackColor = Color.Transparent;
        pageDiagnose.Padding = new Padding(0);
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
        pageGames.Padding = new Padding(0);
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
        pageTools.Padding = new Padding(0);
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
        AddToolButton("prepareLanEnvironment", delegate { RunPrepareLanEnvironment(); });
        AddToolButton("adapterPlan", delegate { RunNativeCli("adapter-plan --adapter-name LocalAreaInterconnection --subnet " + subnet.Text + " --ip " + ip.Text); });
        AddToolButton("adapterScan", delegate { RunNativeAdapterEnsure(); });
        AddToolButton("nativeAdapterEnsure", delegate { RunNativeAdapterEnsure(); });
        AddToolButton("nativeAdapterApply", delegate { RunNativeAdapterApply(); });
        AddToolButton("firewallApply", delegate { RunFirewallApply(); });
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
        pageAbout.Padding = new Padding(0);
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

