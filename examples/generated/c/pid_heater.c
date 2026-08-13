#include "pid_heater.h"
#include <assert.h>
#include <math.h>
/// Model functions 'Heater (PidHeater:Heater)'
static void PidHeaterHeater_init(PidHeaterHeater *model, PidHeater *main);
static void PidHeaterHeater_tick(PidHeaterHeater *model, PidHeater *main);
static bool PidHeaterHeater_is_done(const PidHeaterHeater *model, PidHeater *main);

/// Функция инициализации модели Heater (PidHeater:Heater)
void PidHeaterHeater_init(PidHeaterHeater *model, PidHeater *main) {
    assert(0 != model);
    model->state = PID_HEATER_HEATER_INIT;
    model->err = 0.0;
    model->i_new = 0.0;
    model->loop_pid = (PidState){0.5, 0.0625, 0.25, 1.0, 0.0, 32.0, 0.0, 0.0, 0.0};
    model->raw = 0.0;
    model->release = 38.0;
    model->setpoint = 40.0;
    (*main->write_float)(PID_HEATER_HEATER_PORT_TEMPERATURE, 0.0, main->userdata);
}

/// Функция обработки модели Heater (PidHeater:Heater)
void PidHeaterHeater_tick(PidHeaterHeater *model, PidHeater *main) {
    assert(0 != model);
    assert(0 != main);
    if (model->state == PID_HEATER_HEATER_INIT) {
        main->target = model->setpoint;
        model->state = PID_HEATER_HEATER_HEATING;
    }
    switch (model->state) {
        case PID_HEATER_HEATER_DONE: {
            model->state = PID_HEATER_HEATER_END;
            break;
        }
        case PID_HEATER_HEATER_HEATING: {
            model->err = main->target - main->meas;
            model->i_new = model->loop_pid.i_acc + model->loop_pid.ki * model->err * model->loop_pid.ts;
            model->raw = model->loop_pid.kp * model->err + model->i_new + model->loop_pid.kd * (model->err - model->loop_pid.err_prev) / model->loop_pid.ts;
            if (model->raw > model->loop_pid.out_max) {
                main->ctrl = model->loop_pid.out_max;
                if (model->err <= 0.0) {
                    model->loop_pid.i_acc = model->i_new;
                }
            } else if (model->raw < model->loop_pid.out_min) {
                main->ctrl = model->loop_pid.out_min;
                if (model->err >= 0.0) {
                    model->loop_pid.i_acc = model->i_new;
                }
            } else {
                main->ctrl = model->raw;
                model->loop_pid.i_acc = model->i_new;
            }
            model->loop_pid.err_prev = model->err;
            model->loop_pid.output = main->ctrl;
            main->meas = main->meas + main->gain * main->ctrl - main->loss * (main->meas - main->ambient);
            (*main->write_float)(PID_HEATER_HEATER_PORT_TEMPERATURE, main->meas, main->userdata);
            if (model->err <= 0.0) {
                main->ctrl = 0.0;
                model->state = PID_HEATER_HEATER_HOLDING;
                break;
            }
            break;
        }
        case PID_HEATER_HEATER_HOLDING: {
            main->meas = main->meas - main->loss * (main->meas - main->ambient);
            (*main->write_float)(PID_HEATER_HEATER_PORT_TEMPERATURE, main->meas, main->userdata);
            if (main->meas <= model->release) {
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

/// Функция инициализации модели pid_heater (PidHeater)
void PidHeater_init(PidHeater *model) {
    assert(0 != model);
    model->state = PID_HEATER_INIT;
    PidHeaterHeater_init(&model->pid_heater_heater0, model);
    model->pid_heater_state = PID_HEATER_PID_HEATER_HEATER0;
    model->ambient = 18.0;
    model->ctrl = 0.0;
    model->gain = 0.5;
    model->loss = 0.05;
    model->meas = 25.0;
    model->target = 80.0;
}

/// Функция обработки модели pid_heater (PidHeater)
void PidHeater_tick(PidHeater *model) {
    assert(0 != model);
    if (model->state == PID_HEATER_INIT) {
        model->state = PID_HEATER_PID_HEATER;
    }
    switch (model->state) {
        case PID_HEATER_FINISHED: {
            model->state = PID_HEATER_END;
            break;
        }
        case PID_HEATER_PID_HEATER: {
            if (model->pid_heater_state == PID_HEATER_PID_HEATER_HEATER0) {
                PidHeaterHeater_tick(&model->pid_heater_heater0, model);
                if (PidHeaterHeater_is_done(&model->pid_heater_heater0, model)) {
                    PidHeaterHeater_init(&model->pid_heater_heater1, model);
                    model->pid_heater_heater1.setpoint = 55.0;
                    model->pid_heater_heater1.release = 52.0;
                    model->pid_heater_state = PID_HEATER_PID_HEATER_HEATER1;
                    break;
                }
            } else if (model->pid_heater_state == PID_HEATER_PID_HEATER_HEATER1) {
                PidHeaterHeater_tick(&model->pid_heater_heater1, model);
                if (PidHeaterHeater_is_done(&model->pid_heater_heater1, model)) {
                    model->state = PID_HEATER_FINISHED;
                    break;
                }
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

