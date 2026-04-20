#include "comprehensive.h"
#include <stdio.h>

void log_temp(uint8_t value) {
    printf("temp=%d\n", (int)value);
}

void log_count(uint8_t n) {
    printf("count=%d\n", (int)n);
}

int main(void) {
    Comprehensive ctrl;
    int i;

    Comprehensive_init(&ctrl);

    for (i = 0; i < 1000 && !Comprehensive_is_done(&ctrl); i++) {
        Comprehensive_tick(&ctrl);
    }

    printf("Завершено за %d шагов\n", i);
    return 0;
}
