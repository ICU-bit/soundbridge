using System.Diagnostics;

namespace SoundBridge.UI;

public static class AdbManager
{
    public static bool SetupPortForward(int localPort, int remotePort)
    {
        var startInfo = new ProcessStartInfo
        {
            FileName = "adb",
            Arguments = $"forward tcp:{localPort} tcp:{remotePort}",
            UseShellExecute = false,
            RedirectStandardOutput = true,
            CreateNoWindow = true
        };
        using var process = Process.Start(startInfo);
        process?.WaitForExit();
        return process?.ExitCode == 0;
    }

    public static bool RemovePortForward(int localPort)
    {
        var startInfo = new ProcessStartInfo
        {
            FileName = "adb",
            Arguments = $"forward --remove tcp:{localPort}",
            UseShellExecute = false,
            RedirectStandardOutput = true,
            CreateNoWindow = true
        };
        using var process = Process.Start(startInfo);
        process?.WaitForExit();
        return process?.ExitCode == 0;
    }

    public static bool IsDeviceConnected()
    {
        var startInfo = new ProcessStartInfo
        {
            FileName = "adb",
            Arguments = "devices",
            UseShellExecute = false,
            RedirectStandardOutput = true,
            CreateNoWindow = true
        };
        using var process = Process.Start(startInfo);
        var output = process?.StandardOutput.ReadToEnd();
        process?.WaitForExit();
        return output?.Contains("device") == true && !output.Contains("List of devices");
    }
}
