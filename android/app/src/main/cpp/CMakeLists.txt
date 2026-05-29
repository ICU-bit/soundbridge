cmake_minimum_required(VERSION 3.22.1)

project("soundbridge_native" LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

find_package(opus REQUIRED CONFIG)

add_library(soundbridge_native SHARED
    jni_bridge.cpp
    audio_engine.cpp
    opus_codec.cpp
    udp_socket.cpp
    audio_processor.cpp
)

target_include_directories(soundbridge_native PRIVATE
    ${CMAKE_CURRENT_SOURCE_DIR}
    ${CMAKE_CURRENT_SOURCE_DIR}/include
)

target_link_libraries(soundbridge_native
    android
    log
    OpenSLES
    oboe::oboe
    opus
    ${log-lib}
)

if(ANDROID)
    target_compile_definitions(soundbridge_native PRIVATE
        ANDROID=1
        __ANDROID_API__=26
    )
endif()

set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -Wall -Wextra -O2")
