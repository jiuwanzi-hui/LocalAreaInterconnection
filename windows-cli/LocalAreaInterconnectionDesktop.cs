using System;
using System.Collections.Generic;
using System.Globalization;
using System.Diagnostics;
using System.Drawing;
using System.Drawing.Drawing2D;
using System.IO;
using System.Runtime.InteropServices;
using System.Text;
using System.Windows.Forms;

public class LocalAreaInterconnectionDesktop : Form
{
    TextBox roomName;
    TextBox hostName;
    TextBox subnet;
    TextBox ip;
    TextBox gameName;
    TextBox gameCatalog;
    TextBox ports;
    TextBox observed;
    TextBox netshOutput;
    TextBox pingTarget;
    TextBox packetObservations;
    TextBox coordinationServer;
    TextBox stunServer;
    TextBox remotePeer;
    TextBox invite;
    TextBox output;
    Timer animation;
    Timer runtimeStatusTimer;
    Random random = new Random();
    Particle[] particles;
    string language;
    Label titleLabel;
    Label roomSummary;
    Label connectionSummary;
    Label broadcastSummary;
    Label memberSummary;
    Label nextActionSummary;
    ComboBox languageSelect;
    ToolTip chromeTips;
    TableLayoutPanel rootLayout;
    FlowLayoutPanel actionsPanel;
    Dictionary<string, Label> labelControls = new Dictionary<string, Label>();
    Dictionary<string, Button> buttonControls = new Dictionary<string, Button>();
    Process runtimeProcess;
    Process coordinationProcess;
    StringBuilder runtimeOutput = new StringBuilder();
    StringBuilder coordinationOutput = new StringBuilder();
    string runtimeStopFile = "";
    string latestRuntimeSnapshot = "";
    string latestRuntimeObservationFile = "";
    string latestNativeOfferFile = "";
    string coordinationStoreFile = "";
    string lastRuntimeSnapshotText = "";
    int lastRuntimeLogLength = 0;
    DateTime lastCoordinationRefreshUtc = DateTime.MinValue;
    const int ActionRow = 15;
    const int OutputRow = 16;

    protected override CreateParams CreateParams
    {
        get
        {
            CreateParams cp = base.CreateParams;
            cp.ExStyle |= 0x02000000;
            return cp;
        }
    }

    protected override void WndProc(ref Message m)
    {
        const int wmNcHitTest = 0x84;
        const int htClient = 1;
        const int htLeft = 10;
        const int htRight = 11;
        const int htTop = 12;
        const int htTopLeft = 13;
        const int htTopRight = 14;
        const int htBottom = 15;
        const int htBottomLeft = 16;
        const int htBottomRight = 17;

        base.WndProc(ref m);
        if (m.Msg != wmNcHitTest || (int)m.Result != htClient || WindowState == FormWindowState.Maximized)
        {
            return;
        }

        Point cursor = PointToClient(new Point(SignedLowWord(m.LParam), SignedHighWord(m.LParam)));
        int grip = 8;
        bool left = cursor.X <= grip;
        bool right = cursor.X >= ClientSize.Width - grip;
        bool top = cursor.Y <= grip;
        bool bottom = cursor.Y >= ClientSize.Height - grip;

        if (left && top) m.Result = new IntPtr(htTopLeft);
        else if (right && top) m.Result = new IntPtr(htTopRight);
        else if (left && bottom) m.Result = new IntPtr(htBottomLeft);
        else if (right && bottom) m.Result = new IntPtr(htBottomRight);
        else if (left) m.Result = new IntPtr(htLeft);
        else if (right) m.Result = new IntPtr(htRight);
        else if (top) m.Result = new IntPtr(htTop);
        else if (bottom) m.Result = new IntPtr(htBottom);
    }

    static int SignedLowWord(IntPtr value)
    {
        return (short)((long)value & 0xFFFF);
    }

    static int SignedHighWord(IntPtr value)
    {
        return (short)(((long)value >> 16) & 0xFFFF);
    }

    public static void Main()
    {
        Application.EnableVisualStyles();
        Application.Run(new LocalAreaInterconnectionDesktop());
    }

    public LocalAreaInterconnectionDesktop()
    {
        Text = "LocalAreaInterconnection";
        Width = 980;
        Height = 680;
        StartPosition = FormStartPosition.CenterScreen;
        FormBorderStyle = FormBorderStyle.None;
        MinimumSize = new Size(900, 620);
        DoubleBuffered = true;
        SetStyle(ControlStyles.AllPaintingInWmPaint | ControlStyles.UserPaint | ControlStyles.OptimizedDoubleBuffer, true);
        BackColor = Color.FromArgb(7, 22, 39);
        Font = new Font("Segoe UI", 9);
        Icon = Icon.ExtractAssociatedIcon(Application.ExecutablePath);
        language = LoadLanguage();
        chromeTips = new ToolTip();
        chromeTips.BackColor = Color.FromArgb(14, 38, 58);
        chromeTips.ForeColor = Color.FromArgb(232, 249, 255);

        particles = new Particle[42];
        for (int i = 0; i < particles.Length; i++)
        {
            particles[i] = NewParticle();
        }

        TableLayoutPanel shell = new TableLayoutPanel();
        shell.Dock = DockStyle.Fill;
        shell.BackColor = Color.Transparent;
        shell.ColumnCount = 1;
        shell.RowCount = 2;
        shell.RowStyles.Add(new RowStyle(SizeType.Absolute, 38));
        shell.RowStyles.Add(new RowStyle(SizeType.Percent, 100));
        Controls.Add(shell);

        shell.Controls.Add(TitleBar(), 0, 0);

        rootLayout = new TableLayoutPanel();
        rootLayout.Dock = DockStyle.Fill;
        rootLayout.BackColor = Color.Transparent;
        rootLayout.ColumnCount = 3;
        rootLayout.RowCount = 17;
        rootLayout.Padding = new Padding(12);
        rootLayout.ColumnStyles.Add(new ColumnStyle(SizeType.Absolute, 180));
        rootLayout.ColumnStyles.Add(new ColumnStyle(SizeType.Percent, 54));
        rootLayout.ColumnStyles.Add(new ColumnStyle(SizeType.Percent, 46));
        for (int i = 0; i < 15; i++)
        {
            rootLayout.RowStyles.Add(new RowStyle(SizeType.Absolute, 36));
        }
        rootLayout.RowStyles.Add(new RowStyle(SizeType.Absolute, 128));
        rootLayout.RowStyles.Add(new RowStyle(SizeType.Percent, 100));
        shell.Controls.Add(rootLayout, 0, 1);

        roomName = AddField(rootLayout, 0, "roomName", "Friday LAN");
        hostName = AddField(rootLayout, 1, "host", "Alice");
        subnet = AddField(rootLayout, 2, "virtualSubnet", "10.77.12.0/24");
        ip = AddField(rootLayout, 3, "myVirtualIp", "10.77.12.2");
        gameName = AddField(rootLayout, 4, "gameName", "Example Game");
        gameCatalog = AddField(rootLayout, 5, "gameCatalog", "");
        ports = AddField(rootLayout, 6, "gamePorts", "27015");
        observed = AddField(rootLayout, 7, "observedRules", "udp:27015");
        netshOutput = AddField(rootLayout, 8, "netshOutputFile", "");
        pingTarget = AddField(rootLayout, 9, "pingTarget", "127.0.0.1");
        packetObservations = AddField(rootLayout, 10, "packetObservations", "");
        coordinationServer = AddField(rootLayout, 11, "coordinationServer", "");
        stunServer = AddField(rootLayout, 12, "stunServer", "");
        remotePeer = AddField(rootLayout, 13, "remotePeer", "");
        invite = AddField(rootLayout, 14, "invite", "");

        actionsPanel = new FlowLayoutPanel();
        actionsPanel.Dock = DockStyle.Fill;
        actionsPanel.BackColor = Color.Transparent;
        actionsPanel.Padding = new Padding(0, 4, 0, 0);
        actionsPanel.Margin = new Padding(0);
        actionsPanel.WrapContents = true;
        actionsPanel.AutoScroll = false;
        rootLayout.Controls.Add(new Label(), 0, ActionRow);
        rootLayout.Controls.Add(actionsPanel, 1, ActionRow);

        AddButton(actionsPanel, "createRoom", delegate { CreateRoom(); });
        AddButton(actionsPanel, "copyInvite", delegate { CopyInvite(); });
        AddButton(actionsPanel, "copyIp", delegate { CopyVirtualIp(); });
        AddButton(actionsPanel, "decodeInvite", delegate { DecodeInvite(); });
        AddButton(actionsPanel, "joinRoom", delegate { JoinRoom(); });
        AddButton(actionsPanel, "adapterPlan", delegate { RunCli("adapter-plan --subnet " + subnet.Text + " --ip " + ip.Text); });
        AddButton(actionsPanel, "adapterScan", delegate { RunCli("adapter-scan --adapter-name LocalAreaInterconnection --subnet " + subnet.Text + " --ip " + ip.Text); });
        AddButton(actionsPanel, "nativeAdapterEnsure", delegate { RunNativeAdapterEnsure(); });
        AddButton(actionsPanel, "gamePlan", delegate { RunCli("game-plan --game-name " + Quote(gameName.Text) + " --subnet " + subnet.Text + " --ports " + ports.Text); });
        AddButton(actionsPanel, "gameProfilePlan", delegate { RunGameProfilePlan(); });
        AddButton(actionsPanel, "firewallPlan", delegate { RunCli("firewall-plan --game-name " + Quote(gameName.Text) + " --subnet " + subnet.Text + " --ports " + ports.Text); });
        AddButton(actionsPanel, "firewallDiagnose", delegate { RunCli(FirewallDiagnoseArgs()); });
        AddButton(actionsPanel, "firewallScan", delegate { RunCli("firewall-scan --game-name " + Quote(gameName.Text) + " --subnet " + subnet.Text + " --ports " + ports.Text); });
        AddButton(actionsPanel, "generalDiagnose", delegate { RunCli("diagnose --virtual-adapter ok --firewall allowed --p2p ok --broadcast missing --game-traffic missing"); });
        AddButton(actionsPanel, "networkDiagnose", delegate { RunNetworkDiagnose(); });
        AddButton(actionsPanel, "exportDiagnostics", delegate { ExportDiagnostics(); });
        AddButton(actionsPanel, "udpTest", delegate { RunUdpTest(); });
        AddButton(actionsPanel, "broadcastTest", delegate { RunBroadcastTest(); });
        AddButton(actionsPanel, "nativeRuntimeSelfTest", delegate { RunNativeRuntimeSelfTest(); });
        AddButton(actionsPanel, "nativeOffer", delegate { RunNativeOffer(); });
        AddButton(actionsPanel, "startCoordination", delegate { StartLocalCoordinationServer(); });
        AddButton(actionsPanel, "stopCoordination", delegate { StopLocalCoordinationServer(); });
        AddButton(actionsPanel, "startRuntime", delegate { StartNativeRuntime(); });
        AddButton(actionsPanel, "stopRuntime", delegate { StopNativeRuntime(); });
        AddButton(actionsPanel, "closeRoom", delegate { CloseCoordinationRoom(); });
        AddButton(actionsPanel, "nativeNatSelfTest", delegate { RunNativeNatSelfTest(); });
        AddButton(actionsPanel, "tcpTest", delegate { RunTcpTest(); });
        AddButton(actionsPanel, "browseGameCatalog", delegate { BrowseGameCatalog(); });
        AddButton(actionsPanel, "browseNetsh", delegate { BrowseNetshOutput(); });
        AddButton(actionsPanel, "browsePackets", delegate { BrowsePacketObservations(); });
        AddButton(actionsPanel, "copyOutput", delegate { if (output.Text.Length > 0) Clipboard.SetText(output.Text); });

        Panel detailsPanel = RoomDetailsPanel();
        rootLayout.Controls.Add(detailsPanel, 2, 0);
        rootLayout.SetRowSpan(detailsPanel, 12);

        output = new TextBox();
        output.Multiline = true;
        output.ScrollBars = ScrollBars.None;
        output.WordWrap = true;
        output.Dock = DockStyle.Fill;
        output.Font = new System.Drawing.Font("Consolas", 10);
        StyleTextBox(output);
        Label outputLabel = Label("output");
        rootLayout.Controls.Add(outputLabel, 0, OutputRow);
        Panel outputFrame = Framed(output);
        rootLayout.Controls.Add(outputFrame, 1, OutputRow);
        rootLayout.SetColumnSpan(outputFrame, 2);

        ApplyLanguage();
        UpdateRoomDetails("idle");
        Resize += delegate { AdjustActionLayout(); };
        AdjustActionLayout();

        animation = new Timer();
        animation.Interval = 80;
        animation.Tick += delegate
        {
            MoveParticles();
            Invalidate();
        };
        animation.Start();
        runtimeStatusTimer = new Timer();
        runtimeStatusTimer.Interval = 1500;
        runtimeStatusTimer.Tick += delegate { RefreshRuntimeStatus(); };
        runtimeStatusTimer.Start();
        FormClosing += delegate
        {
            StopRuntimeProcess(1500);
            StopCoordinationProcess(1500);
        };
    }

    Control TitleBar()
    {
        Panel bar = new Panel();
        bar.Dock = DockStyle.Fill;
        bar.BackColor = Color.FromArgb(5, 18, 32);
        bar.MouseDown += BeginDrag;

        PictureBox icon = new PictureBox();
        icon.Image = Icon.ToBitmap();
        icon.SizeMode = PictureBoxSizeMode.StretchImage;
        icon.Left = 12;
        icon.Top = 9;
        icon.Width = 20;
        icon.Height = 20;
        icon.MouseDown += BeginDrag;
        bar.Controls.Add(icon);

        titleLabel = new Label();
        titleLabel.Text = "LocalAreaInterconnection";
        titleLabel.ForeColor = Color.FromArgb(226, 248, 255);
        titleLabel.BackColor = Color.Transparent;
        titleLabel.AutoSize = true;
        titleLabel.Left = 40;
        titleLabel.Top = 10;
        titleLabel.MouseDown += BeginDrag;
        bar.Controls.Add(titleLabel);

        languageSelect = new ComboBox();
        languageSelect.DropDownStyle = ComboBoxStyle.DropDownList;
        languageSelect.FlatStyle = FlatStyle.Flat;
        languageSelect.DrawMode = DrawMode.OwnerDrawFixed;
        languageSelect.Items.Add("English");
        languageSelect.Items.Add("中文");
        languageSelect.Width = 92;
        languageSelect.Height = 24;
        languageSelect.Top = 7;
        languageSelect.BackColor = Color.FromArgb(14, 38, 58);
        languageSelect.ForeColor = Color.FromArgb(232, 249, 255);
        languageSelect.DrawItem += DrawLanguageItem;
        languageSelect.SelectedIndex = language == "zh" ? 1 : 0;
        languageSelect.SelectedIndexChanged += delegate
        {
            language = languageSelect.SelectedIndex == 1 ? "zh" : "en";
            SaveLanguage();
            ApplyLanguage();
            UpdateRoomDetails("idle");
        };
        bar.Controls.Add(languageSelect);

        AddChromeButton(bar, "X", "closeTip", Width - 44, delegate { Close(); });
        AddChromeButton(bar, "[]", "maximizeTip", Width - 88, delegate { WindowState = WindowState == FormWindowState.Maximized ? FormWindowState.Normal : FormWindowState.Maximized; });
        AddChromeButton(bar, "-", "minimizeTip", Width - 132, delegate { WindowState = FormWindowState.Minimized; });
        bar.Resize += delegate
        {
            languageSelect.Left = bar.Width - 236;
            bar.Controls[3].Left = bar.Width - 44;
            bar.Controls[4].Left = bar.Width - 88;
            bar.Controls[5].Left = bar.Width - 132;
        };
        languageSelect.Left = bar.Width - 236;
        return bar;
    }

    void AddChromeButton(Panel bar, string text, string tipKey, int left, EventHandler handler)
    {
        Button button = new Button();
        button.Text = text;
        button.Left = left;
        button.Top = 0;
        button.Width = 44;
        button.Height = 38;
        button.FlatStyle = FlatStyle.Flat;
        button.FlatAppearance.BorderSize = 0;
        button.BackColor = Color.FromArgb(5, 18, 32);
        button.ForeColor = Color.FromArgb(220, 244, 255);
        button.Click += handler;
        chromeTips.SetToolTip(button, T(tipKey));
        button.Tag = tipKey;
        bar.Controls.Add(button);
    }

    void BeginDrag(object sender, MouseEventArgs e)
    {
        if (e.Button != MouseButtons.Left) return;
        Native.ReleaseCapture();
        Native.SendMessage(Handle, 0xA1, new IntPtr(0x2), IntPtr.Zero);
    }

    void DrawLanguageItem(object sender, DrawItemEventArgs e)
    {
        e.DrawBackground();
        bool selected = (e.State & DrawItemState.Selected) == DrawItemState.Selected;
        using (SolidBrush background = new SolidBrush(selected ? Color.FromArgb(42, 112, 150) : Color.FromArgb(14, 38, 58)))
        {
            e.Graphics.FillRectangle(background, e.Bounds);
        }
        if (e.Index >= 0)
        {
            using (SolidBrush text = new SolidBrush(Color.FromArgb(232, 249, 255)))
            {
                e.Graphics.DrawString(languageSelect.Items[e.Index].ToString(), Font, text, e.Bounds.Left + 6, e.Bounds.Top + 2);
            }
        }
    }

    TextBox AddField(TableLayoutPanel root, int row, string key, string value)
    {
        TextBox box = new TextBox();
        box.Dock = DockStyle.Fill;
        box.Text = value;
        StyleTextBox(box);
        root.Controls.Add(Label(key), 0, row);
        root.Controls.Add(Framed(box), 1, row);
        return box;
    }

    Label Label(string key)
    {
        Label label = new Label();
        label.Text = T(key);
        label.AutoSize = true;
        label.Dock = DockStyle.Fill;
        label.TextAlign = System.Drawing.ContentAlignment.MiddleLeft;
        label.ForeColor = Color.FromArgb(210, 238, 255);
        label.BackColor = Color.Transparent;
        labelControls[key] = label;
        return label;
    }

    void AddButton(FlowLayoutPanel panel, string key, EventHandler handler)
    {
        Button button = new Button();
        button.Text = T(key);
        button.Width = Math.Min(184, Math.Max(116, TextRenderer.MeasureText(button.Text, Font).Width + 24));
        button.Height = 28;
        button.Margin = new Padding(0, 0, 8, 8);
        button.FlatStyle = FlatStyle.Flat;
        button.BackColor = Color.FromArgb(34, 95, 132);
        button.ForeColor = Color.FromArgb(236, 250, 255);
        button.FlatAppearance.BorderColor = Color.FromArgb(120, 203, 255);
        button.FlatAppearance.MouseOverBackColor = Color.FromArgb(54, 132, 175);
        button.FlatAppearance.MouseDownBackColor = Color.FromArgb(21, 72, 110);
        button.Click += handler;
        buttonControls[key] = button;
        panel.Controls.Add(button);
    }

    void AdjustActionLayout()
    {
        if (actionsPanel == null || rootLayout == null) return;
        int available = Math.Max(300, actionsPanel.ClientSize.Width - 8);
        int columns = Math.Max(3, Math.Min(6, available / 138));
        int width = Math.Max(112, (available / columns) - 8);
        foreach (Control control in actionsPanel.Controls)
        {
            control.Width = width;
            control.Height = 30;
        }
        int rows = (int)Math.Ceiling(actionsPanel.Controls.Count / (double)columns);
        rootLayout.RowStyles[ActionRow].Height = Math.Max(78, rows * 38 + 12);
    }

    void StyleTextBox(TextBox box)
    {
        box.BorderStyle = BorderStyle.FixedSingle;
        box.BackColor = Color.FromArgb(14, 38, 58);
        box.ForeColor = Color.FromArgb(232, 249, 255);
        box.BorderStyle = BorderStyle.None;
        box.Margin = new Padding(0);
    }

    Panel Framed(Control control)
    {
        Panel panel = new Panel();
        panel.Dock = DockStyle.Fill;
        panel.BackColor = Color.FromArgb(74, 130, 161);
        panel.Padding = new Padding(1);
        control.Dock = DockStyle.Fill;
        panel.Controls.Add(control);
        return panel;
    }

    Panel RoomDetailsPanel()
    {
        Panel outer = new Panel();
        outer.Dock = DockStyle.Fill;
        outer.BackColor = Color.FromArgb(12, 34, 53);
        outer.Padding = new Padding(1);
        outer.Margin = new Padding(12, 0, 0, 8);

        TableLayoutPanel details = new TableLayoutPanel();
        details.Dock = DockStyle.Fill;
        details.BackColor = Color.FromArgb(9, 27, 43);
        details.ColumnCount = 1;
        details.RowCount = 6;
        details.Padding = new Padding(12);
        details.RowStyles.Add(new RowStyle(SizeType.Absolute, 32));
        for (int i = 1; i < 6; i++)
        {
            details.RowStyles.Add(new RowStyle(SizeType.Percent, 20));
        }

        Label header = new Label();
        header.Name = "roomDetailsHeader";
        header.Text = T("roomDetails");
        header.Dock = DockStyle.Fill;
        header.TextAlign = ContentAlignment.MiddleLeft;
        header.Font = new Font(Font.FontFamily, 10, FontStyle.Bold);
        header.ForeColor = Color.FromArgb(232, 249, 255);
        header.BackColor = Color.Transparent;
        labelControls["roomDetails"] = header;
        details.Controls.Add(header, 0, 0);

        roomSummary = DetailLabel();
        connectionSummary = DetailLabel();
        broadcastSummary = DetailLabel();
        memberSummary = DetailLabel();
        nextActionSummary = DetailLabel();
        details.Controls.Add(roomSummary, 0, 1);
        details.Controls.Add(connectionSummary, 0, 2);
        details.Controls.Add(broadcastSummary, 0, 3);
        details.Controls.Add(memberSummary, 0, 4);
        details.Controls.Add(nextActionSummary, 0, 5);

        outer.Controls.Add(details);
        return outer;
    }

    Label DetailLabel()
    {
        Label label = new Label();
        label.Dock = DockStyle.Fill;
        label.AutoEllipsis = true;
        label.TextAlign = ContentAlignment.MiddleLeft;
        label.ForeColor = Color.FromArgb(210, 238, 255);
        label.BackColor = Color.Transparent;
        label.Padding = new Padding(0, 2, 0, 2);
        return label;
    }

    protected override void OnPaintBackground(PaintEventArgs e)
    {
        using (LinearGradientBrush background = new LinearGradientBrush(
            ClientRectangle,
            Color.FromArgb(6, 19, 35),
            Color.FromArgb(21, 72, 103),
            LinearGradientMode.ForwardDiagonal))
        {
            e.Graphics.FillRectangle(background, ClientRectangle);
        }

        using (GraphicsPath mistPath = new GraphicsPath())
        {
            mistPath.AddEllipse(-140, -80, Width + 260, Height / 2 + 120);
            using (PathGradientBrush mist = new PathGradientBrush(mistPath))
            {
                mist.CenterColor = Color.FromArgb(65, 126, 218, 255);
                mist.SurroundColors = new Color[] { Color.FromArgb(0, 126, 218, 255) };
                e.Graphics.FillPath(mist, mistPath);
            }
        }
    }

    protected override void OnPaint(PaintEventArgs e)
    {
        base.OnPaint(e);
        e.Graphics.SmoothingMode = SmoothingMode.AntiAlias;
        DrawParticles(e.Graphics);
    }

    void DrawParticles(Graphics graphics)
    {
        for (int i = 0; i < particles.Length; i++)
        {
            Particle p = particles[i];
            using (SolidBrush brush = new SolidBrush(Color.FromArgb(p.Alpha, 204, 242, 255)))
            {
                graphics.FillEllipse(brush, p.X, p.Y, p.Size, p.Size);
            }
        }
    }

    void MoveParticles()
    {
        for (int i = 0; i < particles.Length; i++)
        {
            particles[i].X += particles[i].Vx;
            particles[i].Y += particles[i].Vy;
            if (particles[i].X > Width + 20 || particles[i].Y < -20)
            {
                particles[i] = NewParticle();
                particles[i].X = -20;
                particles[i].Y = random.Next(Height + 1);
            }
        }
    }

    Particle NewParticle()
    {
        Particle particle = new Particle();
        particle.X = random.Next(1000);
        particle.Y = random.Next(700);
        particle.Vx = 0.18f + (float)random.NextDouble() * 0.55f;
        particle.Vy = -0.16f - (float)random.NextDouble() * 0.26f;
        particle.Size = 1.8f + (float)random.NextDouble() * 3.2f;
        particle.Alpha = 55 + random.Next(115);
        return particle;
    }

    class Particle
    {
        public float X;
        public float Y;
        public float Vx;
        public float Vy;
        public float Size;
        public int Alpha;
    }

    void CreateRoom()
    {
        string text = RunCli("init --room-name " + Quote(roomName.Text) + " --host " + Quote(hostName.Text));
        string generatedInvite = JsonStringValue(text, "invite");
        string generatedSubnet = JsonStringValue(text, "virtualSubnet");
        string generatedHostIp = JsonStringValue(text, "hostIp");
        if (generatedInvite.Length > 0) invite.Text = generatedInvite;
        if (generatedSubnet.Length > 0) subnet.Text = generatedSubnet;
        if (generatedHostIp.Length > 0)
        {
            ip.Text = generatedHostIp;
            pingTarget.Text = generatedHostIp;
        }
        UpdateRoomDetails("created");
    }

    void DecodeInvite()
    {
        string text = RunCli("decode --invite " + Quote(invite.Text));
        string decodedSubnet = JsonStringValue(text, "virtual_subnet");
        if (decodedSubnet.Length > 0) subnet.Text = decodedSubnet;
        string hostPeer = JsonStringValue(text, "host_peer_id");
        if (hostPeer.Length > 0 && hostName.Text.Trim().Length == 0)
        {
            hostName.Text = hostPeer;
        }
        UpdateRoomDetails("decoded");
    }

    void JoinRoom()
    {
        string text = RunCli("join --invite " + Quote(invite.Text) + " --peer " + Quote(hostName.Text));
        string joinedSubnet = JsonStringValue(text, "virtualSubnet");
        string suggestedIp = JsonStringValue(text, "suggestedLocalIp");
        string hostIp = JsonStringValue(text, "hostIp");
        if (joinedSubnet.Length > 0) subnet.Text = joinedSubnet;
        if (suggestedIp.Length > 0) ip.Text = suggestedIp;
        if (hostIp.Length > 0) pingTarget.Text = hostIp;
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
            output.Text = T("nothingToCopy");
            return;
        }
        Clipboard.SetText(value.Trim());
        output.Text = T(messageKey) + Environment.NewLine + value.Trim();
    }

    void RunNetworkDiagnose()
    {
        RunNetworkDiagnoseAndReturn();
    }

    string RunNetworkDiagnoseAndReturn()
    {
        string text = RunCli("network-observe --adapter-name LocalAreaInterconnection --expected-ip " + ip.Text
            + " --subnet " + subnet.Text
            + " --adapter-scan true"
            + PingArgs()
            + PacketObservationArgs()
            + " --broadcast-ports " + ports.Text
            + " --game-ports " + ports.Text);
        UpdateRoomDetailsFromNetworkReport(text);
        return text;
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
                + " --ports " + ports.Text
                + " --packet-io-backend wintun"
                + NetshExportArgs());
            UpdateRoomDetails("exported");
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
        RunNativeCli(args);
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
            observePath = Path.Combine(AppDataDirectory(), "runtime-packets.txt");
            packetObservations.Text = observePath;
        }
        string snapshotPath = Path.Combine(AppDataDirectory(), "runtime-snapshot.json");
        latestRuntimeSnapshot = snapshotPath;
        latestRuntimeObservationFile = observePath;
        string peer = SafePeerId(hostName.Text);
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

    void StartNativeRuntime()
    {
        if (runtimeProcess != null && !runtimeProcess.HasExited)
        {
            output.Text = T("runtimeAlreadyRunning") + Environment.NewLine + RuntimeStatusText();
            return;
        }

        string observePath = packetObservations.Text.Trim();
        if (observePath.Length == 0)
        {
            observePath = Path.Combine(AppDataDirectory(), "runtime-packets.txt");
            packetObservations.Text = observePath;
        }
        latestRuntimeObservationFile = observePath;
        latestRuntimeSnapshot = Path.Combine(AppDataDirectory(), "runtime-snapshot.json");
        runtimeStopFile = Path.Combine(AppDataDirectory(), "runtime.stop");
        if (File.Exists(runtimeStopFile)) File.Delete(runtimeStopFile);
        string peer = SafePeerId(hostName.Text);
        string publishOutput = PublishNativeOfferIfConfigured(peer, true);
        string args = "room-runtime-run"
            + " --room-id desktop_runtime"
            + " --peer-id " + Quote(peer)
            + " --virtual-ip " + ip.Text
            + " --bind " + NativeRuntimeBind()
            + " --key desktop-runtime-room-key"
            + " --game-ports " + FirstPortText("27015")
            + " --broadcast-ports " + FirstPortText("39078")
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
            + RuntimeCoordinationArgs();

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
        if (latestNativeOfferFile.Length > 0)
        {
            output.Text += Environment.NewLine + T("nativeOfferPath") + latestNativeOfferFile;
        }
        if (publishOutput.Length > 0)
        {
            output.Text += Environment.NewLine + Environment.NewLine + publishOutput;
        }
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

    void RunNativeNatSelfTest()
    {
        RunNativeCli("nat-hole-punch-loopback-test"
            + " --room-id desktop_self_test"
            + " --peer-a " + Quote(SafePeerId(hostName.Text) + "_a")
            + " --peer-b " + Quote(SafePeerId(hostName.Text) + "_b")
            + " --attempts 3"
            + " --interval-ms 0"
            + " --message desktop-nat");
    }

    void RunNativeOffer()
    {
        string peer = SafePeerId(hostName.Text);
        string result = CreateNativeOffer(peer, true);
        if (result.Length == 0) return;

        string publishOutput = PublishNativeOfferFileIfConfigured(true);
        if (publishOutput.Length > 0)
        {
            output.Text = result + Environment.NewLine + Environment.NewLine + publishOutput;
        }
        RefreshCoordinationRoomView(false);
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
            coordinationServer.Text = "http://" + bind;
        }
        coordinationStoreFile = Path.Combine(AppDataDirectory(), "coordination-store.json");
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
        if (coordinationStoreFile.Length == 0 || !File.Exists(coordinationStoreFile))
        {
            return;
        }
        string currentSubnet = subnet.Text.Trim();
        if (currentSubnet.Length == 0)
        {
            return;
        }
        string peer = SafePeerId(hostName.Text);
        string arguments = "coordination-room-view"
            + " --store " + Quote(coordinationStoreFile)
            + " --room-id desktop_runtime"
            + " --peer-id " + Quote(peer)
            + " --subnet " + Quote(currentSubnet);
        string text = showOutput ? RunNativeCli(arguments) : RunNativeCliCapture(arguments);
        UpdateRoomDetailsFromCoordinationView(text);
    }

    void RunTcpTest()
    {
        RunPacketTestAndRefresh("tcp-loopback-test --port " + FirstPortText("39079") + " --message ping");
    }

    void RunPacketTestAndRefresh(string command)
    {
        string testOutput = RunCli(command + ObserveFileArgs());
        string path = packetObservations.Text.Trim();
        if (path.Length == 0 || !File.Exists(path))
        {
            return;
        }

        string diagnosticOutput = RunNetworkDiagnoseAndReturn();
        output.Text = testOutput + Environment.NewLine + Environment.NewLine + T("autoNetworkDiagnose") + Environment.NewLine + diagnosticOutput;
    }

    void UpdateRoomDetails(string mode)
    {
        if (roomSummary == null) return;
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
        string broadcast = JsonStringValue(json, "broadcast");
        string gameTraffic = JsonStringValue(json, "game_traffic");
        if (adapter.Length == 0) adapter = T("stateUnknown");
        if (tunnel.Length == 0) tunnel = T("stateUnknown");
        if (p2p.Length == 0) p2p = T("stateUnknown");
        if (broadcast.Length == 0) broadcast = T("stateUnknown");
        if (gameTraffic.Length == 0) gameTraffic = T("stateUnknown");

        roomSummary.Text = T("detailRoom") + " " + SafeText(roomName.Text) + " | " + T("detailSubnet") + " " + SafeText(subnet.Text);
        connectionSummary.Text = T("detailConnection") + " " + T("detailAdapter") + "=" + adapter + ", " + T("detailTunnel") + "=" + tunnel + ", P2P=" + p2p;
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
        string broadcast = JsonStringValue(json, "broadcast");
        string gameTraffic = JsonStringValue(json, "game_traffic");
        string connectedPeers = JsonNumberValue(json, "connected_peer_count");
        string heartbeatPackets = JsonNumberValue(json, "heartbeatPacketsSent");
        string snapshotWrites = JsonNumberValue(json, "snapshotWriteCount");
        if (adapter.Length == 0) adapter = T("stateUnknown");
        if (tunnel.Length == 0) tunnel = T("stateUnknown");
        if (p2p.Length == 0) p2p = T("stateUnknown");
        if (broadcast.Length == 0) broadcast = T("stateUnknown");
        if (gameTraffic.Length == 0) gameTraffic = T("stateUnknown");
        if (connectedPeers.Length == 0) connectedPeers = "0";
        if (heartbeatPackets.Length == 0) heartbeatPackets = "0";
        if (snapshotWrites.Length == 0) snapshotWrites = "0";

        roomSummary.Text = T("detailRoom") + " " + SafeText(roomName.Text) + " | " + T("detailSubnet") + " " + SafeText(subnet.Text);
        connectionSummary.Text = T("detailConnection") + " " + RuntimeStatusText() + ", " + T("detailTunnel") + "=" + tunnel + ", P2P=" + p2p;
        broadcastSummary.Text = T("detailBroadcast") + " " + broadcast + " | " + T("detailGameTraffic") + " " + gameTraffic;
        memberSummary.Text = T("detailMembers") + " " + SafeText(hostName.Text) + " @ " + SafeText(ip.Text)
            + ", " + T("runtimePeers") + "=" + connectedPeers
            + ", " + T("runtimeHeartbeats") + "=" + heartbeatPackets
            + ", " + T("runtimeSnapshots") + "=" + snapshotWrites;
        nextActionSummary.Text = T("detailNext") + " " + DiagnosticNextAction(adapter, tunnel, p2p, broadcast, gameTraffic);
    }

    void UpdateRoomDetailsFromCoordinationView(string json)
    {
        if (roomSummary == null || json.Trim().Length == 0) return;
        string status = JsonStringValue(json, "status");
        string memberCount = JsonNumberValue(json, "member_count");
        string onlineCount = JsonNumberValue(json, "online_count");
        string expiredCount = JsonNumberValue(json, "expired_count");
        string nextAction = JsonStringValue(json, "next_action");
        string members = CoordinationMembersText(json);
        if (status.Length == 0) status = T("stateUnknown");
        if (memberCount.Length == 0) memberCount = "0";
        if (onlineCount.Length == 0) onlineCount = "0";
        if (expiredCount.Length == 0) expiredCount = "0";
        if (nextAction.Length == 0) nextAction = NextActionText("joined");
        if (members.Length == 0) members = MemberText("joined");

        roomSummary.Text = T("detailRoom") + " desktop_runtime | " + T("detailSubnet") + " " + SafeText(subnet.Text);
        connectionSummary.Text = T("detailConnection") + " " + T("coordinationRoomStatus") + "=" + status
            + ", " + T("coordinationOnline") + "=" + onlineCount + "/" + memberCount
            + ", " + T("coordinationExpired") + "=" + expiredCount;
        memberSummary.Text = T("detailMembers") + " " + members;
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
            path = Path.Combine(AppDataDirectory(), "runtime-snapshot.json");
        }
        if (!File.Exists(path)) return "";
        return " --runtime-snapshot " + Quote(path);
    }

    string RuntimeCoordinationArgs()
    {
        string args = "";
        string server = coordinationServer.Text.Trim();
        string peer = remotePeer.Text.Trim();
        if (server.Length > 0 && peer.Length > 0)
        {
            args += " --coordination-server " + Quote(server);
            args += " --coordination-peer " + Quote(peer);
        }
        return args;
    }

    void RefreshCoordinationPresence()
    {
        if (runtimeProcess == null || runtimeProcess.HasExited)
        {
            return;
        }
        if (coordinationServer.Text.Trim().Length == 0)
        {
            return;
        }
        if ((DateTime.UtcNow - lastCoordinationRefreshUtc).TotalSeconds < 15)
        {
            return;
        }
        lastCoordinationRefreshUtc = DateTime.UtcNow;
        string peer = SafePeerId(hostName.Text);
        if (latestNativeOfferFile.Length == 0 || !File.Exists(latestNativeOfferFile))
        {
            CreateNativeOffer(peer, false);
        }
        PublishNativeOfferFileIfConfigured(false);
        RefreshCoordinationRoomView(false);
    }

    string PublishNativeOfferIfConfigured(string peer, bool showOutput)
    {
        if (coordinationServer.Text.Trim().Length == 0) return "";
        if (CreateNativeOffer(peer, false).Length == 0) return output.Text;
        return PublishNativeOfferFileIfConfigured(showOutput);
    }

    string PublishNativeOfferFileIfConfigured(bool showOutput)
    {
        string server = coordinationServer.Text.Trim();
        if (server.Length == 0 || latestNativeOfferFile.Length == 0 || !File.Exists(latestNativeOfferFile))
        {
            return "";
        }
        string arguments = "coordination-http-offer-publish"
            + " --server " + Quote(server)
            + " --offer " + Quote(latestNativeOfferFile)
            + " --ttl-ms 30000";
        return showOutput ? RunNativeCli(arguments) : RunNativeCliCapture(arguments);
    }

    string LeaveCoordinationRoomIfConfigured()
    {
        string server = coordinationServer.Text.Trim();
        if (server.Length == 0)
        {
            return "";
        }
        string peer = SafePeerId(hostName.Text);
        string result = RunNativeCliCapture("coordination-http-leave"
            + " --server " + Quote(server)
            + " --room-id desktop_runtime"
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
            + " --room-id desktop_runtime");
        return T("coordinationRoomClosed") + Environment.NewLine + result;
    }

    string CreateNativeOffer(string peer, bool showOutput)
    {
        latestNativeOfferFile = Path.Combine(AppDataDirectory(), "native-offer-" + peer + ".json");
        string arguments = "nat-candidates"
            + " --room-id desktop_runtime"
            + " --peer-id " + Quote(peer)
            + " --bind " + NativeRuntimeBind()
            + StunArgs()
            + " --nonce " + Quote(peer + "-desktop-offer");
        string text = showOutput ? RunNativeCli(arguments) : RunNativeCliCapture(arguments);
        string offer = JsonObjectValue(text, "offer");
        if (offer.Length > 0)
        {
            File.WriteAllText(latestNativeOfferFile, offer + Environment.NewLine, Encoding.UTF8);
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
        string server = stunServer.Text.Trim();
        if (server.Length == 0) return "";
        return " --stun-server " + Quote(server) + " --stun-timeout-ms 1000";
    }

    string NativeRuntimeBind()
    {
        return "0.0.0.0:39090";
    }

    string CoordinationBind()
    {
        string value = coordinationServer.Text.Trim();
        if (value.Length == 0) return "127.0.0.1:39110";
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
        return value.Length == 0 ? "127.0.0.1:39110" : value;
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
        string exe = Path.Combine(AppDomain.CurrentDomain.BaseDirectory, "LocalAreaInterconnection.Native.Cli.exe");
        if (!File.Exists(exe))
        {
            return T("missingNativeCli") + exe;
        }
        return RunExecutableCapture(exe, arguments);
    }

    Process StartNativeRuntimeProcess(string arguments)
    {
        return StartNativeBackgroundProcess(arguments, runtimeOutput, T("runtimeExited"));
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
        process.BeginOutputReadLine();
        process.BeginErrorReadLine();
        return process;
    }

    void StopRuntimeProcess(int waitMs)
    {
        if (runtimeProcess == null) return;
        try
        {
            if (!runtimeProcess.HasExited)
            {
                if (runtimeStopFile.Length > 0 && !File.Exists(runtimeStopFile))
                {
                    File.WriteAllText(runtimeStopFile, "stop");
                }
                if (!runtimeProcess.WaitForExit(waitMs))
                {
                    runtimeProcess.Kill();
                    runtimeProcess.WaitForExit(2000);
                }
            }
            runtimeProcess.Dispose();
            runtimeProcess = null;
        }
        catch
        {
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
        try
        {
            if (!coordinationProcess.HasExited)
            {
                coordinationProcess.Kill();
                coordinationProcess.WaitForExit(waitMs);
            }
            coordinationProcess.Dispose();
            coordinationProcess = null;
        }
        catch
        {
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
        ProcessStartInfo start = new ProcessStartInfo();
        start.FileName = exe;
        start.Arguments = arguments;
        start.UseShellExecute = false;
        start.RedirectStandardOutput = true;
        start.RedirectStandardError = true;
        start.CreateNoWindow = true;

        using (Process process = Process.Start(start))
        {
            string stdout = process.StandardOutput.ReadToEnd();
            string stderr = process.StandardError.ReadToEnd();
            process.WaitForExit();
            string text = stdout;
            if (stderr.Length > 0)
            {
                text += Environment.NewLine + stderr;
            }
            return text;
        }
    }

    string JsonStringValue(string json, string key)
    {
        string marker = "\"" + key + "\":";
        int start = json.IndexOf(marker, StringComparison.Ordinal);
        if (start < 0) return "";
        start = json.IndexOf('"', start + marker.Length);
        if (start < 0) return "";
        int end = json.IndexOf('"', start + 1);
        if (end < 0) return "";
        return json.Substring(start + 1, end - start - 1).Replace("\\\"", "\"").Replace("\\\\", "\\");
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
            if (peer.Length > 0)
            {
                string text = peer;
                if (virtualIp.Length > 0) text += " @ " + virtualIp;
                if (status.Length > 0) text += " (" + status + ")";
                members.Add(text);
            }
            search = end + 1;
        }
        int memberCount;
        if (Int32.TryParse(JsonNumberValue(json, "member_count"), out memberCount) && memberCount > members.Count)
        {
            members.Add("+" + (memberCount - members.Count).ToString());
        }
        return String.Join(", ", members.ToArray());
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

    string FirewallDiagnoseArgs()
    {
        string args = "firewall-diagnose --game-name " + Quote(gameName.Text) + " --subnet " + subnet.Text + " --ports " + ports.Text;
        if (netshOutput.Text.Trim().Length > 0)
        {
            return args + " --netsh-output " + Quote(netshOutput.Text.Trim());
        }
        return args + " --observed " + observed.Text;
    }

    void BrowseNetshOutput()
    {
        using (OpenFileDialog dialog = new OpenFileDialog())
        {
            dialog.Title = T("selectNetshOutput");
            dialog.Filter = T("textFilesFilter");
            if (dialog.ShowDialog(this) == DialogResult.OK)
            {
                netshOutput.Text = dialog.FileName;
            }
        }
    }

    void BrowsePacketObservations()
    {
        using (SaveFileDialog dialog = new SaveFileDialog())
        {
            dialog.Title = T("selectPacketObservations");
            dialog.Filter = T("textFilesFilter");
            dialog.FileName = "packets.txt";
            if (dialog.ShowDialog(this) == DialogResult.OK)
            {
                packetObservations.Text = dialog.FileName;
            }
        }
    }

    void BrowseGameCatalog()
    {
        using (OpenFileDialog dialog = new OpenFileDialog())
        {
            dialog.Title = T("selectGameCatalog");
            dialog.Filter = T("jsonFilesFilter");
            if (dialog.ShowDialog(this) == DialogResult.OK)
            {
                gameCatalog.Text = dialog.FileName;
            }
        }
    }

    bool LooksLikeIpv4(string value)
    {
        string[] parts = value.Split('.');
        if (parts.Length != 4) return false;
        for (int i = 0; i < parts.Length; i++)
        {
            int number;
            if (!Int32.TryParse(parts[i], out number)) return false;
            if (number < 0 || number > 255) return false;
        }
        return true;
    }

    void ApplyLanguage()
    {
        Text = T("appTitle");
        if (titleLabel != null) titleLabel.Text = T("appTitle");
        foreach (KeyValuePair<string, Label> item in labelControls)
        {
            item.Value.Text = T(item.Key);
        }
        foreach (KeyValuePair<string, Button> item in buttonControls)
        {
            item.Value.Text = T(item.Key);
        }
        UpdateChromeTooltips();
        if (output != null && output.Text.Length == 0)
        {
            output.Text = T("outputHelp");
        }
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

    string T(string key)
    {
        if (language == "zh")
        {
            if (key == "appTitle") return "LocalAreaInterconnection";
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
            if (key == "createRoom") return "创建房间";
            if (key == "copyInvite") return "复制邀请";
            if (key == "copyIp") return "复制我的 IP";
            if (key == "decodeInvite") return "解析邀请";
            if (key == "joinRoom") return "加入房间";
            if (key == "adapterPlan") return "网卡计划";
            if (key == "adapterScan") return "扫描网卡";
            if (key == "nativeAdapterEnsure") return "原生网卡检查";
            if (key == "gamePlan") return "游戏计划";
            if (key == "gameProfilePlan") return "模板游戏计划";
            if (key == "firewallPlan") return "防火墙计划";
            if (key == "firewallDiagnose") return "防火墙诊断";
            if (key == "firewallScan") return "扫描防火墙";
            if (key == "generalDiagnose") return "通用诊断";
            if (key == "networkDiagnose") return "网络诊断";
            if (key == "exportDiagnostics") return "导出诊断";
            if (key == "udpTest") return "UDP 测试";
            if (key == "broadcastTest") return "广播测试";
            if (key == "nativeRuntimeSelfTest") return "原生隧道自检";
            if (key == "startRuntime") return "启动 runtime";
            if (key == "stopRuntime") return "停止 runtime";
            if (key == "startCoordination") return "启动协调";
            if (key == "stopCoordination") return "停止协调";
            if (key == "closeRoom") return "关闭房间";
            if (key == "nativeNatSelfTest") return "NAT 自检";
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
            if (key == "detailHost") return "房主";
            if (key == "stateUnknown") return "未知";
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
            if (key == "stunServer") return "STUN 服务";
            if (key == "remotePeer") return "远端 Peer";
            if (key == "nativeOfferPath") return "Offer 文件: ";
            if (key == "runtimePeers") return "peer";
            if (key == "runtimeHeartbeats") return "心跳";
            if (key == "runtimeSnapshots") return "snapshot";
            if (key == "runtimeLogTail") return "runtime 最近日志:";
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
            if (key == "textFilesFilter") return "文本文件 (*.txt)|*.txt|所有文件 (*.*)|*.*";
            if (key == "jsonFilesFilter") return "JSON 文件 (*.json)|*.json|所有文件 (*.*)|*.*";
            if (key == "missingCli") return "缺少 CLI 程序: ";
            if (key == "missingNativeCli") return "缺少 Rust 原生 CLI 程序，请先重新生成 exe: ";
            if (key == "missingGameCatalog") return "请先选择游戏模板库 JSON 文件。";
        }
        else
        {
            if (key == "appTitle") return "LocalAreaInterconnection";
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
            if (key == "createRoom") return "Create room";
            if (key == "copyInvite") return "Copy invite";
            if (key == "copyIp") return "Copy my IP";
            if (key == "decodeInvite") return "Decode invite";
            if (key == "joinRoom") return "Join room";
            if (key == "adapterPlan") return "Adapter plan";
            if (key == "adapterScan") return "Adapter scan";
            if (key == "nativeAdapterEnsure") return "Native adapter check";
            if (key == "gamePlan") return "Game plan";
            if (key == "gameProfilePlan") return "Profile game plan";
            if (key == "firewallPlan") return "Firewall plan";
            if (key == "firewallDiagnose") return "Firewall diagnose";
            if (key == "firewallScan") return "Firewall scan";
            if (key == "generalDiagnose") return "General diagnose";
            if (key == "networkDiagnose") return "Network diagnose";
            if (key == "exportDiagnostics") return "Export diagnostics";
            if (key == "udpTest") return "UDP test";
            if (key == "broadcastTest") return "Broadcast test";
            if (key == "nativeRuntimeSelfTest") return "Native tunnel self-test";
            if (key == "startRuntime") return "Start runtime";
            if (key == "stopRuntime") return "Stop runtime";
            if (key == "startCoordination") return "Start coordination";
            if (key == "stopCoordination") return "Stop coordination";
            if (key == "closeRoom") return "Close room";
            if (key == "nativeNatSelfTest") return "NAT self-test";
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
            if (key == "detailHost") return "Host";
            if (key == "stateUnknown") return "unknown";
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
            if (key == "stunServer") return "STUN server";
            if (key == "remotePeer") return "Remote peer";
            if (key == "nativeOfferPath") return "Offer file: ";
            if (key == "runtimePeers") return "peers";
            if (key == "runtimeHeartbeats") return "heartbeats";
            if (key == "runtimeSnapshots") return "snapshots";
            if (key == "runtimeLogTail") return "Recent runtime log:";
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
            if (key == "textFilesFilter") return "Text files (*.txt)|*.txt|All files (*.*)|*.*";
            if (key == "jsonFilesFilter") return "JSON files (*.json)|*.json|All files (*.*)|*.*";
            if (key == "missingCli") return "Missing CLI executable: ";
            if (key == "missingNativeCli") return "Missing Rust native CLI executable. Build the latest exe first: ";
            if (key == "missingGameCatalog") return "Select a game catalog JSON file first.";
        }
        return key;
    }

    string Quote(string value)
    {
        return "\"" + value.Replace("\"", "\\\"") + "\"";
    }

    static class Native
    {
        [DllImport("user32.dll")]
        public static extern bool ReleaseCapture();

        [DllImport("user32.dll")]
        public static extern IntPtr SendMessage(IntPtr hWnd, int msg, IntPtr wParam, IntPtr lParam);
    }
}
