cmake_minimum_required(VERSION 3.22.1)

project("soundbridge_native" LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

find_package(opus CONFIG)
if(NOT opus_FOUND)
    message(WARNING "opus not found - audio codec will not be available")
endif()

set(SOURCES
    jni_bridge.cpp
    audio_engine.cpp
    udp_socket.cpp
    audio_processor.cpp
)

if(opus_FOUND)
    list(APPEND SOURCES opus_codec.cpp)
endif()

add_library(soundbridge_native SHARED ${SOURCES})

target_include_directories(soundbridge_native PRIVATE
    ${CMAKE_CURRENT_SOURCE_DIR}
    ${CMAKE_CURRENT_SOURCE_DIR}/include
)

target_link_libraries(soundbridge_native
    android
    log
    OpenSLES
)

if(opus_FOUND)
    target_link_libraries(soundbridge_native opus)
    target_compile_definitions(soundbridge_native PRIVATE SOUNDBRIDGE_HAS_OPUS=1)
endif()

if(ANDROID)
    target_compile_definitions(soundbridge_native PRIVATE
        ANDROID=1
        __ANDROID_API__=26
    )
endif()

set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -Wall -Wextra -O2")
