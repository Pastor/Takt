// Драйвер проверки порождённого C: ПИД-регулятор на q(8, 8)
// (examples/pid_regulator.takt). Сходится с anti-windup и завершается, поднимая
// порт `ready`. Драйвер задаёт колбэк порта и крутит такты до терминала.
#include "pid_regulator.h"
#include <stdio.h>

static void write_bit(PidRegulator_Out_BitPort port, bool val, void *userdata) {
    (void)port;
    (void)userdata;
    printf("ready=%d\n", (int)val);
}

int main(void) {
    PidRegulator fsm;
    int i;

    fsm.write_bit = write_bit;
    fsm.userdata = NULL;
    PidRegulator_init(&fsm);

    for (i = 0; i < 400 && !PidRegulator_is_done(&fsm); i++) {
        PidRegulator_tick(&fsm);
    }

    printf("ПИД: завершено за %d шагов\n", i);
    return 0;
}
