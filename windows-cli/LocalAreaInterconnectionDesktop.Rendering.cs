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
}

