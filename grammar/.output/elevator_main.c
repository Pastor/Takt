#include "elevator.h"

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
    struct Elevator elevator = {
            .userdata = &ports,
            .write_bit = port_write_bit,
            .read_bit = port_read_bit,
            .write_float = port_write_float,
            .read_float = port_read_float};

    Elevator_init(&elevator);
    while (!Elevator_is_done(&elevator)) {
        Elevator_tick(&elevator);
    }
    Elevator_reset(&elevator);
    return 0;
}
