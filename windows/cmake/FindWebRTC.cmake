include(FindPackageHandleStandardArgs)

find_path(WEBRTC_INCLUDE_DIR
    NAMES api/audio/audio_processing.h
    PATHS
        ${WEBRTC_ROOT}/include
        $ENV{WEBRTC_ROOT}/include
        ${CMAKE_CURRENT_SOURCE_DIR}/third_party/webrtc/include
    PATH_SUFFIXES
        webrtc
)

find_library(WEBRTC_APM_LIBRARY
    NAMES webrtc_apm webrtc
    PATHS
        ${WEBRTC_ROOT}/lib
        ${WEBRTC_ROOT}/lib/x64
        $ENV{WEBRTC_ROOT}/lib
        ${CMAKE_CURRENT_SOURCE_DIR}/third_party/webrtc/lib/x64
)

find_package_handle_standard_args(WebRTC
    REQUIRED_VARS WEBRTC_APM_LIBRARY WEBRTC_INCLUDE_DIR
)

if(WebRTC_FOUND AND NOT TARGET WebRTC::APM)
    add_library(WebRTC::APM UNKNOWN IMPORTED)
    set_target_properties(WebRTC::APM PROPERTIES
        IMPORTED_LOCATION "${WEBRTC_APM_LIBRARY}"
        INTERFACE_INCLUDE_DIRECTORIES "${WEBRTC_INCLUDE_DIR}"
    )
endif()

mark_as_advanced(WEBRTC_INCLUDE_DIR WEBRTC_APM_LIBRARY)
