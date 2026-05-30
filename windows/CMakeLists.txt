cmake_minimum_required(VERSION 3.20)

project(SoundBridge
    VERSION 1.0.0
    DESCRIPTION "Cross-platform audio media bridge"
    LANGUAGES CXX C
)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

set(CMAKE_C_STANDARD 17)
set(CMAKE_C_STANDARD_REQUIRED ON)

if(MSVC)
    add_compile_options(/utf-8 /W4 /WX /permissive-)
    add_compile_definitions(
        UNICODE
        _UNICODE
        WIN32_LEAN_AND_MEAN
        NOMINMAX
        _CRT_SECURE_NO_WARNINGS
    )
endif()

set(CMAKE_RUNTIME_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR}/bin)
set(CMAKE_LIBRARY_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR}/lib)
set(CMAKE_ARCHIVE_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR}/lib)

list(APPEND CMAKE_MODULE_PATH ${CMAKE_CURRENT_SOURCE_DIR}/cmake)

find_package(Opus REQUIRED)
find_package(WebRTC)
if(NOT WebRTC_FOUND)
    message(WARNING "WebRTC APM not found - building without AEC/NS/AGC support. "
        "Set WEBRTC_ROOT or place headers in third_party/webrtc/ to enable.")
endif()

add_subdirectory(src)
add_subdirectory(tests)
