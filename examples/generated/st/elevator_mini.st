(*
 * Порождено компилятором Lam (lamc) — цель: Structured Text (IEC 61131-3).
 * Не редактировать вручную: файл перезаписывается при каждой генерации.
 *)

FUNCTION_BLOCK ElevatorMiniCabin
VAR_INPUT
    CabinButton_DC : BOOL;
    CabinButton_F1 : BOOL;
    CabinButton_F2 : BOOL;
    CabinButton_F3 : BOOL;
    CabinButton_F4 : BOOL;
    CabinButton_F5 : BOOL;
    CabinButton_F6 : BOOL;
    CabinButton_F7 : BOOL;
    CabinButton_F8 : BOOL;
    CabinButton_F9 : BOOL;
    FloorButton_F1 : BOOL;
    FloorButton_F2 : BOOL;
    FloorButton_F3 : BOOL;
    FloorButton_F4 : BOOL;
    FloorButton_F5 : BOOL;
    FloorButton_F6 : BOOL;
    FloorButton_F7 : BOOL;
    FloorButton_F8 : BOOL;
    FloorButton_F9 : BOOL;
    FloorSensor_F1_Bottom : BOOL;
    FloorSensor_F2_Bottom : BOOL;
    FloorSensor_F3_Bottom : BOOL;
    FloorSensor_F4_Bottom : BOOL;
    FloorSensor_F5_Bottom : BOOL;
    FloorSensor_F6_Bottom : BOOL;
    FloorSensor_F7_Bottom : BOOL;
    FloorSensor_F8_Bottom : BOOL;
    FloorSensor_F9_Bottom : BOOL;
END_VAR
VAR_OUTPUT
    DoorOpen : BOOL;
END_VAR
    (* Тело: задачи 0041-03…0041-05 (CASE state OF, выражения, адреса). *)
END_FUNCTION_BLOCK

FUNCTION_BLOCK ElevatorMiniMotor
VAR_INPUT
    ElevatorMotor_SensorD : BOOL;
    ElevatorMotor_SensorU : BOOL;
END_VAR
VAR_OUTPUT
    ElevatorMotor_Down : BOOL;
    ElevatorMotor_Stop : BOOL;
    ElevatorMotor_Up : BOOL;
END_VAR
    (* Тело: задачи 0041-03…0041-05 (CASE state OF, выражения, адреса). *)
END_FUNCTION_BLOCK

FUNCTION_BLOCK ElevatorMini
VAR
    command : USINT := 2;
    current_floor : USINT := 1;
    target_floor : USINT := 0;
END_VAR
VAR CONSTANT
    Command_Up : USINT := 0;
    Command_Down : USINT := 1;
    Command_Stop : USINT := 2;
END_VAR
    (* Тело: задачи 0041-03…0041-05 (CASE state OF, выражения, адреса). *)
END_FUNCTION_BLOCK

