#include "stacker.h"
#include <assert.h>
#include <math.h>
/// Константы и порты модели stacker (Stacker)
#define CONST_STACKER_CHARGE_ROW 0
#define CONST_STACKER_CHARGE_SECTION 0
#define CONST_STACKER_CHARGE_STACK 0
#define CONST_STACKER_DROPOFF_ROW 1
#define CONST_STACKER_DROPOFF_SECTION 1
#define CONST_STACKER_DROPOFF_STACK 11
#define CONST_STACKER_PICKUP_ROW 1
#define CONST_STACKER_PICKUP_SECTION 1
#define CONST_STACKER_PICKUP_STACK 0
/// Model functions 'LiftController (Stacker:LiftController)'
static void StackerLiftController_init(StackerLiftController *model, Stacker *main);
static void StackerLiftController_tick(StackerLiftController *model, Stacker *main);
static bool StackerLiftController_is_done(const StackerLiftController *model, Stacker *main);
/// Model functions 'CommandReceiver (Stacker:CommandReceiver)'
static void StackerCommandReceiver_init(StackerCommandReceiver *model, Stacker *main);
static void StackerCommandReceiver_tick(StackerCommandReceiver *model, Stacker *main);
static bool StackerCommandReceiver_is_done(const StackerCommandReceiver *model, Stacker *main);
/// Model functions 'MovementController (Stacker:MovementController)'
static void StackerMovementController_init(StackerMovementController *model, Stacker *main);
static void StackerMovementController_tick(StackerMovementController *model, Stacker *main);
static bool StackerMovementController_is_done(const StackerMovementController *model, Stacker *main);

///Функции моделей
static uint8_t Stacker_travel_time(const Stacker *model, uint8_t to_stack, uint8_t to_row, uint8_t to_section) {
    uint8_t ds = 0;
    uint8_t dr = 0;
    uint8_t dy = 0;
    uint8_t t = 0;
    if ((*model->read_numeric)(STACKER_POS_STACK, model->userdata) > to_stack) {
        ds = (*model->read_numeric)(STACKER_POS_STACK, model->userdata) - to_stack;
    } else {
        ds = to_stack - (*model->read_numeric)(STACKER_POS_STACK, model->userdata);
    }
    if ((*model->read_numeric)(STACKER_POS_ROW, model->userdata) > to_row) {
        dr = (*model->read_numeric)(STACKER_POS_ROW, model->userdata) - to_row;
    } else {
        dr = to_row - (*model->read_numeric)(STACKER_POS_ROW, model->userdata);
    }
    if ((*model->read_numeric)(STACKER_POS_SECTION, model->userdata) > to_section) {
        dy = (*model->read_numeric)(STACKER_POS_SECTION, model->userdata) - to_section;
    } else {
        dy = to_section - (*model->read_numeric)(STACKER_POS_SECTION, model->userdata);
    }
    t = ds;
    if (dr > t) {
        t = dr;
    }
    if (dy > t) {
        t = dy;
    }
    return t;
}

/// Функция инициализации модели LiftController (Stacker:LiftController)
void StackerLiftController_init(StackerLiftController *model, Stacker *main) {
    assert(0 != model);
    model->state = STACKER_LIFT_CONTROLLER_INIT;
}

/// Функция обработки модели LiftController (Stacker:LiftController)
void StackerLiftController_tick(StackerLiftController *model, Stacker *main) {
    assert(0 != model);
    assert(0 != main);
    switch (model->state) {
        case STACKER_LIFT_CONTROLLER_INIT: {
            (*main->write_bit)(STACKER_CMD_FORK, 0, main->userdata);
            model->state = STACKER_LIFT_CONTROLLER_LIFT_IDLE;
            break;
        }
        case STACKER_LIFT_CONTROLLER_LIFT_OPERATING: {
            if (main->lift_request && !(main->lift_op) && (*main->read_bit)(STACKER_SENSE_LOADED, main->userdata)) {
                (*main->write_bit)(STACKER_CMD_FORK, 0, main->userdata);
                main->lift_done = 1;
                model->state = STACKER_LIFT_CONTROLLER_LIFT_DONE;
                break;
            }
            if (main->lift_request && main->lift_op && !((*main->read_bit)(STACKER_SENSE_LOADED, main->userdata))) {
                (*main->write_bit)(STACKER_CMD_FORK, 0, main->userdata);
                main->lift_done = 1;
                model->state = STACKER_LIFT_CONTROLLER_LIFT_DONE;
                break;
            }
            if (!(main->lift_request)) {
                (*main->write_bit)(STACKER_CMD_FORK, 0, main->userdata);
                model->state = STACKER_LIFT_CONTROLLER_LIFT_IDLE;
                break;
            }
            break;
        }
        case STACKER_LIFT_CONTROLLER_LIFT_DONE: {
            if (!(main->lift_request)) {
                (*main->write_bit)(STACKER_CMD_FORK, 0, main->userdata);
                model->state = STACKER_LIFT_CONTROLLER_LIFT_IDLE;
                break;
            }
            break;
        }
        case STACKER_LIFT_CONTROLLER_LIFT_IDLE: {
            if (main->lift_request) {
                (*main->write_bit)(STACKER_CMD_FORK, 1, main->userdata);
                model->state = STACKER_LIFT_CONTROLLER_LIFT_OPERATING;
                break;
            }
            break;
        }
        case STACKER_LIFT_CONTROLLER_END: {
            break;
        }
    }
}

/// Функция сброса модели LiftController (Stacker:LiftController)
void StackerLiftController_reset(StackerLiftController *model, Stacker *main) {
    StackerLiftController_init(model, main);
}

/// Функция проверки терминального состояния модели LiftController (Stacker:LiftController)
bool StackerLiftController_is_done(const StackerLiftController *model, Stacker *main) {
    return model->state == STACKER_LIFT_CONTROLLER_END;
}

/// Функция инициализации модели MovementController (Stacker:MovementController)
void StackerMovementController_init(StackerMovementController *model, Stacker *main) {
    assert(0 != model);
    model->state = STACKER_MOVEMENT_CONTROLLER_INIT;
}

/// Функция обработки модели MovementController (Stacker:MovementController)
void StackerMovementController_tick(StackerMovementController *model, Stacker *main) {
    assert(0 != model);
    assert(0 != main);
    switch (model->state) {
        case STACKER_MOVEMENT_CONTROLLER_INIT: {
            (*main->write_numeric)(STACKER_CMD_TARGET_STACK, CONST_STACKER_CHARGE_STACK, main->userdata);
            (*main->write_numeric)(STACKER_CMD_TARGET_ROW, CONST_STACKER_CHARGE_ROW, main->userdata);
            (*main->write_numeric)(STACKER_CMD_TARGET_SECTION, CONST_STACKER_CHARGE_SECTION, main->userdata);
            (*main->write_bit)(STACKER_CMD_DONE, 0, main->userdata);
            model->state = STACKER_MOVEMENT_CONTROLLER_MOVEMENT_IDLE;
            break;
        }
        case STACKER_MOVEMENT_CONTROLLER_MOVING_TO_STORAGE: {
            if ((*main->read_numeric)(STACKER_POS_STACK, main->userdata) == main->tgt_stack && (*main->read_numeric)(STACKER_POS_ROW, main->userdata) == main->tgt_row && (*main->read_numeric)(STACKER_POS_SECTION, main->userdata) == main->tgt_section) {
                main->lift_request = 1;
                main->lift_op = 0;
                model->state = STACKER_MOVEMENT_CONTROLLER_WAITING_FORK_AT_STORAGE;
                break;
            }
            if ((*main->read_bit)(STACKER_SENSE_BATTERY_LOW, main->userdata)) {
                (*main->write_numeric)(STACKER_CMD_TARGET_STACK, CONST_STACKER_CHARGE_STACK, main->userdata);
                (*main->write_numeric)(STACKER_CMD_TARGET_ROW, CONST_STACKER_CHARGE_ROW, main->userdata);
                (*main->write_numeric)(STACKER_CMD_TARGET_SECTION, CONST_STACKER_CHARGE_SECTION, main->userdata);
                main->lift_request = 0;
                main->lift_done = 0;
                (*main->write_bit)(STACKER_CMD_ACK, 0, main->userdata);
                (*main->write_bit)(STACKER_CMD_DONE, 0, main->userdata);
                main->busy = 0;
                model->state = STACKER_MOVEMENT_CONTROLLER_EMERGENCY_CHARGE;
                break;
            }
            break;
        }
        case STACKER_MOVEMENT_CONTROLLER_WAITING_FORK_AT_CELL: {
            if (main->lift_done) {
                main->lift_request = 0;
                main->lift_done = 0;
                main->busy = 0;
                (*main->write_bit)(STACKER_CMD_DONE, 1, main->userdata);
                model->state = STACKER_MOVEMENT_CONTROLLER_TASK_COMPLETING;
                break;
            }
            break;
        }
        case STACKER_MOVEMENT_CONTROLLER_WAITING_FORK_AT_PICKUP: {
            if (main->lift_done) {
                main->lift_request = 0;
                main->lift_done = 0;
                (*main->write_numeric)(STACKER_CMD_TARGET_STACK, main->tgt_stack, main->userdata);
                (*main->write_numeric)(STACKER_CMD_TARGET_ROW, main->tgt_row, main->userdata);
                (*main->write_numeric)(STACKER_CMD_TARGET_SECTION, main->tgt_section, main->userdata);
                model->state = STACKER_MOVEMENT_CONTROLLER_MOVING_TO_CELL;
                break;
            }
            break;
        }
        case STACKER_MOVEMENT_CONTROLLER_MOVING_TO_CELL: {
            if ((*main->read_numeric)(STACKER_POS_STACK, main->userdata) == main->tgt_stack && (*main->read_numeric)(STACKER_POS_ROW, main->userdata) == main->tgt_row && (*main->read_numeric)(STACKER_POS_SECTION, main->userdata) == main->tgt_section) {
                main->lift_request = 1;
                main->lift_op = 1;
                model->state = STACKER_MOVEMENT_CONTROLLER_WAITING_FORK_AT_CELL;
                break;
            }
            if ((*main->read_bit)(STACKER_SENSE_BATTERY_LOW, main->userdata)) {
                (*main->write_numeric)(STACKER_CMD_TARGET_STACK, CONST_STACKER_CHARGE_STACK, main->userdata);
                (*main->write_numeric)(STACKER_CMD_TARGET_ROW, CONST_STACKER_CHARGE_ROW, main->userdata);
                (*main->write_numeric)(STACKER_CMD_TARGET_SECTION, CONST_STACKER_CHARGE_SECTION, main->userdata);
                main->lift_request = 0;
                main->lift_done = 0;
                (*main->write_bit)(STACKER_CMD_ACK, 0, main->userdata);
                (*main->write_bit)(STACKER_CMD_DONE, 0, main->userdata);
                main->busy = 0;
                model->state = STACKER_MOVEMENT_CONTROLLER_EMERGENCY_CHARGE;
                break;
            }
            break;
        }
        case STACKER_MOVEMENT_CONTROLLER_MOVEMENT_IDLE: {
            if (main->busy && !((*main->read_bit)(STACKER_SENSE_BATTERY_LOW, main->userdata))) {
                model->state = STACKER_MOVEMENT_CONTROLLER_DISPATCH_MOVE;
                break;
            }
            break;
        }
        case STACKER_MOVEMENT_CONTROLLER_TASK_COMPLETING: {
            (*main->write_bit)(STACKER_CMD_DONE, 0, main->userdata);
            (*main->write_numeric)(STACKER_CMD_TARGET_STACK, CONST_STACKER_CHARGE_STACK, main->userdata);
            (*main->write_numeric)(STACKER_CMD_TARGET_ROW, CONST_STACKER_CHARGE_ROW, main->userdata);
            (*main->write_numeric)(STACKER_CMD_TARGET_SECTION, CONST_STACKER_CHARGE_SECTION, main->userdata);
            (*main->write_bit)(STACKER_CMD_DONE, 0, main->userdata);
            model->state = STACKER_MOVEMENT_CONTROLLER_MOVEMENT_IDLE;
            break;
            break;
        }
        case STACKER_MOVEMENT_CONTROLLER_EMERGENCY_CHARGE: {
            if ((*main->read_bit)(STACKER_SENSE_AT_CHARGE, main->userdata)) {
                (*main->write_numeric)(STACKER_CMD_TARGET_STACK, CONST_STACKER_CHARGE_STACK, main->userdata);
                (*main->write_numeric)(STACKER_CMD_TARGET_ROW, CONST_STACKER_CHARGE_ROW, main->userdata);
                (*main->write_numeric)(STACKER_CMD_TARGET_SECTION, CONST_STACKER_CHARGE_SECTION, main->userdata);
                (*main->write_bit)(STACKER_CMD_DONE, 0, main->userdata);
                model->state = STACKER_MOVEMENT_CONTROLLER_MOVEMENT_IDLE;
                break;
            }
            break;
        }
        case STACKER_MOVEMENT_CONTROLLER_DISPATCH_MOVE: {
            if (!(main->tgt_type)) {
                (*main->write_numeric)(STACKER_CMD_TARGET_STACK, CONST_STACKER_PICKUP_STACK, main->userdata);
                (*main->write_numeric)(STACKER_CMD_TARGET_ROW, CONST_STACKER_PICKUP_ROW, main->userdata);
                (*main->write_numeric)(STACKER_CMD_TARGET_SECTION, CONST_STACKER_PICKUP_SECTION, main->userdata);
                model->state = STACKER_MOVEMENT_CONTROLLER_MOVING_TO_PICKUP;
                break;
            }
            if (main->tgt_type) {
                main->lift_request = 0;
                main->lift_done = 0;
                (*main->write_numeric)(STACKER_CMD_TARGET_STACK, main->tgt_stack, main->userdata);
                (*main->write_numeric)(STACKER_CMD_TARGET_ROW, main->tgt_row, main->userdata);
                (*main->write_numeric)(STACKER_CMD_TARGET_SECTION, main->tgt_section, main->userdata);
                model->state = STACKER_MOVEMENT_CONTROLLER_MOVING_TO_STORAGE;
                break;
            }
            break;
        }
        case STACKER_MOVEMENT_CONTROLLER_WAITING_FORK_AT_DROPOFF: {
            if (main->lift_done) {
                main->lift_request = 0;
                main->lift_done = 0;
                main->busy = 0;
                (*main->write_bit)(STACKER_CMD_DONE, 1, main->userdata);
                model->state = STACKER_MOVEMENT_CONTROLLER_TASK_COMPLETING;
                break;
            }
            break;
        }
        case STACKER_MOVEMENT_CONTROLLER_MOVING_TO_DROPOFF: {
            if ((*main->read_numeric)(STACKER_POS_STACK, main->userdata) == CONST_STACKER_DROPOFF_STACK && (*main->read_numeric)(STACKER_POS_ROW, main->userdata) == CONST_STACKER_DROPOFF_ROW && (*main->read_numeric)(STACKER_POS_SECTION, main->userdata) == CONST_STACKER_DROPOFF_SECTION) {
                main->lift_request = 1;
                main->lift_op = 1;
                model->state = STACKER_MOVEMENT_CONTROLLER_WAITING_FORK_AT_DROPOFF;
                break;
            }
            if ((*main->read_bit)(STACKER_SENSE_BATTERY_LOW, main->userdata)) {
                (*main->write_numeric)(STACKER_CMD_TARGET_STACK, CONST_STACKER_CHARGE_STACK, main->userdata);
                (*main->write_numeric)(STACKER_CMD_TARGET_ROW, CONST_STACKER_CHARGE_ROW, main->userdata);
                (*main->write_numeric)(STACKER_CMD_TARGET_SECTION, CONST_STACKER_CHARGE_SECTION, main->userdata);
                main->lift_request = 0;
                main->lift_done = 0;
                (*main->write_bit)(STACKER_CMD_ACK, 0, main->userdata);
                (*main->write_bit)(STACKER_CMD_DONE, 0, main->userdata);
                main->busy = 0;
                model->state = STACKER_MOVEMENT_CONTROLLER_EMERGENCY_CHARGE;
                break;
            }
            break;
        }
        case STACKER_MOVEMENT_CONTROLLER_MOVING_TO_PICKUP: {
            if ((*main->read_numeric)(STACKER_POS_STACK, main->userdata) == CONST_STACKER_PICKUP_STACK && (*main->read_numeric)(STACKER_POS_ROW, main->userdata) == CONST_STACKER_PICKUP_ROW && (*main->read_numeric)(STACKER_POS_SECTION, main->userdata) == CONST_STACKER_PICKUP_SECTION) {
                main->lift_request = 1;
                main->lift_op = 0;
                model->state = STACKER_MOVEMENT_CONTROLLER_WAITING_FORK_AT_PICKUP;
                break;
            }
            if ((*main->read_bit)(STACKER_SENSE_BATTERY_LOW, main->userdata)) {
                (*main->write_numeric)(STACKER_CMD_TARGET_STACK, CONST_STACKER_CHARGE_STACK, main->userdata);
                (*main->write_numeric)(STACKER_CMD_TARGET_ROW, CONST_STACKER_CHARGE_ROW, main->userdata);
                (*main->write_numeric)(STACKER_CMD_TARGET_SECTION, CONST_STACKER_CHARGE_SECTION, main->userdata);
                main->lift_request = 0;
                main->lift_done = 0;
                (*main->write_bit)(STACKER_CMD_ACK, 0, main->userdata);
                (*main->write_bit)(STACKER_CMD_DONE, 0, main->userdata);
                main->busy = 0;
                model->state = STACKER_MOVEMENT_CONTROLLER_EMERGENCY_CHARGE;
                break;
            }
            break;
        }
        case STACKER_MOVEMENT_CONTROLLER_WAITING_FORK_AT_STORAGE: {
            if (main->lift_done) {
                main->lift_request = 0;
                main->lift_done = 0;
                (*main->write_numeric)(STACKER_CMD_TARGET_STACK, CONST_STACKER_DROPOFF_STACK, main->userdata);
                (*main->write_numeric)(STACKER_CMD_TARGET_ROW, CONST_STACKER_DROPOFF_ROW, main->userdata);
                (*main->write_numeric)(STACKER_CMD_TARGET_SECTION, CONST_STACKER_DROPOFF_SECTION, main->userdata);
                model->state = STACKER_MOVEMENT_CONTROLLER_MOVING_TO_DROPOFF;
                break;
            }
            break;
        }
        case STACKER_MOVEMENT_CONTROLLER_END: {
            break;
        }
    }
}

/// Функция сброса модели MovementController (Stacker:MovementController)
void StackerMovementController_reset(StackerMovementController *model, Stacker *main) {
    StackerMovementController_init(model, main);
}

/// Функция проверки терминального состояния модели MovementController (Stacker:MovementController)
bool StackerMovementController_is_done(const StackerMovementController *model, Stacker *main) {
    return model->state == STACKER_MOVEMENT_CONTROLLER_END;
}

/// Функция инициализации модели CommandReceiver (Stacker:CommandReceiver)
void StackerCommandReceiver_init(StackerCommandReceiver *model, Stacker *main) {
    assert(0 != model);
    model->state = STACKER_COMMAND_RECEIVER_INIT;
}

/// Функция обработки модели CommandReceiver (Stacker:CommandReceiver)
void StackerCommandReceiver_tick(StackerCommandReceiver *model, Stacker *main) {
    assert(0 != model);
    assert(0 != main);
    switch (model->state) {
        case STACKER_COMMAND_RECEIVER_INIT: {
            (*main->write_bit)(STACKER_CMD_ACK, 0, main->userdata);
            model->state = STACKER_COMMAND_RECEIVER_WAITING_FOR_TASK;
            break;
        }
        case STACKER_COMMAND_RECEIVER_WAITING_FOR_TASK: {
            if ((*main->read_bit)(STACKER_TASK_VALID, main->userdata) && !(main->busy) && !((*main->read_bit)(STACKER_SENSE_BATTERY_LOW, main->userdata))) {
                main->tgt_stack = (*main->read_numeric)(STACKER_TASK_STACK_NO, main->userdata);
                main->tgt_row = (*main->read_numeric)(STACKER_TASK_ROW_NO, main->userdata);
                main->tgt_section = (*main->read_numeric)(STACKER_TASK_SECTION_NO, main->userdata);
                main->tgt_type = (*main->read_bit)(STACKER_TASK_TYPE, main->userdata);
                main->eta = Stacker_travel_time(main, (*main->read_numeric)(STACKER_TASK_STACK_NO, main->userdata), (*main->read_numeric)(STACKER_TASK_ROW_NO, main->userdata), (*main->read_numeric)(STACKER_TASK_SECTION_NO, main->userdata));
                main->busy = 1;
                (*main->write_bit)(STACKER_CMD_ACK, 1, main->userdata);
                model->state = STACKER_COMMAND_RECEIVER_ACCEPTING_TASK;
                break;
            }
            break;
        }
        case STACKER_COMMAND_RECEIVER_TASK_ACTIVE: {
            if (!(main->busy)) {
                (*main->write_bit)(STACKER_CMD_ACK, 0, main->userdata);
                model->state = STACKER_COMMAND_RECEIVER_WAITING_FOR_TASK;
                break;
            }
            break;
        }
        case STACKER_COMMAND_RECEIVER_ACCEPTING_TASK: {
            (*main->write_bit)(STACKER_CMD_ACK, 0, main->userdata);
            model->state = STACKER_COMMAND_RECEIVER_TASK_ACTIVE;
            break;
            break;
        }
        case STACKER_COMMAND_RECEIVER_END: {
            break;
        }
    }
}

/// Функция сброса модели CommandReceiver (Stacker:CommandReceiver)
void StackerCommandReceiver_reset(StackerCommandReceiver *model, Stacker *main) {
    StackerCommandReceiver_init(model, main);
}

/// Функция проверки терминального состояния модели CommandReceiver (Stacker:CommandReceiver)
bool StackerCommandReceiver_is_done(const StackerCommandReceiver *model, Stacker *main) {
    return model->state == STACKER_COMMAND_RECEIVER_END;
}

/// Функция инициализации модели stacker (Stacker)
void Stacker_init(Stacker *model) {
    assert(0 != model);
    model->state = STACKER_INIT;
    model->tgt_row = 0;
    model->tgt_section = 0;
    model->lift_request = 0;
    model->tgt_stack = 0;
    model->eta = 0;
    model->busy = 0;
    model->lift_done = 0;
    model->tgt_type = 0;
    model->lift_op = 0;
}

/// Функция обработки модели stacker (Stacker)
void Stacker_tick(Stacker *model) {
    assert(0 != model);
    switch (model->state) {
        case STACKER_INIT: {
            StackerCommandReceiver_init(&model->stacker.command_receiver0, model);
            StackerMovementController_init(&model->stacker.movement_controller1, model);
            StackerLiftController_init(&model->stacker.lift_controller2, model);
            model->stacker.state = STACKER_STACKER_INIT;
            model->state = STACKER_STACKER;
            break;
        }
        case STACKER_STACKER: {
            StackerCommandReceiver_tick(&model->stacker.command_receiver0, model);
            StackerMovementController_tick(&model->stacker.movement_controller1, model);
            StackerLiftController_tick(&model->stacker.lift_controller2, model);
            if (StackerCommandReceiver_is_done(&model->stacker.command_receiver0, model) && StackerMovementController_is_done(&model->stacker.movement_controller1, model) && StackerLiftController_is_done(&model->stacker.lift_controller2, model)) {
                model->state = STACKER_END;
                break;
            }
            break;
        }
        case STACKER_END: {
            break;
        }
    }
}

/// Функция сброса модели stacker (Stacker)
void Stacker_reset(Stacker *model) {
    Stacker_init(model);
}

/// Функция проверки терминального состояния модели stacker (Stacker)
bool Stacker_is_done(const Stacker *model) {
    return model->state == STACKER_END;
}

