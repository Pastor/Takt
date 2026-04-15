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
static void ComprehensiveController_init(ComprehensiveController *model, Comprehensive *main);
static void ComprehensiveController_tick(ComprehensiveController *model, Comprehensive *main);
static bool ComprehensiveController_is_done(const ComprehensiveController *model, Comprehensive *main);

///Внешние функции
extern void log_count(uint8_t n);
extern void log_temp(uint8_t value);
///Функции моделей
static uint8_t ComprehensiveController_clamp_temp(const Comprehensive *model, uint8_t value) {
    if (value > CONST_COMPREHENSIVE_CONTROLLER_MAX_TEMP) {
        return CONST_COMPREHENSIVE_CONTROLLER_MAX_TEMP;
    }
    if (value < model->entry.temperature) {
        return model->entry.temperature;
    }
    return value;
}

static uint8_t ComprehensiveController_increment(const Comprehensive *model, uint8_t n) {
    return n + 1;
}

/// Функция инициализации модели Controller (Comprehensive:Controller)
void ComprehensiveController_init(ComprehensiveController *model, Comprehensive *main) {
    assert(0 != model);
    model->state = COMPREHENSIVE_CONTROLLER_INIT;
    model->count = 0;
    model->temperature = 0;
}

/// Функция обработки модели Controller (Comprehensive:Controller)
void ComprehensiveController_tick(ComprehensiveController *model, Comprehensive *main) {
    assert(0 != model);
    assert(0 != main);
    switch (model->state) {
        case COMPREHENSIVE_CONTROLLER_INIT: {
            model->count = 0;
            model->temperature = 0;
            model->state = COMPREHENSIVE_CONTROLLER_IDLE;
            break;
        }
        case COMPREHENSIVE_CONTROLLER_DONE: {
            model->state = COMPREHENSIVE_CONTROLLER_END;
            break;
        }
        case COMPREHENSIVE_CONTROLLER_IDLE: {
            uint8_t delta = 1;
            model->temperature = model->temperature + delta;
            log_temp(model->temperature);
            if (model->temperature > 10) {
                model->count = ComprehensiveController_increment(main, model->count);
                uint8_t boost = 5;
                model->temperature = model->temperature + boost;
                model->state = COMPREHENSIVE_CONTROLLER_HEATING;
                break;
            }
            break;
        }
        case COMPREHENSIVE_CONTROLLER_COOLING: {
            {
                uint8_t i = 0;
                for (; i < 3; i = i + 1)                 model->temperature = model->temperature - 1;

            }
            if (model->temperature < 0) {
                model->temperature = 0;
            }
            log_count(model->count);
            if (model->temperature <= 0) {
                model->temperature = 0;
                model->count = 0;
                model->state = COMPREHENSIVE_CONTROLLER_DONE;
                break;
            }
            if (model->temperature > 0) {
                model->count = 0;
                model->temperature = 0;
                model->state = COMPREHENSIVE_CONTROLLER_IDLE;
                break;
            }
            break;
        }
        case COMPREHENSIVE_CONTROLLER_HEATING: {
            while (model->count < 3)             model->count = model->count + 1;

            if (model->temperature > CONST_COMPREHENSIVE_CONTROLLER_MAX_TEMP) {
                model->state = COMPREHENSIVE_CONTROLLER_COOLING;
                break;
            }
            if (model->count >= CONST_COMPREHENSIVE_CONTROLLER_MAX_COUNT) {
                model->count = 0;
                model->temperature = 0;
                model->state = COMPREHENSIVE_CONTROLLER_IDLE;
                break;
            }
            break;
        }
        case COMPREHENSIVE_CONTROLLER_END: {
            break;
        }
    }
}

/// Функция сброса модели Controller (Comprehensive:Controller)
void ComprehensiveController_reset(ComprehensiveController *model, Comprehensive *main) {
    ComprehensiveController_init(model, main);
}

/// Функция проверки терминального состояния модели Controller (Comprehensive:Controller)
bool ComprehensiveController_is_done(const ComprehensiveController *model, Comprehensive *main) {
    return model->state == COMPREHENSIVE_CONTROLLER_END;
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
            ComprehensiveController_init(&model->entry, model);
            model->state = COMPREHENSIVE_ENTRY;
            break;
        }
        case COMPREHENSIVE_ENTRY: {
            ComprehensiveController_tick(&model->entry, model);
            if (ComprehensiveController_is_done(&model->entry, model)) {
                model->state = COMPREHENSIVE_END;
                break;
            }
            break;
        }
        case COMPREHENSIVE_END: {
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
    return model->state == COMPREHENSIVE_END;
}

