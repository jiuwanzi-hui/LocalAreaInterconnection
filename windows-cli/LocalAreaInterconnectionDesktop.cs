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

public partial class LocalAreaInterconnectionDesktop : Form, IMessageFilter
{
    static readonly Color ShellDark = Color.FromArgb(20, 22, 26);
    static readonly Color TitleDark = Color.FromArgb(16, 18, 22);
    static readonly Color SidebarDark = Color.FromArgb(15, 69, 78);
    static readonly Color SidebarMid = Color.FromArgb(30, 76, 86);
    static readonly Color SidebarDeep = Color.FromArgb(40, 65, 86);
    static readonly Color CardDark = Color.FromArgb(34, 39, 47);
    static readonly Color CardBorder = Color.FromArgb(92, 110, 122);
    static readonly Color CardHighlight = Color.FromArgb(142, 162, 172);
    static readonly Color CardShadow = Color.FromArgb(8, 12, 16);
    static readonly Color FieldDark = Color.FromArgb(22, 27, 34);
    static readonly Color FieldBorder = Color.FromArgb(78, 94, 104);
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
    TextBox relayServer;
    TextBox stunServer;
    TextBox upnpPortMap;
    TextBox remotePeer;
    TextBox invite;
    ThemedOutputBox output;
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
    Label heartbeatPulseLabel;
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
    Panel homeFieldViewport;
    TableLayoutPanel homeFieldTable;
    Panel homeFieldScrollBar;
    Panel homeFieldScrollThumb;
    Button moreToolsButton;
    bool advancedActionsVisible = false;
    int actionScrollOffset = 0;
    bool draggingActionScrollThumb = false;
    int actionScrollDragStartY = 0;
    int actionScrollStartOffset = 0;
    int homeFieldScrollOffset = 0;
    bool draggingHomeFieldScrollThumb = false;
    int homeFieldScrollDragStartY = 0;
    int homeFieldScrollStartOffset = 0;
    Control resizeCursorControl;
    Cursor resizeCursorOriginal;
    Cursor activeResizeCursor;
    bool userActionRunning = false;
    List<Button> advancedActionButtons = new List<Button>();
    Dictionary<string, Label> labelControls = new Dictionary<string, Label>();
    Dictionary<string, Button> buttonControls = new Dictionary<string, Button>();
    Dictionary<string, Button> homeButtonControls = new Dictionary<string, Button>();
    Dictionary<string, List<TextBox>> fieldTextBoxes = new Dictionary<string, List<TextBox>>();
    bool syncingFieldTextBoxes = false;
    Process runtimeProcess;
    Process coordinationProcess;
    readonly object backgroundProcessLock = new object();
    List<Process> backgroundProcesses = new List<Process>();
    StringBuilder runtimeOutput = new StringBuilder();
    StringBuilder coordinationOutput = new StringBuilder();
    string runtimeStopFile = "";
    string latestRuntimeSnapshot = "";
    string latestRuntimeObservationFile = "";
    string latestNativeOfferFile = "";
    string latestNativeOfferBind = "";
    string coordinationStoreFile = "";
    string localRuntimePeerId = "";
    string hostRuntimePeerId = "";
    bool restartingRuntimeForRemotePeer = false;
    bool coordinationPresenceRefreshRunning = false;
    bool coordinationRoomRefreshRunning = false;
    DateTime lastRuntimeP2pRetryUtc = DateTime.MinValue;
    string lastRuntimeP2pRetrySpec = "";
    string lastRuntimeP2pRetrySignature = "";
    string lastRemotePeerOfferSignature = "";
    string lastRuntimeSnapshotText = "";
    string latestCoordinationViewText = "";
    int latestCoordinationOnlineCount = 0;
    int latestCoordinationMemberCount = 0;
    bool heartbeatPulseActive = false;
    int heartbeatPulsePhase = 0;
    int lastRuntimeLogLength = 0;
    string roomUiMode = "none";
    DateTime lastCoordinationRefreshUtc = DateTime.MinValue;
    const int ResizeGripSize = 8;
    const int WmMouseMove = 0x0200;
    const int WmLeftButtonDown = 0x0201;
    const int WmNcLeftButtonDown = 0x00A1;
    const int WmNcHitTest = 0x0084;
    const int HtClient = 1;
    const int HtCaption = 2;
    const int HtLeft = 10;
    const int HtRight = 11;
    const int HtTop = 12;
    const int HtTopLeft = 13;
    const int HtTopRight = 14;
    const int HtBottom = 15;
    const int HtBottomLeft = 16;
    const int HtBottomRight = 17;

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
        ControlBox = false;
        MinimizeBox = false;
        MaximizeBox = false;
        ShowIcon = true;
        ShowInTaskbar = true;
        MinimumSize = new Size(980, 640);
        DoubleBuffered = true;
        SetStyle(ControlStyles.AllPaintingInWmPaint | ControlStyles.UserPaint | ControlStyles.OptimizedDoubleBuffer, true);
        BackColor = ShellDark;
        Font = new Font("Microsoft YaHei UI", 9f, FontStyle.Regular);
        Icon = Icon.ExtractAssociatedIcon(Application.ExecutablePath);
        language = LoadLanguage();
        chromeTips = new ToolTip();
        chromeTips.BackColor = CardDark;
        chromeTips.ForeColor = TextBright;
        Application.AddMessageFilter(this);
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
            ShutdownChildProcesses();
        };
        FormClosed += delegate
        {
            Application.RemoveMessageFilter(this);
            if (activeWindow == this) activeWindow = null;
        };
    }

}
