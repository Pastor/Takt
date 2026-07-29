#ifndef PID_LAW_H__
#define PID_LAW_H__
#include <stdint.h>
#include <stdbool.h>

/* Forward declarations */
typedef struct PidLawPid PidLawPid;
typedef struct PidLaw PidLaw;

typedef enum {
    PID_LAW_PID_READY = 0,
} PidLaw_Out_BitPort;

// NOTICE: Определение констант для модели Pid (PidLaw:Pid)
/* Model Pid (PidLaw:Pid) */
struct PidLawPid {
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
        PID_LAW_PID_INIT,
        PID_LAW_PID_CONTROL,
        PID_LAW_PID_DONE,
        PID_LAW_PID_SETTLED,
        PID_LAW_PID_END
    } state;
};

// NOTICE: Определение констант для модели pid_law (PidLaw)
/* Model pid_law (PidLaw) */
struct PidLaw {
    // NOTICE: Определение переменных модели
    double ctrl;
    double meas;
    double target;
    enum {
        PID_LAW_INIT,
        PID_LAW_MAIN,
        PID_LAW_END
    } state;
    // NOTICE: Определение extend
    PidLawPid main;
    /// NOTICE: Функции портов ввода вывода
    void  *userdata;
    void  (*write_bit)(PidLaw_Out_BitPort port, bool val, void *userdata);
};

void PidLaw_init(PidLaw *main);
void PidLaw_tick(PidLaw *main);
void PidLaw_reset(PidLaw *main);
bool PidLaw_is_done(const PidLaw *main);
#endif
