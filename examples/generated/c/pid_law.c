#include "pid_law.h"
#include <assert.h>
#include <math.h>
/// Model functions 'Pid (PidLaw:Pid)'
static void PidLawPid_init(PidLawPid *model, PidLaw *main);
static void PidLawPid_tick(PidLawPid *model, PidLaw *main);
static bool PidLawPid_is_done(const PidLawPid *model, PidLaw *main);

/// Функция инициализации модели Pid (PidLaw:Pid)
void PidLawPid_init(PidLawPid *model, PidLaw *main) {
    assert(0 != model);
    model->state = PID_LAW_PID_INIT;
    model->deriv = 0.0;
    model->eps = 0.5;
    model->err = 0.0;
    model->err_prev = 0.0;
    model->i_acc = 0.0;
    model->imax = 32.0;
    model->kd = 0.25;
    model->ki = 0.0625;
    model->kp = 0.5;
    model->neg_imax = -32.0;
}

/// Функция обработки модели Pid (PidLaw:Pid)
void PidLawPid_tick(PidLawPid *model, PidLaw *main) {
    assert(0 != model);
    assert(0 != main);
    if (model->state == PID_LAW_PID_INIT) {
        model->neg_imax = 0.0 - model->imax;
        model->state = PID_LAW_PID_CONTROL;
    }
    switch (model->state) {
        case PID_LAW_PID_CONTROL: {
            model->err = main->target - main->meas;
            model->i_acc = model->i_acc + model->err;
            if (model->i_acc > model->imax) {
                model->i_acc = model->imax;
            }
            if (model->i_acc < model->neg_imax) {
                model->i_acc = model->neg_imax;
            }
            model->deriv = model->err - model->err_prev;
            main->ctrl = model->kp * model->err + model->ki * model->i_acc + model->kd * model->deriv;
            model->err_prev = model->err;
            if (model->err < model->eps) {
                main->ctrl = 0.0;
                (*main->write_bit)(PID_LAW_PID_READY, 1, main->userdata);
                model->state = PID_LAW_PID_SETTLED;
                break;
            }
            break;
        }
        case PID_LAW_PID_DONE: {
            model->state = PID_LAW_PID_END;
            break;
        }
        case PID_LAW_PID_SETTLED: {
            model->state = PID_LAW_PID_DONE;
            break;
            break;
        }
        case PID_LAW_PID_END: {
            break;
        }
        default: break;
    }
}

/// Функция сброса модели Pid (PidLaw:Pid)
void PidLawPid_reset(PidLawPid *model, PidLaw *main) {
    PidLawPid_init(model, main);
}

/// Функция проверки терминального состояния модели Pid (PidLaw:Pid)
bool PidLawPid_is_done(const PidLawPid *model, PidLaw *main) {
    return model->state == PID_LAW_PID_END;
}

/// Функция инициализации модели pid_law (PidLaw)
void PidLaw_init(PidLaw *model) {
    assert(0 != model);
    model->state = PID_LAW_INIT;
    PidLawPid_init(&model->main, model);
    model->ctrl = 0.0;
    model->meas = 0.0;
    model->target = 40.0;
}

/// Функция обработки модели pid_law (PidLaw)
void PidLaw_tick(PidLaw *model) {
    assert(0 != model);
    if (model->state == PID_LAW_INIT) {
        model->state = PID_LAW_MAIN;
    }
    switch (model->state) {
        case PID_LAW_MAIN: {
            PidLawPid_tick(&model->main, model);
            if (PidLawPid_is_done(&model->main, model)) {
                model->state = PID_LAW_END;
                break;
            }
            break;
        }
        case PID_LAW_END: {
            break;
        }
        default: break;
    }
}

/// Функция сброса модели pid_law (PidLaw)
void PidLaw_reset(PidLaw *model) {
    PidLaw_init(model);
}

/// Функция проверки терминального состояния модели pid_law (PidLaw)
bool PidLaw_is_done(const PidLaw *model) {
    return model->state == PID_LAW_END;
}

