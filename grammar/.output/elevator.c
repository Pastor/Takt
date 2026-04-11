#include "elevator.h"
#include <math.h>
/// Константы и порты модели elevator (Elevator)
#define PORT_ELEVATOR_BTNS_CAB_HI 0x30000001
#define PORT_ELEVATOR_BTNS_CAB_LO 0x30000000
#define PORT_ELEVATOR_BTNS_FLOOR_1 0x20000000
#define PORT_ELEVATOR_BTNS_FLOOR_2 0x20000001
#define PORT_ELEVATOR_BTNS_FLOOR_3 0x20000002
#define PORT_ELEVATOR_BTNS_FLOOR_4 0x20000003
#define PORT_ELEVATOR_BTNS_FLOOR_5 0x20000004
#define PORT_ELEVATOR_BTNS_FLOOR_6 0x20000005
#define PORT_ELEVATOR_BTNS_FLOOR_7 0x20000006
#define PORT_ELEVATOR_BTNS_FLOOR_8 0x20000007
#define PORT_ELEVATOR_BTNS_FLOOR_9 0x20000008
#define PORT_ELEVATOR_SENSORS_1 0x10000000
#define PORT_ELEVATOR_SENSORS_2 0x10000001
#define PORT_ELEVATOR_SENSORS_3 0x10000002
#define PORT_ELEVATOR_SENSORS_4 0x10000003
#define PORT_ELEVATOR_SENSORS_5 0x10000004
#define PORT_ELEVATOR_SENSORS_6 0x10000005
#define PORT_ELEVATOR_SENSORS_7 0x10000006
#define PORT_ELEVATOR_SENSORS_8 0x10000007
#define PORT_ELEVATOR_SENSORS_9 0x10000008
#define PORT_ELEVATOR_SENSORS_CAB 0x10000009
/// Перечисления модели elevator (Elevator)
#define ENUM_ELEVATOR_BOTTOM 80
#define ENUM_ELEVATOR_TOP 81
/// Перечисления модели Engine (Elevator:Engine)
#define ENUM_ELEVATOR_ENGINE_CLOSING 671
#define ENUM_ELEVATOR_ENGINE_IDLE 670
///Внешние функции
extern void door_close();
extern void door_open();
extern void motor_down();
extern void motor_stop();
extern void motor_up();
extern void read_floor_sensors();
extern void scan_cabin_buttons();
extern void scan_floor_buttons();

void Elevator_init(Elevator *main) {
    main->state = ELEVATOR_INIT;
}

void Elevator_tick(Elevator *main) {
}

void Elevator_reset(Elevator *main) {
    Elevator_init(main);
}

bool Elevator_is_done(const Elevator *main) {
    return main->state == ELEVATOR_END;
}

