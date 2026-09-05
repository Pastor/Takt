#include "elevator.h"

void door_close() {
    // FIXME: Not implement yet
}

void door_open() {
    // FIXME: Not implement yet
}
void motor_down() {
    // FIXME: Not implement yet
}

void motor_stop() {
    // FIXME: Not implement yet
}

void motor_up() {
    // FIXME: Not implement yet
}

void read_floor_sensors() {
    // FIXME: Not implement yet
}

void scan_cabin_buttons() {
    // FIXME: Not implement yet
}

void scan_floor_buttons() {
    // FIXME: Not implement yet
}

struct Ports {
};

static int64_t port_read_numeric(Elevator_In_NumericPort port, uint8_t index, void *userdata) {
    (void)index;
    struct Ports *ports = (struct Ports *) userdata;
    (void) port;
    (void) ports;
    return 0;
}

int main(void) {
    struct Ports ports = {};
    Elevator elevator = {
            .userdata = &ports,
            .read_numeric  = port_read_numeric};

    Elevator_init(&elevator);
    while (!Elevator_is_done(&elevator)) {
        Elevator_tick(&elevator);
    }
    Elevator_reset(&elevator);
    return 0;
}
