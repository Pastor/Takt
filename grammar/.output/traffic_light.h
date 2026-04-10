#ifndef TRAFFIC_LIGHT_H__
#define TRAFFIC_LIGHT_H__
#include <stdint.h>
#include <stdbool.h>

/* Forward declarations */
typedef struct TrafficLightTrafficLight TrafficLightTrafficLight;
typedef struct TrafficLight TrafficLight;

// NOTICE: Определение констант для модели TrafficLight (TrafficLight:TrafficLight)
/* Model TrafficLight (TrafficLight:TrafficLight) */
struct TrafficLightTrafficLight {
    // NOTICE: Определение переменных модели
    uint8_t timer;
    enum {
        TRAFFIC_LIGHT_TRAFFIC_LIGHT_INIT,
        TRAFFIC_LIGHT_TRAFFIC_LIGHT_GREEN,
        TRAFFIC_LIGHT_TRAFFIC_LIGHT_RED,
        TRAFFIC_LIGHT_TRAFFIC_LIGHT_YELLOW,
        TRAFFIC_LIGHT_TRAFFIC_LIGHT_END
    } state;
};

// NOTICE: Определение констант для модели traffic_light (TrafficLight)
/* Model traffic_light (TrafficLight) */
struct TrafficLight {
    // NOTICE: Определение переменных модели
    enum {
        TRAFFIC_LIGHT_INIT,
        TRAFFIC_LIGHT_ENTRY,
        TRAFFIC_LIGHT_END
    } state;
    // NOTICE: Определение extend
    TrafficLightTrafficLight entry;
    /// NOTICE: Функции портов ввода вывода
    void  *userdata;
    void  (*write_bit  )(int address, int bit, bool val, void *userdata);
    bool  (*read_bit   )(int address, int bit, void *userdata);
    void  (*write_float)(int address, int bit, float val, void *userdata);
    float (*read_float )(int address, int bit, void *userdata);
};

void TrafficLight_init(TrafficLight *main);
void TrafficLight_tick(TrafficLight *main);
void TrafficLight_reset(TrafficLight *main);
bool TrafficLight_is_done(const TrafficLight *main);
#endif
