#ifndef STACKER_H__
#define STACKER_H__
#include <stdint.h>
#include <stdbool.h>

/* Forward declarations */
typedef struct StackerMovementController StackerMovementController;
typedef struct StackerLiftController StackerLiftController;
typedef struct StackerCommandReceiver StackerCommandReceiver;
typedef struct Stacker Stacker;

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

// NOTICE: Определение констант для модели MovementController (Stacker:MovementController)
/* Model MovementController (Stacker:MovementController) */
struct StackerMovementController {
    // NOTICE: Определение переменных модели
    enum {
        STACKER_MOVEMENT_CONTROLLER_INIT,
        STACKER_MOVEMENT_CONTROLLER_MOVING_TO_PICKUP,
        STACKER_MOVEMENT_CONTROLLER_MOVING_TO_DROPOFF,
        STACKER_MOVEMENT_CONTROLLER_MOVING_TO_CELL,
        STACKER_MOVEMENT_CONTROLLER_MOVEMENT_IDLE,
        STACKER_MOVEMENT_CONTROLLER_WAITING_FORK_AT_STORAGE,
        STACKER_MOVEMENT_CONTROLLER_WAITING_FORK_AT_PICKUP,
        STACKER_MOVEMENT_CONTROLLER_MOVING_TO_STORAGE,
        STACKER_MOVEMENT_CONTROLLER_EMERGENCY_CHARGE,
        STACKER_MOVEMENT_CONTROLLER_WAITING_FORK_AT_DROPOFF,
        STACKER_MOVEMENT_CONTROLLER_TASK_COMPLETING,
        STACKER_MOVEMENT_CONTROLLER_DISPATCH_MOVE,
        STACKER_MOVEMENT_CONTROLLER_WAITING_FORK_AT_CELL,
        STACKER_MOVEMENT_CONTROLLER_END
    } state;
};

// NOTICE: Определение констант для модели LiftController (Stacker:LiftController)
/* Model LiftController (Stacker:LiftController) */
struct StackerLiftController {
    // NOTICE: Определение переменных модели
    enum {
        STACKER_LIFT_CONTROLLER_INIT,
        STACKER_LIFT_CONTROLLER_LIFT_OPERATING,
        STACKER_LIFT_CONTROLLER_LIFT_DONE,
        STACKER_LIFT_CONTROLLER_LIFT_IDLE,
        STACKER_LIFT_CONTROLLER_END
    } state;
};

// NOTICE: Определение констант для модели CommandReceiver (Stacker:CommandReceiver)
/* Model CommandReceiver (Stacker:CommandReceiver) */
struct StackerCommandReceiver {
    // NOTICE: Определение переменных модели
    enum {
        STACKER_COMMAND_RECEIVER_INIT,
        STACKER_COMMAND_RECEIVER_ACCEPTING_TASK,
        STACKER_COMMAND_RECEIVER_WAITING_FOR_TASK,
        STACKER_COMMAND_RECEIVER_TASK_ACTIVE,
        STACKER_COMMAND_RECEIVER_END
    } state;
};

// NOTICE: Определение констант для модели stacker (Stacker)
/* Model stacker (Stacker) */
struct Stacker {
    // NOTICE: Определение переменных модели
    uint8_t lift_request;
    uint8_t lift_done;
    uint8_t lift_op;
    uint8_t tgt_section;
    uint8_t tgt_stack;
    uint8_t eta;
    uint8_t busy;
    uint8_t tgt_type;
    uint8_t tgt_row;
    enum {
        STACKER_INIT,
        STACKER_STACKER,
        STACKER_END
    } state;
    // NOTICE: Определение extend
    struct {
        StackerCommandReceiver command_receiver0;
        StackerMovementController movement_controller1;
        StackerLiftController lift_controller2;
        enum {
            STACKER_STACKER_INIT,
            STACKER_STACKER_TICK,
            STACKER_STACKER_END
        } state;
    } stacker;
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
