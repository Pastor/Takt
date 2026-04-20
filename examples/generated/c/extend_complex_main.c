#include "extend_complex.h"

bool has_flag(bool v) {
    return v;
}

struct Ports {
};

static void port_write_bit(ExtendComplex_BitPort port, bool val, void *userdata) {
    struct Ports *ports = (struct Ports *) userdata;
    (void) port;
    (void) val;
    (void) ports;
}

static bool port_read_bit(ExtendComplex_BitPort port, void *userdata) {
    struct Ports *ports = (struct Ports *) userdata;
    (void) port;
    (void) ports;
    return false;
}

int main(void) {
    struct Ports ports = {};
    ExtendComplex extend_complex = {
      .x = 0, .y = 0,
      .userdata = &ports,
      .write_bit = port_write_bit,
      .read_bit = port_read_bit};

    ExtendComplex_init(&extend_complex);
    while (!ExtendComplex_is_done(&extend_complex)) {
        ExtendComplex_tick(&extend_complex);
    }
    ExtendComplex_reset(&extend_complex);
    return 0;
}
