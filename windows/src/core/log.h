#pragma once

#ifdef SOUNDBRIDGE_HAS_SPDLOG
#include <spdlog/spdlog.h>
#else
// Fallback logging macros when spdlog is not available
#define spdlog_trace(...) ((void)0)
#define spdlog_debug(...) ((void)0)
#define spdlog_info(...) ((void)0)
#define spdlog_warn(...) ((void)0)
#define spdlog_error(...) ((void)0)
#define spdlog_critical(...) ((void)0)
#endif
