(*
 * Порождено компилятором Takt (taktc) — цель: Structured Text (IEC 61131-3).
 * Не редактировать вручную: файл перезаписывается при каждой генерации.
 *)

TYPE
    PidState :
    STRUCT
        kp : LREAL;
        ki : LREAL;
        kd : LREAL;
        ts : LREAL;
        out_min : LREAL;
        out_max : LREAL;
        i_acc : LREAL;
        err_prev : LREAL;
        output : LREAL;
    END_STRUCT;
END_TYPE

FUNCTION_BLOCK PidHeaterHeater
VAR_OUTPUT
    is_done : BOOL;
    temperature : LREAL := 0.0;
END_VAR
VAR_IN_OUT
    ambient : LREAL;
    ctrl : LREAL;
    gain : LREAL;
    loss : LREAL;
    meas : LREAL;
    target : LREAL;
END_VAR
VAR
    state : USINT := 0;
    err : LREAL := 0.0;
    i_new : LREAL := 0.0;
    loop_pid : PidState;
    raw : LREAL := 0.0;
    release : LREAL := 38.0;
    setpoint : LREAL := 40.0;
END_VAR
    IF state = 0 THEN (* первый скан *)
        target := setpoint;
        state := 2; (* Heating *)
    END_IF;
    CASE state OF
        1: (* Done *)
            state := 4; (* END *)
        2: (* Heating *)
            err := target - meas;
            i_new := (loop_pid.i_acc) + (((loop_pid.ki) * err) * (loop_pid.ts));
            raw := (((loop_pid.kp) * err) + i_new) + (((loop_pid.kd) * (err - (loop_pid.err_prev))) / (loop_pid.ts));
            IF raw > (loop_pid.out_max) THEN
                ctrl := loop_pid.out_max;
                IF err <= 0.0 THEN
                    loop_pid.i_acc := i_new;
                END_IF;
            ELSE
                IF raw < (loop_pid.out_min) THEN
                    ctrl := loop_pid.out_min;
                    IF err >= 0.0 THEN
                        loop_pid.i_acc := i_new;
                    END_IF;
                ELSE
                    ctrl := raw;
                    loop_pid.i_acc := i_new;
                END_IF;
            END_IF;
            loop_pid.err_prev := err;
            loop_pid.output := ctrl;
            meas := (meas + (gain * ctrl)) - (loss * (meas - ambient));
            temperature := meas;
            IF err <= 0.0 THEN
                ctrl := 0.0;
                state := 3; (* Holding *)
            END_IF;
        3: (* Holding *)
            meas := meas - (loss * (meas - ambient));
            temperature := meas;
            IF meas <= release THEN
                state := 1; (* Done *)
            END_IF;
        4: (* END *)
            state := 4;
    END_CASE;
    is_done := state = 4;
END_FUNCTION_BLOCK

FUNCTION_BLOCK PidHeater
VAR_OUTPUT
    is_done : BOOL;
END_VAR
VAR
    state : USINT := 0;
    pid_heater_heater0 : PidHeaterHeater;
    pid_heater_heater1 : PidHeaterHeater := (setpoint := 55.0, release := 52.0);
    pid_heater_step : USINT;
    ambient : LREAL := 18.0;
    ctrl : LREAL := 0.0;
    gain : LREAL := 0.5;
    loss : LREAL := 0.05;
    meas : LREAL := 25.0;
    target : LREAL := 80.0;
END_VAR
    IF state = 0 THEN (* первый скан *)
        state := 2; (* PidHeater *)
    END_IF;
    CASE state OF
        1: (* Finished *)
            state := 3; (* END *)
        2: (* PidHeater *)
            CASE pid_heater_step OF
                0:
                    pid_heater_heater0(ambient := ambient, ctrl := ctrl, gain := gain, loss := loss, meas := meas, target := target);
                    IF pid_heater_heater0.is_done THEN
                        pid_heater_step := 1;
                    END_IF;
                1:
                    pid_heater_heater1(ambient := ambient, ctrl := ctrl, gain := gain, loss := loss, meas := meas, target := target);
                    IF pid_heater_heater1.is_done THEN
                        pid_heater_step := 2;
                    END_IF;
                2: (* конкатенация завершена *)
                    state := 1; (* Finished *)
            END_CASE;
        3: (* END *)
            state := 3;
    END_CASE;
    is_done := state = 3;
END_FUNCTION_BLOCK

