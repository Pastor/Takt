#include "extend_complex.h"
#include <assert.h>
#include <math.h>
/// Константы и порты модели extend_complex (ExtendComplex)
#define CONST_EXTEND_COMPLEX_C {0, 0, 0}
#define CONST_EXTEND_COMPLEX_ENABLED true
#define PORT_EXTEND_COMPLEX_IDLE 0x44533932
#define PORT_EXTEND_COMPLEX_WAIT 0x44533932
#define PORT_EXTEND_COMPLEX_WORK 0x44533932
/// Перечисления модели extend_complex (ExtendComplex)
#define ENUM_EXTEND_COMPLEX_X 0
#define ENUM_EXTEND_COMPLEX_Y 1
#define ENUM_EXTEND_COMPLEX_Z 2
/// Перечисления модели C (ExtendComplex:C)
#define ENUM_EXTEND_COMPLEX_C_GLOBAL 0
#define ENUM_EXTEND_COMPLEX_C_LOCAL 1
/// Model functions 'C1 (ExtendComplex:C:C1)'
static void ExtendComplexCC1_init(ExtendComplexCC1 *model, const ExtendComplex *main);
static void ExtendComplexCC1_tick(ExtendComplexCC1 *model, const ExtendComplex *main);
static bool ExtendComplexCC1_is_done(const ExtendComplexCC1 *model, const ExtendComplex *main);
/// Model functions 'C2 (ExtendComplex:C:C2)'
static void ExtendComplexCC2_init(ExtendComplexCC2 *model, const ExtendComplex *main);
static void ExtendComplexCC2_tick(ExtendComplexCC2 *model, const ExtendComplex *main);
static bool ExtendComplexCC2_is_done(const ExtendComplexCC2 *model, const ExtendComplex *main);
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
/// Model functions 'A (ExtendComplex:A)'
static void ExtendComplexA_init(ExtendComplexA *model, const ExtendComplex *main);
static void ExtendComplexA_tick(ExtendComplexA *model, const ExtendComplex *main);
static bool ExtendComplexA_is_done(const ExtendComplexA *model, const ExtendComplex *main);
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
static bool ExtendComplex_is_collected(const ExtendComplex *main, uint8_t c, uint8_t v) {
    if (c == 0 && v == 1) {
        return true;
    } else if ((c == 1 && v == 2)) {
        return true;
    }
    return false;
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
            ExtendComplexD_init(&model->start_d0, main);
            model->start_state = EXTEND_COMPLEX_E_START_D0;
            model->state = EXTEND_COMPLEX_E_START;
            break;
        }
        case EXTEND_COMPLEX_E_START: {
            if (model->start_state == EXTEND_COMPLEX_E_START_D0) {
                ExtendComplexD_tick(&model->start_d0, main);
                if (ExtendComplexD_is_done(&model->start_d0, main)) {
                    ExtendComplexF_init(&model->start_f1, main);
                    model->start_state = EXTEND_COMPLEX_E_START_F1;
                    break;
                }
            } else if (model->start_state == EXTEND_COMPLEX_E_START_F1) {
                ExtendComplexF_tick(&model->start_f1, main);
                if (ExtendComplexF_is_done(&model->start_f1, main)) {
                    model->state = EXTEND_COMPLEX_E_END;
                    break;
                }
            }
            break;
        }
        case EXTEND_COMPLEX_E_END: {
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
            ExtendComplexCC1_init(&model->start.c10, main);
            ExtendComplexCC2_init(&model->start.c21, main);
            model->start.state = EXTEND_COMPLEX_C_START_INIT;
            model->state = EXTEND_COMPLEX_C_START;
            break;
        }
        case EXTEND_COMPLEX_C_START: {
            ExtendComplexCC1_tick(&model->start.c10, main);
            ExtendComplexCC2_tick(&model->start.c21, main);
            if (ExtendComplexCC1_is_done(&model->start.c10, main) && ExtendComplexCC2_is_done(&model->start.c21, main)) {
                model->state = EXTEND_COMPLEX_C_END;
                break;
            }
            break;
        }
        case EXTEND_COMPLEX_C_END: {
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
            model->state = EXTEND_COMPLEX_C_C2_START;
            break;
        }
        case EXTEND_COMPLEX_C_C2_START: {
            model->state = EXTEND_COMPLEX_C_C2_END;
            break;
        }
        case EXTEND_COMPLEX_C_C2_END: {
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
    return model->state == EXTEND_COMPLEX_C_C2_END;
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
            model->state = EXTEND_COMPLEX_D_START;
            break;
        }
        case EXTEND_COMPLEX_D_START: {
            model->state = EXTEND_COMPLEX_D_END;
            break;
        }
        case EXTEND_COMPLEX_D_END: {
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
    return model->state == EXTEND_COMPLEX_D_END;
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
            model->state = EXTEND_COMPLEX_F_START;
            break;
        }
        case EXTEND_COMPLEX_F_START: {
            model->state = EXTEND_COMPLEX_F_END;
            break;
        }
        case EXTEND_COMPLEX_F_END: {
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
    return model->state == EXTEND_COMPLEX_F_END;
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
            model->state = EXTEND_COMPLEX_C_C1_START;
            break;
        }
        case EXTEND_COMPLEX_C_C1_START: {
            if (ExtendComplex_is_collected(main, 1, main->y) & ExtendComplex_is_collected(main, 0, main->x) & has_flag(CONST_EXTEND_COMPLEX_ENABLED)) {
                model->state = EXTEND_COMPLEX_C_C1_END;
                break;
            }
            break;
        }
        case EXTEND_COMPLEX_C_C1_END: {
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
            model->state = EXTEND_COMPLEX_A_START;
            break;
        }
        case EXTEND_COMPLEX_A_START: {
            model->state = EXTEND_COMPLEX_A_END;
            break;
        }
        case EXTEND_COMPLEX_A_END: {
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
    return model->state == EXTEND_COMPLEX_A_END;
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
            model->state = EXTEND_COMPLEX_B_START;
            break;
        }
        case EXTEND_COMPLEX_B_START: {
            if ((*main->read_bit)(PORT_EXTEND_COMPLEX_WAIT, 33, main->userdata)) {
                (*main->write_bit)(PORT_EXTEND_COMPLEX_WORK, 2, 0, main->userdata);
                (*main->write_bit)(PORT_EXTEND_COMPLEX_IDLE, 1, 1, main->userdata);
            } else {
                (*main->write_bit)(PORT_EXTEND_COMPLEX_IDLE, 1, 0, main->userdata);
                (*main->write_bit)(PORT_EXTEND_COMPLEX_WORK, 2, 1, main->userdata);
            }
            model->state = EXTEND_COMPLEX_B_END;
            break;
        }
        case EXTEND_COMPLEX_B_END: {
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
    return model->state == EXTEND_COMPLEX_B_END;
}

/// Функция инициализации модели extend_complex (ExtendComplex)
void ExtendComplex_init(ExtendComplex *model) {
    assert(0 != model);
    model->state = EXTEND_COMPLEX_INIT;
    model->x = 1;
    model->y = 2;
}

/// Функция обработки модели extend_complex (ExtendComplex)
void ExtendComplex_tick(ExtendComplex *model) {
    assert(0 != model);
    switch (model->state) {
        case EXTEND_COMPLEX_INIT: {
            ExtendComplexA_init(&model->start_a0, model);
            model->start_state = EXTEND_COMPLEX_START_A0;
            model->state = EXTEND_COMPLEX_START;
            break;
        }
        case EXTEND_COMPLEX_START: {
            if (model->start_state == EXTEND_COMPLEX_START_A0) {
                ExtendComplexA_tick(&model->start_a0, model);
                if (ExtendComplexA_is_done(&model->start_a0, model)) {
                    ExtendComplexB_init(&model->start_b1, model);
                    model->start_state = EXTEND_COMPLEX_START_B1;
                    break;
                }
            } else if (model->start_state == EXTEND_COMPLEX_START_B1) {
                ExtendComplexB_tick(&model->start_b1, model);
                if (ExtendComplexB_is_done(&model->start_b1, model)) {
                    ExtendComplexC_init(&model->start_parallel2.c0, model);
                    ExtendComplexD_init(&model->start_parallel2.d1, model);
                    model->start_parallel2.state = EXTEND_COMPLEX_START_PARALLEL2_INIT;
                    model->start_state = EXTEND_COMPLEX_START_PARALLEL2;
                    break;
                }
            } else if (model->start_state == EXTEND_COMPLEX_START_PARALLEL2) {
                ExtendComplexC_tick(&model->start_parallel2.c0, model);
                ExtendComplexD_tick(&model->start_parallel2.d1, model);
                if (ExtendComplexC_is_done(&model->start_parallel2.c0, model) && ExtendComplexD_is_done(&model->start_parallel2.d1, model)) {
                    ExtendComplexE_init(&model->start_e3, model);
                    model->start_state = EXTEND_COMPLEX_START_E3;
                    break;
                }
            } else if (model->start_state == EXTEND_COMPLEX_START_E3) {
                ExtendComplexE_tick(&model->start_e3, model);
                if (ExtendComplexE_is_done(&model->start_e3, model)) {
                    model->state = EXTEND_COMPLEX_NEXT;
                    break;
                }
            }
            break;
        }
        case EXTEND_COMPLEX_NEXT: {
            ExtendComplexF_tick(&model->next, model);
            if (ExtendComplexF_is_done(&model->next, model)) {
                model->state = EXTEND_COMPLEX_END;
                break;
            }
            break;
        }
        case EXTEND_COMPLEX_END: {
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
    return model->state == EXTEND_COMPLEX_END;
}

