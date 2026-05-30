using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;
using Microsoft.UI.Xaml;
using Microsoft.Windows.AppNotifications;
using Serilog;
using System;

namespace SoundBridge.UI;

public partial class App : Application
{
    private readonly IHost _host;
    private Window? _window;
    private TrayIcon? _trayIcon;
    private ConnectionNotificationService? _notificationService;

    public App()
    {
        InitializeComponent();

        _host = Host.CreateDefaultBuilder()
            .UseSerilog((context, loggerConfiguration) =>
            {
                loggerConfiguration
                    .MinimumLevel.Debug()
                    .WriteTo.Debug();
            })
            .ConfigureServices((context, services) =>
            {
                // 注册通知管理器（单例）
                services.AddSingleton(AppNotificationManager.Default);

                // 注册通知服务（需要 TrayIcon，稍后注入）
                services.AddSingleton<ConnectionNotificationService>(sp =>
                {
                    var logger = sp.GetRequiredService<ILogger<ConnectionNotificationService>>();
                    var notificationManager = sp.GetRequiredService<AppNotificationManager>();
                    var dispatcherQueue = Microsoft.UI.Dispatching.DispatcherQueue.GetForCurrentThread();
                    return new ConnectionNotificationService(logger, notificationManager, dispatcherQueue, _trayIcon);
                });

                services.AddSingleton<MainWindow>();
                services.AddSingleton<MainWindowViewModel>();
            })
            .Build();
    }

    protected override async void OnLaunched(LaunchActivatedEventArgs args)
    {
        await _host.StartAsync();

        _window = _host.Services.GetRequiredService<MainWindow>();
        _window.Activate();

        // 初始化系统托盘图标
        InitializeTrayIcon();

        // 初始化通知服务（依赖 TrayIcon）
        _notificationService = _host.Services.GetService<ConnectionNotificationService>();
    }

    private void InitializeTrayIcon()
    {
        if (_window == null) return;

        // 获取窗口句柄
        var hWnd = WinRT.Interop.WindowNative.GetWindowHandle(_window);
        _trayIcon = new TrayIcon(hWnd, "SoundBridge - Cross-platform Audio Bridge");

        // 双击托盘图标 → 显示主窗口
        _trayIcon.DoubleClick += () =>
        {
            if (_window != null)
            {
                _window.Activate();
            }
        };

        // 右键托盘图标 → 显示上下文菜单
        _trayIcon.RightClick += ShowTrayContextMenu;

        // 显示托盘图标
        _trayIcon.Show();

        // 最小化到托盘
        _window.Closed += (s, e) =>
        {
            // 不真正退出，只是隐藏到托盘
            // 用户需要通过托盘菜单"退出"才能真正退出
        };
    }

    private void ShowTrayContextMenu()
    {
        if (_window == null || _trayIcon == null) return;

        var viewModel = _host.Services.GetService<MainWindowViewModel>();
        bool isConnected = viewModel?.IsConnected ?? false;
        string connectionText = isConnected ? "断开连接" : "连接";

        // 使用 PopupMenu（WinUI 3 推荐方式）
        var menu = new Microsoft.UI.Xaml.Controls.MenuFlyout();

        var showItem = new Microsoft.UI.Xaml.Controls.MenuFlyoutItem { Text = "显示主窗口" };
        showItem.Click += (s, e) => _window.Activate();

        var connectItem = new Microsoft.UI.Xaml.Controls.MenuFlyoutItem { Text = connectionText };
        connectItem.Click += async (s, e) =>
        {
            if (viewModel != null)
                await viewModel.ToggleConnectionCommand.ExecuteAsync(null);
        };

        var exitItem = new Microsoft.UI.Xaml.Controls.MenuFlyoutItem { Text = "退出" };
        exitItem.Click += (s, e) =>
        {
            _notificationService?.Dispose();
            _notificationService = null;

            _trayIcon?.Dispose();
            _trayIcon = null;

            // 清理 ViewModel
            if (viewModel is IDisposable disposable)
                disposable.Dispose();

            _host.StopAsync().Wait();
            Environment.Exit(0);
        };

        menu.Items.Add(showItem);
        menu.Items.Add(new Microsoft.UI.Xaml.Controls.MenuFlyoutSeparator());
        menu.Items.Add(connectItem);
        menu.Items.Add(new Microsoft.UI.Xaml.Controls.MenuFlyoutSeparator());
        menu.Items.Add(exitItem);

        // 显示菜单（在鼠标位置）
        // 注意：WinUI 3 的 MenuFlyout 需要附加到元素上才能显示
        // 这里使用 Win32 TrackPopupMenuEx 作为替代
        ShowWin32ContextMenu(showItem.Text, connectionText);
    }

    private void ShowWin32ContextMenu(string showText, string connectText)
    {
        // 使用 Win32 API 显示上下文菜单
        var hMenu = NativeMethods.CreatePopupMenu();
        if (hMenu == IntPtr.Zero) return;

        const uint MF_STRING = 0x00000000;
        const uint MF_SEPARATOR = 0x00000800;

        NativeMethods.AppendMenu(hMenu, MF_STRING, 1, showText);
        NativeMethods.AppendMenu(hMenu, MF_SEPARATOR, 0, string.Empty);
        NativeMethods.AppendMenu(hMenu, MF_STRING, 2, connectText);
        NativeMethods.AppendMenu(hMenu, MF_SEPARATOR, 0, string.Empty);
        NativeMethods.AppendMenu(hMenu, MF_STRING, 3, "退出");

        // 获取鼠标位置
        NativeMethods.GetCursorPos(out var point);

        // 显示菜单
        uint cmd = NativeMethods.TrackPopupMenu(
            hMenu,
            0x0100, // TPM_RETURNCMD
            point.X,
            point.Y,
            0,
            WinRT.Interop.WindowNative.GetWindowHandle(_window!),
            IntPtr.Zero);

        NativeMethods.DestroyMenu(hMenu);

        // 处理菜单命令
        switch (cmd)
        {
            case 1: // 显示主窗口
                _window?.Activate();
                break;
            case 2: // 连接/断开
                var viewModel = _host.Services.GetService<MainWindowViewModel>();
                if (viewModel != null)
                    _ = viewModel.ToggleConnectionCommand.ExecuteAsync(null);
                break;
            case 3: // 退出
                _notificationService?.Dispose();
                _notificationService = null;

                _trayIcon?.Dispose();
                _trayIcon = null;

                var vm = _host.Services.GetService<MainWindowViewModel>();
                if (vm is IDisposable disposable)
                    disposable.Dispose();

                _host.StopAsync().Wait();
                Environment.Exit(0);
                break;
        }
    }

    public static T GetService<T>() where T : class
    {
        var app = (App)Current;
        return app._host.Services.GetRequiredService<T>();
    }

    /// <summary>
    /// 激活主窗口（供通知点击回调调用）
    /// </summary>
    internal void ActivateMainWindow()
    {
        _window?.Activate();
    }
}
