#pragma once

#ifdef SOUNDBRIDGE_HAS_SPDLOG
#include <spdlog/spdlog.h>
#else
// Fallback: provide both macro and namespace styles for logging
// When spdlog is not available, all calls become no-ops

// Macro style: spdlog_warn("msg")
#define spdlog_trace(...) ((void)0)
#define spdlog_debug(...) ((void)0)
#define spdlog_info(...) ((void)0)
#define spdlog_warn(...) ((void)0)
#define spdlog_error(...) ((void)0)
#define spdlog_critical(...) ((void)0)

// Namespace style: spdlog::warn("msg") — for source compatibility
namespace spdlog {
    template<typename... Args>
    inline void trace(Args&&...) {}
    template<typename... Args>
    inline void debug(Args&&...) {}
    template<typename... Args>
    inline void info(Args&&...) {}
    template<typename... Args>
    inline void warn(Args&&...) {}
    template<typename... Args>
    inline void error(Args&&...) {}
    template<typename... Args>
    inline void critical(Args&&...) {}
} // namespace spdlog

#endif
