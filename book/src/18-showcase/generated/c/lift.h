#ifndef LIFT_H__
#define LIFT_H__
#include <stdint.h>
#include <stdbool.h>

typedef struct Lift Lift;

typedef enum {
    LIFT_BRAKE = 0,
    LIFT_DOORS_OPEN = 1,
    LIFT_MOTOR_DOWN = 2,
    LIFT_MOTOR_UP = 3,
} Lift_Out_BitPort;

typedef enum {
    LIFT_AT_FLOOR = 0,
    LIFT_CALL = 1,
} Lift_In_NumericPort;

typedef enum {
    LIFT_DISPLAY = 0,
} Lift_Out_NumericPort;

// NOTICE: Определение констант для модели lift (Lift)
/* Model lift (Lift) */
struct Lift {
    // NOTICE: Определение переменных модели
    uint8_t doors;
    uint8_t dwell;
    uint8_t moving;
    enum {
        LIFT_INIT,
        LIFT_BOARDING,
        LIFT_GOING_DOWN,
        LIFT_GOING_UP,
        LIFT_LEAVING,
        LIFT_STOPPING,
        LIFT_WAITING,
        LIFT_END
    } state;
    /// NOTICE: Функции портов ввода вывода
    void  *userdata;
    void  (*write_bit)(Lift_Out_BitPort port, bool val, void *userdata);
    void    (*write_numeric)(Lift_Out_NumericPort port, int64_t val, void *userdata);
    int64_t (*read_numeric )(Lift_In_NumericPort port, void *userdata);
};

void Lift_init(Lift *main);
void Lift_tick(Lift *main);
void Lift_reset(Lift *main);
bool Lift_is_done(const Lift *main);
#endif
