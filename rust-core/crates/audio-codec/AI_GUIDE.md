# Audio Codec Crate

## Purpose
Opus 音频编解码，为 SoundBridge 提供低延迟、高质量的音频压缩和解压。

## Current Status
- ✅ Opus 编码器实现完成（统一 encode_vec 路径）
- ✅ Opus 解码器实现完成（使用 decode_float 直接写入）
- ✅ decode_into 零拷贝解码（无额外堆分配）
- ✅ ChannelConfig 与 opus::Channels 无命名冲突
- ✅ 编码逻辑已提取为公共方法 encode_samples
- ✅ OpusConfig derives Copy（所有字段枚举均为 Copy）
- ✅ 22 个单元测试通过（含 stereo 全覆盖 + 非静音断言）
- ✅ Criterion 基准测试完整

## Architecture
```
OpusConfig          - 编解码配置（采样率、声道、比特率、帧大小）
ChannelConfig       - 声道配置（Mono/Stereo）
OpusEncoderCodec    - 独立编码器（encode + encode_interleaved）
OpusDecoderCodec    - 独立解码器（decode + decode_into 零拷贝）
OpusCodec           - 综合编解码器（组合 encoder + decoder）
AudioCodec          - 兼容包装器
```

## API
```rust
let mut codec = AudioCodec::new()?;
let encoded: Vec<u8> = codec.encode(&audio_buffer)?;
let decoded: AudioBuffer<f32> = codec.decode(&encoded)?;

// 零拷贝解码（直接写入预分配 buffer）
let mut output = vec![0f32; 960];
let count = decoder.decode_into(&encoded, &mut output)?;
```

## Dependencies
- opus = "0.3"
- audio-core（workspace）
