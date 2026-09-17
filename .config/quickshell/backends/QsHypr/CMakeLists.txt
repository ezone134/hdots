cmake_minimum_required(VERSION 3.16)
project(QsHypr LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_AUTOMOC ON)

find_package(Qt6 6.2 REQUIRED COMPONENTS Core Network Qml Quick)

qt_add_library(QsHypr SHARED
	qsHypr.hpp
	qsHypr.cpp
)

target_link_libraries(QsHypr PRIVATE Qt6::Core Qt6::Network Qt6::Qml Qt6::Quick)

set_target_properties(QsHypr PROPERTIES
	LIBRARY_OUTPUT_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}
)
