//! Jitter Buffer 实现
//!
//! 用于缓冲网络音频包，处理乱序和延迟抖动。

use std::collections::BTreeMap;

/// Jitter Buffer 配置
#[derive(Debug, Clone)]
pub struct JitterBufferConfig {
    /// 目标延迟（毫秒）
    pub target_delay_ms: u32,

    /// 最小延迟（毫秒）
    pub min_delay_ms: u32,

    /// 最大延迟（毫秒）
    pub max_delay_ms: u32,

    /// 最大缓冲包数
    pub max_packets: usize,
}

impl Default for JitterBufferConfig {
    fn default() -> Self {
        Self {
            target_delay_ms: 40,
            min_delay_ms: 20,
            max_delay_ms: 200,
            max_packets: 100,
        }
    }
}

/// 音频数据包
#[derive(Debug, Clone)]
pub struct AudioPacket {
    /// 序列号
    pub sequence: u32,

    /// 音频数据
    pub data: Vec<f32>,
}

/// Jitter Buffer
pub struct JitterBuffer {
    /// 缓冲区（按序列号排序）
    buffer: BTreeMap<u32, AudioPacket>,

    /// 配置
    config: JitterBufferConfig,

    /// 下一个期望的序列号
    next_sequence: u32,
}

impl JitterBuffer {
    /// 创建新的 Jitter Buffer
    pub fn new(config: JitterBufferConfig) -> Self {
        Self {
            buffer: BTreeMap::new(),
            config,
            next_sequence: 0,
        }
    }

    /// 使用默认配置创建
    pub fn with_default_config() -> Self {
        Self::new(JitterBufferConfig::default())
    }

    /// 推入数据包
    pub fn push(&mut self, sequence: u32, data: Vec<f32>) {
        // 如果缓冲区满了，丢弃最旧的包
        if self.buffer.len() >= self.config.max_packets {
            if let Some((&oldest_seq, _)) = self.buffer.iter().next() {
                self.buffer.remove(&oldest_seq);
            }
        }

        self.buffer.insert(sequence, AudioPacket { sequence, data });
    }

    /// 弹出数据包
    pub fn pop(&mut self) -> Option<AudioPacket> {
        // 检查是否有下一个期望的包
        if let Some(packet) = self.buffer.remove(&self.next_sequence) {
            self.next_sequence += 1;
            Some(packet)
        } else if !self.buffer.is_empty() {
            // 如果没有期望的包，但有其他包，跳过缺失的包
            if let Some((&seq, _)) = self.buffer.iter().next() {
                let packet = self.buffer.remove(&seq).unwrap();
                self.next_sequence = seq + 1;
                Some(packet)
            } else {
                None
            }
        } else {
            None
        }
    }

    /// 获取缓冲区中的包数
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    /// 检查缓冲区是否为空
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    /// 获取配置
    pub fn config(&self) -> &JitterBufferConfig {
        &self.config
    }

    /// 清空缓冲区
    pub fn clear(&mut self) {
        self.buffer.clear();
    }

    /// 调整目标延迟
    pub fn adjust_delay(&mut self, jitter_ms: u32) {
        let new_delay = jitter_ms.clamp(self.config.min_delay_ms, self.config.max_delay_ms);
        self.config.target_delay_ms = new_delay;
    }
}

/// 原始数据包（存储未解码的 Opus 字节）
///
/// 用于在解码前缓冲网络包，比存储 PCM 更省内存
/// （Opus 帧 ~60-120 bytes vs PCM 960×4 = 3840 bytes）
#[derive(Debug, Clone)]
pub struct RawAudioPacket {
    /// 序列号
    pub sequence: u32,

    /// 时间戳
    pub timestamp: u32,

    /// 原始编码数据（Opus 帧）
    pub data: Vec<u8>,
}

/// 原始数据 Jitter Buffer
///
/// 在 Opus 解码前缓冲网络包，处理乱序和延迟抖动。
/// 比 `JitterBuffer`（存储 f32 PCM）更省内存。
pub struct RawJitterBuffer {
    /// 缓冲区（按序列号排序）
    buffer: BTreeMap<u32, RawAudioPacket>,

    /// 配置
    config: JitterBufferConfig,

    /// 下一个期望的序列号
    next_sequence: u32,
}

impl RawJitterBuffer {
    /// 创建新的 Raw Jitter Buffer
    pub fn new(config: JitterBufferConfig) -> Self {
        Self {
            buffer: BTreeMap::new(),
            config,
            next_sequence: 0,
        }
    }

    /// 使用默认配置创建
    pub fn with_default_config() -> Self {
        Self::new(JitterBufferConfig::default())
    }

    /// 推入原始数据包
    pub fn push(&mut self, sequence: u32, timestamp: u32, data: Vec<u8>) {
        // 如果缓冲区满了，丢弃最旧的包
        if self.buffer.len() >= self.config.max_packets {
            if let Some((&oldest_seq, _)) = self.buffer.iter().next() {
                self.buffer.remove(&oldest_seq);
            }
        }

        self.buffer.insert(sequence, RawAudioPacket {
            sequence,
            timestamp,
            data,
        });
    }

    /// 弹出数据包（按序列号顺序）
    pub fn pop(&mut self) -> Option<RawAudioPacket> {
        // 检查是否有下一个期望的包
        if let Some(packet) = self.buffer.remove(&self.next_sequence) {
            self.next_sequence += 1;
            Some(packet)
        } else if !self.buffer.is_empty() {
            // 如果没有期望的包，但有其他包，跳过缺失的包
            if let Some((&seq, _)) = self.buffer.iter().next() {
                let packet = self.buffer.remove(&seq).unwrap();
                self.next_sequence = seq + 1;
                Some(packet)
            } else {
                None
            }
        } else {
            None
        }
    }

    /// 获取缓冲区中的包数
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    /// 检查缓冲区是否为空
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    /// 获取配置
    pub fn config(&self) -> &JitterBufferConfig {
        &self.config
    }

    /// 清空缓冲区
    pub fn clear(&mut self) {
        self.buffer.clear();
        self.next_sequence = 0;
    }

    /// 调整目标延迟
    pub fn adjust_delay(&mut self, jitter_ms: u32) {
        let new_delay = jitter_ms.clamp(self.config.min_delay_ms, self.config.max_delay_ms);
        self.config.target_delay_ms = new_delay;
    }

    /// 获取下一个期望的序列号
    pub fn next_sequence(&self) -> u32 {
        self.next_sequence
    }
}
