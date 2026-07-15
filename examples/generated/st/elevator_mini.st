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
    is_done : BOOL;
    DoorOpen : BOOL;
END_VAR
VAR_IN_OUT
    command : USINT;
    current_floor : USINT;
    target_floor : USINT;
END_VAR
VAR
    state : USINT := 0;
END_VAR
VAR CONSTANT
    Command_Up : USINT := 0;
    Command_Down : USINT := 1;
    Command_Stop : USINT := 2;
END_VAR
    CASE state OF
        0: (* INIT *)
            command := 2;
            state := 2; (* Idle *)
        1: (* AtFloor *)
            DoorOpen := TRUE;
            IF CabinButton_DC THEN
                command := 2;
                state := 2; (* Idle *)
            END_IF;
        2: (* Idle *)
            IF FloorSensor_F1_Bottom THEN
                current_floor := 1;
            END_IF;
            IF FloorSensor_F2_Bottom THEN
                current_floor := 2;
            END_IF;
            IF FloorSensor_F3_Bottom THEN
                current_floor := 3;
            END_IF;
            IF FloorSensor_F4_Bottom THEN
                current_floor := 4;
            END_IF;
            IF FloorSensor_F5_Bottom THEN
                current_floor := 5;
            END_IF;
            IF FloorSensor_F6_Bottom THEN
                current_floor := 6;
            END_IF;
            IF FloorSensor_F7_Bottom THEN
                current_floor := 7;
            END_IF;
            IF FloorSensor_F8_Bottom THEN
                current_floor := 8;
            END_IF;
            IF FloorSensor_F9_Bottom THEN
                current_floor := 9;
            END_IF;
            IF CabinButton_F1 THEN
                target_floor := 1;
            END_IF;
            IF CabinButton_F2 THEN
                target_floor := 2;
            END_IF;
            IF CabinButton_F3 THEN
                target_floor := 3;
            END_IF;
            IF CabinButton_F4 THEN
                target_floor := 4;
            END_IF;
            IF CabinButton_F5 THEN
                target_floor := 5;
            END_IF;
            IF CabinButton_F6 THEN
                target_floor := 6;
            END_IF;
            IF CabinButton_F7 THEN
                target_floor := 7;
            END_IF;
            IF CabinButton_F8 THEN
                target_floor := 8;
            END_IF;
            IF CabinButton_F9 THEN
                target_floor := 9;
            END_IF;
            IF FloorButton_F1 THEN
                target_floor := 1;
            END_IF;
            IF FloorButton_F2 THEN
                target_floor := 2;
            END_IF;
            IF FloorButton_F3 THEN
                target_floor := 3;
            END_IF;
            IF FloorButton_F4 THEN
                target_floor := 4;
            END_IF;
            IF FloorButton_F5 THEN
                target_floor := 5;
            END_IF;
            IF FloorButton_F6 THEN
                target_floor := 6;
            END_IF;
            IF FloorButton_F7 THEN
                target_floor := 7;
            END_IF;
            IF FloorButton_F8 THEN
                target_floor := 8;
            END_IF;
            IF FloorButton_F9 THEN
                target_floor := 9;
            END_IF;
            DoorOpen := TRUE;
            IF target_floor <> 0 THEN
                state := 3; (* Moving *)
            END_IF;
        3: (* Moving *)
            IF FloorSensor_F1_Bottom THEN
                current_floor := 1;
            END_IF;
            IF FloorSensor_F2_Bottom THEN
                current_floor := 2;
            END_IF;
            IF FloorSensor_F3_Bottom THEN
                current_floor := 3;
            END_IF;
            IF FloorSensor_F4_Bottom THEN
                current_floor := 4;
            END_IF;
            IF FloorSensor_F5_Bottom THEN
                current_floor := 5;
            END_IF;
            IF FloorSensor_F6_Bottom THEN
                current_floor := 6;
            END_IF;
            IF FloorSensor_F7_Bottom THEN
                current_floor := 7;
            END_IF;
            IF FloorSensor_F8_Bottom THEN
                current_floor := 8;
            END_IF;
            IF FloorSensor_F9_Bottom THEN
                current_floor := 9;
            END_IF;
            IF target_floor > current_floor THEN
                command := 0;
            END_IF;
            IF target_floor < current_floor THEN
                command := 1;
            END_IF;
            IF target_floor = current_floor THEN
                command := 2;
                target_floor := 0;
                state := 1; (* AtFloor *)
            END_IF;
        4: (* END *)
            state := 4;
    END_CASE;
    is_done := state = 4;
END_FUNCTION_BLOCK

FUNCTION_BLOCK ElevatorMiniMotor
VAR_INPUT
    ElevatorMotor_SensorD : BOOL;
    ElevatorMotor_SensorU : BOOL;
END_VAR
VAR_OUTPUT
    is_done : BOOL;
    ElevatorMotor_Down : BOOL;
    ElevatorMotor_Stop : BOOL;
    ElevatorMotor_Up : BOOL;
END_VAR
VAR_IN_OUT
    command : USINT;
END_VAR
VAR
    state : USINT := 0;
END_VAR
VAR CONSTANT
    Command_Up : USINT := 0;
    Command_Down : USINT := 1;
    Command_Stop : USINT := 2;
END_VAR
    CASE state OF
        0: (* INIT *)
            ElevatorMotor_Stop := TRUE;
            state := 2; (* Idle *)
        1: (* Down *)
            ElevatorMotor_Down := TRUE;
            IF command = Command_Stop OR ElevatorMotor_SensorD THEN
                state := 3; (* Stop *)
            END_IF;
        2: (* Idle *)
            IF command = Command_Up THEN
                state := 4; (* Up *)
            ELSIF command = Command_Down THEN
                state := 1; (* Down *)
            ELSIF command = Command_Stop THEN
                state := 3; (* Stop *)
            END_IF;
        3: (* Stop *)
            ElevatorMotor_Stop := TRUE;
            ElevatorMotor_Stop := TRUE;
            state := 2; (* Idle *)
        4: (* Up *)
            ElevatorMotor_Up := TRUE;
            IF command = Command_Stop OR ElevatorMotor_SensorU THEN
                state := 3; (* Stop *)
            END_IF;
        5: (* END *)
            state := 5;
    END_CASE;
    is_done := state = 5;
END_FUNCTION_BLOCK

FUNCTION_BLOCK ElevatorMini
VAR_OUTPUT
    is_done : BOOL;
END_VAR
VAR
    state : USINT := 0;
    cabin0 : ElevatorMiniCabin;
    motor1 : ElevatorMiniMotor;
    command : USINT := 2;
    current_floor : USINT := 1;
    target_floor : USINT := 0;
END_VAR
VAR CONSTANT
    Command_Up : USINT := 0;
    Command_Down : USINT := 1;
    Command_Stop : USINT := 2;
END_VAR
    CASE state OF
        0: (* INIT *)
            state := 1; (* Main *)
        1: (* Main *)
            cabin0(command := command, current_floor := current_floor, target_floor := target_floor);
            motor1(command := command);
            IF cabin0.is_done AND motor1.is_done THEN
                state := 2; (*  *)
            END_IF;
        2: (* END *)
            state := 2;
    END_CASE;
    is_done := state = 2;
END_FUNCTION_BLOCK

