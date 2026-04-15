#include "this_is_my_model.h"
#include <assert.h>
#include <math.h>
/// Константы и порты модели ThisIsMyModel (ThisIsMyModel)
#define CONST_THIS_IS_MY_MODEL_MATRIX {0, 0, 0, 0, 0, 0, 0, 0}
#define CONST_THIS_IS_MY_MODEL_NUMB 255
#define PORT_THIS_IS_MY_MODEL_A 0x548835
#define PORT_THIS_IS_MY_MODEL_B1 0x648835
/// Model functions 'Pong (ThisIsMyModel:Pong)'
static void ThisIsMyModelPong_init(ThisIsMyModelPong *model, const ThisIsMyModel *main);
static void ThisIsMyModelPong_tick(ThisIsMyModelPong *model, const ThisIsMyModel *main);
static bool ThisIsMyModelPong_is_done(const ThisIsMyModelPong *model, const ThisIsMyModel *main);
/// Model functions 'Ping (ThisIsMyModel:Ping)'
static void ThisIsMyModelPing_init(ThisIsMyModelPing *model, const ThisIsMyModel *main);
static void ThisIsMyModelPing_tick(ThisIsMyModelPing *model, const ThisIsMyModel *main);
static bool ThisIsMyModelPing_is_done(const ThisIsMyModelPing *model, const ThisIsMyModel *main);
/// Model functions 'Toggle (ThisIsMyModel:Toggle)'
static void ThisIsMyModelToggle_init(ThisIsMyModelToggle *model, const ThisIsMyModel *main);
static void ThisIsMyModelToggle_tick(ThisIsMyModelToggle *model, const ThisIsMyModel *main);
static bool ThisIsMyModelToggle_is_done(const ThisIsMyModelToggle *model, const ThisIsMyModel *main);

/// Функция инициализации модели Ping (ThisIsMyModel:Ping)
void ThisIsMyModelPing_init(ThisIsMyModelPing *model, const ThisIsMyModel *main) {
    assert(0 != model);
    model->state = THIS_IS_MY_MODEL_PING_INIT;
    model->toggle = false;
}

/// Функция обработки модели Ping (ThisIsMyModel:Ping)
void ThisIsMyModelPing_tick(ThisIsMyModelPing *model, const ThisIsMyModel *main) {
    assert(0 != model);
    assert(0 != main);
    switch (model->state) {
        case THIS_IS_MY_MODEL_PING_INIT: {
            (*main->write_bit)(PORT_THIS_IS_MY_MODEL_A, 0, true, main->userdata);
            (*main->write_bit)(PORT_THIS_IS_MY_MODEL_A, 1, false, main->userdata);
            model->state = THIS_IS_MY_MODEL_PING_START;
            break;
        }
        case THIS_IS_MY_MODEL_PING_START: {
            (*main->write_bit)(PORT_THIS_IS_MY_MODEL_A, 2, model->toggle, main->userdata);
            model->toggle = !model->toggle;
            if ((*main->read_bit)(PORT_THIS_IS_MY_MODEL_B1, 6, main->userdata)) {
                (*main->write_bit)(PORT_THIS_IS_MY_MODEL_A, 0, false, main->userdata);
                (*main->write_bit)(PORT_THIS_IS_MY_MODEL_A, 1, true, main->userdata);
                model->state = THIS_IS_MY_MODEL_PING_END;
                break;
            }
            break;
        }
        case THIS_IS_MY_MODEL_PING_END: {
            break;
        }
    }
}

/// Функция сброса модели Ping (ThisIsMyModel:Ping)
void ThisIsMyModelPing_reset(ThisIsMyModelPing *model, const ThisIsMyModel *main) {
    ThisIsMyModelPing_init(model, main);
}

/// Функция проверки терминального состояния модели Ping (ThisIsMyModel:Ping)
bool ThisIsMyModelPing_is_done(const ThisIsMyModelPing *model, const ThisIsMyModel *main) {
    return model->state == THIS_IS_MY_MODEL_PING_END;
}

/// Функция инициализации модели Toggle (ThisIsMyModel:Toggle)
void ThisIsMyModelToggle_init(ThisIsMyModelToggle *model, const ThisIsMyModel *main) {
    assert(0 != model);
    model->state = THIS_IS_MY_MODEL_TOGGLE_INIT;
}

/// Функция обработки модели Toggle (ThisIsMyModel:Toggle)
void ThisIsMyModelToggle_tick(ThisIsMyModelToggle *model, const ThisIsMyModel *main) {
    assert(0 != model);
    assert(0 != main);
    switch (model->state) {
        case THIS_IS_MY_MODEL_TOGGLE_INIT: {
            model->state = THIS_IS_MY_MODEL_TOGGLE_ENTRY;
            break;
        }
        case THIS_IS_MY_MODEL_TOGGLE_COMPLETE: {
            if (true) {
                model->state = THIS_IS_MY_MODEL_TOGGLE_END;
                break;
            }
            break;
        }
        case THIS_IS_MY_MODEL_TOGGLE_END: {
            break;
        }
        case THIS_IS_MY_MODEL_TOGGLE_ENTRY: {
            if (main->it == 0) {
                model->state = THIS_IS_MY_MODEL_TOGGLE_PING;
                break;
            }
            break;
        }
        case THIS_IS_MY_MODEL_TOGGLE_PING: {
            ThisIsMyModelPing_tick(&model->ping, main);
            if (ThisIsMyModelPing_is_done(&model->ping, main)) {
                model->state = THIS_IS_MY_MODEL_TOGGLE_PONG;
                break;
            }
            break;
        }
        case THIS_IS_MY_MODEL_TOGGLE_PONG: {
            ThisIsMyModelPong_tick(&model->pong, main);
            if (ThisIsMyModelPong_is_done(&model->pong, main)) {
                model->state = THIS_IS_MY_MODEL_TOGGLE_COMPLETE;
                break;
            }
            break;
        }
    }
}

/// Функция сброса модели Toggle (ThisIsMyModel:Toggle)
void ThisIsMyModelToggle_reset(ThisIsMyModelToggle *model, const ThisIsMyModel *main) {
    ThisIsMyModelToggle_init(model, main);
}

/// Функция проверки терминального состояния модели Toggle (ThisIsMyModel:Toggle)
bool ThisIsMyModelToggle_is_done(const ThisIsMyModelToggle *model, const ThisIsMyModel *main) {
    return model->state == THIS_IS_MY_MODEL_TOGGLE_END;
}

/// Функция инициализации модели Pong (ThisIsMyModel:Pong)
void ThisIsMyModelPong_init(ThisIsMyModelPong *model, const ThisIsMyModel *main) {
    assert(0 != model);
    model->state = THIS_IS_MY_MODEL_PONG_INIT;
}

/// Функция обработки модели Pong (ThisIsMyModel:Pong)
void ThisIsMyModelPong_tick(ThisIsMyModelPong *model, const ThisIsMyModel *main) {
    assert(0 != model);
    assert(0 != main);
    switch (model->state) {
        case THIS_IS_MY_MODEL_PONG_INIT: {
            model->state = THIS_IS_MY_MODEL_PONG_BEGIN;
            break;
        }
        case THIS_IS_MY_MODEL_PONG_STOP: {
            model->state = THIS_IS_MY_MODEL_PONG_END;
            break;
        }
        case THIS_IS_MY_MODEL_PONG_BEGIN: {
            (*main->write_bit)(PORT_THIS_IS_MY_MODEL_A, 5, ((CONST_THIS_IS_MY_MODEL_MATRIX >> 5) & 1u), main->userdata);
            //TODO: условный переход в Stop не поддерживается
            break;
        }
        case THIS_IS_MY_MODEL_PONG_END: {
            break;
        }
    }
}

/// Функция сброса модели Pong (ThisIsMyModel:Pong)
void ThisIsMyModelPong_reset(ThisIsMyModelPong *model, const ThisIsMyModel *main) {
    ThisIsMyModelPong_init(model, main);
}

/// Функция проверки терминального состояния модели Pong (ThisIsMyModel:Pong)
bool ThisIsMyModelPong_is_done(const ThisIsMyModelPong *model, const ThisIsMyModel *main) {
    return model->state == THIS_IS_MY_MODEL_PONG_END;
}

/// Функция инициализации модели ThisIsMyModel (ThisIsMyModel)
void ThisIsMyModel_init(ThisIsMyModel *model) {
    assert(0 != model);
    model->state = THIS_IS_MY_MODEL_INIT;
    model->it = 0;
}

/// Функция обработки модели ThisIsMyModel (ThisIsMyModel)
void ThisIsMyModel_tick(ThisIsMyModel *model) {
    assert(0 != model);
    switch (model->state) {
        case THIS_IS_MY_MODEL_INIT: {
            ThisIsMyModelPing_init(&model->entry_parallel0.ping0, model);
            ThisIsMyModelPong_init(&model->entry_parallel0.pong1, model);
            model->entry_parallel0.state = THIS_IS_MY_MODEL_ENTRY_PARALLEL0_INIT;
            model->entry_state = THIS_IS_MY_MODEL_ENTRY_PARALLEL0;
            model->state = THIS_IS_MY_MODEL_ENTRY;
            break;
        }
        case THIS_IS_MY_MODEL_ENTRY: {
            if (model->entry_state == THIS_IS_MY_MODEL_ENTRY_PARALLEL0) {
                ThisIsMyModelPing_tick(&model->entry_parallel0.ping0, model);
                ThisIsMyModelPong_tick(&model->entry_parallel0.pong1, model);
                if (ThisIsMyModelPing_is_done(&model->entry_parallel0.ping0, model) && ThisIsMyModelPong_is_done(&model->entry_parallel0.pong1, model)) {
                    ThisIsMyModelToggle_init(&model->entry_toggle1, model);
                    model->entry_state = THIS_IS_MY_MODEL_ENTRY_TOGGLE1;
                    break;
                }
            } else if (model->entry_state == THIS_IS_MY_MODEL_ENTRY_TOGGLE1) {
                ThisIsMyModelToggle_tick(&model->entry_toggle1, model);
                if (ThisIsMyModelToggle_is_done(&model->entry_toggle1, model)) {
                    model->state = THIS_IS_MY_MODEL_END;
                    break;
                }
            }
            break;
        }
        case THIS_IS_MY_MODEL_END: {
            break;
        }
    }
}

/// Функция сброса модели ThisIsMyModel (ThisIsMyModel)
void ThisIsMyModel_reset(ThisIsMyModel *model) {
    ThisIsMyModel_init(model);
}

/// Функция проверки терминального состояния модели ThisIsMyModel (ThisIsMyModel)
bool ThisIsMyModel_is_done(const ThisIsMyModel *model) {
    return model->state == THIS_IS_MY_MODEL_END;
}

