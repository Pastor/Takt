#include "elevator.h"

struct Ports {
};

static void port_write_bit(const int address, const int bit, const bool val, void *userdata) {
    (void) address;
    (void) bit;
    (void) val;
    (void) userdata;
}

static bool port_read_bit(const int address, const int bit, void *userdata) {
    (void) address;
    (void) bit;
    (void) userdata;
    return false;
}
static void port_write_float(const int address, const int bit, const float val, void *userdata) {
    (void) address;
    (void) bit;
    (void) val;
    (void) userdata;
}
static float port_read_float(const int address, const int bit, void *userdata) {
    (void) address;
    (void) bit;
    (void) userdata;
    return 0.0f;
}

int main(void) {
    struct Ports ports = {};
    struct Elevator elevator = {.userdata = &ports, .write_bit = port_write_bit, .read_bit = port_read_bit, .write_float = port_write_float, .read_float = port_read_float};

    Elevator_init(&elevator);
    while (!Elevator_is_done(&elevator)) {
        Elevator_tick(&elevator);
    }
    Elevator_reset(&elevator);
    return 0;
}
