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
            ///FIXME: Пока не реализовано
            break;
        }
        case THIS_IS_MY_MODEL_PONG_BEGIN: {
            ///FIXME: Пока не реализовано
            break;
        }
        case THIS_IS_MY_MODEL_PONG_STOP: {
            ///FIXME: Пока не реализовано
            break;
        }
        case THIS_IS_MY_MODEL_PONG_END: {
            ///FIXME: Пока не реализовано
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
    return model->state == THIS_IS_MY_MODEL_PONG_STOP;
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
            ///FIXME: Пока не реализовано
            break;
        }
        case THIS_IS_MY_MODEL_TOGGLE_PONG: {
            ///FIXME: Пока не реализовано
            break;
        }
        case THIS_IS_MY_MODEL_TOGGLE_ENTRY: {
            ///FIXME: Пока не реализовано
            break;
        }
        case THIS_IS_MY_MODEL_TOGGLE_COMPLETE: {
            ///FIXME: Пока не реализовано
            break;
        }
        case THIS_IS_MY_MODEL_TOGGLE_PING: {
            ///FIXME: Пока не реализовано
            break;
        }
        case THIS_IS_MY_MODEL_TOGGLE_END: {
            ///FIXME: Пока не реализовано
            break;
        }
        case THIS_IS_MY_MODEL_TOGGLE_END: {
            ///FIXME: Пока не реализовано
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

/// Функция инициализации модели Ping (ThisIsMyModel:Ping)
void ThisIsMyModelPing_init(ThisIsMyModelPing *model, const ThisIsMyModel *main) {
    assert(0 != model);
    model->state = THIS_IS_MY_MODEL_PING_INIT;
    /// model->toggle = ?;
}

/// Функция обработки модели Ping (ThisIsMyModel:Ping)
void ThisIsMyModelPing_tick(ThisIsMyModelPing *model, const ThisIsMyModel *main) {
    assert(0 != model);
    assert(0 != main);
    switch (model->state) {
        case THIS_IS_MY_MODEL_PING_INIT: {
            ///FIXME: Пока не реализовано
            break;
        }
        case THIS_IS_MY_MODEL_PING_END: {
            ///FIXME: Пока не реализовано
            break;
        }
        case THIS_IS_MY_MODEL_PING_START: {
            ///FIXME: Пока не реализовано
            break;
        }
        case THIS_IS_MY_MODEL_PING_END: {
            ///FIXME: Пока не реализовано
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

/// Функция инициализации модели ThisIsMyModel (ThisIsMyModel)
void ThisIsMyModel_init(ThisIsMyModel *model) {
    assert(0 != model);
    model->state = THIS_IS_MY_MODEL_INIT;
    /// model->it = ?;
}

/// Функция обработки модели ThisIsMyModel (ThisIsMyModel)
void ThisIsMyModel_tick(ThisIsMyModel *model) {
    assert(0 != model);
    switch (model->state) {
        case THIS_IS_MY_MODEL_INIT: {
            ///FIXME: Пока не реализовано
            break;
        }
        case THIS_IS_MY_MODEL_ENTRY: {
            ///FIXME: Пока не реализовано
            break;
        }
        case THIS_IS_MY_MODEL_END: {
            ///FIXME: Пока не реализовано
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
    return model->state == THIS_IS_MY_MODEL_ENTRY;
}

