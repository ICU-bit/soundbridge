using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using System.Threading.Tasks;

namespace SoundBridge.UI;

public partial class MainWindowViewModel : ObservableObject
{
    [ObservableProperty]
    private bool _isConnected;

    [ObservableProperty]
    private bool _isConnecting;

    [ObservableProperty]
    private float _audioLevel;

    [ObservableProperty]
    private bool _isMuted;

    [ObservableProperty]
    private string _serverAddress = "192.168.1.100";

    [ObservableProperty]
    private string _serverPort = "8080";

    [ObservableProperty]
    private string _statusText = "Disconnected";

    [RelayCommand]
    private async Task ToggleConnectionAsync()
    {
        if (IsConnected)
        {
            // 断开连接
            IsConnected = false;
            StatusText = "Disconnected";
        }
        else
        {
            // 连接
            IsConnecting = true;
            StatusText = "Connecting...";

            // TODO: 调用 Rust FFI sb_bind / sb_connect / sb_pipeline_start
            await Task.Delay(500); // 模拟连接过程

            IsConnecting = false;
            IsConnected = true;
            StatusText = $"Connected to {ServerAddress}:{ServerPort}";
        }
    }

    [RelayCommand]
    private void ToggleMute()
    {
        IsMuted = !IsMuted;
        // TODO: 调用 Rust FFI 设置静音
    }
}
