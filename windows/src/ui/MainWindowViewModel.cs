using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using Microsoft.Extensions.Logging;
using System;
using System.Collections.ObjectModel;
using System.Threading;
using System.Threading.Tasks;

namespace SoundBridge.UI;

public partial class MainWindowViewModel : ObservableObject, IDisposable
{
    private readonly ILogger<MainWindowViewModel> _logger;
    private readonly ConnectionNotificationService? _notificationService;
    private IntPtr _engine;
    private IntPtr _deviceStore;
    private IntPtr _discovery;
    private CancellationTokenSource? _statsCts;
    private bool _disposed;

    public MainWindowViewModel(ILogger<MainWindowViewModel> logger, ConnectionNotificationService? notificationService = null)
    {
        _logger = logger;
        _notificationService = notificationService;

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

            // 注册连接状态回调 → 触发 Toast 通知
            _notificationService?.Register(_engine);

            // 读取当前音频模式
            if (NativeMethods.sb_get_audio_mode(_engine, out int mode) == NativeMethods.SB_OK)
            {
                _selectedAudioMode = mode;
                _logger.LogInformation("Initial audio mode: {Mode}", mode);
            }
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

        // 创建设备发现服务
        _discovery = NativeMethods.sb_discovery_create();
        if (_discovery != IntPtr.Zero)
        {
            _logger.LogInformation("Discovery service created");
            // 初始化 mDNS（后台线程执行，避免阻塞 UI）
            Task.Run(() =>
            {
                int rc = NativeMethods.sb_discovery_init(_discovery);
                if (rc == NativeMethods.SB_OK)
                    _logger.LogInformation("mDNS initialized");
                else
                    _logger.LogWarning("mDNS init failed: {Error}", NativeMethods.GetLastError());
            });
        }
        else
        {
            _logger.LogWarning("Failed to create discovery service");
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
    private float _lossRate;

    [ObservableProperty]
    private ushort _localPort;

    [ObservableProperty]
    private double _volume = 80;

    [ObservableProperty]
    private bool _isPaused;

    [ObservableProperty]
    private int _selectedAudioMode; // 0=Balanced, 1=HighQuality, 2=LowLatency

    [ObservableProperty]
    private double _mixRatio = 50; // 0=全PC, 100=全手机, 50=均衡

    [ObservableProperty]
    private int _selectedConnectionType; // 0=WiFiLan, 1=WiFiDirect, 2=UsbAdb, 3=Bluetooth

    [ObservableProperty]
    private bool _isScanning;

    /// <summary>已发现的设备列表</summary>
    public ObservableCollection<string> DiscoveredDevices { get; } = new();

    /// <summary>连接方式名称列表</summary>
    public string[] ConnectionTypeNames { get; } = { "WiFi 局域网", "WiFi 直连", "USB/ADB", "蓝牙" };

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

    [RelayCommand]
    private async Task ScanDevicesAsync()
    {
        if (_discovery == IntPtr.Zero)
        {
            StatusText = "Discovery not available";
            return;
        }

        if (IsScanning) return;

        IsScanning = true;
        StatusText = "Scanning for devices...";
        DiscoveredDevices.Clear();

        try
        {
            var devices = await Task.Run(() =>
            {
                // 获取设备数量
                int count = NativeMethods.sb_discovery_find_devices(_discovery, IntPtr.Zero, 0);
                if (count <= 0) return Array.Empty<string>();

                // 分配指针数组接收设备信息
                var ptrs = new IntPtr[count];
                var handle = System.Runtime.InteropServices.GCHandle.Alloc(ptrs, System.Runtime.InteropServices.GCHandleType.Pinned);
                try
                {
                    NativeMethods.sb_discovery_find_devices(_discovery, handle.AddrOfPinnedObject(), (nuint)count);
                    var results = new string[count];
                    for (int i = 0; i < count; i++)
                    {
                        if (ptrs[i] != IntPtr.Zero)
                        {
                            results[i] = System.Runtime.InteropServices.Marshal.PtrToStringAnsi(ptrs[i]) ?? $"device_{i}";
                            NativeMethods.sb_discovery_free_device_info(ptrs[i]);
                        }
                        else
                        {
                            results[i] = $"device_{i}";
                        }
                    }
                    return results;
                }
                finally
                {
                    handle.Free();
                }
            });

            foreach (var device in devices)
            {
                DiscoveredDevices.Add(device);
            }

            StatusText = devices.Length > 0
                ? $"Found {devices.Length} device(s)"
                : "No devices found";
            _logger.LogInformation("Scan complete: {Count} devices found", devices.Length);
        }
        catch (Exception ex)
        {
            StatusText = $"Scan failed: {ex.Message}";
            _logger.LogError(ex, "Device scan failed");
        }
        finally
        {
            IsScanning = false;
        }
    }

    /// <summary>从发现的设备 JSON 字符串中解析地址并填入 ServerAddress/ServerPort</summary>
    [RelayCommand]
    private void SelectDevice(string? deviceJson)
    {
        if (string.IsNullOrEmpty(deviceJson)) return;

        try
        {
            // 解析简单 JSON: {"name":"...","address":"...","port":...,"hostname":"..."}
            var addressMatch = System.Text.RegularExpressions.Regex.Match(deviceJson, @"""address""\s*:\s*""([^""]+)""");
            var portMatch = System.Text.RegularExpressions.Regex.Match(deviceJson, @"""port""\s*:\s*(\d+)");

            if (addressMatch.Success)
                ServerAddress = addressMatch.Groups[1].Value;
            if (portMatch.Success && ushort.TryParse(portMatch.Groups[1].Value, out ushort port))
                ServerPort = port.ToString();

            _logger.LogInformation("Selected device: {Addr}:{Port}", ServerAddress, ServerPort);
        }
        catch (Exception ex)
        {
            _logger.LogWarning(ex, "Failed to parse device JSON");
        }
    }

    partial void OnVolumeChanged(double value)
    {
        if (_engine == IntPtr.Zero) return;
        float vol = (float)(value / 100.0);
        int rc = NativeMethods.sb_send_volume(_engine, vol);
        if (rc != NativeMethods.SB_OK)
            _logger.LogWarning("sb_send_volume failed: {Error}", NativeMethods.GetLastError());
        else
            _logger.LogDebug("Volume set to {Volume}%", value);
    }

    partial void OnSelectedAudioModeChanged(int value)
    {
        if (_engine == IntPtr.Zero) return;
        int rc = NativeMethods.sb_set_audio_mode(_engine, value);
        if (rc != NativeMethods.SB_OK)
            _logger.LogWarning("sb_set_audio_mode failed: {Error}", NativeMethods.GetLastError());
        else
            _logger.LogInformation("Audio mode set to {Mode}", value switch
            {
                NativeMethods.SB_AUDIO_MODE_HIGH_QUALITY => "HighQuality",
                NativeMethods.SB_AUDIO_MODE_LOW_LATENCY => "LowLatency",
                _ => "Balanced"
            });
    }

    partial void OnMixRatioChanged(double value)
    {
        if (_engine == IntPtr.Zero) return;
        // value: 0=全PC音量, 100=全手机音量, 50=各50%
        float pcVol = (float)((100.0 - value) / 100.0);
        float phoneVol = (float)(value / 100.0);
        int rc = NativeMethods.sb_set_mix_ratio(_engine, pcVol, phoneVol);
        if (rc != NativeMethods.SB_OK)
            _logger.LogWarning("sb_set_mix_ratio failed: {Error}", NativeMethods.GetLastError());
        else
            _logger.LogDebug("Mix ratio set: PC={Pc:F2}, Phone={Phone:F2}", pcVol, phoneVol);
    }

    [RelayCommand]
    private void TogglePause()
    {
        if (_engine == IntPtr.Zero) return;

        if (IsPaused)
        {
            int rc = NativeMethods.sb_send_resume(_engine);
            if (rc == NativeMethods.SB_OK)
            {
                IsPaused = false;
                _logger.LogInformation("Audio resumed");
            }
            else
                _logger.LogWarning("sb_send_resume failed: {Error}", NativeMethods.GetLastError());
        }
        else
        {
            int rc = NativeMethods.sb_send_pause(_engine);
            if (rc == NativeMethods.SB_OK)
            {
                IsPaused = true;
                _logger.LogInformation("Audio paused");
            }
            else
                _logger.LogWarning("sb_send_pause failed: {Error}", NativeMethods.GetLastError());
        }
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

            // 连接失败时手动触发错误通知（回调可能不会触发）
            _notificationService?.ShowNotification(SbConnectionState.Error, $"连接失败: {ex.Message}");
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
        IsPaused = false;
        AudioLevel = 0;
        FramesCaptured = 0;
        FramesPlayed = 0;
        LatencyMs = 0;
        LossRate = 0;
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
                        out float latency,
                        out float lossRate);

                    if (rc == NativeMethods.SB_OK)
                    {
                        FramesCaptured = captured;
                        FramesPlayed = played;
                        LatencyMs = latency;
                        LossRate = lossRate;

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

        // 先取消状态回调注册
        _notificationService?.Unregister();

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

        if (_discovery != IntPtr.Zero)
        {
            NativeMethods.sb_discovery_close(_discovery);
            _discovery = IntPtr.Zero;
            _logger.LogInformation("Discovery service closed");
        }

        GC.SuppressFinalize(this);
    }

    ~MainWindowViewModel() => Dispose();
}
