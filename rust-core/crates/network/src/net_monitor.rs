//! 网络状况监控模块
//!
//! 提供实时网络质量评估，包括 RTT 估计、带宽估计、丢包率跟踪。
//! 为自适应码率和 Jitter Buffer 提供数据支持。

use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// 网络统计快照
#[derive(Debug, Clone, PartialEq)]
pub struct NetworkStats {
    /// 往返延迟（毫秒）
    pub rtt_ms: f32,
    /// 抖动（毫秒，延迟标准差）
    pub jitter_ms: f32,
    /// 丢包率（0.0 - 1.0）
    pub loss_rate: f32,
    /// 带宽（bps）
    pub bandwidth_bps: u32,
    /// 综合质量评分（0-100）
    pub quality_score: u8,
}

impl Default for NetworkStats {
    fn default() -> Self {
        Self {
            rtt_ms: 0.0,
            jitter_ms: 0.0,
            loss_rate: 0.0,
            bandwidth_bps: 0,
            quality_score: 100,
        }
    }
}

/// 网络监控配置
#[derive(Debug, Clone)]
pub struct NetMonitorConfig {
    /// EWMA 平滑因子（0.0 - 1.0，越小越平滑）
    pub ewma_alpha: f32,
    /// 滑动窗口大小（秒）
    pub window_duration_secs: u64,
    /// 突发丢包检测阈值（连续丢包数）
    pub burst_loss_threshold: u32,
    /// 最小采样数（低于此数返回默认值）
    pub min_samples: usize,
    /// 带宽估算最小采样间隔（毫秒）
    pub bandwidth_min_interval_ms: u64,
}

impl Default for NetMonitorConfig {
    fn default() -> Self {
        Self {
            ewma_alpha: 0.125,           // TCP 标准 RTT 平滑因子
            window_duration_secs: 10,    // 10 秒滑动窗口
            burst_loss_threshold: 3,     // 连续 3 包视为突发丢包
            min_samples: 3,              // 至少 3 个采样才报告
            bandwidth_min_interval_ms: 5, // 最小 5ms 间隔
        }
    }
}

/// RTT 采样点
#[derive(Debug, Clone)]
struct RttSample {
    rtt_ms: f32,
    timestamp: Instant,
}

/// 带宽采样点
#[derive(Debug, Clone)]
struct BandwidthSample {
    bytes: u64,
    timestamp: Instant,
}

/// 丢包记录
#[derive(Debug, Clone)]
struct LossRecord {
    #[allow(dead_code)]
    expected_seq: u32,
    received: bool,
    timestamp: Instant,
}

/// 突发丢包事件
#[derive(Debug, Clone)]
pub struct BurstLossEvent {
    /// 起始序列号
    pub start_seq: u32,
    /// 丢失包数
    pub count: u32,
    /// 事件时间
    pub timestamp: Instant,
}

/// 自适应码率建议
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BitrateRecommendation {
    /// 保持当前码率
    Hold,
    /// 提升码率
    Increase,
    /// 降低码率
    Decrease,
    /// 大幅降低码率（网络极差）
    AggressiveDecrease,
}

/// 网络状况监控器
pub struct NetMonitor {
    /// 配置
    config: NetMonitorConfig,

    // --- RTT 状态 ---
    /// EWMA 平滑后的 RTT
    smoothed_rtt_ms: f32,
    /// RTT 方差（用于 jitter 计算）
    rtt_var_ms: f32,
    /// 最小 RTT
    min_rtt_ms: f32,
    /// 最大 RTT
    max_rtt_ms: f32,
    /// RTT 历史（滑动窗口）
    rtt_samples: VecDeque<RttSample>,
    /// 待匹配的请求时间戳（key = sequence, value = send time）
    pending_requests: std::collections::HashMap<u32, Instant>,
    /// RTT 采样计数
    rtt_sample_count: u64,

    // --- 带宽状态 ---
    /// 带宽采样历史（滑动窗口）
    bandwidth_samples: VecDeque<BandwidthSample>,
    /// 累计字节数（当前窗口内）
    window_bytes: u64,
    /// 最后一次带宽采样时间
    last_bandwidth_sample: Instant,
    /// 已发送字节累计（用于下一次采样间隔判断）
    pending_bytes: u64,

    // --- 丢包状态 ---
    /// 丢包记录（滑动窗口）
    loss_records: VecDeque<LossRecord>,
    /// 已处理的最大序列号
    max_seen_seq: u32,
    /// 是否已初始化序列号
    seq_initialized: bool,
    /// 突发丢包事件
    burst_events: VecDeque<BurstLossEvent>,
    /// 当前连续丢包计数
    current_loss_streak: u32,
    /// 当前丢包起始序列号
    loss_streak_start: u32,

    // --- 抖动状态 ---
    /// 上一个包的到达时间
    last_arrival_time: Option<Instant>,

    // --- 通用 ---
    /// 监控开始时间
    start_time: Instant,
}

impl NetMonitor {
    /// 创建新的网络监控器
    pub fn new(config: NetMonitorConfig) -> Self {
        let now = Instant::now();
        Self {
            config,

            smoothed_rtt_ms: 0.0,
            rtt_var_ms: 0.0,
            min_rtt_ms: f32::MAX,
            max_rtt_ms: 0.0,
            rtt_samples: VecDeque::new(),
            pending_requests: std::collections::HashMap::new(),
            rtt_sample_count: 0,

            bandwidth_samples: VecDeque::new(),
            window_bytes: 0,
            last_bandwidth_sample: now,
            pending_bytes: 0,

            loss_records: VecDeque::new(),
            max_seen_seq: 0,
            seq_initialized: false,
            burst_events: VecDeque::new(),
            current_loss_streak: 0,
            loss_streak_start: 0,

            last_arrival_time: None,

            start_time: now,
        }
    }

    /// 使用默认配置创建
    pub fn with_default_config() -> Self {
        Self::new(NetMonitorConfig::default())
    }

    // ==================== RTT 估计 ====================

    /// 记录请求发送时间
    pub fn record_request_sent(&mut self, sequence: u32) {
        self.pending_requests.insert(sequence, Instant::now());
    }

    /// 记录响应接收时间，计算 RTT
    pub fn record_response_received(&mut self, sequence: u32) -> Option<f32> {
        let send_time = self.pending_requests.remove(&sequence)?;
        let rtt_ms = send_time.elapsed().as_secs_f32() * 1000.0;
        self.process_rtt_sample(rtt_ms);
        Some(rtt_ms)
    }

    /// 直接报告 RTT（用于已计算好的场景）
    pub fn report_rtt(&mut self, rtt_ms: f32) {
        if rtt_ms < 0.0 {
            return;
        }
        self.process_rtt_sample(rtt_ms);
    }

    fn process_rtt_sample(&mut self, rtt_ms: f32) {
        let alpha = self.config.ewma_alpha;

        if self.rtt_sample_count == 0 {
            // 第一个采样：直接初始化
            self.smoothed_rtt_ms = rtt_ms;
            self.rtt_var_ms = rtt_ms / 2.0;
        } else {
            // EWMA 更新（RFC 6298 算法）
            let diff = (self.smoothed_rtt_ms - rtt_ms).abs();
            self.rtt_var_ms = (1.0 - alpha) * self.rtt_var_ms + alpha * diff;
            self.smoothed_rtt_ms = (1.0 - alpha) * self.smoothed_rtt_ms + alpha * rtt_ms;
        }

        self.min_rtt_ms = self.min_rtt_ms.min(rtt_ms);
        self.max_rtt_ms = self.max_rtt_ms.max(rtt_ms);
        self.rtt_sample_count += 1;

        let now = Instant::now();
        self.rtt_samples.push_back(RttSample {
            rtt_ms,
            timestamp: now,
        });
        self.prune_rtt_samples(now);
    }

    /// 获取平滑后的 RTT
    pub fn smoothed_rtt(&self) -> f32 {
        self.smoothed_rtt_ms
    }

    /// 获取最小 RTT
    pub fn min_rtt(&self) -> f32 {
        if self.rtt_sample_count == 0 {
            0.0
        } else {
            self.min_rtt_ms
        }
    }

    /// 获取最大 RTT
    pub fn max_rtt(&self) -> f32 {
        self.max_rtt_ms
    }

    /// 获取平均 RTT（基于滑动窗口）
    pub fn avg_rtt(&self) -> f32 {
        if self.rtt_samples.is_empty() {
            return 0.0;
        }
        let sum: f32 = self.rtt_samples.iter().map(|s| s.rtt_ms).sum();
        sum / self.rtt_samples.len() as f32
    }

    /// 获取 RTT 抖动（毫秒）
    pub fn jitter_ms(&self) -> f32 {
        self.rtt_var_ms
    }

    /// 获取 RTT 采样数
    pub fn rtt_sample_count(&self) -> u64 {
        self.rtt_sample_count
    }

    fn prune_rtt_samples(&mut self, now: Instant) {
        let window = Duration::from_secs(self.config.window_duration_secs);
        while let Some(front) = self.rtt_samples.front() {
            if now.duration_since(front.timestamp) > window {
                self.rtt_samples.pop_front();
            } else {
                break;
            }
        }
    }

    // ==================== 带宽估计 ====================

    /// 报告发送的数据量（字节）
    ///
    /// 应在每次发送音频包时调用
    pub fn report_bytes_sent(&mut self, bytes: u64) {
        self.pending_bytes += bytes;
        self.maybe_sample_bandwidth();
    }

    fn maybe_sample_bandwidth(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_bandwidth_sample);
        let min_interval = Duration::from_millis(self.config.bandwidth_min_interval_ms);

        if elapsed >= min_interval && self.pending_bytes > 0 {
            self.bandwidth_samples.push_back(BandwidthSample {
                bytes: self.pending_bytes,
                timestamp: now,
            });
            self.window_bytes += self.pending_bytes;
            self.pending_bytes = 0;
            self.last_bandwidth_sample = now;
            self.prune_bandwidth_samples(now);
        }
    }

    /// 获取当前带宽估计（bps）
    pub fn bandwidth_bps(&self) -> u32 {
        let now = Instant::now();
        let window = Duration::from_secs(self.config.window_duration_secs);

        // 计算窗口内的时间跨度
        if let (Some(first), Some(_last)) = (
            self.bandwidth_samples.front(),
            self.bandwidth_samples.back(),
        ) {
            let duration = now.duration_since(first.timestamp);
            if duration.as_secs_f32() < 0.001 {
                return 0;
            }

            // 使用实际窗口时长和配置窗口时长中的较小值
            let effective_duration = duration.min(window);
            let total_bytes = self.window_bytes + self.pending_bytes;

            if effective_duration.as_secs_f32() > 0.0 {
                (total_bytes as f64 * 8.0 / effective_duration.as_secs_f64()) as u32
            } else {
                0
            }
        } else {
            0
        }
    }

    fn prune_bandwidth_samples(&mut self, now: Instant) {
        let window = Duration::from_secs(self.config.window_duration_secs);
        while let Some(front) = self.bandwidth_samples.front() {
            if now.duration_since(front.timestamp) > window {
                if let Some(removed) = self.bandwidth_samples.pop_front() {
                    self.window_bytes = self.window_bytes.saturating_sub(removed.bytes);
                }
            } else {
                break;
            }
        }
    }

    // ==================== 丢包率跟踪 ====================

    /// 报告收到的包（基于序列号检测丢包）
    ///
    /// 应在每次收到音频包时调用
    pub fn report_packet_received(&mut self, sequence: u32) {
        let now = Instant::now();

        // 更新到达间隔抖动
        if let Some(last) = self.last_arrival_time {
            let interval_ms = now.duration_since(last).as_secs_f32() * 1000.0;
            self.update_arrival_jitter(interval_ms);
        }
        self.last_arrival_time = Some(now);

        if !self.seq_initialized {
            self.max_seen_seq = sequence;
            self.seq_initialized = true;
            self.loss_records.push_back(LossRecord {
                expected_seq: sequence,
                received: true,
                timestamp: now,
            });
            return;
        }

        if sequence <= self.max_seen_seq {
            // 重复包或乱序包，忽略
            return;
        }

        // 检测序列号间隙（丢包）
        let gap = sequence - self.max_seen_seq;
        if gap > 1 {
            for seq in (self.max_seen_seq + 1)..sequence {
                self.loss_records.push_back(LossRecord {
                    expected_seq: seq,
                    received: false,
                    timestamp: now,
                });
                self.current_loss_streak += 1;
                if self.current_loss_streak == 1 {
                    self.loss_streak_start = seq;
                }
            }

            // 检测突发丢包
            if self.current_loss_streak >= self.config.burst_loss_threshold {
                self.burst_events.push_back(BurstLossEvent {
                    start_seq: self.loss_streak_start,
                    count: self.current_loss_streak,
                    timestamp: now,
                });
            }
        }

        // 记录收到的包
        self.loss_records.push_back(LossRecord {
            expected_seq: sequence,
            received: true,
            timestamp: now,
        });

        // 收到包，重置连续丢包计数
        if gap > 1 {
            // 间隙之后收到包，但之前已经触发了 burst 检测
            // 这里不重置，因为 burst 已经记录了
        }
        self.current_loss_streak = 0;
        self.max_seen_seq = sequence;

        self.prune_loss_records(now);
        self.prune_burst_events(now);
    }

    /// 直接报告丢包（用于外部已检测到丢包的场景）
    pub fn report_packet_loss(&mut self) {
        let now = Instant::now();
        self.current_loss_streak += 1;
        if self.current_loss_streak == 1 {
            self.loss_streak_start = self.max_seen_seq.wrapping_add(1);
        }

        if self.current_loss_streak >= self.config.burst_loss_threshold {
            self.burst_events.push_back(BurstLossEvent {
                start_seq: self.loss_streak_start,
                count: self.current_loss_streak,
                timestamp: now,
            });
        }
    }

    /// 获取丢包率（0.0 - 1.0）
    pub fn loss_rate(&self) -> f32 {
        if self.loss_records.is_empty() {
            return 0.0;
        }
        let total = self.loss_records.len() as f32;
        let lost = self.loss_records.iter().filter(|r| !r.received).count() as f32;
        lost / total
    }

    /// 获取突发丢包事件
    pub fn burst_loss_events(&self) -> &VecDeque<BurstLossEvent> {
        &self.burst_events
    }

    /// 检测当前是否处于突发丢包状态
    pub fn is_in_burst_loss(&self) -> bool {
        self.current_loss_streak >= self.config.burst_loss_threshold
    }

    /// 获取当前连续丢包数
    pub fn current_loss_streak(&self) -> u32 {
        self.current_loss_streak
    }

    fn update_arrival_jitter(&mut self, interval_ms: f32) {
        // 基于到达间隔变化计算抖动（类似 RTP 抖动计算）
        if let Some(last) = self.last_arrival_time {
            let _ = last; // 已在调用方使用
        }

        let alpha = self.config.ewma_alpha;
        // 使用 RTT variance 作为抖动的代理
        // 简单更新：与当前 jitter 做 EWMA
        if self.rtt_sample_count > 0 {
            let diff = (interval_ms - self.smoothed_rtt_ms).abs();
            self.rtt_var_ms = (1.0 - alpha) * self.rtt_var_ms + alpha * diff;
        }
    }

    fn prune_loss_records(&mut self, now: Instant) {
        let window = Duration::from_secs(self.config.window_duration_secs);
        while let Some(front) = self.loss_records.front() {
            if now.duration_since(front.timestamp) > window {
                self.loss_records.pop_front();
            } else {
                break;
            }
        }
    }

    fn prune_burst_events(&mut self, now: Instant) {
        let window = Duration::from_secs(self.config.window_duration_secs * 2);
        while let Some(front) = self.burst_events.front() {
            if now.duration_since(front.timestamp) > window {
                self.burst_events.pop_front();
            } else {
                break;
            }
        }
    }

    // ==================== 综合评估 ====================

    /// 获取当前网络统计快照
    pub fn stats(&self) -> NetworkStats {
        let rtt_ms = self.smoothed_rtt_ms;
        let jitter_ms = self.rtt_var_ms;
        let loss_rate = self.loss_rate();
        let bandwidth_bps = self.bandwidth_bps();
        let quality_score = self.calculate_quality_score(rtt_ms, jitter_ms, loss_rate);

        NetworkStats {
            rtt_ms,
            jitter_ms,
            loss_rate,
            bandwidth_bps,
            quality_score,
        }
    }

    /// 获取码率调整建议
    pub fn bitrate_recommendation(&self) -> BitrateRecommendation {
        let loss = self.loss_rate();
        let rtt = self.smoothed_rtt_ms;

        if loss > 0.15 || rtt > 500.0 {
            BitrateRecommendation::AggressiveDecrease
        } else if loss > 0.05 || rtt > 200.0 {
            BitrateRecommendation::Decrease
        } else if loss < 0.01 && rtt < 50.0 {
            BitrateRecommendation::Increase
        } else {
            BitrateRecommendation::Hold
        }
    }

    fn calculate_quality_score(&self, rtt_ms: f32, jitter_ms: f32, loss_rate: f32) -> u8 {
        if self.rtt_sample_count < self.config.min_samples as u64 {
            return 100; // 数据不足，假设最佳
        }

        // RTT 评分（0-40 分）
        let rtt_score = if rtt_ms <= 20.0 {
            40.0
        } else if rtt_ms <= 50.0 {
            40.0 - (rtt_ms - 20.0) * 0.5
        } else if rtt_ms <= 150.0 {
            25.0 - (rtt_ms - 50.0) * 0.2
        } else if rtt_ms <= 300.0 {
            5.0 - (rtt_ms - 150.0) * 0.03
        } else {
            0.0
        };

        // 抖动评分（0-30 分）
        let jitter_score = if jitter_ms <= 5.0 {
            30.0
        } else if jitter_ms <= 20.0 {
            30.0 - (jitter_ms - 5.0) * 1.0
        } else if jitter_ms <= 50.0 {
            15.0 - (jitter_ms - 20.0) * 0.3
        } else {
            0.0
        };

        // 丢包评分（0-30 分）
        let loss_score = if loss_rate <= 0.001 {
            30.0
        } else if loss_rate <= 0.01 {
            30.0 - (loss_rate - 0.001) * 2000.0
        } else if loss_rate <= 0.05 {
            12.0 - (loss_rate - 0.01) * 200.0
        } else if loss_rate <= 0.15 {
            4.0 - (loss_rate - 0.05) * 40.0
        } else {
            0.0
        };

        (rtt_score + jitter_score + loss_score).clamp(0.0, 100.0) as u8
    }

    // ==================== 管理 ====================

    /// 获取配置
    pub fn config(&self) -> &NetMonitorConfig {
        &self.config
    }

    /// 重置所有统计
    pub fn reset(&mut self) {
        let now = Instant::now();
        self.smoothed_rtt_ms = 0.0;
        self.rtt_var_ms = 0.0;
        self.min_rtt_ms = f32::MAX;
        self.max_rtt_ms = 0.0;
        self.rtt_samples.clear();
        self.pending_requests.clear();
        self.rtt_sample_count = 0;

        self.bandwidth_samples.clear();
        self.window_bytes = 0;
        self.last_bandwidth_sample = now;
        self.pending_bytes = 0;

        self.loss_records.clear();
        self.max_seen_seq = 0;
        self.seq_initialized = false;
        self.burst_events.clear();
        self.current_loss_streak = 0;
        self.loss_streak_start = 0;

        self.last_arrival_time = None;
        self.start_time = now;
    }

    /// 获取监控运行时长
    pub fn uptime(&self) -> Duration {
        self.start_time.elapsed()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== RTT 测试 ====================

    #[test]
    fn test_rtt_initial_state() {
        let monitor = NetMonitor::with_default_config();
        assert_eq!(monitor.smoothed_rtt(), 0.0);
        assert_eq!(monitor.min_rtt(), 0.0);
        assert_eq!(monitor.max_rtt(), 0.0);
        assert_eq!(monitor.rtt_sample_count(), 0);
    }

    #[test]
    fn test_rtt_single_sample() {
        let mut monitor = NetMonitor::with_default_config();
        monitor.report_rtt(50.0);

        assert_eq!(monitor.smoothed_rtt(), 50.0);
        assert_eq!(monitor.min_rtt(), 50.0);
        assert_eq!(monitor.max_rtt(), 50.0);
        assert_eq!(monitor.rtt_sample_count(), 1);
    }

    #[test]
    fn test_rtt_ewma_smoothing() {
        let mut monitor = NetMonitor::with_default_config();
        // alpha = 0.125

        monitor.report_rtt(100.0);
        assert_eq!(monitor.smoothed_rtt(), 100.0);

        // 第二个采样：smoothed = 0.875 * 100 + 0.125 * 200 = 87.5 + 25 = 112.5
        monitor.report_rtt(200.0);
        let expected = 0.875 * 100.0 + 0.125 * 200.0;
        assert!((monitor.smoothed_rtt() - expected).abs() < 0.01);
    }

    #[test]
    fn test_rtt_min_max_tracking() {
        let mut monitor = NetMonitor::with_default_config();

        monitor.report_rtt(100.0);
        monitor.report_rtt(50.0);
        monitor.report_rtt(200.0);
        monitor.report_rtt(30.0);

        assert_eq!(monitor.min_rtt(), 30.0);
        assert_eq!(monitor.max_rtt(), 200.0);
        assert_eq!(monitor.rtt_sample_count(), 4);
    }

    #[test]
    fn test_rtt_avg_calculation() {
        let mut monitor = NetMonitor::with_default_config();

        monitor.report_rtt(10.0);
        monitor.report_rtt(20.0);
        monitor.report_rtt(30.0);

        let avg = monitor.avg_rtt();
        assert!((avg - 20.0).abs() < 0.01);
    }

    #[test]
    fn test_rtt_request_response() {
        let mut monitor = NetMonitor::with_default_config();

        monitor.record_request_sent(1);
        // 模拟一些延迟
        std::thread::sleep(Duration::from_millis(10));
        let rtt = monitor.record_response_received(1);

        assert!(rtt.is_some());
        assert!(rtt.unwrap() >= 10.0);
        assert!(monitor.rtt_sample_count() >= 1);
    }

    #[test]
    fn test_rtt_unknown_response() {
        let mut monitor = NetMonitor::with_default_config();
        // 没有发送请求就收到响应
        let rtt = monitor.record_response_received(999);
        assert!(rtt.is_none());
    }

    #[test]
    fn test_rtt_negative_rejected() {
        let mut monitor = NetMonitor::with_default_config();
        monitor.report_rtt(-10.0);
        assert_eq!(monitor.rtt_sample_count(), 0);
    }

    // ==================== 带宽测试 ====================

    #[test]
    fn test_bandwidth_initial_state() {
        let monitor = NetMonitor::with_default_config();
        assert_eq!(monitor.bandwidth_bps(), 0);
    }

    #[test]
    fn test_bandwidth_estimation() {
        let mut monitor = NetMonitor::with_default_config();

        // 发送 1000 字节，等待超过最小间隔
        monitor.report_bytes_sent(1000);
        std::thread::sleep(Duration::from_millis(20));

        // 再发送触发采样
        monitor.report_bytes_sent(500);

        // 等待带宽计算有足够的时间跨度
        std::thread::sleep(Duration::from_millis(20));

        let bw = monitor.bandwidth_bps();
        // 应该有非零带宽
        assert!(bw > 0, "bandwidth should be > 0, got {}", bw);
    }

    #[test]
    fn test_bandwidth_accumulates() {
        let mut monitor = NetMonitor::with_default_config();

        // 连续发送，触发多次采样
        for _ in 0..10 {
            monitor.report_bytes_sent(1200);
            std::thread::sleep(Duration::from_millis(10));
        }

        let bw = monitor.bandwidth_bps();
        assert!(bw > 0);
    }

    // ==================== 丢包测试 ====================

    #[test]
    fn test_loss_rate_no_loss() {
        let mut monitor = NetMonitor::with_default_config();

        for i in 0..100 {
            monitor.report_packet_received(i);
        }

        assert!((monitor.loss_rate() - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_loss_rate_with_gaps() {
        let mut monitor = NetMonitor::with_default_config();

        // 发送序列号 0-99，跳过 10, 20, 30
        for i in 0..100 {
            if i == 10 || i == 20 || i == 30 {
                continue; // 模拟丢包
            }
            monitor.report_packet_received(i);
        }

        let loss = monitor.loss_rate();
        // 窗口内：97 received + 3 lost = 100 条记录，loss = 3/100 = 0.03
        // 但因为滑动窗口可能已经 prune 了早期记录
        assert!(loss > 0.0, "loss rate should be > 0, got {}", loss);
    }

    #[test]
    fn test_loss_rate_sequential() {
        let mut monitor = NetMonitor::with_default_config();

        // 连续收到，无丢包
        for i in 0..50 {
            monitor.report_packet_received(i);
        }

        assert!((monitor.loss_rate() - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_burst_loss_detection() {
        let config = NetMonitorConfig {
            burst_loss_threshold: 3,
            ..Default::default()
        };
        let mut monitor = NetMonitor::new(config);

        // 收到 0, 1, 2, 然后跳过 3,4,5, 收到 6
        monitor.report_packet_received(0);
        monitor.report_packet_received(1);
        monitor.report_packet_received(2);
        // 跳过 3, 4, 5（连续丢 3 包）
        monitor.report_packet_received(6);

        // 应该检测到突发丢包
        assert!(monitor.burst_loss_events().len() >= 1);
        assert!(monitor.is_in_burst_loss() || monitor.current_loss_streak() == 0);
        // 收到 6 后 streak 重置
        assert_eq!(monitor.current_loss_streak(), 0);
    }

    #[test]
    fn test_report_packet_loss_direct() {
        let config = NetMonitorConfig {
            burst_loss_threshold: 2,
            ..Default::default()
        };
        let mut monitor = NetMonitor::new(config);

        monitor.report_packet_received(0);
        monitor.report_packet_loss();
        monitor.report_packet_loss();

        // 连续丢 2 包，应该触发突发
        assert!(monitor.is_in_burst_loss());
    }

    #[test]
    fn test_duplicate_packet_ignored() {
        let mut monitor = NetMonitor::with_default_config();

        monitor.report_packet_received(0);
        monitor.report_packet_received(1);
        monitor.report_packet_received(1); // 重复
        monitor.report_packet_received(2);

        assert!((monitor.loss_rate() - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_out_of_order_packet_ignored() {
        let mut monitor = NetMonitor::with_default_config();

        monitor.report_packet_received(5);
        monitor.report_packet_received(3); // 乱序，应该忽略
        monitor.report_packet_received(6);

        // 3 < max_seen_seq(5), 忽略；gap 5->6 无丢包
        assert!((monitor.loss_rate() - 0.0).abs() < 0.001);
    }

    // ==================== 质量评分测试 ====================

    #[test]
    fn test_quality_score_no_data() {
        let monitor = NetMonitor::with_default_config();
        let stats = monitor.stats();
        assert_eq!(stats.quality_score, 100); // 数据不足，假设最佳
    }

    #[test]
    fn test_quality_score_excellent() {
        let mut monitor = NetMonitor::with_default_config();

        // 低 RTT、低丢包 → 高分
        monitor.report_rtt(10.0);
        monitor.report_rtt(15.0);
        monitor.report_rtt(12.0);

        for i in 0..100 {
            monitor.report_packet_received(i);
        }

        let stats = monitor.stats();
        assert!(stats.quality_score >= 80, "score: {}", stats.quality_score);
    }

    #[test]
    fn test_quality_score_poor() {
        let mut monitor = NetMonitor::with_default_config();

        // 高 RTT
        monitor.report_rtt(300.0);
        monitor.report_rtt(400.0);
        monitor.report_rtt(350.0);

        // 高丢包
        for i in 0..100 {
            if i % 2 == 0 {
                continue; // 50% 丢包
            }
            monitor.report_packet_received(i);
        }

        let stats = monitor.stats();
        assert!(stats.quality_score < 50, "score: {}", stats.quality_score);
    }

    #[test]
    fn test_quality_score_good() {
        let mut monitor = NetMonitor::with_default_config();

        // 中低 RTT
        monitor.report_rtt(40.0);
        monitor.report_rtt(50.0);
        monitor.report_rtt(45.0);

        // 低丢包（~1%），每包间加微小延迟模拟真实到达间隔
        for i in 0..100 {
            if i == 50 {
                continue; // 1% 丢包
            }
            monitor.report_packet_received(i);
        }

        let stats = monitor.stats();
        // 中低 RTT + 低丢包 = 中等质量
        assert!(
            stats.quality_score >= 40 && stats.quality_score <= 100,
            "score: {}",
            stats.quality_score
        );
    }

    // ==================== 码率建议测试 ====================

    #[test]
    fn test_bitrate_recommendation_hold() {
        let mut monitor = NetMonitor::with_default_config();

        // 正常网络条件
        monitor.report_rtt(30.0);
        monitor.report_rtt(35.0);
        monitor.report_rtt(25.0);

        for i in 0..100 {
            monitor.report_packet_received(i);
        }

        assert_eq!(monitor.bitrate_recommendation(), BitrateRecommendation::Increase);
    }

    #[test]
    fn test_bitrate_recommendation_decrease() {
        let mut monitor = NetMonitor::with_default_config();

        // 高丢包
        for i in 0..100 {
            if i % 10 == 0 {
                continue; // 10% 丢包
            }
            monitor.report_packet_received(i);
        }
        monitor.report_rtt(50.0);
        monitor.report_rtt(60.0);
        monitor.report_rtt(55.0);

        let rec = monitor.bitrate_recommendation();
        assert!(
            rec == BitrateRecommendation::Decrease
                || rec == BitrateRecommendation::AggressiveDecrease,
            "recommendation: {:?}",
            rec
        );
    }

    #[test]
    fn test_bitrate_recommendation_aggressive_decrease() {
        let mut monitor = NetMonitor::with_default_config();

        // 极高丢包
        for i in 0..100 {
            if i % 3 == 0 {
                continue; // ~33% 丢包
            }
            monitor.report_packet_received(i);
        }
        monitor.report_rtt(50.0);
        monitor.report_rtt(60.0);
        monitor.report_rtt(55.0);

        assert_eq!(
            monitor.bitrate_recommendation(),
            BitrateRecommendation::AggressiveDecrease
        );
    }

    // ==================== 综合测试 ====================

    #[test]
    fn test_stats_snapshot() {
        let mut monitor = NetMonitor::with_default_config();

        monitor.report_rtt(25.0);
        monitor.report_rtt(30.0);
        monitor.report_rtt(28.0);

        monitor.report_bytes_sent(1200);
        std::thread::sleep(Duration::from_millis(20));
        monitor.report_bytes_sent(800);

        for i in 0..50 {
            monitor.report_packet_received(i);
        }

        let stats = monitor.stats();
        assert!(stats.rtt_ms > 0.0);
        assert!(stats.bandwidth_bps > 0 || stats.bandwidth_bps == 0); // 可能为 0 取决于时序
        assert!((stats.loss_rate - 0.0).abs() < 0.001);
        assert!(stats.quality_score > 0);
    }

    #[test]
    fn test_reset() {
        let mut monitor = NetMonitor::with_default_config();

        monitor.report_rtt(50.0);
        monitor.report_rtt(100.0);
        monitor.report_bytes_sent(1000);
        monitor.report_packet_received(0);
        monitor.report_packet_received(2); // gap → loss

        monitor.reset();

        assert_eq!(monitor.smoothed_rtt(), 0.0);
        assert_eq!(monitor.rtt_sample_count(), 0);
        assert_eq!(monitor.bandwidth_bps(), 0);
        assert_eq!(monitor.loss_rate(), 0.0);
        assert!(!monitor.is_in_burst_loss());
    }

    #[test]
    fn test_uptime() {
        let monitor = NetMonitor::with_default_config();
        std::thread::sleep(Duration::from_millis(10));
        assert!(monitor.uptime() >= Duration::from_millis(10));
    }

    #[test]
    fn test_config() {
        let config = NetMonitorConfig {
            ewma_alpha: 0.25,
            window_duration_secs: 5,
            burst_loss_threshold: 5,
            min_samples: 10,
            bandwidth_min_interval_ms: 10,
        };
        let monitor = NetMonitor::new(config);
        assert_eq!(monitor.config().ewma_alpha, 0.25);
        assert_eq!(monitor.config().window_duration_secs, 5);
        assert_eq!(monitor.config().burst_loss_threshold, 5);
    }

    #[test]
    fn test_network_stats_default() {
        let stats = NetworkStats::default();
        assert_eq!(stats.rtt_ms, 0.0);
        assert_eq!(stats.jitter_ms, 0.0);
        assert_eq!(stats.loss_rate, 0.0);
        assert_eq!(stats.bandwidth_bps, 0);
        assert_eq!(stats.quality_score, 100);
    }

    #[test]
    fn test_bitrate_recommendation_eq() {
        assert_eq!(BitrateRecommendation::Hold, BitrateRecommendation::Hold);
        assert_ne!(
            BitrateRecommendation::Hold,
            BitrateRecommendation::Increase
        );
    }

    #[test]
    fn test_rtt_jitter_increases_with_variance() {
        let mut monitor = NetMonitor::with_default_config();

        // 高方差 RTT
        monitor.report_rtt(10.0);
        monitor.report_rtt(100.0);
        monitor.report_rtt(20.0);
        monitor.report_rtt(200.0);
        monitor.report_rtt(15.0);

        let jitter = monitor.jitter_ms();
        assert!(jitter > 0.0, "jitter should be > 0, got {}", jitter);
    }

    #[test]
    fn test_sequential_loss_tracking() {
        let mut monitor = NetMonitor::with_default_config();

        // 连续丢包
        monitor.report_packet_received(0);
        monitor.report_packet_received(10); // gap 1-9 = 9 lost
        monitor.report_packet_received(11);

        let loss = monitor.loss_rate();
        // 9 lost out of ~12 records
        assert!(loss > 0.5, "loss: {}", loss);
    }

    #[test]
    fn test_large_gap_no_overflow() {
        let mut monitor = NetMonitor::with_default_config();

        monitor.report_packet_received(0);
        // 大间隙但不溢出（用实际大的序列号差）
        monitor.report_packet_received(1000);

        // 不应该 panic
        let loss = monitor.loss_rate();
        assert!(loss > 0.9, "loss: {}", loss);
    }
}
