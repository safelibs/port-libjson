cmake_minimum_required(VERSION 3.9)

project(json_c_package_cmake_smoke LANGUAGES C)

find_package(json-c CONFIG REQUIRED)

add_executable(cmake_smoke main.c)
target_link_libraries(cmake_smoke PRIVATE json-c::json-c)
