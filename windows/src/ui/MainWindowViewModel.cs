using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using Microsoft.Extensions.Logging;
using System;
using System.Threading;
using System.Threading.Tasks;

namespace SoundBridge.UI;

public partial class MainWindowViewModel : ObservableObject, IDisposable
{
    private readonly ILogger<MainWindowViewModel> _logger;
    private IntPtr _engine;
    private IntPtr _deviceStore;
    private CancellationTokenSource? _statsCts;
    private bool _disposed;

    public MainWindowViewModel(ILogger<MainWindowViewModel> logger)
    {
        _logger = logger;

        // 创建引擎
        _engine = NativeMethods.sb_engine_create();
        if (_engine == IntPtr.Zero)
        {
            _logger.LogError("Failed to create engine");
            StatusText = "Engine init failed";
        }
        else
        {
            _logger.LogInformation("Engine created: 0x{Addr}", _engine.ToString("X"));
        }

        // 打开设备存储（持久化到文件）
        string storePath = System.IO.Path.Combine(
            Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData),
            "SoundBridge", "devices.json");
        _deviceStore = NativeMethods.sb_device_store_open(storePath);
        if (_deviceStore != IntPtr.Zero)
        {
            _logger.LogInformation("Device store opened: {Path}", storePath);
            // 恢复上次的服务器地址
            LoadLastServer();
        }
    }

    // ============================================================
    // Observable Properties
    // ============================================================

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

    [ObservableProperty]
    private ulong _framesCaptured;

    [ObservableProperty]
    private ulong _framesPlayed;

    [ObservableProperty]
    private float _latencyMs;

    [ObservableProperty]
    private ushort _localPort;

    // ============================================================
    // Commands
    // ============================================================

    [RelayCommand]
    private async Task ToggleConnectionAsync()
    {
        if (_engine == IntPtr.Zero)
        {
            StatusText = "Engine not initialized";
            return;
        }

        if (IsConnected)
        {
            await DisconnectAsync();
        }
        else
        {
            await ConnectAsync();
        }
    }

    [RelayCommand]
    private void ToggleMute()
    {
        IsMuted = !IsMuted;
        // 静音通过采集层控制（暂停/恢复采集流）
        // TODO: 当 Rust FFI 支持 sb_set_mute 时接入
        _logger.LogInformation("Mute toggled: {IsMuted}", IsMuted);
    }

    // ============================================================
    // Connection Logic
    // ============================================================

    private async Task ConnectAsync()
    {
        IsConnecting = true;
        StatusText = "Connecting...";

        try
        {
            // 在后台线程执行阻塞的 FFI 调用
            await Task.Run(() =>
            {
                // 1. 绑定本地 UDP 端口（0 = 自动分配）
                int rc = NativeMethods.sb_bind(_engine, 0);
                if (rc != NativeMethods.SB_OK)
                    throw new InvalidOperationException($"Bind failed: {NativeMethods.GetLastError()}");

                // 获取分配的本地端口
                rc = NativeMethods.sb_local_port(_engine, out ushort port);
                if (rc != NativeMethods.SB_OK)
                    throw new InvalidOperationException($"Local port query failed: {NativeMethods.GetLastError()}");

                LocalPort = port;
                _logger.LogInformation("Bound to local port {Port}", port);

                // 2. 启动采集（默认设备）
                rc = NativeMethods.sb_capture_start(_engine, IntPtr.Zero);
                if (rc != NativeMethods.SB_OK)
                    throw new InvalidOperationException($"Capture start failed: {NativeMethods.GetLastError()}");

                _logger.LogInformation("Capture started");

                // 3. 启动播放（默认设备）
                rc = NativeMethods.sb_playback_start(_engine, IntPtr.Zero);
                if (rc != NativeMethods.SB_OK)
                {
                    NativeMethods.sb_capture_stop(_engine);
                    throw new InvalidOperationException($"Playback start failed: {NativeMethods.GetLastError()}");
                }

                _logger.LogInformation("Playback started");

                // 4. 设置目标地址
                string targetAddr = $"{ServerAddress}:{ServerPort}";
                rc = NativeMethods.sb_connect(_engine, targetAddr);
                if (rc != NativeMethods.SB_OK)
                {
                    NativeMethods.sb_playback_stop(_engine);
                    NativeMethods.sb_capture_stop(_engine);
                    throw new InvalidOperationException($"Connect to {targetAddr} failed: {NativeMethods.GetLastError()}");
                }

                _logger.LogInformation("Target set to {Addr}", targetAddr);

                // 5. 启动管线（发送线程 + 接收线程）
                rc = NativeMethods.sb_pipeline_start(_engine);
                if (rc != NativeMethods.SB_OK)
                {
                    NativeMethods.sb_playback_stop(_engine);
                    NativeMethods.sb_capture_stop(_engine);
                    throw new InvalidOperationException($"Pipeline start failed: {NativeMethods.GetLastError()}");
                }

                _logger.LogInformation("Pipeline started");
            });

            IsConnecting = false;
            IsConnected = true;
            StatusText = $"Connected to {ServerAddress}:{ServerPort} (local:{LocalPort})";

            // 保存服务器地址到设备存储
            SaveLastServer();

            // 启动统计轮询
            StartStatsPolling();
        }
        catch (Exception ex)
        {
            IsConnecting = false;
            StatusText = $"Connection failed: {ex.Message}";
            _logger.LogError(ex, "Connection failed");
        }
    }

    private async Task DisconnectAsync()
    {
        StopStatsPolling();

        await Task.Run(() =>
        {
            NativeMethods.sb_pipeline_stop(_engine);
            NativeMethods.sb_capture_stop(_engine);
            NativeMethods.sb_playback_stop(_engine);
            _logger.LogInformation("Disconnected, pipeline stopped");
        });

        IsConnected = false;
        IsMuted = false;
        AudioLevel = 0;
        FramesCaptured = 0;
        FramesPlayed = 0;
        LatencyMs = 0;
        StatusText = "Disconnected";
    }

    // ============================================================
    // Stats Polling
    // ============================================================

    private void StartStatsPolling()
    {
        _statsCts = new CancellationTokenSource();
        var ct = _statsCts.Token;

        _ = Task.Run(async () =>
        {
            while (!ct.IsCancellationRequested)
            {
                try
                {
                    int rc = NativeMethods.sb_pipeline_stats(
                        _engine,
                        out ulong captured,
                        out ulong played,
                        out float latency);

                    if (rc == NativeMethods.SB_OK)
                    {
                        FramesCaptured = captured;
                        FramesPlayed = played;
                        LatencyMs = latency;

                        // 音频电平：用帧编码速率估算（简化）
                        // 真实电平需要从采集 ring buffer 读取 RMS
                        AudioLevel = Math.Clamp(captured % 100 / 100f, 0f, 1f);
                    }

                    // 检查管线状态
                    rc = NativeMethods.sb_pipeline_state(_engine, out int state);
                    if (rc == NativeMethods.SB_OK && state == NativeMethods.SB_PIPELINE_ERROR)
                    {
                        StatusText = "Pipeline error";
                        _logger.LogWarning("Pipeline entered error state");
                    }
                }
                catch (Exception ex)
                {
                    _logger.LogDebug(ex, "Stats polling error");
                }

                await Task.Delay(100, ct);
            }
        }, ct);
    }

    private void StopStatsPolling()
    {
        _statsCts?.Cancel();
        _statsCts?.Dispose();
        _statsCts = null;
    }

    // ============================================================
    // Device Store
    // ============================================================

    private const string LastServerName = "last_server";

    private void LoadLastServer()
    {
        if (_deviceStore == IntPtr.Zero) return;

        try
        {
            int has = NativeMethods.sb_device_store_has(_deviceStore, LastServerName);
            if (has != 1) return;

            // 读取地址
            byte[] addrBuf = new byte[256];
            int addrLen = NativeMethods.sb_device_store_get_address(_deviceStore, LastServerName, addrBuf, 256);
            if (addrLen <= 0) return;
            string address = System.Text.Encoding.UTF8.GetString(addrBuf, 0, addrLen);

            // 读取端口
            int rc = NativeMethods.sb_device_store_get_port(_deviceStore, LastServerName, out ushort port);
            if (rc != NativeMethods.SB_OK) return;

            ServerAddress = address;
            ServerPort = port.ToString();
            _logger.LogInformation("Restored last server: {Addr}:{Port}", address, port);
        }
        catch (Exception ex)
        {
            _logger.LogWarning(ex, "Failed to load last server");
        }
    }

    private void SaveLastServer()
    {
        if (_deviceStore == IntPtr.Zero) return;

        try
        {
            if (ushort.TryParse(ServerPort, out ushort port))
            {
                NativeMethods.sb_device_store_add(_deviceStore, LastServerName, ServerAddress, port);
                _logger.LogInformation("Saved last server: {Addr}:{Port}", ServerAddress, port);
            }
        }
        catch (Exception ex)
        {
            _logger.LogWarning(ex, "Failed to save last server");
        }
    }

    // ============================================================
    // Auto-start (Windows Registry)
    // ============================================================

    private const string AppName = "SoundBridge";
    private const string RunKeyPath = @"SOFTWARE\Microsoft\Windows\CurrentVersion\Run";

    /// <summary>检查是否已设置开机自启</summary>
    public static bool IsAutoStartEnabled()
    {
        try
        {
            using var key = Microsoft.Win32.Registry.CurrentUser.OpenSubKey(RunKeyPath, false);
            return key?.GetValue(AppName) != null;
        }
        catch
        {
            return false;
        }
    }

    /// <summary>设置开机自启</summary>
    public static void SetAutoStart(bool enable)
    {
        try
        {
            using var key = Microsoft.Win32.Registry.CurrentUser.OpenSubKey(RunKeyPath, true);
            if (key == null) return;

            if (enable)
            {
                string exePath = Environment.ProcessPath ?? "";
                key.SetValue(AppName, $"\"{exePath}\" --minimized");
            }
            else
            {
                key.DeleteValue(AppName, false);
            }
        }
        catch (Exception)
        {
            // 忽略注册表操作失败
        }
    }

    // ============================================================
    // Dispose
    // ============================================================

    public void Dispose()
    {
        if (_disposed) return;
        _disposed = true;

        StopStatsPolling();

        if (_engine != IntPtr.Zero)
        {
            // 先停止管线，再销毁引擎
            NativeMethods.sb_pipeline_stop(_engine);
            NativeMethods.sb_capture_stop(_engine);
            NativeMethods.sb_playback_stop(_engine);
            NativeMethods.sb_engine_destroy(_engine);
            _engine = IntPtr.Zero;
            _logger.LogInformation("Engine destroyed");
        }

        if (_deviceStore != IntPtr.Zero)
        {
            NativeMethods.sb_device_store_close(_deviceStore);
            _deviceStore = IntPtr.Zero;
            _logger.LogInformation("Device store closed");
        }

        GC.SuppressFinalize(this);
    }

    ~MainWindowViewModel() => Dispose();
}
