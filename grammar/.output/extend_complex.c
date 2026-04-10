#include "extend_complex.h"
/// Константы и порты модели extend_complex (ExtendComplex)
#define CONST_EXTEND_COMPLEX_C {0, 0, 0}
#define CONST_EXTEND_COMPLEX_ENABLED true
/// Перечисления модели extend_complex (ExtendComplex)
#define ENUM_EXTEND_COMPLEX_X 0
#define ENUM_EXTEND_COMPLEX_Y 1
#define ENUM_EXTEND_COMPLEX_Z 2
/// Перечисления модели C (ExtendComplex:C)
#define ENUM_EXTEND_COMPLEX_C_GLOBAL 0
#define ENUM_EXTEND_COMPLEX_C_LOCAL 1
/// Внешние функции
extern bool has_flag(bool v);
/// Функции моделей
static bool ExtendComplex_is_collected(ExtendComplex *main, uint8_t c, uint8_t v) {
    return (false);
}


void ExtendComplex_init(ExtendComplex *main) {
    main->state = EXTEND_COMPLEX_INIT;
}

void ExtendComplex_tick(ExtendComplex *main) {
}

void ExtendComplex_reset(ExtendComplex *main) {
    ExtendComplex_init(main);
}

bool ExtendComplex_is_done(const ExtendComplex *main) {
    return main->state == EXTEND_COMPLEX_NEXT;
}
