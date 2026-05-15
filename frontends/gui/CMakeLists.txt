cmake_minimum_required(VERSION 3.20)
project(synthread-gui VERSION 0.1.0 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_AUTOMOC ON)
set(CMAKE_AUTORCC ON)

# Qt 6 packages — Qt Quick for the UI
find_package(Qt6 REQUIRED COMPONENTS
    Core
    Quick
    QuickControls2
    Widgets
    Network
)

# Optional: Kirigami for KDE Plasma integration
find_package(KF6Kirigami QUIET)

# Rust ↔ C++ FFI bridge (stub — to be wired in Phase 3 completion)
add_subdirectory(rust-bridge)

# GUI sources
set(GUI_SOURCES
    src/main.cpp
    src/app.cpp
    src/app.h
    src/resources.qrc
)

# Main GUI executable
qt_add_executable(synthread-gui ${GUI_SOURCES})

target_link_libraries(synthread-gui PRIVATE
    Qt6::Core
    Qt6::Quick
    Qt6::QuickControls2
    Qt6::Widgets
    Qt6::Network
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
