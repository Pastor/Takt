#include "bit_access.h"
#include <assert.h>
#include <math.h>
/// Константы и порты модели bit_access (BitAccess)
#define PORT_BIT_ACCESS_BTN 0x200000
#define PORT_BIT_ACCESS_LED 0x100000
/// Model functions 'BitOps (BitAccess:BitOps)'
static void BitAccessBitOps_init(BitAccessBitOps *model, const BitAccess *main);
static void BitAccessBitOps_tick(BitAccessBitOps *model, const BitAccess *main);
static bool BitAccessBitOps_is_done(const BitAccessBitOps *model, const BitAccess *main);

/// Функция инициализации модели BitOps (BitAccess:BitOps)
void BitAccessBitOps_init(BitAccessBitOps *model, const BitAccess *main) {
    assert(0 != model);
    model->state = BIT_ACCESS_BIT_OPS_INIT;
    model->flags = 0;
}

/// Функция обработки модели BitOps (BitAccess:BitOps)
void BitAccessBitOps_tick(BitAccessBitOps *model, const BitAccess *main) {
    assert(0 != model);
    assert(0 != main);
    switch (model->state) {
        case BIT_ACCESS_BIT_OPS_INIT: {
            model->state = BIT_ACCESS_BIT_OPS_IDLE;
            break;
        }
        case BIT_ACCESS_BIT_OPS_ACTIVE: {
            (*main->write_bit)(PORT_BIT_ACCESS_LED, 7, (*main->read_bit)(PORT_BIT_ACCESS_BTN, 1, main->userdata), main->userdata);
            if (((model->flags >> 0) & 1u)) {
                model->state = BIT_ACCESS_BIT_OPS_IDLE;
                break;
            }
            break;
        }
        case BIT_ACCESS_BIT_OPS_IDLE: {
            model->flags = (model->flags & ~(1u << 0)) | (((*main->read_bit)(PORT_BIT_ACCESS_BTN, 0, main->userdata) & 1u) << 0);
            if (((model->flags >> 0) & 1u)) {
                model->state = BIT_ACCESS_BIT_OPS_ACTIVE;
                break;
            }
            break;
        }
        case BIT_ACCESS_BIT_OPS_END: {
            break;
        }
    }
}

/// Функция сброса модели BitOps (BitAccess:BitOps)
void BitAccessBitOps_reset(BitAccessBitOps *model, const BitAccess *main) {
    BitAccessBitOps_init(model, main);
}

/// Функция проверки терминального состояния модели BitOps (BitAccess:BitOps)
bool BitAccessBitOps_is_done(const BitAccessBitOps *model, const BitAccess *main) {
    return model->state == BIT_ACCESS_BIT_OPS_END;
}

/// Функция инициализации модели bit_access (BitAccess)
void BitAccess_init(BitAccess *model) {
    assert(0 != model);
    model->state = BIT_ACCESS_INIT;
}

/// Функция обработки модели bit_access (BitAccess)
void BitAccess_tick(BitAccess *model) {
    assert(0 != model);
    switch (model->state) {
        case BIT_ACCESS_INIT: {
            BitAccessBitOps_init(&model->m, model);
            model->state = BIT_ACCESS_M;
            break;
        }
        case BIT_ACCESS_M: {
            BitAccessBitOps_tick(&model->m, model);
            if (BitAccessBitOps_is_done(&model->m, model)) {
                model->state = BIT_ACCESS_END;
                break;
            }
            break;
        }
        case BIT_ACCESS_END: {
            break;
        }
    }
}

/// Функция сброса модели bit_access (BitAccess)
void BitAccess_reset(BitAccess *model) {
    BitAccess_init(model);
}

/// Функция проверки терминального состояния модели bit_access (BitAccess)
bool BitAccess_is_done(const BitAccess *model) {
    return model->state == BIT_ACCESS_END;
}

