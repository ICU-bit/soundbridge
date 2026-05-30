using System;
using System.Runtime.InteropServices;
using Microsoft.UI.Xaml;

namespace SoundBridge.UI;

/// <summary>
/// Windows 系统托盘图标（Shell_NotifyIcon P/Invoke）
/// </summary>
public sealed class TrayIcon : IDisposable
{
    // ============================================================
    // P/Invoke
    // ============================================================

    private const int NIM_ADD = 0x00000000;
    private const int NIM_MODIFY = 0x00000001;
    private const int NIM_DELETE = 0x00000002;
    private const int NIF_MESSAGE = 0x00000001;
    private const int NIF_ICON = 0x00000002;
    private const int NIF_TIP = 0x00000004;
    private const int NIF_INFO = 0x00000010;

    private const int WM_TRAYICON = 0x0400 + 1; // WM_USER + 1
    private const int WM_LBUTTONUP = 0x0202;
    private const int WM_RBUTTONUP = 0x0205;

    private const int NIIF_NONE = 0x00000000;
    private const int NIIF_INFO = 0x00000001;
    private const int NIIF_WARNING = 0x00000002;
    private const int NIIF_ERROR = 0x00000003;

    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
    private struct NOTIFYICONDATA
    {
        public int cbSize;
        public IntPtr hWnd;
        public int uID;
        public int uFlags;
        public int uCallbackMessage;
        public IntPtr hIcon;
        [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 128)]
        public string szTip;
        public int dwState;
        public int dwStateMask;
        [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 256)]
        public string szInfo;
        public int uTimeoutOrVersion;
        [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 64)]
        public string szInfoTitle;
        public int dwInfoFlags;
        public Guid guidItem;
        public IntPtr hBalloonIcon;
    }

    [DllImport("shell32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    private static extern bool Shell_NotifyIcon(int dwMessage, ref NOTIFYICONDATA lpData);

    [DllImport("user32.dll")]
    private static extern IntPtr LoadIcon(IntPtr hInstance, IntPtr lpIconName);

    [DllImport("user32.dll")]
    private static extern bool DestroyIcon(IntPtr hIcon);

    private static readonly IntPtr IDI_APPLICATION = new(32512);

    // ============================================================
    // Fields
    // ============================================================

    private NOTIFYICONDATA _nid;
    private bool _added;
    private bool _disposed;
    private IntPtr _hWnd;
    private Action? _onDoubleClick;
    private Action? _onRightClick;

    // ============================================================
    // Events
    // ============================================================

    /// <summary>双击托盘图标（或左键单击弹起）</summary>
    public event Action? DoubleClick;

    /// <summary>右键单击托盘图标</summary>
    public event Action? RightClick;

    // ============================================================
    // Constructor
    // ============================================================

    /// <summary>创建托盘图标</summary>
    /// <param name="hWnd">窗口句柄（用于接收回调消息）</param>
    /// <param name="tooltip">鼠标悬停提示文本</param>
    public TrayIcon(IntPtr hWnd, string tooltip = "SoundBridge")
    {
        _hWnd = hWnd;
        _onDoubleClick = () => DoubleClick?.Invoke();
        _onRightClick = () => RightClick?.Invoke();

        _nid = new NOTIFYICONDATA
        {
            cbSize = Marshal.SizeOf<NOTIFYICONDATA>(),
            hWnd = hWnd,
            uID = 1,
            uFlags = NIF_ICON | NIF_TIP | NIF_MESSAGE,
            uCallbackMessage = WM_TRAYICON,
            hIcon = LoadIcon(IntPtr.Zero, IDI_APPLICATION),
            szTip = tooltip,
        };
    }

    // ============================================================
    // Public Methods
    // ============================================================

    /// <summary>显示托盘图标</summary>
    public void Show()
    {
        if (_added) return;
        _added = Shell_NotifyIcon(NIM_ADD, ref _nid);
    }

    /// <summary>隐藏托盘图标</summary>
    public void Hide()
    {
        if (!_added) return;
        Shell_NotifyIcon(NIM_DELETE, ref _nid);
        _added = false;
    }

    /// <summary>更新提示文本</summary>
    public void SetTooltip(string tooltip)
    {
        _nid.szTip = tooltip;
        if (_added)
            Shell_NotifyIcon(NIM_MODIFY, ref _nid);
    }

    /// <summary>显示气泡通知</summary>
    /// <param name="title">标题</param>
    /// <param name="message">消息内容</param>
    /// <param name="timeoutMs">显示时长（毫秒）</param>
    public void ShowBalloon(string title, string message, int timeoutMs = 3000)
    {
        _nid.uFlags |= NIF_INFO;
        _nid.szInfoTitle = title;
        _nid.szInfo = message;
        _nid.uTimeoutOrVersion = timeoutMs;
        _nid.dwInfoFlags = NIIF_INFO;

        if (_added)
            Shell_NotifyIcon(NIM_MODIFY, ref _nid);

        // 清除气泡标志（下次修改时生效）
        _nid.uFlags &= ~NIF_INFO;
    }

    /// <summary>
    /// 处理窗口消息（在 WindowWndProc 中调用）
    /// 返回 true 表示消息已处理
    /// </summary>
    public bool HandleWindowMessage(IntPtr hWnd, uint msg, IntPtr wParam, IntPtr lParam)
    {
        if (msg != WM_TRAYICON) return false;

        int mouseMsg = lParam.ToInt32();
        switch (mouseMsg)
        {
            case WM_LBUTTONUP:
                _onDoubleClick?.Invoke();
                break;
            case WM_RBUTTONUP:
                _onRightClick?.Invoke();
                break;
        }

        return true;
    }

    // ============================================================
    // Dispose
    // ============================================================

    public void Dispose()
    {
        if (_disposed) return;
        _disposed = true;

        Hide();

        if (_nid.hIcon != IntPtr.Zero)
        {
            DestroyIcon(_nid.hIcon);
            _nid.hIcon = IntPtr.Zero;
        }
    }
}
