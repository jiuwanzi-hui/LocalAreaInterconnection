using System;
using System.Drawing;
using System.Drawing.Drawing2D;
using System.IO;

public static class GenerateIcon
{
    public static int Main(string[] args)
    {
        string output = args.Length > 0 ? args[0] : "assets/LocalAreaInterconnection.ico";
        string directory = Path.GetDirectoryName(output);
        if (!String.IsNullOrEmpty(directory) && !Directory.Exists(directory))
        {
            Directory.CreateDirectory(directory);
        }

        using (Bitmap bitmap = new Bitmap(64, 64))
        using (Graphics graphics = Graphics.FromImage(bitmap))
        {
            graphics.SmoothingMode = SmoothingMode.AntiAlias;
            graphics.Clear(Color.Transparent);

            using (GraphicsPath circle = new GraphicsPath())
            {
                circle.AddEllipse(4, 4, 56, 56);
                using (PathGradientBrush mist = new PathGradientBrush(circle))
                {
                    mist.CenterColor = Color.FromArgb(255, 116, 207, 255);
                    mist.SurroundColors = new Color[] { Color.FromArgb(255, 12, 42, 74) };
                    graphics.FillPath(mist, circle);
                }
            }

            using (Pen outer = new Pen(Color.FromArgb(220, 176, 235, 255), 2.2f))
            using (Pen inner = new Pen(Color.FromArgb(150, 88, 178, 255), 1.2f))
            {
                graphics.DrawEllipse(outer, 5, 5, 54, 54);
                graphics.DrawEllipse(inner, 12, 12, 40, 40);
            }

            PointF a = new PointF(21, 24);
            PointF b = new PointF(43, 24);
            PointF c = new PointF(32, 42);
            using (Pen link = new Pen(Color.FromArgb(230, 214, 250, 255), 3.0f))
            {
                link.StartCap = LineCap.Round;
                link.EndCap = LineCap.Round;
                graphics.DrawLine(link, a, b);
                graphics.DrawLine(link, a, c);
                graphics.DrawLine(link, b, c);
            }

            DrawNode(graphics, a, 5);
            DrawNode(graphics, b, 5);
            DrawNode(graphics, c, 6);

            using (LinearGradientBrush shine = new LinearGradientBrush(
                new Rectangle(10, 8, 44, 20),
                Color.FromArgb(110, 255, 255, 255),
                Color.FromArgb(0, 255, 255, 255),
                LinearGradientMode.ForwardDiagonal))
            {
                graphics.FillEllipse(shine, 10, 8, 44, 20);
            }

            using (Icon icon = Icon.FromHandle(bitmap.GetHicon()))
            using (FileStream stream = File.Create(output))
            {
                icon.Save(stream);
            }
        }

        return 0;
    }

    static void DrawNode(Graphics graphics, PointF center, int radius)
    {
        RectangleF glow = new RectangleF(center.X - radius - 3, center.Y - radius - 3, (radius + 3) * 2, (radius + 3) * 2);
        using (GraphicsPath path = new GraphicsPath())
        {
            path.AddEllipse(glow);
            using (PathGradientBrush brush = new PathGradientBrush(path))
            {
                brush.CenterColor = Color.FromArgb(220, 220, 250, 255);
                brush.SurroundColors = new Color[] { Color.FromArgb(0, 220, 250, 255) };
                graphics.FillPath(brush, path);
            }
        }

        using (Brush fill = new SolidBrush(Color.FromArgb(245, 232, 252, 255)))
        using (Pen stroke = new Pen(Color.FromArgb(255, 36, 112, 180), 1.4f))
        {
            graphics.FillEllipse(fill, center.X - radius, center.Y - radius, radius * 2, radius * 2);
            graphics.DrawEllipse(stroke, center.X - radius, center.Y - radius, radius * 2, radius * 2);
        }
    }
}
