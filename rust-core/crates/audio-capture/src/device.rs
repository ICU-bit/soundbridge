//! 音频采集设备管理

use std::sync::{Arc, Mutex};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use audio_core::{AudioBuffer, AudioFormat, RingBuffer, SampleFormat};

use crate::config::CaptureConfig;
use crate::{CaptureError, Result};

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

/// 音频采集设备
pub struct CaptureDevice {
    device: cpal::Device,
    config: CaptureConfig,
    stream: Option<cpal::Stream>,
    ring_buffer: Arc<RingBuffer<f32>>,
    is_running: Arc<Mutex<bool>>,
}

impl CaptureDevice {
    /// 列出所有可用的输入设备
    pub fn list_devices() -> Result<Vec<DeviceInfo>> {
        let host = cpal::default_host();
        let default_device = host.default_input_device();
        let default_name = default_device
            .as_ref()
            .and_then(|d| d.name().ok())
            .unwrap_or_default();

        let mut devices = Vec::new();

        for device in host.input_devices().map_err(|e| CaptureError::StreamError(e.to_string()))? {
            let name = device.name().unwrap_or_else(|_| "未知设备".to_string());
            let is_default = name == default_name;

            // 获取支持的配置
            let mut sample_rates = Vec::new();
            let mut channels = Vec::new();

            if let Ok(configs) = device.supported_input_configs() {
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
        let device = host.default_input_device()
            .ok_or(CaptureError::DeviceNotFound("无默认输入设备".to_string()))?;

        let name = device.name().unwrap_or_else(|_| "未知设备".to_string());

        let mut sample_rates = Vec::new();
        let mut channels = Vec::new();

        if let Ok(configs) = device.supported_input_configs() {
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

    /// 创建新的采集设备
    pub fn new(device_info: &DeviceInfo, config: CaptureConfig) -> Result<Self> {
        let host = cpal::default_host();

        // 查找设备
        let device = host
            .input_devices()
            .map_err(|e| CaptureError::StreamError(e.to_string()))?
            .find(|d| d.name().map(|n| n == device_info.name).unwrap_or(false))
            .ok_or(CaptureError::DeviceNotFound(device_info.name.clone()))?;

        // 验证配置
        let supported = device
            .supported_input_configs()
            .map_err(|e| CaptureError::StreamError(e.to_string()))?
            .any(|c| {
                c.channels() == config.channels
                    && c.min_sample_rate().0 <= config.sample_rate
                    && c.max_sample_rate().0 >= config.sample_rate
            });

        if !supported {
            return Err(CaptureError::ConfigNotSupported(format!(
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
    pub fn new_default(config: CaptureConfig) -> Result<Self> {
        let device_info = Self::default_device()?;
        Self::new(&device_info, config)
    }

    /// 开始采集
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
        let _is_running = self.is_running.clone();

        let stream = self.device.build_input_stream(
            &stream_config,
            move |data: &[f32], _info: &cpal::InputCallbackInfo| {
                // 写入 ring buffer
                ring_buffer.write(data);
            },
            |err| {
                tracing::error!("采集流错误: {}", err);
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

    /// 停止采集
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

    /// 检查是否正在采集
    pub fn is_running(&self) -> bool {
        self.is_running.lock().map(|r| *r).unwrap_or(false)
    }

    /// 读取音频数据
    pub fn read(&self) -> Result<AudioBuffer<f32>> {
        let frame_size = self.config.buffer_size as usize;
        let channels = self.config.channels as usize;
        let total_samples = frame_size * channels;

        let mut samples = vec![0.0f32; total_samples];
        let read = self.ring_buffer.read(&mut samples);

        if read < total_samples {
            samples[read..].fill(0.0);
        }

        let format = AudioFormat {
            sample_rate: self.config.sample_rate,
            channels: self.config.channels,
            sample_format: SampleFormat::F32,
        };

        AudioBuffer::new(samples, format)
            .map_err(|e| CaptureError::StreamError(e.to_string()))
    }

    /// 获取配置
    pub fn config(&self) -> &CaptureConfig {
        &self.config
    }

    /// 获取设备名称
    pub fn device_name(&self) -> String {
        self.device.name().unwrap_or_else(|_| "未知设备".to_string())
    }

    /// 获取采集 ring buffer 的共享引用
    ///
    /// 用于管线线程直接从 ring buffer 读取采集数据。
    pub fn ring_buffer(&self) -> Arc<RingBuffer<f32>> {
        self.ring_buffer.clone()
    }

    /// 获取帧大小（每帧采样数，不含通道展开）
    pub fn frame_size(&self) -> usize {
        self.config.buffer_size as usize
    }

    /// 获取通道数
    pub fn channels(&self) -> u16 {
        self.config.channels
    }
}

impl Drop for CaptureDevice {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}
