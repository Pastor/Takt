#ifndef BATCH_CYCLE_H__
#define BATCH_CYCLE_H__
#include <stdint.h>
#include <stdbool.h>

/* Forward declarations */
typedef struct BatchCycleDose BatchCycleDose;
typedef struct BatchCycleDrain BatchCycleDrain;
typedef struct BatchCycleMix BatchCycleMix;
typedef struct BatchCycle BatchCycle;

typedef enum {
    BATCH_CYCLE_PORT_READY = 0,
} BatchCycle_Out_BitPort;

// NOTICE: Определение констант для модели Dose (BatchCycle:Dose)
/* Model Dose (BatchCycle:Dose) */
struct BatchCycleDose {
    // NOTICE: Определение переменных модели
    uint8_t dosed;
    enum {
        BATCH_CYCLE_DOSE_INIT,
        BATCH_CYCLE_DOSE_FILL,
        BATCH_CYCLE_DOSE_FULL,
        BATCH_CYCLE_DOSE_END
    } state;
};

// NOTICE: Определение констант для модели Drain (BatchCycle:Drain)
/* Model Drain (BatchCycle:Drain) */
struct BatchCycleDrain {
    // NOTICE: Определение переменных модели
    uint8_t drained;
    enum {
        BATCH_CYCLE_DRAIN_INIT,
        BATCH_CYCLE_DRAIN_DRY,
        BATCH_CYCLE_DRAIN_EMPTY,
        BATCH_CYCLE_DRAIN_END
    } state;
};

// NOTICE: Определение констант для модели Mix (BatchCycle:Mix)
/* Model Mix (BatchCycle:Mix) */
struct BatchCycleMix {
    // NOTICE: Определение переменных модели
    uint8_t stirred;
    enum {
        BATCH_CYCLE_MIX_INIT,
        BATCH_CYCLE_MIX_BLENDED,
        BATCH_CYCLE_MIX_STIR,
        BATCH_CYCLE_MIX_END
    } state;
};

// NOTICE: Определение констант для модели batch_cycle (BatchCycle)
/* Model batch_cycle (BatchCycle) */
struct BatchCycle {
    // NOTICE: Определение переменных модели
    uint8_t stage;
    enum {
        BATCH_CYCLE_INIT,
        BATCH_CYCLE_CYCLE,
        BATCH_CYCLE_DONE,
        BATCH_CYCLE_END
    } state;
    // NOTICE: Определение extend
    BatchCycleDose cycle_dose0;
    BatchCycleMix cycle_mix1;
    BatchCycleDrain cycle_drain2;
    enum {
        BATCH_CYCLE_CYCLE_INIT,
        BATCH_CYCLE_CYCLE_DOSE0,
        BATCH_CYCLE_CYCLE_MIX1,
        BATCH_CYCLE_CYCLE_DRAIN2,
        BATCH_CYCLE_CYCLE_END
    } cycle_state;
    /// NOTICE: Функции портов ввода вывода
    void  *userdata;
    void  (*write_bit)(BatchCycle_Out_BitPort port, uint8_t bit, bool val, void *userdata);
};

void BatchCycle_init(BatchCycle *main);
void BatchCycle_tick(BatchCycle *main);
void BatchCycle_reset(BatchCycle *main);
bool BatchCycle_is_done(const BatchCycle *main);
#endif
