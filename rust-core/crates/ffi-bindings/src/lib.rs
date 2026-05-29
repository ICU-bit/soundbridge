//! SoundBridge FFI 绑定模块
//!
//! 提供 C API，供 Windows C++ 和 Android JNI 调用。

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_void};
use std::ptr;
use std::sync::Mutex;

use audio_capture::{CaptureConfig, CaptureDevice};
use audio_playback::{PlaybackConfig, PlaybackDevice};
use audio_mixer::AudioMixer;
use audio_processor::AudioProcessor;

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

/// SoundBridge 引擎
pub struct SbEngine {
    capture: Option<CaptureDevice>,
    playback: Option<PlaybackDevice>,
    mixer: AudioMixer,
    processor: AudioProcessor,
    pipeline_state: PipelineState,
    pipeline_stats: PipelineStats,
}

/// 管线状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PipelineState {
    Stopped,
    Running,
    Error,
}

/// 管线统计
#[derive(Debug, Clone, Default)]
struct PipelineStats {
    frames_captured: u64,
    frames_played: u64,
    frames_encoded: u64,
    frames_decoded: u64,
    frames_dropped: u64,
    latency_ms: f32,
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
        pipeline_stats: PipelineStats::default(),
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
        let _ = Box::from_raw(engine as *mut SbEngine);
    }
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

    // 启动采集
    if let Some(ref mut capture) = engine.capture {
        if let Err(e) = capture.start() {
            set_error(&format!("failed to start capture: {}", e));
            return SbError::StreamError as c_int;
        }
    }

    // 启动播放
    if let Some(ref mut playback) = engine.playback {
        if let Err(e) = playback.start() {
            set_error(&format!("failed to start playback: {}", e));
            return SbError::StreamError as c_int;
        }
    }

    engine.pipeline_state = PipelineState::Running;
    SbError::Ok as c_int
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

    // 停止采集
    if let Some(ref mut capture) = engine.capture {
        let _ = capture.stop();
    }

    // 停止播放
    if let Some(ref mut playback) = engine.playback {
        let _ = playback.stop();
    }

    engine.pipeline_state = PipelineState::Stopped;
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
/// `stats` 必须是有效的指针。
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

    unsafe {
        *frames_captured = engine.pipeline_stats.frames_captured;
        *frames_played = engine.pipeline_stats.frames_played;
        *latency_ms = engine.pipeline_stats.latency_ms;
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
        // 测试基本的错误设置和清除流程
        clear_error();
        let error = sb_last_error();
        assert!(error.is_null(), "Error should be null after clear");

        // 设置错误后，指针应该非空
        set_error("test error");
        let error = sb_last_error();
        assert!(!error.is_null(), "Error should not be null after set");

        // 清除后应该为空
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
}
