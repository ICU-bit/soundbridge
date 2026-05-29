# Audio Codec Crate

## Purpose
Opus 音频编解码，为 SoundBridge 提供低延迟、高质量的音频压缩和解压。

## Current Status
- ✅ Opus 编码器实现完成
- ✅ Opus 解码器实现完成（已修复解码返回静音的 bug）
- ✅ Channels 命名冲突已修复（重命名为 ChannelConfig）
- ✅ decode_into 自引用拷贝已修复
- ✅ Stereo 编码统一使用 encode_vec（交错格式）
- ✅ 23 个单元测试通过
- ✅ Criterion 基准测试完整

## Architecture
```
OpusConfig          - 编解码配置（采样率、声道、比特率、帧大小）
ChannelConfig       - 声道配置（Mono/Stereo），与 opus::Channels 区分
OpusEncoderCodec    - 独立编码器
OpusDecoderCodec    - 独立解码器（decode + decode_into）
OpusCodec           - 综合编解码器
AudioCodec          - 兼容包装器
```

## API
```rust
// 编码
let mut codec = AudioCodec::new()?;
let encoded: Vec<u8> = codec.encode(&audio_buffer)?;

// 解码
let decoded: AudioBuffer<f32> = codec.decode(&encoded)?;

// 零拷贝解码
let mut output = vec![0f32; 960];
let count = decoder.decode_into(&encoded, &mut output)?;
```

## Next Steps
1. 性能基准测试对比优化
2. 考虑 SIMD 优化编码/解码路径
3. 添加重采样支持
4. 添加更多采样率支持（如 16000 Hz）

## Dependencies
- opus = "0.3"
- audio-core（workspace）
