#ifndef COMPREHENSIVE_H__
#define COMPREHENSIVE_H__
#include <stdint.h>
#include <stdbool.h>

/* Forward declarations */
typedef struct ComprehensiveController ComprehensiveController;
typedef struct Comprehensive Comprehensive;

// NOTICE: Определение констант для модели Controller (Comprehensive:Controller)
/* Model Controller (Comprehensive:Controller) */
struct ComprehensiveController {
    // NOTICE: Определение переменных модели
    uint8_t temperature;
    uint8_t count;
    enum {
        COMPREHENSIVE_CONTROLLER_INIT,
        COMPREHENSIVE_CONTROLLER_IDLE,
        COMPREHENSIVE_CONTROLLER_COOLING,
        COMPREHENSIVE_CONTROLLER_DONE,
        COMPREHENSIVE_CONTROLLER_HEATING,
        COMPREHENSIVE_CONTROLLER_END
    } state;
};

// NOTICE: Определение констант для модели comprehensive (Comprehensive)
/* Model comprehensive (Comprehensive) */
struct Comprehensive {
    // NOTICE: Определение переменных модели
    enum {
        COMPREHENSIVE_INIT,
        COMPREHENSIVE_ENTRY,
        COMPREHENSIVE_END
    } state;
    // NOTICE: Определение extend
    ComprehensiveController entry;
    /// NOTICE: Функции портов ввода вывода
    void  *userdata;
    void  (*write_bit  )(int address, int bit, bool val, void *userdata);
    bool  (*read_bit   )(int address, int bit, void *userdata);
    void  (*write_float)(int address, int bit, float val, void *userdata);
    float (*read_float )(int address, int bit, void *userdata);
};

void Comprehensive_init(Comprehensive *main);
void Comprehensive_tick(Comprehensive *main);
void Comprehensive_reset(Comprehensive *main);
bool Comprehensive_is_done(const Comprehensive *main);
#endif
