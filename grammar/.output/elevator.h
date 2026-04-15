#ifndef ELEVATOR_H__
#define ELEVATOR_H__
#include <stdint.h>
#include <stdbool.h>

/* Forward declarations */
typedef struct ElevatorEngine ElevatorEngine;
typedef struct Elevator Elevator;

// NOTICE: Определение констант для модели Engine (Elevator:Engine)
/* Model Engine (Elevator:Engine) */
struct ElevatorEngine {
    // NOTICE: Определение переменных модели
    uint16_t action;
    enum {
        ELEVATOR_ENGINE_INIT,
        ELEVATOR_ENGINE_MOVING_DOWN,
        ELEVATOR_ENGINE_IDLE,
        ELEVATOR_ENGINE_DOOR_CLOSING,
        ELEVATOR_ENGINE_MOVING_UP,
        ELEVATOR_ENGINE_DOOR_OPENING,
        ELEVATOR_ENGINE_END
    } state;
};

// NOTICE: Определение констант для модели elevator (Elevator)
/* Model elevator (Elevator) */
struct Elevator {
    // NOTICE: Определение переменных модели
    uint8_t target_floor;
    uint8_t has_call;
    uint8_t current_floor;
    enum {
        ELEVATOR_INIT,
        ELEVATOR_MAIN,
        ELEVATOR_MIDDLE,
        ELEVATOR_END
    } state;
    // NOTICE: Определение extend
    ElevatorEngine main;
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
        ELEVATOR_MIDDLE_ENGINE4,
        ELEVATOR_MIDDLE_END
    } middle_state;
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
