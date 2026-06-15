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
}

