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

public class LocalAreaInterconnectionDesktop : Form
{
    static readonly Color ShellDark = Color.FromArgb(20, 22, 26);
    static readonly Color TitleDark = Color.FromArgb(16, 18, 22);
    static readonly Color SidebarDark = Color.FromArgb(15, 69, 78);
    static readonly Color SidebarMid = Color.FromArgb(30, 76, 86);
    static readonly Color SidebarDeep = Color.FromArgb(40, 65, 86);
    static readonly Color CardDark = Color.FromArgb(30, 34, 40);
    static readonly Color CardBorder = Color.FromArgb(42, 48, 56);
    static readonly Color FieldDark = Color.FromArgb(26, 30, 36);
    static readonly Color TextBright = Color.FromArgb(240, 244, 246);
    static readonly Color TextMuted = Color.FromArgb(138, 148, 156);
    static readonly Color AccentCyan = Color.FromArgb(0, 212, 216);
    static readonly Color AccentCyanHover = Color.FromArgb(31, 222, 218);
    static readonly Color AccentCyanDown = Color.FromArgb(0, 176, 184);
    static readonly Color ParticleCyan = Color.FromArgb(0, 212, 216);
    static readonly Color ParticleCyan2 = Color.FromArgb(31, 222, 218);

    enum ChromeGlyph
    {
        Minimize,
        Maximize,
        Close
    }

    static LocalAreaInterconnectionDesktop activeWindow;

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
    Timer runtimeStatusTimer;
    Timer particleTimer;
    const int ParticleCount = 34;
    Random random = new Random();
    Particle[] particles;
    string language;
    Panel sidebarPanel;
    Panel contentArea;
    Panel pageHome;
    Panel pageDiagnose;
    Panel pageGames;
    Panel pageTools;
    Panel pageAbout;
    List<Panel> contentPages = new List<Panel>();
    List<Button> navButtons = new List<Button>();
    string activePage = "home";
    Label titleLabel;
    Label roomSummary;
    Label connectionSummary;
    Label broadcastSummary;
    Label memberSummary;
    Label nextActionSummary;
    Button languageButton;
    ToolTip chromeTips;
    Panel actionsHost;
    Panel actionsViewport;
    Panel actionScrollBar;
    Panel actionScrollThumb;
    FlowLayoutPanel actionsPanel;
    Button moreToolsButton;
    bool advancedActionsVisible = false;
    int actionScrollOffset = 0;
    bool draggingActionScrollThumb = false;
    int actionScrollDragStartY = 0;
    int actionScrollStartOffset = 0;
    bool userActionRunning = false;
    List<Button> advancedActionButtons = new List<Button>();
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
    const int ResizeGripSize = 12;

    protected override CreateParams CreateParams
    {
        get
        {
            CreateParams cp = base.CreateParams;
            cp.Style |= 0x00040000 | 0x00020000 | 0x00010000;
            return cp;
        }
    }

    protected override void WndProc(ref Message m)
    {
        const int wmNcCalcSize = 0x83;
        const int wmNcHitTest = 0x84;
        const int htLeft = 10;
        const int htRight = 11;
        const int htTop = 12;
        const int htTopLeft = 13;
        const int htTopRight = 14;
        const int htBottom = 15;
        const int htBottomLeft = 16;
        const int htBottomRight = 17;

        if (m.Msg == wmNcCalcSize && m.WParam != IntPtr.Zero)
        {
            m.Result = IntPtr.Zero;
            return;
        }

        if (m.Msg == wmNcHitTest && WindowState != FormWindowState.Maximized)
        {
            Point cursor = PointToClient(new Point(SignedLowWord(m.LParam), SignedHighWord(m.LParam)));
            int grip = 10;
            bool inside = cursor.X >= 0 && cursor.Y >= 0 && cursor.X <= ClientSize.Width && cursor.Y <= ClientSize.Height;
            bool left = cursor.X <= grip;
            bool right = cursor.X >= ClientSize.Width - grip;
            bool top = cursor.Y <= grip;
            bool bottom = cursor.Y >= ClientSize.Height - grip;

            if (inside && left && top) { m.Result = new IntPtr(htTopLeft); return; }
            if (inside && right && top) { m.Result = new IntPtr(htTopRight); return; }
            if (inside && left && bottom) { m.Result = new IntPtr(htBottomLeft); return; }
            if (inside && right && bottom) { m.Result = new IntPtr(htBottomRight); return; }
            if (inside && left) { m.Result = new IntPtr(htLeft); return; }
            if (inside && right) { m.Result = new IntPtr(htRight); return; }
            if (inside && top) { m.Result = new IntPtr(htTop); return; }
            if (inside && bottom) { m.Result = new IntPtr(htBottom); return; }
        }

        base.WndProc(ref m);
    }

    static int SignedLowWord(IntPtr value)
    {
        return (short)((long)value & 0xFFFF);
    }

    static int SignedHighWord(IntPtr value)
    {
        return (short)(((long)value >> 16) & 0xFFFF);
    }

    [STAThread]
    public static void Main()
    {
        EnableProcessDpiAwareness();
        Application.EnableVisualStyles();
        Application.SetCompatibleTextRenderingDefault(false);
        Application.SetUnhandledExceptionMode(UnhandledExceptionMode.CatchException);
        Application.ThreadException += delegate(object sender, System.Threading.ThreadExceptionEventArgs e)
        {
            HandleUnhandledException(e.Exception);
        };
        AppDomain.CurrentDomain.UnhandledException += delegate(object sender, UnhandledExceptionEventArgs e)
        {
            Exception exception = e.ExceptionObject as Exception;
            if (exception != null)
            {
                HandleUnhandledException(exception);
            }
        };
        Application.Run(new LocalAreaInterconnectionDesktop());
    }

    static void EnableProcessDpiAwareness()
    {
        try
        {
            Native.SetProcessDpiAwareness(2);
            return;
        }
        catch
        {
        }

        try
        {
            Native.SetProcessDPIAware();
        }
        catch
        {
        }
    }

    static void HandleUnhandledException(Exception exception)
    {
        LocalAreaInterconnectionDesktop window = activeWindow;
        if (window == null || window.IsDisposed) return;
        try
        {
            if (window.InvokeRequired)
            {
                window.BeginInvoke((MethodInvoker)delegate { window.ShowActionError("appTitle", exception); });
            }
            else
            {
                window.ShowActionError("appTitle", exception);
            }
        }
        catch
        {
        }
    }

    public LocalAreaInterconnectionDesktop()
    {
        activeWindow = this;
        Text = "LocalAreaInterconnection";
        Width = 1180;
        Height = 720;
        StartPosition = FormStartPosition.CenterScreen;
        FormBorderStyle = FormBorderStyle.None;
        MinimumSize = new Size(980, 640);
        DoubleBuffered = true;
        SetStyle(ControlStyles.AllPaintingInWmPaint | ControlStyles.UserPaint | ControlStyles.OptimizedDoubleBuffer, true);
        BackColor = ShellDark;
        Font = new Font("Segoe UI", 9);
        Icon = Icon.ExtractAssociatedIcon(Application.ExecutablePath);
        language = LoadLanguage();
        chromeTips = new ToolTip();
        chromeTips.BackColor = CardDark;
        chromeTips.ForeColor = TextBright;
        Resize += delegate { UpdateWindowRegion(); };
        UpdateWindowRegion();

        particles = new Particle[ParticleCount];
        for (int i = 0; i < particles.Length; i++)
        {
            particles[i] = NewParticle();
        }

        TableLayoutPanel shell = new TableLayoutPanel();
        shell.Dock = DockStyle.Fill;
        shell.BackColor = Color.Transparent;
        shell.ColumnCount = 2;
        shell.RowCount = 2;
        shell.ColumnStyles.Add(new ColumnStyle(SizeType.Absolute, 200));
        shell.ColumnStyles.Add(new ColumnStyle(SizeType.Percent, 100));
        shell.RowStyles.Add(new RowStyle(SizeType.Absolute, 38));
        shell.RowStyles.Add(new RowStyle(SizeType.Percent, 100));
        Controls.Add(shell);

        Control titleBar = TitleBar();
        shell.Controls.Add(titleBar, 0, 0);
        shell.SetColumnSpan(titleBar, 2);

        // ===== Sidebar (navigation rail) =====
        sidebarPanel = BuildSidebar();
        shell.Controls.Add(sidebarPanel, 0, 1);

        // ===== Content area: pages share the output console =====
        contentArea = new Panel();
        contentArea.Dock = DockStyle.Fill;
        contentArea.BackColor = Color.Transparent;
        contentArea.Padding = new Padding(0);
        shell.Controls.Add(contentArea, 1, 1);

        BuildPages();

        ApplyLanguage();
        UpdateRoomDetails("idle");
        Resize += delegate { AdjustActionLayout(); };
        AdjustActionLayout();
        SelectPage("home");

        runtimeStatusTimer = new Timer();
        runtimeStatusTimer.Interval = 1500;
        runtimeStatusTimer.Tick += delegate { RefreshRuntimeStatus(); };
        runtimeStatusTimer.Start();

        particleTimer = new Timer();
        particleTimer.Interval = 120;
        particleTimer.Tick += TickParticles;
        particleTimer.Start();
        VisibleChanged += delegate { if (particleTimer != null) { if (Visible) particleTimer.Start(); else particleTimer.Stop(); } };
        FormClosing += delegate
        {
            StopRuntimeProcess(1500);
            StopCoordinationProcess(1500);
        };
        FormClosed += delegate
        {
            if (activeWindow == this) activeWindow = null;
        };
    }

    // ===== Sidebar navigation rail =====
    Panel BuildSidebar()
    {
        Panel rail = new Panel();
        rail.Dock = DockStyle.Fill;
        rail.BackColor = SidebarDark;
        rail.Paint += PaintSidebarBackground;
        rail.Padding = new Padding(0);
        rail.MouseDown += BeginDrag;

        // Brand block at top: circular logo + app name.
        Panel brand = new Panel();
        brand.Width = 200;
        brand.Height = 70;
        brand.Top = 14;
        brand.Left = 0;
        brand.BackColor = Color.Transparent;
        brand.Paint += PaintSidebarBrand;
        brand.MouseDown += BeginDrag;
        rail.Controls.Add(brand);

        AddNavButton(rail, "home", "navHome", 96);
        AddNavButton(rail, "diagnose", "navDiagnose", 140);
        AddNavButton(rail, "games", "navGames", 184);
        AddNavButton(rail, "tools", "navTools", 228);
        AddNavButton(rail, "about", "navAbout", 272);

        return rail;
    }

    void PaintSidebarBackground(object sender, PaintEventArgs e)
    {
        Control c = (Control)sender;
        Rectangle r = new Rectangle(0, 0, c.Width, c.Height);
        if (r.Width <= 0 || r.Height <= 0) return;
        Rectangle top = new Rectangle(0, 0, c.Width, Math.Max(1, c.Height / 2));
        Rectangle bottom = new Rectangle(0, top.Bottom, c.Width, Math.Max(1, c.Height - top.Height));
        using (LinearGradientBrush brush = new LinearGradientBrush(top, SidebarDark, SidebarMid, LinearGradientMode.Vertical))
        {
            e.Graphics.FillRectangle(brush, top);
        }
        using (LinearGradientBrush brush = new LinearGradientBrush(bottom, SidebarMid, SidebarDeep, LinearGradientMode.Vertical))
        {
            e.Graphics.FillRectangle(brush, bottom);
        }
    }

    void PaintSidebarBrand(object sender, PaintEventArgs e)
    {
        Control c = (Control)sender;
        UseCrispGraphics(e.Graphics);
        // circular logo disc with cyan ring
        int size = 38;
        int x = 16;
        int y = 6;
        using (GraphicsPath disc = new GraphicsPath())
        {
            disc.AddEllipse(x, y, size, size);
            using (PathGradientBrush fill = new PathGradientBrush(disc))
            {
                fill.CenterColor = Color.FromArgb(60, AccentCyan);
                fill.SurroundColors = new Color[] { Color.FromArgb(16, AccentCyan) };
                e.Graphics.FillPath(fill, disc);
            }
            using (Pen ring = new Pen(AccentCyan, 1.6f))
            {
                e.Graphics.DrawEllipse(ring, x + 1, y + 1, size - 2, size - 2);
            }
        }
        if (Icon != null)
        {
            try
            {
                using (Bitmap ico = new Bitmap(Icon.ToBitmap(), 22, 22))
                {
                    using (GraphicsPath clip = new GraphicsPath())
                    {
                        clip.AddEllipse(x + 8, y + 8, 22, 22);
                        e.Graphics.SetClip(clip);
                        e.Graphics.DrawImage(ico, x + 8, y + 8, 22, 22);
                        e.Graphics.ResetClip();
                    }
                }
            }
            catch { }
        }
        string title = T("appTitle");
        string tag = T("appTagline");
        using (Font titleFont = new Font(Font.FontFamily, 10.5f, FontStyle.Bold))
        {
            TextRenderer.DrawText(e.Graphics, title, titleFont, new Point(x + size + 8, y + 0), TextBright, TextFormatFlags.Left | TextFormatFlags.Top);
        }
        using (Font tagFont = new Font(Font.FontFamily, 8f, FontStyle.Regular))
        {
            TextRenderer.DrawText(e.Graphics, tag, tagFont, new Point(x + size + 8, y + 20), TextMuted, TextFormatFlags.Left | TextFormatFlags.Top);
        }
    }

    void AddNavButton(Panel parent, string page, string key, int top)
    {
        Button button = new Button();
        button.Left = 8;
        button.Top = top;
        button.Width = 184;
        button.Height = 36;
        button.FlatStyle = FlatStyle.Flat;
        button.FlatAppearance.BorderSize = 0;
        button.BackColor = Color.Transparent;
        button.ForeColor = TextMuted;
        button.Font = new Font(Font.FontFamily, 9.5f, FontStyle.Regular);
        button.TextAlign = ContentAlignment.MiddleLeft;
        button.Padding = new Padding(16, 0, 0, 0);
        button.TabStop = false;
        button.UseVisualStyleBackColor = false;
        button.FlatAppearance.MouseOverBackColor = Color.FromArgb(27, 82, 91);
        button.FlatAppearance.MouseDownBackColor = Color.FromArgb(20, 66, 75);
        button.Tag = page;
        button.Paint += delegate(object sender, PaintEventArgs e) { PaintNavButton((Button)sender, e); };
        button.Click += delegate { SelectPage(page); };
        button.MouseDown += BeginDrag;
        parent.Controls.Add(button);
        navButtons.Add(button);
        buttonControls["nav_" + page] = button;
    }

    void PaintNavButton(Button button, PaintEventArgs e)
    {
        UseCrispGraphics(e.Graphics);
        string page = button.Tag as string;
        bool selected = activePage == page;
        bool hover = button.ClientRectangle.Contains(button.PointToClient(Cursor.Position));

        Rectangle r = new Rectangle(0, 0, button.Width - 1, button.Height - 1);
        using (GraphicsPath path = RoundedRectPath(r, 9))
        {
            Color bg = selected ? Color.FromArgb(18, 96, 106) : hover ? Color.FromArgb(27, 82, 91) : SidebarMid;
            using (SolidBrush brush = new SolidBrush(bg))
            {
                e.Graphics.FillPath(brush, path);
            }
        }
        // selected: left cyan accent bar + soft glow
        if (selected)
        {
            using (GraphicsPath bar = new GraphicsPath())
            {
                bar.AddArc(new Rectangle(2, button.Height / 2 - 9, 4, 4), 180, 90);
                bar.AddArc(new Rectangle(2, button.Height / 2 + 5, 4, 4), 90, 90);
                bar.AddLine(4, button.Height / 2 + 7, 4, button.Height / 2 - 7);
                bar.CloseFigure();
                using (SolidBrush b = new SolidBrush(AccentCyan))
                {
                    e.Graphics.FillPath(b, bar);
                }
            }
        }

        Color text = selected ? AccentCyan : hover ? TextBright : TextMuted;
        string page2 = page;
        string key = page2 == "home" ? "navHome"
            : page2 == "diagnose" ? "navDiagnose"
            : page2 == "games" ? "navGames"
            : page2 == "tools" ? "navTools"
            : "navAbout";
        DrawNavIcon(e.Graphics, page2, 18, button.Height / 2 - 9, selected ? AccentCyan : (hover ? TextBright : TextMuted));
        TextRenderer.DrawText(e.Graphics, T(key), button.Font, new Rectangle(40, 0, button.Width - 44, button.Height), text, TextFormatFlags.Left | TextFormatFlags.VerticalCenter | TextFormatFlags.EndEllipsis);
    }

    void DrawNavIcon(Graphics g, string page, int x, int y, Color color)
    {
        using (Pen pen = new Pen(color, 1.6f))
        {
            pen.StartCap = LineCap.Round;
            pen.EndCap = LineCap.Round;
            if (page == "home")
            {
                g.DrawLine(pen, x, y + 8, x + 9, y);
                g.DrawLine(pen, x + 9, y, x + 18, y + 8);
                g.DrawRectangle(pen, x + 4, y + 7, 10, 8);
            }
            else if (page == "diagnose")
            {
                g.DrawEllipse(pen, x + 1, y + 1, 16, 16);
                g.DrawLine(pen, x + 9, y + 9, x + 15, y + 15);
            }
            else if (page == "games")
            {
                g.DrawRectangle(pen, x, y + 1, 18, 14);
                g.DrawLine(pen, x + 4, y + 1, x + 4, y + 15);
                g.DrawLine(pen, x + 10, y + 1, x + 10, y + 15);
            }
            else if (page == "tools")
            {
                g.DrawEllipse(pen, x + 3, y + 3, 6, 6);
                g.DrawEllipse(pen, x + 11, y + 9, 6, 6);
                g.DrawLine(pen, x + 7, y + 7, x + 13, y + 11);
            }
            else // about
            {
                g.DrawEllipse(pen, x + 8, y, 2, 2);
                g.DrawLine(pen, x + 9, y + 5, x + 9, y + 15);
            }
        }
    }

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

        output = new TextBox();
        output.Multiline = true;
        output.ScrollBars = ScrollBars.None;
        output.WordWrap = true;
        output.Dock = DockStyle.Fill;
        output.Font = new System.Drawing.Font("Consolas", 10);
        StyleTextBox(output);

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

    void BuildHomePage()
    {
        pageHome = new Panel();
        pageHome.Dock = DockStyle.Fill;
        pageHome.BackColor = Color.Transparent;
        pageHome.Padding = new Padding(0, 0, 0, 202);
        contentArea.Controls.Add(pageHome);
        contentPages.Add(pageHome);

        Label title = PageTitle("navHome");
        title.Dock = DockStyle.Top;
        title.Height = 40;
        title.Padding = new Padding(16, 8, 0, 0);
        pageHome.Controls.Add(title);

        TableLayoutPanel body = new TableLayoutPanel();
        body.Dock = DockStyle.Fill;
        body.ColumnCount = 2;
        body.RowCount = 1;
        body.ColumnStyles.Add(new ColumnStyle(SizeType.Percent, 58));
        body.ColumnStyles.Add(new ColumnStyle(SizeType.Percent, 42));
        body.Padding = new Padding(16, 0, 16, 12);
        pageHome.Controls.Add(body);
        body.BringToFront();

        // Left: quick-flow card (fields + main actions)
        Panel leftCard = CardPanel();
        Panel leftInner = (Panel)leftCard.Tag;

        TableLayoutPanel quickLayout = new TableLayoutPanel();
        quickLayout.Dock = DockStyle.Fill;
        quickLayout.BackColor = Color.Transparent;
        quickLayout.ColumnCount = 1;
        quickLayout.RowCount = 3;
        quickLayout.RowStyles.Add(new RowStyle(SizeType.Absolute, 130));
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
        leftTable.RowCount = 3;
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

        Label title = PageTitle("navDiagnose");
        title.Dock = DockStyle.Top;
        title.Height = 40;
        title.Padding = new Padding(16, 8, 0, 0);
        pageDiagnose.Controls.Add(title);

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

        pageDiagnose.Controls.Add(card);
    }

    void BuildGamesPage()
    {
        pageGames = new Panel();
        pageGames.Dock = DockStyle.Fill;
        pageGames.BackColor = Color.Transparent;
        pageGames.Padding = new Padding(0, 0, 0, 202);
        contentArea.Controls.Add(pageGames);
        contentPages.Add(pageGames);

        Label title = PageTitle("navGames");
        title.Dock = DockStyle.Top;
        title.Height = 40;
        title.Padding = new Padding(16, 8, 0, 0);
        pageGames.Controls.Add(title);

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

        pageGames.Controls.Add(card);
    }

    void BuildToolsPage()
    {
        pageTools = new Panel();
        pageTools.Dock = DockStyle.Fill;
        pageTools.BackColor = Color.Transparent;
        pageTools.Padding = new Padding(0, 0, 0, 202);
        contentArea.Controls.Add(pageTools);
        contentPages.Add(pageTools);

        Label title = PageTitle("navTools");
        title.Dock = DockStyle.Top;
        title.Height = 40;
        title.Padding = new Padding(16, 8, 0, 0);
        pageTools.Controls.Add(title);

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
        advancedActionsVisible = true;

        pageTools.Controls.Add(card);
    }

    void AddToolButton(string key, EventHandler handler)
    {
        Button button = new Button();
        button.Text = T(key);
        button.Width = Math.Min(184, Math.Max(116, TextRenderer.MeasureText(button.Text, Font).Width + 24));
        button.Height = 30;
        button.Margin = new Padding(0, 0, 8, 8);
        button.FlatStyle = FlatStyle.Flat;
        button.BackColor = Color.FromArgb(34, 38, 44);
        button.ForeColor = Color.FromArgb(224, 224, 224);
        button.Font = new Font(Font, FontStyle.Regular);
        button.FlatAppearance.BorderColor = AccentCyan;
        button.FlatAppearance.MouseOverBackColor = Color.FromArgb(44, 52, 60);
        button.FlatAppearance.MouseDownBackColor = Color.FromArgb(26, 30, 36);
        button.Click += delegate(object sender, EventArgs e) { RunUserAction(key, handler, sender, e); };
        button.MouseWheel += ScrollActionsWheel;
        button.Resize += delegate { ApplyRoundedRegion(button, 8); };
        ApplyRoundedRegion(button, 8);
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

        Label title = PageTitle("navAbout");
        title.Dock = DockStyle.Top;
        title.Height = 40;
        title.Padding = new Padding(16, 8, 0, 0);
        pageAbout.Controls.Add(title);

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

        pageAbout.Controls.Add(card);
    }

    Control TitleBar()
    {
        Panel bar = new Panel();
        bar.Dock = DockStyle.Fill;
        bar.BackColor = TitleDark;
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
        titleLabel.ForeColor = TextBright;
        titleLabel.BackColor = Color.Transparent;
        titleLabel.AutoSize = true;
        titleLabel.Left = 40;
        titleLabel.Top = 10;
        titleLabel.MouseDown += BeginDrag;
        bar.Controls.Add(titleLabel);

        languageButton = new Button();
        languageButton.Width = 92;
        languageButton.Height = 26;
        languageButton.Top = 6;
        languageButton.FlatStyle = FlatStyle.Flat;
        languageButton.FlatAppearance.BorderSize = 1;
        languageButton.FlatAppearance.BorderColor = Color.FromArgb(86, 86, 86);
        languageButton.FlatAppearance.MouseOverBackColor = Color.FromArgb(56, 56, 56);
        languageButton.FlatAppearance.MouseDownBackColor = Color.FromArgb(38, 38, 38);
        languageButton.BackColor = Color.FromArgb(38, 38, 38);
        languageButton.ForeColor = TextBright;
        languageButton.TextAlign = ContentAlignment.MiddleCenter;
        languageButton.TabStop = false;
        languageButton.UseVisualStyleBackColor = false;
        languageButton.Resize += delegate { ApplyRoundedRegion(languageButton, 6); };
        languageButton.Paint += PaintLanguageButton;
        languageButton.Click += delegate
        {
            language = language == "zh" ? "en" : "zh";
            SaveLanguage();
            ApplyLanguage();
            UpdateRoomDetails("idle");
        };
        bar.Controls.Add(languageButton);
        ApplyRoundedRegion(languageButton, 6);

        Button closeButton = AddChromeButton(bar, ChromeGlyph.Close, "closeTip", 0, delegate { Close(); });
        Button maximizeButton = AddChromeButton(bar, ChromeGlyph.Maximize, "maximizeTip", 0, delegate { WindowState = WindowState == FormWindowState.Maximized ? FormWindowState.Normal : FormWindowState.Maximized; });
        Button minimizeButton = AddChromeButton(bar, ChromeGlyph.Minimize, "minimizeTip", 0, delegate { WindowState = FormWindowState.Minimized; });
        bar.Resize += delegate
        {
            languageButton.Left = bar.Width - 230;
            closeButton.Left = bar.Width - 46;
            maximizeButton.Left = bar.Width - 86;
            minimizeButton.Left = bar.Width - 126;
        };
        languageButton.Left = bar.Width - 230;
        closeButton.Left = bar.Width - 46;
        maximizeButton.Left = bar.Width - 86;
        minimizeButton.Left = bar.Width - 126;
        return bar;
    }

    Button AddChromeButton(Panel bar, ChromeGlyph glyph, string tipKey, int left, EventHandler handler)
    {
        Button button = new Button();
        button.Left = left;
        button.Top = 5;
        button.Width = 34;
        button.Height = 28;
        button.FlatStyle = FlatStyle.Flat;
        button.FlatAppearance.BorderSize = 0;
        button.BackColor = TitleDark;
        button.ForeColor = Color.FromArgb(218, 218, 218);
        button.TabStop = false;
        button.UseVisualStyleBackColor = false;
        button.FlatAppearance.MouseOverBackColor = TitleDark;
        button.FlatAppearance.MouseDownBackColor = TitleDark;
        button.Click += delegate(object sender, EventArgs e)
        {
            RunUserAction(tipKey, handler, sender, e);
        };
        button.Paint += delegate(object sender, PaintEventArgs e)
        {
            PaintChromeGlyph((Button)sender, e, glyph);
        };
        chromeTips.SetToolTip(button, T(tipKey));
        button.Tag = tipKey;
        bar.Controls.Add(button);
        return button;
    }

    void BeginDrag(object sender, MouseEventArgs e)
    {
        if (e.Button != MouseButtons.Left) return;
        Native.ReleaseCapture();
        Native.SendMessage(Handle, 0xA1, new IntPtr(0x2), IntPtr.Zero);
    }

    void PaintLanguageButton(object sender, PaintEventArgs e)
    {
        Button button = (Button)sender;
        UseCrispGraphics(e.Graphics);
        using (GraphicsPath path = RoundedRectPath(new Rectangle(0, 0, button.Width - 1, button.Height - 1), 6))
        using (SolidBrush background = new SolidBrush(button.BackColor))
        {
            e.Graphics.FillPath(background, path);
            using (Pen border = new Pen(Color.FromArgb(86, 86, 86)))
            {
                e.Graphics.DrawPath(border, path);
            }
        }

        string text = language == "zh" ? "中文" : "English";
        TextRenderer.DrawText(
            e.Graphics,
            text,
            Font,
            new Rectangle(8, 1, button.Width - 24, button.Height - 2),
            button.ForeColor,
            TextFormatFlags.VerticalCenter | TextFormatFlags.Left | TextFormatFlags.EndEllipsis);

        Point[] arrow = new Point[]
        {
            new Point(button.Width - 17, button.Height / 2 - 2),
            new Point(button.Width - 9, button.Height / 2 - 2),
            new Point(button.Width - 13, button.Height / 2 + 3)
        };
        using (SolidBrush brush = new SolidBrush(TextMuted))
        {
            e.Graphics.FillPolygon(brush, arrow);
        }
    }

    void ApplyRoundedRegion(Control control, int radius)
    {
        if (control == null || control.Width <= 0 || control.Height <= 0) return;
        if (control.Region != null)
        {
            control.Region.Dispose();
        }
        using (GraphicsPath path = RoundedRectPath(new Rectangle(0, 0, control.Width, control.Height), radius))
        {
            control.Region = new Region(path);
        }
    }

    void UpdateWindowRegion()
    {
        if (WindowState == FormWindowState.Maximized)
        {
            if (Region != null)
            {
                Region.Dispose();
                Region = null;
            }
            return;
        }
        ApplyRoundedRegion(this, 14);
    }

    GraphicsPath RoundedRectPath(Rectangle bounds, int radius)
    {
        int diameter = Math.Max(1, radius * 2);
        GraphicsPath path = new GraphicsPath();
        Rectangle arc = new Rectangle(bounds.Left, bounds.Top, diameter, diameter);
        path.AddArc(arc, 180, 90);
        arc.X = bounds.Right - diameter;
        path.AddArc(arc, 270, 90);
        arc.Y = bounds.Bottom - diameter;
        path.AddArc(arc, 0, 90);
        arc.X = bounds.Left;
        path.AddArc(arc, 90, 90);
        path.CloseFigure();
        return path;
    }

    void PaintChromeGlyph(Button button, PaintEventArgs e, ChromeGlyph glyph)
    {
        UseCrispGraphics(e.Graphics);
        bool hover = button.ClientRectangle.Contains(button.PointToClient(Cursor.Position));
        Color background = glyph == ChromeGlyph.Close && hover
            ? Color.FromArgb(184, 54, 54)
            : hover ? Color.FromArgb(48, 48, 48) : TitleDark;
        using (GraphicsPath path = RoundedRectPath(new Rectangle(0, 0, button.Width - 1, button.Height - 1), 8))
        using (SolidBrush brush = new SolidBrush(background))
        {
            e.Graphics.FillPath(brush, path);
        }

        Color glyphColor = glyph == ChromeGlyph.Close && hover
            ? Color.White
            : Color.FromArgb(218, 218, 218);
        using (Pen pen = new Pen(glyphColor, 1.9f))
        {
            pen.StartCap = LineCap.Round;
            pen.EndCap = LineCap.Round;
            int centerX = button.Width / 2;
            int centerY = button.Height / 2;
            if (glyph == ChromeGlyph.Minimize)
            {
                e.Graphics.DrawLine(pen, centerX - 5, centerY + 5, centerX + 5, centerY + 5);
            }
            else if (glyph == ChromeGlyph.Maximize)
            {
                e.Graphics.DrawRectangle(pen, centerX - 5, centerY - 5, 10, 10);
            }
            else
            {
                e.Graphics.DrawLine(pen, centerX - 5, centerY - 5, centerX + 5, centerY + 5);
                e.Graphics.DrawLine(pen, centerX + 5, centerY - 5, centerX - 5, centerY + 5);
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
        label.ForeColor = Color.FromArgb(226, 238, 238);
        label.BackColor = Color.Transparent;
        label.Font = new Font(Font.FontFamily, 9.5f, FontStyle.Regular);
        label.Padding = new Padding(6, 0, 0, 0);
        labelControls[key] = label;
        return label;
    }

    Button AddButton(FlowLayoutPanel panel, string key, EventHandler handler)
    {
        return AddButton(panel, key, handler, false);
    }

    Button AddButton(FlowLayoutPanel panel, string key, EventHandler handler, bool advanced)
    {
        Button button = new Button();
        button.Text = T(key);
        button.Width = Math.Min(184, Math.Max(116, TextRenderer.MeasureText(button.Text, Font).Width + 24));
        button.Height = advanced ? 28 : 32;
        button.Margin = new Padding(0, 0, 8, 8);
        button.FlatStyle = FlatStyle.Flat;
        button.BackColor = advanced ? Color.FromArgb(34, 38, 44) : AccentCyan;
        button.ForeColor = advanced ? Color.FromArgb(224, 224, 224) : Color.FromArgb(8, 30, 34);
        button.Font = new Font(Font, advanced ? FontStyle.Regular : FontStyle.Bold);
        button.FlatAppearance.BorderColor = advanced ? AccentCyan : AccentCyan;
        button.FlatAppearance.MouseOverBackColor = advanced ? Color.FromArgb(44, 52, 60) : AccentCyanHover;
        button.FlatAppearance.MouseDownBackColor = advanced ? Color.FromArgb(26, 30, 36) : AccentCyanDown;
        button.Click += delegate(object sender, EventArgs e)
        {
            RunUserAction(key, handler, sender, e);
        };
        button.MouseWheel += ScrollActionsWheel;
        button.Resize += delegate { ApplyRoundedRegion(button, 8); };
        ApplyRoundedRegion(button, 8);
        buttonControls[key] = button;
        if (advanced)
        {
            advancedActionButtons.Add(button);
            button.Visible = advancedActionsVisible;
        }
        panel.Controls.Add(button);
        return button;
    }

    void ToggleAdvancedActions()
    {
        advancedActionsVisible = !advancedActionsVisible;
        UpdateAdvancedActions();
    }

    void RunUserAction(string key, EventHandler handler, object sender, EventArgs e)
    {
        try
        {
            handler(sender, e);
        }
        catch (Exception ex)
        {
            ShowActionError(key, ex);
        }
    }

    void ShowActionError(string key, Exception ex)
    {
        if (output == null) return;
        output.Text = T("actionCouldNotFinish")
            + Environment.NewLine
            + ActionNextHint(key)
            + Environment.NewLine
            + Environment.NewLine
            + T("technicalSummary")
            + Environment.NewLine
            + ex.Message;
    }

    string ActionNextHint(string key)
    {
        if (key == "quickJoinRoom" || key == "decodeInvite" || key == "joinRoom")
        {
            return T("joinNeedsInvite");
        }
        if (key == "startLanSession" || key == "startRuntime" || key == "nativeOffer")
        {
            return T("startNeedsRoom");
        }
        if (key == "copyInvite")
        {
            return T("copyInviteNeedsRoom");
        }
        if (key == "quickHostRoom" || key == "createRoom")
        {
            return T("hostNeedsName");
        }
        return T("tryMainFlowAgain");
    }

    void SetActionButtonsEnabled(bool enabled)
    {
        if (actionsPanel == null) return;
        foreach (Control control in actionsPanel.Controls)
        {
            control.Enabled = enabled;
        }
    }

    void UpdateAdvancedActions()
    {
        foreach (Button button in advancedActionButtons)
        {
            button.Visible = advancedActionsVisible;
        }
        if (moreToolsButton != null)
        {
            moreToolsButton.Text = T(advancedActionsVisible ? "hideTools" : "moreTools");
        }
        actionScrollOffset = 0;
        AdjustActionLayout();
    }

    void AdjustActionLayout()
    {
        if (actionsPanel == null || actionsViewport == null) return;
        int available = Math.Max(220, actionsViewport.ClientSize.Width - 2);
        int columns = Math.Max(2, Math.Min(3, available / 136));
        int width = Math.Max(112, (available / columns) - 8);
        int visibleControls = 0;
        foreach (Control control in actionsPanel.Controls)
        {
            if (!control.Visible) continue;
            control.Width = width;
            Button btn = control as Button;
            if (btn != null)
            {
                btn.Height = advancedActionButtons.Contains(btn) ? 28 : 30;
            }
            visibleControls++;
        }

        int rows = (int)Math.Ceiling(visibleControls / (double)columns);
        int contentHeight = Math.Max(actionsViewport.ClientSize.Height, rows * 40);
        actionsPanel.SetBounds(0, -actionScrollOffset, available, contentHeight);
        ClampActionScroll();
        UpdateActionScrollBar();
    }

    void ScrollActionsWheel(object sender, MouseEventArgs e)
    {
        int step = e.Delta > 0 ? -42 : 42;
        SetActionScrollOffset(actionScrollOffset + step);
    }

    void BeginActionScrollThumbDrag(object sender, MouseEventArgs e)
    {
        if (e.Button != MouseButtons.Left) return;
        draggingActionScrollThumb = true;
        actionScrollDragStartY = actionScrollBar.PointToClient(actionScrollThumb.PointToScreen(e.Location)).Y;
        actionScrollStartOffset = actionScrollOffset;
    }

    void DragActionScrollThumb(object sender, MouseEventArgs e)
    {
        if (!draggingActionScrollThumb) return;
        int currentY = actionScrollBar.PointToClient(actionScrollThumb.PointToScreen(e.Location)).Y;
        int track = Math.Max(1, actionScrollBar.ClientSize.Height - actionScrollThumb.Height - 4);
        int maxOffset = MaxActionScrollOffset();
        int deltaOffset = (currentY - actionScrollDragStartY) * maxOffset / track;
        SetActionScrollOffset(actionScrollStartOffset + deltaOffset);
    }

    void EndActionScrollThumbDrag(object sender, MouseEventArgs e)
    {
        draggingActionScrollThumb = false;
    }

    void SetActionScrollOffset(int value)
    {
        actionScrollOffset = Math.Max(0, Math.Min(MaxActionScrollOffset(), value));
        if (actionsPanel != null)
        {
            actionsPanel.Top = -actionScrollOffset;
        }
        UpdateActionScrollBar();
    }

    void ClampActionScroll()
    {
        actionScrollOffset = Math.Max(0, Math.Min(MaxActionScrollOffset(), actionScrollOffset));
        if (actionsPanel != null)
        {
            actionsPanel.Top = -actionScrollOffset;
        }
    }

    int MaxActionScrollOffset()
    {
        if (actionsPanel == null || actionsViewport == null) return 0;
        return Math.Max(0, actionsPanel.Height - actionsViewport.ClientSize.Height);
    }

    void UpdateActionScrollBar()
    {
        if (actionScrollBar == null || actionScrollThumb == null || actionsPanel == null || actionsViewport == null) return;
        int maxOffset = MaxActionScrollOffset();
        bool visible = maxOffset > 0;
        actionScrollBar.Visible = visible;
        if (!visible) return;

        int trackHeight = Math.Max(1, actionScrollBar.ClientSize.Height - 4);
        int thumbHeight = Math.Max(28, actionsViewport.ClientSize.Height * trackHeight / Math.Max(actionsPanel.Height, 1));
        int travel = Math.Max(1, trackHeight - thumbHeight);
        int thumbTop = 2 + (actionScrollOffset * travel / maxOffset);
        actionScrollThumb.SetBounds(2, thumbTop, Math.Max(4, actionScrollBar.Width - 4), thumbHeight);
        ApplyRoundedRegion(actionScrollThumb, 4);
        actionScrollBar.Invalidate();
    }

    void PaintActionScrollBar(object sender, PaintEventArgs e)
    {
        e.Graphics.Clear(actionScrollBar.BackColor);
        using (Pen pen = new Pen(Color.FromArgb(76, 76, 76)))
        {
            e.Graphics.DrawLine(pen, actionScrollBar.Width / 2, 4, actionScrollBar.Width / 2, actionScrollBar.Height - 4);
        }
    }

    void StyleTextBox(TextBox box)
    {
        box.BackColor = FieldDark;
        box.ForeColor = TextBright;
        box.BorderStyle = BorderStyle.None;
        box.Margin = new Padding(0);
        if (!box.Multiline)
        {
            box.AutoSize = false;
            box.Height = Math.Max(20, TextRenderer.MeasureText("Ag", box.Font).Height + 2);
        }
    }

    Panel Framed(Control control)
    {
        return Framed(control, CardBorder);
    }

    Panel Framed(Control control, Color border)
    {
        Panel panel = new Panel();
        panel.Dock = DockStyle.Fill;
        panel.BackColor = border;
        panel.Padding = new Padding(1);
        panel.Margin = new Padding(0, 3, 10, 3);
        panel.Resize += delegate { ApplyRoundedRegion(panel, 10); };
        TextBox textBox = control as TextBox;
        if (textBox != null && !textBox.Multiline)
        {
            Panel field = new Panel();
            field.Dock = DockStyle.Fill;
            field.BackColor = FieldDark;
            field.Padding = new Padding(9, 0, 9, 0);
            field.Resize += delegate { LayoutSingleLineTextBox(field, textBox); };
            textBox.Dock = DockStyle.None;
            field.Controls.Add(textBox);
            panel.Controls.Add(field);
            LayoutSingleLineTextBox(field, textBox);
        }
        else
        {
            control.Dock = DockStyle.Fill;
            panel.Controls.Add(control);
        }
        return panel;
    }

    void LayoutSingleLineTextBox(Panel field, TextBox box)
    {
        if (field.ClientSize.Width <= 0 || field.ClientSize.Height <= 0) return;
        int left = field.Padding.Left;
        int width = Math.Max(1, field.ClientSize.Width - field.Padding.Left - field.Padding.Right);
        int top = Math.Max(1, (field.ClientSize.Height - box.Height) / 2);
        box.SetBounds(left, top, width, box.Height);
    }

    Panel RoomDetailsPanel()
    {
        Panel outer = new Panel();
        outer.Dock = DockStyle.Fill;
        outer.BackColor = CardBorder;
        outer.Padding = new Padding(1);
        outer.Margin = new Padding(14, 0, 0, 10);
        outer.Resize += delegate { ApplyRoundedRegion(outer, 12); };

        TableLayoutPanel details = new TableLayoutPanel();
        details.Dock = DockStyle.Fill;
        details.BackColor = CardDark;
        details.ColumnCount = 1;
        details.RowCount = 6;
        details.Padding = new Padding(18, 14, 18, 14);
        details.Resize += delegate { ApplyRoundedRegion(details, 12); };
        details.RowStyles.Add(new RowStyle(SizeType.Absolute, 32));
        details.RowStyles.Add(new RowStyle(SizeType.Percent, 16));
        details.RowStyles.Add(new RowStyle(SizeType.Percent, 18));
        details.RowStyles.Add(new RowStyle(SizeType.Percent, 14));
        details.RowStyles.Add(new RowStyle(SizeType.Percent, 34));
        details.RowStyles.Add(new RowStyle(SizeType.Percent, 18));

        Label header = new Label();
        header.Name = "roomDetailsHeader";
        header.Text = T("roomDetails");
        header.Dock = DockStyle.Fill;
        header.TextAlign = ContentAlignment.MiddleLeft;
        header.Font = new Font(Font.FontFamily, 10, FontStyle.Bold);
        header.ForeColor = TextBright;
        header.BackColor = Color.Transparent;
        labelControls["roomDetails"] = header;
        details.Controls.Add(header, 0, 0);

        roomSummary = DetailLabel();
        connectionSummary = DetailLabel();
        broadcastSummary = DetailLabel();
        memberSummary = DetailLabel();
        nextActionSummary = DetailLabel();
        memberSummary.AutoEllipsis = false;
        memberSummary.TextAlign = ContentAlignment.TopLeft;
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
        label.UseMnemonic = false;
        label.TextAlign = ContentAlignment.MiddleLeft;
        label.ForeColor = Color.FromArgb(216, 216, 216);
        label.BackColor = Color.Transparent;
        label.Padding = new Padding(0, 2, 0, 2);
        return label;
    }

    protected override void OnPaintBackground(PaintEventArgs e)
    {
        e.Graphics.Clear(ShellDark);
        // Particles are now drawn on the form background by OnPaint so they
        // render as a subtle cyan layer behind docked content panels.
    }

    protected override void OnPaint(PaintEventArgs e)
    {
        base.OnPaint(e);
        UseCrispGraphics(e.Graphics);
        // Paint the cyan particle field on the form surface. Docked panels
        // (sidebar + content pages) sit above this, so particles only show in
        // the title bar and any unoccupied padding.
        DrawParticles(e.Graphics);
    }

    void DrawParticles(Graphics graphics)
    {
        if (particles == null) return;
        for (int i = 0; i < particles.Length; i++)
        {
            Particle p = particles[i];
            Color dotColor = Color.FromArgb(Math.Min(60, p.Alpha), p.UseAlt ? ParticleCyan2 : ParticleCyan);
            using (SolidBrush glow = new SolidBrush(Color.FromArgb(10, p.UseAlt ? ParticleCyan2 : ParticleCyan)))
            {
                graphics.FillEllipse(glow, p.X - 1.2f, p.Y - 1.2f, p.Size + 2.4f, p.Size + 2.4f);
            }
            using (SolidBrush brush = new SolidBrush(dotColor))
            {
                graphics.FillEllipse(brush, p.X, p.Y, p.Size, p.Size);
            }
        }
    }

    void MoveParticles()
    {
        if (particles == null) return;
        int w = Math.Max(1, ClientSize.Width);
        int h = Math.Max(1, ClientSize.Height);
        for (int i = 0; i < particles.Length; i++)
        {
            Particle p = particles[i];
            p.Phase += 0.015f;
            p.X += p.Vx + (float)Math.Sin(p.Phase) * 0.12f;
            p.Y += p.Vy + (float)Math.Cos(p.Phase * 0.8f) * 0.10f;
            if (p.X > w + 20) { p.X = -20f; p.Y = random.Next(h); }
            else if (p.X < -30) { p.X = w + 10f; p.Y = random.Next(h); }
            if (p.Y < -30) { p.Y = h + 10f; p.X = random.Next(w); }
            else if (p.Y > h + 30) { p.Y = -20f; p.X = random.Next(w); }
        }
    }

    void TickParticles(object sender, EventArgs e)
    {
        if (!IsHandleCreated || WindowState == FormWindowState.Minimized) return;
        MoveParticles();
        Invalidate();
    }

    Particle NewParticle()
    {
        Particle particle = new Particle();
        int w = Math.Max(1, ClientSize.Width);
        int h = Math.Max(1, ClientSize.Height);
        particle.X = random.Next(w);
        particle.Y = random.Next(h);
        // slow, gentle drift instead of a hard horizontal sweep
        particle.Vx = -0.12f + (float)random.NextDouble() * 0.24f;
        particle.Vy = -0.10f - (float)random.NextDouble() * 0.18f;
        particle.Size = 1.2f + (float)random.NextDouble() * 1.8f;
        particle.Alpha = 30 + random.Next(80);
        particle.Phase = (float)(random.NextDouble() * Math.PI * 2);
        particle.UseAlt = random.Next(3) == 0;
        return particle;
    }

    void UseCrispGraphics(Graphics graphics)
    {
        graphics.SmoothingMode = SmoothingMode.AntiAlias;
        graphics.PixelOffsetMode = PixelOffsetMode.HighQuality;
        graphics.CompositingQuality = CompositingQuality.HighQuality;
        graphics.InterpolationMode = InterpolationMode.HighQualityBicubic;
    }

    class Particle
    {
        public float X;
        public float Y;
        public float Vx;
        public float Vy;
        public float Size;
        public int Alpha;
        public float Phase;
        public bool UseAlt;
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
        output.Text = T("quickLanStarting");
        Application.DoEvents();
        StartLocalCoordinationServer();
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
        if (hostPeer.Length > 0 && hostName.Text.Trim().Length == 0)
        {
            hostName.Text = hostPeer;
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
                output.Text = CustomerNetworkSummary(network, "");
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
            observePath = Path.Combine(AppDataDirectory(), "runtime-packets.txt");
            packetObservations.Text = observePath;
        }
        latestRuntimeObservationFile = observePath;
        latestRuntimeSnapshot = Path.Combine(AppDataDirectory(), "runtime-snapshot.json");
        runtimeStopFile = Path.Combine(AppDataDirectory(), "runtime.stop");
        if (File.Exists(runtimeStopFile)) File.Delete(runtimeStopFile);
        string peer = SafePeerId(hostName.Text);
        string publishOutput = PublishNativeOfferIfConfigured(peer, showDetails);
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
        if (latestNativeOfferFile.Length > 0)
        {
            output.Text += Environment.NewLine + T("nativeOfferPath") + latestNativeOfferFile;
        }
        if (showDetails && publishOutput.Length > 0)
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
                + " --room-id desktop_runtime"
                + " --peer-id " + Quote(SafePeerId(hostName.Text))
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
                + " --room-id desktop_runtime"
                + " --peer-id " + Quote(SafePeerId(hostName.Text))
                + " --virtual-ip " + ip.Text
                + " --subnet " + subnet.Text
                + " --adapter-name LocalAreaInterconnection"
                + " --packet-io-backend wintun"
                + " --restore-adapter true"
                + " --cleanup-routes true");
            string planPath = Path.Combine(AppDataDirectory(), "runtime-cleanup-plan-apply.json");
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
            + " --peer-a " + Quote(SafePeerId(hostName.Text) + "_a")
            + " --peer-b " + Quote(SafePeerId(hostName.Text) + "_b")
            + " --attempts 3"
            + " --interval-ms 0"
            + " --message desktop-nat");
    }

    void RunRelayFallbackPlan()
    {
        string peer = SafePeerId(hostName.Text);
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
        string peer = SafePeerId(hostName.Text);
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

        string text = RunNativeCli("connection-path-plan"
            + " --local-offer " + Quote(latestNativeOfferFile)
            + " --remote-offer " + Quote(remoteOffer)
            + " --p2p-status failed");
        UpdateRoomDetailsFromConnectionPathPlan(text);
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
        string testOutput = RunNativeCli(command + ObserveFileArgs());
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
        string runtimePeers = RuntimePeersText(json);
        string relayFallback = RuntimeRelayFallbackText(json);
        if (adapter.Length == 0) adapter = T("stateUnknown");
        if (tunnel.Length == 0) tunnel = T("stateUnknown");
        if (p2p.Length == 0) p2p = T("stateUnknown");
        if (path.Length == 0 || path == "skipped") path = T("stateUnknown");
        if (broadcast.Length == 0) broadcast = T("stateUnknown");
        if (gameTraffic.Length == 0) gameTraffic = T("stateUnknown");
        if (connectedPeers.Length == 0) connectedPeers = "0";
        if (heartbeatPackets.Length == 0) heartbeatPackets = "0";
        if (snapshotWrites.Length == 0) snapshotWrites = "0";

        roomSummary.Text = T("detailRoom") + " " + SafeText(roomName.Text) + " | " + T("detailSubnet") + " " + SafeText(subnet.Text);
        connectionSummary.Text = T("detailConnection") + " " + RuntimeStatusText()
            + ", " + T("detailTunnel") + "=" + tunnel
            + ", P2P=" + p2p
            + ", " + T("detailPath") + "=" + path;
        broadcastSummary.Text = T("detailBroadcast") + " " + broadcast + " | " + T("detailGameTraffic") + " " + gameTraffic;
        memberSummary.Text = T("detailMembers") + Environment.NewLine
            + (runtimePeers.Length > 0 ? runtimePeers : SafeText(hostName.Text) + " @ " + SafeText(ip.Text))
            + Environment.NewLine
            + T("runtimePeers") + "=" + connectedPeers
            + ", " + T("runtimeHeartbeats") + "=" + heartbeatPackets
            + ", " + T("runtimeSnapshots") + "=" + snapshotWrites;
        if (relayFallback.Length > 0)
        {
            memberSummary.Text += Environment.NewLine + T("detailRelay") + Environment.NewLine + relayFallback;
        }
        nextActionSummary.Text = T("detailNext") + " " + DiagnosticNextAction(adapter, tunnel, p2p, broadcast, gameTraffic);
    }

    void UpdateRoomDetailsFromRuntimeCleanupPlan(string json)
    {
        if (roomSummary == null || json.Trim().Length == 0) return;
        string backend = JsonStringValue(json, "packet_io_backend");
        bool restoreAdapter = JsonBoolValue(json, "restore_adapter");
        bool requiresElevation = JsonBoolValue(json, "requires_elevation");
        int commandCount = JsonObjectCount(JsonArrayValue(json, "commands"));
        int stepCount = JsonObjectCount(JsonArrayValue(json, "process_cleanup_steps"));
        if (backend.Length == 0) backend = T("stateUnknown");

        roomSummary.Text = T("detailRoom") + " desktop_runtime | " + T("detailSubnet") + " " + SafeText(subnet.Text);
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

        roomSummary.Text = T("detailRoom") + " desktop_runtime | " + T("detailSubnet") + " " + SafeText(subnet.Text);
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

        roomSummary.Text = T("detailRoom") + " desktop_runtime | " + T("detailSubnet") + " " + SafeText(subnet.Text);
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

        roomSummary.Text = T("detailRoom") + " desktop_runtime | " + T("detailSubnet") + " " + SafeText(subnet.Text);
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

        roomSummary.Text = T("detailRoom") + " desktop_runtime | " + T("detailSubnet") + " " + SafeText(subnet.Text);
        connectionSummary.Text = T("detailConnection") + " " + T("wintunStatus") + "=" + status
            + ", " + T("detailAdapter") + "=" + adapterName;
        broadcastSummary.Text = "wintun.dll " + dllPath;
        memberSummary.Text = error.Length > 0 ? error : SafeText(hostName.Text) + " @ " + SafeText(ip.Text);
        nextActionSummary.Text = T("detailNext") + " " + nextAction;
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
        if (localPeer.Length == 0) localPeer = SafePeerId(hostName.Text);
        if (remote.Length == 0) remote = RemotePeerIdForKick();
        if (p2pCount.Length == 0) p2pCount = "0";
        if (relayCount.Length == 0) relayCount = "0";
        if (relayEndpoint.Length == 0) relayEndpoint = T("stateUnknown");
        if (nextAction.Length == 0) nextAction = T("nextFixTunnel");

        roomSummary.Text = T("detailRoom") + " desktop_runtime | " + T("detailSubnet") + " " + SafeText(subnet.Text);
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
        string endpoint = JsonFirstStringInArray(JsonArrayValue(json, "selected_endpoints"));
        string nextAction = JsonFirstStringInArray(JsonArrayValue(json, "recommended_actions"));
        if (status.Length == 0) status = T("stateUnknown");
        if (selectedPath.Length == 0) selectedPath = T("stateUnknown");
        if (localNat.Length == 0) localNat = T("stateUnknown");
        if (remoteNat.Length == 0) remoteNat = T("stateUnknown");
        if (endpoint.Length == 0) endpoint = T("stateUnknown");
        if (nextAction.Length == 0) nextAction = T("nextFixTunnel");

        roomSummary.Text = T("detailRoom") + " desktop_runtime | " + T("detailSubnet") + " " + SafeText(subnet.Text);
        connectionSummary.Text = T("detailConnection") + " " + T("connectionPathPlan") + "=" + status
            + ", " + T("detailPath") + "=" + selectedPath;
        broadcastSummary.Text = "NAT local=" + localNat + ", remote=" + remoteNat;
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
        string server = coordinationServer.Text.Trim();
        string peer = remotePeer.Text.Trim();
        if (server.Length > 0 && peer.Length > 0)
        {
            args += " --coordination-server " + Quote(server);
            args += " --coordination-peer " + Quote(peer);
        }
        return args;
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
        string peer = SafePeerId(hostName.Text);
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
            + " --room-id desktop_runtime"
            + " --peer-id " + Quote(localPeer));
        string offer = NatOfferObjectByPeer(fetch, remoteId);
        if (offer.Length == 0)
        {
            return "";
        }
        string path = Path.Combine(AppDataDirectory(), "remote-offer-relay-" + remoteId + ".json");
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
            string path = Path.Combine(AppDataDirectory(), fileName);
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
            + " --room-id desktop_runtime"
            + " --closed-by " + Quote(SafePeerId(hostName.Text)));
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
        string kickedBy = SafePeerId(hostName.Text);
        string result = RunNativeCliCapture("coordination-http-kick"
            + " --server " + Quote(server)
            + " --room-id desktop_runtime"
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
            if (key == "quickNextHost") return "下一步：点击“启动联机”，然后进游戏创建 LAN 房间。";
            if (key == "quickJoinedNext") return "已读取邀请并加入房间。下一步：点击“启动联机”，然后进游戏找 LAN 房间。";
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
            if (key == "gamePlan") return "游戏计划";
            if (key == "gameProfileList") return "模板列表";
            if (key == "gameProfilePlan") return "模板游戏计划";
            if (key == "gamePortScan") return "游戏端口扫描";
            if (key == "gameReadiness") return "游戏就绪";
            if (key == "gameReadinessCheck") return "游戏就绪检查";
            if (key == "firewallPlan") return "防火墙计划";
            if (key == "firewallDiagnose") return "防火墙诊断";
            if (key == "firewallScan") return "扫描防火墙";
            if (key == "generalDiagnose") return "通用诊断";
            if (key == "networkDiagnose") return "网络诊断";
            if (key == "exportDiagnostics") return "导出诊断";
            if (key == "udpTest") return "UDP 测试";
            if (key == "broadcastTest") return "广播测试";
            if (key == "nativeRuntimeSelfTest") return "原生隧道自检";
            if (key == "wintunDetect") return "Wintun 检测";
            if (key == "wintunProbe") return "Wintun 探针";
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
            if (key == "stunServer") return "STUN 服务";
            if (key == "remotePeer") return "远端 Peer";
            if (key == "nativeOfferPath") return "Offer 文件: ";
            if (key == "runtimePeers") return "peer";
            if (key == "runtimeHeartbeats") return "心跳";
            if (key == "runtimeSnapshots") return "snapshot";
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
            if (key == "coordinationPeerRequired") return "需要先填写远端 Peer。";
            if (key == "coordinationPeerKicked") return "已请求踢出远端 Peer:";
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
            if (key == "quickNextHost") return "Next: click Start LAN, then create a LAN room in the game.";
            if (key == "quickJoinedNext") return "Invite decoded and room joined. Next: click Start LAN, then find the LAN room in the game.";
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
            if (key == "gamePlan") return "Game plan";
            if (key == "gameProfileList") return "Profile list";
            if (key == "gameProfilePlan") return "Profile game plan";
            if (key == "gamePortScan") return "Game port scan";
            if (key == "gameReadiness") return "game readiness";
            if (key == "gameReadinessCheck") return "Game readiness";
            if (key == "firewallPlan") return "Firewall plan";
            if (key == "firewallDiagnose") return "Firewall diagnose";
            if (key == "firewallScan") return "Firewall scan";
            if (key == "generalDiagnose") return "General diagnose";
            if (key == "networkDiagnose") return "Network diagnose";
            if (key == "exportDiagnostics") return "Export diagnostics";
            if (key == "udpTest") return "UDP test";
            if (key == "broadcastTest") return "Broadcast test";
            if (key == "nativeRuntimeSelfTest") return "Native tunnel self-test";
            if (key == "wintunDetect") return "Wintun detect";
            if (key == "wintunProbe") return "Wintun probe";
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
            if (key == "stunServer") return "STUN server";
            if (key == "remotePeer") return "Remote peer";
            if (key == "nativeOfferPath") return "Offer file: ";
            if (key == "runtimePeers") return "peers";
            if (key == "runtimeHeartbeats") return "heartbeats";
            if (key == "runtimeSnapshots") return "snapshots";
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
            if (key == "coordinationPeerRequired") return "Fill the remote peer first.";
            if (key == "coordinationPeerKicked") return "Requested remote peer kick:";
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

    static class Native
    {
        [DllImport("shcore.dll")]
        public static extern int SetProcessDpiAwareness(int awareness);

        [DllImport("user32.dll")]
        public static extern bool SetProcessDPIAware();

        [DllImport("user32.dll")]
        public static extern bool ReleaseCapture();

        [DllImport("user32.dll")]
        public static extern IntPtr SendMessage(IntPtr hWnd, int msg, IntPtr wParam, IntPtr lParam);
    }
}
