//! # PID 带宽控制器
//!
//! 基于 PID（比例-积分-微分）算法的自适应带宽控制器，
//! 替代原有的 3 级硬阈值码率切换策略。
//!
//! ## 设计原理
//!
//! PID 控制器通过三个分量协同工作：
//! - **P（比例）**：根据当前误差快速响应
//! - **I（积分）**：消除稳态误差，处理持续偏差
//! - **D（微分）**：预测趋势，抑制振荡
//!
//! ## 误差计算
//!
//! 综合考虑三个网络指标：
//! - 丢包率误差：`(loss_rate - setpoint_loss) * 100`
//! - 延迟误差：`(latency_ms - setpoint_latency) / 10`
//! - 抖动误差：`jitter_ms / 20`
//!
//! ## 使用示例
//!
//! ```rust
//! use network::bandwidth_pid::*;
//!
//! let mut controller = PidBandwidthController::with_default_config();
//!
//! // 模拟网络恶化
//! let metrics = NetworkMetrics {
//!     loss_rate: 0.05,
//!     latency_ms: 80.0,
//!     jitter_ms: 20.0,
//! };
//!
//! let new_bitrate = controller.update(metrics);
//! println!("调整后码率: {} bps", new_bitrate);
//! ```

use std::time::Instant;

/// PID 控制器配置参数
///
/// 包含 PID 增益系数和网络指标目标值。
///
/// # 默认值
///
/// | 参数 | 默认值 | 说明 |
/// |------|--------|------|
/// | kp | 0.5 | 比例增益 |
/// | ki | 0.1 | 积分增益 |
/// | kd | 0.2 | 微分增益 |
/// | setpoint_loss | 0.02 | 目标丢包率 (2%) |
/// | setpoint_latency | 50.0 | 目标延迟 (ms) |
/// | min_bitrate | 32,000 | 最低码率 (bps) |
/// | max_bitrate | 9,216,000 | 最高码率 (bps) |
#[derive(Debug, Clone)]
pub struct PidConfig {
    /// 比例增益 - 控制对当前误差的响应强度
    pub kp: f32,
    /// 积分增益 - 控制对累积误差的响应强度
    pub ki: f32,
    /// 微分增益 - 控制对误差变化率的响应强度
    pub kd: f32,
    /// 目标丢包率 (0.0 ~ 1.0)
    pub setpoint_loss: f32,
    /// 目标延迟 (ms)
    pub setpoint_latency: f32,
    /// 最低码率 (bps)
    pub min_bitrate: u32,
    /// 最高码率 (bps)
    pub max_bitrate: u32,
}

impl Default for PidConfig {
    fn default() -> Self {
        Self {
            kp: 0.5,
            ki: 0.1,
            kd: 0.2,
            setpoint_loss: 0.02,
            setpoint_latency: 50.0,
            min_bitrate: 32_000,
            max_bitrate: 9_216_000,
        }
    }
}

/// 网络质量指标
///
/// 用于输入 PID 控制器的实时网络状况数据。
#[derive(Debug, Clone, Copy)]
pub struct NetworkMetrics {
    /// 丢包率 (0.0 ~ 1.0)
    pub loss_rate: f32,
    /// 往返延迟 (ms)
    pub latency_ms: f32,
    /// 抖动 (ms)
    pub jitter_ms: f32,
}

/// PID 带宽控制器
///
/// 根据网络质量指标动态调整音频传输码率。
///
/// # 算法流程
///
/// 1. 计算综合误差（丢包 + 延迟 + 抖动）
/// 2. 应用 PID 公式：`output = Kp * e + Ki * ∫e + Kd * de/dt`
/// 3. 调整码率：`bitrate -= output * 1000`
/// 4. 限制码率在 [min_bitrate, max_bitrate] 范围内
///
/// # 示例
///
/// ```rust
/// use network::bandwidth_pid::*;
///
/// let mut controller = PidBandwidthController::new(PidConfig {
///     kp: 0.3,
///     ki: 0.05,
///     kd: 0.15,
///     ..Default::default()
/// });
///
/// // 连续更新
/// for _ in 0..10 {
///     let bitrate = controller.update(NetworkMetrics {
///         loss_rate: 0.01,
///         latency_ms: 30.0,
///         jitter_ms: 5.0,
///     });
/// }
/// ```
pub struct PidBandwidthController {
    config: PidConfig,
    integral: f32,
    prev_error: f32,
    last_update: Instant,
    current_bitrate: u32,
}

impl PidBandwidthController {
    /// 创建新的 PID 控制器
    ///
    /// # Arguments
    ///
    /// * `config` - PID 配置参数
    pub fn new(config: PidConfig) -> Self {
        let initial_bitrate = (config.min_bitrate + config.max_bitrate) / 2;
        Self {
            config,
            integral: 0.0,
            prev_error: 0.0,
            last_update: Instant::now(),
            current_bitrate: initial_bitrate,
        }
    }

    /// 使用默认配置创建控制器
    pub fn with_default_config() -> Self {
        Self::new(PidConfig::default())
    }

    /// 获取当前码率 (bps)
    pub fn current_bitrate(&self) -> u32 {
        self.current_bitrate
    }

    /// 设置码率（自动限制在配置范围内）
    pub fn set_bitrate(&mut self, bitrate: u32) {
        self.current_bitrate = bitrate.clamp(self.config.min_bitrate, self.config.max_bitrate);
    }

    /// 根据网络指标更新码率
    ///
    /// # Arguments
    ///
    /// * `metrics` - 当前网络质量指标
    ///
    /// # Returns
    ///
    /// 调整后的码率 (bps)
    pub fn update(&mut self, metrics: NetworkMetrics) -> u32 {
        let now = Instant::now();
        let dt = now.duration_since(self.last_update).as_secs_f32();
        self.last_update = now;

        // 最小间隔 10ms，防止微分项爆炸
        const MIN_DT: f32 = 0.01;
        if dt < MIN_DT {
            return self.current_bitrate;
        }

        // 计算综合误差
        let loss_error = (metrics.loss_rate - self.config.setpoint_loss) * 100.0;
        let latency_error = (metrics.latency_ms - self.config.setpoint_latency) / 10.0;
        let jitter_error = metrics.jitter_ms / 20.0;
        let error = loss_error + latency_error + jitter_error;

        // 积分项（带抗饱和）
        self.integral += error * dt;
        self.integral = self.integral.clamp(-1000.0, 1000.0);

        // 微分项（带限幅，防止 dt 过小时爆炸）
        let raw_derivative = (error - self.prev_error) / dt;
        let derivative = raw_derivative.clamp(-500.0, 500.0);
        self.prev_error = error;

        // PID 输出
        let output = self.config.kp * error
            + self.config.ki * self.integral
            + self.config.kd * derivative;

        // 调整码率
        let new_bitrate = self.current_bitrate as f32 - output * 1000.0;
        self.current_bitrate =
            new_bitrate.clamp(self.config.min_bitrate as f32, self.config.max_bitrate as f32)
                as u32;

        self.current_bitrate
    }

    /// 重置控制器状态
    ///
    /// 清除积分和微分累积，重新开始控制周期。
    pub fn reset(&mut self) {
        self.integral = 0.0;
        self.prev_error = 0.0;
        self.last_update = Instant::now();
    }
}
