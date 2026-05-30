# Audio Mixer Crate

## Purpose

音频混音模块，支持多音频流混合、音量控制和软削波。

## Current Status

- ✅ 多音频流混合实现完成
- ✅ 音量控制（PC/手机独立音量）
- ✅ 静音支持
- ✅ 声道混合（Mono → Stereo）
- ✅ soft_clip (tanh) 软削波防止溢出
- ✅ Clone trait 实现
- ✅ 测试用例通过

## Architecture

```
AudioMixer          - 混音引擎
MixConfig           - 混音配置（PC音量、手机音量）
```

## API

```rust
use audio_mixer::AudioMixer;

let mut mixer = AudioMixer::new(pc_volume, phone_volume)?;
mixer.mix_two_into(&local_buf, &remote_buf, &mut output)?;
mixer.set_pc_volume(0.8)?;
mixer.set_phone_volume(1.0)?;
```

## Dependencies

- audio-core (workspace)
