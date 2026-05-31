using System.Diagnostics;

namespace SoundBridge;

public static class HotspotManager
{
    public static bool CreateHotspot(string ssid, string password)
    {
        var startInfo = new ProcessStartInfo
        {
            FileName = "netsh",
            Arguments = $"wlan set hostednetwork mode=allow ssid={ssid} key={password}",
            UseShellExecute = false,
            RedirectStandardOutput = true,
            CreateNoWindow = true
        };
        using var process = Process.Start(startInfo);
        process?.WaitForExit();

        var startInfo2 = new ProcessStartInfo
        {
            FileName = "netsh",
            Arguments = "wlan start hostednetwork",
            UseShellExecute = false,
            RedirectStandardOutput = true,
            CreateNoWindow = true
        };
        using var process2 = Process.Start(startInfo2);
        process2?.WaitForExit();
        return process2?.ExitCode == 0;
    }

    public static bool StopHotspot()
    {
        var startInfo = new ProcessStartInfo
        {
            FileName = "netsh",
            Arguments = "wlan stop hostednetwork",
            UseShellExecute = false,
            RedirectStandardOutput = true,
            CreateNoWindow = true
        };
        using var process = Process.Start(startInfo);
        process?.WaitForExit();
        return process?.ExitCode == 0;
    }
}
