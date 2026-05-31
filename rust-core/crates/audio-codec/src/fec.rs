use opus::{Bitrate as OpusBitrate, Decoder as OpusDecoder, Encoder as OpusEncoder};

use crate::{Bitrate, CodecError, OpusConfig, Result};

/// FEC 编解码配置
#[derive(Debug, Clone, Copy)]
pub struct FecConfig {
    pub opus: OpusConfig,
    /// 期望的丢包百分比 (0-100)
    pub packet_loss_perc: i32,
}

impl Default for FecConfig {
    fn default() -> Self {
        Self {
            opus: OpusConfig::default(),
            packet_loss_perc: 5,
        }
    }
}

impl FecConfig {
    pub fn new(opus: OpusConfig, packet_loss_perc: i32) -> Self {
        Self {
            opus,
            packet_loss_perc: packet_loss_perc.clamp(0, 100),
        }
    }
}

/// FEC 编码器 — 封装 Opus 编码器并启用 inband FEC
///
/// Inband FEC 在当前帧中嵌入前一帧的冗余数据。
/// 解码器在检测到丢包时可利用冗余数据恢复音频。
pub struct FecEncoder {
    encoder: OpusEncoder,
    config: FecConfig,
}

impl FecEncoder {
    pub fn new(config: FecConfig) -> Result<Self> {
        let mut encoder = OpusEncoder::new(
            config.opus.sample_rate.value(),
            config.opus.channels.to_opus_channels(),
            config.opus.application,
        )
        .map_err(|e| CodecError::EncoderError(e.to_string()))?;

        encoder
            .set_bitrate(OpusBitrate::Bits(config.opus.bitrate.bits_per_second()))
            .map_err(|e| CodecError::EncoderError(e.to_string()))?;

        encoder
            .set_inband_fec(true)
            .map_err(|e| CodecError::EncoderError(e.to_string()))?;

        encoder
            .set_packet_loss_perc(config.packet_loss_perc)
            .map_err(|e| CodecError::EncoderError(e.to_string()))?;

        Ok(Self { encoder, config })
    }

    pub fn config(&self) -> &FecConfig {
        &self.config
    }

    /// 动态更新丢包率预期
    pub fn set_packet_loss_perc(&mut self, perc: i32) -> Result<()> {
        let clamped = perc.clamp(0, 100);
        self.encoder
            .set_packet_loss_perc(clamped)
            .map_err(|e| CodecError::EncoderError(e.to_string()))?;
        self.config.packet_loss_perc = clamped;
        Ok(())
    }

    /// 动态调整码率
    pub fn set_bitrate(&mut self, bitrate: Bitrate) -> Result<()> {
        self.encoder
            .set_bitrate(OpusBitrate::Bits(bitrate.bits_per_second()))
            .map_err(|e| CodecError::EncoderError(e.to_string()))?;
        self.config.opus.bitrate = bitrate;
        Ok(())
    }

    /// 带 FEC 的编码，返回编码后的字节
    pub fn encode_with_fec(&mut self, samples: &[f32]) -> Result<Vec<u8>> {
        let expected = self.config.opus.total_samples();
        if samples.len() != expected {
            return Err(CodecError::BufferSizeMismatch {
                expected,
                actual: samples.len(),
            });
        }

        let samples_i16: Vec<i16> = samples
            .iter()
            .map(|&s| (s * 32767.0).clamp(-32768.0, 32767.0) as i16)
            .collect();

        let frame_size = self.config.opus.frame_size_samples();
        self.encoder
            .encode_vec(&samples_i16, frame_size)
            .map_err(|e| CodecError::EncodingFailed(e.to_string()))
    }

    /// 零分配编码：写入预分配缓冲区，返回写入字节数
    pub fn encode_with_fec_into(
        &mut self,
        samples: &[f32],
        i16_buf: &mut [i16],
        output: &mut [u8],
    ) -> Result<usize> {
        let expected = self.config.opus.total_samples();
        if samples.len() != expected {
            return Err(CodecError::BufferSizeMismatch {
                expected,
                actual: samples.len(),
            });
        }
        if i16_buf.len() < expected {
            return Err(CodecError::BufferSizeMismatch {
                expected,
                actual: i16_buf.len(),
            });
        }

        for (dst, &src) in i16_buf.iter_mut().zip(samples.iter()) {
            *dst = (src * 32767.0).clamp(-32768.0, 32767.0) as i16;
        }

        self.encoder
            .encode(&i16_buf[..expected], output)
            .map_err(|e| CodecError::EncodingFailed(e.to_string()))
    }
}

/// FEC 解码器 — 封装 Opus 解码器，支持利用 FEC 数据恢复丢失帧
pub struct FecDecoder {
    decoder: OpusDecoder,
    config: FecConfig,
}

impl FecDecoder {
    pub fn new(config: FecConfig) -> Result<Self> {
        let decoder = OpusDecoder::new(
            config.opus.sample_rate.value(),
            config.opus.channels.to_opus_channels(),
        )
        .map_err(|e| CodecError::DecoderError(e.to_string()))?;

        Ok(Self { decoder, config })
    }

    pub fn config(&self) -> &FecConfig {
        &self.config
    }

    /// 正常解码（fec=false）
    pub fn decode(&mut self, data: &[u8]) -> Result<Vec<f32>> {
        self.decode_impl(data, false)
    }

    /// 利用 FEC 数据解码丢失帧（fec=true，data 传入下一帧数据）
    pub fn decode_fec(&mut self, data: &[u8]) -> Result<Vec<f32>> {
        self.decode_impl(data, true)
    }

    fn decode_impl(&mut self, data: &[u8], fec: bool) -> Result<Vec<f32>> {
        let expected = self.config.opus.total_samples();
        let channels = self.config.opus.channels.count() as usize;

        let mut output = vec![0.0f32; expected];
        let decoded_count = self
            .decoder
            .decode_float(data, &mut output, fec)
            .map_err(|e| CodecError::DecodingFailed(e.to_string()))?;

        let total_decoded = decoded_count * channels;
        output.truncate(total_decoded);
        Ok(output)
    }

    /// 零分配解码，返回写入的总样本数
    pub fn decode_into(&mut self, data: &[u8], output: &mut [f32]) -> Result<usize> {
        self.decode_into_impl(data, output, false)
    }

    /// 零分配 FEC 解码
    pub fn decode_fec_into(&mut self, data: &[u8], output: &mut [f32]) -> Result<usize> {
        self.decode_into_impl(data, output, true)
    }

    fn decode_into_impl(&mut self, data: &[u8], output: &mut [f32], fec: bool) -> Result<usize> {
        let expected = self.config.opus.total_samples();
        let channels = self.config.opus.channels.count() as usize;

        if output.len() < expected {
            return Err(CodecError::BufferSizeMismatch {
                expected,
                actual: output.len(),
            });
        }

        let decoded_count = self
            .decoder
            .decode_float(data, output, fec)
            .map_err(|e| CodecError::DecodingFailed(e.to_string()))?;

        Ok(decoded_count * channels)
    }
}

/// FEC 编解码器 — 组合 FecEncoder + FecDecoder
pub struct FecCodec {
    encoder: FecEncoder,
    decoder: FecDecoder,
    config: FecConfig,
}

impl FecCodec {
    pub fn new(config: FecConfig) -> Result<Self> {
        let encoder = FecEncoder::new(config)?;
        let decoder = FecDecoder::new(config)?;
        Ok(Self {
            encoder,
            decoder,
            config,
        })
    }

    pub fn encode(&mut self, samples: &[f32]) -> Result<Vec<u8>> {
        self.encoder.encode_with_fec(samples)
    }

    pub fn decode(&mut self, data: &[u8]) -> Result<Vec<f32>> {
        self.decoder.decode(data)
    }

    /// 解码丢失帧（利用 FEC 冗余数据，需提供下一帧数据）
    pub fn decode_lost(&mut self, next_packet: &[u8]) -> Result<Vec<f32>> {
        self.decoder.decode_fec(next_packet)
    }

    pub fn encoder(&mut self) -> &mut FecEncoder {
        &mut self.encoder
    }

    pub fn decoder(&mut self) -> &mut FecDecoder {
        &mut self.decoder
    }

    pub fn config(&self) -> &FecConfig {
        &self.config
    }
}
