#pragma once

#if defined(_WIN32) || defined(_WIN64)
    #ifdef SOUNDBRIDGE_EXPORTS
        #define SOUNDBRIDGE_API __declspec(dllexport)
    #else
        #define SOUNDBRIDGE_API __declspec(dllimport)
    #endif
#else
    #define SOUNDBRIDGE_API __attribute__((visibility("default")))
#endif

#define SOUNDBRIDGE_CALL __cdecl
