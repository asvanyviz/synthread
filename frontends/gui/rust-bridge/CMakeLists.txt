# Rust → C++ FFI Bridge
#
# Compiles the Rust core as a C dynamic library,
# generates C headers via cbindgen, and links into
# the Qt 6 / Kirigami frontend.

cmake_minimum_required(VERSION 3.20)
project(synthread-bridge LANGUAGES C)

# Rust build as cdylib
# In Phase 3: use cargo to build a cdylib target
# add_custom_target(synthread-core-lib
#     COMMAND cargo build --release --manifest-path ${CMAKE_SOURCE_DIR}/../../core/Cargo.toml
#     COMMENT "Building synthread-core cdylib"
# )

# add_library(synthread-bridge STATIC src/bridge.c)
# target_include_directories(synthread-bridge PUBLIC ${CMAKE_CURRENT_BINARY_DIR}/include)
message(STATUS "Rust bridge stub — to be implemented in Phase 3")
