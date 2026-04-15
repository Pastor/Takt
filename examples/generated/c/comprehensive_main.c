#include "comprehensive.h"
#include <stdio.h>

/* Заглушка для read_bit — всегда возвращает false */
static bool stub_read_bit(int port, int bit, void *userdata) {
    (void)port; (void)bit; (void)userdata;
    return false;
}

/* Заглушка для write_bit — ничего не делает */
static void stub_write_bit(int port, int bit, bool value, void *userdata) {
    (void)port; (void)bit; (void)value; (void)userdata;
}

/* Заглушка для read_float */
static float stub_read_float(int port, int bit, void *userdata) {
    (void)port; (void)bit; (void)userdata;
    return 0.0f;
}

/* Заглушка для write_float */
static void stub_write_float(int port, int bit, float value, void *userdata) {
    (void)port; (void)bit; (void)value; (void)userdata;
}

void log_temp(uint8_t value) {
    printf("temp=%d\n", (int)value);
}

void log_count(uint8_t n) {
    printf("count=%d\n", (int)n);
}

int main(void) {
    Comprehensive ctrl;
    int i;

    ctrl.read_bit    = stub_read_bit;
    ctrl.write_bit   = stub_write_bit;
    ctrl.read_float  = stub_read_float;
    ctrl.write_float = stub_write_float;
    ctrl.userdata    = NULL;

    Comprehensive_init(&ctrl);

    for (i = 0; i < 1000 && !Comprehensive_is_done(&ctrl); i++) {
        Comprehensive_tick(&ctrl);
    }

    printf("Завершено за %d шагов\n", i);
    return 0;
}
