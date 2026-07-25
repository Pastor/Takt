#include "lift.h"
#include <assert.h>
#include <math.h>
/// Константы и порты модели lift (Lift)
#define CONST_LIFT_DWELL_TICKS 3
/// Функция инициализации модели lift (Lift)
void Lift_init(Lift *model) {
    assert(0 != model);
    model->state = LIFT_INIT;
    model->doors = 0;
    model->dwell = 0;
    model->moving = 0;
}

/// Функция обработки модели lift (Lift)
void Lift_tick(Lift *model) {
    assert(0 != model);
    assert(model->moving == 0 || model->doors == 0);
    if (model->state == LIFT_INIT) {
        model->moving = 0;
        (*model->write_bit)(LIFT_MOTOR_UP, 0, model->userdata);
        (*model->write_bit)(LIFT_MOTOR_DOWN, 0, model->userdata);
        (*model->write_bit)(LIFT_BRAKE, 1, model->userdata);
        model->doors = 0;
        (*model->write_bit)(LIFT_DOORS_OPEN, 0, model->userdata);
        model->state = LIFT_WAITING;
    }
    switch (model->state) {
        case LIFT_BOARDING: {
            model->dwell = model->dwell + 1;
            if (model->dwell >= CONST_LIFT_DWELL_TICKS) {
                model->doors = 0;
                (*model->write_bit)(LIFT_DOORS_OPEN, 0, model->userdata);
                model->state = LIFT_LEAVING;
                break;
            }
            break;
        }
        case LIFT_GOING_DOWN: {
            (*model->write_numeric)(LIFT_DISPLAY, (*model->read_numeric)(LIFT_AT_FLOOR, model->userdata), model->userdata);
            if ((*model->read_numeric)(LIFT_AT_FLOOR, model->userdata) <= (*model->read_numeric)(LIFT_CALL, model->userdata)) {
                model->moving = 0;
                (*model->write_bit)(LIFT_MOTOR_UP, 0, model->userdata);
                (*model->write_bit)(LIFT_MOTOR_DOWN, 0, model->userdata);
                (*model->write_bit)(LIFT_BRAKE, 1, model->userdata);
                model->state = LIFT_STOPPING;
                break;
            }
            break;
        }
        case LIFT_GOING_UP: {
            (*model->write_numeric)(LIFT_DISPLAY, (*model->read_numeric)(LIFT_AT_FLOOR, model->userdata), model->userdata);
            if ((*model->read_numeric)(LIFT_AT_FLOOR, model->userdata) >= (*model->read_numeric)(LIFT_CALL, model->userdata)) {
                model->moving = 0;
                (*model->write_bit)(LIFT_MOTOR_UP, 0, model->userdata);
                (*model->write_bit)(LIFT_MOTOR_DOWN, 0, model->userdata);
                (*model->write_bit)(LIFT_BRAKE, 1, model->userdata);
                model->state = LIFT_STOPPING;
                break;
            }
            break;
        }
        case LIFT_LEAVING: {
            model->moving = 0;
            (*model->write_bit)(LIFT_MOTOR_UP, 0, model->userdata);
            (*model->write_bit)(LIFT_MOTOR_DOWN, 0, model->userdata);
            (*model->write_bit)(LIFT_BRAKE, 1, model->userdata);
            model->doors = 0;
            (*model->write_bit)(LIFT_DOORS_OPEN, 0, model->userdata);
            model->state = LIFT_WAITING;
            break;
            break;
        }
        case LIFT_STOPPING: {
            model->doors = 1;
            (*model->write_bit)(LIFT_DOORS_OPEN, 1, model->userdata);
            model->dwell = 0;
            model->state = LIFT_BOARDING;
            break;
            break;
        }
        case LIFT_WAITING: {
            (*model->write_numeric)(LIFT_DISPLAY, (*model->read_numeric)(LIFT_AT_FLOOR, model->userdata), model->userdata);
            if ((*model->read_numeric)(LIFT_CALL, model->userdata) == (*model->read_numeric)(LIFT_AT_FLOOR, model->userdata)) {
                model->doors = 1;
                (*model->write_bit)(LIFT_DOORS_OPEN, 1, model->userdata);
                model->dwell = 0;
                model->state = LIFT_BOARDING;
                break;
            }
            if ((*model->read_numeric)(LIFT_CALL, model->userdata) > (*model->read_numeric)(LIFT_AT_FLOOR, model->userdata)) {
                model->moving = 1;
                (*model->write_bit)(LIFT_BRAKE, 0, model->userdata);
                (*model->write_bit)(LIFT_MOTOR_UP, 1, model->userdata);
                model->state = LIFT_GOING_UP;
                break;
            }
            if ((*model->read_numeric)(LIFT_CALL, model->userdata) > 0 && (*model->read_numeric)(LIFT_CALL, model->userdata) < (*model->read_numeric)(LIFT_AT_FLOOR, model->userdata)) {
                model->moving = 1;
                (*model->write_bit)(LIFT_BRAKE, 0, model->userdata);
                (*model->write_bit)(LIFT_MOTOR_DOWN, 1, model->userdata);
                model->state = LIFT_GOING_DOWN;
                break;
            }
            break;
        }
        case LIFT_END: {
            break;
        }
        default: break;
    }
}

/// Функция сброса модели lift (Lift)
void Lift_reset(Lift *model) {
    Lift_init(model);
}

/// Функция проверки терминального состояния модели lift (Lift)
bool Lift_is_done(const Lift *model) {
    return model->state == LIFT_END;
}

