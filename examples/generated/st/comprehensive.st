(*
 * Порождено компилятором Lam (lamc) — цель: Structured Text (IEC 61131-3).
 * Не редактировать вручную: файл перезаписывается при каждой генерации.
 *)

FUNCTION_BLOCK ComprehensiveController
VAR
    count : USINT := 0;
    mode : USINT := 0;
    temperature : USINT := 0;
END_VAR
VAR CONSTANT
    MAX_COUNT : USINT := 10;
    MAX_TEMP : USINT := 100;
END_VAR
    (* Тело: задачи 0041-03…0041-05 (CASE state OF, выражения, адреса). *)
END_FUNCTION_BLOCK

FUNCTION_BLOCK Comprehensive
VAR CONSTANT
    Mode_Auto : USINT := 0;
    Mode_Manual : USINT := 1;
    Mode_Emergency : USINT := 2;
END_VAR
    (* Тело: задачи 0041-03…0041-05 (CASE state OF, выражения, адреса). *)
END_FUNCTION_BLOCK

