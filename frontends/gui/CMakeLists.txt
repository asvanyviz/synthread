cmake_minimum_required(VERSION 3.20)
project(synthread-gui VERSION 0.1.0 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_AUTOMOC ON)

# Find Qt 6 packages
find_package(Qt6 REQUIRED COMPONENTS
    Core
    Quick
    Widgets
)

# Kirigami — KDE framework for convergent UIs
find_package(KF6Kirigami QUIET)

# Rust ↔ C++ FFI bridge
# The Rust core library is compiled as a cdylib and linked here.
# See rust-bridge/ for FFI glue code.

add_subdirectory(rust-bridge)

# Main GUI executable
add_executable(synthread-gui
    src/main.cpp
    src/app.cpp
)

target_link_libraries(synthread-gui PRIVATE
    Qt6::Core
    Qt6::Quick
    Qt6::Widgets
    synthread-bridge
)

if(KF6Kirigami_FOUND)
    target_link_libraries(synthread-gui PRIVATE KF6::Kirigami)
    target_compile_definitions(synthread-gui PRIVATE HAS_KIRIGAMI)
    message(STATUS "Kirigami found — KDE Plasma integration enabled")
else()
    message(WARNING "Kirigami not found — building without KDE integration")
endif()

# Install
install(TARGETS synthread-gui DESTINATION bin)
