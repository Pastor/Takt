#ifndef PID_HEATER_H__
#define PID_HEATER_H__
#include <stdint.h>
#include <stdbool.h>

/* Forward declarations */
typedef struct PidHeaterHeater PidHeaterHeater;
typedef struct PidHeaterPid PidHeaterPid;
typedef struct PidHeater PidHeater;

typedef enum {
    PID_HEATER_PID_PORT_READY = 0,
} PidHeater_Out_BitPort;

typedef enum {
    PID_HEATER_HEATER_PORT_TEMPERATURE = 0,
} PidHeater_Out_RationalPort;

// NOTICE: Определение констант для модели Heater (PidHeater:Heater)
/* Model Heater (PidHeater:Heater) */
struct PidHeaterHeater {
    // NOTICE: Определение переменных модели
    double release;
    double setpoint;
    enum {
        PID_HEATER_HEATER_INIT,
        PID_HEATER_HEATER_DONE,
        PID_HEATER_HEATER_HEATING,
        PID_HEATER_HEATER_HOLDING,
        PID_HEATER_HEATER_END
    } state;
};

// NOTICE: Определение констант для модели Pid (PidHeater:Pid)
/* Model Pid (PidHeater:Pid) */
struct PidHeaterPid {
    // NOTICE: Определение переменных модели
    double deriv;
    double eps;
    double err;
    double err_prev;
    double i_acc;
    double imax;
    double kd;
    double ki;
    double kp;
    double neg_imax;
    enum {
        PID_HEATER_PID_INIT,
        PID_HEATER_PID_CONTROL,
        PID_HEATER_PID_DONE,
        PID_HEATER_PID_SETTLED,
        PID_HEATER_PID_END
    } state;
};

// NOTICE: Определение констант для модели pid_heater (PidHeater)
/* Model pid_heater (PidHeater) */
struct PidHeater {
    // NOTICE: Определение переменных модели
    double ambient;
    double ctrl;
    double gain;
    double loss;
    double meas;
    double target;
    enum {
        PID_HEATER_INIT,
        PID_HEATER_FINISHED,
        PID_HEATER_PID_HEATER,
        PID_HEATER_END
    } state;
    // NOTICE: Определение extend
    struct {
        PidHeaterPid pid0;
        PidHeaterHeater heater1;
        enum {
            PID_HEATER_PID_HEATER_PARALLEL0_INIT,
            PID_HEATER_PID_HEATER_PARALLEL0_TICK,
            PID_HEATER_PID_HEATER_PARALLEL0_END
        } state;
    } pid_heater_parallel0;
    struct {
        PidHeaterPid pid0;
        PidHeaterHeater heater1;
        enum {
            PID_HEATER_PID_HEATER_PARALLEL1_INIT,
            PID_HEATER_PID_HEATER_PARALLEL1_TICK,
            PID_HEATER_PID_HEATER_PARALLEL1_END
        } state;
    } pid_heater_parallel1;
    enum {
        PID_HEATER_PID_HEATER_INIT,
        PID_HEATER_PID_HEATER_PARALLEL0,
        PID_HEATER_PID_HEATER_PARALLEL1,
        PID_HEATER_PID_HEATER_END
    } pid_heater_state;
    /// NOTICE: Функции портов ввода вывода
    void  *userdata;
    void  (*write_bit)(PidHeater_Out_BitPort port, bool val, void *userdata);
    void  (*write_float)(PidHeater_Out_RationalPort port, float val, void *userdata);
};

void PidHeater_init(PidHeater *main);
void PidHeater_tick(PidHeater *main);
void PidHeater_reset(PidHeater *main);
bool PidHeater_is_done(const PidHeater *main);
#endif
