#include "float_regulator.h"
#include <assert.h>
#include <math.h>
/// Model functions 'FloatRegulator (FloatRegulator:FloatRegulator)'
static void FloatRegulatorFloatRegulator_init(FloatRegulatorFloatRegulator *model, FloatRegulator *main);
static void FloatRegulatorFloatRegulator_tick(FloatRegulatorFloatRegulator *model, FloatRegulator *main);
static bool FloatRegulatorFloatRegulator_is_done(const FloatRegulatorFloatRegulator *model, FloatRegulator *main);

/// Функция инициализации модели FloatRegulator (FloatRegulator:FloatRegulator)
void FloatRegulatorFloatRegulator_init(FloatRegulatorFloatRegulator *model, FloatRegulator *main) {
    assert(0 != model);
    model->state = FLOAT_REGULATOR_FLOAT_REGULATOR_INIT;
    model->half = 0.5;
    model->near = 9.5;
    model->setpoint = 10.0;
    model->value = 0.0;
}

/// Функция обработки модели FloatRegulator (FloatRegulator:FloatRegulator)
void FloatRegulatorFloatRegulator_tick(FloatRegulatorFloatRegulator *model, FloatRegulator *main) {
    assert(0 != model);
    assert(0 != main);
    if (model->state == FLOAT_REGULATOR_FLOAT_REGULATOR_INIT) {
        model->state = FLOAT_REGULATOR_FLOAT_REGULATOR_ADJUST;
    }
    switch (model->state) {
        case FLOAT_REGULATOR_FLOAT_REGULATOR_ADJUST: {
            model->value = model->value + (model->setpoint - model->value) * model->half;
            if (model->value >= model->near) {
                model->state = FLOAT_REGULATOR_FLOAT_REGULATOR_SETTLED;
                break;
            }
            break;
        }
        case FLOAT_REGULATOR_FLOAT_REGULATOR_DONE: {
            (*main->write_bit)(FLOAT_REGULATOR_FLOAT_REGULATOR_PORT_READY, 1, main->userdata);
            model->state = FLOAT_REGULATOR_FLOAT_REGULATOR_END;
            break;
        }
        case FLOAT_REGULATOR_FLOAT_REGULATOR_SETTLED: {
            model->value = model->setpoint;
            model->state = FLOAT_REGULATOR_FLOAT_REGULATOR_DONE;
            break;
            break;
        }
        case FLOAT_REGULATOR_FLOAT_REGULATOR_END: {
            break;
        }
        default: break;
    }
}

/// Функция сброса модели FloatRegulator (FloatRegulator:FloatRegulator)
void FloatRegulatorFloatRegulator_reset(FloatRegulatorFloatRegulator *model, FloatRegulator *main) {
    FloatRegulatorFloatRegulator_init(model, main);
}

/// Функция проверки терминального состояния модели FloatRegulator (FloatRegulator:FloatRegulator)
bool FloatRegulatorFloatRegulator_is_done(const FloatRegulatorFloatRegulator *model, FloatRegulator *main) {
    return model->state == FLOAT_REGULATOR_FLOAT_REGULATOR_END;
}

/// Функция инициализации модели float_regulator (FloatRegulator)
void FloatRegulator_init(FloatRegulator *model) {
    assert(0 != model);
    model->state = FLOAT_REGULATOR_INIT;
    FloatRegulatorFloatRegulator_init(&model->main, model);
}

/// Функция обработки модели float_regulator (FloatRegulator)
void FloatRegulator_tick(FloatRegulator *model) {
    assert(0 != model);
    if (model->state == FLOAT_REGULATOR_INIT) {
        model->state = FLOAT_REGULATOR_MAIN;
    }
    switch (model->state) {
        case FLOAT_REGULATOR_MAIN: {
            FloatRegulatorFloatRegulator_tick(&model->main, model);
            if (FloatRegulatorFloatRegulator_is_done(&model->main, model)) {
                model->state = FLOAT_REGULATOR_END;
                break;
            }
            break;
        }
        case FLOAT_REGULATOR_END: {
            break;
        }
        default: break;
    }
}

/// Функция сброса модели float_regulator (FloatRegulator)
void FloatRegulator_reset(FloatRegulator *model) {
    FloatRegulator_init(model);
}

/// Функция проверки терминального состояния модели float_regulator (FloatRegulator)
bool FloatRegulator_is_done(const FloatRegulator *model) {
    return model->state == FLOAT_REGULATOR_END;
}

