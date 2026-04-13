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
/// Model functions 'A (ExtendComplex:A)'
static void ExtendComplexA_init(ExtendComplexA *model, const ExtendComplex *main);
static void ExtendComplexA_tick(ExtendComplexA *model, const ExtendComplex *main);
static bool ExtendComplexA_is_done(const ExtendComplexA *model, const ExtendComplex *main);
/// Model functions 'C2 (ExtendComplex:C:C2)'
static void ExtendComplexCC2_init(ExtendComplexCC2 *model, const ExtendComplex *main);
static void ExtendComplexCC2_tick(ExtendComplexCC2 *model, const ExtendComplex *main);
static bool ExtendComplexCC2_is_done(const ExtendComplexCC2 *model, const ExtendComplex *main);
/// Model functions 'C1 (ExtendComplex:C:C1)'
static void ExtendComplexCC1_init(ExtendComplexCC1 *model, const ExtendComplex *main);
static void ExtendComplexCC1_tick(ExtendComplexCC1 *model, const ExtendComplex *main);
static bool ExtendComplexCC1_is_done(const ExtendComplexCC1 *model, const ExtendComplex *main);
/// Model functions 'C (ExtendComplex:C)'
static void ExtendComplexC_init(ExtendComplexC *model, const ExtendComplex *main);
static void ExtendComplexC_tick(ExtendComplexC *model, const ExtendComplex *main);
static bool ExtendComplexC_is_done(const ExtendComplexC *model, const ExtendComplex *main);
/// Model functions 'B (ExtendComplex:B)'
static void ExtendComplexB_init(ExtendComplexB *model, const ExtendComplex *main);
static void ExtendComplexB_tick(ExtendComplexB *model, const ExtendComplex *main);
static bool ExtendComplexB_is_done(const ExtendComplexB *model, const ExtendComplex *main);
/// Model functions 'D (ExtendComplex:D)'
static void ExtendComplexD_init(ExtendComplexD *model, const ExtendComplex *main);
static void ExtendComplexD_tick(ExtendComplexD *model, const ExtendComplex *main);
static bool ExtendComplexD_is_done(const ExtendComplexD *model, const ExtendComplex *main);
/// Model functions 'F (ExtendComplex:F)'
static void ExtendComplexF_init(ExtendComplexF *model, const ExtendComplex *main);
static void ExtendComplexF_tick(ExtendComplexF *model, const ExtendComplex *main);
static bool ExtendComplexF_is_done(const ExtendComplexF *model, const ExtendComplex *main);
/// Model functions 'E (ExtendComplex:E)'
static void ExtendComplexE_init(ExtendComplexE *model, const ExtendComplex *main);
static void ExtendComplexE_tick(ExtendComplexE *model, const ExtendComplex *main);
static bool ExtendComplexE_is_done(const ExtendComplexE *model, const ExtendComplex *main);

///Внешние функции
extern bool has_flag(bool v);
///Функции моделей
static bool ExtendComplex_is_collected(ExtendComplex *main, uint8_t c, uint8_t v) {
    if (c == 0 && v == 1) {
        return true;
    } else if ((c == 1 && v == 2)) {
        return true;
    }
    return false;
}

/// Функция инициализации модели C2 (ExtendComplex:C:C2)
void ExtendComplexCC2_init(ExtendComplexCC2 *model, const ExtendComplex *main) {
    assert(0 != model);
    model->state = EXTEND_COMPLEX_C_C2_INIT;
}

/// Функция обработки модели C2 (ExtendComplex:C:C2)
void ExtendComplexCC2_tick(ExtendComplexCC2 *model, const ExtendComplex *main) {
    assert(0 != model);
    assert(0 != main);
    switch (model->state) {
        case EXTEND_COMPLEX_C_C2_INIT: {
            ///FIXME: Пока не реализовано
            break;
        }
        case EXTEND_COMPLEX_C_C2_START: {
            ///FIXME: Пока не реализовано
            break;
        }
        case EXTEND_COMPLEX_C_C2_END: {
            ///FIXME: Пока не реализовано
            break;
        }
    }
}

/// Функция сброса модели C2 (ExtendComplex:C:C2)
void ExtendComplexCC2_reset(ExtendComplexCC2 *model, const ExtendComplex *main) {
    ExtendComplexCC2_init(model, main);
}

/// Функция проверки терминального состояния модели C2 (ExtendComplex:C:C2)
bool ExtendComplexCC2_is_done(const ExtendComplexCC2 *model, const ExtendComplex *main) {
    return model->state == EXTEND_COMPLEX_C_C2_START;
}

/// Функция инициализации модели E (ExtendComplex:E)
void ExtendComplexE_init(ExtendComplexE *model, const ExtendComplex *main) {
    assert(0 != model);
    model->state = EXTEND_COMPLEX_E_INIT;
}

/// Функция обработки модели E (ExtendComplex:E)
void ExtendComplexE_tick(ExtendComplexE *model, const ExtendComplex *main) {
    assert(0 != model);
    assert(0 != main);
    switch (model->state) {
        case EXTEND_COMPLEX_E_INIT: {
            ///FIXME: Пока не реализовано
            break;
        }
        case EXTEND_COMPLEX_E_START: {
            ///FIXME: Пока не реализовано
            break;
        }
        case EXTEND_COMPLEX_E_END: {
            ///FIXME: Пока не реализовано
            break;
        }
        case EXTEND_COMPLEX_E_END: {
            ///FIXME: Пока не реализовано
            break;
        }
    }
}

/// Функция сброса модели E (ExtendComplex:E)
void ExtendComplexE_reset(ExtendComplexE *model, const ExtendComplex *main) {
    ExtendComplexE_init(model, main);
}

/// Функция проверки терминального состояния модели E (ExtendComplex:E)
bool ExtendComplexE_is_done(const ExtendComplexE *model, const ExtendComplex *main) {
    return model->state == EXTEND_COMPLEX_E_END;
}

/// Функция инициализации модели B (ExtendComplex:B)
void ExtendComplexB_init(ExtendComplexB *model, const ExtendComplex *main) {
    assert(0 != model);
    model->state = EXTEND_COMPLEX_B_INIT;
}

/// Функция обработки модели B (ExtendComplex:B)
void ExtendComplexB_tick(ExtendComplexB *model, const ExtendComplex *main) {
    assert(0 != model);
    assert(0 != main);
    switch (model->state) {
        case EXTEND_COMPLEX_B_INIT: {
            ///FIXME: Пока не реализовано
            break;
        }
        case EXTEND_COMPLEX_B_START: {
            ///FIXME: Пока не реализовано
            break;
        }
        case EXTEND_COMPLEX_B_END: {
            ///FIXME: Пока не реализовано
            break;
        }
    }
}

/// Функция сброса модели B (ExtendComplex:B)
void ExtendComplexB_reset(ExtendComplexB *model, const ExtendComplex *main) {
    ExtendComplexB_init(model, main);
}

/// Функция проверки терминального состояния модели B (ExtendComplex:B)
bool ExtendComplexB_is_done(const ExtendComplexB *model, const ExtendComplex *main) {
    return model->state == EXTEND_COMPLEX_B_START;
}

/// Функция инициализации модели D (ExtendComplex:D)
void ExtendComplexD_init(ExtendComplexD *model, const ExtendComplex *main) {
    assert(0 != model);
    model->state = EXTEND_COMPLEX_D_INIT;
}

/// Функция обработки модели D (ExtendComplex:D)
void ExtendComplexD_tick(ExtendComplexD *model, const ExtendComplex *main) {
    assert(0 != model);
    assert(0 != main);
    switch (model->state) {
        case EXTEND_COMPLEX_D_INIT: {
            ///FIXME: Пока не реализовано
            break;
        }
        case EXTEND_COMPLEX_D_START: {
            ///FIXME: Пока не реализовано
            break;
        }
        case EXTEND_COMPLEX_D_END: {
            ///FIXME: Пока не реализовано
            break;
        }
    }
}

/// Функция сброса модели D (ExtendComplex:D)
void ExtendComplexD_reset(ExtendComplexD *model, const ExtendComplex *main) {
    ExtendComplexD_init(model, main);
}

/// Функция проверки терминального состояния модели D (ExtendComplex:D)
bool ExtendComplexD_is_done(const ExtendComplexD *model, const ExtendComplex *main) {
    return model->state == EXTEND_COMPLEX_D_START;
}

/// Функция инициализации модели C (ExtendComplex:C)
void ExtendComplexC_init(ExtendComplexC *model, const ExtendComplex *main) {
    assert(0 != model);
    model->state = EXTEND_COMPLEX_C_INIT;
}

/// Функция обработки модели C (ExtendComplex:C)
void ExtendComplexC_tick(ExtendComplexC *model, const ExtendComplex *main) {
    assert(0 != model);
    assert(0 != main);
    switch (model->state) {
        case EXTEND_COMPLEX_C_INIT: {
            ///FIXME: Пока не реализовано
            break;
        }
        case EXTEND_COMPLEX_C_START: {
            ///FIXME: Пока не реализовано
            break;
        }
        case EXTEND_COMPLEX_C_END: {
            ///FIXME: Пока не реализовано
            break;
        }
        case EXTEND_COMPLEX_C_END: {
            ///FIXME: Пока не реализовано
            break;
        }
    }
}

/// Функция сброса модели C (ExtendComplex:C)
void ExtendComplexC_reset(ExtendComplexC *model, const ExtendComplex *main) {
    ExtendComplexC_init(model, main);
}

/// Функция проверки терминального состояния модели C (ExtendComplex:C)
bool ExtendComplexC_is_done(const ExtendComplexC *model, const ExtendComplex *main) {
    return model->state == EXTEND_COMPLEX_C_END;
}

/// Функция инициализации модели C1 (ExtendComplex:C:C1)
void ExtendComplexCC1_init(ExtendComplexCC1 *model, const ExtendComplex *main) {
    assert(0 != model);
    model->state = EXTEND_COMPLEX_C_C1_INIT;
}

/// Функция обработки модели C1 (ExtendComplex:C:C1)
void ExtendComplexCC1_tick(ExtendComplexCC1 *model, const ExtendComplex *main) {
    assert(0 != model);
    assert(0 != main);
    switch (model->state) {
        case EXTEND_COMPLEX_C_C1_INIT: {
            ///FIXME: Пока не реализовано
            break;
        }
        case EXTEND_COMPLEX_C_C1_END: {
            ///FIXME: Пока не реализовано
            break;
        }
        case EXTEND_COMPLEX_C_C1_START: {
            ///FIXME: Пока не реализовано
            break;
        }
        case EXTEND_COMPLEX_C_C1_END: {
            ///FIXME: Пока не реализовано
            break;
        }
    }
}

/// Функция сброса модели C1 (ExtendComplex:C:C1)
void ExtendComplexCC1_reset(ExtendComplexCC1 *model, const ExtendComplex *main) {
    ExtendComplexCC1_init(model, main);
}

/// Функция проверки терминального состояния модели C1 (ExtendComplex:C:C1)
bool ExtendComplexCC1_is_done(const ExtendComplexCC1 *model, const ExtendComplex *main) {
    return model->state == EXTEND_COMPLEX_C_C1_END;
}

/// Функция инициализации модели A (ExtendComplex:A)
void ExtendComplexA_init(ExtendComplexA *model, const ExtendComplex *main) {
    assert(0 != model);
    model->state = EXTEND_COMPLEX_A_INIT;
}

/// Функция обработки модели A (ExtendComplex:A)
void ExtendComplexA_tick(ExtendComplexA *model, const ExtendComplex *main) {
    assert(0 != model);
    assert(0 != main);
    switch (model->state) {
        case EXTEND_COMPLEX_A_INIT: {
            ///FIXME: Пока не реализовано
            break;
        }
        case EXTEND_COMPLEX_A_START: {
            ///FIXME: Пока не реализовано
            break;
        }
        case EXTEND_COMPLEX_A_END: {
            ///FIXME: Пока не реализовано
            break;
        }
    }
}

/// Функция сброса модели A (ExtendComplex:A)
void ExtendComplexA_reset(ExtendComplexA *model, const ExtendComplex *main) {
    ExtendComplexA_init(model, main);
}

/// Функция проверки терминального состояния модели A (ExtendComplex:A)
bool ExtendComplexA_is_done(const ExtendComplexA *model, const ExtendComplex *main) {
    return model->state == EXTEND_COMPLEX_A_START;
}

/// Функция инициализации модели F (ExtendComplex:F)
void ExtendComplexF_init(ExtendComplexF *model, const ExtendComplex *main) {
    assert(0 != model);
    model->state = EXTEND_COMPLEX_F_INIT;
}

/// Функция обработки модели F (ExtendComplex:F)
void ExtendComplexF_tick(ExtendComplexF *model, const ExtendComplex *main) {
    assert(0 != model);
    assert(0 != main);
    switch (model->state) {
        case EXTEND_COMPLEX_F_INIT: {
            ///FIXME: Пока не реализовано
            break;
        }
        case EXTEND_COMPLEX_F_START: {
            ///FIXME: Пока не реализовано
            break;
        }
        case EXTEND_COMPLEX_F_END: {
            ///FIXME: Пока не реализовано
            break;
        }
    }
}

/// Функция сброса модели F (ExtendComplex:F)
void ExtendComplexF_reset(ExtendComplexF *model, const ExtendComplex *main) {
    ExtendComplexF_init(model, main);
}

/// Функция проверки терминального состояния модели F (ExtendComplex:F)
bool ExtendComplexF_is_done(const ExtendComplexF *model, const ExtendComplex *main) {
    return model->state == EXTEND_COMPLEX_F_START;
}

/// Функция инициализации модели extend_complex (ExtendComplex)
void ExtendComplex_init(ExtendComplex *model) {
    assert(0 != model);
    model->state = EXTEND_COMPLEX_INIT;
    model->y = 2;
    model->x = 1;
}

/// Функция обработки модели extend_complex (ExtendComplex)
void ExtendComplex_tick(ExtendComplex *model) {
    assert(0 != model);
    switch (model->state) {
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

/// Функция сброса модели extend_complex (ExtendComplex)
void ExtendComplex_reset(ExtendComplex *model) {
    ExtendComplex_init(model);
}

/// Функция проверки терминального состояния модели extend_complex (ExtendComplex)
bool ExtendComplex_is_done(const ExtendComplex *model) {
    return model->state == EXTEND_COMPLEX_NEXT;
}

