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
use audio_core::{AudioMode, AudioModeManager, RingBuffer};
use audio_mixer::AudioMixer;
use audio_playback::{PlaybackConfig, PlaybackDevice};
use audio_processor::AudioProcessor;
use protocol::{ControlMessage, ControlMessageType, Packet, PacketHeader, Protocol};
use std::time::Instant;

/// 连接状态（FFI 暴露）
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SbConnectionState {
    /// 未连接
    Disconnected = 0,
    /// 正在连接
    Connecting = 1,
    /// 已连接
    Connected = 2,
    /// 连接错误
    Error = 3,
}

/// 状态回调函数类型
///
/// `state` 是新的连接状态。
/// `user_data` 是注册回调时传入的用户数据指针。
pub type SbStateCallback = extern "C" fn(state: SbConnectionState, user_data: *mut c_void);

/// 音频模式
///
/// - `Balanced`（0）：均衡模式，50-100ms 延迟
/// - `HighQuality`（1）：高音质模式，48kHz/24bit
/// - `LowLatency`（2）：超低延迟模式，<30ms
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SbAudioMode {
    /// 均衡模式（默认）
    Balanced = 0,
    /// 高音质模式
    HighQuality = 1,
    /// 超低延迟模式
    LowLatency = 2,
}

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
    /// 最后接收到的序列号（用于丢包检测）
    last_received_seq: AtomicU64,
    /// 累计丢包数
    packets_lost: AtomicU64,
    /// 丢包率（f32 的 bits 表示，用于原子存储）
    loss_rate_bits: AtomicU32,
}

impl SharedPipelineStats {
    fn new() -> Self {
        Self {
            frames_encoded: AtomicU64::new(0),
            frames_decoded: AtomicU64::new(0),
            frames_dropped: AtomicU64::new(0),
            packets_sent: AtomicU64::new(0),
            last_received_seq: AtomicU64::new(0),
            packets_lost: AtomicU64::new(0),
            loss_rate_bits: AtomicU32::new(0),
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

    /// 音频模式
    audio_mode: SbAudioMode,

    /// 音频模式管理器
    mode_manager: AudioModeManager,

    /// 混音比例（PC 音量, 手机音量）- 使用 AtomicU32 实现跨线程实时更新
    /// 通过 f32::to_bits() / f32::from_bits() 进行原子读写
    mix_pc_volume: Arc<AtomicU32>,
    mix_phone_volume: Arc<AtomicU32>,

    /// 连接状态
    connection_state: SbConnectionState,

    /// 状态回调
    state_callback: Option<SbStateCallback>,

    /// 状态回调用户数据
    state_user_data: *mut c_void,

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

    /// 音量 (0.0 ~ 1.0)
    volume: f32,

    /// 是否暂停
    paused: bool,
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
        audio_mode: SbAudioMode::Balanced,
        mode_manager: AudioModeManager::new(),
        mix_pc_volume: Arc::new(AtomicU32::new(0.5f32.to_bits())),
        mix_phone_volume: Arc::new(AtomicU32::new(0.5f32.to_bits())),
        connection_state: SbConnectionState::Disconnected,
        state_callback: None,
        state_user_data: ptr::null_mut(),
        target_addr: None,
        udp_socket: None,
        local_port: 0,
        sequence: Arc::new(AtomicU32::new(0)),
        pipeline: None,
        volume: 1.0,
        paused: false,
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

/// 设置连接状态并触发回调
///
/// 如果状态未变化则不触发回调。
fn set_connection_state(engine: &mut SbEngine, new_state: SbConnectionState) {
    if engine.connection_state == new_state {
        return;
    }
    engine.connection_state = new_state;
    if let Some(cb) = engine.state_callback {
        cb(new_state, engine.state_user_data);
    }
}

/// 设置连接状态回调
///
/// 当连接状态发生变化时，调用 `callback(state, user_data)`。
/// 传入 `None` 作为 callback 可取消注册。
///
/// # Safety
/// `engine` 必须是通过 `sb_engine_create` 创建的有效指针。
/// `callback` 如果非 null，必须在引擎生命周期内保持有效。
/// `user_data` 会被原样传递给回调，调用者负责其生命周期。
#[no_mangle]
pub unsafe extern "C" fn sb_set_state_callback(
    engine: *mut c_void,
    callback: Option<SbStateCallback>,
    user_data: *mut c_void,
) -> c_int {
    clear_error();

    if engine.is_null() {
        set_error("engine is null");
        return SbError::InvalidArgument as c_int;
    }

    let engine = unsafe { &mut *(engine as *mut SbEngine) };
    engine.state_callback = callback;
    engine.state_user_data = user_data;

    SbError::Ok as c_int
}

/// 获取当前连接状态
///
/// # Safety
/// `engine` 必须是通过 `sb_engine_create` 创建的有效指针。
/// `state` 必须是有效的指针。
#[no_mangle]
pub unsafe extern "C" fn sb_get_connection_state(
    engine: *mut c_void,
    state: *mut SbConnectionState,
) -> c_int {
    clear_error();

    if engine.is_null() || state.is_null() {
        set_error("engine or state is null");
        return SbError::InvalidArgument as c_int;
    }

    let engine = unsafe { &*(engine as *const SbEngine) };
    unsafe {
        *state = engine.connection_state;
    }

    SbError::Ok as c_int
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
            set_connection_state(engine, SbConnectionState::Connecting);
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

    // 创建编解码器（使用当前音频模式的配置）
    let mode_config = engine.mode_manager.current_config();
    let sample_rate = audio_codec::SampleRate::from_u32(mode_config.sample_rate)
        .unwrap_or(audio_codec::SampleRate::Hz48000);
    let bitrate = match mode_config.bitrate {
        64000 => audio_codec::Bitrate::Kbps64,
        128000 => audio_codec::Bitrate::Kbps128,
        256000 => audio_codec::Bitrate::Kbps256,
        _ => audio_codec::Bitrate::Kbps128,
    };
    let frame_size = match mode_config.frame_size_ms {
        10 => audio_codec::FrameSize::Ms10,
        20 => audio_codec::FrameSize::Ms20,
        40 => audio_codec::FrameSize::Ms40,
        _ => audio_codec::FrameSize::Ms20,
    };
    let opus_config = OpusConfig::new(
        sample_rate,
        audio_codec::ChannelConfig::Mono,
        bitrate,
        frame_size,
    );
    tracing::info!(
        "Creating codec with mode {:?}: sr={}, bitrate={}, frame_ms={}",
        engine.mode_manager.current_mode(),
        opus_config.sample_rate.value(),
        opus_config.bitrate.bits_per_second(),
        opus_config.frame_size.milliseconds()
    );
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

    // 创建本地混音 ring buffer：发送线程写入采集数据副本，接收线程读取用于混音
    // 避免接收线程和发送线程同时读取同一个 SPSC capture_ring
    let local_mix_ring: Arc<RingBuffer<f32>> = Arc::new(RingBuffer::new(4800));

    // 克隆 socket 和 target 给发送线程
    let send_socket = socket.clone();
    let recv_socket = socket;
    let frame_size = opus_config.frame_size_samples(); // 960 samples per frame (20ms @ 48kHz)

    // 启动发送线程：采集 ring buffer → 编码 → UDP 发送
    let sender_running = running.clone();
    let sender_stats = stats.clone();
    let sender_sequence = sequence.clone();
    let sender_capture_ring = capture_ring.clone();
    let sender_mix_ring = local_mix_ring.clone();
    let sender_handle = match std::thread::Builder::new()
        .name("sb-sender".to_string())
        .spawn(move || {
            let mut encoder = encoder;
            let mut frame_buf = vec![0.0f32; frame_size];
            let mut i16_buf = vec![0i16; frame_size]; // 预分配 i16 缓冲区
            let mut opus_buf = vec![0u8; 1500]; // 预分配编码输出缓冲区
            let protocol = Protocol::new();
            let mut packet_buf = Vec::with_capacity(1500); // 预分配序列化缓冲区
            let start_time = Instant::now();

            tracing::info!("Sender thread started, frame_size={}", frame_size);

            while sender_running.load(Ordering::Relaxed) {
                // 从采集 ring buffer 读取一帧数据
                // cpal 回调持续向 ring buffer 写入数据，这里轮询读取
                let read = sender_capture_ring.read(&mut frame_buf);
                if read < frame_size {
                    // 数据不足一帧，让出 CPU 时间片
                    std::thread::yield_now();
                    continue;
                }

                // 将采集数据写入本地混音 ring buffer，供接收线程混音使用
                sender_mix_ring.write(&frame_buf[..frame_size]);

                // 编码一帧（零分配版本）
                let opus_len = match encoder.encode_interleaved_into(
                    &frame_buf[..frame_size],
                    &mut i16_buf,
                    &mut opus_buf,
                ) {
                    Ok(len) => len,
                    Err(e) => {
                        tracing::warn!("Encode error: {}", e);
                        continue;
                    }
                };

                let seq = sender_sequence.fetch_add(1, Ordering::Relaxed);
                let timestamp_ms = start_time.elapsed().as_millis() as u32;

                // 构造协议包
                let header = PacketHeader {
                    sequence: seq,
                    timestamp_ms,
                    flags: 0x01, // 音频数据标志
                    channels: 1, // mono
                    opus_length: opus_len as u16,
                };

                // 使用零分配序列化
                if let Err(e) = protocol.serialize_audio_into(&header, &opus_buf[..opus_len], &mut packet_buf) {
                    tracing::warn!("Serialize error: {}", e);
                    continue;
                }

                match send_socket.send_to(&packet_buf, target) {
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

            tracing::info!("Sender thread stopped");
        }) {
        Ok(h) => Some(h),
        Err(e) => {
            set_error(&format!("failed to spawn sender thread: {}", e));
            return SbError::Error as c_int;
        }
    };

    // 启动接收线程：UDP 接收 → 解码 → 混音 → 播放 ring buffer
    let receiver_running = running.clone();
    let receiver_stats = stats.clone();
    let receiver_playback_ring = playback_ring.clone();
    let receiver_mix_ring = local_mix_ring.clone();
    let receiver_mixer = engine.mixer.clone();
    let pc_volume = engine.mix_pc_volume.clone();
    let phone_volume = engine.mix_phone_volume.clone();
    let receiver_handle = match std::thread::Builder::new()
        .name("sb-receiver".to_string())
        .spawn(move || {
            let mut decoder = decoder;
            let protocol = Protocol::new();
            let mut recv_buf = vec![0u8; 1500]; // 一个 UDP MTU
            let mut mix_buf = vec![0.0f32; frame_size];
            let mut decode_buf = vec![0.0f32; frame_size]; // 预分配解码缓冲区
            let mut remote_buf = vec![0.0f32; frame_size]; // 预分配远端音频缓冲区
            let mut mixed_buf = vec![0.0f32; frame_size]; // 预分配混音缓冲区

            tracing::info!("Receiver thread started");

            recv_socket
                .set_nonblocking(true)
                .expect("Failed to set non-blocking");

            while receiver_running.load(Ordering::Relaxed) {
                match recv_socket.recv_from(&mut recv_buf) {
                    Ok((len, _from)) => {
                        match protocol.deserialize_header(&recv_buf[..len]) {
                            Ok((header, data, is_audio)) => {
                                if is_audio {
                                    // 丢包检测：跟踪序列号间隙
                                    let seq = header.sequence as u64;
                                    let last_seq = receiver_stats.last_received_seq.load(Ordering::Relaxed);
                                    if last_seq > 0 && seq > last_seq + 1 {
                                        let lost = seq - last_seq - 1;
                                        receiver_stats.packets_lost.fetch_add(lost, Ordering::Relaxed);
                                    }
                                    receiver_stats.last_received_seq.store(seq, Ordering::Relaxed);
                                    
                                    // 更新丢包率
                                    let total_received = receiver_stats.frames_decoded.load(Ordering::Relaxed);
                                    let total_lost = receiver_stats.packets_lost.load(Ordering::Relaxed);
                                    if total_received + total_lost > 0 {
                                        let loss_rate = total_lost as f32 / (total_received + total_lost) as f32;
                                        receiver_stats.loss_rate_bits.store(loss_rate.to_bits(), Ordering::Relaxed);
                                    }
                                    
                                    // 使用零分配解码路径
                                    match decoder.decode_into(data, &mut decode_buf) {
                                        Ok(decoded_count) => {
                                            let remote_samples = &decode_buf[..decoded_count];

                                            // 从本地混音 ring buffer 读取本地音频（由发送线程写入）
                                            let local_read = receiver_mix_ring.read(&mut mix_buf);
                                            if local_read >= frame_size {
                                                // 读取最新的混音比例（原子读取，支持运行时更新）
                                                let pc_vol = f32::from_bits(pc_volume.load(Ordering::Relaxed));
                                                let phone_vol = f32::from_bits(phone_volume.load(Ordering::Relaxed));
                                                
                                                // 将远端音频填充到 frame_size（不足部分补零）
                                                let remote_len = remote_samples.len().min(frame_size);
                                                remote_buf.fill(0.0);
                                                remote_buf[..remote_len].copy_from_slice(&remote_samples[..remote_len]);
                                                
                                                // 混音：本地采集 + 远端解码（长度一致）
                                                if let Err(e) = receiver_mixer.mix_two_into(
                                                    &mix_buf[..frame_size],
                                                    pc_vol,
                                                    &remote_buf,
                                                    phone_vol,
                                                    &mut mixed_buf,
                                                ) {
                                                    // 混音失败，直接写入远端音频
                                                    tracing::warn!("Mix error: {}", e);
                                                    receiver_playback_ring.write(remote_samples);
                                                } else {
                                                    receiver_playback_ring.write(&mixed_buf[..frame_size]);
                                                }
                                            } else {
                                                // 本地数据不足，直接写入远端音频
                                                receiver_playback_ring.write(remote_samples);
                                            }

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
                            }
                            Err(e) => {
                                tracing::warn!("Packet parse error: {}", e);
                            }
                        }
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::yield_now();
                    }
                    Err(e) => {
                        tracing::warn!("Recv error: {}", e);
                        std::thread::yield_now();
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
    set_connection_state(engine, SbConnectionState::Connected);
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
    set_connection_state(engine, SbConnectionState::Disconnected);
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
/// `frames_captured` / `frames_played` / `latency_ms` / `loss_rate` 必须是有效的指针。
#[no_mangle]
pub unsafe extern "C" fn sb_pipeline_stats(
    engine: *mut c_void,
    frames_captured: *mut u64,
    frames_played: *mut u64,
    latency_ms: *mut f32,
    loss_rate: *mut f32,
) -> c_int {
    clear_error();

    if engine.is_null() || frames_captured.is_null() || frames_played.is_null() || latency_ms.is_null() || loss_rate.is_null() {
        set_error("invalid arguments");
        return SbError::InvalidArgument as c_int;
    }

    let engine = unsafe { &*(engine as *const SbEngine) };

    if let Some(ref pipeline) = engine.pipeline {
        let stats = &pipeline.stats;
        unsafe {
            *frames_captured = stats.frames_encoded.load(Ordering::Relaxed);
            *frames_played = stats.frames_decoded.load(Ordering::Relaxed);
            // 基于缓冲区大小估算管线延迟
            // WASAPI buffer (50ms) + ring buffer (~42ms) + cpal buffer (20ms) + codec (~5ms) + network (~5ms)
            // 注：精确测量需要端到端时间戳同步，此处为保守估算
            *latency_ms = 122.0;
            *loss_rate = f32::from_bits(stats.loss_rate_bits.load(Ordering::Relaxed));
        }
    } else {
        unsafe {
            *frames_captured = 0;
            *frames_played = 0;
            *latency_ms = 0.0;
            *loss_rate = 0.0;
        }
    }

    SbError::Ok as c_int
}

// ============================================================
// 双向控制 FFI（音量、暂停、恢复）
// ============================================================

/// 内部发送控制消息
///
/// 通过 UDP 发送控制消息到远端。
/// 前置条件：engine 必须已 bind 并 connect。
fn send_control_packet(engine: &SbEngine, msg_type: ControlMessageType, payload: &[u8]) -> c_int {
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

    // 构造控制消息
    let control_msg = ControlMessage {
        message_type: msg_type,
        payload: payload.to_vec(),
    };

    // 序列化控制消息
    let mut control_data = Vec::new();
    if let Err(e) = control_msg.encode(&mut control_data) {
        set_error(&format!("failed to encode control message: {}", e));
        return SbError::Error as c_int;
    }

    // 构造协议包头（flags=0x00 表示控制数据）
    let seq = engine.sequence.fetch_add(1, Ordering::Relaxed);
    let header = PacketHeader {
        sequence: seq,
        timestamp_ms: 0,
        flags: 0x00,
        channels: 0,
        opus_length: control_data.len() as u16,
    };

    let packet = Packet::Control {
        header,
        data: control_data,
    };

    let protocol = Protocol::new();
    let packet_data = match protocol.serialize(&packet) {
        Ok(d) => d,
        Err(e) => {
            set_error(&format!("failed to serialize control packet: {}", e));
            return SbError::Error as c_int;
        }
    };

    match socket.send_to(&packet_data, target) {
        Ok(_) => SbError::Ok as c_int,
        Err(e) => {
            set_error(&format!("failed to send control packet: {}", e));
            SbError::NetworkError as c_int
        }
    }
}

/// 发送音量控制命令
///
/// 音量范围：0.0（静音）到 1.0（最大）。
///
/// # Safety
/// `engine` 必须是通过 `sb_engine_create` 创建的有效指针。
#[no_mangle]
pub unsafe extern "C" fn sb_send_volume(engine: *mut c_void, volume: f32) -> c_int {
    clear_error();

    if engine.is_null() {
        set_error("engine is null");
        return SbError::InvalidArgument as c_int;
    }

    if !(0.0..=1.0).contains(&volume) {
        set_error("volume must be between 0.0 and 1.0");
        return SbError::InvalidArgument as c_int;
    }

    let engine = unsafe { &mut *(engine as *mut SbEngine) };

    let result = send_control_packet(engine, ControlMessageType::Volume, &volume.to_be_bytes());
    if result == SbError::Ok as c_int {
        engine.volume = volume;
    }
    result
}

/// 发送暂停命令
///
/// # Safety
/// `engine` 必须是通过 `sb_engine_create` 创建的有效指针。
#[no_mangle]
pub unsafe extern "C" fn sb_send_pause(engine: *mut c_void) -> c_int {
    clear_error();

    if engine.is_null() {
        set_error("engine is null");
        return SbError::InvalidArgument as c_int;
    }

    let engine = unsafe { &mut *(engine as *mut SbEngine) };

    let result = send_control_packet(engine, ControlMessageType::StopAudio, &[]);
    if result == SbError::Ok as c_int {
        engine.paused = true;
    }
    result
}

/// 发送恢复命令
///
/// # Safety
/// `engine` 必须是通过 `sb_engine_create` 创建的有效指针。
#[no_mangle]
pub unsafe extern "C" fn sb_send_resume(engine: *mut c_void) -> c_int {
    clear_error();

    if engine.is_null() {
        set_error("engine is null");
        return SbError::InvalidArgument as c_int;
    }

    let engine = unsafe { &mut *(engine as *mut SbEngine) };

    let result = send_control_packet(engine, ControlMessageType::StartAudio, &[]);
    if result == SbError::Ok as c_int {
        engine.paused = false;
    }
    result
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

    if store.is_null() || name.is_null() || buf.is_null() || buf_len == 0 {
        set_error("null argument or zero buffer");
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

    if store.is_null() || buf.is_null() || buf_len == 0 {
        set_error("null argument or zero buffer");
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

// ============================================================
// 音频模式（AudioMode）FFI
// ============================================================

/// 设置音频模式
///
/// 切换音频模式（均衡/高音质/超低延迟），会更新编解码器参数。
/// 如果管线正在运行，需要重启管线才能生效。
///
/// # Safety
/// `engine` 必须是通过 `sb_engine_create` 创建的有效指针。
#[no_mangle]
pub unsafe extern "C" fn sb_set_audio_mode(engine: *mut c_void, mode: SbAudioMode) -> c_int {
    clear_error();

    if engine.is_null() {
        set_error("engine is null");
        return SbError::InvalidArgument as c_int;
    }

    let engine = unsafe { &mut *(engine as *mut SbEngine) };
    engine.audio_mode = mode;

    // 转换 FFI 枚举到 audio-core 枚举
    let audio_mode = match mode {
        SbAudioMode::Balanced => AudioMode::Balanced,
        SbAudioMode::HighQuality => AudioMode::HighQuality,
        SbAudioMode::LowLatency => AudioMode::LowLatency,
    };

    // 通过 AudioModeManager 切换模式
    engine.mode_manager.switch_mode(audio_mode);

    let config = engine.mode_manager.current_config();
    tracing::info!(
        "Audio mode switched to {:?}: bitrate={}, complexity={}, frame_size_ms={}",
        audio_mode,
        config.bitrate,
        config.complexity,
        config.frame_size_ms
    );

    SbError::Ok as c_int
}

/// 获取音频模式
///
/// # Safety
/// `engine` 必须是通过 `sb_engine_create` 创建的有效指针。
/// `mode` 必须是有效的指针。
#[no_mangle]
pub unsafe extern "C" fn sb_get_audio_mode(engine: *mut c_void, mode: *mut SbAudioMode) -> c_int {
    clear_error();

    if engine.is_null() || mode.is_null() {
        set_error("engine or mode is null");
        return SbError::InvalidArgument as c_int;
    }

    let engine = unsafe { &*(engine as *const SbEngine) };
    unsafe {
        *mode = engine.audio_mode;
    }
    SbError::Ok as c_int
}

/// 设置混音比例
///
/// 控制 PC（本地）和手机（远端）的音量平衡。
/// `pc_volume` 和 `phone_volume` 范围为 0.0 到 1.0。
/// 默认值为 0.5 / 0.5（均衡混合）。
///
/// # Safety
/// `engine` 必须是通过 `sb_engine_create` 创建的有效指针。
#[no_mangle]
pub unsafe extern "C" fn sb_set_mix_ratio(
    engine: *mut c_void,
    pc_volume: f32,
    phone_volume: f32,
) -> c_int {
    clear_error();

    if engine.is_null() {
        set_error("engine is null");
        return SbError::InvalidArgument as c_int;
    }

    if !(0.0..=1.0).contains(&pc_volume) || !(0.0..=1.0).contains(&phone_volume) {
        set_error("volume must be between 0.0 and 1.0");
        return SbError::InvalidArgument as c_int;
    }

    let engine = unsafe { &mut *(engine as *mut SbEngine) };
    engine.mix_pc_volume.store(pc_volume.to_bits(), Ordering::Relaxed);
    engine.mix_phone_volume.store(phone_volume.to_bits(), Ordering::Relaxed);

    tracing::info!("Mix ratio set: pc={}, phone={}", pc_volume, phone_volume);
    SbError::Ok as c_int
}

/// 获取混音比例
///
/// # Safety
/// `engine` 必须是通过 `sb_engine_create` 创建的有效指针。
/// `pc_volume` 和 `phone_volume` 必须是有效的指针。
#[no_mangle]
pub unsafe extern "C" fn sb_get_mix_ratio(
    engine: *mut c_void,
    pc_volume: *mut f32,
    phone_volume: *mut f32,
) -> c_int {
    clear_error();

    if engine.is_null() || pc_volume.is_null() || phone_volume.is_null() {
        set_error("engine or volume pointer is null");
        return SbError::InvalidArgument as c_int;
    }

    let engine = unsafe { &*(engine as *const SbEngine) };
    unsafe {
        *pc_volume = f32::from_bits(engine.mix_pc_volume.load(Ordering::Relaxed));
        *phone_volume = f32::from_bits(engine.mix_phone_volume.load(Ordering::Relaxed));
    }
    SbError::Ok as c_int
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

    // ---- 连接状态回调测试 ----

    use std::sync::atomic::AtomicI32;
    use std::sync::Mutex;

    /// 测试用全局状态：记录回调收到的状态值
    static CALLBACK_STATE: AtomicI32 = AtomicI32::new(-1);
    /// 测试用全局状态：记录回调被调用的次数
    static CALLBACK_COUNT: AtomicI32 = AtomicI32::new(0);
    /// 串行化使用共享全局状态的回调测试，防止并行竞态
    static CALLBACK_TEST_LOCK: Mutex<()> = Mutex::new(());

    extern "C" fn test_state_callback(state: SbConnectionState, _user_data: *mut c_void) {
        CALLBACK_STATE.store(state as i32, Ordering::SeqCst);
        CALLBACK_COUNT.fetch_add(1, Ordering::SeqCst);
    }

    #[test]
    fn test_set_state_callback_null_engine() {
        let result = unsafe {
            sb_set_state_callback(ptr::null_mut(), Some(test_state_callback), ptr::null_mut())
        };
        assert_eq!(result, SbError::InvalidArgument as c_int);
        let error = sb_last_error();
        assert!(!error.is_null());
    }

    #[test]
    fn test_set_state_callback_ok() {
        unsafe {
            let engine = sb_engine_create();
            let result = sb_set_state_callback(engine, Some(test_state_callback), ptr::null_mut());
            assert_eq!(result, SbError::Ok as c_int);
            sb_engine_destroy(engine);
        }
    }

    #[test]
    fn test_set_state_callback_none() {
        unsafe {
            let engine = sb_engine_create();
            // 先设置回调
            let result = sb_set_state_callback(engine, Some(test_state_callback), ptr::null_mut());
            assert_eq!(result, SbError::Ok as c_int);
            // 取消回调
            let result = sb_set_state_callback(engine, None, ptr::null_mut());
            assert_eq!(result, SbError::Ok as c_int);
            sb_engine_destroy(engine);
        }
    }

    #[test]
    fn test_get_connection_state_initial() {
        unsafe {
            let engine = sb_engine_create();
            let mut state = SbConnectionState::Error; // 故意设为非默认值
            let result = sb_get_connection_state(engine, &mut state);
            assert_eq!(result, SbError::Ok as c_int);
            assert_eq!(state, SbConnectionState::Disconnected);
            sb_engine_destroy(engine);
        }
    }

    #[test]
    fn test_get_connection_state_null_engine() {
        let mut state = SbConnectionState::Disconnected;
        let result = unsafe { sb_get_connection_state(ptr::null_mut(), &mut state) };
        assert_eq!(result, SbError::InvalidArgument as c_int);
    }

    #[test]
    fn test_get_connection_state_null_state() {
        unsafe {
            let engine = sb_engine_create();
            let result = sb_get_connection_state(engine, ptr::null_mut());
            assert_eq!(result, SbError::InvalidArgument as c_int);
            sb_engine_destroy(engine);
        }
    }

    #[test]
    fn test_state_callback_fires_on_connect() {
        let _guard = CALLBACK_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());

        CALLBACK_STATE.store(-1, Ordering::SeqCst);
        CALLBACK_COUNT.store(0, Ordering::SeqCst);

        unsafe {
            let engine = sb_engine_create();
            sb_set_state_callback(engine, Some(test_state_callback), ptr::null_mut());

            // sb_connect 成功后应触发 Connecting 状态
            let addr = CString::new("127.0.0.1:12345").unwrap();
            let result = sb_connect(engine, addr.as_ptr());
            assert_eq!(result, SbError::Ok as c_int);

            assert_eq!(CALLBACK_STATE.load(Ordering::SeqCst), SbConnectionState::Connecting as i32);
            assert_eq!(CALLBACK_COUNT.load(Ordering::SeqCst), 1);

            sb_engine_destroy(engine);
        }
    }

    #[test]
    fn test_state_callback_no_fire_on_same_state() {
        let _guard = CALLBACK_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());

        CALLBACK_STATE.store(-1, Ordering::SeqCst);
        CALLBACK_COUNT.store(0, Ordering::SeqCst);

        unsafe {
            let engine = sb_engine_create();
            sb_set_state_callback(engine, Some(test_state_callback), ptr::null_mut());

            // 连接同一地址两次（状态不变，不应再次触发）
            let addr = CString::new("127.0.0.1:12345").unwrap();
            sb_connect(engine, addr.as_ptr());
            assert_eq!(CALLBACK_COUNT.load(Ordering::SeqCst), 1);

            sb_connect(engine, addr.as_ptr());
            assert_eq!(CALLBACK_COUNT.load(Ordering::SeqCst), 1); // 不变

            sb_engine_destroy(engine);
        }
    }

    #[test]
    fn test_state_callback_user_data() {
        extern "C" fn callback_with_data(_state: SbConnectionState, user_data: *mut c_void) {
            unsafe {
                let ptr = user_data as *mut i32;
                *ptr = 42;
            }
        }

        unsafe {
            let engine = sb_engine_create();
            let mut value: i32 = 0;
            sb_set_state_callback(engine, Some(callback_with_data), &mut value as *mut i32 as *mut c_void);

            let addr = CString::new("127.0.0.1:12345").unwrap();
            sb_connect(engine, addr.as_ptr());

            assert_eq!(value, 42, "user_data should have been written to 42");

            sb_engine_destroy(engine);
        }
    }

    #[test]
    fn test_connection_state_enum_values() {
        // 验证 repr(C) 枚举值
        assert_eq!(SbConnectionState::Disconnected as c_int, 0);
        assert_eq!(SbConnectionState::Connecting as c_int, 1);
        assert_eq!(SbConnectionState::Connected as c_int, 2);
        assert_eq!(SbConnectionState::Error as c_int, 3);
    }

    // ============================================================
    // 双向控制 FFI 测试（TDD - 先写测试）
    // ============================================================

    #[test]
    fn test_send_volume_null_engine() {
        let result = unsafe { sb_send_volume(ptr::null_mut(), 0.5) };
        assert_eq!(result, SbError::InvalidArgument as c_int);
    }

    #[test]
    fn test_send_volume_invalid_range() {
        unsafe {
            let engine = sb_engine_create();

            let result = sb_send_volume(engine, -0.1);
            assert_eq!(result, SbError::InvalidArgument as c_int);

            let result = sb_send_volume(engine, 1.1);
            assert_eq!(result, SbError::InvalidArgument as c_int);

            sb_engine_destroy(engine);
        }
    }

    #[test]
    fn test_send_volume_no_target() {
        unsafe {
            let engine = sb_engine_create();
            sb_bind(engine, 0);

            let result = sb_send_volume(engine, 0.5);
            assert_eq!(result, SbError::PipelineNotReady as c_int);

            sb_engine_destroy(engine);
        }
    }

    #[test]
    fn test_send_volume_no_socket() {
        unsafe {
            let engine = sb_engine_create();
            let addr = CString::new("127.0.0.1:12345").unwrap();
            sb_connect(engine, addr.as_ptr());

            let result = sb_send_volume(engine, 0.5);
            assert_eq!(result, SbError::PipelineNotReady as c_int);

            sb_engine_destroy(engine);
        }
    }

    #[test]
    fn test_send_volume_success() {
        // 创建接收端 UDP socket
        let receiver = UdpSocket::bind("127.0.0.1:0").unwrap();
        let recv_addr = receiver.local_addr().unwrap();
        receiver.set_nonblocking(true).unwrap();

        unsafe {
            let engine = sb_engine_create();
            sb_bind(engine, 0);

            let addr = CString::new(format!("{}", recv_addr)).unwrap();
            let result = sb_connect(engine, addr.as_ptr());
            assert_eq!(result, SbError::Ok as c_int);

            // 发送音量控制
            let result = sb_send_volume(engine, 0.75);
            assert_eq!(result, SbError::Ok as c_int);

            // 验证引擎内部状态
            let engine_ref = &*(engine as *const SbEngine);
            assert_eq!(engine_ref.volume, 0.75);

            // 验证接收端收到了数据
            let mut recv_buf = vec![0u8; 4096];
            std::thread::sleep(std::time::Duration::from_millis(50));
            let recv_result = receiver.recv_from(&mut recv_buf);
            assert!(recv_result.is_ok(), "Should receive control packet");

            // 反序列化验证
            let (len, _) = recv_result.unwrap();
            let protocol = Protocol::new();
            let packet = protocol.deserialize(&recv_buf[..len]).unwrap();
            match packet {
                Packet::Control { header, data } => {
                    assert_eq!(header.flags, 0x00); // 控制数据标志
                    let (control_msg, _) = ControlMessage::decode(&data).unwrap();
                    assert_eq!(control_msg.message_type, ControlMessageType::Volume);
                    assert_eq!(control_msg.payload.len(), 4);
                    let vol = f32::from_be_bytes([
                        control_msg.payload[0],
                        control_msg.payload[1],
                        control_msg.payload[2],
                        control_msg.payload[3],
                    ]);
                    assert!((vol - 0.75).abs() < f32::EPSILON);
                }
                _ => panic!("Expected Control packet"),
            }

            sb_engine_destroy(engine);
        }
    }

    #[test]
    fn test_send_pause_null_engine() {
        let result = unsafe { sb_send_pause(ptr::null_mut()) };
        assert_eq!(result, SbError::InvalidArgument as c_int);
    }

    #[test]
    fn test_send_pause_success() {
        let receiver = UdpSocket::bind("127.0.0.1:0").unwrap();
        let recv_addr = receiver.local_addr().unwrap();
        receiver.set_nonblocking(true).unwrap();

        unsafe {
            let engine = sb_engine_create();
            sb_bind(engine, 0);

            let addr = CString::new(format!("{}", recv_addr)).unwrap();
            sb_connect(engine, addr.as_ptr());

            let result = sb_send_pause(engine);
            assert_eq!(result, SbError::Ok as c_int);

            // 验证引擎内部状态
            let engine_ref = &*(engine as *const SbEngine);
            assert!(engine_ref.paused);

            // 验证接收端收到了数据
            let mut recv_buf = vec![0u8; 4096];
            std::thread::sleep(std::time::Duration::from_millis(50));
            let recv_result = receiver.recv_from(&mut recv_buf);
            assert!(recv_result.is_ok(), "Should receive pause packet");

            let (len, _) = recv_result.unwrap();
            let protocol = Protocol::new();
            let packet = protocol.deserialize(&recv_buf[..len]).unwrap();
            match packet {
                Packet::Control { header: _, data } => {
                    let (control_msg, _) = ControlMessage::decode(&data).unwrap();
                    assert_eq!(control_msg.message_type, ControlMessageType::StopAudio);
                    assert!(control_msg.payload.is_empty());
                }
                _ => panic!("Expected Control packet"),
            }

            sb_engine_destroy(engine);
        }
    }

    #[test]
    fn test_send_resume_null_engine() {
        let result = unsafe { sb_send_resume(ptr::null_mut()) };
        assert_eq!(result, SbError::InvalidArgument as c_int);
    }

    #[test]
    fn test_send_resume_success() {
        let receiver = UdpSocket::bind("127.0.0.1:0").unwrap();
        let recv_addr = receiver.local_addr().unwrap();
        receiver.set_nonblocking(true).unwrap();

        unsafe {
            let engine = sb_engine_create();
            sb_bind(engine, 0);

            let addr = CString::new(format!("{}", recv_addr)).unwrap();
            sb_connect(engine, addr.as_ptr());

            // 先暂停
            sb_send_pause(engine);
            {
                let engine_ref = &*(engine as *const SbEngine);
                assert!(engine_ref.paused);
            }

            // 发送恢复
            let result = sb_send_resume(engine);
            assert_eq!(result, SbError::Ok as c_int);

            // 验证引擎内部状态
            let engine_ref = &*(engine as *const SbEngine);
            assert!(!engine_ref.paused);

            // 验证接收端收到恢复消息（跳过暂停消息）
            // 先读掉暂停消息
            let mut recv_buf = vec![0u8; 4096];
            std::thread::sleep(std::time::Duration::from_millis(50));
            let _ = receiver.recv_from(&mut recv_buf);

            // 再读恢复消息
            let recv_result = receiver.recv_from(&mut recv_buf);
            assert!(recv_result.is_ok(), "Should receive resume packet");

            let (len, _) = recv_result.unwrap();
            let protocol = Protocol::new();
            let packet = protocol.deserialize(&recv_buf[..len]).unwrap();
            match packet {
                Packet::Control { header: _, data } => {
                    let (control_msg, _) = ControlMessage::decode(&data).unwrap();
                    assert_eq!(control_msg.message_type, ControlMessageType::StartAudio);
                    assert!(control_msg.payload.is_empty());
                }
                _ => panic!("Expected Control packet"),
            }

            sb_engine_destroy(engine);
        }
    }

    #[test]
    fn test_send_volume_boundary_values() {
        let receiver = UdpSocket::bind("127.0.0.1:0").unwrap();
        let recv_addr = receiver.local_addr().unwrap();
        receiver.set_nonblocking(true).unwrap();

        unsafe {
            let engine = sb_engine_create();
            sb_bind(engine, 0);

            let addr = CString::new(format!("{}", recv_addr)).unwrap();
            sb_connect(engine, addr.as_ptr());

            // 测试最小值 0.0
            let result = sb_send_volume(engine, 0.0);
            assert_eq!(result, SbError::Ok as c_int);

            // 测试最大值 1.0
            let result = sb_send_volume(engine, 1.0);
            assert_eq!(result, SbError::Ok as c_int);

            sb_engine_destroy(engine);
        }
    }

    #[test]
    fn test_audio_mode_default() {
        unsafe {
            let engine = sb_engine_create();
            let mut mode = SbAudioMode::LowLatency; // 故意设为非默认值

            let result = sb_get_audio_mode(engine, &mut mode);
            assert_eq!(result, SbError::Ok as c_int);
            assert_eq!(mode, SbAudioMode::Balanced, "Default audio mode should be Balanced");

            sb_engine_destroy(engine);
        }
    }

    #[test]
    fn test_set_and_get_audio_mode() {
        unsafe {
            let engine = sb_engine_create();

            // 设置为高音质模式
            let result = sb_set_audio_mode(engine, SbAudioMode::HighQuality);
            assert_eq!(result, SbError::Ok as c_int);

            // 读取应该返回高音质模式
            let mut mode = SbAudioMode::Balanced;
            let result = sb_get_audio_mode(engine, &mut mode);
            assert_eq!(result, SbError::Ok as c_int);
            assert_eq!(mode, SbAudioMode::HighQuality);

            // 切换为超低延迟模式
            let result = sb_set_audio_mode(engine, SbAudioMode::LowLatency);
            assert_eq!(result, SbError::Ok as c_int);

            let mut mode = SbAudioMode::Balanced;
            let result = sb_get_audio_mode(engine, &mut mode);
            assert_eq!(result, SbError::Ok as c_int);
            assert_eq!(mode, SbAudioMode::LowLatency);

            // 切换回均衡模式
            let result = sb_set_audio_mode(engine, SbAudioMode::Balanced);
            assert_eq!(result, SbError::Ok as c_int);

            let mut mode = SbAudioMode::HighQuality;
            let result = sb_get_audio_mode(engine, &mut mode);
            assert_eq!(result, SbError::Ok as c_int);
            assert_eq!(mode, SbAudioMode::Balanced);

            sb_engine_destroy(engine);
        }
    }

    #[test]
    fn test_audio_mode_null_engine() {
        // sb_set_audio_mode with null engine
        let result = unsafe { sb_set_audio_mode(ptr::null_mut(), SbAudioMode::Balanced) };
        assert_eq!(result, SbError::InvalidArgument as c_int);

        // sb_get_audio_mode with null engine
        let mut mode = SbAudioMode::Balanced;
        let result = unsafe { sb_get_audio_mode(ptr::null_mut(), &mut mode) };
        assert_eq!(result, SbError::InvalidArgument as c_int);
    }

    #[test]
    fn test_audio_mode_null_mode_pointer() {
        unsafe {
            let engine = sb_engine_create();

            // sb_get_audio_mode with null mode pointer
            let result = sb_get_audio_mode(engine, ptr::null_mut());
            assert_eq!(result, SbError::InvalidArgument as c_int);

            sb_engine_destroy(engine);
        }
    }

    #[test]
    fn test_audio_mode_enum_values() {
        // 验证 repr(C) 枚举值
        assert_eq!(SbAudioMode::Balanced as c_int, 0);
        assert_eq!(SbAudioMode::HighQuality as c_int, 1);
        assert_eq!(SbAudioMode::LowLatency as c_int, 2);
    }

    #[test]
    fn test_mix_ratio_default() {
        unsafe {
            let engine = sb_engine_create();

            let mut pc_volume = 0.0f32;
            let mut phone_volume = 0.0f32;
            let result = sb_get_mix_ratio(engine, &mut pc_volume, &mut phone_volume);
            assert_eq!(result, SbError::Ok as c_int);
            assert!((pc_volume - 0.5).abs() < 0.001);
            assert!((phone_volume - 0.5).abs() < 0.001);

            sb_engine_destroy(engine);
        }
    }

    #[test]
    fn test_mix_ratio_set_and_get() {
        unsafe {
            let engine = sb_engine_create();

            // 设置混音比例
            let result = sb_set_mix_ratio(engine, 0.3, 0.7);
            assert_eq!(result, SbError::Ok as c_int);

            // 验证设置后的值
            let mut pc_volume = 0.0f32;
            let mut phone_volume = 0.0f32;
            let result = sb_get_mix_ratio(engine, &mut pc_volume, &mut phone_volume);
            assert_eq!(result, SbError::Ok as c_int);
            assert!((pc_volume - 0.3).abs() < 0.001);
            assert!((phone_volume - 0.7).abs() < 0.001);

            sb_engine_destroy(engine);
        }
    }

    #[test]
    fn test_mix_ratio_boundary_values() {
        unsafe {
            let engine = sb_engine_create();

            // 测试最小值
            let result = sb_set_mix_ratio(engine, 0.0, 0.0);
            assert_eq!(result, SbError::Ok as c_int);

            // 测试最大值
            let result = sb_set_mix_ratio(engine, 1.0, 1.0);
            assert_eq!(result, SbError::Ok as c_int);

            sb_engine_destroy(engine);
        }
    }

    #[test]
    fn test_mix_ratio_invalid_values() {
        unsafe {
            let engine = sb_engine_create();

            // 测试超出范围的值
            let result = sb_set_mix_ratio(engine, -0.1, 0.5);
            assert_eq!(result, SbError::InvalidArgument as c_int);

            let result = sb_set_mix_ratio(engine, 0.5, 1.1);
            assert_eq!(result, SbError::InvalidArgument as c_int);

            sb_engine_destroy(engine);
        }
    }

    #[test]
    fn test_mix_ratio_null_engine() {
        let mut pc_volume = 0.0f32;
        let mut phone_volume = 0.0f32;

        let result = unsafe { sb_set_mix_ratio(ptr::null_mut(), 0.5, 0.5) };
        assert_eq!(result, SbError::InvalidArgument as c_int);

        let result = unsafe { sb_get_mix_ratio(ptr::null_mut(), &mut pc_volume, &mut phone_volume) };
        assert_eq!(result, SbError::InvalidArgument as c_int);
    }

    #[test]
    fn test_mix_ratio_null_pointers() {
        unsafe {
            let engine = sb_engine_create();

            let result = sb_get_mix_ratio(engine, ptr::null_mut(), ptr::null_mut());
            assert_eq!(result, SbError::InvalidArgument as c_int);

            sb_engine_destroy(engine);
        }
    }

    // ============================================================
    // 带宽自适应测试
    // ============================================================

    #[test]
    fn test_shared_pipeline_stats_loss_rate() {
        let stats = SharedPipelineStats::new();

        // 初始状态：无丢包
        assert_eq!(stats.packets_lost.load(Ordering::Relaxed), 0);
        assert_eq!(stats.loss_rate_bits.load(Ordering::Relaxed), 0);

        // 模拟接收 10 个包，丢失 2 个
        stats.frames_decoded.store(10, Ordering::Relaxed);
        stats.packets_lost.store(2, Ordering::Relaxed);

        // 计算丢包率（使用 frames_decoded 而非 packets_sent）
        let total_received = stats.frames_decoded.load(Ordering::Relaxed);
        let total_lost = stats.packets_lost.load(Ordering::Relaxed);
        let loss_rate = total_lost as f32 / (total_received + total_lost) as f32;

        // 验证丢包率计算
        assert!((loss_rate - 0.16666667).abs() < 0.001);
    }

    #[test]
    fn test_shared_pipeline_stats_sequence_tracking() {
        let stats = SharedPipelineStats::new();

        // 模拟接收序列号：1, 2, 3, 5, 6, 8（丢失 4 和 7）
        let sequences = vec![1, 2, 3, 5, 6, 8];
        let mut last_seq = 0u64;

        for seq in sequences {
            let seq_u64 = seq as u64;
            if last_seq > 0 && seq_u64 > last_seq + 1 {
                let lost = seq_u64 - last_seq - 1;
                stats.packets_lost.fetch_add(lost, Ordering::Relaxed);
            }
            stats.last_received_seq.store(seq_u64, Ordering::Relaxed);
            last_seq = seq_u64;
        }

        // 验证累计丢包数：丢失 4 和 7，共 2 个
        assert_eq!(stats.packets_lost.load(Ordering::Relaxed), 2);
        assert_eq!(stats.last_received_seq.load(Ordering::Relaxed), 8);
    }

    #[test]
    fn test_pipeline_stats_null_loss_rate() {
        unsafe {
            let engine = sb_engine_create();

            let mut frames_captured = 0u64;
            let mut frames_played = 0u64;
            let mut latency_ms = 0.0f32;

            // 传入 null loss_rate 指针
            let result = sb_pipeline_stats(
                engine,
                &mut frames_captured,
                &mut frames_played,
                &mut latency_ms,
                ptr::null_mut(),
            );
            assert_eq!(result, SbError::InvalidArgument as c_int);

            sb_engine_destroy(engine);
        }
    }
}
