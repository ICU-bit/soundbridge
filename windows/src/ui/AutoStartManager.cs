using Microsoft.Win32;

namespace SoundBridge.UI;

public static class AutoStartManager
{
    private const string AppName = "SoundBridge";
    private const string RegistryKey = @"SOFTWARE\Microsoft\Windows\CurrentVersion\Run";

    public static bool IsEnabled()
    {
        try
        {
            using var key = Registry.CurrentUser.OpenSubKey(RegistryKey, false);
            return key?.GetValue(AppName) != null;
        }
        catch
        {
            return false;
        }
    }

    public static void Enable()
    {
        try
        {
            using var key = Registry.CurrentUser.OpenSubKey(RegistryKey, true);
            if (key == null) return;

            var exePath = Environment.ProcessPath ?? "";
            key.SetValue(AppName, $"\"{exePath}\" --minimized");
        }
        catch
        {
            // Registry operation failed silently
        }
    }

    public static void Disable()
    {
        try
        {
            using var key = Registry.CurrentUser.OpenSubKey(RegistryKey, true);
            key?.DeleteValue(AppName, false);
        }
        catch
        {
            // Registry operation failed silently
        }
    }
}
