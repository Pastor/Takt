#ifndef PID_REGULATOR_H__
#define PID_REGULATOR_H__
#include <stdint.h>
#include <stdbool.h>

/* Forward declarations */
typedef struct PidRegulatorPid PidRegulatorPid;
typedef struct PidRegulator PidRegulator;

typedef enum {
    PID_REGULATOR_PID_READY = 0,
} PidRegulator_Out_BitPort;

// NOTICE: Определение констант для модели Pid (PidRegulator:Pid)
/* Model Pid (PidRegulator:Pid) */
struct PidRegulatorPid {
    // NOTICE: Определение переменных модели
    int16_t ctrl;
    int16_t deriv;
    int16_t eps;
    int16_t err;
    int16_t err_prev;
    int16_t i_acc;
    int16_t imax;
    int16_t kd;
    int16_t ki;
    int16_t kp;
    int16_t kplant;
    int16_t meas;
    int16_t neg_imax;
    int16_t target;
    enum {
        PID_REGULATOR_PID_INIT,
        PID_REGULATOR_PID_CONTROL,
        PID_REGULATOR_PID_DONE,
        PID_REGULATOR_PID_SETTLED,
        PID_REGULATOR_PID_END
    } state;
};

// NOTICE: Определение констант для модели pid_regulator (PidRegulator)
/* Model pid_regulator (PidRegulator) */
struct PidRegulator {
    // NOTICE: Определение переменных модели
    enum {
        PID_REGULATOR_INIT,
        PID_REGULATOR_MAIN,
        PID_REGULATOR_END
    } state;
    // NOTICE: Определение extend
    PidRegulatorPid main;
    /// NOTICE: Функции портов ввода вывода
    void  *userdata;
    void  (*write_bit)(PidRegulator_Out_BitPort port, bool val, void *userdata);
};

void PidRegulator_init(PidRegulator *main);
void PidRegulator_tick(PidRegulator *main);
void PidRegulator_reset(PidRegulator *main);
bool PidRegulator_is_done(const PidRegulator *main);
#endif
