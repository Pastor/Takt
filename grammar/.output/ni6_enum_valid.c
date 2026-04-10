#include "ni6_enum_valid.h"
#include <math.h>
/// Перечисления модели ni6_enum_valid (Ni6EnumValid)
#define ENUM_NI6_ENUM_VALID_EAST 2
#define ENUM_NI6_ENUM_VALID_NORTH 0
#define ENUM_NI6_ENUM_VALID_SOUTH 1
#define ENUM_NI6_ENUM_VALID_WEST 3

void Ni6EnumValid_init(Ni6EnumValid *main) {
    main->state = NI6_ENUM_VALID_INIT;
}

void Ni6EnumValid_tick(Ni6EnumValid *main) {
}

void Ni6EnumValid_reset(Ni6EnumValid *main) {
    Ni6EnumValid_init(main);
}

bool Ni6EnumValid_is_done(const Ni6EnumValid *main) {
    return main->state == NI6_ENUM_VALID_M;
}

