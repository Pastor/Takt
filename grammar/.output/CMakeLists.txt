cmake_minimum_required(VERSION 4.3)
project(_output C)

set(CMAKE_C_STANDARD 90)

add_executable(elevator elevator_main.c elevator.c)
add_executable(extend_complex extend_complex_main.c extend_complex.c)
