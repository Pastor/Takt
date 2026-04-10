#ifndef EXTEND_COMPLEX_H__
#define EXTEND_COMPLEX_H__
#include <stdint.h>
#include <stdbool.h>

/* Forward declarations */
typedef struct ExtendComplexF ExtendComplexF;
typedef struct ExtendComplexCC1 ExtendComplexCC1;
typedef struct ExtendComplexCC2 ExtendComplexCC2;
typedef struct ExtendComplexC ExtendComplexC;
typedef struct ExtendComplexD ExtendComplexD;
typedef struct ExtendComplexB ExtendComplexB;
typedef struct ExtendComplexA ExtendComplexA;
typedef struct ExtendComplexE ExtendComplexE;
typedef struct ExtendComplex ExtendComplex;

// NOTICE: Определение констант для модели F (ExtendComplex:F)
/* Model F (ExtendComplex:F) */
struct ExtendComplexF {
    // NOTICE: Определение переменных модели
    enum {
        EXTEND_COMPLEX_F_INIT,
        EXTEND_COMPLEX_F_START,
        EXTEND_COMPLEX_F_END
    } state;
};

// NOTICE: Определение констант для модели C1 (ExtendComplex:C:C1)
/* Model C1 (ExtendComplex:C:C1) */
struct ExtendComplexCC1 {
    // NOTICE: Определение переменных модели
    enum {
        EXTEND_COMPLEX_C_C1_INIT,
        EXTEND_COMPLEX_C_C1_START,
        EXTEND_COMPLEX_C_C1_END
    } state;
};

// NOTICE: Определение констант для модели C2 (ExtendComplex:C:C2)
/* Model C2 (ExtendComplex:C:C2) */
struct ExtendComplexCC2 {
    // NOTICE: Определение переменных модели
    enum {
        EXTEND_COMPLEX_C_C2_INIT,
        EXTEND_COMPLEX_C_C2_START,
        EXTEND_COMPLEX_C_C2_END
    } state;
};

// NOTICE: Определение констант для модели C (ExtendComplex:C)
/* Model C (ExtendComplex:C) */
struct ExtendComplexC {
    // NOTICE: Определение переменных модели
    enum {
        EXTEND_COMPLEX_C_INIT,
        EXTEND_COMPLEX_C_END,
        EXTEND_COMPLEX_C_START
    } state;
    // NOTICE: Определение extend
    struct {
        ExtendComplexCC1 c10;
        ExtendComplexCC2 c21;
        enum {
            EXTEND_COMPLEX_C_START_INIT,
            EXTEND_COMPLEX_C_START_TICK,
            EXTEND_COMPLEX_C_START_END
        } state;
    } start;
};

// NOTICE: Определение констант для модели D (ExtendComplex:D)
/* Model D (ExtendComplex:D) */
struct ExtendComplexD {
    // NOTICE: Определение переменных модели
    enum {
        EXTEND_COMPLEX_D_INIT,
        EXTEND_COMPLEX_D_START,
        EXTEND_COMPLEX_D_END
    } state;
};

// NOTICE: Определение констант для модели B (ExtendComplex:B)
/* Model B (ExtendComplex:B) */
struct ExtendComplexB {
    // NOTICE: Определение переменных модели
    enum {
        EXTEND_COMPLEX_B_INIT,
        EXTEND_COMPLEX_B_START,
        EXTEND_COMPLEX_B_END
    } state;
};

// NOTICE: Определение констант для модели A (ExtendComplex:A)
/* Model A (ExtendComplex:A) */
struct ExtendComplexA {
    // NOTICE: Определение переменных модели
    enum {
        EXTEND_COMPLEX_A_INIT,
        EXTEND_COMPLEX_A_START,
        EXTEND_COMPLEX_A_END
    } state;
};

// NOTICE: Определение констант для модели E (ExtendComplex:E)
/* Model E (ExtendComplex:E) */
struct ExtendComplexE {
    // NOTICE: Определение переменных модели
    enum {
        EXTEND_COMPLEX_E_INIT,
        EXTEND_COMPLEX_E_START,
        EXTEND_COMPLEX_E_END
    } state;
    // NOTICE: Определение extend
    ExtendComplexD start_d0;
    ExtendComplexF start_f1;
    enum {
        EXTEND_COMPLEX_E_START_INIT,
        EXTEND_COMPLEX_E_START_D0,
        EXTEND_COMPLEX_E_START_F1,
        EXTEND_COMPLEX_E_START_END
    } start_state;
};

// NOTICE: Определение констант для модели extend_complex (ExtendComplex)
/* Model extend_complex (ExtendComplex) */
struct ExtendComplex {
    // NOTICE: Определение переменных модели
    uint8_t x;
    uint8_t y;
    enum {
        EXTEND_COMPLEX_INIT,
        EXTEND_COMPLEX_START,
        EXTEND_COMPLEX_NEXT,
        EXTEND_COMPLEX_END
    } state;
    // NOTICE: Определение extend
    ExtendComplexA start_a0;
    ExtendComplexB start_b1;
    struct {
        ExtendComplexC c0;
        ExtendComplexD d1;
        enum {
            EXTEND_COMPLEX_START_PARALLEL2_INIT,
            EXTEND_COMPLEX_START_PARALLEL2_TICK,
            EXTEND_COMPLEX_START_PARALLEL2_END
        } state;
    } start_parallel2;
    ExtendComplexE start_e3;
    enum {
        EXTEND_COMPLEX_START_INIT,
        EXTEND_COMPLEX_START_A0,
        EXTEND_COMPLEX_START_B1,
        EXTEND_COMPLEX_START_PARALLEL2,
        EXTEND_COMPLEX_START_E3,
        EXTEND_COMPLEX_START_END
    } start_state;
    ExtendComplexF next;
    /// NOTICE: Функции портов ввода вывода
    void  *userdata;
    void  (*write_bit  )(int address, int bit, bool val, void *userdata);
    bool  (*read_bit   )(int address, int bit, void *userdata);
    void  (*write_float)(int address, int bit, float val, void *userdata);
    float (*read_float )(int address, int bit, void *userdata);
};

void ExtendComplex_init(ExtendComplex *main);
void ExtendComplex_tick(ExtendComplex *main);
void ExtendComplex_reset(ExtendComplex *main);
bool ExtendComplex_is_done(const ExtendComplex *main);
#endif
