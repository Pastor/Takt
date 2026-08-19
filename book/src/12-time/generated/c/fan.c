#include "fan.h"
#include <assert.h>
#include <math.h>
/// Model functions 'Fan (Fan:Fan)'
static void FanFan_init(FanFan *model, Fan *main);
static void FanFan_tick(FanFan *model, Fan *main);
static bool FanFan_is_done(const FanFan *model, Fan *main);

/// Функция инициализации модели Fan (Fan:Fan)
void FanFan_init(FanFan *model, Fan *main) {
    assert(0 != model);
    (void)main;
    model->state = FAN_FAN_INIT;
    model->takt_dwell = 0;
    model->takt_prev_state = (unsigned)FAN_FAN_INIT;
}

/// Функция обработки модели Fan (Fan:Fan)
void FanFan_tick(FanFan *model, Fan *main) {
    assert(0 != model);
    assert(0 != main);
    if (model->state == FAN_FAN_INIT) {
        (*main->write_bit)(FAN_FAN_PORT_MOTOR, 0, main->userdata);
        model->state = FAN_FAN_IDLE;
    }
    switch (model->state) {
        case FAN_FAN_IDLE: {
            if ((*main->read_bit)(FAN_FAN_PORT_LIGHT, main->userdata) == 1) {
                (*main->write_bit)(FAN_FAN_PORT_MOTOR, 1, main->userdata);
                model->state = FAN_FAN_WORKING;
                break;
            }
            break;
        }
        case FAN_FAN_OVERRUN: {
            if ((*main->read_bit)(FAN_FAN_PORT_LIGHT, main->userdata) == 1) {
                (*main->write_bit)(FAN_FAN_PORT_MOTOR, 1, main->userdata);
                model->state = FAN_FAN_WORKING;
                break;
            }
            if (model->takt_dwell >= 180000) {
                (*main->write_bit)(FAN_FAN_PORT_MOTOR, 0, main->userdata);
                model->state = FAN_FAN_IDLE;
                break;
            }
            break;
        }
        case FAN_FAN_WORKING: {
            if ((*main->read_bit)(FAN_FAN_PORT_LIGHT, main->userdata) == 0) {
                model->state = FAN_FAN_OVERRUN;
                break;
            }
            break;
        }
        case FAN_FAN_END: {
            break;
        }
        default: break;
    }
    if ((unsigned)model->state != model->takt_prev_state) {
        model->takt_dwell = 1;
        model->takt_prev_state = (unsigned)model->state;
    } else {
        model->takt_dwell++;
    }
}

/// Функция сброса модели Fan (Fan:Fan)
void FanFan_reset(FanFan *model, Fan *main) {
    FanFan_init(model, main);
}

/// Функция проверки терминального состояния модели Fan (Fan:Fan)
bool FanFan_is_done(const FanFan *model, Fan *main) {
    (void)main;
    return model->state == FAN_FAN_END;
}

/// Функция инициализации модели fan (Fan)
void Fan_init(Fan *model) {
    assert(0 != model);
    model->state = FAN_INIT;
    FanFan_init(&model->main, model);
}

/// Функция обработки модели fan (Fan)
void Fan_tick(Fan *model) {
    assert(0 != model);
    if (model->state == FAN_INIT) {
        model->state = FAN_MAIN;
    }
    switch (model->state) {
        case FAN_MAIN: {
            FanFan_tick(&model->main, model);
            if (FanFan_is_done(&model->main, model)) {
                model->state = FAN_END;
                break;
            }
            break;
        }
        case FAN_END: {
            break;
        }
        default: break;
    }
}

/// Функция сброса модели fan (Fan)
void Fan_reset(Fan *model) {
    Fan_init(model);
}

/// Функция проверки терминального состояния модели fan (Fan)
bool Fan_is_done(const Fan *model) {
    return model->state == FAN_END;
}

