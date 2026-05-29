//! 音频播放设备管理

use std::sync::{Arc, Mutex};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use audio_core::{AudioBuffer, RingBuffer};

use crate::config::PlaybackConfig;
use crate::{PlaybackError, Result};

/// 音频设备信息
#[derive(Debug, Clone)]
pub struct DeviceInfo {
    /// 设备名称
    pub name: String,

    /// 是否为默认设备
    pub is_default: bool,

    /// 支持的采样率
    pub sample_rates: Vec<u32>,

    /// 支持的通道数
    pub channels: Vec<u16>,
}

/// 音频播放设备
pub struct PlaybackDevice {
    device: cpal::Device,
    config: PlaybackConfig,
    stream: Option<cpal::Stream>,
    ring_buffer: Arc<RingBuffer<f32>>,
    is_running: Arc<Mutex<bool>>,
}

impl PlaybackDevice {
    /// 列出所有可用的输出设备
    pub fn list_devices() -> Result<Vec<DeviceInfo>> {
        let host = cpal::default_host();
        let default_device = host.default_output_device();
        let default_name = default_device
            .as_ref()
            .and_then(|d| d.name().ok())
            .unwrap_or_default();

        let mut devices = Vec::new();

        for device in host.output_devices().map_err(|e| PlaybackError::StreamError(e.to_string()))? {
            let name = device.name().unwrap_or_else(|_| "未知设备".to_string());
            let is_default = name == default_name;

            // 获取支持的配置
            let mut sample_rates = Vec::new();
            let mut channels = Vec::new();

            if let Ok(configs) = device.supported_output_configs() {
                for config in configs {
                    sample_rates.push(config.min_sample_rate().0);
                    sample_rates.push(config.max_sample_rate().0);
                    channels.push(config.channels());
                }
            }

            // 去重
            sample_rates.sort();
            sample_rates.dedup();
            channels.sort();
            channels.dedup();

            devices.push(DeviceInfo {
                name,
                is_default,
                sample_rates,
                channels,
            });
        }

        Ok(devices)
    }

    /// 获取默认设备信息
    pub fn default_device() -> Result<DeviceInfo> {
        let host = cpal::default_host();
        let device = host.default_output_device()
            .ok_or(PlaybackError::DeviceNotFound("无默认输出设备".to_string()))?;

        let name = device.name().unwrap_or_else(|_| "未知设备".to_string());

        let mut sample_rates = Vec::new();
        let mut channels = Vec::new();

        if let Ok(configs) = device.supported_output_configs() {
            for config in configs {
                sample_rates.push(config.min_sample_rate().0);
                sample_rates.push(config.max_sample_rate().0);
                channels.push(config.channels());
            }
        }

        sample_rates.sort();
        sample_rates.dedup();
        channels.sort();
        channels.dedup();

        Ok(DeviceInfo {
            name,
            is_default: true,
            sample_rates,
            channels,
        })
    }

    /// 创建新的播放设备
    pub fn new(device_info: &DeviceInfo, config: PlaybackConfig) -> Result<Self> {
        let host = cpal::default_host();

        // 查找设备
        let device = host
            .output_devices()
            .map_err(|e| PlaybackError::StreamError(e.to_string()))?
            .find(|d| d.name().map(|n| n == device_info.name).unwrap_or(false))
            .ok_or(PlaybackError::DeviceNotFound(device_info.name.clone()))?;

        // 验证配置
        let supported = device
            .supported_output_configs()
            .map_err(|e| PlaybackError::StreamError(e.to_string()))?
            .any(|c| {
                c.channels() == config.channels
                    && c.min_sample_rate().0 <= config.sample_rate
                    && c.max_sample_rate().0 >= config.sample_rate
            });

        if !supported {
            return Err(PlaybackError::ConfigNotSupported(format!(
                "设备 {} 不支持配置: {}Hz, {}ch",
                device_info.name, config.sample_rate, config.channels
            )));
        }

        // 创建 ring buffer（容量为缓冲区大小的 4 倍）
        let ring_buffer = Arc::new(RingBuffer::new(config.buffer_size as usize * 4));

        Ok(Self {
            device,
            config,
            stream: None,
            ring_buffer,
            is_running: Arc::new(Mutex::new(false)),
        })
    }

    /// 使用默认设备创建
    pub fn new_default(config: PlaybackConfig) -> Result<Self> {
        let device_info = Self::default_device()?;
        Self::new(&device_info, config)
    }

    /// 开始播放
    pub fn start(&mut self) -> Result<()> {
        if self.stream.is_some() {
            return Ok(()); // 已经在运行
        }

        let stream_config = cpal::StreamConfig {
            channels: self.config.channels,
            sample_rate: cpal::SampleRate(self.config.sample_rate),
            buffer_size: cpal::BufferSize::Default,
        };

        let ring_buffer = self.ring_buffer.clone();

        let stream = self.device.build_output_stream(
            &stream_config,
            move |data: &mut [f32], _info: &cpal::OutputCallbackInfo| {
                // 从 ring buffer 读取数据
                let read = ring_buffer.read(data);
                // 如果数据不足，填充静音
                if read < data.len() {
                    data[read..].fill(0.0);
                }
            },
            |err| {
                tracing::error!("播放流错误: {}", err);
            },
            None,
        )?;

        stream.play()?;
        self.stream = Some(stream);
        if let Ok(mut running) = self.is_running.lock() {
            *running = true;
        }

        Ok(())
    }

    /// 停止播放
    pub fn stop(&mut self) -> Result<()> {
        if let Some(stream) = self.stream.take() {
            drop(stream);
        }
        if let Ok(mut running) = self.is_running.lock() {
            *running = false;
        }
        self.ring_buffer.clear();
        Ok(())
    }

    /// 检查是否正在播放
    pub fn is_running(&self) -> bool {
        self.is_running.lock().map(|r| *r).unwrap_or(false)
    }

    /// 写入音频数据
    pub fn write(&self, buffer: &AudioBuffer<f32>) -> Result<()> {
        let samples = buffer.samples();
        let written = self.ring_buffer.write(samples);

        if written < samples.len() {
            tracing::warn!("播放缓冲区溢出，丢失 {} 个样本", samples.len() - written);
        }

        Ok(())
    }

    /// 获取配置
    pub fn config(&self) -> &PlaybackConfig {
        &self.config
    }

    /// 获取设备名称
    pub fn device_name(&self) -> String {
        self.device.name().unwrap_or_else(|_| "未知设备".to_string())
    }
}

impl Drop for PlaybackDevice {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}
