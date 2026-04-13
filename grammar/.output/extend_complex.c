#include "extend_complex.h"
#include <assert.h>
#include <math.h>
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
/// Model functions 'F (ExtendComplex:F)'
static void ExtendComplexF_init(ExtendComplex *main);
static void ExtendComplexF_tick(ExtendComplex *main);
static void ExtendComplexF_is_done(ExtendComplex *main);
/// Model functions 'B (ExtendComplex:B)'
static void ExtendComplexB_init(ExtendComplex *main);
static void ExtendComplexB_tick(ExtendComplex *main);
static void ExtendComplexB_is_done(ExtendComplex *main);
/// Model functions 'C1 (ExtendComplex:C:C1)'
static void ExtendComplexCC1_init(ExtendComplex *main);
static void ExtendComplexCC1_tick(ExtendComplex *main);
static void ExtendComplexCC1_is_done(ExtendComplex *main);
/// Model functions 'C2 (ExtendComplex:C:C2)'
static void ExtendComplexCC2_init(ExtendComplex *main);
static void ExtendComplexCC2_tick(ExtendComplex *main);
static void ExtendComplexCC2_is_done(ExtendComplex *main);
/// Model functions 'C (ExtendComplex:C)'
static void ExtendComplexC_init(ExtendComplex *main);
static void ExtendComplexC_tick(ExtendComplex *main);
static void ExtendComplexC_is_done(ExtendComplex *main);
/// Model functions 'D (ExtendComplex:D)'
static void ExtendComplexD_init(ExtendComplex *main);
static void ExtendComplexD_tick(ExtendComplex *main);
static void ExtendComplexD_is_done(ExtendComplex *main);
/// Model functions 'A (ExtendComplex:A)'
static void ExtendComplexA_init(ExtendComplex *main);
static void ExtendComplexA_tick(ExtendComplex *main);
static void ExtendComplexA_is_done(ExtendComplex *main);
/// Model functions 'E (ExtendComplex:E)'
static void ExtendComplexE_init(ExtendComplex *main);
static void ExtendComplexE_tick(ExtendComplex *main);
static void ExtendComplexE_is_done(ExtendComplex *main);

///Внешние функции
extern bool has_flag(bool v);
///Функции моделей
static bool ExtendComplex_is_collected(ExtendComplex *main, uint8_t c, uint8_t v) {
    if (c == 0 && v == 1) {
        return true;
    }
 else {
        if ((c == 1 && v == 2)) {
            return true;
        }
    }
    return false;
}


void ExtendComplex_init(ExtendComplex *main) {
    main->state = EXTEND_COMPLEX_INIT;
}

void ExtendComplex_tick(ExtendComplex *main) {
    assert(0 != main);
    switch (main->state) {
        case EXTEND_COMPLEX_INIT: {
            ///FIXME: Пока не реализовано
            break;
        }
        case EXTEND_COMPLEX_START: {
            ///FIXME: Пока не реализовано
            break;
        }
        case EXTEND_COMPLEX_NEXT: {
            ///FIXME: Пока не реализовано
            break;
        }
        case EXTEND_COMPLEX_END: {
            ///FIXME: Пока не реализовано
            break;
        }
    }
}

void ExtendComplex_reset(ExtendComplex *main) {
    ExtendComplex_init(main);
}

bool ExtendComplex_is_done(const ExtendComplex *main) {
    return main->state == EXTEND_COMPLEX_NEXT;
}

