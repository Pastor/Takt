#include "elevator.h"
#include <assert.h>
#include <math.h>
/// Константы и порты модели elevator (Elevator)
#define PORT_ELEVATOR_BTNS_CAB_HI 0x30000001
#define PORT_ELEVATOR_BTNS_CAB_LO 0x30000000
#define PORT_ELEVATOR_BTNS_FLOOR_1 0x20000000
#define PORT_ELEVATOR_BTNS_FLOOR_2 0x20000001
#define PORT_ELEVATOR_BTNS_FLOOR_3 0x20000002
#define PORT_ELEVATOR_BTNS_FLOOR_4 0x20000003
#define PORT_ELEVATOR_BTNS_FLOOR_5 0x20000004
#define PORT_ELEVATOR_BTNS_FLOOR_6 0x20000005
#define PORT_ELEVATOR_BTNS_FLOOR_7 0x20000006
#define PORT_ELEVATOR_BTNS_FLOOR_8 0x20000007
#define PORT_ELEVATOR_BTNS_FLOOR_9 0x20000008
#define PORT_ELEVATOR_SENSORS_1 0x10000000
#define PORT_ELEVATOR_SENSORS_2 0x10000001
#define PORT_ELEVATOR_SENSORS_3 0x10000002
#define PORT_ELEVATOR_SENSORS_4 0x10000003
#define PORT_ELEVATOR_SENSORS_5 0x10000004
#define PORT_ELEVATOR_SENSORS_6 0x10000005
#define PORT_ELEVATOR_SENSORS_7 0x10000006
#define PORT_ELEVATOR_SENSORS_8 0x10000007
#define PORT_ELEVATOR_SENSORS_9 0x10000008
#define PORT_ELEVATOR_SENSORS_CAB 0x10000009
/// Перечисления модели elevator (Elevator)
#define ENUM_ELEVATOR_BOTTOM 80
#define ENUM_ELEVATOR_TOP 81
/// Перечисления модели Engine (Elevator:Engine)
#define ENUM_ELEVATOR_ENGINE_CLOSING 671
#define ENUM_ELEVATOR_ENGINE_IDLE 670
/// Model functions 'Engine (Elevator:Engine)'
static void ElevatorEngine_init(ElevatorEngine *model, Elevator *main);
static void ElevatorEngine_tick(ElevatorEngine *model, Elevator *main);
static bool ElevatorEngine_is_done(const ElevatorEngine *model, Elevator *main);

///Внешние функции
extern void door_close();
extern void door_open();
extern void motor_down();
extern void motor_stop();
extern void motor_up();
extern void read_floor_sensors();
extern void scan_cabin_buttons();
extern void scan_floor_buttons();
/// Функция инициализации модели Engine (Elevator:Engine)
void ElevatorEngine_init(ElevatorEngine *model, Elevator *main) {
    assert(0 != model);
    model->state = ELEVATOR_ENGINE_INIT;
    model->action = 671;
}

/// Функция обработки модели Engine (Elevator:Engine)
void ElevatorEngine_tick(ElevatorEngine *model, Elevator *main) {
    assert(0 != model);
    assert(0 != main);
    switch (model->state) {
        case ELEVATOR_ENGINE_INIT: {
            model->state = ELEVATOR_ENGINE_IDLE;
            break;
        }
        case ELEVATOR_ENGINE_MOVING_UP: {
            if ((*main->read_bit)(PORT_ELEVATOR_SENSORS_CAB, 0, main->userdata)) {
            }
            if (main->current_floor == main->target_floor) {
                main->has_call = 0;
                model->state = ELEVATOR_ENGINE_DOOR_OPENING;
                break;
            }
            break;
        }
        case ELEVATOR_ENGINE_DOOR_CLOSING: {
            if ((*main->read_bit)(PORT_ELEVATOR_SENSORS_CAB, 1, main->userdata) || main->current_floor == main->target_floor) {
                model->state = ELEVATOR_ENGINE_IDLE;
                break;
            }
            if (!((*main->read_bit)(PORT_ELEVATOR_SENSORS_CAB, 1, main->userdata)) && main->current_floor < main->target_floor) {
                model->state = ELEVATOR_ENGINE_MOVING_UP;
                break;
            }
            if (!((*main->read_bit)(PORT_ELEVATOR_SENSORS_CAB, 1, main->userdata)) && main->current_floor > main->target_floor) {
                model->state = ELEVATOR_ENGINE_MOVING_DOWN;
                break;
            }
            break;
        }
        case ELEVATOR_ENGINE_IDLE: {
            if ((*main->read_bit)(PORT_ELEVATOR_SENSORS_CAB, 0, main->userdata)) {
            }
            if (main->has_call == 1 && !(main->current_floor == main->target_floor)) {
                model->state = ELEVATOR_ENGINE_DOOR_CLOSING;
                break;
            }
            break;
        }
        case ELEVATOR_ENGINE_DOOR_OPENING: {
            model->state = ELEVATOR_ENGINE_END;
            break;
        }
        case ELEVATOR_ENGINE_MOVING_DOWN: {
            if ((*main->read_bit)(PORT_ELEVATOR_SENSORS_CAB, 0, main->userdata)) {
            }
            if (main->current_floor == main->target_floor) {
                main->has_call = 0;
                model->state = ELEVATOR_ENGINE_DOOR_OPENING;
                break;
            }
            break;
        }
        case ELEVATOR_ENGINE_END: {
            break;
        }
    }
}

/// Функция сброса модели Engine (Elevator:Engine)
void ElevatorEngine_reset(ElevatorEngine *model, Elevator *main) {
    ElevatorEngine_init(model, main);
}

/// Функция проверки терминального состояния модели Engine (Elevator:Engine)
bool ElevatorEngine_is_done(const ElevatorEngine *model, Elevator *main) {
    return model->state == ELEVATOR_ENGINE_END;
}

/// Функция инициализации модели elevator (Elevator)
void Elevator_init(Elevator *model) {
    assert(0 != model);
    model->state = ELEVATOR_INIT;
    model->current_floor = 1;
    model->target_floor = 1;
    model->has_call = 0;
}

/// Функция обработки модели elevator (Elevator)
void Elevator_tick(Elevator *model) {
    assert(0 != model);
    switch (model->state) {
        case ELEVATOR_INIT: {
            ElevatorEngine_init(&model->main, model);
            model->state = ELEVATOR_MAIN;
            break;
        }
        case ELEVATOR_MAIN: {
            ElevatorEngine_tick(&model->main, model);
            if (ElevatorEngine_is_done(&model->main, model)) {
                model->state = ELEVATOR_MIDDLE;
                break;
            }
            break;
        }
        case ELEVATOR_MIDDLE: {
            if (model->middle_state == ELEVATOR_MIDDLE_ENGINE0) {
                ElevatorEngine_tick(&model->middle_engine0, model);
                if (ElevatorEngine_is_done(&model->middle_engine0, model)) {
                    ElevatorEngine_init(&model->middle_parallel1.engine0, model);
                    ElevatorEngine_init(&model->middle_parallel1.engine1, model);
                    model->middle_parallel1.state = ELEVATOR_MIDDLE_PARALLEL1_INIT;
                    model->middle_state = ELEVATOR_MIDDLE_PARALLEL1;
                    break;
                }
            } else if (model->middle_state == ELEVATOR_MIDDLE_PARALLEL1) {
                ElevatorEngine_tick(&model->middle_parallel1.engine0, model);
                ElevatorEngine_tick(&model->middle_parallel1.engine1, model);
                if (ElevatorEngine_is_done(&model->middle_parallel1.engine0, model) && ElevatorEngine_is_done(&model->middle_parallel1.engine1, model)) {
                    ElevatorEngine_init(&model->middle_engine2, model);
                    model->middle_state = ELEVATOR_MIDDLE_ENGINE2;
                    break;
                }
            } else if (model->middle_state == ELEVATOR_MIDDLE_ENGINE2) {
                ElevatorEngine_tick(&model->middle_engine2, model);
                if (ElevatorEngine_is_done(&model->middle_engine2, model)) {
                    ElevatorEngine_init(&model->middle_engine3, model);
                    model->middle_state = ELEVATOR_MIDDLE_ENGINE3;
                    break;
                }
            } else if (model->middle_state == ELEVATOR_MIDDLE_ENGINE3) {
                ElevatorEngine_tick(&model->middle_engine3, model);
                if (ElevatorEngine_is_done(&model->middle_engine3, model)) {
                    ElevatorEngine_init(&model->middle_engine4, model);
                    model->middle_state = ELEVATOR_MIDDLE_ENGINE4;
                    break;
                }
            } else if (model->middle_state == ELEVATOR_MIDDLE_ENGINE4) {
                ElevatorEngine_tick(&model->middle_engine4, model);
                if (ElevatorEngine_is_done(&model->middle_engine4, model)) {
                    model->state = ELEVATOR_END;
                    break;
                }
            }
            break;
        }
        case ELEVATOR_END: {
            break;
        }
    }
}

/// Функция сброса модели elevator (Elevator)
void Elevator_reset(Elevator *model) {
    Elevator_init(model);
}

/// Функция проверки терминального состояния модели elevator (Elevator)
bool Elevator_is_done(const Elevator *model) {
    return model->state == ELEVATOR_END;
}

