# Audio Processor Crate

## Purpose

音频处理模块，实现 AEC（回声消除）、NS（噪声抑制）、AGC（自动增益控制）。

## Current Status

- ✅ 回声消除（AEC）- NLMS 自适应滤波器
- ✅ 噪声抑制（NS）- SNR 估计
- ✅ 自动增益控制（AGC）- 攻击/释放时间平滑
- ✅ 可插拔的处理器链
- ✅ 测试用例通过

## Architecture

```
NlmsAec             - NLMS 自适应滤波回声消除
SnrNoiseSuppressor  - SNR 估计噪声抑制
AttackReleaseAgc    - 攻击/释放时间自动增益控制
AudioProcessor      - 处理器链组合
```

## Algorithms

- AEC: Normalized LMS (NLMS) adaptive filter
- NS: SNR-based noise estimation
- AGC: Attack/Release time smoothing

## Dependencies

- audio-core (workspace)
