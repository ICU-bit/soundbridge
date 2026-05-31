//! # 断线重连模块
//!
//! 提供自动断线重连机制，包括：
//! - 心跳超时检测
//! - 指数退避重试策略
//! - 最大重试次数限制
//! - 重连状态机
//!
//! ## 工作流程
//!
//! ```text
//! Connected → Disconnected (heartbeat timeout)
//!   → Reconnecting (attempt 1, delay 1s)
//!   → Connected (success)
//!   OR
//!   → Reconnecting (attempt 2, delay 2s)
//!   → ...
//!   → Exhausted (max retries reached)
//! ```
//!
//! ## 指数退避
//!
//! 重连延迟按 2 的指数递增：1s → 2s → 4s → 8s → 16s → 32s（上限）

use crate::Result;
use std::time::{Duration, Instant};

/// 默认初始重连延迟（毫秒）
pub const DEFAULT_INITIAL_BACKOFF_MS: u64 = 1_000;

/// 默认最大重试次数
pub const DEFAULT_MAX_RETRIES: u32 = 10;

/// 默认最大退避延迟（毫秒），约 32 秒
pub const DEFAULT_MAX_BACKOFF_MS: u64 = 32_000;

/// 默认心跳超时检测间隔（毫秒）
pub const DEFAULT_HEARTBEAT_CHECK_INTERVAL_MS: u64 = 1_000;

/// 重连状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReconnectState {
    /// 空闲（未连接过）
    Idle,
    /// 已连接
    Connected,
    /// 检测到断开
    Disconnected,
    /// 正在重连
    Reconnecting,
    /// 重连成功
    Recovered,
    /// 重连次数耗尽
    Exhausted,
}

impl std::fmt::Display for ReconnectState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Idle => write!(f, "Idle"),
            Self::Connected => write!(f, "Connected"),
            Self::Disconnected => write!(f, "Disconnected"),
            Self::Reconnecting => write!(f, "Reconnecting"),
            Self::Recovered => write!(f, "Recovered"),
            Self::Exhausted => write!(f, "Exhausted"),
        }
    }
}

/// 重连配置
#[derive(Debug, Clone)]
pub struct ReconnectConfig {
    /// 初始退避延迟（毫秒）
    pub initial_backoff_ms: u64,
    /// 最大退避延迟（毫秒）
    pub max_backoff_ms: u64,
    /// 最大重试次数
    pub max_retries: u32,
    /// 心跳超时时间（毫秒），超过此时间认为连接断开
    pub heartbeat_timeout_ms: u64,
    /// 心跳检测间隔（毫秒）
    pub heartbeat_check_interval_ms: u64,
}

impl Default for ReconnectConfig {
    fn default() -> Self {
        Self {
            initial_backoff_ms: DEFAULT_INITIAL_BACKOFF_MS,
            max_backoff_ms: DEFAULT_MAX_BACKOFF_MS,
            max_retries: DEFAULT_MAX_RETRIES,
            heartbeat_timeout_ms: crate::session::HEARTBEAT_TIMEOUT_MS,
            heartbeat_check_interval_ms: DEFAULT_HEARTBEAT_CHECK_INTERVAL_MS,
        }
    }
}

/// 重连统计信息
#[derive(Debug, Clone, Default)]
pub struct ReconnectStats {
    /// 总重连尝试次数
    pub total_attempts: u32,
    /// 成功重连次数
    pub successful_reconnects: u32,
    /// 失败重连次数
    pub failed_reconnects: u32,
    /// 最后一次重连耗时（毫秒）
    pub last_reconnect_duration_ms: Option<u64>,
    /// 当前退避延迟（毫秒）
    pub current_backoff_ms: u64,
}

/// 重连管理器
///
/// 管理连接断开后的自动重连逻辑。支持心跳超时检测和指数退避重试。
///
/// # 示例
///
/// ```rust
/// use network::reconnect::{ReconnectManager, ReconnectConfig};
///
/// let config = ReconnectConfig::default();
/// let mut manager = ReconnectManager::new(config);
///
/// // 模拟连接建立
/// manager.on_connected();
/// assert_eq!(manager.state(), network::reconnect::ReconnectState::Connected);
///
/// // 模拟心跳丢失（手动标记断开）
/// manager.on_disconnected();
/// assert_eq!(manager.state(), network::reconnect::ReconnectState::Disconnected);
///
/// // 检查是否需要重连
/// assert!(manager.should_reconnect());
/// ```
pub struct ReconnectManager {
    /// 当前状态
    state: ReconnectState,
    /// 配置
    config: ReconnectConfig,
    /// 当前重试次数
    retry_count: u32,
    /// 当前退避延迟
    current_backoff_ms: u64,
    /// 最后一次心跳时间
    last_heartbeat: Option<Instant>,
    /// 最后一次活动时间
    last_activity: Option<Instant>,
    /// 下一次重连时间
    next_reconnect_at: Option<Instant>,
    /// 重连开始时间（用于计算耗时）
    reconnect_start: Option<Instant>,
    /// 统计信息
    stats: ReconnectStats,
}

impl ReconnectManager {
    /// 创建新的重连管理器
    pub fn new(config: ReconnectConfig) -> Self {
        let initial_backoff = config.initial_backoff_ms;
        Self {
            state: ReconnectState::Idle,
            config,
            retry_count: 0,
            current_backoff_ms: initial_backoff,
            last_heartbeat: None,
            last_activity: None,
            next_reconnect_at: None,
            reconnect_start: None,
            stats: ReconnectStats {
                current_backoff_ms: initial_backoff,
                ..Default::default()
            },
        }
    }

    /// 获取当前状态
    pub fn state(&self) -> ReconnectState {
        self.state
    }

    /// 获取配置
    pub fn config(&self) -> &ReconnectConfig {
        &self.config
    }

    /// 获取统计信息
    pub fn stats(&self) -> &ReconnectStats {
        &self.stats
    }

    /// 获取当前重试次数
    pub fn retry_count(&self) -> u32 {
        self.retry_count
    }

    /// 当连接建立时调用
    pub fn on_connected(&mut self) {
        self.state = ReconnectState::Connected;
        self.retry_count = 0;
        self.current_backoff_ms = self.config.initial_backoff_ms;
        self.last_heartbeat = Some(Instant::now());
        self.last_activity = Some(Instant::now());
        self.next_reconnect_at = None;

        // 如果是从重连恢复，记录成功
        if self.reconnect_start.is_some() {
            if let Some(start) = self.reconnect_start.take() {
                self.stats.last_reconnect_duration_ms = Some(start.elapsed().as_millis() as u64);
                self.stats.successful_reconnects += 1;
                self.state = ReconnectState::Recovered;
            }
        }

        self.stats.current_backoff_ms = self.current_backoff_ms;
    }

    /// 当连接断开时调用
    pub fn on_disconnected(&mut self) {
        if self.state == ReconnectState::Connected || self.state == ReconnectState::Recovered {
            self.state = ReconnectState::Disconnected;
            self.last_heartbeat = None;
        }
    }

    /// 更新心跳时间（收到对端心跳响应时调用）
    pub fn update_heartbeat(&mut self) {
        self.last_heartbeat = Some(Instant::now());
        self.last_activity = Some(Instant::now());
    }

    /// 检查心跳是否超时
    ///
    /// 当已连接状态且超过心跳超时时间未收到心跳响应时返回 `true`。
    pub fn is_heartbeat_timeout(&self) -> bool {
        if self.state != ReconnectState::Connected && self.state != ReconnectState::Recovered {
            return false;
        }
        if let Some(last) = self.last_heartbeat {
            last.elapsed() >= Duration::from_millis(self.config.heartbeat_timeout_ms)
        } else {
            false
        }
    }

    /// 检查是否需要重连
    ///
    /// 当状态为 `Disconnected` 且未超过最大重试次数时返回 `true`。
    pub fn should_reconnect(&self) -> bool {
        self.state == ReconnectState::Disconnected
            || (self.state == ReconnectState::Reconnecting
                && self.retry_count < self.config.max_retries
                && self.next_reconnect_at.is_none_or(|t| Instant::now() >= t))
    }

    /// 获取当前退避延迟
    pub fn current_backoff(&self) -> Duration {
        Duration::from_millis(self.current_backoff_ms)
    }

    /// 启动一次重连尝试
    ///
    /// 将状态设为 `Reconnecting`，递增重试计数，计算下一次重连时间。
    ///
    /// # Returns
    ///
    /// `Ok(true)` 表示可以尝试重连，`Ok(false)` 表示已耗尽重试次数。
    pub fn start_reconnect(&mut self) -> Result<bool> {
        if self.retry_count >= self.config.max_retries {
            self.state = ReconnectState::Exhausted;
            self.stats.failed_reconnects += 1;
            return Ok(false);
        }

        self.state = ReconnectState::Reconnecting;
        self.retry_count += 1;
        self.stats.total_attempts += 1;

        if self.reconnect_start.is_none() {
            self.reconnect_start = Some(Instant::now());
        }

        // 计算下一次重连时间（指数退避）
        self.next_reconnect_at =
            Some(Instant::now() + Duration::from_millis(self.current_backoff_ms));

        self.stats.current_backoff_ms = self.current_backoff_ms;

        // 递增退避延迟（指数退避，不超过最大值）
        self.current_backoff_ms = (self.current_backoff_ms * 2).min(self.config.max_backoff_ms);

        Ok(true)
    }

    /// 重连失败后的处理
    ///
    /// 将状态回退到 `Disconnected`，等待下一次重连尝试。
    pub fn on_reconnect_failed(&mut self) {
        self.state = ReconnectState::Disconnected;
        self.next_reconnect_at = None;
    }

    /// 重置管理器到初始状态
    pub fn reset(&mut self) {
        self.state = ReconnectState::Idle;
        self.retry_count = 0;
        self.current_backoff_ms = self.config.initial_backoff_ms;
        self.last_heartbeat = None;
        self.last_activity = None;
        self.next_reconnect_at = None;
        self.reconnect_start = None;
        self.stats = ReconnectStats {
            current_backoff_ms: self.config.initial_backoff_ms,
            ..Default::default()
        };
    }

    /// 计算当前应使用的退避延迟（不修改状态）
    ///
    /// 用于外部预估下次重连等待时间。
    pub fn backoff_for_attempt(&self, attempt: u32) -> Duration {
        let mut delay = self.config.initial_backoff_ms;
        for _ in 0..attempt {
            delay = (delay * 2).min(self.config.max_backoff_ms);
        }
        Duration::from_millis(delay)
    }

    /// 定时检查（由外部定时器调用）
    ///
    /// 检查心跳超时，自动触发断开通知。
    ///
    /// # Returns
    ///
    /// `true` 表示检测到心跳超时（调用方应标记断开并开始重连）。
    pub fn tick(&mut self) -> bool {
        if self.is_heartbeat_timeout() {
            self.on_disconnected();
            return true;
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_state() {
        let manager = ReconnectManager::new(ReconnectConfig::default());
        assert_eq!(manager.state(), ReconnectState::Idle);
        assert_eq!(manager.retry_count(), 0);
    }

    #[test]
    fn test_connected_state() {
        let mut manager = ReconnectManager::new(ReconnectConfig::default());
        manager.on_connected();
        assert_eq!(manager.state(), ReconnectState::Connected);
        assert!(!manager.is_heartbeat_timeout());
    }

    #[test]
    fn test_disconnect_and_reconnect() {
        let mut manager = ReconnectManager::new(ReconnectConfig::default());
        manager.on_connected();
        manager.on_disconnected();

        assert_eq!(manager.state(), ReconnectState::Disconnected);
        assert!(manager.should_reconnect());

        let can_reconnect = manager.start_reconnect().unwrap();
        assert!(can_reconnect);
        assert_eq!(manager.state(), ReconnectState::Reconnecting);
        assert_eq!(manager.retry_count(), 1);
    }

    #[test]
    fn test_exponential_backoff() {
        let config = ReconnectConfig {
            initial_backoff_ms: 1000,
            max_backoff_ms: 32000,
            ..Default::default()
        };
        let mut manager = ReconnectManager::new(config);

        manager.on_connected();
        manager.on_disconnected();

        // Attempt 1: 1s
        assert_eq!(manager.backoff_for_attempt(0), Duration::from_millis(1000));
        manager.start_reconnect().unwrap();
        manager.on_reconnect_failed();

        // Attempt 2: 2s
        assert_eq!(manager.backoff_for_attempt(1), Duration::from_millis(2000));
        manager.start_reconnect().unwrap();
        manager.on_reconnect_failed();

        // Attempt 3: 4s
        assert_eq!(manager.backoff_for_attempt(2), Duration::from_millis(4000));
        manager.start_reconnect().unwrap();
        manager.on_reconnect_failed();

        // Attempt 4: 8s
        assert_eq!(manager.backoff_for_attempt(3), Duration::from_millis(8000));
    }

    #[test]
    fn test_max_retries_exhausted() {
        let config = ReconnectConfig {
            max_retries: 3,
            ..Default::default()
        };
        let mut manager = ReconnectManager::new(config);

        manager.on_connected();
        manager.on_disconnected();

        // Exhaust all retries
        for _ in 0..3 {
            manager.start_reconnect().unwrap();
            manager.on_reconnect_failed();
        }

        // 4th attempt should fail
        let can_reconnect = manager.start_reconnect().unwrap();
        assert!(!can_reconnect);
        assert_eq!(manager.state(), ReconnectState::Exhausted);
    }

    #[test]
    fn test_backoff_caps_at_max() {
        let config = ReconnectConfig {
            initial_backoff_ms: 1000,
            max_backoff_ms: 8000,
            max_retries: 20,
            ..Default::default()
        };
        let manager = ReconnectManager::new(config);

        // After many attempts, backoff should not exceed max
        let backoff = manager.backoff_for_attempt(20);
        assert!(backoff <= Duration::from_millis(8000));
    }

    #[test]
    fn test_reconnect_success_resets_state() {
        let mut manager = ReconnectManager::new(ReconnectConfig::default());

        manager.on_connected();
        manager.on_disconnected();
        manager.start_reconnect().unwrap();
        manager.on_connected(); // success

        assert_eq!(manager.state(), ReconnectState::Recovered);
        assert_eq!(manager.retry_count(), 0);
        assert_eq!(manager.stats().successful_reconnects, 1);
    }

    #[test]
    fn test_heartbeat_timeout_detection() {
        let config = ReconnectConfig {
            heartbeat_timeout_ms: 50, // 50ms for test
            ..Default::default()
        };
        let mut manager = ReconnectManager::new(config);

        manager.on_connected();
        assert!(!manager.is_heartbeat_timeout());

        // Wait for timeout
        std::thread::sleep(Duration::from_millis(60));
        assert!(manager.is_heartbeat_timeout());
    }

    #[test]
    fn test_tick_detects_timeout() {
        let config = ReconnectConfig {
            heartbeat_timeout_ms: 50,
            ..Default::default()
        };
        let mut manager = ReconnectManager::new(config);

        manager.on_connected();
        assert!(!manager.tick());

        std::thread::sleep(Duration::from_millis(60));
        assert!(manager.tick()); // Should return true and mark disconnected
        assert_eq!(manager.state(), ReconnectState::Disconnected);
    }

    #[test]
    fn test_reset() {
        let mut manager = ReconnectManager::new(ReconnectConfig::default());

        manager.on_connected();
        manager.on_disconnected();
        manager.start_reconnect().unwrap();

        manager.reset();
        assert_eq!(manager.state(), ReconnectState::Idle);
        assert_eq!(manager.retry_count(), 0);
        assert_eq!(manager.stats().total_attempts, 0);
    }

    #[test]
    fn test_reconnect_state_display() {
        assert_eq!(format!("{}", ReconnectState::Idle), "Idle");
        assert_eq!(format!("{}", ReconnectState::Connected), "Connected");
        assert_eq!(format!("{}", ReconnectState::Disconnected), "Disconnected");
        assert_eq!(format!("{}", ReconnectState::Reconnecting), "Reconnecting");
        assert_eq!(format!("{}", ReconnectState::Recovered), "Recovered");
        assert_eq!(format!("{}", ReconnectState::Exhausted), "Exhausted");
    }

    #[test]
    fn test_stats_tracking() {
        let config = ReconnectConfig {
            max_retries: 5,
            ..Default::default()
        };
        let mut manager = ReconnectManager::new(config);

        manager.on_connected();
        manager.on_disconnected();

        // 2 failed attempts
        manager.start_reconnect().unwrap();
        manager.on_reconnect_failed();
        manager.start_reconnect().unwrap();
        manager.on_reconnect_failed();

        assert_eq!(manager.stats().total_attempts, 2);
        assert_eq!(manager.stats().failed_reconnects, 0); // only counts on Exhausted

        // 1 successful
        manager.start_reconnect().unwrap();
        manager.on_connected();

        assert_eq!(manager.stats().total_attempts, 3);
        assert_eq!(manager.stats().successful_reconnects, 1);
    }

    #[test]
    fn test_should_not_reconnect_when_connected() {
        let mut manager = ReconnectManager::new(ReconnectConfig::default());
        manager.on_connected();
        assert!(!manager.should_reconnect());
    }

    #[test]
    fn test_should_not_reconnect_when_exhausted() {
        let config = ReconnectConfig {
            max_retries: 1,
            ..Default::default()
        };
        let mut manager = ReconnectManager::new(config);

        manager.on_connected();
        manager.on_disconnected();
        manager.start_reconnect().unwrap();
        manager.on_reconnect_failed();
        manager.start_reconnect().unwrap(); // marks Exhausted

        assert!(!manager.should_reconnect());
        assert_eq!(manager.state(), ReconnectState::Exhausted);
    }
}
