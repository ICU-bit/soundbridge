//! SoundBridge FFI 绑定模块
//!
//! 提供 C API，供 Windows C++ 和 Android JNI 调用。

use std::ffi::{CStr, CString};
use std::net::{SocketAddr, UdpSocket};
use std::os::raw::{c_char, c_int, c_void};
use std::ptr;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use audio_capture::{CaptureConfig, CaptureDevice};
use audio_codec::{OpusConfig, OpusDecoderCodec, OpusEncoderCodec};
use audio_mixer::AudioMixer;
use audio_playback::{PlaybackConfig, PlaybackDevice};
use audio_processor::AudioProcessor;
use protocol::{PacketHeader, Protocol};
use std::time::Instant;

/// FFI 错误码
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SbError {
    /// 成功
    Ok = 0,

    /// 通用错误
    Error = -1,

    /// 无效参数
    InvalidArgument = -2,

    /// 设备未找到
    DeviceNotFound = -3,

    /// 配置错误
    ConfigError = -4,

    /// 流错误
    StreamError = -5,

    /// 编解码错误
    CodecError = -6,

    /// 网络错误
    NetworkError = -7,

    /// 管线未就绪
    PipelineNotReady = -8,
}

/// 全局错误信息
static LAST_ERROR: Mutex<Option<CString>> = Mutex::new(None);

/// 设置错误信息
fn set_error(msg: &str) {
    if let Ok(mut error) = LAST_ERROR.lock() {
        *error = Some(CString::new(msg).unwrap_or_else(|_| CString::new("unknown error").unwrap()));
    }
}

/// 获取最后的错误信息
#[no_mangle]
pub extern "C" fn sb_last_error() -> *const c_char {
    if let Ok(error) = LAST_ERROR.lock() {
        if let Some(ref msg) = *error {
            return msg.as_ptr();
        }
    }
    ptr::null()
}

/// 清除错误信息
fn clear_error() {
    if let Ok(mut error) = LAST_ERROR.lock() {
        *error = None;
    }
}

/// 管线状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PipelineState {
    Stopped,
    Running,
    #[allow(dead_code)]
    Error,
}

/// 共享的管线统计（原子操作，线程安全）
#[derive(Debug)]
struct SharedPipelineStats {
    frames_encoded: AtomicU64,
    frames_decoded: AtomicU64,
    frames_dropped: AtomicU64,
    packets_sent: AtomicU64,
}

impl SharedPipelineStats {
    fn new() -> Self {
        Self {
            frames_encoded: AtomicU64::new(0),
            frames_decoded: AtomicU64::new(0),
            frames_dropped: AtomicU64::new(0),
            packets_sent: AtomicU64::new(0),
        }
    }
}

/// 管线句柄（运行时资源）
struct PipelineHandle {
    /// 停止信号
    running: Arc<AtomicBool>,

    /// 发送线程句柄
    sender_handle: Option<JoinHandle<()>>,

    /// 接收线程句柄
    receiver_handle: Option<JoinHandle<()>>,

    /// 共享统计
    stats: Arc<SharedPipelineStats>,
}

/// SoundBridge 引擎
pub struct SbEngine {
    /// 采集设备
    capture: Option<CaptureDevice>,

    /// 播放设备
    playback: Option<PlaybackDevice>,

    /// 混音器
    mixer: AudioMixer,

    /// 音频处理器
    processor: AudioProcessor,

    /// 管线状态
    pipeline_state: PipelineState,

    /// 目标地址（远端）
    target_addr: Option<SocketAddr>,

    /// UDP socket（用于发送和接收）
    udp_socket: Option<Arc<UdpSocket>>,

    /// 本地监听端口
    local_port: u16,

    /// 序列号（原子递增）
    sequence: Arc<AtomicU32>,

    /// 管线句柄
    pipeline: Option<PipelineHandle>,
}

/// 创建引擎
#[no_mangle]
pub extern "C" fn sb_engine_create() -> *mut c_void {
    clear_error();

    let engine = SbEngine {
        capture: None,
        playback: None,
        mixer: AudioMixer::default(),
        processor: AudioProcessor::with_default_config().expect("Failed to create AudioProcessor"),
        pipeline_state: PipelineState::Stopped,
        target_addr: None,
        udp_socket: None,
        local_port: 0,
        sequence: Arc::new(AtomicU32::new(0)),
        pipeline: None,
    };

    Box::into_raw(Box::new(engine)) as *mut c_void
}

/// 销毁引擎
///
/// # Safety
/// `engine` 必须是通过 `sb_engine_create` 创建的有效指针。
#[no_mangle]
pub unsafe extern "C" fn sb_engine_destroy(engine: *mut c_void) {
    clear_error();

    if engine.is_null() {
        set_error("engine is null");
        return;
    }

    unsafe {
        let engine = &mut *(engine as *mut SbEngine);
        // 先停止管线
        if engine.pipeline_state == PipelineState::Running {
            stop_pipeline_internal(engine);
        }
        let _ = Box::from_raw(engine as *mut SbEngine);
    }
}

/// 绑定本地 UDP 端口
///
/// # Safety
/// `engine` 必须是通过 `sb_engine_create` 创建的有效指针。
/// `port` 为 0 时自动分配端口。
#[no_mangle]
pub unsafe extern "C" fn sb_bind(engine: *mut c_void, port: u16) -> c_int {
    clear_error();

    if engine.is_null() {
        set_error("engine is null");
        return SbError::InvalidArgument as c_int;
    }

    let engine = unsafe { &mut *(engine as *mut SbEngine) };

    let addr = format!("0.0.0.0:{}", port);
    match UdpSocket::bind(&addr) {
        Ok(socket) => {
            match socket.local_addr() {
                Ok(local_addr) => {
                    engine.local_port = local_addr.port();
                    engine.udp_socket = Some(Arc::new(socket));
                    SbError::Ok as c_int
                }
                Err(e) => {
                    set_error(&format!("failed to get local addr: {}", e));
                    SbError::NetworkError as c_int
                }
            }
        }
        Err(e) => {
            set_error(&format!("failed to bind UDP socket: {}", e));
            SbError::NetworkError as c_int
        }
    }
}

/// 设置目标地址
///
/// # Safety
/// `engine` 必须是通过 `sb_engine_create` 创建的有效指针。
/// `addr` 必须是有效的 C 字符串，格式为 "ip:port"。
#[no_mangle]
pub unsafe extern "C" fn sb_connect(engine: *mut c_void, addr: *const c_char) -> c_int {
    clear_error();

    if engine.is_null() || addr.is_null() {
        set_error("engine or addr is null");
        return SbError::InvalidArgument as c_int;
    }

    let engine = unsafe { &mut *(engine as *mut SbEngine) };

    let addr_str = unsafe { CStr::from_ptr(addr) }.to_string_lossy().to_string();
    match addr_str.parse::<SocketAddr>() {
        Ok(target) => {
            engine.target_addr = Some(target);
            SbError::Ok as c_int
        }
        Err(e) => {
            set_error(&format!("invalid address '{}': {}", addr_str, e));
            SbError::InvalidArgument as c_int
        }
    }
}

/// 获取本地端口
///
/// # Safety
/// `engine` 必须是通过 `sb_engine_create` 创建的有效指针。
/// `port` 必须是有效的指针。
#[no_mangle]
pub unsafe extern "C" fn sb_local_port(engine: *mut c_void, port: *mut u16) -> c_int {
    clear_error();

    if engine.is_null() || port.is_null() {
        set_error("engine or port is null");
        return SbError::InvalidArgument as c_int;
    }

    let engine = unsafe { &*(engine as *const SbEngine) };
    unsafe {
        *port = engine.local_port;
    }

    SbError::Ok as c_int
}

/// 开始音频采集
///
/// # Safety
/// `engine` 必须是通过 `sb_engine_create` 创建的有效指针。
/// `device_name` 必须是有效的 C 字符串，或者为 null（使用默认设备）。
#[no_mangle]
pub unsafe extern "C" fn sb_capture_start(engine: *mut c_void, device_name: *const c_char) -> c_int {
    clear_error();

    if engine.is_null() {
        set_error("engine is null");
        return SbError::InvalidArgument as c_int;
    }

    let engine = unsafe { &mut *(engine as *mut SbEngine) };

    let config = CaptureConfig::default();

    let device = if device_name.is_null() {
        match CaptureDevice::new_default(config) {
            Ok(d) => d,
            Err(e) => {
                set_error(&format!("failed to create capture device: {}", e));
                return SbError::DeviceNotFound as c_int;
            }
        }
    } else {
        let name = unsafe { CStr::from_ptr(device_name) }.to_string_lossy().to_string();
        let devices = match CaptureDevice::list_devices() {
            Ok(d) => d,
            Err(e) => {
                set_error(&format!("failed to list devices: {}", e));
                return SbError::DeviceNotFound as c_int;
            }
        };
        let device_info = devices.into_iter().find(|d| d.name == name);
        match device_info {
            Some(info) => match CaptureDevice::new(&info, config) {
                Ok(d) => d,
                Err(e) => {
                    set_error(&format!("failed to create capture device: {}", e));
                    return SbError::DeviceNotFound as c_int;
                }
            },
            None => {
                set_error(&format!("device not found: {}", name));
                return SbError::DeviceNotFound as c_int;
            }
        }
    };

    engine.capture = Some(device);

    if let Some(ref mut capture) = engine.capture {
        if let Err(e) = capture.start() {
            set_error(&format!("failed to start capture: {}", e));
            return SbError::StreamError as c_int;
        }
    }

    SbError::Ok as c_int
}

/// 停止音频采集
///
/// # Safety
/// `engine` 必须是通过 `sb_engine_create` 创建的有效指针。
#[no_mangle]
pub unsafe extern "C" fn sb_capture_stop(engine: *mut c_void) -> c_int {
    clear_error();

    if engine.is_null() {
        set_error("engine is null");
        return SbError::InvalidArgument as c_int;
    }

    let engine = unsafe { &mut *(engine as *mut SbEngine) };

    if let Some(ref mut capture) = engine.capture {
        if let Err(e) = capture.stop() {
            set_error(&format!("failed to stop capture: {}", e));
            return SbError::StreamError as c_int;
        }
    }

    engine.capture = None;
    SbError::Ok as c_int
}

/// 读取音频数据
///
/// # Safety
/// `engine` 必须是通过 `sb_engine_create` 创建的有效指针。
/// `buf` 必须是有效的缓冲区，至少 `len` 个 f32 元素。
#[no_mangle]
pub unsafe extern "C" fn sb_capture_read(engine: *mut c_void, buf: *mut f32, len: usize) -> c_int {
    clear_error();

    if engine.is_null() || buf.is_null() {
        set_error("engine or buf is null");
        return SbError::InvalidArgument as c_int;
    }

    let engine = unsafe { &*(engine as *const SbEngine) };

    if let Some(ref capture) = engine.capture {
        match capture.read() {
            Ok(buffer) => {
                let samples = buffer.samples();
                let copy_len = samples.len().min(len);
                unsafe {
                    ptr::copy_nonoverlapping(samples.as_ptr(), buf, copy_len);
                }
                copy_len as c_int
            }
            Err(e) => {
                set_error(&format!("failed to read: {}", e));
                SbError::StreamError as c_int
            }
        }
    } else {
        set_error("capture not started");
        SbError::Error as c_int
    }
}

/// 开始音频播放
///
/// # Safety
/// `engine` 必须是通过 `sb_engine_create` 创建的有效指针。
/// `device_name` 必须是有效的 C 字符串，或者为 null（使用默认设备）。
#[no_mangle]
pub unsafe extern "C" fn sb_playback_start(engine: *mut c_void, device_name: *const c_char) -> c_int {
    clear_error();

    if engine.is_null() {
        set_error("engine is null");
        return SbError::InvalidArgument as c_int;
    }

    let engine = unsafe { &mut *(engine as *mut SbEngine) };

    let config = PlaybackConfig::default();

    let device = if device_name.is_null() {
        match PlaybackDevice::new_default(config) {
            Ok(d) => d,
            Err(e) => {
                set_error(&format!("failed to create playback device: {}", e));
                return SbError::DeviceNotFound as c_int;
            }
        }
    } else {
        let name = unsafe { CStr::from_ptr(device_name) }.to_string_lossy().to_string();
        let devices = match PlaybackDevice::list_devices() {
            Ok(d) => d,
            Err(e) => {
                set_error(&format!("failed to list devices: {}", e));
                return SbError::DeviceNotFound as c_int;
            }
        };
        let device_info = devices.into_iter().find(|d| d.name == name);
        match device_info {
            Some(info) => match PlaybackDevice::new(&info, config) {
                Ok(d) => d,
                Err(e) => {
                    set_error(&format!("failed to create playback device: {}", e));
                    return SbError::DeviceNotFound as c_int;
                }
            },
            None => {
                set_error(&format!("device not found: {}", name));
                return SbError::DeviceNotFound as c_int;
            }
        }
    };

    engine.playback = Some(device);

    if let Some(ref mut playback) = engine.playback {
        if let Err(e) = playback.start() {
            set_error(&format!("failed to start playback: {}", e));
            return SbError::StreamError as c_int;
        }
    }

    SbError::Ok as c_int
}

/// 停止音频播放
///
/// # Safety
/// `engine` 必须是通过 `sb_engine_create` 创建的有效指针。
#[no_mangle]
pub unsafe extern "C" fn sb_playback_stop(engine: *mut c_void) -> c_int {
    clear_error();

    if engine.is_null() {
        set_error("engine is null");
        return SbError::InvalidArgument as c_int;
    }

    let engine = unsafe { &mut *(engine as *mut SbEngine) };

    if let Some(ref mut playback) = engine.playback {
        if let Err(e) = playback.stop() {
            set_error(&format!("failed to stop playback: {}", e));
            return SbError::StreamError as c_int;
        }
    }

    engine.playback = None;
    SbError::Ok as c_int
}

/// 写入音频数据
///
/// # Safety
/// `engine` 必须是通过 `sb_engine_create` 创建的有效指针。
/// `buf` 必须是有效的缓冲区，至少 `len` 个 f32 元素。
#[no_mangle]
pub unsafe extern "C" fn sb_playback_write(engine: *mut c_void, buf: *const f32, len: usize) -> c_int {
    clear_error();

    if engine.is_null() || buf.is_null() {
        set_error("engine or buf is null");
        return SbError::InvalidArgument as c_int;
    }

    let engine = unsafe { &*(engine as *const SbEngine) };

    if let Some(ref playback) = engine.playback {
        let samples = unsafe { std::slice::from_raw_parts(buf, len) };
        let format = audio_core::AudioFormat {
            sample_rate: 48000,
            channels: 2,
            sample_format: audio_core::SampleFormat::F32,
        };
        match audio_core::AudioBuffer::new(samples.to_vec(), format) {
            Ok(buffer) => {
                if let Err(e) = playback.write(&buffer) {
                    set_error(&format!("failed to write: {}", e));
                    return SbError::StreamError as c_int;
                }
            }
            Err(e) => {
                set_error(&format!("failed to create buffer: {}", e));
                return SbError::Error as c_int;
            }
        }
    } else {
        set_error("playback not started");
        return SbError::Error as c_int;
    }

    SbError::Ok as c_int
}

/// 混音多路音频
///
/// # Safety
/// `inputs` 必须是有效的指针数组，每个指针指向有效的 f32 缓冲区。
/// `input_lens` 必须是有效的长度数组。
/// `volumes` 必须是有效的音量数组。
/// `output` 必须是有效的输出缓冲区。
#[no_mangle]
pub unsafe extern "C" fn sb_mixer_mix(
    engine: *mut c_void,
    inputs: *const *const f32,
    input_lens: *const usize,
    volumes: *const f32,
    input_count: usize,
    output: *mut f32,
    output_len: usize,
) -> c_int {
    clear_error();

    if engine.is_null() || inputs.is_null() || input_lens.is_null() || volumes.is_null() || output.is_null() {
        set_error("invalid arguments");
        return SbError::InvalidArgument as c_int;
    }

    let engine = unsafe { &*(engine as *const SbEngine) };

    let mut input_slices = Vec::with_capacity(input_count);
    let mut volume_slice = Vec::with_capacity(input_count);

    for i in 0..input_count {
        let input_ptr = unsafe { *inputs.add(i) };
        let input_len = unsafe { *input_lens.add(i) };
        let volume = unsafe { *volumes.add(i) };

        if input_ptr.is_null() {
            set_error("input is null");
            return SbError::InvalidArgument as c_int;
        }

        let slice = unsafe { std::slice::from_raw_parts(input_ptr, input_len) };
        input_slices.push(slice);
        volume_slice.push(volume);
    }

    match engine.mixer.mix(&input_slices, &volume_slice) {
        Ok(mixed) => {
            let copy_len = mixed.len().min(output_len);
            unsafe {
                ptr::copy_nonoverlapping(mixed.as_ptr(), output, copy_len);
            }
            copy_len as c_int
        }
        Err(e) => {
            set_error(&format!("mix failed: {}", e));
            SbError::Error as c_int
        }
    }
}

/// 处理音频数据（就地修改）
///
/// # Safety
/// `engine` 必须是通过 `sb_engine_create` 创建的有效指针。
/// `buf` 必须是有效的缓冲区，至少 `len` 个 f32 元素。
#[no_mangle]
pub unsafe extern "C" fn sb_processor_process(engine: *mut c_void, buf: *mut f32, len: usize) -> c_int {
    clear_error();

    if engine.is_null() || buf.is_null() {
        set_error("engine or buf is null");
        return SbError::InvalidArgument as c_int;
    }

    let engine = unsafe { &mut *(engine as *mut SbEngine) };
    let buffer = unsafe { std::slice::from_raw_parts_mut(buf, len) };

    if let Err(e) = engine.processor.process(buffer) {
        set_error(&format!("process failed: {}", e));
        return SbError::Error as c_int;
    }

    SbError::Ok as c_int
}

/// 获取列表设备数量
///
/// # Safety
/// `count` 必须是有效的指针。
#[no_mangle]
pub unsafe extern "C" fn sb_capture_device_count(count: *mut usize) -> c_int {
    clear_error();

    if count.is_null() {
        set_error("count is null");
        return SbError::InvalidArgument as c_int;
    }

    match CaptureDevice::list_devices() {
        Ok(devices) => {
            unsafe {
                *count = devices.len();
            }
            SbError::Ok as c_int
        }
        Err(e) => {
            set_error(&format!("failed to list devices: {}", e));
            SbError::DeviceNotFound as c_int
        }
    }
}

/// 获取播放设备数量
///
/// # Safety
/// `count` 必须是有效的指针。
#[no_mangle]
pub unsafe extern "C" fn sb_playback_device_count(count: *mut usize) -> c_int {
    clear_error();

    if count.is_null() {
        set_error("count is null");
        return SbError::InvalidArgument as c_int;
    }

    match PlaybackDevice::list_devices() {
        Ok(devices) => {
            unsafe {
                *count = devices.len();
            }
            SbError::Ok as c_int
        }
        Err(e) => {
            set_error(&format!("failed to list devices: {}", e));
            SbError::DeviceNotFound as c_int
        }
    }
}

/// 启动音频管线
///
/// 管线会启动两个线程：
/// - 发送线程：采集 → 编码 → UDP 发送
/// - 接收线程：UDP 接收 → 解码 → 播放
///
/// 前置条件：
/// - 必须先调用 sb_capture_start 启动采集
/// - 必须先调用 sb_playback_start 启动播放
/// - 必须先调用 sb_bind 绑定 UDP 端口
/// - 必须先调用 sb_connect 设置目标地址
///
/// # Safety
/// `engine` 必须是通过 `sb_engine_create` 创建的有效指针。
#[no_mangle]
pub unsafe extern "C" fn sb_pipeline_start(engine: *mut c_void) -> c_int {
    clear_error();

    if engine.is_null() {
        set_error("engine is null");
        return SbError::InvalidArgument as c_int;
    }

    let engine = unsafe { &mut *(engine as *mut SbEngine) };

    if engine.pipeline_state == PipelineState::Running {
        return SbError::Ok as c_int;
    }

    // 前置检查
    if engine.capture.is_none() {
        set_error("capture not started - call sb_capture_start first");
        return SbError::PipelineNotReady as c_int;
    }

    if engine.playback.is_none() {
        set_error("playback not started - call sb_playback_start first");
        return SbError::PipelineNotReady as c_int;
    }

    let socket = match engine.udp_socket.as_ref() {
        Some(s) => s.clone(),
        None => {
            set_error("UDP socket not bound - call sb_bind first");
            return SbError::PipelineNotReady as c_int;
        }
    };

    let target = match engine.target_addr {
        Some(t) => t,
        None => {
            set_error("target address not set - call sb_connect first");
            return SbError::PipelineNotReady as c_int;
        }
    };

    // 创建编解码器
    let opus_config = OpusConfig::default(); // 48kHz, mono, 128kbps, 20ms
    let encoder = match OpusEncoderCodec::new(opus_config.clone()) {
        Ok(e) => e,
        Err(e) => {
            set_error(&format!("failed to create encoder: {}", e));
            return SbError::CodecError as c_int;
        }
    };

    let decoder = match OpusDecoderCodec::new(opus_config.clone()) {
        Ok(d) => d,
        Err(e) => {
            set_error(&format!("failed to create decoder: {}", e));
            return SbError::CodecError as c_int;
        }
    };

    // 创建共享状态
    let running = Arc::new(AtomicBool::new(true));
    let stats = Arc::new(SharedPipelineStats::new());
    let sequence = engine.sequence.clone();

    // 获取采集和播放设备的 ring buffer（线程安全的 Arc 引用）
    let capture_ring = engine.capture.as_ref().unwrap().ring_buffer();
    let playback_ring = engine.playback.as_ref().unwrap().ring_buffer();

    // 克隆 socket 和 target 给发送线程
    let send_socket = socket.clone();
    let recv_socket = socket;
    let frame_size = opus_config.frame_size_samples(); // 960 samples per frame (20ms @ 48kHz)

    // 启动发送线程：采集 ring buffer → 编码 → UDP 发送
    let sender_running = running.clone();
    let sender_stats = stats.clone();
    let sender_sequence = sequence.clone();
    let sender_capture_ring = capture_ring.clone();
    let sender_handle = match std::thread::Builder::new()
        .name("sb-sender".to_string())
        .spawn(move || {
            let mut encoder = encoder;
            let mut frame_buf = vec![0.0f32; frame_size];
            let protocol = Protocol::new();
            let start_time = Instant::now();

            tracing::info!("Sender thread started, frame_size={}", frame_size);

            while sender_running.load(Ordering::Relaxed) {
                // 从采集 ring buffer 读取一帧数据
                // cpal 回调持续向 ring buffer 写入数据，这里轮询读取
                let read = sender_capture_ring.read(&mut frame_buf);
                if read < frame_size {
                    // 数据不足一帧，等待更多数据
                    std::thread::sleep(std::time::Duration::from_millis(1));
                    continue;
                }

                // 编码一帧
                match encoder.encode_interleaved(&frame_buf[..frame_size]) {
                    Ok(opus_data) => {
                        let seq = sender_sequence.fetch_add(1, Ordering::Relaxed);
                        let timestamp_ms = start_time.elapsed().as_millis() as u32;

                        // 构造协议包
                        let header = PacketHeader {
                            sequence: seq,
                            timestamp_ms,
                            flags: 0x01, // 音频数据标志
                            channels: 1, // mono
                            opus_length: opus_data.len() as u16,
                        };

                        let packet = protocol.serialize(&protocol::Packet::Audio {
                            header,
                            data: opus_data,
                        });

                        match packet {
                            Ok(packet_data) => {
                                match send_socket.send_to(&packet_data, target) {
                                    Ok(_) => {
                                        sender_stats.packets_sent.fetch_add(1, Ordering::Relaxed);
                                        sender_stats.frames_encoded.fetch_add(1, Ordering::Relaxed);
                                    }
                                    Err(e) => {
                                        tracing::warn!("Failed to send packet: {}", e);
                                        sender_stats.frames_dropped.fetch_add(1, Ordering::Relaxed);
                                    }
                                }
                            }
                            Err(e) => {
                                tracing::warn!("Failed to serialize packet: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Encode error: {}", e);
                    }
                }
            }

            tracing::info!("Sender thread stopped");
        }) {
        Ok(h) => Some(h),
        Err(e) => {
            set_error(&format!("failed to spawn sender thread: {}", e));
            return SbError::Error as c_int;
        }
    };

    // 启动接收线程：UDP 接收 → 解码 → 播放 ring buffer
    let receiver_running = running.clone();
    let receiver_stats = stats.clone();
    let receiver_playback_ring = playback_ring.clone();
    let receiver_handle = match std::thread::Builder::new()
        .name("sb-receiver".to_string())
        .spawn(move || {
            let mut decoder = decoder;
            let protocol = Protocol::new();
            let mut recv_buf = vec![0u8; 4096];

            tracing::info!("Receiver thread started");

            recv_socket
                .set_nonblocking(true)
                .expect("Failed to set non-blocking");

            while receiver_running.load(Ordering::Relaxed) {
                match recv_socket.recv_from(&mut recv_buf) {
                    Ok((len, _from)) => {
                        match protocol.deserialize(&recv_buf[..len]) {
                            Ok(packet) => {
                                match packet {
                                    protocol::Packet::Audio { header: _, data } => {
                                        match decoder.decode(&data) {
                                            Ok(audio_buffer) => {
                                                // 将解码数据写入播放 ring buffer
                                                let samples = audio_buffer.samples();
                                                receiver_playback_ring.write(samples);
                                                receiver_stats
                                                    .frames_decoded
                                                    .fetch_add(1, Ordering::Relaxed);
                                            }
                                            Err(e) => {
                                                tracing::warn!("Decode error: {}", e);
                                                receiver_stats
                                                    .frames_dropped
                                                    .fetch_add(1, Ordering::Relaxed);
                                            }
                                        }
                                    }
                                    _ => {}
                                }
                            }
                            Err(e) => {
                                tracing::warn!("Packet parse error: {}", e);
                            }
                        }
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(std::time::Duration::from_millis(1));
                    }
                    Err(e) => {
                        tracing::warn!("Recv error: {}", e);
                        std::thread::sleep(std::time::Duration::from_millis(10));
                    }
                }
            }

            tracing::info!("Receiver thread stopped");
        }) {
        Ok(h) => Some(h),
        Err(e) => {
            set_error(&format!("failed to spawn receiver thread: {}", e));
            running.store(false, Ordering::Relaxed);
            if let Some(handle) = sender_handle {
                let _ = handle.join();
            }
            return SbError::Error as c_int;
        }
    };

    engine.pipeline = Some(PipelineHandle {
        running,
        sender_handle,
        receiver_handle,
        stats,
    });

    engine.pipeline_state = PipelineState::Running;
    SbError::Ok as c_int
}

/// 内部停止管线
fn stop_pipeline_internal(engine: &mut SbEngine) {
    if let Some(pipeline) = engine.pipeline.take() {
        // 发送停止信号
        pipeline.running.store(false, Ordering::Relaxed);

        // 等待线程结束
        if let Some(handle) = pipeline.sender_handle {
            let _ = handle.join();
        }
        if let Some(handle) = pipeline.receiver_handle {
            let _ = handle.join();
        }
    }
    engine.pipeline_state = PipelineState::Stopped;
}

/// 停止音频管线
///
/// # Safety
/// `engine` 必须是通过 `sb_engine_create` 创建的有效指针。
#[no_mangle]
pub unsafe extern "C" fn sb_pipeline_stop(engine: *mut c_void) -> c_int {
    clear_error();

    if engine.is_null() {
        set_error("engine is null");
        return SbError::InvalidArgument as c_int;
    }

    let engine = unsafe { &mut *(engine as *mut SbEngine) };

    // 停止管线线程
    stop_pipeline_internal(engine);

    // 停止采集
    if let Some(ref mut capture) = engine.capture {
        let _ = capture.stop();
    }

    // 停止播放
    if let Some(ref mut playback) = engine.playback {
        let _ = playback.stop();
    }

    SbError::Ok as c_int
}

/// 获取管线状态
///
/// # Safety
/// `engine` 必须是通过 `sb_engine_create` 创建的有效指针。
/// `state` 必须是有效的指针。
#[no_mangle]
pub unsafe extern "C" fn sb_pipeline_state(engine: *mut c_void, state: *mut c_int) -> c_int {
    clear_error();

    if engine.is_null() || state.is_null() {
        set_error("engine or state is null");
        return SbError::InvalidArgument as c_int;
    }

    let engine = unsafe { &*(engine as *const SbEngine) };
    let state_value = match engine.pipeline_state {
        PipelineState::Stopped => 0,
        PipelineState::Running => 1,
        PipelineState::Error => 2,
    };

    unsafe {
        *state = state_value;
    }

    SbError::Ok as c_int
}

/// 获取管线统计
///
/// # Safety
/// `engine` 必须是通过 `sb_engine_create` 创建的有效指针。
/// `frames_captured` / `frames_played` / `latency_ms` 必须是有效的指针。
#[no_mangle]
pub unsafe extern "C" fn sb_pipeline_stats(
    engine: *mut c_void,
    frames_captured: *mut u64,
    frames_played: *mut u64,
    latency_ms: *mut f32,
) -> c_int {
    clear_error();

    if engine.is_null() || frames_captured.is_null() || frames_played.is_null() || latency_ms.is_null() {
        set_error("invalid arguments");
        return SbError::InvalidArgument as c_int;
    }

    let engine = unsafe { &*(engine as *const SbEngine) };

    if let Some(ref pipeline) = engine.pipeline {
        let stats = &pipeline.stats;
        unsafe {
            *frames_captured = stats.frames_encoded.load(Ordering::Relaxed);
            *frames_played = stats.frames_decoded.load(Ordering::Relaxed);
            // 估算延迟（帧数 * 20ms 每帧）
            *latency_ms = 20.0; // 固定 20ms 帧延迟
        }
    } else {
        unsafe {
            *frames_captured = 0;
            *frames_played = 0;
            *latency_ms = 0.0;
        }
    }

    SbError::Ok as c_int
}

// ============================================================
// 设备存储（DeviceStore）FFI
// ============================================================

use discovery::DeviceStore;

/// 打开设备存储（JSON 文件持久化）
///
/// `path` 必须是有效的 UTF-8 文件路径。
/// 文件不存在时自动创建。
///
/// # Safety
/// 返回的句柄必须通过 `sb_device_store_close` 释放。
#[no_mangle]
pub unsafe extern "C" fn sb_device_store_open(path: *const c_char) -> *mut c_void {
    clear_error();

    if path.is_null() {
        set_error("path is null");
        return ptr::null_mut();
    }

    let path_str = unsafe { CStr::from_ptr(path) }.to_string_lossy().to_string();
    let path = std::path::Path::new(&path_str);

    let store = DeviceStore::with_file(path);
    Box::into_raw(Box::new(store)) as *mut c_void
}

/// 关闭设备存储
///
/// # Safety
/// `store` 必须是通过 `sb_device_store_open` 创建的有效指针。
#[no_mangle]
pub unsafe extern "C" fn sb_device_store_close(store: *mut c_void) {
    clear_error();

    if store.is_null() {
        return;
    }

    unsafe {
        let _ = Box::from_raw(store as *mut DeviceStore);
    }
}

/// 添加或更新设备
///
/// # Safety
/// `store` 和 `name` 必须是有效的指针。
#[no_mangle]
pub unsafe extern "C" fn sb_device_store_add(
    store: *mut c_void,
    name: *const c_char,
    address: *const c_char,
    port: u16,
) -> c_int {
    clear_error();

    if store.is_null() || name.is_null() || address.is_null() {
        set_error("null argument");
        return SbError::InvalidArgument as c_int;
    }

    let store = unsafe { &mut *(store as *mut DeviceStore) };
    let name = unsafe { CStr::from_ptr(name) }.to_string_lossy().to_string();
    let addr_str = unsafe { CStr::from_ptr(address) }.to_string_lossy().to_string();

    match addr_str.parse::<std::net::IpAddr>() {
        Ok(addr) => {
            store.add_device(&name, addr, port);
            SbError::Ok as c_int
        }
        Err(e) => {
            set_error(&format!("invalid address '{}': {}", addr_str, e));
            SbError::InvalidArgument as c_int
        }
    }
}

/// 删除设备
///
/// # Safety
/// `store` 和 `name` 必须是有效的指针。
#[no_mangle]
pub unsafe extern "C" fn sb_device_store_remove(
    store: *mut c_void,
    name: *const c_char,
) -> c_int {
    clear_error();

    if store.is_null() || name.is_null() {
        set_error("null argument");
        return SbError::InvalidArgument as c_int;
    }

    let store = unsafe { &mut *(store as *mut DeviceStore) };
    let name = unsafe { CStr::from_ptr(name) }.to_string_lossy().to_string();

    if store.remove_device(&name) {
        SbError::Ok as c_int
    } else {
        set_error("device not found");
        SbError::DeviceNotFound as c_int
    }
}

/// 设置设备自动连接
///
/// # Safety
/// `store` 和 `name` 必须是有效的指针。
#[no_mangle]
pub unsafe extern "C" fn sb_device_store_set_auto_connect(
    store: *mut c_void,
    name: *const c_char,
    auto_connect: bool,
) -> c_int {
    clear_error();

    if store.is_null() || name.is_null() {
        set_error("null argument");
        return SbError::InvalidArgument as c_int;
    }

    let store = unsafe { &mut *(store as *mut DeviceStore) };
    let name = unsafe { CStr::from_ptr(name) }.to_string_lossy().to_string();

    store.set_auto_connect(&name, auto_connect);
    SbError::Ok as c_int
}

/// 获取设备数量
///
/// # Safety
/// `store` 必须是有效的指针。
#[no_mangle]
pub unsafe extern "C" fn sb_device_store_count(
    store: *mut c_void,
    count: *mut usize,
) -> c_int {
    clear_error();

    if store.is_null() || count.is_null() {
        set_error("null argument");
        return SbError::InvalidArgument as c_int;
    }

    let store = unsafe { &*(store as *const DeviceStore) };
    unsafe {
        *count = store.len();
    }

    SbError::Ok as c_int
}

/// 检查设备是否存在
///
/// # Safety
/// `store` 和 `name` 必须是有效的指针。
#[no_mangle]
pub unsafe extern "C" fn sb_device_store_has(
    store: *mut c_void,
    name: *const c_char,
) -> c_int {
    clear_error();

    if store.is_null() || name.is_null() {
        set_error("null argument");
        return SbError::InvalidArgument as c_int;
    }

    let store = unsafe { &*(store as *const DeviceStore) };
    let name = unsafe { CStr::from_ptr(name) }.to_string_lossy().to_string();

    if store.has_device(&name) { 1 } else { 0 }
}

/// 清除所有设备
///
/// # Safety
/// `store` 必须是有效的指针。
#[no_mangle]
pub unsafe extern "C" fn sb_device_store_clear(store: *mut c_void) {
    clear_error();

    if store.is_null() {
        return;
    }

    let store = unsafe { &mut *(store as *mut DeviceStore) };
    store.clear();
}

/// 获取设备地址（写入 buf，返回写入字节数，不含 null）
///
/// # Safety
/// `store`、`name`、`buf` 必须是有效的指针。
#[no_mangle]
pub unsafe extern "C" fn sb_device_store_get_address(
    store: *mut c_void,
    name: *const c_char,
    buf: *mut c_char,
    buf_len: usize,
) -> c_int {
    clear_error();

    if store.is_null() || name.is_null() || buf.is_null() {
        set_error("null argument");
        return -1;
    }

    let store = unsafe { &*(store as *const DeviceStore) };
    let name = unsafe { CStr::from_ptr(name) }.to_string_lossy().to_string();

    match store.get_device(&name) {
        Some(device) => {
            let addr = &device.address;
            let bytes = addr.as_bytes();
            let copy_len = bytes.len().min(buf_len - 1);
            unsafe {
                ptr::copy_nonoverlapping(bytes.as_ptr(), buf as *mut u8, copy_len);
                *buf.add(copy_len) = 0; // null terminator
            }
            copy_len as c_int
        }
        None => {
            set_error("device not found");
            -1
        }
    }
}

/// 获取设备端口
///
/// # Safety
/// `store` 和 `name` 必须是有效的指针。
#[no_mangle]
pub unsafe extern "C" fn sb_device_store_get_port(
    store: *mut c_void,
    name: *const c_char,
    port: *mut u16,
) -> c_int {
    clear_error();

    if store.is_null() || name.is_null() || port.is_null() {
        set_error("null argument");
        return SbError::InvalidArgument as c_int;
    }

    let store = unsafe { &*(store as *const DeviceStore) };
    let name = unsafe { CStr::from_ptr(name) }.to_string_lossy().to_string();

    match store.get_device(&name) {
        Some(device) => {
            unsafe {
                *port = device.port;
            }
            SbError::Ok as c_int
        }
        None => {
            set_error("device not found");
            SbError::DeviceNotFound as c_int
        }
    }
}

/// 获取第 N 个设备的名称（按添加顺序）
///
/// # Safety
/// `store` 和 `buf` 必须是有效的指针。
#[no_mangle]
pub unsafe extern "C" fn sb_device_store_get_name_at(
    store: *mut c_void,
    index: usize,
    buf: *mut c_char,
    buf_len: usize,
) -> c_int {
    clear_error();

    if store.is_null() || buf.is_null() {
        set_error("null argument");
        return -1;
    }

    let store = unsafe { &*(store as *const DeviceStore) };
    let devices: Vec<_> = store.get_all_devices();

    if index >= devices.len() {
        set_error("index out of range");
        return -1;
    }

    let name = &devices[index].name;
    let bytes = name.as_bytes();
    let copy_len = bytes.len().min(buf_len - 1);
    unsafe {
        ptr::copy_nonoverlapping(bytes.as_ptr(), buf as *mut u8, copy_len);
        *buf.add(copy_len) = 0;
    }
    copy_len as c_int
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_create_destroy() {
        unsafe {
            let engine = sb_engine_create();
            assert!(!engine.is_null());
            sb_engine_destroy(engine);
        }
    }

    #[test]
    fn test_last_error() {
        clear_error();
        let error = sb_last_error();
        assert!(error.is_null(), "Error should be null after clear");

        set_error("test error");
        let error = sb_last_error();
        assert!(!error.is_null(), "Error should not be null after set");

        clear_error();
        let error = sb_last_error();
        assert!(error.is_null(), "Error should be null after second clear");
    }

    #[test]
    fn test_capture_device_count() {
        let mut count = 0usize;
        let result = unsafe { sb_capture_device_count(&mut count) };
        assert_eq!(result, SbError::Ok as c_int);
        println!("Capture devices: {}", count);
    }

    #[test]
    fn test_playback_device_count() {
        let mut count = 0usize;
        let result = unsafe { sb_playback_device_count(&mut count) };
        assert_eq!(result, SbError::Ok as c_int);
        println!("Playback devices: {}", count);
    }

    #[test]
    fn test_null_engine() {
        let result = unsafe { sb_capture_start(ptr::null_mut(), ptr::null()) };
        assert_eq!(result, SbError::InvalidArgument as c_int);
    }

    #[test]
    fn test_bind_and_connect() {
        unsafe {
            let engine = sb_engine_create();

            // 绑定端口
            let result = sb_bind(engine, 0);
            assert_eq!(result, SbError::Ok as c_int);

            // 获取本地端口
            let mut port: u16 = 0;
            let result = sb_local_port(engine, &mut port);
            assert_eq!(result, SbError::Ok as c_int);
            assert!(port > 0, "Port should be non-zero after bind");

            // 设置目标地址
            let addr = CString::new("127.0.0.1:12345").unwrap();
            let result = sb_connect(engine, addr.as_ptr());
            assert_eq!(result, SbError::Ok as c_int);

            sb_engine_destroy(engine);
        }
    }

    #[test]
    fn test_pipeline_not_ready() {
        unsafe {
            let engine = sb_engine_create();

            // 没有启动采集和播放，管线应该返回 PipelineNotReady
            let result = sb_pipeline_start(engine);
            assert_eq!(result, SbError::PipelineNotReady as c_int);

            sb_engine_destroy(engine);
        }
    }

    #[test]
    fn test_pipeline_state() {
        unsafe {
            let engine = sb_engine_create();

            let mut state: c_int = -1;
            let result = sb_pipeline_state(engine, &mut state);
            assert_eq!(result, SbError::Ok as c_int);
            assert_eq!(state, 0); // Stopped

            sb_engine_destroy(engine);
        }
    }
}
