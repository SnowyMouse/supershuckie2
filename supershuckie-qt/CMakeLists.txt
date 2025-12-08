cmake_minimum_required(VERSION 4.0)

project("SuperShuckie"
    VERSION 1.0
    LANGUAGES CXX
)

include(corrosion.cmake)

corrosion_import_crate(MANIFEST_PATH "${CMAKE_SOURCE_DIR}/../supershuckie-core-c/Cargo.toml")

set(CMAKE_AUTOMOC ON)

if(${WIN32})
    set(EXE_TYPE "WIN32")
endif()

add_executable(supershuckie ${EXE_TYPE}
    src/main.cpp
    src/main_window.cpp
    src/render_widget.cpp
)

set_property(TARGET supershuckie PROPERTY CXX_STANDARD 20)

find_package(SDL3 REQUIRED CONFIG REQUIRED COMPONENTS SDL3)
find_package(Qt6 REQUIRED COMPONENTS Core Widgets)

target_include_directories(supershuckie
    PRIVATE "${CMAKE_SOURCE_DIR}/../supershuckie-core-c/include"
)

target_link_libraries(supershuckie
    PRIVATE Qt6::Widgets
    PRIVATE Qt6::Core
    PRIVATE SDL3::SDL3
    PUBLIC supershuckie_core_c
)