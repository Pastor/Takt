#ifndef REGULATOR_H__
#define REGULATOR_H__
#include <stdint.h>
#include <stdbool.h>

/* Forward declarations */
typedef struct RegulatorRegulator RegulatorRegulator;
typedef struct Regulator Regulator;

typedef enum {
    REGULATOR_REGULATOR_PORT_READY = 0,
} Regulator_Out_BitPort;

// NOTICE: Определение констант для модели Regulator (Regulator:Regulator)
/* Model Regulator (Regulator:Regulator) */
struct RegulatorRegulator {
    // NOTICE: Определение переменных модели
    int16_t half;
    int16_t near;
    int16_t setpoint;
    int16_t value;
    enum {
        REGULATOR_REGULATOR_INIT,
        REGULATOR_REGULATOR_ADJUST,
        REGULATOR_REGULATOR_DONE,
        REGULATOR_REGULATOR_SETTLED,
        REGULATOR_REGULATOR_END
    } state;
};

// NOTICE: Определение констант для модели regulator (Regulator)
/* Model regulator (Regulator) */
struct Regulator {
    // NOTICE: Определение переменных модели
    enum {
        REGULATOR_INIT,
        REGULATOR_MAIN,
        REGULATOR_END
    } state;
    // NOTICE: Определение extend
    RegulatorRegulator main;
    /// NOTICE: Функции портов ввода вывода
    void  *userdata;
    void  (*write_bit)(Regulator_Out_BitPort port, bool val, void *userdata);
};

void Regulator_init(Regulator *main);
void Regulator_tick(Regulator *main);
void Regulator_reset(Regulator *main);
bool Regulator_is_done(const Regulator *main);
#endif
