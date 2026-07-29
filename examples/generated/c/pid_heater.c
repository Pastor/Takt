#include "pid_heater.h"
#include <assert.h>
#include <math.h>
/// Model functions 'Heater (PidHeater:Heater)'
static void PidHeaterHeater_init(PidHeaterHeater *model, PidHeater *main);
static void PidHeaterHeater_tick(PidHeaterHeater *model, PidHeater *main);
static bool PidHeaterHeater_is_done(const PidHeaterHeater *model, PidHeater *main);
/// Model functions 'Pid (PidHeater:Pid)'
static void PidHeaterPid_init(PidHeaterPid *model, PidHeater *main);
static void PidHeaterPid_tick(PidHeaterPid *model, PidHeater *main);
static bool PidHeaterPid_is_done(const PidHeaterPid *model, PidHeater *main);

/// Функция инициализации модели Heater (PidHeater:Heater)
void PidHeaterHeater_init(PidHeaterHeater *model, PidHeater *main) {
    assert(0 != model);
    model->state = PID_HEATER_HEATER_INIT;
}

/// Функция обработки модели Heater (PidHeater:Heater)
void PidHeaterHeater_tick(PidHeaterHeater *model, PidHeater *main) {
    assert(0 != model);
    assert(0 != main);
    if (model->state == PID_HEATER_HEATER_INIT) {
        model->state = PID_HEATER_HEATER_HEATING;
    }
    switch (model->state) {
        case PID_HEATER_HEATER_DONE: {
            model->state = PID_HEATER_HEATER_END;
            break;
        }
        case PID_HEATER_HEATER_HEATING: {
            main->meas = main->meas + main->gain * main->ctrl - main->loss * (main->meas - main->ambient);
            (*main->write_float)(PID_HEATER_HEATER_TEMPERATURE, main->meas, main->userdata);
            if (main->ctrl <= 0.0) {
                model->state = PID_HEATER_HEATER_HOLDING;
                break;
            }
            break;
        }
        case PID_HEATER_HEATER_HOLDING: {
            main->meas = main->meas - main->loss * (main->meas - main->ambient);
            (*main->write_float)(PID_HEATER_HEATER_TEMPERATURE, main->meas, main->userdata);
            if (main->meas <= main->release) {
                model->state = PID_HEATER_HEATER_DONE;
                break;
            }
            break;
        }
        case PID_HEATER_HEATER_END: {
            break;
        }
        default: break;
    }
}

/// Функция сброса модели Heater (PidHeater:Heater)
void PidHeaterHeater_reset(PidHeaterHeater *model, PidHeater *main) {
    PidHeaterHeater_init(model, main);
}

/// Функция проверки терминального состояния модели Heater (PidHeater:Heater)
bool PidHeaterHeater_is_done(const PidHeaterHeater *model, PidHeater *main) {
    return model->state == PID_HEATER_HEATER_END;
}

/// Функция инициализации модели Pid (PidHeater:Pid)
void PidHeaterPid_init(PidHeaterPid *model, PidHeater *main) {
    assert(0 != model);
    model->state = PID_HEATER_PID_INIT;
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

/// Функция обработки модели Pid (PidHeater:Pid)
void PidHeaterPid_tick(PidHeaterPid *model, PidHeater *main) {
    assert(0 != model);
    assert(0 != main);
    if (model->state == PID_HEATER_PID_INIT) {
        model->state = PID_HEATER_PID_CONTROL;
    }
    switch (model->state) {
        case PID_HEATER_PID_CONTROL: {
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
                (*main->write_bit)(PID_HEATER_PID_READY, 1, main->userdata);
                model->state = PID_HEATER_PID_SETTLED;
                break;
            }
            break;
        }
        case PID_HEATER_PID_DONE: {
            model->state = PID_HEATER_PID_END;
            break;
        }
        case PID_HEATER_PID_SETTLED: {
            model->state = PID_HEATER_PID_DONE;
            break;
            break;
        }
        case PID_HEATER_PID_END: {
            break;
        }
        default: break;
    }
}

/// Функция сброса модели Pid (PidHeater:Pid)
void PidHeaterPid_reset(PidHeaterPid *model, PidHeater *main) {
    PidHeaterPid_init(model, main);
}

/// Функция проверки терминального состояния модели Pid (PidHeater:Pid)
bool PidHeaterPid_is_done(const PidHeaterPid *model, PidHeater *main) {
    return model->state == PID_HEATER_PID_END;
}

/// Функция инициализации модели pid_heater (PidHeater)
void PidHeater_init(PidHeater *model) {
    assert(0 != model);
    model->state = PID_HEATER_INIT;
    PidHeaterPid_init(&model->pid_heater.pid0, model);
    PidHeaterHeater_init(&model->pid_heater.heater1, model);
    model->pid_heater.state = PID_HEATER_PID_HEATER_INIT;
    model->ambient = 18.0;
    model->ctrl = 0.0;
    model->gain = 0.5;
    model->loss = 0.05;
    model->meas = 0.0;
    model->release = 38.0;
    model->target = 40.0;
}

/// Функция обработки модели pid_heater (PidHeater)
void PidHeater_tick(PidHeater *model) {
    assert(0 != model);
    if (model->state == PID_HEATER_INIT) {
        model->state = PID_HEATER_PID_HEATER;
    }
    switch (model->state) {
        case PID_HEATER_PID_HEATER: {
            PidHeaterPid_tick(&model->pid_heater.pid0, model);
            PidHeaterHeater_tick(&model->pid_heater.heater1, model);
            if (PidHeaterPid_is_done(&model->pid_heater.pid0, model) && PidHeaterHeater_is_done(&model->pid_heater.heater1, model)) {
                model->state = PID_HEATER_END;
                break;
            }
            break;
        }
        case PID_HEATER_END: {
            break;
        }
        default: break;
    }
}

/// Функция сброса модели pid_heater (PidHeater)
void PidHeater_reset(PidHeater *model) {
    PidHeater_init(model);
}

/// Функция проверки терминального состояния модели pid_heater (PidHeater)
bool PidHeater_is_done(const PidHeater *model) {
    return model->state == PID_HEATER_END;
}

