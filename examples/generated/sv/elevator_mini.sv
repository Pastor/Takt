// Порождено компилятором Lam (lamc) — цель: SystemVerilog (IEEE 1800).
// Не редактировать вручную: файл перезаписывается при каждой генерации.
//
// Такт модели Lam ≡ фронт clk (posedge). Сброс синхронный, активный низкий:
// ветвь if (!rst_n) несёт стартовое состояние — синтетического INIT нет,
// поэтому тело стартового состояния исполняется на такте 1 (контракт 0033).

module elevator_mini (
    input  logic clk,   // служебный порт цели sv: в .lam его нет
    input  logic rst_n, // служебный порт цели sv: сброс, активный низкий
    input  logic en = 1'b1, // служебный порт цели sv: clock enable; НЕ обязателен (умолчание 1)
    input  logic CabinButton_DC,
    input  logic CabinButton_F1,
    input  logic CabinButton_F2,
    input  logic CabinButton_F3,
    input  logic CabinButton_F4,
    input  logic CabinButton_F5,
    input  logic CabinButton_F6,
    input  logic CabinButton_F7,
    input  logic CabinButton_F8,
    input  logic CabinButton_F9,
    input  logic FloorButton_F1,
    input  logic FloorButton_F2,
    input  logic FloorButton_F3,
    input  logic FloorButton_F4,
    input  logic FloorButton_F5,
    input  logic FloorButton_F6,
    input  logic FloorButton_F7,
    input  logic FloorButton_F8,
    input  logic FloorButton_F9,
    input  logic FloorSensor_F1_Bottom,
    input  logic FloorSensor_F2_Bottom,
    input  logic FloorSensor_F3_Bottom,
    input  logic FloorSensor_F4_Bottom,
    input  logic FloorSensor_F5_Bottom,
    input  logic FloorSensor_F6_Bottom,
    input  logic FloorSensor_F7_Bottom,
    input  logic FloorSensor_F8_Bottom,
    input  logic FloorSensor_F9_Bottom,
    input  logic ElevatorMotor_SensorD,
    input  logic ElevatorMotor_SensorU,
    output logic DoorOpen,
    output logic ElevatorMotor_Down,
    output logic ElevatorMotor_Stop,
    output logic ElevatorMotor_Up,
    output logic is_done
);
    typedef enum logic [1:0] {
        COMMAND_UP = 2'd0,
        COMMAND_DOWN = 2'd1,
        COMMAND_STOP = 2'd2
    } command_e;

    // Состояния модели 'Cabin (ElevatorMini:Cabin)'. Синтетического INIT нет: стартовое
    // состояние живёт в ветви сброса (контракт ADR 0033).
    typedef enum logic [1:0] {
        ELEVATOR_MINI_CABIN_AT_FLOOR = 2'd0,
        ELEVATOR_MINI_CABIN_IDLE = 2'd1,
        ELEVATOR_MINI_CABIN_MOVING = 2'd2,
        ELEVATOR_MINI_CABIN_END = 2'd3
    } elevator_mini_cabin_state_e;

    // Состояния модели 'Motor (ElevatorMini:Motor)'. Синтетического INIT нет: стартовое
    // состояние живёт в ветви сброса (контракт ADR 0033).
    typedef enum logic [2:0] {
        ELEVATOR_MINI_MOTOR_DOWN = 3'd0,
        ELEVATOR_MINI_MOTOR_IDLE = 3'd1,
        ELEVATOR_MINI_MOTOR_STOP = 3'd2,
        ELEVATOR_MINI_MOTOR_UP = 3'd3,
        ELEVATOR_MINI_MOTOR_END = 3'd4
    } elevator_mini_motor_state_e;

    // Состояния модели 'elevator_mini (ElevatorMini)'. Синтетического INIT нет: стартовое
    // состояние живёт в ветви сброса (контракт ADR 0033).
    typedef enum logic [0:0] {
        ELEVATOR_MINI_MAIN = 1'd0,
        ELEVATOR_MINI_END = 1'd1
    } elevator_mini_state_e;

    elevator_mini_cabin_state_e elevator_mini_cabin_state;
    elevator_mini_cabin_state_e elevator_mini_cabin_state_next;
    elevator_mini_motor_state_e elevator_mini_motor_state;
    elevator_mini_motor_state_e elevator_mini_motor_state_next;
    elevator_mini_state_e state;
    elevator_mini_state_e state_next;
    command_e elevator_mini_command;
    command_e elevator_mini_command_next;
    logic [7:0] elevator_mini_current_floor;
    logic [7:0] elevator_mini_current_floor_next;
    logic [7:0] elevator_mini_target_floor;
    logic [7:0] elevator_mini_target_floor_next;
    logic DoorOpen_next;
    logic ElevatorMotor_Down_next;
    logic ElevatorMotor_Stop_next;
    logic ElevatorMotor_Up_next;

    // Комбинационная часть: БЛОКИРУЮЩИЕ присваивания, поэтому порядок
    // операторов и видимость записей внутри такта — в точности как в C.
    always_comb begin
        // Умолчание «остаться как есть». Без него неполное присваивание
        // даёт защёлку (verilator: LATCH).
        elevator_mini_cabin_state_next = elevator_mini_cabin_state;
        elevator_mini_motor_state_next = elevator_mini_motor_state;
        state_next = state;
        elevator_mini_command_next = elevator_mini_command;
        elevator_mini_current_floor_next = elevator_mini_current_floor;
        elevator_mini_target_floor_next = elevator_mini_target_floor;
        DoorOpen_next = DoorOpen;
        ElevatorMotor_Down_next = ElevatorMotor_Down;
        ElevatorMotor_Stop_next = ElevatorMotor_Stop;
        ElevatorMotor_Up_next = ElevatorMotor_Up;

        unique case (state)
            ELEVATOR_MINI_MAIN: begin
                // Под-модель 'Cabin (ElevatorMini:Cabin)' — инлайн её такта.
                unique case (elevator_mini_cabin_state)
                    ELEVATOR_MINI_CABIN_AT_FLOOR: begin
                        DoorOpen_next = 1'b1;
                        if (CabinButton_DC) begin
                            elevator_mini_command_next = COMMAND_STOP;
                            elevator_mini_cabin_state_next = ELEVATOR_MINI_CABIN_IDLE;
                        end
                    end
                    ELEVATOR_MINI_CABIN_IDLE: begin
                        if (FloorSensor_F1_Bottom) begin
                            elevator_mini_current_floor_next = 1;
                        end
                        if (FloorSensor_F2_Bottom) begin
                            elevator_mini_current_floor_next = 2;
                        end
                        if (FloorSensor_F3_Bottom) begin
                            elevator_mini_current_floor_next = 3;
                        end
                        if (FloorSensor_F4_Bottom) begin
                            elevator_mini_current_floor_next = 4;
                        end
                        if (FloorSensor_F5_Bottom) begin
                            elevator_mini_current_floor_next = 5;
                        end
                        if (FloorSensor_F6_Bottom) begin
                            elevator_mini_current_floor_next = 6;
                        end
                        if (FloorSensor_F7_Bottom) begin
                            elevator_mini_current_floor_next = 7;
                        end
                        if (FloorSensor_F8_Bottom) begin
                            elevator_mini_current_floor_next = 8;
                        end
                        if (FloorSensor_F9_Bottom) begin
                            elevator_mini_current_floor_next = 9;
                        end
                        if (CabinButton_F1) begin
                            elevator_mini_target_floor_next = 1;
                        end
                        if (CabinButton_F2) begin
                            elevator_mini_target_floor_next = 2;
                        end
                        if (CabinButton_F3) begin
                            elevator_mini_target_floor_next = 3;
                        end
                        if (CabinButton_F4) begin
                            elevator_mini_target_floor_next = 4;
                        end
                        if (CabinButton_F5) begin
                            elevator_mini_target_floor_next = 5;
                        end
                        if (CabinButton_F6) begin
                            elevator_mini_target_floor_next = 6;
                        end
                        if (CabinButton_F7) begin
                            elevator_mini_target_floor_next = 7;
                        end
                        if (CabinButton_F8) begin
                            elevator_mini_target_floor_next = 8;
                        end
                        if (CabinButton_F9) begin
                            elevator_mini_target_floor_next = 9;
                        end
                        if (FloorButton_F1) begin
                            elevator_mini_target_floor_next = 1;
                        end
                        if (FloorButton_F2) begin
                            elevator_mini_target_floor_next = 2;
                        end
                        if (FloorButton_F3) begin
                            elevator_mini_target_floor_next = 3;
                        end
                        if (FloorButton_F4) begin
                            elevator_mini_target_floor_next = 4;
                        end
                        if (FloorButton_F5) begin
                            elevator_mini_target_floor_next = 5;
                        end
                        if (FloorButton_F6) begin
                            elevator_mini_target_floor_next = 6;
                        end
                        if (FloorButton_F7) begin
                            elevator_mini_target_floor_next = 7;
                        end
                        if (FloorButton_F8) begin
                            elevator_mini_target_floor_next = 8;
                        end
                        if (FloorButton_F9) begin
                            elevator_mini_target_floor_next = 9;
                        end
                        DoorOpen_next = 1'b1;
                        if ((elevator_mini_target_floor_next != 0)) begin
                            elevator_mini_cabin_state_next = ELEVATOR_MINI_CABIN_MOVING;
                        end
                    end
                    ELEVATOR_MINI_CABIN_MOVING: begin
                        if (FloorSensor_F1_Bottom) begin
                            elevator_mini_current_floor_next = 1;
                        end
                        if (FloorSensor_F2_Bottom) begin
                            elevator_mini_current_floor_next = 2;
                        end
                        if (FloorSensor_F3_Bottom) begin
                            elevator_mini_current_floor_next = 3;
                        end
                        if (FloorSensor_F4_Bottom) begin
                            elevator_mini_current_floor_next = 4;
                        end
                        if (FloorSensor_F5_Bottom) begin
                            elevator_mini_current_floor_next = 5;
                        end
                        if (FloorSensor_F6_Bottom) begin
                            elevator_mini_current_floor_next = 6;
                        end
                        if (FloorSensor_F7_Bottom) begin
                            elevator_mini_current_floor_next = 7;
                        end
                        if (FloorSensor_F8_Bottom) begin
                            elevator_mini_current_floor_next = 8;
                        end
                        if (FloorSensor_F9_Bottom) begin
                            elevator_mini_current_floor_next = 9;
                        end
                        if ((elevator_mini_target_floor_next > elevator_mini_current_floor_next)) begin
                            elevator_mini_command_next = COMMAND_UP;
                        end
                        if ((elevator_mini_target_floor_next < elevator_mini_current_floor_next)) begin
                            elevator_mini_command_next = COMMAND_DOWN;
                        end
                        if ((elevator_mini_target_floor_next == elevator_mini_current_floor_next)) begin
                            elevator_mini_command_next = COMMAND_STOP;
                            elevator_mini_target_floor_next = 0;
                            elevator_mini_cabin_state_next = ELEVATOR_MINI_CABIN_AT_FLOOR;
                        end
                    end
                    ELEVATOR_MINI_CABIN_END: begin end
                endcase
                // Под-модель 'Motor (ElevatorMini:Motor)' — инлайн её такта.
                unique case (elevator_mini_motor_state)
                    ELEVATOR_MINI_MOTOR_DOWN: begin
                        ElevatorMotor_Down_next = 1'b1;
                        if (((elevator_mini_command_next == COMMAND_STOP) || ElevatorMotor_SensorD)) begin
                            elevator_mini_motor_state_next = ELEVATOR_MINI_MOTOR_STOP;
                        end
                    end
                    ELEVATOR_MINI_MOTOR_IDLE: begin
                        if ((elevator_mini_command_next == COMMAND_UP)) begin
                            elevator_mini_motor_state_next = ELEVATOR_MINI_MOTOR_UP;
                        end
                        else if ((elevator_mini_command_next == COMMAND_DOWN)) begin
                            elevator_mini_motor_state_next = ELEVATOR_MINI_MOTOR_DOWN;
                        end
                        else if ((elevator_mini_command_next == COMMAND_STOP)) begin
                            elevator_mini_motor_state_next = ELEVATOR_MINI_MOTOR_STOP;
                        end
                    end
                    ELEVATOR_MINI_MOTOR_STOP: begin
                        ElevatorMotor_Stop_next = 1'b1;
                        begin
                            ElevatorMotor_Stop_next = 1'b1;
                            elevator_mini_motor_state_next = ELEVATOR_MINI_MOTOR_IDLE;
                        end
                    end
                    ELEVATOR_MINI_MOTOR_UP: begin
                        ElevatorMotor_Up_next = 1'b1;
                        if (((elevator_mini_command_next == COMMAND_STOP) || ElevatorMotor_SensorU)) begin
                            elevator_mini_motor_state_next = ELEVATOR_MINI_MOTOR_STOP;
                        end
                    end
                    ELEVATOR_MINI_MOTOR_END: begin end
                endcase
                if ((elevator_mini_cabin_state_next == ELEVATOR_MINI_CABIN_END) && (elevator_mini_motor_state_next == ELEVATOR_MINI_MOTOR_END)) begin
                    state_next = ELEVATOR_MINI_END;
                end
            end
            ELEVATOR_MINI_END: begin end
        endcase
    end

    // Регистровая часть: НЕБЛОКИРУЮЩИЕ присваивания. Ветвь сброса несёт
    // стартовые состояния ВСЕХ уровней — они сбрасываются одним фронтом,
    // поэтому сдвиг такта равен нулю на любой глубине (контракт 0033).
    always_ff @(posedge clk) begin
        if (!rst_n) begin
            elevator_mini_cabin_state <= ELEVATOR_MINI_CABIN_IDLE;
            elevator_mini_motor_state <= ELEVATOR_MINI_MOTOR_IDLE;
            state <= ELEVATOR_MINI_MAIN;
            elevator_mini_command <= COMMAND_STOP;
            elevator_mini_current_floor <= 1;
            elevator_mini_target_floor <= 0;
            DoorOpen <= '0;
            ElevatorMotor_Down <= '0;
            ElevatorMotor_Stop <= '0;
            ElevatorMotor_Up <= '0;
            elevator_mini_command <= COMMAND_STOP;
            ElevatorMotor_Stop <= 1'b1;
        end else if (en) begin
            elevator_mini_cabin_state <= elevator_mini_cabin_state_next;
            elevator_mini_motor_state <= elevator_mini_motor_state_next;
            state <= state_next;
            elevator_mini_command <= elevator_mini_command_next;
            elevator_mini_current_floor <= elevator_mini_current_floor_next;
            elevator_mini_target_floor <= elevator_mini_target_floor_next;
            DoorOpen <= DoorOpen_next;
            ElevatorMotor_Down <= ElevatorMotor_Down_next;
            ElevatorMotor_Stop <= ElevatorMotor_Stop_next;
            ElevatorMotor_Up <= ElevatorMotor_Up_next;
        end
    end

    // Терминальность модели наблюдаема снаружи — аналог _is_done() цели c.
    assign is_done = (state == ELEVATOR_MINI_END);
endmodule
