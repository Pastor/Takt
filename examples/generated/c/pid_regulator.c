#include "pid_regulator.h"
#include <assert.h>
#include <math.h>
static int64_t lam_q_floordiv(int64_t x, int64_t d) {
    int64_t q = x / d;
    return ((x % d != 0) && ((x < 0) != (d < 0))) ? q - 1 : q;
}
static int64_t lam_q_mul(int64_t a, int64_t b, unsigned n) {
    return lam_q_floordiv(a * b, (int64_t)1 << n);
}
/// Model functions 'Pid (PidRegulator:Pid)'
static void PidRegulatorPid_init(PidRegulatorPid *model, PidRegulator *main);
static void PidRegulatorPid_tick(PidRegulatorPid *model, PidRegulator *main);
static bool PidRegulatorPid_is_done(const PidRegulatorPid *model, PidRegulator *main);

/// Функция инициализации модели Pid (PidRegulator:Pid)
void PidRegulatorPid_init(PidRegulatorPid *model, PidRegulator *main) {
    assert(0 != model);
    model->state = PID_REGULATOR_PID_INIT;
    model->ctrl = 0;
    model->deriv = 0;
    model->eps = 32;
    model->err = 0;
    model->err_prev = 0;
    model->i_acc = 0;
    model->imax = 8192;
    model->kd = 64;
    model->ki = 16;
    model->kp = 128;
    model->kplant = 128;
    model->meas = 0;
    model->neg_imax = -8192;
    model->target = 2048;
}

/// Функция обработки модели Pid (PidRegulator:Pid)
void PidRegulatorPid_tick(PidRegulatorPid *model, PidRegulator *main) {
    assert(0 != model);
    assert(0 != main);
    if (model->state == PID_REGULATOR_PID_INIT) {
        model->state = PID_REGULATOR_PID_CONTROL;
    }
    switch (model->state) {
        case PID_REGULATOR_PID_CONTROL: {
            model->err = (int16_t)((int64_t)(model->target) - (int64_t)(model->meas));
            model->i_acc = (int16_t)((int64_t)(model->i_acc) + (int64_t)(model->err));
            if (model->i_acc > model->imax) {
                model->i_acc = model->imax;
            }
            if (model->i_acc < model->neg_imax) {
                model->i_acc = model->neg_imax;
            }
            model->deriv = (int16_t)((int64_t)(model->err) - (int64_t)(model->err_prev));
            model->ctrl = (int16_t)((int64_t)((int16_t)((int64_t)((int16_t)(lam_q_mul((int64_t)(model->kp), (int64_t)(model->err), 8))) + (int64_t)((int16_t)(lam_q_mul((int64_t)(model->ki), (int64_t)(model->i_acc), 8))))) + (int64_t)((int16_t)(lam_q_mul((int64_t)(model->kd), (int64_t)(model->deriv), 8))));
            model->meas = (int16_t)((int64_t)(model->meas) + (int64_t)((int16_t)(lam_q_mul((int64_t)(model->kplant), (int64_t)(model->ctrl), 8))));
            model->err_prev = model->err;
            if (model->err < model->eps) {
                model->state = PID_REGULATOR_PID_SETTLED;
                break;
            }
            break;
        }
        case PID_REGULATOR_PID_DONE: {
            (*main->write_bit)(PID_REGULATOR_PID_READY, 1, main->userdata);
            model->state = PID_REGULATOR_PID_END;
            break;
        }
        case PID_REGULATOR_PID_SETTLED: {
            model->meas = model->target;
            model->state = PID_REGULATOR_PID_DONE;
            break;
            break;
        }
        case PID_REGULATOR_PID_END: {
            break;
        }
        default: break;
    }
}

/// Функция сброса модели Pid (PidRegulator:Pid)
void PidRegulatorPid_reset(PidRegulatorPid *model, PidRegulator *main) {
    PidRegulatorPid_init(model, main);
}

/// Функция проверки терминального состояния модели Pid (PidRegulator:Pid)
bool PidRegulatorPid_is_done(const PidRegulatorPid *model, PidRegulator *main) {
    return model->state == PID_REGULATOR_PID_END;
}

/// Функция инициализации модели pid_regulator (PidRegulator)
void PidRegulator_init(PidRegulator *model) {
    assert(0 != model);
    model->state = PID_REGULATOR_INIT;
    PidRegulatorPid_init(&model->main, model);
}

/// Функция обработки модели pid_regulator (PidRegulator)
void PidRegulator_tick(PidRegulator *model) {
    assert(0 != model);
    if (model->state == PID_REGULATOR_INIT) {
        model->state = PID_REGULATOR_MAIN;
    }
    switch (model->state) {
        case PID_REGULATOR_MAIN: {
            PidRegulatorPid_tick(&model->main, model);
            if (PidRegulatorPid_is_done(&model->main, model)) {
                model->state = PID_REGULATOR_END;
                break;
            }
            break;
        }
        case PID_REGULATOR_END: {
            break;
        }
        default: break;
    }
}

/// Функция сброса модели pid_regulator (PidRegulator)
void PidRegulator_reset(PidRegulator *model) {
    PidRegulator_init(model);
}

/// Функция проверки терминального состояния модели pid_regulator (PidRegulator)
bool PidRegulator_is_done(const PidRegulator *model) {
    return model->state == PID_REGULATOR_END;
}

