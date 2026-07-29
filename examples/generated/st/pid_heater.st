(*
 * Порождено компилятором Takt (taktc) — цель: Structured Text (IEC 61131-3).
 * Не редактировать вручную: файл перезаписывается при каждой генерации.
 *)

FUNCTION_BLOCK PidHeaterHeater
VAR_OUTPUT
    is_done : BOOL;
    temperature : LREAL;
END_VAR
VAR_IN_OUT
    ambient : LREAL;
    ctrl : LREAL;
    gain : LREAL;
    loss : LREAL;
    meas : LREAL;
    release : LREAL;
END_VAR
VAR
    state : USINT := 0;
END_VAR
    CASE state OF
        0: (* INIT *)
            state := 2; (* Heating *)
        1: (* Done *)
            state := 4; (* END *)
        2: (* Heating *)
            meas := (meas + (gain * ctrl)) - (loss * (meas - ambient));
            temperature := meas;
            IF ctrl <= 0.0 THEN
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

FUNCTION_BLOCK PidHeaterPid
VAR_OUTPUT
    is_done : BOOL;
    ready : BOOL;
END_VAR
VAR_IN_OUT
    ctrl : LREAL;
    meas : LREAL;
    target : LREAL;
END_VAR
VAR
    state : USINT := 0;
    deriv : LREAL := 0.0;
    eps : LREAL := 0.5;
    err : LREAL := 0.0;
    err_prev : LREAL := 0.0;
    i_acc : LREAL := 0.0;
    imax : LREAL := 32.0;
    kd : LREAL := 0.25;
    ki : LREAL := 0.0625;
    kp : LREAL := 0.5;
    neg_imax : LREAL := -32.0;
END_VAR
    CASE state OF
        0: (* INIT *)
            state := 1; (* Control *)
        1: (* Control *)
            err := target - meas;
            i_acc := i_acc + err;
            IF i_acc > imax THEN
                i_acc := imax;
            END_IF;
            IF i_acc < neg_imax THEN
                i_acc := neg_imax;
            END_IF;
            deriv := err - err_prev;
            ctrl := ((kp * err) + (ki * i_acc)) + (kd * deriv);
            err_prev := err;
            IF err < eps THEN
                ctrl := 0.0;
                ready := TRUE;
                state := 3; (* Settled *)
            END_IF;
        2: (* Done *)
            state := 4; (* END *)
        3: (* Settled *)
            state := 2; (* Done *)
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
    pid0 : PidHeaterPid;
    heater1 : PidHeaterHeater;
    ambient : LREAL := 18.0;
    ctrl : LREAL := 0.0;
    gain : LREAL := 0.5;
    loss : LREAL := 0.05;
    meas : LREAL := 0.0;
    release : LREAL := 38.0;
    target : LREAL := 40.0;
END_VAR
    CASE state OF
        0: (* INIT *)
            state := 1; (* PidHeater *)
        1: (* PidHeater *)
            pid0(ctrl := ctrl, meas := meas, target := target);
            heater1(ambient := ambient, ctrl := ctrl, gain := gain, loss := loss, meas := meas, release := release);
            IF pid0.is_done AND heater1.is_done THEN
                state := 2; (*  *)
            END_IF;
        2: (* END *)
            state := 2;
    END_CASE;
    is_done := state = 2;
END_FUNCTION_BLOCK

