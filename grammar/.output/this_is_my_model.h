#ifndef THIS_IS_MY_MODEL_H__
#define THIS_IS_MY_MODEL_H__
#include <stdint.h>
#include <stdbool.h>

// NOTICE: Определение констант для модели ThisIsMyModel:Entry_Sequence
/* Model Entry_Sequence (ThisIsMyModel:Entry_Sequence) */
typedef struct ThisIsMyModelEntrySequence {
    enum {
        THIS_IS_MY_MODEL_ENTRY_SEQUENCE_INIT,
        THIS_IS_MY_MODEL_ENTRY_SEQUENCE_STEP0,
        THIS_IS_MY_MODEL_ENTRY_SEQUENCE_STEP1
    } state;
    // NOTICE: Определение переменных модели
    // FIXME: Определение extend
};

// NOTICE: Определение констант для модели ThisIsMyModel:Ping
/* Model Ping (ThisIsMyModel:Ping) */
typedef struct ThisIsMyModelPing {
    enum {
        THIS_IS_MY_MODEL_PING_INIT,
        THIS_IS_MY_MODEL_PING_START,
        THIS_IS_MY_MODEL_PING_END
    } state;
    // NOTICE: Определение переменных модели
    bool toggle;
};

// NOTICE: Определение констант для модели ThisIsMyModel:Toggle
/* Model Toggle (ThisIsMyModel:Toggle) */
typedef struct ThisIsMyModelToggle {
    enum {
        THIS_IS_MY_MODEL_TOGGLE_INIT,
        THIS_IS_MY_MODEL_TOGGLE_ENTRY,
        THIS_IS_MY_MODEL_TOGGLE_PONG,
        THIS_IS_MY_MODEL_TOGGLE_PING,
        THIS_IS_MY_MODEL_TOGGLE_COMPLETE,
        THIS_IS_MY_MODEL_TOGGLE_END
    } state;
    // NOTICE: Определение переменных модели
    // FIXME: Определение extend
};

// NOTICE: Определение констант для модели ThisIsMyModel:Pong
/* Model Pong (ThisIsMyModel:Pong) */
typedef struct ThisIsMyModelPong {
    enum {
        THIS_IS_MY_MODEL_PONG_INIT,
        THIS_IS_MY_MODEL_PONG_STOP,
        THIS_IS_MY_MODEL_PONG_BEGIN
    } state;
    // NOTICE: Определение переменных модели
};

// NOTICE: Определение констант для модели ThisIsMyModel
/* Model ThisIsMyModel (ThisIsMyModel) */
typedef struct ThisIsMyModel {
    enum {
        THIS_IS_MY_MODEL_INIT,
        THIS_IS_MY_MODEL_ENTRY
    } state;
    // NOTICE: Определение переменных модели
    uint64_t it;
    /// NOTICE: Функции портов ввода вывода
    void  (*write_bit  )(int address, int bit, bool val, void *userdata);
    bool  (*read_bit   )(int address, int bit, void *userdata);
    void  (*write_float)(int address, int bit, float val, void *userdata);
    float (*read_float )(int address, int bit, void *userdata);
    // FIXME: Определение extend
};

void ThisIsMyModel_init(ThisIsMyModel *main);
void ThisIsMyModel_tick(ThisIsMyModel *main);
void ThisIsMyModel_reset(ThisIsMyModel *main);
bool ThisIsMyModel_is_done(const ThisIsMyModel *main);
#endif
