#include "traffic_light.h"
#include <math.h>

void TrafficLight_init(TrafficLight *main) {
    main->state = TRAFFIC_LIGHT_INIT;
}

void TrafficLight_tick(TrafficLight *main) {
}

void TrafficLight_reset(TrafficLight *main) {
    TrafficLight_init(main);
}

bool TrafficLight_is_done(const TrafficLight *main) {
    return main->state == TRAFFIC_LIGHT_ENTRY;
}

