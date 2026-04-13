#ifndef THIS_IS_MY_MODEL_H__
#define THIS_IS_MY_MODEL_H__
#include <stdint.h>
#include <stdbool.h>

/* Forward declarations */
typedef struct ThisIsMyModelPing ThisIsMyModelPing;
typedef struct ThisIsMyModelPong ThisIsMyModelPong;
typedef struct ThisIsMyModelToggle ThisIsMyModelToggle;
typedef struct ThisIsMyModel ThisIsMyModel;

// NOTICE: Определение констант для модели Ping (ThisIsMyModel:Ping)
/* Model Ping (ThisIsMyModel:Ping) */
struct ThisIsMyModelPing {
    // NOTICE: Определение переменных модели
    bool toggle;
    enum {
        THIS_IS_MY_MODEL_PING_INIT,
        THIS_IS_MY_MODEL_PING_START,
        THIS_IS_MY_MODEL_PING_END
    } state;
};

// NOTICE: Определение констант для модели Pong (ThisIsMyModel:Pong)
/* Model Pong (ThisIsMyModel:Pong) */
struct ThisIsMyModelPong {
    // NOTICE: Определение переменных модели
    enum {
        THIS_IS_MY_MODEL_PONG_INIT,
        THIS_IS_MY_MODEL_PONG_STOP,
        THIS_IS_MY_MODEL_PONG_BEGIN,
        THIS_IS_MY_MODEL_PONG_END
    } state;
};

// NOTICE: Определение констант для модели Toggle (ThisIsMyModel:Toggle)
/* Model Toggle (ThisIsMyModel:Toggle) */
struct ThisIsMyModelToggle {
    // NOTICE: Определение переменных модели
    enum {
        THIS_IS_MY_MODEL_TOGGLE_INIT,
        THIS_IS_MY_MODEL_TOGGLE_END,
        THIS_IS_MY_MODEL_TOGGLE_PONG,
        THIS_IS_MY_MODEL_TOGGLE_PING,
        THIS_IS_MY_MODEL_TOGGLE_ENTRY,
        THIS_IS_MY_MODEL_TOGGLE_COMPLETE
    } state;
    // NOTICE: Определение extend
    ThisIsMyModelPong pong;
    ThisIsMyModelPing ping;
};

// NOTICE: Определение констант для модели ThisIsMyModel (ThisIsMyModel)
/* Model ThisIsMyModel (ThisIsMyModel) */
struct ThisIsMyModel {
    // NOTICE: Определение переменных модели
    uint64_t it;
    enum {
        THIS_IS_MY_MODEL_INIT,
        THIS_IS_MY_MODEL_ENTRY,
        THIS_IS_MY_MODEL_END
    } state;
    // NOTICE: Определение extend
    struct {
        ThisIsMyModelPing ping0;
        ThisIsMyModelPong pong1;
        enum {
            THIS_IS_MY_MODEL_ENTRY_PARALLEL0_INIT,
            THIS_IS_MY_MODEL_ENTRY_PARALLEL0_TICK,
            THIS_IS_MY_MODEL_ENTRY_PARALLEL0_END
        } state;
    } entry_parallel0;
    ThisIsMyModelToggle entry_toggle1;
    enum {
        THIS_IS_MY_MODEL_ENTRY_INIT,
        THIS_IS_MY_MODEL_ENTRY_PARALLEL0,
        THIS_IS_MY_MODEL_ENTRY_TOGGLE1,
        THIS_IS_MY_MODEL_ENTRY_END
    } entry_state;
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
