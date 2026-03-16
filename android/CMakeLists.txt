cmake_minimum_required(VERSION 3.9.0)
project(RNDns)

set(PACKAGE_NAME RNDns)
set(CMAKE_VERBOSE_MAKEFILE ON)
set(CMAKE_CXX_STANDARD 20)

# Add our custom implementation and JNI adapter
add_library(${PACKAGE_NAME} SHARED
        ../cpp/HybridDns.cpp
        OnLoad.cpp
)

# Include paths for our headers
include_directories(
    ${CMAKE_CURRENT_SOURCE_DIR}/../cpp  # HybridDns.hpp and dns_ffi.h
)

# Include the Nitrogen-generated CMake file
include(${CMAKE_CURRENT_SOURCE_DIR}/../nitrogen/generated/android/RNDns+autolinking.cmake)

# Link the Rust DNS library
target_link_directories(RNDns PRIVATE ${CMAKE_CURRENT_SOURCE_DIR}/libs/${ANDROID_ABI})
target_link_libraries(RNDns 
    rust_c_dns
    log # For android logging
)

# Android 15 16KB page size alignment
if(ANDROID_ABI STREQUAL "arm64-v8a" OR ANDROID_ABI STREQUAL "x86_64")
    set(CMAKE_SHARED_LINKER_FLAGS "${CMAKE_SHARED_LINKER_FLAGS} -Wl,-z,max-page-size=16384")
endif()
