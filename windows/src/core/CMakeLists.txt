add_library(SoundBridgeCore OBJECT
    audio_engine.cpp
    audio_pipeline.cpp
    session.cpp
)

target_include_directories(SoundBridgeCore PUBLIC
    ${CMAKE_CURRENT_SOURCE_DIR}
    ${CMAKE_CURRENT_SOURCE_DIR}/../audio
    ${CMAKE_CURRENT_SOURCE_DIR}/../network
    ${CMAKE_CURRENT_SOURCE_DIR}/../../include
)

target_link_libraries(SoundBridgeCore PUBLIC
    Opus::Opus
)
