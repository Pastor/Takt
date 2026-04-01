cmake_minimum_required(VERSION 4.3)
project(_output C)

set(CMAKE_C_STANDARD 90)

add_executable(elevator elevator_main.c elevator.c)
