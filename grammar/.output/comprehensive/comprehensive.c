#include "comprehensive.h"
#include <math.h>
/// Перечисления модели comprehensive (Comprehensive)
#define ENUM_COMPREHENSIVE_AUTO 0
#define ENUM_COMPREHENSIVE_EMERGENCY 2
#define ENUM_COMPREHENSIVE_MANUAL 1
/// Константы и порты модели Controller (Comprehensive:Controller)
#define CONST_COMPREHENSIVE_CONTROLLER_MAX_COUNT 10
#define CONST_COMPREHENSIVE_CONTROLLER_MAX_TEMP 100
///Внешние функции
extern void log_count(uint8_t n);
extern void log_temp(uint8_t value);
///Функции моделей
static uint8_t ComprehensiveController_clamp_temp(Comprehensive *main, uint8_t value) {
    if (value > CONST_COMPREHENSIVE_CONTROLLER_MAX_TEMP) {
        return CONST_COMPREHENSIVE_CONTROLLER_MAX_TEMP;
    }
    if (value < main->entry.temperature) {
        return main->entry.temperature;
    }
    return value;
}

static uint8_t ComprehensiveController_increment(Comprehensive *main, uint8_t n) {
    return n + 1;
}


void Comprehensive_init(Comprehensive *main) {
    main->state = COMPREHENSIVE_INIT;
}

void Comprehensive_tick(Comprehensive *main) {
}

void Comprehensive_reset(Comprehensive *main) {
    Comprehensive_init(main);
}

bool Comprehensive_is_done(const Comprehensive *main) {
    return main->state == COMPREHENSIVE_ENTRY;
}

