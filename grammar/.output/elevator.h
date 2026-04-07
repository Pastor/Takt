#ifndef ELEVATOR_H__
#define ELEVATOR_H__
#include <stdint.h>
#include <stdbool.h>

// NOTICE: Определение констант для модели elevator:Engine
/* Model Engine (Elevator:Engine) */
typedef struct ElevatorEngine {
    // NOTICE: Определение переменных модели
    uint16_t action;
    enum {
        ELEVATOR_ENGINE_INIT,
        ELEVATOR_ENGINE_MOVING_DOWN,
        ELEVATOR_ENGINE_MOVING_UP,
        ELEVATOR_ENGINE_DOOR_OPENING,
        ELEVATOR_ENGINE_IDLE,
        ELEVATOR_ENGINE_DOOR_CLOSING
    } state;
};

// NOTICE: Определение констант для модели elevator
/* Model elevator (Elevator) */
typedef struct Elevator {
    // NOTICE: Определение переменных модели
    uint8_t has_call;
    uint8_t target_floor;
    uint8_t current_floor;
    enum {
        ELEVATOR_INIT,
        ELEVATOR_END,
        ELEVATOR_MIDDLE,
        ELEVATOR_MAIN
    } state;
    // NOTICE: Определение extend
    ElevatorEngine middle_engine0;
    struct {
        ElevatorEngine engine0;
        ElevatorEngine engine1;
        enum {
            ELEVATOR_MIDDLE_PARALLEL1_INIT,
            ELEVATOR_MIDDLE_PARALLEL1_TICK,
            ELEVATOR_MIDDLE_PARALLEL1_END
        } state;
    } middle_parallel1;
    ElevatorEngine middle_engine2;
    ElevatorEngine middle_engine3;
    ElevatorEngine middle_engine4;
    enum {
        ELEVATOR_MIDDLE_INIT,
        ELEVATOR_MIDDLE_ENGINE0,
        ELEVATOR_MIDDLE_PARALLEL1,
        ELEVATOR_MIDDLE_ENGINE2,
        ELEVATOR_MIDDLE_ENGINE3,
        ELEVATOR_MIDDLE_ENGINE4
    } middle_state;
    ElevatorEngine main;
    /// NOTICE: Функции портов ввода вывода
    void  *userdata;
    void  (*write_bit  )(int address, int bit, bool val, void *userdata);
    bool  (*read_bit   )(int address, int bit, void *userdata);
    void  (*write_float)(int address, int bit, float val, void *userdata);
    float (*read_float )(int address, int bit, void *userdata);
};

void Elevator_init(Elevator *main);
void Elevator_tick(Elevator *main);
void Elevator_reset(Elevator *main);
bool Elevator_is_done(const Elevator *main);
#endif
