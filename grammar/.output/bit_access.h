#ifndef BIT_ACCESS_H__
#define BIT_ACCESS_H__
#include <stdint.h>
#include <stdbool.h>

/* Forward declarations */
typedef struct BitAccessBitOps BitAccessBitOps;
typedef struct BitAccess BitAccess;

// NOTICE: Определение констант для модели BitOps (BitAccess:BitOps)
/* Model BitOps (BitAccess:BitOps) */
struct BitAccessBitOps {
    // NOTICE: Определение переменных модели
    uint8_t flags;
    enum {
        BIT_ACCESS_BIT_OPS_INIT,
        BIT_ACCESS_BIT_OPS_ACTIVE,
        BIT_ACCESS_BIT_OPS_IDLE,
        BIT_ACCESS_BIT_OPS_END
    } state;
};

// NOTICE: Определение констант для модели bit_access (BitAccess)
/* Model bit_access (BitAccess) */
struct BitAccess {
    // NOTICE: Определение переменных модели
    enum {
        BIT_ACCESS_INIT,
        BIT_ACCESS_M,
        BIT_ACCESS_END
    } state;
    // NOTICE: Определение extend
    BitAccessBitOps m;
    /// NOTICE: Функции портов ввода вывода
    void  *userdata;
    void  (*write_bit  )(int address, int bit, bool val, void *userdata);
    bool  (*read_bit   )(int address, int bit, void *userdata);
    void  (*write_float)(int address, int bit, float val, void *userdata);
    float (*read_float )(int address, int bit, void *userdata);
};

void BitAccess_init(BitAccess *main);
void BitAccess_tick(BitAccess *main);
void BitAccess_reset(BitAccess *main);
bool BitAccess_is_done(const BitAccess *main);
#endif
