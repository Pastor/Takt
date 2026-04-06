#ifndef THIS_IS_MY_MODEL_H__
#define THIS_IS_MY_MODEL_H__
#include <stdint.h>
#include <stdbool.h>

// NOTICE: Определение констант для модели ThisIsMyModel:Ping
/* Model Ping (ThisIsMyModel:Ping) */
typedef struct ThisIsMyModelPing {
    enum {
        THIS_IS_MY_MODEL_PING_INIT,
        THIS_IS_MY_MODEL_PING_END,
        THIS_IS_MY_MODEL_PING_START
    } state;
    // NOTICE: Определение переменных модели
    bool toggle;
};

// NOTICE: Определение констант для модели ThisIsMyModel:Toggle
/* Model Toggle (ThisIsMyModel:Toggle) */
typedef struct ThisIsMyModelToggle {
    enum {
        THIS_IS_MY_MODEL_TOGGLE_INIT,
        THIS_IS_MY_MODEL_TOGGLE_PING,
        THIS_IS_MY_MODEL_TOGGLE_PONG,
        THIS_IS_MY_MODEL_TOGGLE_END,
        THIS_IS_MY_MODEL_TOGGLE_COMPLETE,
        THIS_IS_MY_MODEL_TOGGLE_ENTRY
    } state;
    // NOTICE: Определение переменных модели
    // NOTICE: Определение extend
    ThisIsMyModelPing ping;
    ThisIsMyModelPong pong;
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
    // NOTICE: Определение extend
    struct {
        ThisIsMyModelPing ping0;
        ThisIsMyModelPong pong1;
    } entry_parallel0;
    ThisIsMyModelToggle entry_toggle1;
    /// NOTICE: Функции портов ввода вывода
    void  *userdata;
    void  (*write_bit  )(int address, int bit, bool val, void *userdata);
    bool  (*read_bit   )(int address, int bit, void *userdata);
    void  (*write_float)(int address, int bit, float val, void *userdata);
    float (*read_float )(int address, int bit, void *userdata);
};

void ThisIsMyModel_init(ThisIsMyModel *main);
void ThisIsMyModel_tick(ThisIsMyModel *main);
void ThisIsMyModel_reset(ThisIsMyModel *main);
bool ThisIsMyModel_is_done(const ThisIsMyModel *main);
#endif
