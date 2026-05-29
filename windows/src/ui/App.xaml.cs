using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;
using Microsoft.UI.Xaml;
using Serilog;

namespace SoundBridge.UI;

public partial class App : Application
{
    private readonly IHost _host;
    private Window? _window;

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
    }

    public static T GetService<T>() where T : class
    {
        var app = (App)Current;
        return app._host.Services.GetRequiredService<T>();
    }
}
