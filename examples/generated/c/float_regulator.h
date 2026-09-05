#ifndef FLOAT_REGULATOR_H__
#define FLOAT_REGULATOR_H__
#include <stdint.h>
#include <stdbool.h>

/* Forward declarations */
typedef struct FloatRegulatorFloatRegulator FloatRegulatorFloatRegulator;
typedef struct FloatRegulator FloatRegulator;

typedef enum {
    FLOAT_REGULATOR_FLOAT_REGULATOR_PORT_READY = 0,
} FloatRegulator_Out_BitPort;

// NOTICE: Определение констант для модели FloatRegulator (FloatRegulator:FloatRegulator)
/* Model FloatRegulator (FloatRegulator:FloatRegulator) */
struct FloatRegulatorFloatRegulator {
    // NOTICE: Определение переменных модели
    double half;
    double near;
    double setpoint;
    double value;
    enum {
        FLOAT_REGULATOR_FLOAT_REGULATOR_INIT,
        FLOAT_REGULATOR_FLOAT_REGULATOR_ADJUST,
        FLOAT_REGULATOR_FLOAT_REGULATOR_DONE,
        FLOAT_REGULATOR_FLOAT_REGULATOR_SETTLED,
        FLOAT_REGULATOR_FLOAT_REGULATOR_END
    } state;
};

// NOTICE: Определение констант для модели float_regulator (FloatRegulator)
/* Model float_regulator (FloatRegulator) */
struct FloatRegulator {
    // NOTICE: Определение переменных модели
    enum {
        FLOAT_REGULATOR_INIT,
        FLOAT_REGULATOR_MAIN,
        FLOAT_REGULATOR_END
    } state;
    // NOTICE: Определение extend
    FloatRegulatorFloatRegulator main;
    /// NOTICE: Функции портов ввода вывода
    void  *userdata;
    void  (*write_bit)(FloatRegulator_Out_BitPort port, uint8_t bit, bool val, void *userdata);
};

void FloatRegulator_init(FloatRegulator *main);
void FloatRegulator_tick(FloatRegulator *main);
void FloatRegulator_reset(FloatRegulator *main);
bool FloatRegulator_is_done(const FloatRegulator *main);
#endif
