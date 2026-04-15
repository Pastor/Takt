#include "traffic_light.h"
#include <assert.h>
#include <math.h>
/// Model functions 'TrafficLight (TrafficLight:TrafficLight)'
static void TrafficLightTrafficLight_init(TrafficLightTrafficLight *model, TrafficLight *main);
static void TrafficLightTrafficLight_tick(TrafficLightTrafficLight *model, TrafficLight *main);
static bool TrafficLightTrafficLight_is_done(const TrafficLightTrafficLight *model, TrafficLight *main);

/// Функция инициализации модели TrafficLight (TrafficLight:TrafficLight)
void TrafficLightTrafficLight_init(TrafficLightTrafficLight *model, TrafficLight *main) {
    assert(0 != model);
    model->state = TRAFFIC_LIGHT_TRAFFIC_LIGHT_INIT;
    model->timer = 0;
}

/// Функция обработки модели TrafficLight (TrafficLight:TrafficLight)
void TrafficLightTrafficLight_tick(TrafficLightTrafficLight *model, TrafficLight *main) {
    assert(0 != model);
    assert(0 != main);
    switch (model->state) {
        case TRAFFIC_LIGHT_TRAFFIC_LIGHT_INIT: {
            model->timer = 0;
            model->state = TRAFFIC_LIGHT_TRAFFIC_LIGHT_RED;
            break;
        }
        case TRAFFIC_LIGHT_TRAFFIC_LIGHT_YELLOW: {
            model->timer = model->timer + 1;
            if (model->timer > 3) {
                model->timer = 0;
                model->state = TRAFFIC_LIGHT_TRAFFIC_LIGHT_GREEN;
                break;
            }
            break;
        }
        case TRAFFIC_LIGHT_TRAFFIC_LIGHT_RED: {
            model->timer = model->timer + 1;
            if (model->timer > 10) {
                model->timer = 0;
                model->state = TRAFFIC_LIGHT_TRAFFIC_LIGHT_YELLOW;
                break;
            }
            break;
        }
        case TRAFFIC_LIGHT_TRAFFIC_LIGHT_GREEN: {
            model->timer = model->timer + 1;
            if (model->timer > 15) {
                model->timer = 0;
                model->state = TRAFFIC_LIGHT_TRAFFIC_LIGHT_RED;
                break;
            }
            break;
        }
        case TRAFFIC_LIGHT_TRAFFIC_LIGHT_END: {
            break;
        }
    }
}

/// Функция сброса модели TrafficLight (TrafficLight:TrafficLight)
void TrafficLightTrafficLight_reset(TrafficLightTrafficLight *model, TrafficLight *main) {
    TrafficLightTrafficLight_init(model, main);
}

/// Функция проверки терминального состояния модели TrafficLight (TrafficLight:TrafficLight)
bool TrafficLightTrafficLight_is_done(const TrafficLightTrafficLight *model, TrafficLight *main) {
    return model->state == TRAFFIC_LIGHT_TRAFFIC_LIGHT_END;
}

/// Функция инициализации модели traffic_light (TrafficLight)
void TrafficLight_init(TrafficLight *model) {
    assert(0 != model);
    model->state = TRAFFIC_LIGHT_INIT;
}

/// Функция обработки модели traffic_light (TrafficLight)
void TrafficLight_tick(TrafficLight *model) {
    assert(0 != model);
    switch (model->state) {
        case TRAFFIC_LIGHT_INIT: {
            TrafficLightTrafficLight_init(&model->entry, model);
            model->state = TRAFFIC_LIGHT_ENTRY;
            break;
        }
        case TRAFFIC_LIGHT_ENTRY: {
            TrafficLightTrafficLight_tick(&model->entry, model);
            if (TrafficLightTrafficLight_is_done(&model->entry, model)) {
                model->state = TRAFFIC_LIGHT_END;
                break;
            }
            break;
        }
        case TRAFFIC_LIGHT_END: {
            break;
        }
    }
}

/// Функция сброса модели traffic_light (TrafficLight)
void TrafficLight_reset(TrafficLight *model) {
    TrafficLight_init(model);
}

/// Функция проверки терминального состояния модели traffic_light (TrafficLight)
bool TrafficLight_is_done(const TrafficLight *model) {
    return model->state == TRAFFIC_LIGHT_END;
}

