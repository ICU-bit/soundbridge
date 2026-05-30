#include <gtest/gtest.h>
#include "core/audio_pipeline.h"
#include "core/audio_types.h"

#include <cmath>
#include <vector>

namespace soundbridge {
namespace tests {

class AudioPipelineTest : public ::testing::Test {
protected:
    void SetUp() override {
        config_.format.sample_rate = 48000;
        config_.format.channels = 1;
        config_.format.sample_format = AudioSampleFormat::Float32;
        config_.format.bits_per_sample = 32;
        config_.format.frame_size = 960;
        config_.enable_aec = false;
        config_.enable_ns = false;
        config_.enable_agc = false;
        config_.opus_bitrate = 64000;
    }

    PipelineConfig config_;

    // 生成正弦波测试数据
    std::vector<float> generate_sine_wave(uint32_t frame_count, float frequency = 440.0f) {
        std::vector<float> samples(frame_count);
        for (uint32_t i = 0; i < frame_count; ++i) {
            samples[i] = 0.5f * std::sin(2.0f * 3.14159265f * frequency * i / 48000.0f);
        }
        return samples;
    }
};

// 测试管线初始化
TEST_F(AudioPipelineTest, InitializeSuccess) {
    AudioPipeline pipeline(config_);
    EXPECT_FALSE(pipeline.is_initialized());
    EXPECT_TRUE(pipeline.initialize());
    EXPECT_TRUE(pipeline.is_initialized());
}

// 测试管线关闭
TEST_F(AudioPipelineTest, Shutdown) {
    AudioPipeline pipeline(config_);
    ASSERT_TRUE(pipeline.initialize());
    EXPECT_TRUE(pipeline.is_initialized());

    pipeline.shutdown();
    EXPECT_FALSE(pipeline.is_initialized());
}

// 测试编码解码往返
TEST_F(AudioPipelineTest, EncodeDecodeRoundtrip) {
    AudioPipeline pipeline(config_);
    ASSERT_TRUE(pipeline.initialize());

    auto input = generate_sine_wave(960);
    auto encoded = pipeline.encode(input.data(), 960);

    EXPECT_FALSE(encoded.empty());
    EXPECT_LE(encoded.size(), 1500);

    auto decoded = pipeline.decode(encoded.data(), encoded.size());
    EXPECT_EQ(decoded.size(), 960);

    // 解码后的数据应该与输入相似
    float max_diff = 0.0f;
    for (size_t i = 0; i < 960; ++i) {
        max_diff = std::max(max_diff, std::abs(input[i] - decoded[i]));
    }
    EXPECT_LT(max_diff, 0.5f);
}

// 测试采集处理（无 AEC/NS/AGC 时应该直通）
TEST_F(AudioPipelineTest, ProcessCapturePassthrough) {
    AudioPipeline pipeline(config_);
    ASSERT_TRUE(pipeline.initialize());

    auto input = generate_sine_wave(960);
    std::vector<float> output = input;

    pipeline.process_capture(output.data(), 960);

    // 无 AEC/NS/AGC 时，输出应该与输入相似
    float max_diff = 0.0f;
    for (size_t i = 0; i < 960; ++i) {
        max_diff = std::max(max_diff, std::abs(input[i] - output[i]));
    }
    EXPECT_LT(max_diff, 0.01f);
}

// 测试播放处理
TEST_F(AudioPipelineTest, ProcessRender) {
    AudioPipeline pipeline(config_);
    ASSERT_TRUE(pipeline.initialize());

    auto input = generate_sine_wave(960);
    std::vector<float> output = input;

    pipeline.process_render(output.data(), 960);

    // 播放处理后数据应该存在
    bool has_nonzero = false;
    for (auto sample : output) {
        if (std::abs(sample) > 0.001f) {
            has_nonzero = true;
            break;
        }
    }
    EXPECT_TRUE(has_nonzero);
}

// 测试启用 AEC
TEST_F(AudioPipelineTest, InitializeWithAEC) {
    config_.enable_aec = true;
    AudioPipeline pipeline(config_);
    EXPECT_TRUE(pipeline.initialize());
    EXPECT_TRUE(pipeline.is_initialized());
}

// 测试启用 NS
TEST_F(AudioPipelineTest, InitializeWithNS) {
    config_.enable_ns = true;
    AudioPipeline pipeline(config_);
    EXPECT_TRUE(pipeline.initialize());
    EXPECT_TRUE(pipeline.is_initialized());
}

// 测试启用 AGC
TEST_F(AudioPipelineTest, InitializeWithAGC) {
    config_.enable_agc = true;
    AudioPipeline pipeline(config_);
    EXPECT_TRUE(pipeline.initialize());
    EXPECT_TRUE(pipeline.is_initialized());
}

// 测试全部启用
TEST_F(AudioPipelineTest, InitializeWithAllProcessing) {
    config_.enable_aec = true;
    config_.enable_ns = true;
    config_.enable_agc = true;
    AudioPipeline pipeline(config_);
    EXPECT_TRUE(pipeline.initialize());
    EXPECT_TRUE(pipeline.is_initialized());
}

// 测试编码静音
TEST_F(AudioPipelineTest, EncodeSilence) {
    AudioPipeline pipeline(config_);
    ASSERT_TRUE(pipeline.initialize());

    std::vector<float> silence(960, 0.0f);
    auto encoded = pipeline.encode(silence.data(), 960);
    EXPECT_FALSE(encoded.empty());

    auto decoded = pipeline.decode(encoded.data(), encoded.size());
    EXPECT_EQ(decoded.size(), 960);

    // 解码后的静音应该接近 0
    for (auto sample : decoded) {
        EXPECT_LT(std::abs(sample), 0.01f);
    }
}

// 测试不同码率
TEST_F(AudioPipelineTest, DifferentBitrates) {
    config_.opus_bitrate = 128000;
    AudioPipeline pipeline1(config_);
    EXPECT_TRUE(pipeline1.initialize());

    config_.opus_bitrate = 256000;
    AudioPipeline pipeline2(config_);
    EXPECT_TRUE(pipeline2.initialize());
}

} // namespace tests
} // namespace soundbridge
