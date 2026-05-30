using Microsoft.Extensions.Logging;
using Microsoft.UI.Dispatching;
using Microsoft.Windows.AppNotifications;
using Microsoft.Windows.AppNotifications.Builder;
using System;
using System.Runtime.InteropServices;

namespace SoundBridge.UI;

/// <summary>
/// 连接状态通知服务 — 监听 Rust FFI 状态回调，弹出 Windows Toast 通知。
///
/// 使用方式：
///   1. DI 注入 AppNotificationManager + TrayIcon
///   2. 构造后调用 Register(engine) 订阅 sb_set_state_callback
///   3. Dispose 时取消注册并关闭通知中心
/// </summary>
internal sealed class ConnectionNotificationService : IDisposable
{
    // ============================================================
    // Constants
    // ============================================================

    private const string AppId = "SoundBridge";

    // ============================================================
    // Fields
    // ============================================================

    private readonly ILogger<ConnectionNotificationService> _logger;
    private readonly AppNotificationManager _notificationManager;
    private readonly DispatcherQueue _dispatcherQueue;
    private readonly TrayIcon? _trayIcon;

    // GCHandle 防止回调委托被 GC 回收
    private GCHandle _callbackHandle;
    private SbStateCallback? _callback;
    private IntPtr _engine;
    private bool _disposed;

    // 防止同一状态短时间内重复通知（500ms 去抖）
    private SbConnectionState _lastNotifiedState = SbConnectionState.Disconnected;
    private long _lastNotifyTick;

    // ============================================================
    // Constructor
    // ============================================================

    public ConnectionNotificationService(
        ILogger<ConnectionNotificationService> logger,
        AppNotificationManager notificationManager,
        DispatcherQueue dispatcherQueue,
        TrayIcon? trayIcon = null)
    {
        _logger = logger;
        _notificationManager = notificationManager;
        _dispatcherQueue = dispatcherQueue;
        _trayIcon = trayIcon;

        // 初始化通知管理器（unpackaged app 需要显式初始化）
        _notificationManager.NotificationInvoked += OnNotificationInvoked;
        _notificationManager.Register();
        _logger.LogDebug("AppNotificationManager registered");
    }

    // ============================================================
    // Public Methods
    // ============================================================

    /// <summary>
    /// 注册 Rust 引擎的状态回调。
    /// 调用后状态变化会自动弹出 Toast 通知。
    /// </summary>
    /// <param name="engine">引擎指针（来自 sb_engine_create）</param>
    public void Register(IntPtr engine)
    {
        if (engine == IntPtr.Zero)
        {
            _logger.LogWarning("Cannot register state callback: engine is null");
            return;
        }

        _engine = engine;

        // 创建回调委托并固定（防止 GC）
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

    /// <summary>
    /// 取消状态回调注册。
    /// </summary>
    public void Unregister()
    {
        if (_engine == IntPtr.Zero || _callback == null) return;

        NativeMethods.sb_set_state_callback(_engine, null, IntPtr.Zero);
        _logger.LogInformation("State callback unregistered");

        if (_callbackHandle.IsAllocated)
            _callbackHandle.Free();

        _callback = null;
    }

    /// <summary>
    /// 手动弹出一条连接状态通知（供外部调用，如 ConnectAsync 成功后）。
    /// </summary>
    public void ShowNotification(SbConnectionState state, string? detail = null)
    {
        // 去抖：同状态 500ms 内不重复弹
        long now = Environment.TickCount64;
        if (state == _lastNotifiedState && (now - _lastNotifyTick) < 500)
            return;

        _lastNotifiedState = state;
        _lastNotifyTick = now;

        var (title, message, icon) = GetNotificationContent(state, detail);

        // 1. WinUI 3 Toast 通知
        ShowToast(title, message, icon);

        // 2. 托盘气泡通知（兼容旧版 Windows）
        ShowTrayBalloon(title, message);
    }

    // ============================================================
    // Private: FFI Callback（从 Rust 线程调用）
    // ============================================================

    /// <summary>
    /// Rust 引擎状态回调 — 可能从任意线程调用。
    /// </summary>
    private void OnStateChanged(SbConnectionState state, IntPtr userData)
    {
        _logger.LogInformation("State callback fired: {State}", state);

        // 切换到 UI 线程弹通知
        _dispatcherQueue.TryEnqueue(() =>
        {
            ShowNotification(state);
        });
    }

    // ============================================================
    // Private: Toast 通知
    // ============================================================

    private void ShowToast(string title, string message, string icon)
    {
        try
        {
            var builder = new AppNotificationBuilder()
                .AddText(title)
                .AddText(message);

            // 设置通知图标（使用 app logo）
            var notification = builder.BuildNotification();
            notification.Tag = "connection-state";
            notification.Group = "status";

            _notificationManager.Show(notification);
            _logger.LogDebug("Toast shown: {Title} - {Message}", title, message);
        }
        catch (Exception ex)
        {
            _logger.LogWarning(ex, "Failed to show toast notification");
        }
    }

    // ============================================================
    // Private: 托盘气泡（降级方案）
    // ============================================================

    private void ShowTrayBalloon(string title, string message)
    {
        try
        {
            _trayIcon?.ShowBalloon(title, message, 3000);
        }
        catch (Exception ex)
        {
            _logger.LogDebug(ex, "Failed to show tray balloon");
        }
    }

    // ============================================================
    // Private: 通知内容
    // ============================================================

    private static (string Title, string Message, string Icon) GetNotificationContent(
        SbConnectionState state, string? detail)
    {
        return state switch
        {
            SbConnectionState.Disconnected => (
                "SoundBridge",
                detail ?? "已断开连接",
                "\uE711" // Close icon
            ),
            SbConnectionState.Connecting => (
                "SoundBridge - 正在连接",
                detail ?? "正在建立连接...",
                "\uE768" // Play icon
            ),
            SbConnectionState.Connected => (
                "SoundBridge - 已连接",
                detail ?? "音频通道已建立",
                "\uE73E" // Checkmark icon
            ),
            SbConnectionState.Error => (
                "SoundBridge - 连接错误",
                detail ?? "连接发生错误，请检查网络",
                "\uE730" // Error icon
            ),
            _ => ("SoundBridge", detail ?? "状态未知", "\uE783" // Info icon
            ),
        };
    }

    // ============================================================
    // Private: 通知点击处理
    // ============================================================

    private void OnNotificationInvoked(AppNotificationManager sender, AppNotificationActivatedEventArgs args)
    {
        _logger.LogDebug("Notification invoked: tag={Tag}", args.Arguments);

        // 通知被点击 → 激活主窗口
        _dispatcherQueue.TryEnqueue(() =>
        {
            // 通过静态引用激活窗口（避免循环依赖）
            var app = (App)Microsoft.UI.Xaml.Application.Current;
            app.ActivateMainWindow();
        });
    }

    // ============================================================
    // Dispose
    // ============================================================

    public void Dispose()
    {
        if (_disposed) return;
        _disposed = true;

        Unregister();

        _notificationManager.NotificationInvoked -= OnNotificationInvoked;
        _notificationManager.Unregister();

        _logger.LogInformation("ConnectionNotificationService disposed");
    }
}
