#include "extend_complex.h"

bool has_flag(bool v) {
  return v;
}

struct Ports {
};

static void port_write_bit(int address, int bit, bool val, void *userdata) {
    struct Ports *ports = (struct Ports *) userdata;
    (void) address;
    (void) bit;
    (void) val;
    (void) ports;
}

static bool port_read_bit(int address, int bit, void *userdata) {
    struct Ports *ports = (struct Ports *) userdata;
    (void) address;
    (void) bit;
    (void) ports;
    return false;
}
static void port_write_float(int address, int bit, float val, void *userdata) {
    struct Ports *ports = (struct Ports *) userdata;
    (void) address;
    (void) bit;
    (void) val;
    (void) ports;
}
static float port_read_float(int address, int bit, void *userdata) {
    struct Ports *ports = (struct Ports *) userdata;
    (void) address;
    (void) bit;
    (void) ports;
    return 0.0f;
}

int main(void) {
    struct Ports ports = {};
    ExtendComplex extend_complex = {
      .x = 0, .y = 0,
      .userdata = &ports,
            .write_bit = port_write_bit,
            .read_bit = port_read_bit,
            .write_float = port_write_float,
            .read_float = port_read_float
    };

    ExtendComplex_init(&extend_complex);
    while (!ExtendComplex_is_done(&extend_complex)) {
        ExtendComplex_tick(&extend_complex);
    }
    ExtendComplex_reset(&extend_complex);
    return 0;
}
