(*
 * Порождено компилятором Lam (lamc) — цель: Structured Text (IEC 61131-3).
 * Не редактировать вручную: файл перезаписывается при каждой генерации.
 *)

FUNCTION clamp_temp : USINT
VAR_INPUT
    value : USINT;
END_VAR
VAR_IN_OUT
    temperature : USINT;
END_VAR
VAR CONSTANT
    MAX_TEMP : USINT := 100;
END_VAR
    IF value > MAX_TEMP THEN
        clamp_temp := MAX_TEMP;
        RETURN;
    END_IF;
    IF value < temperature THEN
        clamp_temp := temperature;
        RETURN;
    END_IF;
    clamp_temp := value;
    RETURN;
END_FUNCTION

FUNCTION increment : USINT
VAR_INPUT
    n : USINT;
END_VAR
    increment := n + 1;
    RETURN;
END_FUNCTION

FUNCTION log_count : USINT
VAR_INPUT
    n : USINT;
END_VAR
    (* extern fn log_count — заглушка (ST-009) *)
    log_count := 0;
END_FUNCTION

FUNCTION log_temp : USINT
VAR_INPUT
    value : USINT;
END_VAR
    (* extern fn log_temp — заглушка (ST-009) *)
    log_temp := 0;
END_FUNCTION

FUNCTION_BLOCK ComprehensiveController
VAR_OUTPUT
    is_done : BOOL;
END_VAR
VAR
    state : USINT := 0;
    i : USINT;
    _st_discard_usint : USINT;
    delta : USINT;
    boost : USINT;
    count : USINT := 0;
    mode : USINT := 0;
    temperature : USINT := 0;
END_VAR
VAR CONSTANT
    Mode_Auto : USINT := 0;
    Mode_Manual : USINT := 1;
    Mode_Emergency : USINT := 2;
    MAX_COUNT : USINT := 10;
    MAX_TEMP : USINT := 100;
END_VAR
    CASE state OF
        0: (* INIT *)
            count := 0;
            temperature := 0;
            state := 4; (* Idle *)
        1: (* Cooling *)
            i := 0;
            WHILE i < 3 DO
                IF temperature > 0 THEN
                    temperature := temperature - 1;
                END_IF;
                i := i + 1;
            END_WHILE;
            _st_discard_usint := log_count(count);
            IF temperature <= 0 THEN
                temperature := 0;
                count := 0;
                state := 2; (* Done *)
            ELSIF temperature > 0 THEN
                count := 0;
                temperature := 0;
                state := 4; (* Idle *)
            END_IF;
        2: (* Done *)
            state := 5; (* END *)
        3: (* Heating *)
            WHILE count < 3 DO
                count := count + 1;
            END_WHILE;
            IF mode = 0 THEN
                _st_discard_usint := log_temp(temperature);
            ELSIF mode = 1 THEN
                _st_discard_usint := log_count(count);
            ELSE
                count := 0;
            END_IF;
            IF temperature > MAX_TEMP THEN
                state := 1; (* Cooling *)
            ELSIF count >= MAX_COUNT THEN
                count := 0;
                temperature := 0;
                state := 4; (* Idle *)
            END_IF;
        4: (* Idle *)
            delta := 1;
            temperature := temperature + delta;
            _st_discard_usint := log_temp(temperature);
            IF temperature > 10 THEN
                count := increment(count);
                boost := 5;
                temperature := temperature + boost;
                state := 3; (* Heating *)
            END_IF;
        5: (* END *)
            state := 5;
    END_CASE;
    is_done := state = 5;
END_FUNCTION_BLOCK

FUNCTION_BLOCK Comprehensive
VAR_OUTPUT
    is_done : BOOL;
END_VAR
VAR
    state : USINT := 0;
    controller0 : ComprehensiveController;
END_VAR
VAR CONSTANT
    Mode_Auto : USINT := 0;
    Mode_Manual : USINT := 1;
    Mode_Emergency : USINT := 2;
END_VAR
    CASE state OF
        0: (* INIT *)
            state := 1; (* Entry *)
        1: (* Entry *)
            controller0();
            IF controller0.is_done THEN
                state := 2; (*  *)
            END_IF;
        2: (* END *)
            state := 2;
    END_CASE;
    is_done := state = 2;
END_FUNCTION_BLOCK

