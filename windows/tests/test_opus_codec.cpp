#include <gtest/gtest.h>
#include "audio/opus_codec.h"

#include <cmath>
#include <vector>

namespace soundbridge {
namespace tests {

class OpusCodecTest : public ::testing::Test {
protected:
    void SetUp() override {
        codec = std::make_unique<OpusCodec>();
    }

    void TearDown() override {
        if (codec) {
            codec->shutdown();
        }
    }

    std::unique_ptr<OpusCodec> codec;

    // 生成正弦波测试数据
    std::vector<float> generate_sine_wave(uint32_t frame_count, float frequency = 440.0f) {
        std::vector<float> samples(frame_count);
        for (uint32_t i = 0; i < frame_count; ++i) {
            samples[i] = 0.5f * std::sin(2.0f * 3.14159265f * frequency * i / 48000.0f);
        }
        return samples;
    }
};

// 测试初始化
TEST_F(OpusCodecTest, InitializeSuccess) {
    EXPECT_FALSE(codec->is_initialized());
    EXPECT_TRUE(codec->initialize(48000, 1, 64000));
    EXPECT_TRUE(codec->is_initialized());
}

// 测试初始化参数
TEST_F(OpusCodecTest, InitializeDifferentBitrates) {
    EXPECT_TRUE(codec->initialize(48000, 1, 64000));
    codec->shutdown();

    EXPECT_TRUE(codec->initialize(48000, 1, 128000));
    codec->shutdown();

    EXPECT_TRUE(codec->initialize(48000, 1, 256000));
}

// 测试编码解码往返
TEST_F(OpusCodecTest, EncodeDecodeRoundtrip) {
    ASSERT_TRUE(codec->initialize(48000, 1, 64000));

    auto input = generate_sine_wave(960); // 20ms @ 48kHz
    auto encoded = codec->encode(input.data(), 960);

    EXPECT_FALSE(encoded.empty());
    EXPECT_LE(encoded.size(), 1500); // 合理的编码大小

    auto decoded = codec->decode(encoded.data(), encoded.size());
    EXPECT_EQ(decoded.size(), 960);

    // 解码后的数据应该与输入相似（有编解码误差）
    float max_diff = 0.0f;
    for (size_t i = 0; i < 960; ++i) {
        max_diff = std::max(max_diff, std::abs(input[i] - decoded[i]));
    }
    EXPECT_LT(max_diff, 0.5f); // 编解码误差应该小于 0.5
}

// 测试立体声编码解码
TEST_F(OpusCodecTest, StereoEncodeDecode) {
    ASSERT_TRUE(codec->initialize(48000, 2, 128000));

    // 立体声数据：左右声道交替
    std::vector<float> stereo_input(960 * 2);
    for (uint32_t i = 0; i < 960; ++i) {
        stereo_input[i * 2] = 0.5f * std::sin(2.0f * 3.14159265f * 440.0f * i / 48000.0f);     // 左声道
        stereo_input[i * 2 + 1] = 0.3f * std::sin(2.0f * 3.14159265f * 880.0f * i / 48000.0f); // 右声道
    }

    auto encoded = codec->encode(stereo_input.data(), 960);
    EXPECT_FALSE(encoded.empty());

    auto decoded = codec->decode(encoded.data(), encoded.size());
    EXPECT_EQ(decoded.size(), 960 * 2);
}

// 测试动态码率调整
TEST_F(OpusCodecTest, SetBitrate) {
    ASSERT_TRUE(codec->initialize(48000, 1, 64000));

    EXPECT_TRUE(codec->set_bitrate(128000));
    EXPECT_TRUE(codec->set_bitrate(256000));
    EXPECT_TRUE(codec->set_bitrate(64000));
}

// 测试复杂度调整
TEST_F(OpusCodecTest, SetComplexity) {
    ASSERT_TRUE(codec->initialize(48000, 1, 64000));

    for (int c = 0; c <= 10; ++c) {
        EXPECT_TRUE(codec->set_complexity(c));
    }
}

// 测试编码静音
TEST_F(OpusCodecTest, EncodeSilence) {
    ASSERT_TRUE(codec->initialize(48000, 1, 64000));

    std::vector<float> silence(960, 0.0f);
    auto encoded = codec->encode(silence.data(), 960);
    EXPECT_FALSE(encoded.empty());

    auto decoded = codec->decode(encoded.data(), encoded.size());
    EXPECT_EQ(decoded.size(), 960);

    // 解码后的静音应该接近 0
    for (auto sample : decoded) {
        EXPECT_LT(std::abs(sample), 0.01f);
    }
}

// 测试重复编码稳定性
TEST_F(OpusCodecTest, RepeatedEncodeStability) {
    ASSERT_TRUE(codec->initialize(48000, 1, 64000));

    auto input = generate_sine_wave(960);

    // 连续编码 100 帧
    for (int i = 0; i < 100; ++i) {
        auto encoded = codec->encode(input.data(), 960);
        EXPECT_FALSE(encoded.empty());
    }
}

} // namespace tests
} // namespace soundbridge
