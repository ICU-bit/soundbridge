using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;

namespace SoundBridge.UI;

public sealed partial class MainWindow : Window
{
    public MainWindowViewModel ViewModel { get; }

    public MainWindow(MainWindowViewModel viewModel)
    {
        ViewModel = viewModel;
        InitializeComponent();

        // 设置窗口标题栏
        Title = "SoundBridge";
    }
}
