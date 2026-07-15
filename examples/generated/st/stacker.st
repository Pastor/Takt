(*
 * Порождено компилятором Lam (lamc) — цель: Structured Text (IEC 61131-3).
 * Не редактировать вручную: файл перезаписывается при каждой генерации.
 *)

FUNCTION_BLOCK StackerCommandReceiver
    (* Тело: задачи 0041-03…0041-05 (CASE state OF, выражения, адреса). *)
END_FUNCTION_BLOCK

FUNCTION_BLOCK StackerLiftController
    (* Тело: задачи 0041-03…0041-05 (CASE state OF, выражения, адреса). *)
END_FUNCTION_BLOCK

FUNCTION_BLOCK StackerMovementController
    (* Тело: задачи 0041-03…0041-05 (CASE state OF, выражения, адреса). *)
END_FUNCTION_BLOCK

FUNCTION_BLOCK Stacker
VAR_INPUT
    pos_row : USINT;
    pos_section : USINT;
    pos_stack : USINT;
    sense_at_charge : BOOL;
    sense_battery_low : BOOL;
    sense_loaded : BOOL;
    task_row_no : USINT;
    task_section_no : USINT;
    task_stack_no : USINT;
    task_type : BOOL;
    task_valid : BOOL;
END_VAR
VAR_OUTPUT
    cmd_ack : BOOL;
    cmd_done : BOOL;
    cmd_fork : BOOL;
    cmd_target_row : USINT;
    cmd_target_section : USINT;
    cmd_target_stack : USINT;
END_VAR
VAR
    busy : BOOL := FALSE;
    eta : USINT := 0;
    lift_done : BOOL := FALSE;
    lift_op : BOOL := FALSE;
    lift_request : BOOL := FALSE;
    tgt_row : USINT := 0;
    tgt_section : USINT := 0;
    tgt_stack : USINT := 0;
    tgt_type : BOOL := FALSE;
END_VAR
VAR CONSTANT
    CHARGE_ROW : USINT := 0;
    CHARGE_SECTION : USINT := 0;
    CHARGE_STACK : USINT := 0;
    DROPOFF_ROW : USINT := 1;
    DROPOFF_SECTION : USINT := 1;
    DROPOFF_STACK : USINT := 11;
    PICKUP_ROW : USINT := 1;
    PICKUP_SECTION : USINT := 1;
    PICKUP_STACK : USINT := 0;
END_VAR
    (* Тело: задачи 0041-03…0041-05 (CASE state OF, выражения, адреса). *)
END_FUNCTION_BLOCK

