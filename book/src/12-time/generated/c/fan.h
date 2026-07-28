#ifndef FAN_H__
#define FAN_H__
#include <stdint.h>
#include <stdbool.h>

/* Forward declarations */
typedef struct FanFan FanFan;
typedef struct Fan Fan;

typedef enum {
    FAN_FAN_LIGHT = 0,
} Fan_In_BitPort;

typedef enum {
    FAN_FAN_MOTOR = 0,
} Fan_Out_BitPort;

// NOTICE: Определение констант для модели Fan (Fan:Fan)
/* Model Fan (Fan:Fan) */
struct FanFan {
    // NOTICE: Определение переменных модели
    enum {
        FAN_FAN_INIT,
        FAN_FAN_IDLE,
        FAN_FAN_OVERRUN,
        FAN_FAN_WORKING,
        FAN_FAN_END
    } state;
    // NOTICE: Счётчик тактов, прошедших с входа в состояние (выдержка `after`)
    uint32_t takt_dwell;
    unsigned takt_prev_state;
};

// NOTICE: Определение констант для модели fan (Fan)
/* Model fan (Fan) */
struct Fan {
    // NOTICE: Определение переменных модели
    enum {
        FAN_INIT,
        FAN_MAIN,
        FAN_END
    } state;
    // NOTICE: Определение extend
    FanFan main;
    /// NOTICE: Функции портов ввода вывода
    void  *userdata;
    void  (*write_bit)(Fan_Out_BitPort port, bool val, void *userdata);
    bool  (*read_bit )(Fan_In_BitPort port, void *userdata);
};

void Fan_init(Fan *main);
void Fan_tick(Fan *main);
void Fan_reset(Fan *main);
bool Fan_is_done(const Fan *main);
#endif
