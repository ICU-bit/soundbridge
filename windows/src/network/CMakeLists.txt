add_library(SoundBridgeNetwork OBJECT
    udp_transport.cpp
    quic_transport.cpp
    srtp_context.cpp
    dtls_session.cpp
)

target_include_directories(SoundBridgeNetwork PUBLIC
    ${CMAKE_CURRENT_SOURCE_DIR}
    ${CMAKE_CURRENT_SOURCE_DIR}/../core
    ${CMAKE_CURRENT_SOURCE_DIR}/../../include
)

target_link_libraries(SoundBridgeNetwork PUBLIC
    ws2_32
    bcrypt
)
