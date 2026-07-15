(*
 * Порождено компилятором Lam (lamc) — цель: Structured Text (IEC 61131-3).
 * Не редактировать вручную: файл перезаписывается при каждой генерации.
 *)

FUNCTION_BLOCK ElevatorEngine
VAR CONSTANT
    Action_Idle : UINT := 670;
    Action_Closing : UINT := 671;
END_VAR
    (* Тело: задачи 0041-03…0041-05 (CASE state OF, выражения, адреса). *)
END_FUNCTION_BLOCK

FUNCTION_BLOCK Elevator
VAR_INPUT
    sensors_1 : USINT;
    sensors_2 : USINT;
    sensors_3 : USINT;
    sensors_4 : USINT;
    sensors_5 : USINT;
    sensors_6 : USINT;
    sensors_7 : USINT;
    sensors_8 : USINT;
    sensors_9 : USINT;
    sensors_cab : USINT;
END_VAR
VAR
    current_floor : USINT := 1;
    has_call : USINT := 0;
    target_floor : USINT := 1;
END_VAR
VAR CONSTANT
    Floor_Bottom : USINT := 80;
    Floor_Top : USINT := 81;
END_VAR
    (* Тело: задачи 0041-03…0041-05 (CASE state OF, выражения, адреса). *)
END_FUNCTION_BLOCK

