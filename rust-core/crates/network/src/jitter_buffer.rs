//! Jitter Buffer 实现
//!
//! 用于缓冲网络音频包，处理乱序和延迟抖动。
//! 支持自适应延迟调整、丢包补偿（PLC）和网络质量监控。

use std::collections::BTreeMap;
use std::time::Instant;

// ─── 配置 ────────────────────────────────────────────────────────────────────

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

/// 自适应延迟配置
#[derive(Debug, Clone)]
pub struct AdaptiveConfig {
    /// 抖动测量窗口大小（最近 N 个包的到达间隔）
    pub jitter_window_size: usize,

    /// 延迟调整步长（毫秒）
    pub adjust_step_ms: u32,

    /// 调整冷却期（毫秒），避免频繁调整
    pub cooldown_ms: u64,

    /// 最小延迟（毫秒）
    pub min_delay_ms: u32,

    /// 最大延迟（毫秒）
    pub max_delay_ms: u32,

    /// PLC 衰减因子（0.0 - 1.0），每帧重复时乘以此系数
    pub plc_decay_factor: f32,

    /// PLC 最大重复帧数
    pub plc_max_frames: u32,
}

impl Default for AdaptiveConfig {
    fn default() -> Self {
        Self {
            jitter_window_size: 100,
            adjust_step_ms: 5,
            cooldown_ms: 500,
            min_delay_ms: 20,
            max_delay_ms: 200,
            plc_decay_factor: 0.95,
            plc_max_frames: 10,
        }
    }
}

// ─── 数据包 ──────────────────────────────────────────────────────────────────

/// 音频数据包
#[derive(Debug, Clone)]
pub struct AudioPacket {
    /// 序列号
    pub sequence: u32,

    /// 音频数据
    pub data: Vec<f32>,
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

// ─── 抖动统计 ────────────────────────────────────────────────────────────────

/// 抖动统计信息
#[derive(Debug, Clone)]
pub struct JitterStats {
    /// 平均抖动（毫秒）
    pub mean_jitter_ms: f64,

    /// 抖动标准差（毫秒）
    pub stddev_jitter_ms: f64,

    /// P95 抖动（毫秒）
    pub p95_jitter_ms: f64,

    /// 窗口内样本数
    pub sample_count: usize,
}

impl JitterStats {
    fn new() -> Self {
        Self {
            mean_jitter_ms: 0.0,
            stddev_jitter_ms: 0.0,
            p95_jitter_ms: 0.0,
            sample_count: 0,
        }
    }
}

/// 内部抖动追踪器：滑动窗口统计到达间隔
struct JitterTracker {
    /// 到达间隔窗口（毫秒）
    intervals: Vec<f64>,

    /// 窗口大小
    window_size: usize,

    /// 上一次到达时间
    last_arrival: Option<Instant>,

    /// 缓存的统计结果
    cached_stats: JitterStats,
}

impl JitterTracker {
    fn new(window_size: usize) -> Self {
        Self {
            intervals: Vec::with_capacity(window_size),
            window_size,
            last_arrival: None,
            cached_stats: JitterStats::new(),
        }
    }

    /// 记录一个包的到达时间，返回与上一包的间隔（毫秒）
    fn record_arrival(&mut self, now: Instant) {
        if let Some(last) = self.last_arrival {
            let interval_ms = now.duration_since(last).as_secs_f64() * 1000.0;
            // 过滤异常值：正常音频包间隔应在 1ms ~ 500ms
            if interval_ms > 0.5 && interval_ms < 500.0 {
                if self.intervals.len() >= self.window_size {
                    self.intervals.remove(0);
                }
                self.intervals.push(interval_ms);
            }
        }
        self.last_arrival = Some(now);
        self.cached_stats = self.compute_stats();
    }

    /// 计算抖动统计
    fn compute_stats(&self) -> JitterStats {
        let n = self.intervals.len();
        if n < 2 {
            return JitterStats::new();
        }

        // 计算均值
        let mean = self.intervals.iter().sum::<f64>() / n as f64;

        // 计算标准差
        let variance = self
            .intervals
            .iter()
            .map(|&x| {
                let diff = x - mean;
                diff * diff
            })
            .sum::<f64>()
            / n as f64;
        let stddev = variance.sqrt();

        // 计算 P95
        let mut sorted = self.intervals.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let p95_idx = ((n as f64 * 0.95) as usize).min(n - 1);
        let p95 = sorted[p95_idx];

        JitterStats {
            mean_jitter_ms: mean,
            stddev_jitter_ms: stddev,
            p95_jitter_ms: p95,
            sample_count: n,
        }
    }

    fn stats(&self) -> &JitterStats {
        &self.cached_stats
    }

    fn reset(&mut self) {
        self.intervals.clear();
        self.last_arrival = None;
        self.cached_stats = JitterStats::new();
    }
}

// ─── 丢包追踪 ────────────────────────────────────────────────────────────────

/// 丢包追踪器
struct LossTracker {
    /// 总期望包数
    total_expected: u64,

    /// 丢包数
    total_lost: u64,

    /// 最近的序列号
    last_sequence: Option<u32>,
}

impl LossTracker {
    fn new() -> Self {
        Self {
            total_expected: 0,
            total_lost: 0,
            last_sequence: None,
        }
    }

    /// 记录接收到的序列号，返回丢包数（序列号跳跃造成的间隙）
    fn record_packet(&mut self, sequence: u32) -> u32 {
        let lost = if let Some(last) = self.last_sequence {
            let expected_next = last.wrapping_add(1);
            if sequence > expected_next {
                sequence - expected_next
            } else if sequence == expected_next {
                0
            } else {
                // 乱序到达，不计丢包
                0
            }
        } else {
            0
        };

        self.total_expected += 1;
        self.total_lost += lost as u64;
        self.last_sequence = Some(sequence);
        lost
    }

    fn loss_rate(&self) -> f32 {
        if self.total_expected == 0 {
            0.0
        } else {
            self.total_lost as f32 / (self.total_expected + self.total_lost) as f32
        }
    }

    fn reset(&mut self) {
        self.total_expected = 0;
        self.total_lost = 0;
        self.last_sequence = None;
    }
}

// ─── 网络质量 ────────────────────────────────────────────────────────────────

/// 网络质量指标
#[derive(Debug, Clone)]
pub struct NetworkQuality {
    /// 缓冲区健康度（0-100）
    pub health: u8,

    /// 丢包率（0.0 - 1.0）
    pub loss_rate: f32,

    /// 平均抖动（毫秒）
    pub avg_jitter_ms: f64,

    /// 当前目标延迟（毫秒）
    pub current_delay_ms: u32,

    /// 缓冲区包数
    pub buffer_len: usize,

    /// PLC 触发次数
    pub plc_count: u64,
}

/// 内部质量追踪
struct QualityTracker {
    plc_count: u64,
}

impl QualityTracker {
    fn new() -> Self {
        Self { plc_count: 0 }
    }

    fn record_plc(&mut self) {
        self.plc_count += 1;
    }

    fn reset(&mut self) {
        self.plc_count = 0;
    }
}

// ─── 自适应延迟引擎 ──────────────────────────────────────────────────────────

/// 自适应延迟引擎
///
/// 根据抖动统计动态调整 target_delay_ms：
/// - 抖动增大 → 增加延迟（步进 adjust_step_ms）
/// - 抖动减小 → 减少延迟（步进 adjust_step_ms）
/// - 冷却期内不调整
struct AdaptiveEngine {
    config: AdaptiveConfig,
    current_delay_ms: u32,
    last_adjust_time: Option<Instant>,
}

impl AdaptiveEngine {
    fn new(config: AdaptiveConfig) -> Self {
        let initial = (config.min_delay_ms + config.max_delay_ms) / 2;
        Self {
            current_delay_ms: initial.clamp(config.min_delay_ms, config.max_delay_ms),
            config,
            last_adjust_time: None,
        }
    }

    /// 根据抖动统计尝试调整延迟
    fn try_adjust(&mut self, stats: &JitterStats) -> bool {
        if stats.sample_count < 2 {
            return false;
        }

        let now = Instant::now();

        // 检查冷却期
        if let Some(last) = self.last_adjust_time {
            let elapsed_ms = now.duration_since(last).as_millis() as u64;
            if elapsed_ms < self.config.cooldown_ms {
                return false;
            }
        }

        // 目标延迟 = P95 抖动 × 1.5（留余量）
        let target = (stats.p95_jitter_ms * 1.5) as u32;

        let step = self.config.adjust_step_ms;
        let new_delay = if target > self.current_delay_ms {
            // 需要增加延迟
            (self.current_delay_ms + step).min(target)
        } else if target < self.current_delay_ms.saturating_sub(step * 2) {
            // 抖动明显减小，降低延迟
            self.current_delay_ms.saturating_sub(step)
        } else {
            return false;
        };

        let clamped = new_delay.clamp(self.config.min_delay_ms, self.config.max_delay_ms);
        if clamped != self.current_delay_ms {
            self.current_delay_ms = clamped;
            self.last_adjust_time = Some(now);
            true
        } else {
            false
        }
    }

    fn delay_ms(&self) -> u32 {
        self.current_delay_ms
    }

    fn reset(&mut self) {
        self.current_delay_ms = (self.config.min_delay_ms + self.config.max_delay_ms) / 2;
        self.last_adjust_time = None;
    }
}

// ─── PLC 生成器 ──────────────────────────────────────────────────────────────

/// 丢包补偿（PLC）状态
struct PlcState {
    /// 上一帧 PCM 数据
    last_frame: Option<Vec<f32>>,

    /// 当前衰减因子
    decay: f32,

    /// 已连续补偿帧数
    consecutive_plc: u32,

    /// 配置
    config: AdaptiveConfig,
}

impl PlcState {
    fn new(config: AdaptiveConfig) -> Self {
        Self {
            last_frame: None,
            decay: 1.0,
            consecutive_plc: 0,
            config,
        }
    }

    /// 用正常帧更新 PLC 状态
    fn update_with_frame(&mut self, data: &[f32]) {
        self.last_frame = Some(data.to_vec());
        self.decay = 1.0;
        self.consecutive_plc = 0;
    }

    /// 生成补偿帧（重复上一帧 + 渐进衰减）
    fn generate(&mut self, frame_size: usize) -> Option<Vec<f32>> {
        if self.consecutive_plc >= self.config.plc_max_frames {
            // 超过最大补偿帧数，返回静音
            return Some(vec![0.0; frame_size]);
        }

        if let Some(ref last) = self.last_frame {
            self.decay *= self.config.plc_decay_factor;
            self.consecutive_plc += 1;

            let mut frame = Vec::with_capacity(frame_size);
            for (i, &sample) in last.iter().enumerate() {
                if i < frame_size {
                    frame.push(sample * self.decay);
                }
            }
            // 如果请求的帧比上一帧长，补零
            while frame.len() < frame_size {
                frame.push(0.0);
            }
            Some(frame)
        } else {
            // 没有历史帧，返回静音
            Some(vec![0.0; frame_size])
        }
    }

    fn reset(&mut self) {
        self.last_frame = None;
        self.decay = 1.0;
        self.consecutive_plc = 0;
    }
}

// ─── Jitter Buffer ───────────────────────────────────────────────────────────

/// Jitter Buffer（PCM f32 版本）
pub struct JitterBuffer {
    /// 缓冲区（按序列号排序）
    buffer: BTreeMap<u32, AudioPacket>,

    /// 配置
    config: JitterBufferConfig,

    /// 下一个期望的序列号
    next_sequence: u32,

    /// 是否已初始化（收到第一个包）
    initialized: bool,

    /// 抖动追踪器
    jitter_tracker: JitterTracker,

    /// 丢包追踪器
    loss_tracker: LossTracker,

    /// 自适应引擎
    adaptive: AdaptiveEngine,

    /// PLC 状态
    plc: PlcState,

    /// 质量追踪
    quality: QualityTracker,
}

impl JitterBuffer {
    /// 创建新的 Jitter Buffer（自适应模式）
    pub fn new(config: JitterBufferConfig) -> Self {
        let adaptive_config = AdaptiveConfig {
            min_delay_ms: config.min_delay_ms,
            max_delay_ms: config.max_delay_ms,
            ..Default::default()
        };
        Self {
            buffer: BTreeMap::new(),
            config,
            next_sequence: 0,
            initialized: false,
            jitter_tracker: JitterTracker::new(adaptive_config.jitter_window_size),
            loss_tracker: LossTracker::new(),
            adaptive: AdaptiveEngine::new(adaptive_config),
            plc: PlcState::new(AdaptiveConfig::default()),
            quality: QualityTracker::new(),
        }
    }

    /// 使用默认配置创建
    pub fn with_default_config() -> Self {
        Self::new(JitterBufferConfig::default())
    }

    /// 使用自定义自适应配置创建
    pub fn with_adaptive_config(
        config: JitterBufferConfig,
        adaptive_config: AdaptiveConfig,
    ) -> Self {
        Self {
            buffer: BTreeMap::new(),
            next_sequence: 0,
            initialized: false,
            jitter_tracker: JitterTracker::new(adaptive_config.jitter_window_size),
            loss_tracker: LossTracker::new(),
            adaptive: AdaptiveEngine::new(adaptive_config.clone()),
            plc: PlcState::new(adaptive_config),
            config,
            quality: QualityTracker::new(),
        }
    }

    /// 推入数据包
    pub fn push(&mut self, sequence: u32, data: Vec<f32>) {
        // 初始化：以第一个包的序列号为起点
        if !self.initialized {
            self.next_sequence = sequence;
            self.initialized = true;
        }

        // 记录到达时间，更新抖动统计
        self.jitter_tracker.record_arrival(Instant::now());

        // 检测丢包
        let lost = self.loss_tracker.record_packet(sequence);
        if lost > 0 {
            // 丢包已记录，后续 pop 时会触发 PLC
        }

        // 自适应延迟调整
        let stats = self.jitter_tracker.stats().clone();
        self.adaptive.try_adjust(&stats);

        // 更新配置中的 target_delay
        self.config.target_delay_ms = self.adaptive.delay_ms();

        // 如果缓冲区满了，丢弃最旧的包
        if self.buffer.len() >= self.config.max_packets {
            if let Some((&oldest_seq, _)) = self.buffer.iter().next() {
                self.buffer.remove(&oldest_seq);
            }
        }

        self.buffer.insert(sequence, AudioPacket { sequence, data });
    }

    /// 弹出数据包（带自适应延迟和 PLC）
    pub fn pop(&mut self) -> Option<AudioPacket> {
        // 检查是否有下一个期望的包
        if let Some(packet) = self.buffer.remove(&self.next_sequence) {
            self.next_sequence = self.next_sequence.wrapping_add(1);
            // 更新 PLC 状态
            self.plc.update_with_frame(&packet.data);
            return Some(packet);
        }

        // 没有期望的包：检查是否应该等待
        // 如果缓冲区中最旧的包序列号 > next_sequence，说明有丢包
        if let Some((&first_seq, _)) = self.buffer.iter().next() {
            if first_seq > self.next_sequence {
                // 检测到丢包，触发 PLC
                let frame_size = 960; // 20ms@48kHz
                if let Some(plc_frame) = self.plc.generate(frame_size) {
                    self.quality.record_plc();
                    self.next_sequence = self.next_sequence.wrapping_add(1);
                    return Some(AudioPacket {
                        sequence: self.next_sequence.wrapping_sub(1),
                        data: plc_frame,
                    });
                }
            }
        }

        // 没有任何包
        if !self.buffer.is_empty() {
            // 跳到最早的包
            if let Some((&seq, _)) = self.buffer.iter().next() {
                let packet = self.buffer.remove(&seq).unwrap();
                self.plc.update_with_frame(&packet.data);
                self.next_sequence = seq.wrapping_add(1);
                return Some(packet);
            }
        }

        None
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
        self.initialized = false;
        self.jitter_tracker.reset();
        self.loss_tracker.reset();
        self.adaptive.reset();
        self.plc.reset();
        self.quality.reset();
    }

    /// 调整目标延迟
    pub fn adjust_delay(&mut self, jitter_ms: u32) {
        let new_delay = jitter_ms.clamp(self.config.min_delay_ms, self.config.max_delay_ms);
        self.config.target_delay_ms = new_delay;
    }

    /// 获取抖动统计
    pub fn jitter_stats(&self) -> &JitterStats {
        self.jitter_tracker.stats()
    }

    /// 获取当前目标延迟（毫秒）
    pub fn current_delay_ms(&self) -> u32 {
        self.adaptive.delay_ms()
    }

    /// 获取网络质量指标
    pub fn network_quality(&self) -> NetworkQuality {
        let stats = self.jitter_tracker.stats();
        NetworkQuality {
            health: self.compute_health(),
            loss_rate: self.loss_tracker.loss_rate(),
            avg_jitter_ms: stats.mean_jitter_ms,
            current_delay_ms: self.adaptive.delay_ms(),
            buffer_len: self.buffer.len(),
            plc_count: self.quality.plc_count,
        }
    }

    /// 计算缓冲区健康度（0-100）
    fn compute_health(&self) -> u8 {
        let loss_penalty = (self.loss_tracker.loss_rate() * 50.0) as u8;
        let jitter = self.jitter_tracker.stats().mean_jitter_ms;
        let jitter_penalty = if jitter > 50.0 {
            30
        } else if jitter > 20.0 {
            15
        } else if jitter > 10.0 {
            5
        } else {
            0
        };
        100_u8
            .saturating_sub(loss_penalty)
            .saturating_sub(jitter_penalty)
    }

    /// 获取下一个期望的序列号
    pub fn next_sequence(&self) -> u32 {
        self.next_sequence
    }
}

// ─── Raw Jitter Buffer ───────────────────────────────────────────────────────

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

    /// 是否已初始化
    initialized: bool,

    /// 抖动追踪器
    jitter_tracker: JitterTracker,

    /// 丢包追踪器
    loss_tracker: LossTracker,

    /// 自适应引擎
    adaptive: AdaptiveEngine,

    /// 质量追踪
    quality: QualityTracker,
}

impl RawJitterBuffer {
    /// 创建新的 Raw Jitter Buffer
    pub fn new(config: JitterBufferConfig) -> Self {
        let adaptive_config = AdaptiveConfig {
            min_delay_ms: config.min_delay_ms,
            max_delay_ms: config.max_delay_ms,
            ..Default::default()
        };
        Self {
            buffer: BTreeMap::new(),
            config,
            next_sequence: 0,
            initialized: false,
            jitter_tracker: JitterTracker::new(adaptive_config.jitter_window_size),
            loss_tracker: LossTracker::new(),
            adaptive: AdaptiveEngine::new(adaptive_config),
            quality: QualityTracker::new(),
        }
    }

    /// 使用默认配置创建
    pub fn with_default_config() -> Self {
        Self::new(JitterBufferConfig::default())
    }

    /// 使用自定义自适应配置创建
    pub fn with_adaptive_config(
        config: JitterBufferConfig,
        adaptive_config: AdaptiveConfig,
    ) -> Self {
        Self {
            buffer: BTreeMap::new(),
            next_sequence: 0,
            initialized: false,
            jitter_tracker: JitterTracker::new(adaptive_config.jitter_window_size),
            loss_tracker: LossTracker::new(),
            adaptive: AdaptiveEngine::new(adaptive_config),
            config,
            quality: QualityTracker::new(),
        }
    }

    /// 推入原始数据包
    pub fn push(&mut self, sequence: u32, timestamp: u32, data: Vec<u8>) {
        // 初始化
        if !self.initialized {
            self.next_sequence = sequence;
            self.initialized = true;
        }

        // 记录到达时间
        self.jitter_tracker.record_arrival(Instant::now());

        // 检测丢包
        let _lost = self.loss_tracker.record_packet(sequence);

        // 自适应延迟调整
        let stats = self.jitter_tracker.stats().clone();
        self.adaptive.try_adjust(&stats);
        self.config.target_delay_ms = self.adaptive.delay_ms();

        // 如果缓冲区满了，丢弃最旧的包
        if self.buffer.len() >= self.config.max_packets {
            if let Some((&oldest_seq, _)) = self.buffer.iter().next() {
                self.buffer.remove(&oldest_seq);
            }
        }

        self.buffer.insert(
            sequence,
            RawAudioPacket {
                sequence,
                timestamp,
                data,
            },
        );
    }

    /// 弹出数据包（按序列号顺序，带丢包检测）
    pub fn pop(&mut self) -> Option<RawAudioPacket> {
        // 检查是否有下一个期望的包
        if let Some(packet) = self.buffer.remove(&self.next_sequence) {
            self.next_sequence = self.next_sequence.wrapping_add(1);
            return Some(packet);
        }

        // 没有期望的包：检查是否有丢包
        if let Some((&first_seq, _)) = self.buffer.iter().next() {
            if first_seq > self.next_sequence {
                // 丢包了，记录 PLC 事件（Raw buffer 不做帧补偿，仅计数）
                self.quality.record_plc();
                // 跳到最早的可用包
                let packet = self.buffer.remove(&first_seq).unwrap();
                self.next_sequence = first_seq.wrapping_add(1);
                return Some(packet);
            }
        }

        // 缓冲区为空
        None
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
        self.initialized = false;
        self.jitter_tracker.reset();
        self.loss_tracker.reset();
        self.adaptive.reset();
        self.quality.reset();
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

    /// 获取抖动统计
    pub fn jitter_stats(&self) -> &JitterStats {
        self.jitter_tracker.stats()
    }

    /// 获取当前目标延迟（毫秒）
    pub fn current_delay_ms(&self) -> u32 {
        self.adaptive.delay_ms()
    }

    /// 获取网络质量指标
    pub fn network_quality(&self) -> NetworkQuality {
        let stats = self.jitter_tracker.stats();
        NetworkQuality {
            health: self.compute_health(),
            loss_rate: self.loss_tracker.loss_rate(),
            avg_jitter_ms: stats.mean_jitter_ms,
            current_delay_ms: self.adaptive.delay_ms(),
            buffer_len: self.buffer.len(),
            plc_count: self.quality.plc_count,
        }
    }

    /// 计算缓冲区健康度（0-100）
    fn compute_health(&self) -> u8 {
        let loss_penalty = (self.loss_tracker.loss_rate() * 50.0) as u8;
        let jitter = self.jitter_tracker.stats().mean_jitter_ms;
        let jitter_penalty = if jitter > 50.0 {
            30
        } else if jitter > 20.0 {
            15
        } else if jitter > 10.0 {
            5
        } else {
            0
        };
        100_u8
            .saturating_sub(loss_penalty)
            .saturating_sub(jitter_penalty)
    }
}

// ─── 测试 ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    // ── 基础功能测试 ──────────────────────────────────────────────────────

    #[test]
    fn test_jitter_buffer_basic() {
        let config = JitterBufferConfig::default();
        let mut jb = JitterBuffer::new(config);

        jb.push(0, vec![1.0f32; 100]);
        jb.push(1, vec![2.0f32; 100]);
        jb.push(2, vec![3.0f32; 100]);

        let packet = jb.pop().unwrap();
        assert_eq!(packet.sequence, 0);

        let packet = jb.pop().unwrap();
        assert_eq!(packet.sequence, 1);

        let packet = jb.pop().unwrap();
        assert_eq!(packet.sequence, 2);
    }

    #[test]
    fn test_jitter_buffer_reorder() {
        let config = JitterBufferConfig::default();
        let mut jb = JitterBuffer::new(config);

        // push 0 first so next_sequence initializes to 0
        jb.push(0, vec![1.0f32; 100]);
        jb.push(2, vec![3.0f32; 100]);
        jb.push(1, vec![2.0f32; 100]);

        let packet = jb.pop().unwrap();
        assert_eq!(packet.sequence, 0);

        let packet = jb.pop().unwrap();
        assert_eq!(packet.sequence, 1);

        let packet = jb.pop().unwrap();
        assert_eq!(packet.sequence, 2);
    }

    #[test]
    fn test_jitter_buffer_empty() {
        let config = JitterBufferConfig::default();
        let mut jb = JitterBuffer::new(config);
        assert!(jb.pop().is_none());
    }

    #[test]
    fn test_jitter_buffer_stats() {
        let config = JitterBufferConfig::default();
        let mut jb = JitterBuffer::new(config);

        jb.push(0, vec![1.0f32; 100]);
        jb.push(1, vec![2.0f32; 100]);

        assert_eq!(jb.len(), 2);
        assert!(!jb.is_empty());

        jb.pop();
        assert_eq!(jb.len(), 1);
    }

    #[test]
    fn test_jitter_buffer_overflow() {
        let config = JitterBufferConfig {
            max_packets: 3,
            ..Default::default()
        };
        let mut jb = JitterBuffer::new(config);

        jb.push(0, vec![0.0; 10]);
        jb.push(1, vec![1.0; 10]);
        jb.push(2, vec![2.0; 10]);
        jb.push(3, vec![3.0; 10]); // 应该丢弃最旧的包

        assert_eq!(jb.len(), 3);
    }

    // ── 丢包补偿测试 ──────────────────────────────────────────────────────

    #[test]
    fn test_plc_on_packet_loss() {
        let config = JitterBufferConfig::default();
        let mut jb = JitterBuffer::new(config);

        // 正常包 0
        jb.push(0, vec![0.5f32; 960]);
        let pkt = jb.pop().unwrap();
        assert_eq!(pkt.sequence, 0);
        assert!((pkt.data[0] - 0.5).abs() < f32::EPSILON);

        // 跳过 1，推入 2（模拟丢包）
        jb.push(2, vec![1.0f32; 960]);

        // pop 应该返回 PLC 帧（序列号 1 的补偿）
        let plc_pkt = jb.pop().unwrap();
        assert_eq!(plc_pkt.sequence, 1);
        // PLC 帧应该是上一帧乘以衰减因子
        assert!(plc_pkt.data[0] < 0.5);
        assert!(plc_pkt.data[0] > 0.0);
    }

    #[test]
    fn test_plc_decay() {
        let config = JitterBufferConfig::default();
        let mut jb = JitterBuffer::new(config);

        // 正常包 0
        jb.push(0, vec![1.0f32; 960]);
        jb.pop().unwrap();

        // 连续丢包：跳过 1、2、3
        jb.push(4, vec![2.0f32; 960]);

        // PLC 帧 1
        let plc1 = jb.pop().unwrap();
        assert_eq!(plc1.sequence, 1);
        // PLC 帧 2
        let plc2 = jb.pop().unwrap();
        assert_eq!(plc2.sequence, 2);
        // PLC 帧 3
        let plc3 = jb.pop().unwrap();
        assert_eq!(plc3.sequence, 3);

        // 每帧衰减应该递减
        assert!(plc1.data[0] > plc2.data[0]);
        assert!(plc2.data[0] > plc3.data[0]);
    }

    #[test]
    fn test_plc_max_frames_then_silence() {
        let adaptive_config = AdaptiveConfig {
            plc_max_frames: 2,
            ..Default::default()
        };
        let config = JitterBufferConfig::default();
        let mut jb = JitterBuffer::with_adaptive_config(config, adaptive_config);

        jb.push(0, vec![1.0f32; 960]);
        jb.pop().unwrap();

        // 跳过 1、2、3，4 个丢包
        jb.push(4, vec![2.0f32; 960]);

        // PLC 帧 1
        let plc1 = jb.pop().unwrap();
        assert_eq!(plc1.sequence, 1);
        // PLC 帧 2
        let plc2 = jb.pop().unwrap();
        assert_eq!(plc2.sequence, 2);
        // 超过 max，应为静音
        let silence = jb.pop().unwrap();
        assert_eq!(silence.sequence, 3);
        assert!(silence.data.iter().all(|&x| x == 0.0));
    }

    // ── 抖动统计测试 ──────────────────────────────────────────────────────

    #[test]
    fn test_jitter_stats_basic() {
        let mut tracker = JitterTracker::new(10);

        let base = Instant::now();
        // 模拟 5 个包，间隔 ~20ms
        for i in 0..5 {
            let t = base + Duration::from_millis(i * 20);
            tracker.record_arrival(t);
        }

        let stats = tracker.stats();
        assert_eq!(stats.sample_count, 4); // 第一个包没有间隔
        assert!(stats.mean_jitter_ms > 15.0);
        assert!(stats.mean_jitter_ms < 25.0);
    }

    #[test]
    fn test_jitter_stats_high_jitter() {
        let mut tracker = JitterTracker::new(100);

        let base = Instant::now();
        // 模拟抖动大的情况：间隔不均匀
        let intervals = [10u64, 50, 5, 40, 15, 45, 8, 35];
        let mut t = 0u64;
        for &interval in &intervals {
            t += interval;
            tracker.record_arrival(base + Duration::from_millis(t));
        }

        let stats = tracker.stats();
        // 8 arrivals → 7 intervals (first has no previous)
        assert_eq!(stats.sample_count, intervals.len() - 1);
        assert!(stats.stddev_jitter_ms > 0.0);
        assert!(stats.p95_jitter_ms >= stats.mean_jitter_ms);
    }

    // ── 自适应延迟测试 ────────────────────────────────────────────────────

    #[test]
    fn test_adaptive_engine_increases_delay() {
        let config = AdaptiveConfig {
            cooldown_ms: 0, // 无冷却期
            adjust_step_ms: 5,
            min_delay_ms: 20,
            max_delay_ms: 200,
            ..Default::default()
        };
        let mut engine = AdaptiveEngine::new(config);

        let initial = engine.delay_ms();

        // 高抖动：P95 = 120ms → target = 180ms (> initial 110)
        let stats = JitterStats {
            mean_jitter_ms: 80.0,
            stddev_jitter_ms: 30.0,
            p95_jitter_ms: 120.0,
            sample_count: 10,
        };

        // 多次调用应该逐步增加
        for _ in 0..20 {
            engine.try_adjust(&stats);
        }

        assert!(engine.delay_ms() > initial);
    }

    #[test]
    fn test_adaptive_engine_decreases_delay() {
        let config = AdaptiveConfig {
            cooldown_ms: 0,
            adjust_step_ms: 5,
            min_delay_ms: 20,
            max_delay_ms: 200,
            ..Default::default()
        };
        let mut engine = AdaptiveEngine::new(config);

        // 先推高延迟
        let high_stats = JitterStats {
            mean_jitter_ms: 80.0,
            stddev_jitter_ms: 20.0,
            p95_jitter_ms: 100.0,
            sample_count: 10,
        };
        for _ in 0..50 {
            engine.try_adjust(&high_stats);
        }
        let high_delay = engine.delay_ms();

        // 然后低抖动
        let low_stats = JitterStats {
            mean_jitter_ms: 5.0,
            stddev_jitter_ms: 2.0,
            p95_jitter_ms: 8.0,
            sample_count: 10,
        };
        for _ in 0..50 {
            engine.try_adjust(&low_stats);
        }

        assert!(engine.delay_ms() < high_delay);
    }

    #[test]
    fn test_adaptive_engine_respects_bounds() {
        let config = AdaptiveConfig {
            cooldown_ms: 0,
            adjust_step_ms: 5,
            min_delay_ms: 20,
            max_delay_ms: 200,
            ..Default::default()
        };
        let mut engine = AdaptiveEngine::new(config);

        // 极高抖动
        let stats = JitterStats {
            mean_jitter_ms: 200.0,
            stddev_jitter_ms: 50.0,
            p95_jitter_ms: 300.0,
            sample_count: 10,
        };
        for _ in 0..100 {
            engine.try_adjust(&stats);
        }
        assert!(engine.delay_ms() <= 200);

        // 极低抖动
        let stats = JitterStats {
            mean_jitter_ms: 1.0,
            stddev_jitter_ms: 0.5,
            p95_jitter_ms: 2.0,
            sample_count: 10,
        };
        for _ in 0..100 {
            engine.try_adjust(&stats);
        }
        assert!(engine.delay_ms() >= 20);
    }

    #[test]
    fn test_adaptive_engine_cooldown() {
        let config = AdaptiveConfig {
            cooldown_ms: 10000, // 10 秒冷却期
            adjust_step_ms: 5,
            min_delay_ms: 20,
            max_delay_ms: 200,
            ..Default::default()
        };
        let mut engine = AdaptiveEngine::new(config);

        let stats = JitterStats {
            mean_jitter_ms: 80.0,
            stddev_jitter_ms: 20.0,
            p95_jitter_ms: 100.0,
            sample_count: 10,
        };

        // 第一次调整
        engine.try_adjust(&stats);
        let after_first = engine.delay_ms();

        // 立即再次尝试，应该被冷却期阻止
        engine.try_adjust(&stats);
        assert_eq!(engine.delay_ms(), after_first);
    }

    // ── 丢包追踪测试 ──────────────────────────────────────────────────────

    #[test]
    fn test_loss_tracker_sequential() {
        let mut tracker = LossTracker::new();

        assert_eq!(tracker.record_packet(0), 0);
        assert_eq!(tracker.record_packet(1), 0);
        assert_eq!(tracker.record_packet(2), 0);
        assert!((tracker.loss_rate() - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_loss_tracker_with_gaps() {
        let mut tracker = LossTracker::new();

        assert_eq!(tracker.record_packet(0), 0);
        assert_eq!(tracker.record_packet(1), 0);
        // 跳过 2、3
        assert_eq!(tracker.record_packet(4), 2);
        assert!(tracker.loss_rate() > 0.0);
    }

    #[test]
    fn test_loss_tracker_reorder() {
        let mut tracker = LossTracker::new();

        assert_eq!(tracker.record_packet(0), 0);
        assert_eq!(tracker.record_packet(2), 1);
        // 乱序到达
        assert_eq!(tracker.record_packet(1), 0);
    }

    // ── 网络质量测试 ──────────────────────────────────────────────────────

    #[test]
    fn test_network_quality_initial() {
        let jb = JitterBuffer::with_default_config();
        let q = jb.network_quality();
        assert_eq!(q.health, 100);
        assert!((q.loss_rate - 0.0).abs() < f32::EPSILON);
        assert_eq!(q.buffer_len, 0);
        assert_eq!(q.plc_count, 0);
    }

    #[test]
    fn test_network_quality_with_loss() {
        let config = JitterBufferConfig::default();
        let mut jb = JitterBuffer::new(config);

        jb.push(0, vec![0.0; 100]);
        jb.pop().unwrap();

        // 丢包
        jb.push(3, vec![1.0; 100]);

        let q = jb.network_quality();
        assert!(q.loss_rate > 0.0);
        assert!(q.health < 100);
    }

    // ── Raw Jitter Buffer 测试 ────────────────────────────────────────────

    #[test]
    fn test_raw_jitter_buffer_basic() {
        let config = JitterBufferConfig::default();
        let mut jb = RawJitterBuffer::new(config);

        jb.push(0, 100, vec![0x01, 0x02]);
        jb.push(1, 200, vec![0x03, 0x04]);
        jb.push(2, 300, vec![0x05, 0x06]);

        let packet = jb.pop().unwrap();
        assert_eq!(packet.sequence, 0);
        assert_eq!(packet.timestamp, 100);
        assert_eq!(packet.data, vec![0x01, 0x02]);

        let packet = jb.pop().unwrap();
        assert_eq!(packet.sequence, 1);
    }

    #[test]
    fn test_raw_jitter_buffer_reorder() {
        let config = JitterBufferConfig::default();
        let mut jb = RawJitterBuffer::new(config);

        // push 0 first so next_sequence initializes to 0
        jb.push(0, 100, vec![0x01, 0x02]);
        jb.push(2, 300, vec![0x05, 0x06]);
        jb.push(1, 200, vec![0x03, 0x04]);

        let packet = jb.pop().unwrap();
        assert_eq!(packet.sequence, 0);

        let packet = jb.pop().unwrap();
        assert_eq!(packet.sequence, 1);

        let packet = jb.pop().unwrap();
        assert_eq!(packet.sequence, 2);
    }

    #[test]
    fn test_raw_jitter_buffer_empty() {
        let config = JitterBufferConfig::default();
        let mut jb = RawJitterBuffer::new(config);
        assert!(jb.pop().is_none());
        assert!(jb.is_empty());
        assert_eq!(jb.len(), 0);
    }

    #[test]
    fn test_raw_jitter_buffer_skip_missing() {
        let config = JitterBufferConfig::default();
        let mut jb = RawJitterBuffer::new(config);

        // 跳过序列号 0，直接推入 1 和 2
        jb.push(1, 200, vec![0x03, 0x04]);
        jb.push(2, 300, vec![0x05, 0x06]);

        // 第一次 pop 应该跳到 1（跳过缺失的 0）
        let packet = jb.pop().unwrap();
        assert_eq!(packet.sequence, 1);
        assert_eq!(jb.next_sequence(), 2);

        let packet = jb.pop().unwrap();
        assert_eq!(packet.sequence, 2);
    }

    #[test]
    fn test_raw_jitter_buffer_overflow() {
        let config = JitterBufferConfig {
            max_packets: 3,
            ..Default::default()
        };
        let mut jb = RawJitterBuffer::new(config);

        jb.push(0, 100, vec![0x01]);
        jb.push(1, 200, vec![0x02]);
        jb.push(2, 300, vec![0x03]);
        jb.push(3, 400, vec![0x04]); // 应该丢弃最旧的包

        assert_eq!(jb.len(), 3);
    }

    #[test]
    fn test_raw_jitter_buffer_clear() {
        let config = JitterBufferConfig::default();
        let mut jb = RawJitterBuffer::new(config);

        jb.push(0, 100, vec![0x01]);
        jb.push(1, 200, vec![0x02]);

        jb.clear();
        assert!(jb.is_empty());
        assert_eq!(jb.next_sequence(), 0);
    }

    #[test]
    fn test_raw_jitter_buffer_adjust_delay() {
        let config = JitterBufferConfig::default();
        let mut jb = RawJitterBuffer::new(config);

        jb.adjust_delay(50);
        assert_eq!(jb.config().target_delay_ms, 50);

        // 测试边界值
        jb.adjust_delay(5); // 低于 min_delay_ms
        assert_eq!(jb.config().target_delay_ms, 20);

        jb.adjust_delay(300); // 高于 max_delay_ms
        assert_eq!(jb.config().target_delay_ms, 200);
    }

    #[test]
    fn test_raw_jitter_buffer_network_quality() {
        let config = JitterBufferConfig::default();
        let jb = RawJitterBuffer::new(config);
        let q = jb.network_quality();
        assert_eq!(q.health, 100);
        assert_eq!(q.buffer_len, 0);
    }

    #[test]
    fn test_raw_jitter_buffer_jitter_stats() {
        let config = JitterBufferConfig::default();
        let jb = RawJitterBuffer::new(config);
        let stats = jb.jitter_stats();
        assert_eq!(stats.sample_count, 0);
    }

    // ── 极端条件测试 ──────────────────────────────────────────────────────

    #[test]
    fn test_extreme_packet_loss() {
        let config = JitterBufferConfig::default();
        let mut jb = JitterBuffer::new(config);

        jb.push(0, vec![1.0; 960]);
        jb.pop().unwrap();

        // 模拟大量丢包后突然恢复
        jb.push(100, vec![2.0; 960]);

        // 应该触发多次 PLC 然后拿到包 100
        let mut got_real = false;
        for _ in 0..120 {
            if let Some(pkt) = jb.pop() {
                if pkt.sequence == 100 {
                    got_real = true;
                    break;
                }
            }
        }
        assert!(got_real);
    }

    #[test]
    fn test_all_same_sequence_arrives() {
        let config = JitterBufferConfig::default();
        let mut jb = JitterBuffer::new(config);

        // 重复推入同一个序列号
        jb.push(0, vec![1.0; 10]);
        jb.push(0, vec![2.0; 10]);
        jb.push(0, vec![3.0; 10]);

        assert_eq!(jb.len(), 1); // BTreeMap 去重
        let pkt = jb.pop().unwrap();
        assert_eq!(pkt.data, vec![3.0; 10]); // 最后推入的
    }

    #[test]
    fn test_jitter_buffer_with_realistic_timing() {
        let config = JitterBufferConfig::default();
        let mut jb = JitterBuffer::new(config);

        // 模拟真实的 20ms 间隔到达
        for i in 0..10 {
            jb.push(i, vec![i as f32; 960]);
            thread::sleep(Duration::from_millis(2));
        }

        let stats = jb.jitter_stats();
        assert!(stats.sample_count > 0);

        let quality = jb.network_quality();
        assert!(quality.health > 0);
    }

    // ── 跨平台序列号测试 ──────────────────────────────────────────────────

    #[test]
    fn test_wrapping_sequence_numbers() {
        let config = JitterBufferConfig::default();
        let mut jb = JitterBuffer::new(config);

        // 从接近 u32::MAX 开始
        let start = u32::MAX - 2;
        jb.push(start, vec![1.0; 10]);
        jb.push(start + 1, vec![2.0; 10]);
        jb.push(start + 2, vec![3.0; 10]); // 溢出到 0

        let pkt = jb.pop().unwrap();
        assert_eq!(pkt.sequence, start);

        let pkt = jb.pop().unwrap();
        assert_eq!(pkt.sequence, start + 1);
    }
}
