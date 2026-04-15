#include "ni6_enum_valid.h"
#include <assert.h>
#include <math.h>
/// Перечисления модели ni6_enum_valid (Ni6EnumValid)
#define ENUM_NI6_ENUM_VALID_EAST 2
#define ENUM_NI6_ENUM_VALID_NORTH 0
#define ENUM_NI6_ENUM_VALID_SOUTH 1
#define ENUM_NI6_ENUM_VALID_WEST 3
/// Model functions 'Robot (Ni6EnumValid:Robot)'
static void Ni6EnumValidRobot_init(Ni6EnumValidRobot *model, Ni6EnumValid *main);
static void Ni6EnumValidRobot_tick(Ni6EnumValidRobot *model, Ni6EnumValid *main);
static bool Ni6EnumValidRobot_is_done(const Ni6EnumValidRobot *model, Ni6EnumValid *main);

/// Функция инициализации модели Robot (Ni6EnumValid:Robot)
void Ni6EnumValidRobot_init(Ni6EnumValidRobot *model, Ni6EnumValid *main) {
    assert(0 != model);
    model->state = NI6_ENUM_VALID_ROBOT_INIT;
    model->dir = 2;
}

/// Функция обработки модели Robot (Ni6EnumValid:Robot)
void Ni6EnumValidRobot_tick(Ni6EnumValidRobot *model, Ni6EnumValid *main) {
    assert(0 != model);
    assert(0 != main);
    switch (model->state) {
        case NI6_ENUM_VALID_ROBOT_INIT: {
            model->dir = 1;
            model->state = NI6_ENUM_VALID_ROBOT_IDLE;
            break;
        }
        case NI6_ENUM_VALID_ROBOT_MOVING: {
            model->dir = 3;
            if (true) {
                model->dir = 1;
                model->state = NI6_ENUM_VALID_ROBOT_IDLE;
                break;
            }
            break;
        }
        case NI6_ENUM_VALID_ROBOT_IDLE: {
            if (true) {
                model->state = NI6_ENUM_VALID_ROBOT_MOVING;
                break;
            }
            break;
        }
        case NI6_ENUM_VALID_ROBOT_END: {
            break;
        }
    }
}

/// Функция сброса модели Robot (Ni6EnumValid:Robot)
void Ni6EnumValidRobot_reset(Ni6EnumValidRobot *model, Ni6EnumValid *main) {
    Ni6EnumValidRobot_init(model, main);
}

/// Функция проверки терминального состояния модели Robot (Ni6EnumValid:Robot)
bool Ni6EnumValidRobot_is_done(const Ni6EnumValidRobot *model, Ni6EnumValid *main) {
    return model->state == NI6_ENUM_VALID_ROBOT_END;
}

/// Функция инициализации модели ni6_enum_valid (Ni6EnumValid)
void Ni6EnumValid_init(Ni6EnumValid *model) {
    assert(0 != model);
    model->state = NI6_ENUM_VALID_INIT;
    model->heading = 0;
}

/// Функция обработки модели ni6_enum_valid (Ni6EnumValid)
void Ni6EnumValid_tick(Ni6EnumValid *model) {
    assert(0 != model);
    switch (model->state) {
        case NI6_ENUM_VALID_INIT: {
            Ni6EnumValidRobot_init(&model->m, model);
            model->state = NI6_ENUM_VALID_M;
            break;
        }
        case NI6_ENUM_VALID_M: {
            Ni6EnumValidRobot_tick(&model->m, model);
            if (Ni6EnumValidRobot_is_done(&model->m, model)) {
                model->state = NI6_ENUM_VALID_END;
                break;
            }
            break;
        }
        case NI6_ENUM_VALID_END: {
            break;
        }
    }
}

/// Функция сброса модели ni6_enum_valid (Ni6EnumValid)
void Ni6EnumValid_reset(Ni6EnumValid *model) {
    Ni6EnumValid_init(model);
}

/// Функция проверки терминального состояния модели ni6_enum_valid (Ni6EnumValid)
bool Ni6EnumValid_is_done(const Ni6EnumValid *model) {
    return model->state == NI6_ENUM_VALID_END;
}

