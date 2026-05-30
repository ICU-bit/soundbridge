# Audio Playback Crate

## Purpose

音频播放模块，基于 cpal 库实现跨平台音频输出，支持混音播放。

## Current Status

- ✅ 基于 CPAL 的音频播放实现完成
- ✅ 设备枚举和选择功能
- ✅ 音频格式配置（48kHz, Mono, Float32）
- ✅ 混音输出支持
- ✅ 环形缓冲区用于音频数据缓存
- ✅ 测试用例通过

## Architecture

```
CpalPlayback        - CPAL 音频播放器
PlaybackConfig      - 播放配置（设备、格式、帧大小）
```

## Key Parameters

- Sample Rate: 48000 Hz
- Channels: Mono (1)
- Frame Size: 960 samples (20ms)
- Format: Float32

## Dependencies

- cpal = "0.15.2" (workspace)
- audio-core (workspace)
