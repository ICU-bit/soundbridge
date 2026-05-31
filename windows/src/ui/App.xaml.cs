using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;
using Serilog;
using System;
using System.Windows;
using System.Windows.Interop;

namespace SoundBridge.UI;

public partial class App : Application
{
    private IHost? _host;
    private MainWindow? _window;
    private TrayIcon? _trayIcon;
    private ConnectionNotificationService? _notificationService;

    protected override async void OnStartup(StartupEventArgs e)
    {
        base.OnStartup(e);

        var logPath = System.IO.Path.Combine(
            Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData),
            "SoundBridge", "logs", "soundbridge-.log");

        _host = Host.CreateDefaultBuilder()
            .UseSerilog((_, lc) => lc.MinimumLevel.Debug()
                .WriteTo.Debug()
                .WriteTo.File(logPath, rollingInterval: RollingInterval.Day))
            .ConfigureServices((_, services) =>
            {
                services.AddSingleton<ConnectionNotificationService>(sp =>
                {
                    var logger = sp.GetRequiredService<ILogger<ConnectionNotificationService>>();
                    var dq = System.Windows.Threading.Dispatcher.CurrentDispatcher;
                    return new ConnectionNotificationService(logger, dq, _trayIcon);
                });
                services.AddSingleton<MainWindowViewModel>();
                services.AddSingleton<MainWindow>();
            })
            .Build();

        await _host.StartAsync();
        _window = _host.Services.GetRequiredService<MainWindow>();
        _window.Show();
        InitializeTrayIcon();
        _notificationService = _host.Services.GetService<ConnectionNotificationService>();
    }

    private void InitializeTrayIcon()
    {
        if (_window == null) return;
        var hWnd = new WindowInteropHelper(_window).Handle;
        _trayIcon = new TrayIcon(hWnd, "SoundBridge - Cross-platform Audio Bridge");
        _trayIcon.DoubleClick += () => _window?.Dispatcher.Invoke(() => { _window.Show(); _window.Activate(); });
        _trayIcon.RightClick += ShowTrayContextMenu;
        _trayIcon.Show();
    }

    private void ShowTrayContextMenu()
    {
        if (_window == null || _trayIcon == null) return;

        var viewModel = _host?.Services.GetService<MainWindowViewModel>();
        bool isConnected = viewModel?.IsConnected ?? false;
        string connectionText = isConnected ? "Disconnect" : "Connect";

        var hMenu = NativeMethods.CreatePopupMenu();
        if (hMenu == IntPtr.Zero) return;

        const uint MF_STRING = 0x00000000;
        const uint MF_SEPARATOR = 0x00000800;

        NativeMethods.AppendMenu(hMenu, MF_STRING, 1, "Show Window");
        NativeMethods.AppendMenu(hMenu, MF_SEPARATOR, 0, string.Empty);
        NativeMethods.AppendMenu(hMenu, MF_STRING, 2, connectionText);
        NativeMethods.AppendMenu(hMenu, MF_SEPARATOR, 0, string.Empty);
        NativeMethods.AppendMenu(hMenu, MF_STRING, 3, "Exit");

        NativeMethods.GetCursorPos(out var point);

        uint cmd = NativeMethods.TrackPopupMenu(
            hMenu,
            0x0100,
            point.X,
            point.Y,
            0,
            new WindowInteropHelper(_window!).Handle,
            IntPtr.Zero);

        NativeMethods.DestroyMenu(hMenu);

        switch (cmd)
        {
            case 1:
                _window.Dispatcher.Invoke(() => { _window.Show(); _window.Activate(); });
                break;
            case 2:
                if (viewModel != null)
                    _ = viewModel.ToggleConnectionCommand.ExecuteAsync(null);
                break;
            case 3:
                _notificationService?.Dispose();
                _trayIcon?.Dispose();
                if (viewModel is IDisposable disposable)
                    disposable.Dispose();
                _host?.StopAsync().Wait();
                Shutdown();
                break;
        }
    }

    protected override void OnExit(ExitEventArgs e)
    {
        _trayIcon?.Dispose();
        _host?.StopAsync().Wait();
        base.OnExit(e);
    }

    public static T GetService<T>() where T : class
    {
        var app = (App)Current;
        return app._host!.Services.GetRequiredService<T>();
    }
}
