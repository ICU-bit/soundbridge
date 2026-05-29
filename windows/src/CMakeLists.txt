add_subdirectory(core)
add_subdirectory(audio)
add_subdirectory(network)

add_library(SoundBridge SHARED
    $<TARGET_OBJECTS:SoundBridgeCore>
    $<TARGET_OBJECTS:SoundBridgeAudio>
    $<TARGET_OBJECTS:SoundBridgeNetwork>
)

target_link_libraries(SoundBridge PUBLIC
    SoundBridgeCore
    SoundBridgeAudio
    SoundBridgeNetwork
    Opus::Opus
    WebRTC::APM
    ws2_32
    mfplat
    mfuuid
    mfreadwrite
    mmdevapi
    avrt
)

target_include_directories(SoundBridge PUBLIC
    ${CMAKE_CURRENT_SOURCE_DIR}/../include
)

target_compile_definitions(SoundBridge PRIVATE
    SOUNDBRIDGE_EXPORTS
)

set_target_properties(SoundBridge PROPERTIES
    OUTPUT_NAME "SoundBridge"
    PDB_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR}/pdb
    MSVC_RUNTIME_LIBRARY "MultiThreaded$<$<CONFIG:Debug>:Debug>DLL"
)
