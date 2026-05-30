cmake_minimum_required(VERSION 3.22.1)

project("soundbridge_native" LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# ── Option: link against Rust FFI static library ──
# When ON, the JNI bridge delegates engine lifecycle to Rust sb_* functions.
# When OFF (default), uses the local C++ AudioEngine implementation.
option(SOUNDBRIDGE_USE_RUST_FFI "Link against Rust FFI static library (libsoundbridge_bindings)" OFF)

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

# ── Rust FFI integration ──
if(SOUNDBRIDGE_USE_RUST_FFI)
    # Path to the pre-built Rust static library.
    # Build with: cargo ndk -t arm64-v8a --manifest-path ../../rust-core/Cargo.toml -p ffi-bindings --release
    # Then set SOUNDBRIDGE_RUST_FFI_LIB_DIR to the target directory.
    if(NOT DEFINED SOUNDBRIDGE_RUST_FFI_LIB_DIR)
        set(SOUNDBRIDGE_RUST_FFI_LIB_DIR
            "${CMAKE_CURRENT_SOURCE_DIR}/../../../../rust-core/target/aarch64-linux-android/release"
            CACHE PATH "Directory containing libsoundbridge_bindings.a")
    endif()

    if(NOT DEFINED SOUNDBRIDGE_RUST_FFI_INCLUDE_DIR)
        set(SOUNDBRIDGE_RUST_FFI_INCLUDE_DIR
            "${CMAKE_CURRENT_SOURCE_DIR}/../../../../rust-core/crates/ffi-bindings/include"
            CACHE PATH "Directory containing soundbridge.h")
    endif()

    # Verify the static library exists
    set(SB_RUST_LIB "${SOUNDBRIDGE_RUST_FFI_LIB_DIR}/libsoundbridge_bindings.a")
    if(EXISTS "${SB_RUST_LIB}")
        message(STATUS "SoundBridge: Rust FFI library found at ${SB_RUST_LIB}")

        target_compile_definitions(soundbridge_native PRIVATE SOUNDBRIDGE_USE_RUST_FFI=1)

        target_include_directories(soundbridge_native PRIVATE
            ${SOUNDBRIDGE_RUST_FFI_INCLUDE_DIR}
        )

        target_link_libraries(soundbridge_native
            "${SB_RUST_LIB}"
            dl        # dlopen for Rust runtime
            m         # math library
            log       # Android logging (may already be linked)
        )
    else()
        message(WARNING
            "SoundBridge: SOUNDBRIDGE_USE_RUST_FFI is ON but libsoundbridge_bindings.a not found at ${SB_RUST_LIB}\n"
            "  Build with: cargo ndk -t arm64-v8a --manifest-path <workspace>/rust-core/Cargo.toml -p ffi-bindings --release\n"
            "  Or set SOUNDBRIDGE_RUST_FFI_LIB_DIR to the correct path.\n"
            "  Falling back to local C++ AudioEngine."
        )
    endif()
else()
    message(STATUS "SoundBridge: Using local C++ AudioEngine (Rust FFI disabled)")
endif()

if(ANDROID)
    target_compile_definitions(soundbridge_native PRIVATE
        ANDROID=1
        __ANDROID_API__=26
    )
endif()

set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -Wall -Wextra -O2")
