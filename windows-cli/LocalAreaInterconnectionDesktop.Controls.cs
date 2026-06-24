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
        label.Font = new Font(Font.FontFamily, 9f, FontStyle.Regular);
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
        Button button = new ClearTextButton();
        button.Text = T(key);
        button.Width = Math.Min(184, Math.Max(116, TextRenderer.MeasureText(button.Text, Font).Width + 24));
        button.Height = advanced ? 28 : 32;
        button.Margin = new Padding(0, 0, 8, 8);
        button.Font = new Font(Font, advanced ? FontStyle.Regular : FontStyle.Bold);
        StyleClearButton((ClearTextButton)button, advanced);
        button.Click += delegate(object sender, EventArgs e)
        {
            RunUserAction(key, handler, sender, e);
        };
        button.MouseWheel += ScrollActionsWheel;
        buttonControls[key] = button;
        if (advanced)
        {
            advancedActionButtons.Add(button);
            button.Visible = advancedActionsVisible;
        }
        panel.Controls.Add(button);
        return button;
    }

    void StyleClearButton(ClearTextButton button, bool secondary)
    {
        button.FlatStyle = FlatStyle.Flat;
        button.FlatAppearance.BorderSize = 0;
        button.UseVisualStyleBackColor = false;
        button.TabStop = false;
        button.Radius = 8;
        button.NormalBack = secondary ? Color.FromArgb(34, 38, 44) : AccentCyan;
        button.HoverBack = secondary ? Color.FromArgb(44, 52, 60) : AccentCyanHover;
        button.DownBack = secondary ? Color.FromArgb(26, 30, 36) : AccentCyanDown;
        button.Border = secondary ? Color.FromArgb(62, 102, 110) : AccentCyan;
        button.TextNormal = secondary ? Color.FromArgb(224, 224, 224) : Color.FromArgb(8, 30, 34);
        button.TextDisabled = Color.FromArgb(118, 128, 136);
        button.BackColor = button.NormalBack;
        button.ForeColor = button.TextNormal;
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
        if (key == "startLanSession" || key == "startRuntime" || key == "nativeOffer" || key == "copyDirectCode")
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

    void UpdateHomeActionButtons()
    {
        SetHomeButtonVisible("quickHostRoom", roomUiMode == "none" || roomUiMode == "host");
        SetHomeButtonVisible("quickJoinRoom", roomUiMode == "none");
        SetHomeButtonVisible("copyInvite", roomUiMode == "host");
        SetHomeButtonVisible("copyDirectCode", roomUiMode == "host" || roomUiMode == "joined" || roomUiMode == "running");
        SetHomeButtonVisible("startLanSession", roomUiMode == "host" || roomUiMode == "joined" || roomUiMode == "running");
        SetHomeButtonVisible("checkConnection", roomUiMode == "host" || roomUiMode == "joined" || roomUiMode == "running");
        SetHomeButtonVisible("moreTools", true);
    }

    void SetHomeButtonVisible(string key, bool visible)
    {
        if (!homeButtonControls.ContainsKey(key)) return;
        homeButtonControls[key].Visible = visible;
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

    void AdjustHomeFieldLayout()
    {
        if (homeFieldViewport == null || homeFieldTable == null) return;
        int width = Math.Max(220, homeFieldViewport.ClientSize.Width);
        int contentHeight = Math.Max(1, 14 + 34 * Math.Max(1, homeFieldTable.RowCount) + 14);
        homeFieldTable.SetBounds(0, -homeFieldScrollOffset, width, contentHeight);
        ClampHomeFieldScroll();
        UpdateHomeFieldScrollBar();
    }

    void ScrollHomeFieldsWheel(object sender, MouseEventArgs e)
    {
        int step = e.Delta > 0 ? -34 : 34;
        SetHomeFieldScrollOffset(homeFieldScrollOffset + step);
    }

    void BeginHomeFieldScrollThumbDrag(object sender, MouseEventArgs e)
    {
        if (e.Button != MouseButtons.Left) return;
        draggingHomeFieldScrollThumb = true;
        homeFieldScrollDragStartY = homeFieldScrollBar.PointToClient(homeFieldScrollThumb.PointToScreen(e.Location)).Y;
        homeFieldScrollStartOffset = homeFieldScrollOffset;
    }

    void DragHomeFieldScrollThumb(object sender, MouseEventArgs e)
    {
        if (!draggingHomeFieldScrollThumb) return;
        int currentY = homeFieldScrollBar.PointToClient(homeFieldScrollThumb.PointToScreen(e.Location)).Y;
        int track = Math.Max(1, homeFieldScrollBar.ClientSize.Height - homeFieldScrollThumb.Height - 4);
        int maxOffset = MaxHomeFieldScrollOffset();
        int deltaOffset = (currentY - homeFieldScrollDragStartY) * maxOffset / track;
        SetHomeFieldScrollOffset(homeFieldScrollStartOffset + deltaOffset);
    }

    void EndHomeFieldScrollThumbDrag(object sender, MouseEventArgs e)
    {
        draggingHomeFieldScrollThumb = false;
    }

    void SetHomeFieldScrollOffset(int value)
    {
        homeFieldScrollOffset = Math.Max(0, Math.Min(MaxHomeFieldScrollOffset(), value));
        if (homeFieldTable != null)
        {
            homeFieldTable.Top = -homeFieldScrollOffset;
        }
        UpdateHomeFieldScrollBar();
    }

    void ClampHomeFieldScroll()
    {
        homeFieldScrollOffset = Math.Max(0, Math.Min(MaxHomeFieldScrollOffset(), homeFieldScrollOffset));
        if (homeFieldTable != null)
        {
            homeFieldTable.Top = -homeFieldScrollOffset;
        }
    }

    int MaxHomeFieldScrollOffset()
    {
        if (homeFieldTable == null || homeFieldViewport == null) return 0;
        return Math.Max(0, homeFieldTable.Height - homeFieldViewport.ClientSize.Height);
    }

    void UpdateHomeFieldScrollBar()
    {
        if (homeFieldScrollBar == null || homeFieldScrollThumb == null || homeFieldTable == null || homeFieldViewport == null) return;
        int maxOffset = MaxHomeFieldScrollOffset();
        bool visible = maxOffset > 0;
        homeFieldScrollBar.Visible = visible;
        if (!visible) return;

        int trackHeight = Math.Max(1, homeFieldScrollBar.ClientSize.Height - 4);
        int thumbHeight = Math.Max(28, homeFieldViewport.ClientSize.Height * trackHeight / Math.Max(homeFieldTable.Height, 1));
        int travel = Math.Max(1, trackHeight - thumbHeight);
        int thumbTop = 2 + (homeFieldScrollOffset * travel / maxOffset);
        homeFieldScrollThumb.SetBounds(2, thumbTop, Math.Max(4, homeFieldScrollBar.Width - 4), thumbHeight);
        ApplyRoundedRegion(homeFieldScrollThumb, 4);
        homeFieldScrollBar.Invalidate();
    }

    void PaintHomeFieldScrollBar(object sender, PaintEventArgs e)
    {
        e.Graphics.Clear(homeFieldScrollBar.BackColor);
        using (Pen pen = new Pen(Color.FromArgb(76, 76, 76)))
        {
            e.Graphics.DrawLine(pen, homeFieldScrollBar.Width / 2, 4, homeFieldScrollBar.Width / 2, homeFieldScrollBar.Height - 4);
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
        return Framed(control, FieldBorder);
    }

    Panel Framed(Control control, Color border)
    {
        Panel panel = new Panel();
        panel.Dock = DockStyle.Fill;
        panel.BackColor = border;
        panel.Padding = new Padding(3);
        panel.Margin = new Padding(0, 3, 10, 3);
        panel.Resize += delegate { ApplyRoundedRegion(panel, 10); };
        panel.Paint += PaintInsetFrame;
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

    void PaintRaisedCardFrame(object sender, PaintEventArgs e)
    {
        Control control = (Control)sender;
        if (control.Width <= 2 || control.Height <= 2) return;

        UseCrispGraphics(e.Graphics);
        Rectangle bounds = new Rectangle(0, 0, control.Width - 1, control.Height - 1);
        using (GraphicsPath path = RoundedRectPath(bounds, 12))
        using (Pen shadow = new Pen(Color.FromArgb(0, 0, 0), 2f))
        using (Pen border = new Pen(CardBorder, 1.6f))
        using (Pen highlight = new Pen(CardHighlight, 1.2f))
        using (Pen lowlight = new Pen(Color.FromArgb(14, 20, 26), 1.2f))
        {
            e.Graphics.DrawPath(shadow, path);
            e.Graphics.DrawPath(border, path);
            e.Graphics.DrawLine(highlight, 11, 1, Math.Max(11, control.Width - 13), 1);
            e.Graphics.DrawLine(highlight, 1, 11, 1, Math.Max(11, control.Height - 13));
            e.Graphics.DrawLine(lowlight, 12, control.Height - 2, Math.Max(12, control.Width - 14), control.Height - 2);
            e.Graphics.DrawLine(lowlight, control.Width - 2, 12, control.Width - 2, Math.Max(12, control.Height - 14));
        }
    }

    void PaintInsetFrame(object sender, PaintEventArgs e)
    {
        Control control = (Control)sender;
        if (control.Width <= 2 || control.Height <= 2) return;

        UseCrispGraphics(e.Graphics);
        Rectangle bounds = new Rectangle(0, 0, control.Width - 1, control.Height - 1);
        using (GraphicsPath path = RoundedRectPath(bounds, 10))
        using (Pen border = new Pen(FieldBorder, 1.5f))
        using (Pen top = new Pen(Color.FromArgb(122, 142, 152), 1f))
        using (Pen bottom = new Pen(Color.FromArgb(4, 8, 12), 1.2f))
        {
            e.Graphics.DrawPath(border, path);
            e.Graphics.DrawLine(top, 9, 1, Math.Max(9, control.Width - 11), 1);
            e.Graphics.DrawLine(bottom, 9, control.Height - 2, Math.Max(9, control.Width - 11), control.Height - 2);
        }
    }

    Panel RoomDetailsPanel()
    {
        Panel outer = new Panel();
        outer.Dock = DockStyle.Fill;
        outer.BackColor = CardShadow;
        outer.Padding = new Padding(3, 3, 5, 5);
        outer.Margin = new Padding(14, 0, 0, 10);
        outer.Resize += delegate { ApplyRoundedRegion(outer, 12); };
        outer.Paint += PaintRaisedCardFrame;

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

        FlowLayoutPanel headerLine = new FlowLayoutPanel();
        headerLine.Dock = DockStyle.Fill;
        headerLine.BackColor = Color.Transparent;
        headerLine.FlowDirection = FlowDirection.LeftToRight;
        headerLine.WrapContents = false;
        headerLine.Margin = new Padding(0);
        headerLine.Padding = new Padding(0);

        heartbeatPulseLabel = new Label();
        heartbeatPulseLabel.Text = "●";
        heartbeatPulseLabel.Width = 18;
        heartbeatPulseLabel.Height = 28;
        heartbeatPulseLabel.TextAlign = ContentAlignment.MiddleLeft;
        heartbeatPulseLabel.Font = new Font(Font.FontFamily, 13, FontStyle.Bold);
        heartbeatPulseLabel.ForeColor = TextMuted;
        heartbeatPulseLabel.BackColor = Color.Transparent;
        heartbeatPulseLabel.Margin = new Padding(0, 0, 4, 0);
        headerLine.Controls.Add(heartbeatPulseLabel);

        Label header = new Label();
        header.Name = "roomDetailsHeader";
        header.Text = T("roomDetails");
        header.Width = 240;
        header.Height = 28;
        header.TextAlign = ContentAlignment.MiddleLeft;
        header.Font = new Font(Font.FontFamily, 10, FontStyle.Bold);
        header.ForeColor = TextBright;
        header.BackColor = Color.Transparent;
        header.Margin = new Padding(0);
        labelControls["roomDetails"] = header;
        headerLine.Controls.Add(header);
        details.Controls.Add(headerLine, 0, 0);

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
}

