#include "batch_cycle.h"
#include <assert.h>
#include <math.h>
/// Model functions 'Dose (BatchCycle:Dose)'
static void BatchCycleDose_init(BatchCycleDose *model, BatchCycle *main);
static void BatchCycleDose_tick(BatchCycleDose *model, BatchCycle *main);
static bool BatchCycleDose_is_done(const BatchCycleDose *model, BatchCycle *main);
/// Model functions 'Drain (BatchCycle:Drain)'
static void BatchCycleDrain_init(BatchCycleDrain *model, BatchCycle *main);
static void BatchCycleDrain_tick(BatchCycleDrain *model, BatchCycle *main);
static bool BatchCycleDrain_is_done(const BatchCycleDrain *model, BatchCycle *main);
/// Model functions 'Mix (BatchCycle:Mix)'
static void BatchCycleMix_init(BatchCycleMix *model, BatchCycle *main);
static void BatchCycleMix_tick(BatchCycleMix *model, BatchCycle *main);
static bool BatchCycleMix_is_done(const BatchCycleMix *model, BatchCycle *main);

/// Функция инициализации модели Dose (BatchCycle:Dose)
void BatchCycleDose_init(BatchCycleDose *model, BatchCycle *main) {
    assert(0 != model);
    model->state = BATCH_CYCLE_DOSE_INIT;
    model->dosed = 0;
}

/// Функция обработки модели Dose (BatchCycle:Dose)
void BatchCycleDose_tick(BatchCycleDose *model, BatchCycle *main) {
    assert(0 != model);
    assert(0 != main);
    if (model->state == BATCH_CYCLE_DOSE_INIT) {
        model->state = BATCH_CYCLE_DOSE_FILL;
    }
    switch (model->state) {
        case BATCH_CYCLE_DOSE_FILL: {
            main->stage = 1;
            model->dosed = model->dosed + 1;
            if (model->dosed >= 3) {
                model->state = BATCH_CYCLE_DOSE_FULL;
                break;
            }
            break;
        }
        case BATCH_CYCLE_DOSE_FULL: {
            model->state = BATCH_CYCLE_DOSE_END;
            break;
        }
        case BATCH_CYCLE_DOSE_END: {
            break;
        }
        default: break;
    }
}

/// Функция сброса модели Dose (BatchCycle:Dose)
void BatchCycleDose_reset(BatchCycleDose *model, BatchCycle *main) {
    BatchCycleDose_init(model, main);
}

/// Функция проверки терминального состояния модели Dose (BatchCycle:Dose)
bool BatchCycleDose_is_done(const BatchCycleDose *model, BatchCycle *main) {
    return model->state == BATCH_CYCLE_DOSE_END;
}

/// Функция инициализации модели Drain (BatchCycle:Drain)
void BatchCycleDrain_init(BatchCycleDrain *model, BatchCycle *main) {
    assert(0 != model);
    model->state = BATCH_CYCLE_DRAIN_INIT;
    model->drained = 0;
}

/// Функция обработки модели Drain (BatchCycle:Drain)
void BatchCycleDrain_tick(BatchCycleDrain *model, BatchCycle *main) {
    assert(0 != model);
    assert(0 != main);
    if (model->state == BATCH_CYCLE_DRAIN_INIT) {
        model->state = BATCH_CYCLE_DRAIN_EMPTY;
    }
    switch (model->state) {
        case BATCH_CYCLE_DRAIN_DRY: {
            model->state = BATCH_CYCLE_DRAIN_END;
            break;
        }
        case BATCH_CYCLE_DRAIN_EMPTY: {
            main->stage = 3;
            model->drained = model->drained + 1;
            if (model->drained >= 2) {
                model->state = BATCH_CYCLE_DRAIN_DRY;
                break;
            }
            break;
        }
        case BATCH_CYCLE_DRAIN_END: {
            break;
        }
        default: break;
    }
}

/// Функция сброса модели Drain (BatchCycle:Drain)
void BatchCycleDrain_reset(BatchCycleDrain *model, BatchCycle *main) {
    BatchCycleDrain_init(model, main);
}

/// Функция проверки терминального состояния модели Drain (BatchCycle:Drain)
bool BatchCycleDrain_is_done(const BatchCycleDrain *model, BatchCycle *main) {
    return model->state == BATCH_CYCLE_DRAIN_END;
}

/// Функция инициализации модели Mix (BatchCycle:Mix)
void BatchCycleMix_init(BatchCycleMix *model, BatchCycle *main) {
    assert(0 != model);
    model->state = BATCH_CYCLE_MIX_INIT;
    model->stirred = 0;
}

/// Функция обработки модели Mix (BatchCycle:Mix)
void BatchCycleMix_tick(BatchCycleMix *model, BatchCycle *main) {
    assert(0 != model);
    assert(0 != main);
    if (model->state == BATCH_CYCLE_MIX_INIT) {
        model->state = BATCH_CYCLE_MIX_STIR;
    }
    switch (model->state) {
        case BATCH_CYCLE_MIX_BLENDED: {
            model->state = BATCH_CYCLE_MIX_END;
            break;
        }
        case BATCH_CYCLE_MIX_STIR: {
            main->stage = 2;
            model->stirred = model->stirred + 1;
            if (model->stirred >= 2) {
                model->state = BATCH_CYCLE_MIX_BLENDED;
                break;
            }
            break;
        }
        case BATCH_CYCLE_MIX_END: {
            break;
        }
        default: break;
    }
}

/// Функция сброса модели Mix (BatchCycle:Mix)
void BatchCycleMix_reset(BatchCycleMix *model, BatchCycle *main) {
    BatchCycleMix_init(model, main);
}

/// Функция проверки терминального состояния модели Mix (BatchCycle:Mix)
bool BatchCycleMix_is_done(const BatchCycleMix *model, BatchCycle *main) {
    return model->state == BATCH_CYCLE_MIX_END;
}

/// Функция инициализации модели batch_cycle (BatchCycle)
void BatchCycle_init(BatchCycle *model) {
    assert(0 != model);
    model->state = BATCH_CYCLE_INIT;
    BatchCycleDose_init(&model->cycle_dose0, model);
    model->cycle_state = BATCH_CYCLE_CYCLE_DOSE0;
    model->stage = 0;
}

/// Функция обработки модели batch_cycle (BatchCycle)
void BatchCycle_tick(BatchCycle *model) {
    assert(0 != model);
    if (model->state == BATCH_CYCLE_INIT) {
        model->state = BATCH_CYCLE_CYCLE;
    }
    switch (model->state) {
        case BATCH_CYCLE_CYCLE: {
            if (model->cycle_state == BATCH_CYCLE_CYCLE_DOSE0) {
                BatchCycleDose_tick(&model->cycle_dose0, model);
                if (BatchCycleDose_is_done(&model->cycle_dose0, model)) {
                    BatchCycleMix_init(&model->cycle_mix1, model);
                    model->cycle_state = BATCH_CYCLE_CYCLE_MIX1;
                    break;
                }
            } else if (model->cycle_state == BATCH_CYCLE_CYCLE_MIX1) {
                BatchCycleMix_tick(&model->cycle_mix1, model);
                if (BatchCycleMix_is_done(&model->cycle_mix1, model)) {
                    BatchCycleDrain_init(&model->cycle_drain2, model);
                    model->cycle_state = BATCH_CYCLE_CYCLE_DRAIN2;
                    break;
                }
            } else if (model->cycle_state == BATCH_CYCLE_CYCLE_DRAIN2) {
                BatchCycleDrain_tick(&model->cycle_drain2, model);
                if (BatchCycleDrain_is_done(&model->cycle_drain2, model)) {
                    model->state = BATCH_CYCLE_DONE;
                    break;
                }
            }
            break;
        }
        case BATCH_CYCLE_DONE: {
            (*model->write_bit)(BATCH_CYCLE_PORT_READY, 1, model->userdata);
            model->state = BATCH_CYCLE_END;
            break;
        }
        case BATCH_CYCLE_END: {
            break;
        }
        default: break;
    }
}

/// Функция сброса модели batch_cycle (BatchCycle)
void BatchCycle_reset(BatchCycle *model) {
    BatchCycle_init(model);
}

/// Функция проверки терминального состояния модели batch_cycle (BatchCycle)
bool BatchCycle_is_done(const BatchCycle *model) {
    return model->state == BATCH_CYCLE_END;
}

