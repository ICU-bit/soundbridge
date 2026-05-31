using Microsoft.Extensions.Logging;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using System;
using System.Collections.Generic;

namespace SoundBridge.UI;

public sealed partial class MainWindow : Window
{
    public MainWindowViewModel ViewModel { get; }
    private readonly ILogger<MainWindow> _logger;
    private HotkeyManager? _hotkeyManager;

    /// <summary>音频模式选项（索引对应 SbAudioMode 枚举值）</summary>
    public IReadOnlyList<string> AudioModeOptions { get; } = new[]
    {
        "Balanced",
        "High Quality",
        "Low Latency"
    };

    public MainWindow(MainWindowViewModel viewModel, ILoggerFactory loggerFactory)
    {
        ViewModel = viewModel;
        _logger = loggerFactory.CreateLogger<MainWindow>();
        InitializeComponent();

        // 设置窗口标题栏
        Title = "SoundBridge";

        // 初始化全局快捷键
        InitializeHotkeys(loggerFactory);
    }

    private void InitializeHotkeys(ILoggerFactory loggerFactory)
    {
        var hWnd = WinRT.Interop.WindowNative.GetWindowHandle(this);
        var logger = loggerFactory.CreateLogger<HotkeyManager>();
        _hotkeyManager = new HotkeyManager(hWnd, logger);

        // Ctrl+Alt+T → 连接/断开
        _hotkeyManager.Register(
            HotkeyManager.MOD_CONTROL | HotkeyManager.MOD_ALT,
            0x54, // 'T'
            async () =>
            {
                try
                {
                    await ViewModel.ToggleConnectionCommand.ExecuteAsync(null);
                }
                catch (Exception ex)
                {
                    _logger.LogError(ex, "Hotkey connect toggle failed");
                }
            });

        // Ctrl+Alt+M → 静音切换
        _hotkeyManager.Register(
            HotkeyManager.MOD_CONTROL | HotkeyManager.MOD_ALT,
            0x4D, // 'M'
            () => ViewModel.ToggleMuteCommand.Execute(null));

        // Ctrl+Alt+S → 显示窗口
        _hotkeyManager.Register(
            HotkeyManager.MOD_CONTROL | HotkeyManager.MOD_ALT,
            0x53, // 'S'
            () => this.Activate());
    }

    /// <summary>设备列表选择变更 → 填入服务器地址和端口</summary>
    private void DeviceList_SelectionChanged(object sender, SelectionChangedEventArgs e)
    {
        if (sender is ListView listView && listView.SelectedItem is string deviceJson)
        {
            ViewModel.SelectDeviceCommand.Execute(deviceJson);
        }
    }

    /// <summary>音质档位变更</summary>
    private void AudioProfileComboBox_SelectionChanged(object sender, SelectionChangedEventArgs e)
    {
        if (AudioProfileComboBox.SelectedIndex < 0) return;
        var profile = (NativeMethods.SbAudioProfile)AudioProfileComboBox.SelectedIndex;
        NativeMethods.sb_set_audio_profile(profile);
        _logger.LogInformation("Audio profile changed to {Profile}", profile);
    }

    /// <summary>自动档切换</summary>
    private void AutoProfileToggle_Toggled(object sender, RoutedEventArgs e)
    {
        if (AutoProfileToggle.IsOn)
        {
            NativeMethods.sb_set_audio_profile(NativeMethods.SbAudioProfile.Auto);
            AudioProfileComboBox.SelectedIndex = (int)NativeMethods.SbAudioProfile.Auto;
            _logger.LogInformation("Auto profile enabled");
        }
    }

    /// <summary>均衡器预设变更</summary>
    private void EqPresetComboBox_SelectionChanged(object sender, SelectionChangedEventArgs e)
    {
        if (EqPresetComboBox.SelectedIndex < 0) return;
        var preset = (NativeMethods.SbEqPreset)EqPresetComboBox.SelectedIndex;
        NativeMethods.sb_set_eq_preset(preset);
        _logger.LogInformation("EQ preset changed to {Preset}", preset);
    }

    /// <summary>均衡器频段增益变更</summary>
    private void EqBand_ValueChanged(object sender, Microsoft.UI.Xaml.Controls.Primitives.RangeBaseValueChangedEventArgs e)
    {
        if (sender is not Slider slider) return;
        var name = slider.Name;
        if (!name.StartsWith("EqBand")) return;
        if (!int.TryParse(name.Substring("EqBand".Length), out var band)) return;
        NativeMethods.sb_set_eq_band((uint)band, (float)slider.Value, 1.0f);
    }

    protected override IntPtr WndProc(IntPtr hWnd, uint msg, IntPtr wParam, IntPtr lParam, ref bool handled)
    {
        // 处理全局快捷键消息
        if (_hotkeyManager?.HandleWindowMessage(hWnd, msg, wParam, lParam) == true)
        {
            handled = true;
        }

        return base.WndProc(hWnd, msg, wParam, lParam, ref handled);
    }

    protected override void OnClosed(object sender, WindowEventArgs args)
    {
        // 释放全局快捷键
        _hotkeyManager?.Dispose();
        _hotkeyManager = null;
        _logger.LogInformation("MainWindow closed, hotkeys unregistered");

        base.OnClosed(sender, args);
    }
}
