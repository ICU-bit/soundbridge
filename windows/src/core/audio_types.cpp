#include "audio_types.h"

#include <algorithm>
#include <cstring>

namespace soundbridge {

// ============================================================
// AudioBuffer — fixed-capacity frame-oriented ring buffer
// ============================================================

AudioBuffer::AudioBuffer(uint32_t capacity_frames, uint8_t channels)
    : data_(static_cast<size_t>(capacity_frames) * channels, 0.0f)
    , capacity_(capacity_frames)
    , write_pos_(0)
    , read_pos_(0)
    , available_(0)
    , channels_(channels)
{
}

bool AudioBuffer::write(const float* data, uint32_t frame_count) {
    std::lock_guard<std::mutex> lock(mutex_);

    if (available_ + frame_count > capacity_) {
        return false;
    }

    const size_t samples = static_cast<size_t>(frame_count) * channels_;
    const size_t write_idx = static_cast<size_t>(write_pos_) * channels_;

    for (size_t i = 0; i < samples; ++i) {
        data_[(write_idx + i) % data_.size()] = data[i];
    }

    write_pos_ = (write_pos_ + frame_count) % capacity_;
    available_ += frame_count;
    return true;
}

bool AudioBuffer::read(float* data, uint32_t frame_count) {
    std::lock_guard<std::mutex> lock(mutex_);

    if (available_ < frame_count) {
        return false;
    }

    const size_t samples = static_cast<size_t>(frame_count) * channels_;
    const size_t read_idx = static_cast<size_t>(read_pos_) * channels_;

    for (size_t i = 0; i < samples; ++i) {
        data[i] = data_[(read_idx + i) % data_.size()];
    }

    read_pos_ = (read_pos_ + frame_count) % capacity_;
    available_ -= frame_count;
    return true;
}

void AudioBuffer::clear() {
    std::lock_guard<std::mutex> lock(mutex_);
    write_pos_ = 0;
    read_pos_ = 0;
    available_ = 0;
}

uint32_t AudioBuffer::available_frames() const {
    std::lock_guard<std::mutex> lock(mutex_);
    return available_;
}

// ============================================================
// AudioRingBuffer — sample-oriented circular buffer
// ============================================================

AudioRingBuffer::AudioRingBuffer(size_t buffer_size)
    : buffer_(buffer_size, 0.0f)
    , head_(0)
    , tail_(0)
    , size_(buffer_size)
{
}

size_t AudioRingBuffer::write(const float* data, size_t sample_count) {
    std::lock_guard<std::mutex> lock(mutex_);

    const size_t free_space = size_ - available_read_internal();
    const size_t to_write = std::min(sample_count, free_space);

    for (size_t i = 0; i < to_write; ++i) {
        buffer_[tail_] = data[i];
        tail_ = (tail_ + 1) % size_;
    }
    return to_write;
}

size_t AudioRingBuffer::read(float* data, size_t sample_count) {
    std::lock_guard<std::mutex> lock(mutex_);

    const size_t avail = available_read_internal();
    const size_t to_read = std::min(sample_count, avail);

    for (size_t i = 0; i < to_read; ++i) {
        data[i] = buffer_[head_];
        head_ = (head_ + 1) % size_;
    }
    return to_read;
}

void AudioRingBuffer::reset() {
    std::lock_guard<std::mutex> lock(mutex_);
    head_ = 0;
    tail_ = 0;
    std::fill(buffer_.begin(), buffer_.end(), 0.0f);
}

size_t AudioRingBuffer::available_read() const {
    std::lock_guard<std::mutex> lock(mutex_);
    return available_read_internal();
}

size_t AudioRingBuffer::available_write() const {
    std::lock_guard<std::mutex> lock(mutex_);
    return size_ - available_read_internal();
}

size_t AudioRingBuffer::available_read_internal() const {
    if (tail_ >= head_) {
        return tail_ - head_;
    }
    return size_ - head_ + tail_;
}

} // namespace soundbridge
