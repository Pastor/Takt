(*
 * Порождено компилятором Takt (taktc) — цель: Structured Text (IEC 61131-3).
 * Не редактировать вручную: файл перезаписывается при каждой генерации.
 *)

FUNCTION_BLOCK PidLawPid
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
    IF state = 0 THEN (* первый скан *)
        neg_imax := 0.0 - imax;
        state := 1; (* Control *)
    END_IF;
    CASE state OF
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

FUNCTION_BLOCK PidLaw
VAR_OUTPUT
    is_done : BOOL;
END_VAR
VAR
    state : USINT := 0;
    pid0 : PidLawPid;
    ctrl : LREAL := 0.0;
    meas : LREAL := 0.0;
    target : LREAL := 40.0;
END_VAR
    IF state = 0 THEN (* первый скан *)
        state := 1; (* Main *)
    END_IF;
    CASE state OF
        1: (* Main *)
            pid0(ctrl := ctrl, meas := meas, target := target);
            IF pid0.is_done THEN
                state := 2; (*  *)
            END_IF;
        2: (* END *)
            state := 2;
    END_CASE;
    is_done := state = 2;
END_FUNCTION_BLOCK

