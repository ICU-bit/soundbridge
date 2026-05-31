using Microsoft.Extensions.Logging;
using System;
using System.Collections.Generic;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Interop;

namespace SoundBridge.UI;

public partial class MainWindow : Window
{
    public MainWindowViewModel ViewModel { get; }
    private readonly ILogger<MainWindow> _logger;
    private readonly ILoggerFactory _loggerFactory;
    private HotkeyManager? _hotkeyManager;

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
        _loggerFactory = loggerFactory;
        InitializeComponent();

        Title = "SoundBridge";
        DataContext = ViewModel;

        this.Closed += OnMainWindowClosed;
        this.SourceInitialized += OnSourceInitialized;
    }

    private void OnSourceInitialized(object? sender, EventArgs e)
    {
        var hWnd = new WindowInteropHelper(this).Handle;
        var logger = _loggerFactory.CreateLogger<HotkeyManager>();
        _hotkeyManager = new HotkeyManager(hWnd, logger);

        _hotkeyManager.Register(HotkeyManager.MOD_CONTROL | HotkeyManager.MOD_ALT, 0x54,
            async () => { try { await ViewModel.ToggleConnectionCommand.ExecuteAsync(null); } catch (Exception ex) { _logger.LogError(ex, "Hotkey failed"); } });
        _hotkeyManager.Register(HotkeyManager.MOD_CONTROL | HotkeyManager.MOD_ALT, 0x4D,
            () => ViewModel.ToggleMuteCommand.Execute(null));
        _hotkeyManager.Register(HotkeyManager.MOD_CONTROL | HotkeyManager.MOD_ALT, 0x53,
            () => this.Activate());
    }

    private void DeviceList_SelectionChanged(object sender, SelectionChangedEventArgs e)
    {
        if (sender is ListBox listBox && listBox.SelectedItem is string deviceJson)
        {
            ViewModel.SelectDeviceCommand.Execute(deviceJson);
        }
    }

    private void AudioProfileComboBox_SelectionChanged(object sender, SelectionChangedEventArgs e)
    {
        if (AudioProfileComboBox.SelectedIndex < 0) return;
        var profile = (NativeMethods.SbAudioProfile)AudioProfileComboBox.SelectedIndex;
        NativeMethods.sb_set_audio_profile(profile);
    }

    private void AutoProfileToggle_Changed(object sender, RoutedEventArgs e)
    {
        if (AutoProfileToggle.IsChecked == true)
        {
            NativeMethods.sb_set_audio_profile(NativeMethods.SbAudioProfile.Auto);
            AudioProfileComboBox.SelectedIndex = (int)NativeMethods.SbAudioProfile.Auto;
        }
    }

    private void EqPresetComboBox_SelectionChanged(object sender, SelectionChangedEventArgs e)
    {
        if (EqPresetComboBox.SelectedIndex < 0) return;
        var preset = (NativeMethods.SbEqPreset)EqPresetComboBox.SelectedIndex;
        NativeMethods.sb_set_eq_preset(preset);
    }

    private void EqBand_ValueChanged(object sender, RoutedPropertyChangedEventArgs<double> e)
    {
        if (sender is not Slider slider) return;
        var name = slider.Name;
        if (!name.StartsWith("EqBand")) return;
        if (!int.TryParse(name.Substring("EqBand".Length), out var band)) return;
        NativeMethods.sb_set_eq_band((uint)band, (float)slider.Value, 1.0f);
    }

    private void OnMainWindowClosed(object? sender, EventArgs e)
    {
        _hotkeyManager?.Dispose();
        _hotkeyManager = null;
    }
}
