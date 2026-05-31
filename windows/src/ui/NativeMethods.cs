using System.Runtime.InteropServices;

namespace SoundBridge.UI;

/// <summary>
/// Rust FFI 连接状态枚举（对应 SbConnectionState）
/// </summary>
public enum SbConnectionState : int
{
    Disconnected = 0,
    Connecting = 1,
    Connected = 2,
    Error = 3,
}

/// <summary>
/// 状态回调委托（对应 SbStateCallback）
/// </summary>
[UnmanagedFunctionPointer(CallingConvention.Cdecl)]
internal delegate void SbStateCallback(SbConnectionState state, IntPtr userData);

/// <summary>
/// Rust FFI 绑定 - P/Invoke 声明
/// 对应 rust-core/crates/ffi-bindings/src/lib.rs
/// </summary>
internal static partial class NativeMethods
{
    private const string DllName = "ffi_bindings";

    // ============================================================
    // 初始化
    // ============================================================

    /// <summary>初始化 FFI 库（设置 panic hook、tracing，必须在其他 sb_* 调用前调用一次）</summary>
    [DllImport(DllName, CallingConvention = CallingConvention.Cdecl)]
    internal static extern int sb_init();

    // ============================================================
    // 引擎生命周期
    // ============================================================

    /// <summary>创建引擎实例</summary>
    [DllImport(DllName, CallingConvention = CallingConvention.Cdecl)]
    internal static extern IntPtr sb_engine_create();

    /// <summary>销毁引擎实例</summary>
    [DllImport(DllName, CallingConvention = CallingConvention.Cdecl)]
    internal static extern void sb_engine_destroy(IntPtr engine);

    /// <summary>获取最后的错误信息</summary>
    [DllImport(DllName, CallingConvention = CallingConvention.Cdecl, CharSet = CharSet.Ansi)]
    internal static extern IntPtr sb_last_error();

    // ============================================================
    // 网络绑定
    // ============================================================

    /// <summary>绑定本地 UDP 端口（port=0 自动分配）</summary>
    [DllImport(DllName, CallingConvention = CallingConvention.Cdecl)]
    internal static extern int sb_bind(IntPtr engine, ushort port);

    /// <summary>设置目标地址（格式: "ip:port"）</summary>
    [DllImport(DllName, CallingConvention = CallingConvention.Cdecl, CharSet = CharSet.Ansi)]
    internal static extern int sb_connect(IntPtr engine, [MarshalAs(UnmanagedType.LPStr)] string addr);

    /// <summary>获取本地监听端口</summary>
    [DllImport(DllName, CallingConvention = CallingConvention.Cdecl)]
    internal static extern int sb_local_port(IntPtr engine, out ushort port);

    // ============================================================
    // 连接状态回调
    // ============================================================

    /// <summary>设置连接状态回调（状态变化时触发）</summary>
    [DllImport(DllName, CallingConvention = CallingConvention.Cdecl)]
    internal static extern int sb_set_state_callback(IntPtr engine, SbStateCallback? callback, IntPtr userData);

    /// <summary>获取当前连接状态</summary>
    [DllImport(DllName, CallingConvention = CallingConvention.Cdecl)]
    internal static extern int sb_get_connection_state(IntPtr engine, out SbConnectionState state);

    // ============================================================
    // 音频采集
    // ============================================================

    /// <summary>开始音频采集（deviceName=null 使用默认设备）</summary>
    [DllImport(DllName, CallingConvention = CallingConvention.Cdecl, CharSet = CharSet.Ansi)]
    internal static extern int sb_capture_start(IntPtr engine, IntPtr deviceName);

    /// <summary>停止音频采集</summary>
    [DllImport(DllName, CallingConvention = CallingConvention.Cdecl)]
    internal static extern int sb_capture_stop(IntPtr engine);

    /// <summary>读取音频数据</summary>
    [DllImport(DllName, CallingConvention = CallingConvention.Cdecl)]
    internal static extern int sb_capture_read(IntPtr engine, [Out] float[] buf, nuint len);

    /// <summary>获取采集设备数量</summary>
    [DllImport(DllName, CallingConvention = CallingConvention.Cdecl)]
    internal static extern int sb_capture_device_count(out nuint count);

    // ============================================================
    // 音频播放
    // ============================================================

    /// <summary>开始音频播放（deviceName=null 使用默认设备）</summary>
    [DllImport(DllName, CallingConvention = CallingConvention.Cdecl, CharSet = CharSet.Ansi)]
    internal static extern int sb_playback_start(IntPtr engine, IntPtr deviceName);

    /// <summary>停止音频播放</summary>
    [DllImport(DllName, CallingConvention = CallingConvention.Cdecl)]
    internal static extern int sb_playback_stop(IntPtr engine);

    /// <summary>写入音频数据</summary>
    [DllImport(DllName, CallingConvention = CallingConvention.Cdecl)]
    internal static extern int sb_playback_write(IntPtr engine, float[] buf, nuint len);

    /// <summary>获取播放设备数量</summary>
    [DllImport(DllName, CallingConvention = CallingConvention.Cdecl)]
    internal static extern int sb_playback_device_count(out nuint count);

    // ============================================================
    // 音频管线
    // ============================================================

    /// <summary>启动音频管线（采集→编码→发送 + 接收→解码→播放）</summary>
    [DllImport(DllName, CallingConvention = CallingConvention.Cdecl)]
    internal static extern int sb_pipeline_start(IntPtr engine);

    /// <summary>停止音频管线</summary>
    [DllImport(DllName, CallingConvention = CallingConvention.Cdecl)]
    internal static extern int sb_pipeline_stop(IntPtr engine);

    /// <summary>获取管线状态（0=Stopped, 1=Running, 2=Error）</summary>
    [DllImport(DllName, CallingConvention = CallingConvention.Cdecl)]
    internal static extern int sb_pipeline_state(IntPtr engine, out int state);

    /// <summary>获取管线统计</summary>
    [DllImport(DllName, CallingConvention = CallingConvention.Cdecl)]
    internal static extern int sb_pipeline_stats(
        IntPtr engine,
        out ulong framesCaptured,
        out ulong framesPlayed,
        out float latencyMs,
        out float lossRate);

    /// <summary>获取真实音频电平（RMS, 0.0-1.0）</summary>
    [DllImport(DllName, CallingConvention = CallingConvention.Cdecl)]
    internal static extern int sb_get_audio_level(IntPtr engine, out float level);

    /// <summary>设置 WASAPI 独占模式标志（影响延迟计算）</summary>
    [DllImport(DllName, CallingConvention = CallingConvention.Cdecl)]
    internal static extern int sb_set_exclusive_mode(IntPtr engine, [MarshalAs(UnmanagedType.U1)] bool exclusive);

    // ============================================================
    // 音量控制 / 暂停恢复
    // ============================================================

    /// <summary>设置发送音量（0.0 ~ 1.0）</summary>
    [DllImport(DllName, CallingConvention = CallingConvention.Cdecl)]
    internal static extern int sb_send_volume(IntPtr engine, float volume);

    /// <summary>暂停音频管线发送</summary>
    [DllImport(DllName, CallingConvention = CallingConvention.Cdecl)]
    internal static extern int sb_send_pause(IntPtr engine);

    /// <summary>恢复音频管线发送</summary>
    [DllImport(DllName, CallingConvention = CallingConvention.Cdecl)]
    internal static extern int sb_send_resume(IntPtr engine);

    /// <summary>设置静音状态（1=静音, 0=取消静音）</summary>
    [DllImport(DllName, CallingConvention = CallingConvention.Cdecl)]
    internal static extern int sb_set_mute(IntPtr engine, int muted);

    /// <summary>获取静音状态（1=静音, 0=非静音, 负数=错误）</summary>
    [DllImport(DllName, CallingConvention = CallingConvention.Cdecl)]
    internal static extern int sb_get_mute(IntPtr engine);

    // ============================================================
    // 音频处理
    // ============================================================

    /// <summary>处理音频数据（就地修改）</summary>
    [DllImport(DllName, CallingConvention = CallingConvention.Cdecl)]
    internal static extern int sb_processor_process(IntPtr engine, [In, Out] float[] buf, nuint len);

    /// <summary>混音多路音频</summary>
    [DllImport(DllName, CallingConvention = CallingConvention.Cdecl)]
    internal static extern int sb_mixer_mix(
        IntPtr engine,
        IntPtr[] inputs,
        nuint[] inputLens,
        float[] volumes,
        nuint inputCount,
        [Out] float[] output,
        nuint outputLen);

    // ============================================================
    // 音频模式
    // ============================================================

    /// <summary>设置音频模式（0=Balanced, 1=HighQuality, 2=LowLatency）</summary>
    [DllImport(DllName, CallingConvention = CallingConvention.Cdecl)]
    internal static extern int sb_set_audio_mode(IntPtr engine, int mode);

    /// <summary>获取当前音频模式</summary>
    [DllImport(DllName, CallingConvention = CallingConvention.Cdecl)]
    internal static extern int sb_get_audio_mode(IntPtr engine, out int mode);

    // ============================================================
    // 音质档位
    // ============================================================

    /// <summary>音质档位枚举</summary>
    internal enum SbAudioProfile : uint
    {
        BandwidthSaving = 0,
        Standard = 1,
        HighQuality = 2,
        Lossless = 3,
        HighResolution = 4,
        StudioMaster = 5,
        Auto = 6,
        Custom = 7,
    }

    /// <summary>均衡器预设枚举</summary>
    internal enum SbEqPreset : uint
    {
        Flat = 0,
        Gaming = 1,
        Music = 2,
        Voice = 3,
        Bass = 4,
        Treble = 5,
    }

    /// <summary>设置音质档位</summary>
    [DllImport(DllName, CallingConvention = CallingConvention.Cdecl)]
    internal static extern int sb_set_audio_profile(SbAudioProfile profile);

    /// <summary>获取当前音质档位</summary>
    [DllImport(DllName, CallingConvention = CallingConvention.Cdecl)]
    internal static extern SbAudioProfile sb_get_audio_profile();

    /// <summary>设置通道数</summary>
    [DllImport(DllName, CallingConvention = CallingConvention.Cdecl)]
    internal static extern int sb_set_channels(uint channels);

    /// <summary>获取通道数</summary>
    [DllImport(DllName, CallingConvention = CallingConvention.Cdecl)]
    internal static extern uint sb_get_channels();

    /// <summary>设置均衡器频段增益</summary>
    [DllImport(DllName, CallingConvention = CallingConvention.Cdecl)]
    internal static extern int sb_set_eq_band(uint band, float gainDb, float q);

    /// <summary>设置均衡器预设</summary>
    [DllImport(DllName, CallingConvention = CallingConvention.Cdecl)]
    internal static extern int sb_set_eq_preset(SbEqPreset preset);

    /// <summary>启用/禁用均衡器</summary>
    [DllImport(DllName, CallingConvention = CallingConvention.Cdecl)]
    internal static extern int sb_set_eq_enabled(int enabled);

    // ============================================================
    // 混音比例
    // ============================================================

    /// <summary>设置混音比例（pcVolume 和 phoneVolume 范围 0.0~1.0）</summary>
    [DllImport(DllName, CallingConvention = CallingConvention.Cdecl)]
    internal static extern int sb_set_mix_ratio(IntPtr engine, float pcVolume, float phoneVolume);

    /// <summary>获取混音比例</summary>
    [DllImport(DllName, CallingConvention = CallingConvention.Cdecl)]
    internal static extern int sb_get_mix_ratio(IntPtr engine, out float pcVolume, out float phoneVolume);

    // ============================================================
    // 错误码
    // ============================================================

    internal const int SB_OK = 0;
    internal const int SB_ERROR = -1;
    internal const int SB_INVALID_ARGUMENT = -2;
    internal const int SB_DEVICE_NOT_FOUND = -3;
    internal const int SB_CONFIG_ERROR = -4;
    internal const int SB_STREAM_ERROR = -5;
    internal const int SB_CODEC_ERROR = -6;
    internal const int SB_NETWORK_ERROR = -7;
    internal const int SB_PIPELINE_NOT_READY = -8;

    // ============================================================
    // 音频模式常量
    // ============================================================

    internal const int SB_AUDIO_MODE_BALANCED = 0;
    internal const int SB_AUDIO_MODE_HIGH_QUALITY = 1;
    internal const int SB_AUDIO_MODE_LOW_LATENCY = 2;

    // ============================================================
    // 管线状态
    // ============================================================

    internal const int SB_PIPELINE_STOPPED = 0;
    internal const int SB_PIPELINE_RUNNING = 1;
    internal const int SB_PIPELINE_ERROR = 2;

    /// <summary>获取最后错误的文本</summary>
    internal static string? GetLastError()
    {
        IntPtr ptr = sb_last_error();
        return ptr == IntPtr.Zero ? null : Marshal.PtrToStringAnsi(ptr);
    }

    // ============================================================
    // 设备存储（DeviceStore）
    // ============================================================

    /// <summary>打开设备存储（JSON 文件持久化）</summary>
    [DllImport(DllName, CallingConvention = CallingConvention.Cdecl, CharSet = CharSet.Ansi)]
    internal static extern IntPtr sb_device_store_open([MarshalAs(UnmanagedType.LPStr)] string path);

    /// <summary>关闭设备存储</summary>
    [DllImport(DllName, CallingConvention = CallingConvention.Cdecl)]
    internal static extern void sb_device_store_close(IntPtr store);

    /// <summary>添加或更新设备</summary>
    [DllImport(DllName, CallingConvention = CallingConvention.Cdecl, CharSet = CharSet.Ansi)]
    internal static extern int sb_device_store_add(
        IntPtr store,
        [MarshalAs(UnmanagedType.LPStr)] string name,
        [MarshalAs(UnmanagedType.LPStr)] string address,
        ushort port);

    /// <summary>删除设备</summary>
    [DllImport(DllName, CallingConvention = CallingConvention.Cdecl, CharSet = CharSet.Ansi)]
    internal static extern int sb_device_store_remove(IntPtr store, [MarshalAs(UnmanagedType.LPStr)] string name);

    /// <summary>设置设备自动连接</summary>
    [DllImport(DllName, CallingConvention = CallingConvention.Cdecl, CharSet = CharSet.Ansi)]
    internal static extern int sb_device_store_set_auto_connect(
        IntPtr store,
        [MarshalAs(UnmanagedType.LPStr)] string name,
        [MarshalAs(UnmanagedType.I1)] bool autoConnect);

    /// <summary>获取设备数量</summary>
    [DllImport(DllName, CallingConvention = CallingConvention.Cdecl)]
    internal static extern int sb_device_store_count(IntPtr store, out nuint count);

    /// <summary>检查设备是否存在（返回 1=true, 0=false）</summary>
    [DllImport(DllName, CallingConvention = CallingConvention.Cdecl, CharSet = CharSet.Ansi)]
    internal static extern int sb_device_store_has(IntPtr store, [MarshalAs(UnmanagedType.LPStr)] string name);

    /// <summary>清除所有设备</summary>
    [DllImport(DllName, CallingConvention = CallingConvention.Cdecl)]
    internal static extern void sb_device_store_clear(IntPtr store);

    /// <summary>获取设备地址（写入 buf，返回写入字节数，-1=未找到）</summary>
    [DllImport(DllName, CallingConvention = CallingConvention.Cdecl, CharSet = CharSet.Ansi)]
    internal static extern int sb_device_store_get_address(
        IntPtr store,
        [MarshalAs(UnmanagedType.LPStr)] string name,
        [Out] byte[] buf,
        nuint bufLen);

    /// <summary>获取设备端口</summary>
    [DllImport(DllName, CallingConvention = CallingConvention.Cdecl, CharSet = CharSet.Ansi)]
    internal static extern int sb_device_store_get_port(
        IntPtr store,
        [MarshalAs(UnmanagedType.LPStr)] string name,
        out ushort port);

    /// <summary>获取第 N 个设备的名称（返回写入字节数，-1=越界）</summary>
    [DllImport(DllName, CallingConvention = CallingConvention.Cdecl)]
    internal static extern int sb_device_store_get_name_at(
        IntPtr store,
        nuint index,
        [Out] byte[] buf,
        nuint bufLen);

    // ============================================================
    // 设备发现（DeviceDiscovery）
    // ============================================================

    /// <summary>创建设备发现服务</summary>
    [DllImport(DllName, CallingConvention = CallingConvention.Cdecl)]
    internal static extern IntPtr sb_discovery_create();

    /// <summary>关闭设备发现服务</summary>
    [DllImport(DllName, CallingConvention = CallingConvention.Cdecl)]
    internal static extern void sb_discovery_close(IntPtr discovery);

    /// <summary>初始化 mDNS 守护进程</summary>
    [DllImport(DllName, CallingConvention = CallingConvention.Cdecl)]
    internal static extern int sb_discovery_init(IntPtr discovery);

    /// <summary>注册本设备到 mDNS 网络</summary>
    [DllImport(DllName, CallingConvention = CallingConvention.Cdecl, CharSet = CharSet.Ansi)]
    internal static extern int sb_discovery_register(IntPtr discovery, [MarshalAs(UnmanagedType.LPStr)] string name, ushort port);

    /// <summary>发现网络上的设备（返回设备数量，devicesBuf 可为 null）</summary>
    [DllImport(DllName, CallingConvention = CallingConvention.Cdecl)]
    internal static extern int sb_discovery_find_devices(IntPtr discovery, IntPtr devicesBuf, nuint bufSize);

    /// <summary>释放发现的设备信息</summary>
    [DllImport(DllName, CallingConvention = CallingConvention.Cdecl)]
    internal static extern void sb_discovery_free_device_info(IntPtr deviceInfo);

    // ============================================================
    // Win32 菜单 API（托盘图标上下文菜单）
    // ============================================================

    [StructLayout(LayoutKind.Sequential)]
    internal struct POINT
    {
        public int X;
        public int Y;
    }

    /// <summary>创建弹出菜单</summary>
    [DllImport("user32.dll", SetLastError = true)]
    internal static extern IntPtr CreatePopupMenu();

    /// <summary>追加菜单项</summary>
    [DllImport("user32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    internal static extern bool AppendMenu(IntPtr hMenu, uint uFlags, uint uIDNewItem, string lpNewItem);

    /// <summary>获取鼠标位置</summary>
    [DllImport("user32.dll")]
    internal static extern bool GetCursorPos(out POINT lpPoint);

    /// <summary>显示弹出菜单</summary>
    [DllImport("user32.dll")]
    internal static extern uint TrackPopupMenu(
        IntPtr hMenu,
        uint uFlags,
        int x,
        int y,
        int nReserved,
        IntPtr hWnd,
        IntPtr prcRect);

    /// <summary>销毁菜单</summary>
    [DllImport("user32.dll")]
    internal static extern bool DestroyMenu(IntPtr hMenu);
}
