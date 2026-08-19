#include "regulator.h"
#include <assert.h>
#include <math.h>
static int64_t lam_q_floordiv(int64_t x, int64_t d) {
    int64_t q = x / d;
    return ((x % d != 0) && ((x < 0) != (d < 0))) ? q - 1 : q;
}
static int64_t lam_q_mul(int64_t a, int64_t b, unsigned n) {
    return lam_q_floordiv(a * b, (int64_t)1 << n);
}
/// Model functions 'Regulator (Regulator:Regulator)'
static void RegulatorRegulator_init(RegulatorRegulator *model, Regulator *main);
static void RegulatorRegulator_tick(RegulatorRegulator *model, Regulator *main);
static bool RegulatorRegulator_is_done(const RegulatorRegulator *model, Regulator *main);

/// Функция инициализации модели Regulator (Regulator:Regulator)
void RegulatorRegulator_init(RegulatorRegulator *model, Regulator *main) {
    assert(0 != model);
    (void)main;
    model->state = REGULATOR_REGULATOR_INIT;
    model->half = 128;
    model->near = 2432;
    model->setpoint = 2560;
    model->value = 0;
}

/// Функция обработки модели Regulator (Regulator:Regulator)
void RegulatorRegulator_tick(RegulatorRegulator *model, Regulator *main) {
    assert(0 != model);
    assert(0 != main);
    if (model->state == REGULATOR_REGULATOR_INIT) {
        model->state = REGULATOR_REGULATOR_ADJUST;
    }
    switch (model->state) {
        case REGULATOR_REGULATOR_ADJUST: {
            model->value = (int16_t)((int64_t)(model->value) + (int64_t)((int16_t)(lam_q_mul((int64_t)(((int16_t)((int64_t)(model->setpoint) - (int64_t)(model->value)))), (int64_t)(model->half), 8))));
            if (model->value >= model->near) {
                model->state = REGULATOR_REGULATOR_SETTLED;
                break;
            }
            break;
        }
        case REGULATOR_REGULATOR_DONE: {
            (*main->write_bit)(REGULATOR_REGULATOR_PORT_READY, 1, main->userdata);
            model->state = REGULATOR_REGULATOR_END;
            break;
        }
        case REGULATOR_REGULATOR_SETTLED: {
            model->value = model->setpoint;
            model->state = REGULATOR_REGULATOR_DONE;
            break;
        }
        case REGULATOR_REGULATOR_END: {
            break;
        }
        default: break;
    }
}

/// Функция сброса модели Regulator (Regulator:Regulator)
void RegulatorRegulator_reset(RegulatorRegulator *model, Regulator *main) {
    RegulatorRegulator_init(model, main);
}

/// Функция проверки терминального состояния модели Regulator (Regulator:Regulator)
bool RegulatorRegulator_is_done(const RegulatorRegulator *model, Regulator *main) {
    (void)main;
    return model->state == REGULATOR_REGULATOR_END;
}

/// Функция инициализации модели regulator (Regulator)
void Regulator_init(Regulator *model) {
    assert(0 != model);
    model->state = REGULATOR_INIT;
    RegulatorRegulator_init(&model->main, model);
}

/// Функция обработки модели regulator (Regulator)
void Regulator_tick(Regulator *model) {
    assert(0 != model);
    if (model->state == REGULATOR_INIT) {
        model->state = REGULATOR_MAIN;
    }
    switch (model->state) {
        case REGULATOR_MAIN: {
            RegulatorRegulator_tick(&model->main, model);
            if (RegulatorRegulator_is_done(&model->main, model)) {
                model->state = REGULATOR_END;
                break;
            }
            break;
        }
        case REGULATOR_END: {
            break;
        }
        default: break;
    }
}

/// Функция сброса модели regulator (Regulator)
void Regulator_reset(Regulator *model) {
    Regulator_init(model);
}

/// Функция проверки терминального состояния модели regulator (Regulator)
bool Regulator_is_done(const Regulator *model) {
    return model->state == REGULATOR_END;
}

