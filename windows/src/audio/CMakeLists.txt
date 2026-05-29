add_library(SoundBridgeAudio OBJECT
    wasapi_capture.cpp
    wasapi_renderer.cpp
    opus_codec.cpp
    webrtc_apm.cpp
)

target_include_directories(SoundBridgeAudio PUBLIC
    ${CMAKE_CURRENT_SOURCE_DIR}
    ${CMAKE_CURRENT_SOURCE_DIR}/../core
    ${CMAKE_CURRENT_SOURCE_DIR}/../../include
)

target_link_libraries(SoundBridgeAudio PUBLIC
    Opus::Opus
    WebRTC::APM
    mfplat
    mfuuid
    mmdevapi
    avrt
)
