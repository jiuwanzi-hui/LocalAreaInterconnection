using System;
using System.Diagnostics;
using System.IO;
using System.Text;
using System.Windows.Forms;

public partial class LocalAreaInterconnectionDesktop
{
    const int ElevatedCommandTimeoutMs = 120000;

    string NativeCliPath()
    {
        return Path.Combine(AppDomain.CurrentDomain.BaseDirectory, "LocalAreaInterconnection.Native.Cli.exe");
    }

    bool IsRunningAsAdministrator()
    {
        try
        {
            using (System.Security.Principal.WindowsIdentity identity = System.Security.Principal.WindowsIdentity.GetCurrent())
            {
                System.Security.Principal.WindowsPrincipal principal = new System.Security.Principal.WindowsPrincipal(identity);
                return principal.IsInRole(System.Security.Principal.WindowsBuiltInRole.Administrator);
            }
        }
        catch
        {
            return false;
        }
    }

    bool ConfirmAdminAction(string messageKey, string titleKey)
    {
        return MessageBox.Show(
            this,
            T(messageKey),
            T(titleKey),
            MessageBoxButtons.YesNo,
            MessageBoxIcon.Warning) == DialogResult.Yes;
    }

    string RunNativeCliElevatedWithConfirmation(string arguments, string messageKey, string titleKey, string outputPrefix)
    {
        if (!ConfirmAdminAction(messageKey, titleKey))
        {
            string preview = RunNativeCli(PreviewArguments(arguments));
            return SetOutputTextSafe(preview + Environment.NewLine + Environment.NewLine + T("adminActionPreviewOnly"));
        }
        return RunNativeCliElevated(arguments, outputPrefix);
    }

    string RunNativeCliElevated(string arguments, string outputPrefix)
    {
        return RunNativeCliElevatedBatch(new string[] { arguments }, outputPrefix);
    }

    string RunNativeCliElevatedBatchWithConfirmation(string[] argumentLines, string messageKey, string titleKey, string outputPrefix)
    {
        if (!ConfirmAdminAction(messageKey, titleKey))
        {
            StringBuilder preview = new StringBuilder();
            for (int i = 0; i < argumentLines.Length; i++)
            {
                if (i > 0) preview.AppendLine();
                preview.AppendLine(RunNativeCliCapture(PreviewArguments(argumentLines[i])).Trim());
            }
            return SetOutputTextSafe(preview.ToString() + Environment.NewLine + Environment.NewLine + T("adminActionPreviewOnly"));
        }
        return RunNativeCliElevatedBatch(argumentLines, outputPrefix);
    }

    string RunNativeCliElevatedBatch(string[] argumentLines, string outputPrefix)
    {
        string exe = NativeCliPath();
        if (!File.Exists(exe))
        {
            return SetOutputTextSafe(T("missingNativeCli") + exe);
        }

        string stamp = DateTime.UtcNow.ToString("yyyyMMddHHmmssfff");
        string safePrefix = SafePeerId(outputPrefix);
        string scriptPath = Path.Combine(LogDirectory(), "elevated-" + safePrefix + "-" + stamp + ".cmd");
        string stdoutPath = Path.Combine(LogDirectory(), "elevated-" + safePrefix + "-" + stamp + ".out.txt");
        string stderrPath = Path.Combine(LogDirectory(), "elevated-" + safePrefix + "-" + stamp + ".err.txt");
        string exitPath = Path.Combine(LogDirectory(), "elevated-" + safePrefix + "-" + stamp + ".exit.txt");

        StringBuilder script = new StringBuilder();
        script.AppendLine("@echo off");
        script.AppendLine("chcp 65001 > nul");
        script.AppendLine("set LAI_EXIT=0");
        for (int i = 0; i < argumentLines.Length; i++)
        {
            script.Append(QuoteBatch(exe)).Append(" ").Append(argumentLines[i])
                .Append(" 1>> ").Append(QuoteBatch(stdoutPath))
                .Append(" 2>> ").Append(QuoteBatch(stderrPath)).AppendLine();
            script.AppendLine("if errorlevel 1 (");
            script.AppendLine("  > " + QuoteBatch(exitPath) + " echo 1");
            script.AppendLine("  exit /b 1");
            script.AppendLine(")");
        }
        script.AppendLine("> " + QuoteBatch(exitPath) + " echo 0");
        script.AppendLine("exit /b 0");
        File.WriteAllText(scriptPath, script.ToString(), new UTF8Encoding(false));

        try
        {
            ProcessStartInfo start = new ProcessStartInfo();
            start.FileName = scriptPath;
            start.Verb = "runas";
            start.UseShellExecute = true;
            start.WindowStyle = ProcessWindowStyle.Hidden;
            using (Process process = Process.Start(start))
            {
                if (process == null)
                {
                    return SetOutputTextSafe(T("adminActionStartFailed"));
                }
                if (!process.WaitForExit(ElevatedCommandTimeoutMs))
                {
                    try
                    {
                        KillProcessTree(process, 3000);
                    }
                    catch
                    {
                    }
                    return SetOutputTextSafe(T("adminActionTimedOut"));
                }
            }
        }
        catch (Exception ex)
        {
            return SetOutputTextSafe(T("adminActionCancelled") + Environment.NewLine + ex.Message);
        }

        string stdout = ReadOptionalText(stdoutPath);
        string stderr = ReadOptionalText(stderrPath);
        string exitCode = ReadOptionalText(exitPath).Trim();
        string text = T("adminActionFinished") + " " + (exitCode.Length == 0 ? T("stateUnknown") : exitCode)
            + Environment.NewLine + T("adminActionStdout") + Environment.NewLine + stdout.Trim();
        if (stderr.Trim().Length > 0)
        {
            text += Environment.NewLine + Environment.NewLine + T("adminActionStderr") + Environment.NewLine + stderr.Trim();
        }
        return SetOutputTextSafe(text);
    }

    string SetOutputTextSafe(string text)
    {
        if (output == null) return text;
        try
        {
            if (InvokeRequired && IsHandleCreated)
            {
                BeginInvoke((MethodInvoker)delegate
                {
                    if (output != null && !IsDisposed)
                    {
                        output.Text = text;
                    }
                });
            }
            else
            {
                output.Text = text;
            }
        }
        catch
        {
        }
        return text;
    }

    bool ElevatedTextLooksSuccessful(string text)
    {
        if (text.IndexOf(T("adminActionCancelled"), StringComparison.OrdinalIgnoreCase) >= 0) return false;
        if (text.IndexOf(T("adminActionStartFailed"), StringComparison.OrdinalIgnoreCase) >= 0) return false;
        if (text.IndexOf(T("adminActionTimedOut"), StringComparison.OrdinalIgnoreCase) >= 0) return false;
        if (text.IndexOf(T("adminActionFinished"), StringComparison.OrdinalIgnoreCase) < 0) return false;
        string exitCode = ElevatedExitCode(text);
        return exitCode == "0";
    }

    string ElevatedExitCode(string text)
    {
        string marker = T("adminActionFinished");
        int markerIndex = text.IndexOf(marker, StringComparison.OrdinalIgnoreCase);
        if (markerIndex < 0) return "";
        string after = text.Substring(markerIndex + marker.Length).TrimStart();
        int lineEnd = after.IndexOfAny(new char[] { '\r', '\n' });
        if (lineEnd >= 0) after = after.Substring(0, lineEnd);
        return after.Trim();
    }

    string ReadOptionalText(string path)
    {
        try
        {
            if (File.Exists(path)) return File.ReadAllText(path, Encoding.UTF8);
        }
        catch
        {
        }
        try
        {
            if (File.Exists(path)) return File.ReadAllText(path, Encoding.Default);
        }
        catch
        {
        }
        return "";
    }

    string QuoteBatch(string value)
    {
        return "\"" + value.Replace("\"", "\"\"") + "\"";
    }

    string PreviewArguments(string arguments)
    {
        return arguments
            .Replace(" --yes true", "")
            .Replace(" --yes=true", "")
            .Replace(" --yes", "");
    }
}
