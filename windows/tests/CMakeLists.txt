find_package(GTest QUIET)

if(GTest_FOUND)
    add_executable(SoundBridgeTests
        test_audio_pipeline.cpp
        test_opus_codec.cpp
        test_udp_transport.cpp
    )

    target_link_libraries(SoundBridgeTests PRIVATE
        SoundBridge
        GTest::gtest
        GTest::gtest_main
    )

    target_include_directories(SoundBridgeTests PRIVATE
        ${CMAKE_CURRENT_SOURCE_DIR}/../src/core
        ${CMAKE_CURRENT_SOURCE_DIR}/../src/audio
        ${CMAKE_CURRENT_SOURCE_DIR}/../src/network
        ${CMAKE_CURRENT_SOURCE_DIR}/../include
    )

    include(GoogleTest)
    gtest_discover_tests(SoundBridgeTests)
endif()
