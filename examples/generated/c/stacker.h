#ifndef STACKER_H__
#define STACKER_H__
#include <stdint.h>
#include <stdbool.h>

typedef enum {
    STACKER_SENSE_AT_CHARGE = 0,
    STACKER_SENSE_BATTERY_LOW = 1,
    STACKER_SENSE_LOADED = 2,
    STACKER_TASK_TYPE = 3,
    STACKER_TASK_VALID = 4,
} Stacker_In_BitPort;

typedef enum {
    STACKER_CMD_ACK = 0,
    STACKER_CMD_DONE = 1,
    STACKER_CMD_FORK = 2,
} Stacker_Out_BitPort;

typedef enum {
    STACKER_POS_ROW = 0,
    STACKER_POS_SECTION = 1,
    STACKER_POS_STACK = 2,
    STACKER_TASK_ROW_NO = 3,
    STACKER_TASK_SECTION_NO = 4,
    STACKER_TASK_STACK_NO = 5,
} Stacker_In_NumericPort;

typedef enum {
    STACKER_CMD_TARGET_ROW = 0,
    STACKER_CMD_TARGET_SECTION = 1,
    STACKER_CMD_TARGET_STACK = 2,
} Stacker_Out_NumericPort;

// NOTICE: Определение констант для модели stacker (Stacker)
/* Model stacker (Stacker) */
struct Stacker {
    // NOTICE: Определение переменных модели
    int tgt_type;
    uint8_t tgt_section;
    uint8_t tgt_row;
    uint8_t tgt_stack;
    int busy;
    enum {
        STACKER_INIT,
        STACKER_MOVING_TO_STORAGE,
        STACKER_DISPATCH_TASK,
        STACKER_DELIVERING_LOAD,
        STACKER_IDLE,
        STACKER_TAKING_FROM_CELL,
        STACKER_MOVING_TO_PICKUP,
        STACKER_MOVING_TO_DROPOFF,
        STACKER_EMERGENCY_CHARGE,
        STACKER_TAKING_AT_PICKUP,
        STACKER_MOVING_TO_CELL,
        STACKER_PLACING_IN_CELL,
        STACKER_COMPLETING,
        STACKER_END
    } state;
    /// NOTICE: Функции портов ввода вывода
    void  *userdata;
    void  (*write_bit)(Stacker_Out_BitPort port, bool val, void *userdata);
    bool  (*read_bit )(Stacker_In_BitPort port, void *userdata);
    void    (*write_numeric)(Stacker_Out_NumericPort port, int64_t val, void *userdata);
    int64_t (*read_numeric )(Stacker_In_NumericPort port, void *userdata);
};

void Stacker_init(Stacker *main);
void Stacker_tick(Stacker *main);
void Stacker_reset(Stacker *main);
bool Stacker_is_done(const Stacker *main);
#endif
