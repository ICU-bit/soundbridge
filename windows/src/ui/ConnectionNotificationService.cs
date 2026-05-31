using Microsoft.Extensions.Logging;
using System;
using System.Runtime.InteropServices;
using System.Windows.Threading;

namespace SoundBridge.UI;

/// <summary>
/// 连接状态通知服务 — 监听 Rust FFI 状态回调，弹出托盘气泡通知。
/// </summary>
public sealed class ConnectionNotificationService : IDisposable
{
    private readonly ILogger<ConnectionNotificationService> _logger;
    private readonly Dispatcher _dispatcher;
    private readonly TrayIcon? _trayIcon;

    private GCHandle _callbackHandle;
    private SbStateCallback? _callback;
    private IntPtr _engine;
    private bool _disposed;

    private SbConnectionState _lastNotifiedState = SbConnectionState.Disconnected;
    private long _lastNotifyTick;

    public ConnectionNotificationService(
        ILogger<ConnectionNotificationService> logger,
        Dispatcher dispatcher,
        TrayIcon? trayIcon = null)
    {
        _logger = logger;
        _dispatcher = dispatcher;
        _trayIcon = trayIcon;
    }

    public void Register(IntPtr engine)
    {
        if (engine == IntPtr.Zero)
        {
            _logger.LogWarning("Cannot register state callback: engine is null");
            return;
        }

        _engine = engine;
        _callback = OnStateChanged;
        _callbackHandle = GCHandle.Alloc(_callback);

        int rc = NativeMethods.sb_set_state_callback(engine, _callback, IntPtr.Zero);
        if (rc != NativeMethods.SB_OK)
        {
            _logger.LogError("sb_set_state_callback failed: {Error}", NativeMethods.GetLastError());
            _callbackHandle.Free();
            _callback = null;
            return;
        }

        _logger.LogInformation("State callback registered on engine 0x{Addr}", engine.ToString("X"));
    }

    public void Unregister()
    {
        if (_engine == IntPtr.Zero || _callback == null) return;

        NativeMethods.sb_set_state_callback(_engine, null, IntPtr.Zero);
        _logger.LogInformation("State callback unregistered");

        if (_callbackHandle.IsAllocated)
            _callbackHandle.Free();

        _callback = null;
    }

    public void ShowNotification(SbConnectionState state, string? detail = null)
    {
        long now = Environment.TickCount64;
        if (state == _lastNotifiedState && (now - _lastNotifyTick) < 500)
            return;

        _lastNotifiedState = state;
        _lastNotifyTick = now;

        var (title, message) = GetNotificationContent(state, detail);
        ShowTrayBalloon(title, message);
    }

    private void OnStateChanged(SbConnectionState state, IntPtr userData)
    {
        _logger.LogInformation("State callback fired: {State}", state);
        _dispatcher.Invoke(() => ShowNotification(state));
    }

    private void ShowTrayBalloon(string title, string message)
    {
        try
        {
            _trayIcon?.ShowBalloon(title, message, 3000);
            _logger.LogDebug("Tray balloon shown: {Title} - {Message}", title, message);
        }
        catch (Exception ex)
        {
            _logger.LogDebug(ex, "Failed to show tray balloon");
        }
    }

    private static (string Title, string Message) GetNotificationContent(
        SbConnectionState state, string? detail)
    {
        return state switch
        {
            SbConnectionState.Disconnected => ("SoundBridge", detail ?? "已断开连接"),
            SbConnectionState.Connecting => ("SoundBridge - 正在连接", detail ?? "正在建立连接..."),
            SbConnectionState.Connected => ("SoundBridge - 已连接", detail ?? "音频通道已建立"),
            SbConnectionState.Error => ("SoundBridge - 连接错误", detail ?? "连接发生错误，请检查网络"),
            _ => ("SoundBridge", detail ?? "状态未知"),
        };
    }

    public void Dispose()
    {
        if (_disposed) return;
        _disposed = true;

        Unregister();
        _logger.LogInformation("ConnectionNotificationService disposed");
    }
}
