(*
 * Порождено компилятором Lam (lamc) — цель: Structured Text (IEC 61131-3).
 * Не редактировать вручную: файл перезаписывается при каждой генерации.
 *)

FUNCTION_BLOCK FloatRegulatorFloatRegulator
VAR_OUTPUT
    is_done : BOOL;
    ready : BOOL;
END_VAR
VAR
    state : USINT := 0;
    half : LREAL := 0.5;
    near : LREAL := 9.5;
    setpoint : LREAL := 10.0;
    value : LREAL := 0.0;
END_VAR
    CASE state OF
        0: (* INIT *)
            state := 1; (* Adjust *)
        1: (* Adjust *)
            value := value + ((setpoint - value) * half);
            IF value >= near THEN
                state := 3; (* Settled *)
            END_IF;
        2: (* Done *)
            ready := TRUE;
            state := 4; (* END *)
        3: (* Settled *)
            value := setpoint;
            state := 2; (* Done *)
        4: (* END *)
            state := 4;
    END_CASE;
    is_done := state = 4;
END_FUNCTION_BLOCK

FUNCTION_BLOCK FloatRegulator
VAR_OUTPUT
    is_done : BOOL;
END_VAR
VAR
    state : USINT := 0;
    float_regulator0 : FloatRegulatorFloatRegulator;
END_VAR
    CASE state OF
        0: (* INIT *)
            state := 1; (* Main *)
        1: (* Main *)
            float_regulator0();
            IF float_regulator0.is_done THEN
                state := 2; (*  *)
            END_IF;
        2: (* END *)
            state := 2;
    END_CASE;
    is_done := state = 2;
END_FUNCTION_BLOCK

