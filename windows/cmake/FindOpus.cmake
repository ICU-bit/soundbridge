include(FindPackageHandleStandardArgs)

find_path(OPUS_INCLUDE_DIR
    NAMES opus/opus.h
    PATHS
        ${OPUS_ROOT}/include
        $ENV{OPUS_ROOT}/include
        ${CMAKE_CURRENT_SOURCE_DIR}/third_party/opus/include
)

find_library(OPUS_LIBRARY
    NAMES opus
    PATHS
        ${OPUS_ROOT}/lib
        ${OPUS_ROOT}/lib/x64
        $ENV{OPUS_ROOT}/lib
        ${CMAKE_CURRENT_SOURCE_DIR}/third_party/opus/lib/x64
)

find_package_handle_standard_args(Opus
    REQUIRED_VARS OPUS_LIBRARY OPUS_INCLUDE_DIR
)

if(Opus_FOUND AND NOT TARGET Opus::Opus)
    add_library(Opus::Opus UNKNOWN IMPORTED)
    set_target_properties(Opus::Opus PROPERTIES
        IMPORTED_LOCATION "${OPUS_LIBRARY}"
        INTERFACE_INCLUDE_DIRECTORIES "${OPUS_INCLUDE_DIR}"
    )
endif()

mark_as_advanced(OPUS_INCLUDE_DIR OPUS_LIBRARY)
