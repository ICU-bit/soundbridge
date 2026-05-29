#pragma once

#include <soundbridge/types.h>

#include <cstdint>
#include <cstddef>
#include <functional>

namespace soundbridge {

class ITransport {
public:
    virtual ~ITransport() = default;

    virtual bool connect(const NetworkEndpoint& endpoint) = 0;
    virtual void disconnect() = 0;

    virtual bool send(const uint8_t* data, size_t size) = 0;
    virtual bool receive(uint8_t* buffer, size_t buffer_size, size_t& received) = 0;

    virtual bool is_connected() const = 0;
    virtual TransportType type() const = 0;
};

} // namespace soundbridge
