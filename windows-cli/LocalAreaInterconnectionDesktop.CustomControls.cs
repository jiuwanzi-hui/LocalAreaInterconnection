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
    class ThemedOutputBox : UserControl
    {
        const int ScrollBarWidth = 10;
        const int TextInsetX = 8;
        const int TextInsetY = 7;
        const int ScrollGap = 6;
        const int EmGetFirstVisibleLine = 0x00CE;
        const int EmLineScroll = 0x00B6;
        readonly RichTextBox textBox;
        readonly Panel scrollBar;
        readonly Panel scrollThumb;
        bool draggingThumb = false;
        int dragStartY = 0;
        int dragStartFirstLine = 0;

        public ThemedOutputBox()
        {
            SetStyle(ControlStyles.ResizeRedraw, true);
            BackColor = FieldDark;
            ForeColor = TextBright;
            TabStop = true;

            textBox = new RichTextBox();
            textBox.BorderStyle = BorderStyle.None;
            textBox.BackColor = FieldDark;
            textBox.ForeColor = TextBright;
            textBox.ReadOnly = true;
            textBox.ScrollBars = RichTextBoxScrollBars.None;
            textBox.WordWrap = true;
            textBox.ShortcutsEnabled = true;
            textBox.HideSelection = false;
            textBox.DetectUrls = false;
            textBox.Margin = new Padding(0);
            textBox.VScroll += delegate { UpdateScrollBar(); };
            textBox.MouseEnter += delegate { FocusTextBoxForWheel(); };
            textBox.MouseWheel += OutputMouseWheel;
            textBox.TextChanged += delegate
            {
                ScrollToFirstLine(0);
                UpdateScrollBar();
            };

            scrollBar = new Panel();
            scrollBar.BackColor = FieldDark;
            scrollBar.Paint += PaintScrollBar;
            scrollBar.MouseEnter += delegate { FocusTextBoxForWheel(); };
            scrollBar.MouseWheel += OutputMouseWheel;
            scrollBar.MouseDown += ScrollBarMouseDown;

            scrollThumb = new Panel();
            scrollThumb.BackColor = AccentCyan;
            scrollThumb.Cursor = Cursors.Hand;
            scrollThumb.Resize += delegate { ApplyThumbRegion(); };
            scrollThumb.MouseEnter += delegate { FocusTextBoxForWheel(); };
            scrollThumb.MouseWheel += OutputMouseWheel;
            scrollThumb.MouseDown += ThumbMouseDown;
            scrollThumb.MouseMove += ThumbMouseMove;
            scrollThumb.MouseUp += ThumbMouseUp;
            scrollBar.Controls.Add(scrollThumb);

            Controls.Add(textBox);
            Controls.Add(scrollBar);
            MouseEnter += delegate { FocusTextBoxForWheel(); };
            MouseWheel += OutputMouseWheel;
            LayoutChildren();
        }

        public override string Text
        {
            get { return textBox.Text; }
            set { textBox.Text = value ?? ""; }
        }

        public void FocusTextBoxForWheel()
        {
            try
            {
                if (textBox != null && textBox.CanFocus) textBox.Focus();
            }
            catch
            {
            }
        }

        public void ScrollWheelDelta(int delta)
        {
            int direction = delta > 0 ? -1 : 1;
            ScrollToFirstLine(FirstVisibleLine() + direction * 4);
        }

        protected override void OnFontChanged(EventArgs e)
        {
            if (textBox != null) textBox.Font = Font;
            UpdateScrollBar();
            base.OnFontChanged(e);
        }

        protected override void OnForeColorChanged(EventArgs e)
        {
            if (textBox != null) textBox.ForeColor = ForeColor;
            base.OnForeColorChanged(e);
        }

        protected override void OnBackColorChanged(EventArgs e)
        {
            if (textBox != null) textBox.BackColor = BackColor;
            if (scrollBar != null) scrollBar.BackColor = BackColor;
            base.OnBackColorChanged(e);
        }

        protected override void OnSizeChanged(EventArgs e)
        {
            LayoutChildren();
            UpdateScrollBar();
            base.OnSizeChanged(e);
        }

        void LayoutChildren()
        {
            if (textBox == null || scrollBar == null) return;
            int barLeft = Math.Max(0, ClientSize.Width - ScrollBarWidth - 4);
            int textWidth = Math.Max(1, barLeft - ScrollGap - TextInsetX);
            int textHeight = Math.Max(1, ClientSize.Height - TextInsetY * 2);
            textBox.SetBounds(TextInsetX, TextInsetY, textWidth, textHeight);
            scrollBar.SetBounds(barLeft, TextInsetY, ScrollBarWidth, textHeight);
            scrollBar.BringToFront();
        }

        int FirstVisibleLine()
        {
            if (!textBox.IsHandleCreated) return 0;
            return Native.SendMessage(textBox.Handle, EmGetFirstVisibleLine, IntPtr.Zero, IntPtr.Zero).ToInt32();
        }

        int TotalLineCount()
        {
            if (textBox.TextLength == 0) return 1;
            return textBox.GetLineFromCharIndex(Math.Max(0, textBox.TextLength - 1)) + 1;
        }

        int VisibleLineCount()
        {
            int lineHeight = Math.Max(1, TextRenderer.MeasureText("Ag", textBox.Font).Height);
            return Math.Max(1, textBox.ClientSize.Height / lineHeight);
        }

        int MaxFirstVisibleLine()
        {
            return Math.Max(0, TotalLineCount() - VisibleLineCount());
        }

        void ScrollToFirstLine(int target)
        {
            if (!textBox.IsHandleCreated) return;
            int clamped = Math.Max(0, Math.Min(MaxFirstVisibleLine(), target));
            int delta = clamped - FirstVisibleLine();
            if (delta != 0)
            {
                Native.SendMessage(textBox.Handle, EmLineScroll, IntPtr.Zero, new IntPtr(delta));
            }
            UpdateScrollBar();
        }

        void UpdateScrollBar()
        {
            if (scrollBar == null || scrollThumb == null || textBox == null) return;
            int max = MaxFirstVisibleLine();
            scrollBar.Visible = max > 0;
            if (!scrollBar.Visible) return;

            int trackHeight = Math.Max(1, scrollBar.ClientSize.Height - 4);
            int thumbHeight = Math.Max(28, VisibleLineCount() * trackHeight / Math.Max(TotalLineCount(), 1));
            int travel = Math.Max(1, trackHeight - thumbHeight);
            int thumbTop = 2 + FirstVisibleLine() * travel / max;
            scrollThumb.SetBounds(2, thumbTop, Math.Max(4, scrollBar.Width - 4), thumbHeight);
            ApplyThumbRegion();
            scrollBar.Invalidate();
        }

        void PaintScrollBar(object sender, PaintEventArgs e)
        {
            e.Graphics.Clear(scrollBar.BackColor);
            if (!scrollBar.Visible) return;
            using (Pen pen = new Pen(Color.FromArgb(58, 72, 78)))
            {
                e.Graphics.DrawLine(pen, scrollBar.Width / 2, 4, scrollBar.Width / 2, scrollBar.Height - 4);
            }
        }

        void OutputMouseWheel(object sender, MouseEventArgs e)
        {
            ScrollWheelDelta(e.Delta);
        }

        void ScrollBarMouseDown(object sender, MouseEventArgs e)
        {
            if (e.Button != MouseButtons.Left) return;
            int page = Math.Max(1, VisibleLineCount() - 1);
            ScrollToFirstLine(FirstVisibleLine() + (e.Y < scrollThumb.Top ? -page : page));
        }

        void ThumbMouseDown(object sender, MouseEventArgs e)
        {
            if (e.Button != MouseButtons.Left) return;
            draggingThumb = true;
            dragStartY = scrollBar.PointToClient(scrollThumb.PointToScreen(e.Location)).Y;
            dragStartFirstLine = FirstVisibleLine();
            scrollThumb.Capture = true;
        }

        void ThumbMouseMove(object sender, MouseEventArgs e)
        {
            if (!draggingThumb) return;
            int currentY = scrollBar.PointToClient(scrollThumb.PointToScreen(e.Location)).Y;
            int max = MaxFirstVisibleLine();
            int track = Math.Max(1, scrollBar.ClientSize.Height - scrollThumb.Height - 4);
            int delta = (currentY - dragStartY) * max / track;
            ScrollToFirstLine(dragStartFirstLine + delta);
        }

        void ThumbMouseUp(object sender, MouseEventArgs e)
        {
            draggingThumb = false;
            scrollThumb.Capture = false;
        }

        void ApplyThumbRegion()
        {
            if (scrollThumb.Width <= 0 || scrollThumb.Height <= 0) return;
            if (scrollThumb.Region != null) scrollThumb.Region.Dispose();
            using (GraphicsPath path = OutputRoundRect(new Rectangle(0, 0, scrollThumb.Width, scrollThumb.Height), 4))
            {
                scrollThumb.Region = new Region(path);
            }
        }

        static GraphicsPath OutputRoundRect(Rectangle bounds, int radius)
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
    }

    class ClearTextButton : Button
    {
        bool hovering;
        bool pressing;

        public int Radius = 8;
        public Color NormalBack = AccentCyan;
        public Color HoverBack = AccentCyanHover;
        public Color DownBack = AccentCyanDown;
        public Color Border = AccentCyan;
        public Color TextNormal = Color.FromArgb(8, 30, 34);
        public Color TextDisabled = Color.FromArgb(118, 128, 136);

        public ClearTextButton()
        {
            SetStyle(ControlStyles.UserPaint | ControlStyles.AllPaintingInWmPaint | ControlStyles.OptimizedDoubleBuffer | ControlStyles.ResizeRedraw, true);
            FlatStyle = FlatStyle.Flat;
            FlatAppearance.BorderSize = 0;
            UseVisualStyleBackColor = false;
            TabStop = false;
        }

        protected override void OnMouseEnter(EventArgs e)
        {
            hovering = true;
            Invalidate();
            base.OnMouseEnter(e);
        }

        protected override void OnMouseLeave(EventArgs e)
        {
            hovering = false;
            pressing = false;
            Invalidate();
            base.OnMouseLeave(e);
        }

        protected override void OnMouseDown(MouseEventArgs e)
        {
            if (e.Button == MouseButtons.Left)
            {
                pressing = true;
                Invalidate();
            }
            base.OnMouseDown(e);
        }

        protected override void OnMouseUp(MouseEventArgs e)
        {
            pressing = false;
            Invalidate();
            base.OnMouseUp(e);
        }

        protected override void OnEnabledChanged(EventArgs e)
        {
            Invalidate();
            base.OnEnabledChanged(e);
        }

        protected override void OnPaint(PaintEventArgs e)
        {
            e.Graphics.SmoothingMode = SmoothingMode.AntiAlias;
            e.Graphics.PixelOffsetMode = PixelOffsetMode.HighQuality;
            Color clear = Parent == null ? Color.FromArgb(20, 22, 26) : Parent.BackColor;
            if (clear == Color.Transparent && Parent != null && Parent.Parent != null)
            {
                clear = Parent.Parent.BackColor;
            }
            if (clear == Color.Transparent)
            {
                clear = Color.FromArgb(20, 22, 26);
            }
            using (SolidBrush clearBrush = new SolidBrush(clear))
            {
                e.Graphics.FillRectangle(clearBrush, ClientRectangle);
            }

            Rectangle bounds = new Rectangle(1, 1, Math.Max(1, Width - 3), Math.Max(1, Height - 3));
            Color fill = !Enabled ? Color.FromArgb(28, 32, 38) : pressing ? DownBack : hovering ? HoverBack : NormalBack;
            using (GraphicsPath path = ButtonRoundRect(bounds, Radius))
            using (SolidBrush brush = new SolidBrush(fill))
            {
                e.Graphics.FillPath(brush, path);
                using (Pen pen = new Pen(!Enabled ? Color.FromArgb(52, 58, 66) : Border))
                {
                    e.Graphics.DrawPath(pen, path);
                }
            }

            TextRenderer.DrawText(
                e.Graphics,
                Text,
                Font,
                ClientRectangle,
                Enabled ? TextNormal : TextDisabled,
                TextFormatFlags.HorizontalCenter | TextFormatFlags.VerticalCenter | TextFormatFlags.EndEllipsis | TextFormatFlags.NoPrefix);
        }

        static GraphicsPath ButtonRoundRect(Rectangle bounds, int radius)
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
    }
}

