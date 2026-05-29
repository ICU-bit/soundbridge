use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SampleFormat {
    I16,
    I32,
    F32,
    F64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AudioFormat {
    pub sample_rate: u32,
    pub channels: u16,
    pub sample_format: SampleFormat,
}

#[derive(Debug, Error)]
pub enum AudioError {
    #[error("invalid buffer size")]
    InvalidBufferSize,
    #[error("format mismatch")]
    FormatMismatch,
}

pub trait Sample: Clone + Copy + 'static {
    const FORMAT: SampleFormat;
}

impl Sample for i16 {
    const FORMAT: SampleFormat = SampleFormat::I16;
}

impl Sample for i32 {
    const FORMAT: SampleFormat = SampleFormat::I32;
}

impl Sample for f32 {
    const FORMAT: SampleFormat = SampleFormat::F32;
}

impl Sample for f64 {
    const FORMAT: SampleFormat = SampleFormat::F64;
}

pub struct AudioBuffer<T: Sample> {
    data: Vec<u8>,
    sample_count: usize,
    format: AudioFormat,
    _phantom: std::marker::PhantomData<T>,
}

impl<T: Sample> AudioBuffer<T> {
    pub fn new(data: Vec<T>, format: AudioFormat) -> Result<Self, AudioError> {
        let sample_count = data.len();
        let byte_count = sample_count * std::mem::size_of::<T>();
        let mut byte_vec = Vec::with_capacity(byte_count);
        let src = unsafe {
            std::slice::from_raw_parts(data.as_ptr() as *const u8, byte_count)
        };
        byte_vec.extend_from_slice(src);
        Ok(Self {
            data: byte_vec,
            sample_count,
            format,
            _phantom: std::marker::PhantomData,
        })
    }

    pub fn format(&self) -> AudioFormat {
        self.format
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }

    pub fn samples(&self) -> &[T] {
        let align = std::mem::align_of::<T>();
        assert_eq!(
            self.data.as_ptr() as usize % align,
            0,
            "AudioBuffer data is not properly aligned for type {}",
            std::any::type_name::<T>()
        );
        unsafe {
            std::slice::from_raw_parts(
                self.data.as_ptr() as *const T,
                self.sample_count,
            )
        }
    }

    pub fn sample_count(&self) -> usize {
        self.sample_count
    }

    pub fn frame_count(&self) -> usize {
        self.sample_count / self.format.channels as usize
    }
}

impl<T: Sample> Clone for AudioBuffer<T> {
    fn clone(&self) -> Self {
        Self {
            data: self.data.clone(),
            sample_count: self.sample_count,
            format: self.format,
            _phantom: std::marker::PhantomData,
        }
    }
}
