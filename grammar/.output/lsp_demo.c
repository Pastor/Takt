#include "lsp_demo.h"
#include <assert.h>
#include <math.h>
/// Перечисления модели lsp_demo (LspDemo)
#define ENUM_LSP_DEMO_EAST 2
#define ENUM_LSP_DEMO_HIGH 10
#define ENUM_LSP_DEMO_LOW 0
#define ENUM_LSP_DEMO_MEDIUM 5
#define ENUM_LSP_DEMO_NORTH 0
#define ENUM_LSP_DEMO_SOUTH 1
#define ENUM_LSP_DEMO_WEST 3
/// Model functions 'Robot (LspDemo:Robot)'
static void LspDemoRobot_init(LspDemoRobot *model, LspDemo *main);
static void LspDemoRobot_tick(LspDemoRobot *model, LspDemo *main);
static bool LspDemoRobot_is_done(const LspDemoRobot *model, LspDemo *main);

///Внешние функции
extern void log(uint8_t msg);
/// Функция инициализации модели Robot (LspDemo:Robot)
void LspDemoRobot_init(LspDemoRobot *model, LspDemo *main) {
    assert(0 != model);
    model->state = LSP_DEMO_ROBOT_INIT;
    model->speed = 0;
    model->active = false;
}

/// Функция обработки модели Robot (LspDemo:Robot)
void LspDemoRobot_tick(LspDemoRobot *model, LspDemo *main) {
    assert(0 != model);
    assert(0 != main);
    switch (model->state) {
        case LSP_DEMO_ROBOT_INIT: {
            model->speed = 0;
            model->active = false;
            model->state = LSP_DEMO_ROBOT_IDLE;
            break;
        }
        case LSP_DEMO_ROBOT_MOVING: {
            model->speed = 100;
            if (true) {
                model->speed = 0;
                model->active = false;
                model->state = LSP_DEMO_ROBOT_IDLE;
                break;
            }
            break;
        }
        case LSP_DEMO_ROBOT_IDLE: {
            if (model->active) {
                model->state = LSP_DEMO_ROBOT_MOVING;
                break;
            }
            break;
        }
        case LSP_DEMO_ROBOT_END: {
            break;
        }
    }
}

/// Функция сброса модели Robot (LspDemo:Robot)
void LspDemoRobot_reset(LspDemoRobot *model, LspDemo *main) {
    LspDemoRobot_init(model, main);
}

/// Функция проверки терминального состояния модели Robot (LspDemo:Robot)
bool LspDemoRobot_is_done(const LspDemoRobot *model, LspDemo *main) {
    return model->state == LSP_DEMO_ROBOT_END;
}

/// Функция инициализации модели lsp_demo (LspDemo)
void LspDemo_init(LspDemo *model) {
    assert(0 != model);
    model->state = LSP_DEMO_INIT;
    model->heading = 0;
}

/// Функция обработки модели lsp_demo (LspDemo)
void LspDemo_tick(LspDemo *model) {
    assert(0 != model);
    switch (model->state) {
        case LSP_DEMO_INIT: {
            LspDemoRobot_init(&model->main, model);
            model->state = LSP_DEMO_MAIN;
            break;
        }
        case LSP_DEMO_MAIN: {
            LspDemoRobot_tick(&model->main, model);
            if (LspDemoRobot_is_done(&model->main, model)) {
                model->state = LSP_DEMO_END;
                break;
            }
            break;
        }
        case LSP_DEMO_END: {
            break;
        }
    }
}

/// Функция сброса модели lsp_demo (LspDemo)
void LspDemo_reset(LspDemo *model) {
    LspDemo_init(model);
}

/// Функция проверки терминального состояния модели lsp_demo (LspDemo)
bool LspDemo_is_done(const LspDemo *model) {
    return model->state == LSP_DEMO_END;
}

