#include "comprehensive.h"
#include <assert.h>
#include <math.h>
/// Перечисления модели comprehensive (Comprehensive)
#define ENUM_COMPREHENSIVE_AUTO 0
#define ENUM_COMPREHENSIVE_EMERGENCY 2
#define ENUM_COMPREHENSIVE_MANUAL 1
/// Константы и порты модели Controller (Comprehensive:Controller)
#define CONST_COMPREHENSIVE_CONTROLLER_MAX_COUNT 10
#define CONST_COMPREHENSIVE_CONTROLLER_MAX_TEMP 100
/// Model functions 'Controller (Comprehensive:Controller)'
static void ComprehensiveController_init(ComprehensiveController *model, const Comprehensive *main);
static void ComprehensiveController_tick(ComprehensiveController *model, const Comprehensive *main);
static bool ComprehensiveController_is_done(const ComprehensiveController *model, const Comprehensive *main);

///Внешние функции
extern void log_count(uint8_t n);
extern void log_temp(uint8_t value);
///Функции моделей
static uint8_t ComprehensiveController_clamp_temp(Comprehensive *main, uint8_t value) {
    if (value > CONST_COMPREHENSIVE_CONTROLLER_MAX_TEMP) {
        return CONST_COMPREHENSIVE_CONTROLLER_MAX_TEMP;
    }
    if (value < model->entry.temperature) {
        return model->entry.temperature;
    }
    return value;
}

static uint8_t ComprehensiveController_increment(Comprehensive *main, uint8_t n) {
    return n + 1;
}

/// Функция инициализации модели Controller (Comprehensive:Controller)
void ComprehensiveController_init(ComprehensiveController *model, const Comprehensive *main) {
    assert(0 != model);
    model->state = COMPREHENSIVE_CONTROLLER_INIT;
    model->temperature = 0;
    model->count = 0;
}

/// Функция обработки модели Controller (Comprehensive:Controller)
void ComprehensiveController_tick(ComprehensiveController *model, const Comprehensive *main) {
    assert(0 != model);
    assert(0 != main);
    switch (model->state) {
        case COMPREHENSIVE_CONTROLLER_INIT: {
            //FIXME: Пока не реализовано до конца
            model->entry.count = 0;
            model->entry.temperature = 0;
            model->state = COMPREHENSIVE_CONTROLLER_IDLE;
            break;
        }
        case COMPREHENSIVE_CONTROLLER_IDLE: {
            //FIXME: Пока не реализовано
            break;
        }
        case COMPREHENSIVE_CONTROLLER_COOLING: {
            //FIXME: Пока не реализовано
            break;
        }
        case COMPREHENSIVE_CONTROLLER_DONE: {
            //FIXME: Пока не реализовано
            break;
        }
        case COMPREHENSIVE_CONTROLLER_HEATING: {
            //FIXME: Пока не реализовано
            break;
        }
        case COMPREHENSIVE_CONTROLLER_END: {
            ///FIXME: Пока не реализовано
            break;
        }
    }
}

/// Функция сброса модели Controller (Comprehensive:Controller)
void ComprehensiveController_reset(ComprehensiveController *model, const Comprehensive *main) {
    ComprehensiveController_init(model, main);
}

/// Функция проверки терминального состояния модели Controller (Comprehensive:Controller)
bool ComprehensiveController_is_done(const ComprehensiveController *model, const Comprehensive *main) {
    return model->state == COMPREHENSIVE_CONTROLLER_DONE;
}

/// Функция инициализации модели comprehensive (Comprehensive)
void Comprehensive_init(Comprehensive *model) {
    assert(0 != model);
    model->state = COMPREHENSIVE_INIT;
}

/// Функция обработки модели comprehensive (Comprehensive)
void Comprehensive_tick(Comprehensive *model) {
    assert(0 != model);
    switch (model->state) {
        case COMPREHENSIVE_INIT: {
            //FIXME: Пока не реализовано до конца
            ComprehensiveController_init(&model->entry);
            model->state = COMPREHENSIVE_ENTRY;
            break;
        }
        case COMPREHENSIVE_ENTRY: {
            //FIXME: Пока не реализовано
            break;
        }
        case COMPREHENSIVE_END: {
            ///FIXME: Пока не реализовано
            break;
        }
    }
}

/// Функция сброса модели comprehensive (Comprehensive)
void Comprehensive_reset(Comprehensive *model) {
    Comprehensive_init(model);
}

/// Функция проверки терминального состояния модели comprehensive (Comprehensive)
bool Comprehensive_is_done(const Comprehensive *model) {
    return model->state == COMPREHENSIVE_ENTRY;
}

