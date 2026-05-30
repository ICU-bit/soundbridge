using System.Runtime.InteropServices;

namespace SoundBridge.UI;

/// <summary>
/// Rust FFI 绑定 - P/Invoke 声明
/// 对应 rust-core/crates/ffi-bindings/src/lib.rs
/// </summary>
internal static partial class NativeMethods
{
    private const string DllName = "ffi_bindings";

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
        out float latencyMs);

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
