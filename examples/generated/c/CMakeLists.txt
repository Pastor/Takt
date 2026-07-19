cmake_minimum_required(VERSION 3.10)
project(generated C)

set(CMAKE_C_STANDARD 99)
set(CMAKE_C_STANDARD_REQUIRED ON)

# Elevator — пример конечного автомата лифта
add_executable(elevator elevator_main.c elevator.c)
target_link_libraries(elevator m)

# ExtendComplex — пример с составными состояниями
add_executable(extend_complex extend_complex_main.c extend_complex.c)
target_link_libraries(extend_complex m)

# ElevatorMini — полный 9-этажный лифт с эмулятором датчиков
if(EXISTS "${CMAKE_CURRENT_SOURCE_DIR}/elevator_mini.c")
    add_executable(elevator_mini elevator_mini_main.c elevator_mini.c)
    target_link_libraries(elevator_mini m)
endif()

# Comprehensive — всесторонний пример (генерируется скриптом generate_examples.sh)
if(EXISTS "${CMAKE_CURRENT_SOURCE_DIR}/comprehensive.c")
    add_executable(comprehensive comprehensive_main.c comprehensive.c)
    target_link_libraries(comprehensive m)
endif()

# Stacker — симулятор штабелятора с позитивными и негативными сценариями
if(EXISTS "${CMAKE_CURRENT_SOURCE_DIR}/stacker.c")
    add_executable(stacker stacker_main.c stacker.c)
    target_link_libraries(stacker m)
endif()

# Regulator — пропорциональный регулятор на fixed-point q(8, 8) (фича 0061)
if(EXISTS "${CMAKE_CURRENT_SOURCE_DIR}/regulator.c")
    add_executable(regulator regulator_main.c regulator.c)
    target_link_libraries(regulator m)
endif()
