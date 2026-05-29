using Microsoft.UI;
using Microsoft.UI.Xaml.Data;
using Microsoft.UI.Xaml.Media;
using System;

namespace SoundBridge.UI;

/// <summary>
/// bool → 连接状态背景色
/// </summary>
public class BoolToColorConverter : IValueConverter
{
    public object Convert(object value, Type targetType, object parameter, string language)
    {
        bool isConnected = value is bool b && b;
        return isConnected
            ? new SolidColorBrush(Colors.ForestGreen)
            : new SolidColorBrush(Colors.IndianRed);
    }

    public object ConvertBack(object value, Type targetType, object parameter, string language) =>
        throw new NotImplementedException();
}

/// <summary>
/// bool → 状态指示灯颜色
/// </summary>
public class BoolToStatusColorConverter : IValueConverter
{
    public object Convert(object value, Type targetType, object parameter, string language)
    {
        bool isConnected = value is bool b && b;
        return isConnected
            ? new SolidColorBrush(Colors.LightGreen)
            : new SolidColorBrush(Colors.Red);
    }

    public object ConvertBack(object value, Type targetType, object parameter, string language) =>
        throw new NotImplementedException();
}

/// <summary>
/// float → 音频电平颜色（绿/黄/红）
/// </summary>
public class LevelToColorConverter : IValueConverter
{
    public object Convert(object value, Type targetType, object parameter, string language)
    {
        float level = value is float f ? f : 0f;
        if (level < 0.3f) return new SolidColorBrush(Colors.LimeGreen);
        if (level < 0.7f) return new SolidColorBrush(Colors.Gold);
        return new SolidColorBrush(Colors.Red);
    }

    public object ConvertBack(object value, Type targetType, object parameter, string language) =>
        throw new NotImplementedException();
}

/// <summary>
/// float → 百分比文本
/// </summary>
public class LevelToPercentConverter : IValueConverter
{
    public object Convert(object value, Type targetType, object parameter, string language)
    {
        float level = value is float f ? f : 0f;
        return $"{(int)(level * 100)}%";
    }

    public object ConvertBack(object value, Type targetType, object parameter, string language) =>
        throw new NotImplementedException();
}

/// <summary>
/// bool → 连接/断开图标字形
/// </summary>
public class BoolToIconConverter : IValueConverter
{
    public object Convert(object value, Type targetType, object parameter, string language)
    {
        bool isConnected = value is bool b && b;
        return isConnected ? "\uE711" : "\uE768"; // Close / Play
    }

    public object ConvertBack(object value, Type targetType, object parameter, string language) =>
        throw new NotImplementedException();
}

/// <summary>
/// bool → "Disconnect" / "Connect"
/// </summary>
public class BoolToConnectTextConverter : IValueConverter
{
    public object Convert(object value, Type targetType, object parameter, string language)
    {
        bool isConnected = value is bool b && b;
        return isConnected ? "Disconnect" : "Connect";
    }

    public object ConvertBack(object value, Type targetType, object parameter, string language) =>
        throw new NotImplementedException();
}

/// <summary>
/// bool → 静音图标字形
/// </summary>
public class BoolToMuteIconConverter : IValueConverter
{
    public object Convert(object value, Type targetType, object parameter, string language)
    {
        bool isMuted = value is bool b && b;
        return isMuted ? "\uE720" : "\uE729"; // MicOff / Mic
    }

    public object ConvertBack(object value, Type targetType, object parameter, string language) =>
        throw new NotImplementedException();
}

/// <summary>
/// bool → "Unmute" / "Mute"
/// </summary>
public class BoolToMuteTextConverter : IValueConverter
{
    public object Convert(object value, Type targetType, object parameter, string language)
    {
        bool isMuted = value is bool b && b;
        return isMuted ? "Unmute" : "Mute";
    }

    public object ConvertBack(object value, Type targetType, object parameter, string language) =>
        throw new NotImplementedException();
}
