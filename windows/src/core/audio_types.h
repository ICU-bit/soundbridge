#pragma once

#include <soundbridge/types.h>

#include <mutex>
#include <atomic>
#include <queue>
#include <span>

namespace soundbridge {

class AudioBuffer {
public:
    AudioBuffer() = default;
    AudioBuffer(uint32_t capacity_frames, uint8_t channels);

    bool write(const float* data, uint32_t frame_count);
    bool read(float* data, uint32_t frame_count);
    void clear();

    uint32_t available_frames() const;
    uint32_t capacity() const { return capacity_; }
    bool is_empty() const { return available_frames() == 0; }
    bool is_full() const { return available_frames() >= capacity_; }

private:
    std::vector<float> data_;
    uint32_t capacity_ = 0;
    uint32_t write_pos_ = 0;
    uint32_t read_pos_ = 0;
    uint32_t available_ = 0;
    uint8_t channels_ = 0;
    mutable std::mutex mutex_;
};

class AudioRingBuffer {
public:
    explicit AudioRingBuffer(size_t buffer_size);

    size_t write(const float* data, size_t sample_count);
    size_t read(float* data, size_t sample_count);
    void reset();

    size_t available_read() const;
    size_t available_write() const;

private:
    std::vector<float> buffer_;
    size_t head_ = 0;
    size_t tail_ = 0;
    size_t size_;
    mutable std::mutex mutex_;
};

struct AudioPacketHeader {
    uint32_t magic = 0x53424447;
    uint16_t version = 1;
    uint16_t sequence = 0;
    uint32_t timestamp = 0;
    uint32_t payload_size = 0;
    uint8_t channels = 0;
    uint32_t sample_rate = 0;
    uint16_t frame_size = 0;
};

} // namespace soundbridge
