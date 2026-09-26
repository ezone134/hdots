cmake_minimum_required(VERSION 3.16)
project(QsIo LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_AUTOMOC ON)

find_package(Qt6 6.2 REQUIRED COMPONENTS Core Quick)

qt_add_library(QsIo SHARED
	qsio.hpp
	qsio.cpp
)

target_link_libraries(QsIo PRIVATE Qt6::Core Qt6::Quick)

set_target_properties(QsIo PROPERTIES
	LIBRARY_OUTPUT_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}
)
