#ifndef NI6_ENUM_VALID_H__
#define NI6_ENUM_VALID_H__
#include <stdint.h>
#include <stdbool.h>

/* Forward declarations */
typedef struct Ni6EnumValidRobot Ni6EnumValidRobot;
typedef struct Ni6EnumValid Ni6EnumValid;

// NOTICE: Определение констант для модели Robot (Ni6EnumValid:Robot)
/* Model Robot (Ni6EnumValid:Robot) */
struct Ni6EnumValidRobot {
    // NOTICE: Определение переменных модели
    uint8_t dir;
    enum {
        NI6_ENUM_VALID_ROBOT_INIT,
        NI6_ENUM_VALID_ROBOT_IDLE,
        NI6_ENUM_VALID_ROBOT_MOVING,
        NI6_ENUM_VALID_ROBOT_END
    } state;
};

// NOTICE: Определение констант для модели ni6_enum_valid (Ni6EnumValid)
/* Model ni6_enum_valid (Ni6EnumValid) */
struct Ni6EnumValid {
    // NOTICE: Определение переменных модели
    uint8_t heading;
    enum {
        NI6_ENUM_VALID_INIT,
        NI6_ENUM_VALID_M,
        NI6_ENUM_VALID_END
    } state;
    // NOTICE: Определение extend
    Ni6EnumValidRobot m;
    /// NOTICE: Функции портов ввода вывода
    void  *userdata;
    void  (*write_bit  )(int address, int bit, bool val, void *userdata);
    bool  (*read_bit   )(int address, int bit, void *userdata);
    void  (*write_float)(int address, int bit, float val, void *userdata);
    float (*read_float )(int address, int bit, void *userdata);
};

void Ni6EnumValid_init(Ni6EnumValid *main);
void Ni6EnumValid_tick(Ni6EnumValid *main);
void Ni6EnumValid_reset(Ni6EnumValid *main);
bool Ni6EnumValid_is_done(const Ni6EnumValid *main);
#endif
