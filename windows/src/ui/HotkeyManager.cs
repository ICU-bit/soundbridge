using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using Microsoft.Extensions.Logging;

namespace SoundBridge.UI;

/// <summary>
/// 全局快捷键管理器（Win32 RegisterHotKey）
/// </summary>
public sealed class HotkeyManager : IDisposable
{
    // ============================================================
    // P/Invoke
    // ============================================================

    private const int WM_HOTKEY = 0x0312;

    // Modifiers
    private const uint MOD_ALT = 0x0001;
    private const uint MOD_CONTROL = 0x0002;
    private const uint MOD_SHIFT = 0x0004;
    private const uint MOD_WIN = 0x0008;
    private const uint MOD_NOREPEAT = 0x4000;

    [DllImport("user32.dll", SetLastError = true)]
    private static extern bool RegisterHotKey(IntPtr hWnd, int id, uint fsModifiers, uint vk);

    [DllImport("user32.dll", SetLastError = true)]
    private static extern bool UnregisterHotKey(IntPtr hWnd, int id);

    // ============================================================
    // Fields
    // ============================================================

    private readonly ILogger<HotkeyManager> _logger;
    private readonly IntPtr _hWnd;
    private readonly Dictionary<int, Action> _hotkeys = new();
    private int _nextId = 1;
    private bool _disposed;

    // ============================================================
    // Constructor
    // ============================================================

    public HotkeyManager(IntPtr hWnd, ILogger<HotkeyManager> logger)
    {
        _hWnd = hWnd;
        _logger = logger;
    }

    // ============================================================
    // Public Methods
    // ============================================================

    /// <summary>
    /// 注册全局快捷键
    /// </summary>
    /// <param name="modifiers">修饰键（MOD_CONTROL | MOD_ALT 等）</param>
    /// <param name="vk">虚拟键码（如 'M' = 0x4D）</param>
    /// <param name="action">触发时的回调</param>
    /// <returns>热键 ID，失败返回 -1</returns>
    public int Register(uint modifiers, uint vk, Action action)
    {
        int id = _nextId++;
        bool norepeat = true;

        if (RegisterHotKey(_hWnd, id, modifiers | (norepeat ? MOD_NOREPEAT : 0), vk))
        {
            _hotkeys[id] = action;
            _logger.LogDebug("Registered hotkey id={Id} mod=0x{Mod:X} vk=0x{Vk:X}", id, modifiers, vk);
            return id;
        }

        _logger.LogWarning("Failed to register hotkey id={Id} mod=0x{Mod:X} vk=0x{Vk:X}, error={Error}",
            id, modifiers, vk, Marshal.GetLastWin32Error());
        return -1;
    }

    /// <summary>
    /// 注销全局快捷键
    /// </summary>
    public void Unregister(int id)
    {
        if (id < 0) return;
        UnregisterHotKey(_hWnd, id);
        _hotkeys.Remove(id);
    }

    /// <summary>
    /// 处理窗口消息（在 WndProc 中调用）
    /// 返回 true 表示消息已处理
    /// </summary>
    public bool HandleWindowMessage(IntPtr hWnd, uint msg, IntPtr wParam, IntPtr lParam)
    {
        if (msg != WM_HOTKEY) return false;

        int id = wParam.ToInt32();
        if (_hotkeys.TryGetValue(id, out var action))
        {
            try
            {
                action.Invoke();
            }
            catch (Exception ex)
            {
                _logger.LogError(ex, "Hotkey handler error for id={Id}", id);
            }
            return true;
        }

        return false;
    }

    // ============================================================
    // Dispose
    // ============================================================

    public void Dispose()
    {
        if (_disposed) return;
        _disposed = true;

        foreach (var id in _hotkeys.Keys)
        {
            UnregisterHotKey(_hWnd, id);
        }
        _hotkeys.Clear();
        _logger.LogInformation("All hotkeys unregistered");
    }
}
