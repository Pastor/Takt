(*
 * Порождено компилятором Lam (lamc) — цель: Structured Text (IEC 61131-3).
 * Не редактировать вручную: файл перезаписывается при каждой генерации.
 *)

TYPE
    Coord :
    STRUCT
        x : ARRAY [0..31] OF BOOL;
        y : ARRAY [0..31] OF BOOL;
        z : ARRAY [0..31] OF BOOL;
    END_STRUCT;
END_TYPE

FUNCTION_BLOCK ExtendComplexA
    (* Тело: задачи 0041-03…0041-05 (CASE state OF, выражения, адреса). *)
END_FUNCTION_BLOCK

FUNCTION_BLOCK ExtendComplexB
    (* Тело: задачи 0041-03…0041-05 (CASE state OF, выражения, адреса). *)
END_FUNCTION_BLOCK

FUNCTION_BLOCK ExtendComplexC
VAR CONSTANT
    Point_Global : USINT := 0;
    Point_Local : USINT := 1;
END_VAR
    (* Тело: задачи 0041-03…0041-05 (CASE state OF, выражения, адреса). *)
END_FUNCTION_BLOCK

FUNCTION_BLOCK ExtendComplexCC1
    (* Тело: задачи 0041-03…0041-05 (CASE state OF, выражения, адреса). *)
END_FUNCTION_BLOCK

FUNCTION_BLOCK ExtendComplexCC2
    (* Тело: задачи 0041-03…0041-05 (CASE state OF, выражения, адреса). *)
END_FUNCTION_BLOCK

FUNCTION_BLOCK ExtendComplexD
    (* Тело: задачи 0041-03…0041-05 (CASE state OF, выражения, адреса). *)
END_FUNCTION_BLOCK

FUNCTION_BLOCK ExtendComplexE
    (* Тело: задачи 0041-03…0041-05 (CASE state OF, выражения, адреса). *)
END_FUNCTION_BLOCK

FUNCTION_BLOCK ExtendComplexF
    (* Тело: задачи 0041-03…0041-05 (CASE state OF, выражения, адреса). *)
END_FUNCTION_BLOCK

FUNCTION_BLOCK ExtendComplex
VAR_INPUT
    WAIT : BOOL;
END_VAR
VAR_OUTPUT
    IDLE : BOOL;
    WORK : BOOL;
END_VAR
VAR
    x : USINT := 1;
    y : USINT := 2;
END_VAR
VAR CONSTANT
    Constant_X : USINT := 0;
    Constant_Y : USINT := 1;
    Constant_Z : USINT := 2;
    ENABLED : BOOL;
END_VAR
    (* Тело: задачи 0041-03…0041-05 (CASE state OF, выражения, адреса). *)
END_FUNCTION_BLOCK

