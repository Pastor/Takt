(*
 * Порождено компилятором Lam (lamc) — цель: Structured Text (IEC 61131-3).
 * Не редактировать вручную: файл перезаписывается при каждой генерации.
 *)

FUNCTION LAM_Q_FLOORDIV : LINT
VAR_INPUT
    x : LINT;
    d : LINT;
END_VAR
VAR
    q : LINT;
END_VAR
    q := x / d;
    IF (x MOD d <> 0) AND ((x < 0) <> (d < 0)) THEN
        LAM_Q_FLOORDIV := q - 1;
    ELSE
        LAM_Q_FLOORDIV := q;
    END_IF;
END_FUNCTION

FUNCTION_BLOCK PidRegulatorPid
VAR_OUTPUT
    is_done : BOOL;
    ready : BOOL;
END_VAR
VAR
    state : USINT := 0;
    ctrl : INT := 0;
    deriv : INT := 0;
    eps : INT := 32;
    err : INT := 0;
    err_prev : INT := 0;
    i_acc : INT := 0;
    imax : INT := 8192;
    kd : INT := 64;
    ki : INT := 16;
    kp : INT := 128;
    kplant : INT := 128;
    meas : INT := 0;
    neg_imax : INT := -8192;
    target : INT := 2048;
END_VAR
    CASE state OF
        0: (* INIT *)
            state := 1; (* Control *)
        1: (* Control *)
            err := LINT_TO_INT(INT_TO_LINT(target) - INT_TO_LINT(meas));
            i_acc := LINT_TO_INT(INT_TO_LINT(i_acc) + INT_TO_LINT(err));
            IF i_acc > imax THEN
                i_acc := imax;
            END_IF;
            IF i_acc < neg_imax THEN
                i_acc := neg_imax;
            END_IF;
            deriv := LINT_TO_INT(INT_TO_LINT(err) - INT_TO_LINT(err_prev));
            ctrl := LINT_TO_INT(INT_TO_LINT(LINT_TO_INT(INT_TO_LINT(LINT_TO_INT(LAM_Q_FLOORDIV(INT_TO_LINT(kp) * INT_TO_LINT(err), 256))) + INT_TO_LINT(LINT_TO_INT(LAM_Q_FLOORDIV(INT_TO_LINT(ki) * INT_TO_LINT(i_acc), 256))))) + INT_TO_LINT(LINT_TO_INT(LAM_Q_FLOORDIV(INT_TO_LINT(kd) * INT_TO_LINT(deriv), 256))));
            meas := LINT_TO_INT(INT_TO_LINT(meas) + INT_TO_LINT(LINT_TO_INT(LAM_Q_FLOORDIV(INT_TO_LINT(kplant) * INT_TO_LINT(ctrl), 256))));
            err_prev := err;
            IF err < eps THEN
                state := 3; (* Settled *)
            END_IF;
        2: (* Done *)
            ready := TRUE;
            state := 4; (* END *)
        3: (* Settled *)
            meas := target;
            state := 2; (* Done *)
        4: (* END *)
            state := 4;
    END_CASE;
    is_done := state = 4;
END_FUNCTION_BLOCK

FUNCTION_BLOCK PidRegulator
VAR_OUTPUT
    is_done : BOOL;
END_VAR
VAR
    state : USINT := 0;
    pid0 : PidRegulatorPid;
END_VAR
    CASE state OF
        0: (* INIT *)
            state := 1; (* Main *)
        1: (* Main *)
            pid0();
            IF pid0.is_done THEN
                state := 2; (*  *)
            END_IF;
        2: (* END *)
            state := 2;
    END_CASE;
    is_done := state = 2;
END_FUNCTION_BLOCK

