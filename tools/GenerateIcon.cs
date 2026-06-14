using System;
using System.Collections.Generic;
using System.Drawing;
using System.Drawing.Drawing2D;
using System.Drawing.Imaging;
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

        int[] sizes = new int[] { 16, 24, 32, 48, 64, 128, 256 };
        List<byte[]> images = new List<byte[]>();
        for (int i = 0; i < sizes.Length; i++)
        {
            using (Bitmap bitmap = RenderIcon(sizes[i]))
            using (MemoryStream stream = new MemoryStream())
            {
                bitmap.Save(stream, ImageFormat.Png);
                images.Add(stream.ToArray());
            }
        }

        using (FileStream stream = File.Create(output))
        using (BinaryWriter writer = new BinaryWriter(stream))
        {
            writer.Write((ushort)0);
            writer.Write((ushort)1);
            writer.Write((ushort)sizes.Length);

            int offset = 6 + sizes.Length * 16;
            for (int i = 0; i < sizes.Length; i++)
            {
                int size = sizes[i];
                writer.Write((byte)(size >= 256 ? 0 : size));
                writer.Write((byte)(size >= 256 ? 0 : size));
                writer.Write((byte)0);
                writer.Write((byte)0);
                writer.Write((ushort)1);
                writer.Write((ushort)32);
                writer.Write(images[i].Length);
                writer.Write(offset);
                offset += images[i].Length;
            }

            for (int i = 0; i < images.Count; i++)
            {
                writer.Write(images[i]);
            }
        }

        using (Bitmap preview = RenderIcon(256))
        {
            preview.Save(Path.Combine(directory, "LocalAreaInterconnection.preview.png"), ImageFormat.Png);
        }

        return 0;
    }

    static Bitmap RenderIcon(int size)
    {
        Bitmap bitmap = new Bitmap(size, size, PixelFormat.Format32bppArgb);
        using (Graphics graphics = Graphics.FromImage(bitmap))
        {
            graphics.SmoothingMode = SmoothingMode.AntiAlias;
            graphics.PixelOffsetMode = PixelOffsetMode.HighQuality;
            graphics.CompositingQuality = CompositingQuality.HighQuality;
            graphics.InterpolationMode = InterpolationMode.HighQualityBicubic;
            graphics.Clear(Color.Transparent);

            float s = size / 256f;
            RectangleF tile = new RectangleF(12 * s, 12 * s, 232 * s, 232 * s);
            using (GraphicsPath tilePath = RoundedRect(tile, 48 * s))
            using (LinearGradientBrush bg = new LinearGradientBrush(
                tile,
                Color.FromArgb(255, 15, 69, 78),
                Color.FromArgb(255, 40, 65, 86),
                LinearGradientMode.ForwardDiagonal))
            {
                graphics.FillPath(bg, tilePath);
            }

            RectangleF ring = new RectangleF(47 * s, 47 * s, 162 * s, 162 * s);
            using (Pen glow = new Pen(Color.FromArgb(80, 0, 212, 216), Math.Max(1f, 14 * s)))
            using (Pen cyan = new Pen(Color.FromArgb(255, 0, 212, 216), Math.Max(1f, 8 * s)))
            using (Pen soft = new Pen(Color.FromArgb(160, 180, 245, 248), Math.Max(1f, 3 * s)))
            {
                graphics.DrawEllipse(glow, ring);
                graphics.DrawEllipse(cyan, ring);
                RectangleF innerRing = new RectangleF(69 * s, 69 * s, 118 * s, 118 * s);
                graphics.DrawEllipse(soft, innerRing);
            }

            DrawNodeLink(graphics, s);
            DrawLetters(graphics, size, s);
        }
        return bitmap;
    }

    static void DrawNodeLink(Graphics graphics, float s)
    {
        PointF a = new PointF(82 * s, 103 * s);
        PointF b = new PointF(174 * s, 103 * s);
        PointF c = new PointF(128 * s, 170 * s);
        using (Pen link = new Pen(Color.FromArgb(170, 224, 253, 255), Math.Max(1f, 8 * s)))
        {
            link.StartCap = LineCap.Round;
            link.EndCap = LineCap.Round;
            graphics.DrawLine(link, a, b);
            graphics.DrawLine(link, a, c);
            graphics.DrawLine(link, b, c);
        }
        DrawNode(graphics, a, 18 * s);
        DrawNode(graphics, b, 18 * s);
        DrawNode(graphics, c, 20 * s);
    }

    static void DrawLetters(Graphics graphics, int size, float s)
    {
        if (size < 32) return;
        using (Font font = new Font("Segoe UI", Math.Max(8f, 39 * s), FontStyle.Bold, GraphicsUnit.Pixel))
        using (StringFormat format = new StringFormat())
        using (Brush shadow = new SolidBrush(Color.FromArgb(90, 0, 0, 0)))
        using (Brush text = new SolidBrush(Color.White))
        {
            format.Alignment = StringAlignment.Center;
            format.LineAlignment = StringAlignment.Center;
            RectangleF r = new RectangleF(48 * s, 88 * s, 160 * s, 82 * s);
            graphics.DrawString("LA", font, shadow, new RectangleF(r.X + 2 * s, r.Y + 3 * s, r.Width, r.Height), format);
            graphics.DrawString("LA", font, text, r, format);
        }
    }

    static void DrawNode(Graphics graphics, PointF center, float radius)
    {
        using (Brush fill = new SolidBrush(Color.FromArgb(255, 240, 254, 255)))
        using (Pen stroke = new Pen(Color.FromArgb(255, 0, 146, 160), Math.Max(1f, radius * 0.18f)))
        {
            graphics.FillEllipse(fill, center.X - radius, center.Y - radius, radius * 2, radius * 2);
            graphics.DrawEllipse(stroke, center.X - radius, center.Y - radius, radius * 2, radius * 2);
        }
    }

    static GraphicsPath RoundedRect(RectangleF bounds, float radius)
    {
        float diameter = Math.Max(1f, radius * 2f);
        GraphicsPath path = new GraphicsPath();
        RectangleF arc = new RectangleF(bounds.Left, bounds.Top, diameter, diameter);
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
