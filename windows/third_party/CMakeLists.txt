option(SOUNDBRIDGE_USE_SYSTEM_OPUS "Use system-installed Opus" ON)
option(SOUNDBRIDGE_USE_SYSTEM_WEBRTC "Use system-installed WebRTC" ON)

if(NOT SOUNDBRIDGE_USE_SYSTEM_OPUS)
    add_subdirectory(opus)
endif()

if(NOT SOUNDBRIDGE_USE_SYSTEM_WEBRTC)
    add_subdirectory(webrtc)
endif()
