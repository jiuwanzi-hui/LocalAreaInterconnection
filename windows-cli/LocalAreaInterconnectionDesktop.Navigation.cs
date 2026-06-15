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
        using (Font titleFont = new Font(Font.FontFamily, 10f, FontStyle.Bold))
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
        button.Font = new Font(Font.FontFamily, 9f, FontStyle.Regular);
        button.TextAlign = ContentAlignment.MiddleLeft;
        button.Padding = new Padding(16, 0, 0, 0);
        button.TabStop = false;
        button.UseVisualStyleBackColor = false;
        button.FlatAppearance.MouseOverBackColor = Color.FromArgb(27, 82, 91);
        button.FlatAppearance.MouseDownBackColor = Color.FromArgb(20, 66, 75);
        button.Tag = page;
        button.Paint += delegate(object sender, PaintEventArgs e) { PaintNavButton((Button)sender, e); };
        button.Click += delegate { SelectPage(page); };
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
}

