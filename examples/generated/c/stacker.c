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
///Функции моделей
static int Stacker_accept_task(const Stacker *model) {
    model->tgt_stack = (*model->read_numeric)(STACKER_TASK_STACK_NO, model->userdata);
    model->tgt_row = (*model->read_numeric)(STACKER_TASK_ROW_NO, model->userdata);
    model->tgt_section = (*model->read_numeric)(STACKER_TASK_SECTION_NO, model->userdata);
    model->tgt_type = (*model->read_bit)(STACKER_TASK_TYPE, model->userdata);
    model->busy = 1;
    (*model->write_bit)(STACKER_CMD_ACK, 1, model->userdata);
    return 1;
}

static int Stacker_complete_task(const Stacker *model) {
    model->busy = 0;
    (*model->write_bit)(STACKER_CMD_DONE, 1, model->userdata);
    (*model->write_bit)(STACKER_CMD_ACK, 0, model->userdata);
    return 1;
}

/// Функция инициализации модели stacker (Stacker)
void Stacker_init(Stacker *model) {
    assert(0 != model);
    model->state = STACKER_INIT;
    model->tgt_type = 0;
    model->tgt_section = 0;
    model->tgt_row = 0;
    model->tgt_stack = 0;
    model->busy = 0;
}

/// Функция обработки модели stacker (Stacker)
void Stacker_tick(Stacker *model) {
    assert(0 != model);
    switch (model->state) {
        case STACKER_INIT: {
            (*model->write_numeric)(STACKER_CMD_TARGET_STACK, CONST_STACKER_CHARGE_STACK, model->userdata);
            (*model->write_numeric)(STACKER_CMD_TARGET_ROW, CONST_STACKER_CHARGE_ROW, model->userdata);
            (*model->write_numeric)(STACKER_CMD_TARGET_SECTION, CONST_STACKER_CHARGE_SECTION, model->userdata);
            (*model->write_bit)(STACKER_CMD_FORK, 0, model->userdata);
            (*model->write_bit)(STACKER_CMD_DONE, 0, model->userdata);
            (*model->write_bit)(STACKER_CMD_ACK, 0, model->userdata);
            model->state = STACKER_IDLE;
            break;
        }
        case STACKER_MOVING_TO_STORAGE: {
            if ((*model->read_numeric)(STACKER_POS_STACK, model->userdata) == model->tgt_stack && (*model->read_numeric)(STACKER_POS_ROW, model->userdata) == model->tgt_row && (*model->read_numeric)(STACKER_POS_SECTION, model->userdata) == model->tgt_section) {
                (*model->write_bit)(STACKER_CMD_FORK, 1, model->userdata);
                model->state = STACKER_TAKING_FROM_CELL;
                break;
            }
            if ((*model->read_bit)(STACKER_SENSE_BATTERY_LOW, model->userdata)) {
                (*model->write_numeric)(STACKER_CMD_TARGET_STACK, CONST_STACKER_CHARGE_STACK, model->userdata);
                (*model->write_numeric)(STACKER_CMD_TARGET_ROW, CONST_STACKER_CHARGE_ROW, model->userdata);
                (*model->write_numeric)(STACKER_CMD_TARGET_SECTION, CONST_STACKER_CHARGE_SECTION, model->userdata);
                (*model->write_bit)(STACKER_CMD_FORK, 0, model->userdata);
                (*model->write_bit)(STACKER_CMD_ACK, 0, model->userdata);
                (*model->write_bit)(STACKER_CMD_DONE, 0, model->userdata);
                model->busy = 0;
                model->state = STACKER_EMERGENCY_CHARGE;
                break;
            }
            break;
        }
        case STACKER_DISPATCH_TASK: {
            if (!(model->tgt_type)) {
                (*model->write_numeric)(STACKER_CMD_TARGET_STACK, CONST_STACKER_PICKUP_STACK, model->userdata);
                (*model->write_numeric)(STACKER_CMD_TARGET_ROW, CONST_STACKER_PICKUP_ROW, model->userdata);
                (*model->write_numeric)(STACKER_CMD_TARGET_SECTION, CONST_STACKER_PICKUP_SECTION, model->userdata);
                (*model->write_bit)(STACKER_CMD_FORK, 0, model->userdata);
                model->state = STACKER_MOVING_TO_PICKUP;
                break;
            }
            if (model->tgt_type) {
                (*model->write_numeric)(STACKER_CMD_TARGET_STACK, model->tgt_stack, model->userdata);
                (*model->write_numeric)(STACKER_CMD_TARGET_ROW, model->tgt_row, model->userdata);
                (*model->write_numeric)(STACKER_CMD_TARGET_SECTION, model->tgt_section, model->userdata);
                (*model->write_bit)(STACKER_CMD_FORK, 0, model->userdata);
                model->state = STACKER_MOVING_TO_STORAGE;
                break;
            }
            break;
        }
        case STACKER_DELIVERING_LOAD: {
            if (!(*model->read_bit)(STACKER_SENSE_LOADED, model->userdata)) {
                (*model->write_bit)(STACKER_CMD_FORK, 0, model->userdata);
            }
            if (!((*model->read_bit)(STACKER_SENSE_LOADED, model->userdata))) {
                Stacker_complete_task(model);
                model->state = STACKER_COMPLETING;
                break;
            }
            break;
        }
        case STACKER_IDLE: {
            if ((*model->read_bit)(STACKER_TASK_VALID, model->userdata) & !model->busy & !(*model->read_bit)(STACKER_SENSE_BATTERY_LOW, model->userdata)) {
                Stacker_accept_task(model);
            }
            if (model->busy && !((*model->read_bit)(STACKER_SENSE_BATTERY_LOW, model->userdata))) {
                (*model->write_bit)(STACKER_CMD_ACK, 0, model->userdata);
                model->state = STACKER_DISPATCH_TASK;
                break;
            }
            break;
        }
        case STACKER_TAKING_FROM_CELL: {
            if ((*model->read_bit)(STACKER_SENSE_LOADED, model->userdata)) {
                (*model->write_numeric)(STACKER_CMD_TARGET_STACK, CONST_STACKER_DROPOFF_STACK, model->userdata);
                (*model->write_numeric)(STACKER_CMD_TARGET_ROW, CONST_STACKER_DROPOFF_ROW, model->userdata);
                (*model->write_numeric)(STACKER_CMD_TARGET_SECTION, CONST_STACKER_DROPOFF_SECTION, model->userdata);
                (*model->write_bit)(STACKER_CMD_FORK, 0, model->userdata);
                model->state = STACKER_MOVING_TO_DROPOFF;
                break;
            }
            break;
        }
        case STACKER_MOVING_TO_PICKUP: {
            if ((*model->read_numeric)(STACKER_POS_STACK, model->userdata) == CONST_STACKER_PICKUP_STACK && (*model->read_numeric)(STACKER_POS_ROW, model->userdata) == CONST_STACKER_PICKUP_ROW && (*model->read_numeric)(STACKER_POS_SECTION, model->userdata) == CONST_STACKER_PICKUP_SECTION) {
                (*model->write_bit)(STACKER_CMD_FORK, 1, model->userdata);
                model->state = STACKER_TAKING_AT_PICKUP;
                break;
            }
            if ((*model->read_bit)(STACKER_SENSE_BATTERY_LOW, model->userdata)) {
                (*model->write_numeric)(STACKER_CMD_TARGET_STACK, CONST_STACKER_CHARGE_STACK, model->userdata);
                (*model->write_numeric)(STACKER_CMD_TARGET_ROW, CONST_STACKER_CHARGE_ROW, model->userdata);
                (*model->write_numeric)(STACKER_CMD_TARGET_SECTION, CONST_STACKER_CHARGE_SECTION, model->userdata);
                (*model->write_bit)(STACKER_CMD_FORK, 0, model->userdata);
                (*model->write_bit)(STACKER_CMD_ACK, 0, model->userdata);
                (*model->write_bit)(STACKER_CMD_DONE, 0, model->userdata);
                model->busy = 0;
                model->state = STACKER_EMERGENCY_CHARGE;
                break;
            }
            break;
        }
        case STACKER_MOVING_TO_DROPOFF: {
            if ((*model->read_numeric)(STACKER_POS_STACK, model->userdata) == CONST_STACKER_DROPOFF_STACK && (*model->read_numeric)(STACKER_POS_ROW, model->userdata) == CONST_STACKER_DROPOFF_ROW && (*model->read_numeric)(STACKER_POS_SECTION, model->userdata) == CONST_STACKER_DROPOFF_SECTION) {
                (*model->write_bit)(STACKER_CMD_FORK, 1, model->userdata);
                model->state = STACKER_DELIVERING_LOAD;
                break;
            }
            if ((*model->read_bit)(STACKER_SENSE_BATTERY_LOW, model->userdata)) {
                (*model->write_numeric)(STACKER_CMD_TARGET_STACK, CONST_STACKER_CHARGE_STACK, model->userdata);
                (*model->write_numeric)(STACKER_CMD_TARGET_ROW, CONST_STACKER_CHARGE_ROW, model->userdata);
                (*model->write_numeric)(STACKER_CMD_TARGET_SECTION, CONST_STACKER_CHARGE_SECTION, model->userdata);
                (*model->write_bit)(STACKER_CMD_FORK, 0, model->userdata);
                (*model->write_bit)(STACKER_CMD_ACK, 0, model->userdata);
                (*model->write_bit)(STACKER_CMD_DONE, 0, model->userdata);
                model->busy = 0;
                model->state = STACKER_EMERGENCY_CHARGE;
                break;
            }
            break;
        }
        case STACKER_EMERGENCY_CHARGE: {
            if ((*model->read_bit)(STACKER_SENSE_AT_CHARGE, model->userdata)) {
                (*model->write_numeric)(STACKER_CMD_TARGET_STACK, CONST_STACKER_CHARGE_STACK, model->userdata);
                (*model->write_numeric)(STACKER_CMD_TARGET_ROW, CONST_STACKER_CHARGE_ROW, model->userdata);
                (*model->write_numeric)(STACKER_CMD_TARGET_SECTION, CONST_STACKER_CHARGE_SECTION, model->userdata);
                (*model->write_bit)(STACKER_CMD_FORK, 0, model->userdata);
                (*model->write_bit)(STACKER_CMD_DONE, 0, model->userdata);
                (*model->write_bit)(STACKER_CMD_ACK, 0, model->userdata);
                model->state = STACKER_IDLE;
                break;
            }
            break;
        }
        case STACKER_TAKING_AT_PICKUP: {
            if ((*model->read_bit)(STACKER_SENSE_LOADED, model->userdata)) {
                (*model->write_numeric)(STACKER_CMD_TARGET_STACK, model->tgt_stack, model->userdata);
                (*model->write_numeric)(STACKER_CMD_TARGET_ROW, model->tgt_row, model->userdata);
                (*model->write_numeric)(STACKER_CMD_TARGET_SECTION, model->tgt_section, model->userdata);
                (*model->write_bit)(STACKER_CMD_FORK, 0, model->userdata);
                model->state = STACKER_MOVING_TO_CELL;
                break;
            }
            break;
        }
        case STACKER_MOVING_TO_CELL: {
            if ((*model->read_numeric)(STACKER_POS_STACK, model->userdata) == model->tgt_stack && (*model->read_numeric)(STACKER_POS_ROW, model->userdata) == model->tgt_row && (*model->read_numeric)(STACKER_POS_SECTION, model->userdata) == model->tgt_section) {
                (*model->write_bit)(STACKER_CMD_FORK, 1, model->userdata);
                model->state = STACKER_PLACING_IN_CELL;
                break;
            }
            if ((*model->read_bit)(STACKER_SENSE_BATTERY_LOW, model->userdata)) {
                (*model->write_numeric)(STACKER_CMD_TARGET_STACK, CONST_STACKER_CHARGE_STACK, model->userdata);
                (*model->write_numeric)(STACKER_CMD_TARGET_ROW, CONST_STACKER_CHARGE_ROW, model->userdata);
                (*model->write_numeric)(STACKER_CMD_TARGET_SECTION, CONST_STACKER_CHARGE_SECTION, model->userdata);
                (*model->write_bit)(STACKER_CMD_FORK, 0, model->userdata);
                (*model->write_bit)(STACKER_CMD_ACK, 0, model->userdata);
                (*model->write_bit)(STACKER_CMD_DONE, 0, model->userdata);
                model->busy = 0;
                model->state = STACKER_EMERGENCY_CHARGE;
                break;
            }
            break;
        }
        case STACKER_PLACING_IN_CELL: {
            if (!(*model->read_bit)(STACKER_SENSE_LOADED, model->userdata)) {
                (*model->write_bit)(STACKER_CMD_FORK, 0, model->userdata);
            }
            if (!((*model->read_bit)(STACKER_SENSE_LOADED, model->userdata))) {
                Stacker_complete_task(model);
                model->state = STACKER_COMPLETING;
                break;
            }
            break;
        }
        case STACKER_COMPLETING: {
            (*model->write_bit)(STACKER_CMD_DONE, 0, model->userdata);
            if ((*model->read_bit)(STACKER_TASK_VALID, model->userdata) && !((*model->read_bit)(STACKER_SENSE_BATTERY_LOW, model->userdata))) {
                (*model->write_bit)(STACKER_CMD_ACK, 0, model->userdata);
                model->state = STACKER_DISPATCH_TASK;
                break;
            }
            if (!((*model->read_bit)(STACKER_TASK_VALID, model->userdata))) {
                (*model->write_numeric)(STACKER_CMD_TARGET_STACK, CONST_STACKER_CHARGE_STACK, model->userdata);
                (*model->write_numeric)(STACKER_CMD_TARGET_ROW, CONST_STACKER_CHARGE_ROW, model->userdata);
                (*model->write_numeric)(STACKER_CMD_TARGET_SECTION, CONST_STACKER_CHARGE_SECTION, model->userdata);
                (*model->write_bit)(STACKER_CMD_FORK, 0, model->userdata);
                (*model->write_bit)(STACKER_CMD_DONE, 0, model->userdata);
                (*model->write_bit)(STACKER_CMD_ACK, 0, model->userdata);
                model->state = STACKER_IDLE;
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

