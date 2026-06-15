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
    protected override CreateParams CreateParams
    {
        get
        {
            const int wsCaption = 0x00C00000;
            const int wsThickFrame = 0x00040000;
            const int wsMinimizeBox = 0x00020000;
            const int wsMaximizeBox = 0x00010000;
            const int wsSysMenu = 0x00080000;
            CreateParams cp = base.CreateParams;
            cp.Style &= ~(wsCaption | wsMinimizeBox | wsMaximizeBox | wsSysMenu);
            cp.Style |= wsThickFrame;
            return cp;
        }
    }

    protected override void WndProc(ref Message m)
    {
        const int wmNcCalcSize = 0x83;
        const int wmNcPaint = 0x85;
        const int wmNcActivate = 0x86;

        if (m.Msg == wmNcCalcSize && m.WParam != IntPtr.Zero)
        {
            m.Result = IntPtr.Zero;
            return;
        }

        if (m.Msg == wmNcPaint)
        {
            m.Result = IntPtr.Zero;
            return;
        }

        if (m.Msg == wmNcActivate)
        {
            m.Result = new IntPtr(1);
            return;
        }

        if (m.Msg == WmNcHitTest && WindowState != FormWindowState.Maximized)
        {
            Point cursor = PointToClient(new Point(SignedLowWord(m.LParam), SignedHighWord(m.LParam)));
            int hit = ResizeHitTest(cursor);
            if (hit != HtClient)
            {
                m.Result = new IntPtr(hit);
                return;
            }
        }

        base.WndProc(ref m);
    }

    public bool PreFilterMessage(ref Message m)
    {
        if (WindowState == FormWindowState.Maximized)
        {
            ClearResizeCursor();
            return false;
        }
        if (m.Msg != WmMouseMove && m.Msg != WmLeftButtonDown) return false;

        Control source = Control.FromHandle(m.HWnd);
        if (!IsControlInThisWindow(source))
        {
            ClearResizeCursor();
            return false;
        }

        Point sourcePoint = new Point(SignedLowWord(m.LParam), SignedHighWord(m.LParam));
        Point clientPoint = PointToClient(source.PointToScreen(sourcePoint));
        int hit = ResizeHitTest(clientPoint);
        if (hit == HtClient)
        {
            ClearResizeCursor();
            return false;
        }

        if (m.Msg == WmMouseMove)
        {
            SetResizeCursor(source, ResizeCursor(hit));
            return false;
        }

        Native.ReleaseCapture();
        Native.SendMessage(Handle, WmNcLeftButtonDown, new IntPtr(hit), IntPtr.Zero);
        return true;
    }

    bool IsControlInThisWindow(Control control)
    {
        while (control != null)
        {
            if (control == this) return true;
            control = control.Parent;
        }
        return false;
    }

    int ResizeHitTest(Point cursor)
    {
        bool inside = cursor.X >= 0 && cursor.Y >= 0 && cursor.X <= ClientSize.Width && cursor.Y <= ClientSize.Height;
        if (!inside) return HtClient;

        bool left = cursor.X <= ResizeGripSize;
        bool right = cursor.X >= ClientSize.Width - ResizeGripSize;
        bool top = cursor.Y <= ResizeGripSize;
        bool bottom = cursor.Y >= ClientSize.Height - ResizeGripSize;

        if (left && top) return HtTopLeft;
        if (right && top) return HtTopRight;
        if (left && bottom) return HtBottomLeft;
        if (right && bottom) return HtBottomRight;
        if (left) return HtLeft;
        if (right) return HtRight;
        if (top) return HtTop;
        if (bottom) return HtBottom;
        return HtClient;
    }

    Cursor ResizeCursor(int hit)
    {
        if (hit == HtLeft || hit == HtRight) return Cursors.SizeWE;
        if (hit == HtTop || hit == HtBottom) return Cursors.SizeNS;
        if (hit == HtTopLeft || hit == HtBottomRight) return Cursors.SizeNWSE;
        if (hit == HtTopRight || hit == HtBottomLeft) return Cursors.SizeNESW;
        return Cursors.Default;
    }

    void SetResizeCursor(Control control, Cursor cursor)
    {
        if (control == null || cursor == null) return;
        if (resizeCursorControl == control && activeResizeCursor == cursor) return;
        ClearResizeCursor();
        resizeCursorControl = control;
        resizeCursorOriginal = control.Cursor;
        activeResizeCursor = cursor;
        control.Cursor = cursor;
    }

    void ClearResizeCursor()
    {
        if (resizeCursorControl != null && !resizeCursorControl.IsDisposed)
        {
            resizeCursorControl.Cursor = resizeCursorOriginal ?? Cursors.Default;
        }
        resizeCursorControl = null;
        resizeCursorOriginal = null;
        activeResizeCursor = null;
    }

    static int SignedLowWord(IntPtr value)
    {
        return (short)((long)value & 0xFFFF);
    }

    static int SignedHighWord(IntPtr value)
    {
        return (short)(((long)value >> 16) & 0xFFFF);
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
        Native.SendMessage(Handle, WmNcLeftButtonDown, new IntPtr(HtCaption), IntPtr.Zero);
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
}

